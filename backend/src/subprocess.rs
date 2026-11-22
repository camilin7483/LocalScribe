use crate::domain::Metrics;
use anyhow::{Context, Result, anyhow, ensure};
use std::{
    path::Path,
    process::Stdio,
    time::{Duration, Instant},
};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::{io::AsyncReadExt, process::Command};
use tokio_util::sync::CancellationToken;

pub fn command(path: &Path) -> Command {
    let mut process = Command::new(path);
    process
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(target_os = "windows")]
    process.creation_flags(0x08000000);
    process
}

pub async fn run(
    mut command: Command,
    cancellation: &CancellationToken,
    memory_mib: u64,
    timeout: Duration,
    stdout_limit: usize,
    stage: &'static str,
) -> Result<(Vec<u8>, Metrics)> {
    let start = Instant::now();
    command.stdout(if stdout_limit == 0 {
        Stdio::null()
    } else {
        Stdio::piped()
    });
    let mut child = command
        .spawn()
        .with_context(|| format!("{stage} could not start."))?;
    let pid = Pid::from_u32(child.id().context("Could not track process.")?);
    let stdout = child.stdout.take().map(|pipe| {
        tokio::spawn(async move {
            let mut buffer = Vec::new();
            pipe.take(stdout_limit as u64 + 1)
                .read_to_end(&mut buffer)
                .await?;
            Ok::<_, std::io::Error>(buffer)
        })
    });
    let mut system = System::new();
    let mut metrics = Metrics::default();
    let mut samples = tokio::time::interval(Duration::from_millis(75));
    samples.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);
    let status = loop {
        tokio::select! {
            biased;
            _ = cancellation.cancelled() => break Err(anyhow!("Cancelled.")),
            _ = &mut deadline => break Err(anyhow!("{stage} exceeded its time limit.")),
            _ = samples.tick() => {
                system.refresh_processes_specifics(
                    ProcessesToUpdate::Some(&[pid]), true,
                    ProcessRefreshKind::nothing().with_memory().with_cpu(),
                );
                if let Some(process) = system.process(pid) {
                    metrics.peak_rss_bytes = metrics.peak_rss_bytes.max(process.memory());
                    metrics.cpu_ms = metrics.cpu_ms.max(process.accumulated_cpu_time());
                    if memory_mib > 0 && process.memory() > memory_mib * 1024 * 1024 {
                        break Err(anyhow!("{stage} reached the memory limit. Increase the limit or select a smaller model."));
                    }
                }
            }
            status = child.wait() => break status.context("Process did not exit cleanly."),
        }
    };
    let status = match status {
        Ok(status) => status,
        Err(error) => {
            let _ = child.kill().await;
            if let Some(reader) = stdout {
                reader.abort();
            }
            return Err(error);
        }
    };
    metrics.wall_ms = start.elapsed().as_millis() as u64;
    ensure!(
        status.success(),
        "{stage} failed. Check the file, model, and runtime versions."
    );
    let output = match stdout {
        Some(reader) => reader.await??,
        None => Vec::new(),
    };
    ensure!(
        output.len() <= stdout_limit,
        "{stage} returned too much data."
    );
    Ok((output, metrics))
}
