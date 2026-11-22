use crate::{
    State,
    domain::{Job, Segment},
    media, models, subprocess,
    whisper_worker::{INFERENCE_TIMEOUT, Worker},
};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{path::Path, time::Instant};
use tokio::process::Command;
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
    let mut stages = metrics.stages.take().unwrap_or_default();
    stages.peak_child_tree_estimate_bytes = stages
        .peak_child_tree_estimate_bytes
        .max(metrics.peak_rss_bytes);
    let source = state.source(&job);
    let duration = match job.duration_ms {
        Some(duration) => duration,
        None => {
            let (duration, sample) = media::probe(ffprobe, &source, cancellation).await?;
            metrics.add(&sample);
            stages.probe_ms += sample.wall_ms;
            metrics.stages = Some(stages.clone());
            state.db.record_probe(&job.id, duration, &metrics)?;
            duration
        }
    };
    let temporary = tempfile::tempdir_in(&state.scratch)?;
    let wav = temporary.path().join("window.wav");
    let prefix = temporary.path().join("result");
    let json_path = prefix.with_extension("json");
    let mut start = job.processed_ms;
    let mut language = job.detected_language.clone();
    let mut use_worker = duration - start > job.plan.chunk_ms
        && std::env::var("LOCALSCRIBE_WHISPER_PERSISTENT").as_deref() == Ok("1");
    let mut worker: Option<Worker> = None;
    let make_command = || {
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
        command
    };
    while start < duration {
        let end = (start + job.plan.chunk_ms).min(duration);
        let input_from = (start - OVERLAP_MS).max(0);
        let input_end = (end + OVERLAP_MS).min(duration);
        let mut decoder_budget = job.plan.memory_mib;
        let mut worker_rss_mib = 0;
        if let Some(active) = worker.as_mut() {
            worker_rss_mib = active.resident_mib();
            decoder_budget = decoder_budget.saturating_sub(worker_rss_mib);
            if decoder_budget < 128 {
                worker.take().expect("active worker").stop().await;
                use_worker = false;
                stages.worker_fallbacks += 1;
                decoder_budget = job.plan.memory_mib;
                worker_rss_mib = 0;
            }
        }
        let decode_window = |budget| {
            media::pcm(
                ffmpeg,
                &source,
                &wav,
                input_from,
                input_end - input_from,
                cancellation,
                budget,
            )
        };
        let decode_started = Instant::now();
        let decoded = match decode_window(decoder_budget).await {
            Err(_) if worker.is_some() && !cancellation.is_cancelled() => {
                worker.take().expect("active worker").stop().await;
                use_worker = false;
                worker_rss_mib = 0;
                stages.worker_fallbacks += 1;
                stages.fallback_ms += decode_started.elapsed().as_millis() as u64;
                decode_window(job.plan.memory_mib).await?
            }
            outcome => outcome?,
        };
        metrics.add(&decoded);
        stages.decode_ms += decoded.wall_ms;
        stages.peak_child_tree_estimate_bytes = stages
            .peak_child_tree_estimate_bytes
            .max(worker_rss_mib * 1_048_576 + decoded.peak_rss_bytes);

        let (mut segments, mut detected) = (Vec::new(), None);
        let scan = Instant::now();
        let has_audio = tokio::fs::metadata(&wav).await?.len() > 320;
        let silent = has_audio && media::is_digital_silence(&wav).await?;
        stages.silence_scan_ms += scan.elapsed().as_millis() as u64;
        if silent {
            stages.silent_windows += 1;
        }
        if has_audio && !silent {
            let _ = tokio::fs::remove_file(&json_path).await;
            let mut shared = None;
            if use_worker {
                let attempted = Instant::now();
                let result = async {
                    if worker.is_none() {
                        worker = Some(Worker::start(make_command())?);
                    }
                    let mut sampled = worker
                        .as_mut()
                        .expect("started worker")
                        .infer(cancellation, job.plan.memory_mib)
                        .await?;
                    let output = read_output(&json_path).await?;
                    sampled.wall_ms = attempted.elapsed().as_millis() as u64;
                    Ok::<_, anyhow::Error>((output, sampled))
                }
                .await;
                if let Ok(result) = result {
                    stages.persistent_windows += 1;
                    shared = Some(result);
                } else {
                    if let Some(active) = worker.take() {
                        active.stop().await;
                    }
                    if cancellation.is_cancelled() {
                        return Ok(());
                    }
                    stages.worker_fallbacks += 1;
                    stages.fallback_ms += attempted.elapsed().as_millis() as u64;
                    use_worker = false;
                    let _ = tokio::fs::remove_file(&json_path).await;
                }
            }
            let (parsed, inferred) = if let Some(result) = shared {
                result
            } else {
                stages.cli_windows += 1;
                isolated_window(
                    make_command(),
                    cancellation,
                    job.plan.memory_mib,
                    &json_path,
                )
                .await?
            };
            metrics.add(&inferred);
            stages.inference_ms += inferred.wall_ms;
            segments = to_segments(parsed.transcription, input_from, start, end, duration);
            if !segments.is_empty() && !parsed.result.language.is_empty() {
                detected = Some(parsed.result.language);
            }
        }
        if language.is_none() {
            language.clone_from(&detected);
        }
        stages.peak_child_tree_estimate_bytes = stages
            .peak_child_tree_estimate_bytes
            .max(metrics.peak_rss_bytes);
        metrics.stages = Some(stages.clone());
        if cancellation.is_cancelled()
            || !state
                .db
                .checkpoint(&job.id, &segments, end, language.as_deref(), &metrics)?
        {
            return Ok(());
        }
        start = end;
    }
    if let Some(active) = worker.take() {
        active.stop().await;
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

async fn isolated_window(
    command: Command,
    cancellation: &CancellationToken,
    memory_mib: u64,
    json_path: &Path,
) -> Result<(WhisperOutput, crate::domain::Metrics)> {
    let (_, inferred) = subprocess::run(
        command,
        cancellation,
        memory_mib,
        INFERENCE_TIMEOUT,
        0,
        "Whisper inference",
    )
    .await?;
    Ok((read_output(json_path).await?, inferred))
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
                original_start_ms: start,
                original_end_ms: end,
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
