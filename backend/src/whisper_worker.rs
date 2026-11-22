use crate::domain::Metrics;
use anyhow::{Context, Result, anyhow, ensure};
use std::{
    process::Stdio,
    time::{Duration, Instant},
};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, ChildStdin, ChildStdout, Command},
};
use tokio_util::sync::CancellationToken;

const MIB: u64 = 1024 * 1024;
const SAMPLE_INTERVAL: Duration = Duration::from_millis(75);
pub const INFERENCE_TIMEOUT: Duration = Duration::from_secs(1800);

pub struct Worker {
    child: Child,
    input: ChildStdin,
    output: ChildStdout,
    pid: Pid,
    system: System,
    previous_cpu_ms: u64,
}

impl Worker {
    pub fn start(mut command: Command) -> Result<Self> {
        let mut child = command
            .arg("--localscribe-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let pid = Pid::from_u32(child.id().context("Could not track Whisper worker.")?);
        let input = child
            .stdin
            .take()
            .context("Missing Whisper worker input.")?;
        let output = child
            .stdout
            .take()
            .context("Missing Whisper worker output.")?;
        Ok(Self {
            child,
            input,
            output,
            pid,
            system: System::new(),
            previous_cpu_ms: 0,
        })
    }

    pub fn resident_mib(&mut self) -> u64 {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[self.pid]),
            true,
            ProcessRefreshKind::nothing().with_memory(),
        );
        self.system
            .process(self.pid)
            .map_or(0, |process| process.memory().div_ceil(MIB))
    }

    pub async fn stop(mut self) {
        let _ = self.child.kill().await;
    }

    pub async fn infer(
        &mut self,
        cancellation: &CancellationToken,
        memory_mib: u64,
    ) -> Result<Metrics> {
        let started = Instant::now();
        let mut maximum_rss = 0;
        let mut cpu_ms = self.previous_cpu_ms;
        let mut interval = tokio::time::interval(SAMPLE_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let exchange = async {
            self.input.write_all(b"R").await?;
            let response = self.output.read_u8().await?;
            ensure!(
                response == b'R',
                "Whisper worker returned an invalid result."
            );
            Ok(())
        };
        tokio::pin!(exchange);
        let deadline = tokio::time::sleep(INFERENCE_TIMEOUT);
        tokio::pin!(deadline);
        loop {
            tokio::select! {
                biased;
                _ = cancellation.cancelled() => return Err(anyhow!("Cancelled.")),
                _ = &mut deadline => return Err(anyhow!("Whisper worker exceeded its time limit.")),
                _ = interval.tick() => {
                    self.system.refresh_processes_specifics(
                        ProcessesToUpdate::Some(&[self.pid]), true,
                        ProcessRefreshKind::nothing().with_memory().with_cpu(),
                    );
                    if let Some(process) = self.system.process(self.pid) {
                        maximum_rss = maximum_rss.max(process.memory());
                        cpu_ms = cpu_ms.max(process.accumulated_cpu_time());
                        ensure!(memory_mib == 0 || process.memory() <= memory_mib.saturating_mul(MIB),
                            "Whisper worker reached the memory limit.");
                    }
                }
                result = &mut exchange => {
                    result?;
                    break;
                }
            }
        }
        let window_cpu = cpu_ms.saturating_sub(self.previous_cpu_ms);
        self.previous_cpu_ms = cpu_ms;
        Ok(Metrics {
            wall_ms: started.elapsed().as_millis() as u64,
            peak_rss_bytes: maximum_rss,
            cpu_ms: window_cpu,
            ..Metrics::default()
        })
    }
}
