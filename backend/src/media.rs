use crate::{domain::Metrics, hardware, subprocess};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

pub const EXTENSIONS: &[&str] = &[
    "mp3", "wav", "flac", "m4a", "ogg", "aac", "mp4", "mkv", "mov", "webm", "avi",
];
const DEMUXERS: &str = "mp3,wav,flac,aac,ogg,mov,matroska,webm,avi";
const MAX_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Clone)]
pub struct Programs {
    pub ffmpeg: Option<PathBuf>,
    pub ffprobe: Option<PathBuf>,
    pub whisper: Option<PathBuf>,
    pub optimized: bool,
}

impl Programs {
    pub fn locate() -> Self {
        let avx = hardware::supports_optimized_cpu()
            .then(|| hardware::program("whisper-cli-avx2", "LOCALSCRIBE_WHISPER_AVX2"))
            .flatten();
        let optimized = avx.is_some() && std::env::var_os("LOCALSCRIBE_WHISPER").is_none();
        Self {
            ffmpeg: hardware::program("ffmpeg", "LOCALSCRIBE_FFMPEG"),
            ffprobe: hardware::program("ffprobe", "LOCALSCRIBE_FFPROBE"),
            whisper: if optimized {
                avx
            } else {
                hardware::program("whisper-cli", "LOCALSCRIBE_WHISPER")
            },
            optimized,
        }
    }

    pub fn ready(&self) -> bool {
        self.ffmpeg.is_some() && self.ffprobe.is_some() && self.whisper.is_some()
    }

    pub async fn validate(&mut self) {
        for (slot, flag) in [
            (&mut self.ffmpeg, "-version"),
            (&mut self.ffprobe, "-version"),
            (&mut self.whisper, "-h"),
        ] {
            if let Some(path) = slot {
                let mut command = subprocess::command(path);
                command.arg(flag);
                if subprocess::run(
                    command,
                    &CancellationToken::new(),
                    0,
                    Duration::from_secs(8),
                    0,
                    "Runtime check",
                )
                .await
                .is_err()
                {
                    *slot = None;
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct Probe {
    streams: Vec<Stream>,
    format: Option<Stream>,
}

#[derive(Deserialize)]
struct Stream {
    duration: Option<String>,
}

fn local_input(command: &mut Command) {
    command.args([
        "-protocol_whitelist",
        "file",
        "-format_whitelist",
        DEMUXERS,
        "-max_alloc",
        "134217728",
    ]);
}

pub async fn probe(
    program: &Path,
    source: &Path,
    cancellation: &CancellationToken,
) -> Result<(i64, Metrics)> {
    let mut command = subprocess::command(program);
    local_input(&mut command);
    command
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=duration:format=duration",
            "-of",
            "json",
            "-i",
        ])
        .arg(source);
    let (output, metrics) = subprocess::run(
        command,
        cancellation,
        384,
        Duration::from_secs(45),
        256 * 1024,
        "Media inspection",
    )
    .await?;
    let probe: Probe = serde_json::from_slice(&output)
        .context("FFprobe returned an invalid media description.")?;
    let stream = probe
        .streams
        .first()
        .context("This file has no audio track.")?;
    let seconds = |stream: &Stream| {
        stream
            .duration
            .as_ref()
            .and_then(|text| text.parse::<f64>().ok())
            .filter(|value| value.is_finite() && *value > 0.0)
    };
    let duration = probe
        .format
        .as_ref()
        .and_then(seconds)
        .or_else(|| seconds(stream))
        .context("This file does not have a seekable audio duration.")?;
    let duration = (duration * 1000.0).ceil() as i64;
    ensure!(
        duration <= MAX_MS,
        "The longest supported file is 24 hours."
    );
    Ok((duration, metrics))
}

fn decode(program: &Path, source: &Path, start_ms: i64) -> Command {
    let mut command = subprocess::command(program);
    command.args(["-v", "error", "-nostdin", "-y"]);
    local_input(&mut command);
    if start_ms > 0 {
        command.arg("-ss").arg(seconds(start_ms));
    }
    command.args(["-threads", "2", "-i"]).arg(source).args([
        "-map",
        "0:a:0",
        "-vn",
        "-sn",
        "-dn",
        "-map_metadata",
        "-1",
        "-map_chapters",
        "-1",
        "-ac",
        "1",
    ]);
    command
}

fn seconds(ms: i64) -> String {
    format!("{}.{:03}", ms / 1000, ms % 1000)
}

pub async fn pcm(
    program: &Path,
    source: &Path,
    target: &Path,
    start_ms: i64,
    duration_ms: i64,
    cancellation: &CancellationToken,
    budget: u64,
) -> Result<Metrics> {
    let mut command = decode(program, source, start_ms);
    command
        .args(["-af", "aresample=async=1:first_pts=0", "-t"])
        .arg(seconds(duration_ms))
        .args(["-ar", "16000", "-c:a", "pcm_s16le", "-f", "wav"])
        .arg(target);
    let (_, metrics) = subprocess::run(
        command,
        cancellation,
        budget,
        Duration::from_secs(180),
        0,
        "Audio decoding",
    )
    .await?;
    Ok(metrics)
}

pub async fn preview(
    program: &Path,
    source: &Path,
    target: &Path,
    duration_ms: i64,
    cancellation: &CancellationToken,
) -> Result<()> {
    let mut command = decode(program, source, 0);
    command
        .args(["-af", "aresample=async=1:first_pts=0", "-t"])
        .arg(seconds(duration_ms))
        .args([
            "-ar",
            "24000",
            "-c:a",
            "libmp3lame",
            "-b:a",
            "48k",
            "-f",
            "mp3",
        ])
        .arg(target);
    subprocess::run(
        command,
        cancellation,
        384,
        Duration::from_secs(600),
        0,
        "Playback preview",
    )
    .await?;
    Ok(())
}

pub fn browser_type(extension: &str) -> Option<&'static str> {
    match extension {
        "wav" => Some("audio/wav"),
        "mp3" => Some("audio/mpeg"),
        "flac" => Some("audio/flac"),
        "ogg" => Some("audio/ogg"),
        "m4a" => Some("audio/mp4"),
        "mp4" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        _ => None,
    }
}
