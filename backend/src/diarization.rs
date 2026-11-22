use crate::{
    State,
    domain::{Job, Segment, Suggestion},
    hardware, media, subprocess,
};
use anyhow::{Context, Result, ensure};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    time::Duration,
};

struct Engine {
    command: PathBuf,
    segmentation: PathBuf,
    embedding: PathBuf,
}

impl Engine {
    fn locate() -> Option<Self> {
        let command = hardware::program(
            "sherpa-onnx-offline-speaker-diarization",
            "LOCALSCRIBE_DIARIZATION",
        )?;
        let segmentation = model("LOCALSCRIBE_SPEAKER_SEGMENTATION")?;
        let embedding = model("LOCALSCRIBE_SPEAKER_EMBEDDING")?;
        Some(Self {
            command,
            segmentation,
            embedding,
        })
    }
}

fn model(variable: &str) -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os(variable)?)
        .canonicalize()
        .ok()?;
    path.is_file().then_some(path)
}

fn parameter(flag: &str, path: &Path) -> OsString {
    let mut value = OsString::from(flag);
    value.push(path);
    value
}

pub fn available() -> bool {
    Engine::locate().is_some()
}

pub async fn suggest(
    state: &State,
    job: &Job,
    segments: &[Segment],
    label: &str,
    speaker_count: Option<usize>,
) -> Result<Vec<Suggestion>> {
    let engine =
        Engine::locate().context("Install sherpa-onnx and configure its two local models.")?;
    let ffmpeg = state
        .programs
        .ffmpeg
        .as_deref()
        .context("FFmpeg is missing.")?;
    let duration = job.duration_ms.context("This job has no media duration.")?;
    ensure!(
        duration <= 30 * 60 * 1000,
        "Automatic speaker identification supports recordings up to 30 minutes."
    );
    let budget = state
        .hardware
        .live()
        .available_mib
        .saturating_sub(384)
        .min(2048);
    ensure!(
        budget >= 512,
        "Not enough memory to run speaker identification."
    );
    let scratch = tempfile::tempdir_in(&state.scratch)?;
    let wave = scratch.path().join("diarization.wav");
    media::pcm(
        ffmpeg,
        &state.source(job),
        &wave,
        0,
        duration,
        &state.shutdown,
        budget,
    )
    .await?;
    let mut command = subprocess::command(&engine.command);
    command
        .arg(parameter(
            "--segmentation.pyannote-model=",
            &engine.segmentation,
        ))
        .arg(parameter("--embedding.model=", &engine.embedding))
        .args([
            "--segmentation.num-threads=2",
            "--embedding.num-threads=2",
            "--clustering.cluster-threshold=0.8",
        ]);
    if let Some(count) = speaker_count {
        command.arg(format!("--clustering.num-clusters={count}"));
    }
    command.arg(&wave);
    let (result, _) = subprocess::run(
        command,
        &state.shutdown,
        budget,
        Duration::from_secs(600),
        512 * 1024,
        "Speaker identification",
    )
    .await?;
    let spans: Vec<_> = String::from_utf8(result)?
        .lines()
        .filter_map(parse)
        .collect();
    ensure!(
        !spans.is_empty(),
        "The diarization engine returned no speaker turns."
    );
    Ok(segments
        .iter()
        .filter_map(|segment| {
            let overlap = |(from, to, _): &(i64, i64, u16)| {
                (segment.end_ms.min(*to) - segment.start_ms.max(*from)).max(0)
            };
            let span = spans.iter().max_by_key(|span| overlap(span))?;
            if overlap(span) == 0 {
                return None;
            }
            Some(Suggestion {
                id: segment.id,
                text: segment.text.clone(),
                speaker: Some(format!("{label} {}", span.2 + 1)),
            })
        })
        .collect())
}

fn parse(line: &str) -> Option<(i64, i64, u16)> {
    let mut fields = line.split_whitespace();
    let to_ms = |value: &str| {
        value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .map(|value| (value * 1000.0).round() as i64)
    };
    let start = to_ms(fields.next()?)?;
    if fields.next()? != "--" {
        return None;
    }
    let end = to_ms(fields.next()?)?;
    let speaker = fields
        .next()?
        .strip_prefix("speaker_")?
        .parse::<u16>()
        .ok()?;
    (start < end && speaker < 1000).then_some((start, end, speaker))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_timed_speaker_turns() {
        assert_eq!(parse("1.250 -- 4.191 speaker_01"), Some((1_250, 4_191, 1)));
        assert_eq!(parse("3.000 -- 2.000 speaker_01"), None);
        assert_eq!(parse("Started"), None);
    }
}
