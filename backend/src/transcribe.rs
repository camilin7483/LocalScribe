use crate::{
    State,
    domain::{Job, Segment},
    media, models, subprocess,
};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{path::Path, time::Duration};
use tokio_util::sync::CancellationToken;

const OVERLAP_MS: i64 = 1_200;
const MAX_JSON: u64 = 4 * 1024 * 1024;

#[derive(Deserialize)]
struct WhisperOutput {
    result: WhisperResult,
    transcription: Vec<WhisperSegment>,
}

#[derive(Deserialize)]
struct WhisperResult {
    language: String,
}

#[derive(Deserialize)]
struct WhisperSegment {
    offsets: Offsets,
    text: String,
}

#[derive(Deserialize)]
struct Offsets {
    from: i64,
    to: i64,
}

pub async fn work(state: &State, job: Job, cancellation: &CancellationToken) -> Result<()> {
    let ffmpeg = state
        .programs
        .ffmpeg
        .as_deref()
        .context("FFmpeg is missing. Open System diagnostics.")?;
    let ffprobe = state
        .programs
        .ffprobe
        .as_deref()
        .context("FFprobe is missing. Open System diagnostics.")?;
    let engine = state
        .programs
        .whisper
        .as_deref()
        .context("whisper.cpp is missing. Open System diagnostics.")?;
    let model = models::find(&job.plan.model).context("The selected model is unavailable.")?;
    let model_path = state.models.path(model);
    let vad_model = models::find("silero").unwrap();
    let vad_path = state.models.path(vad_model);
    models::verify_file(&model_path, model).await?;
    if job.vad {
        models::verify_file(&vad_path, vad_model).await?;
    }

    let mut metrics = job.metrics.clone();
    let source = state.source(&job);
    let duration = match job.duration_ms {
        Some(duration) => duration,
        None => {
            let (duration, sample) = media::probe(ffprobe, &source, cancellation).await?;
            metrics.add(&sample);
            state.db.set_duration(&job.id, duration)?;
            duration
        }
    };
    let temporary = tempfile::tempdir_in(&state.scratch)?;
    let wav = temporary.path().join("window.wav");
    let prefix = temporary.path().join("result");
    let json_path = prefix.with_extension("json");
    let mut start = job.processed_ms;
    let mut language = job.detected_language.clone();
    while start < duration {
        let end = (start + job.plan.chunk_ms).min(duration);
        let input_from = (start - OVERLAP_MS).max(0);
        let input_end = (end + OVERLAP_MS).min(duration);
        let decoded = media::pcm(
            ffmpeg,
            &source,
            &wav,
            input_from,
            input_end - input_from,
            cancellation,
            job.plan.memory_mib,
        )
        .await?;
        metrics.add(&decoded);

        let (mut segments, mut detected) = (Vec::new(), None);
        if tokio::fs::metadata(&wav).await?.len() > 320 {
            let _ = tokio::fs::remove_file(&json_path).await;
            let mut command = subprocess::command(engine);
            command
                .args(["-ng", "-oj", "-of"])
                .arg(&prefix)
                .arg("-m")
                .arg(&model_path)
                .arg("-f")
                .arg(&wav)
                .arg("-l")
                .arg(&job.options.language)
                .arg("-t")
                .arg(job.plan.threads.to_string())
                .arg("-bs")
                .arg(job.plan.beam_size.to_string())
                .args(["-bo", "1"]);
            if job.vad {
                command
                    .args(["--vad", "-vm"])
                    .arg(&vad_path)
                    .args(["-vsd", "300"]);
            }
            let (_, inferred) = subprocess::run(
                command,
                cancellation,
                job.plan.memory_mib,
                Duration::from_secs(1800),
                0,
                "Whisper inference",
            )
            .await?;
            metrics.add(&inferred);
            let parsed = read_output(&json_path).await?;
            segments = to_segments(parsed.transcription, input_from, start, end, duration);
            if !segments.is_empty() && !parsed.result.language.is_empty() {
                detected = Some(parsed.result.language);
            }
        }
        if language.is_none() {
            language.clone_from(&detected);
        }
        if cancellation.is_cancelled()
            || !state
                .db
                .checkpoint(&job.id, &segments, end, language.as_deref(), &metrics)?
        {
            return Ok(());
        }
        start = end;
    }
    if let Some(reference) = job.options.reference {
        let hypothesis = state
            .db
            .segments(&job.id)?
            .into_iter()
            .map(|segment| segment.original)
            .collect::<Vec<_>>()
            .join(" ");
        metrics.wer =
            tokio::task::spawn_blocking(move || crate::wer::rate(&reference, &hypothesis)).await?;
    }
    state.db.finish(&job.id, &metrics)?;
    Ok(())
}

async fn read_output(path: &Path) -> Result<WhisperOutput> {
    let size = tokio::fs::metadata(path)
        .await
        .context("Whisper did not produce a transcript file.")?
        .len();
    ensure!(
        size <= MAX_JSON,
        "Whisper returned too many segments in a window."
    );
    serde_json::from_slice(&tokio::fs::read(path).await?)
        .context("Whisper returned an invalid transcript file.")
}

fn to_segments(
    raw: Vec<WhisperSegment>,
    input_from: i64,
    from: i64,
    to: i64,
    duration: i64,
) -> Vec<Segment> {
    raw.into_iter()
        .filter_map(|segment| {
            let text = segment.text.trim();
            let start = input_from
                .saturating_add(segment.offsets.from)
                .clamp(0, duration);
            let end = input_from
                .saturating_add(segment.offsets.to)
                .clamp(start, duration);
            let midpoint = start + (end - start) / 2;
            if text.is_empty()
                || end <= start
                || midpoint < from
                || (midpoint >= to && to != duration)
            {
                return None;
            }
            Some(Segment {
                id: 0,
                start_ms: start,
                end_ms: end,
                text: text.into(),
                original: text.into(),
                speaker: None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlap_uses_global_timestamps_and_window_ownership() {
        let raw = vec![
            WhisperSegment {
                offsets: Offsets { from: 100, to: 550 },
                text: " previous ".into(),
            },
            WhisperSegment {
                offsets: Offsets {
                    from: 1_300,
                    to: 3_500,
                },
                text: " current ".into(),
            },
        ];
        let segments = to_segments(raw, 28_800, 30_000, 60_000, 90_000);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].start_ms, 30_100);
        assert_eq!(segments[0].original, "current");
    }
}
