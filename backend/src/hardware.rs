use crate::{
    domain::{Plan, Profile, Settings},
    models::{CATALOG, CatalogModel},
};
use serde::Serialize;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use sysinfo::System;

const MIB: u64 = 1024 * 1024;

#[derive(Clone, Serialize)]
pub struct Trial {
    pub threads: usize,
    pub million_ops_per_second: f64,
}

#[derive(Clone, Serialize)]
pub struct Hardware {
    pub cpu: String,
    pub arch: &'static str,
    pub logical: usize,
    pub physical: Option<usize>,
    pub ram_mib: u64,
    pub available_mib: u64,
    pub adapters: Vec<String>,
    pub avx2: bool,
    pub engine: String,
    pub calibration: Vec<Trial>,
}

impl Hardware {
    pub fn inspect(engine: String) -> Self {
        let mut system = System::new();
        system.refresh_cpu_list(sysinfo::CpuRefreshKind::nothing());
        system.refresh_memory();
        let cpu = system
            .cpus()
            .first()
            .map(|cpu| cpu.brand())
            .filter(|name| !name.is_empty())
            .unwrap_or("Unknown CPU");
        let logical = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1);
        let physical = System::physical_core_count().filter(|n| *n > 0);
        let limit = physical.unwrap_or(logical).min(logical).clamp(1, 8);
        let mut candidates = vec![1, 2.min(limit), 4.min(limit), limit];
        candidates.dedup();
        Self {
            cpu: cpu.trim().into(),
            arch: std::env::consts::ARCH,
            logical,
            physical,
            ram_mib: system.total_memory() / MIB,
            available_mib: system.available_memory() / MIB,
            adapters: adapters(),
            avx2: supports_optimized_cpu(),
            engine,
            calibration: candidates.into_iter().map(calibrate).collect(),
        }
    }

    pub fn live(&self) -> Self {
        let mut result = self.clone();
        let mut system = System::new();
        system.refresh_memory();
        result.available_mib = system.available_memory() / MIB;
        result
    }

    pub fn budget(&self, settings: &Settings) -> u64 {
        let headroom = self.available_mib.saturating_sub(384);
        let auto = (self.ram_mib / 3).clamp(512, 4096);
        headroom.min(if settings.memory_mib == 0 {
            auto
        } else {
            settings.memory_mib
        })
    }

    pub fn plan(
        &self,
        profile: Profile,
        model: Option<&CatalogModel>,
        settings: &Settings,
    ) -> Option<Plan> {
        let budget = self.budget(settings);
        let fastest = self
            .calibration
            .iter()
            .map(|trial| trial.million_ops_per_second)
            .fold(0.0, f64::max);
        let single = self.calibration.first()?.million_ops_per_second.max(0.1);
        let scaling = fastest / single;
        let (preferred, cap, chunk_ms, beam_size) = match profile {
            Profile::Eco => (0, 2, 30_000, 1),
            Profile::Balanced => (
                usize::from(self.logical >= 3 && scaling >= 1.15),
                4,
                60_000,
                3,
            ),
            Profile::Precision => (2, 8, 90_000, 5),
        };
        let model = model.or_else(|| {
            CATALOG[..=preferred]
                .iter()
                .rev()
                .find(|model| model.estimated_mib <= budget)
        })?;
        if model.estimated_mib > budget || model.vad {
            return None;
        }
        let eligible: Vec<_> = self
            .calibration
            .iter()
            .filter(|trial| trial.threads <= cap)
            .collect();
        let best = eligible
            .iter()
            .map(|trial| trial.million_ops_per_second)
            .fold(0.0, f64::max);
        let threads = eligible
            .iter()
            .find(|trial| trial.million_ops_per_second >= best * 0.86)?
            .threads;
        let workers = if self.logical >= 8
            && scaling >= 2.5
            && budget >= model.estimated_mib * 2
            && threads * 2 <= self.logical
        {
            settings.max_workers.min(2)
        } else {
            1
        };
        Some(Plan {
            profile,
            model: model.id.into(),
            quantization: "Q5_1".into(),
            threads,
            chunk_ms,
            memory_mib: budget.min(model.estimated_mib * 2),
            estimated_mib: model.estimated_mib,
            workers,
            beam_size,
            engine: self.engine.clone(),
        })
    }
}

fn calibrate(threads: usize) -> Trial {
    const N: usize = 4096;
    let signal: Vec<f32> = (0..N)
        .map(|i| ((i * 17 % 997) as f32 - 498.0) / 498.0)
        .collect();
    let elapsed = Instant::now();
    let stop = elapsed + Duration::from_millis(90);
    let operations: u64 = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|offset| {
                let signal = &signal;
                scope.spawn(move || {
                    let mut repetitions = 0;
                    while Instant::now() < stop {
                        let total = signal
                            .iter()
                            .zip(signal[offset..].iter().cycle())
                            .fold(0.0, |sum, (a, b)| sum + a * b);
                        std::hint::black_box(total);
                        repetitions += 1;
                    }
                    repetitions * N as u64 * 2
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap_or_default())
            .sum()
    });
    Trial {
        threads,
        million_ops_per_second: operations as f64 / elapsed.elapsed().as_secs_f64() / 1_000_000.0,
    }
}

pub fn supports_optimized_cpu() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        is_x86_feature_detected!("avx")
            && is_x86_feature_detected!("avx2")
            && is_x86_feature_detected!("sse4.2")
            && is_x86_feature_detected!("bmi2")
            && is_x86_feature_detected!("fma")
            && is_x86_feature_detected!("f16c")
    }
    #[cfg(not(target_arch = "x86_64"))]
    false
}

pub fn program(name: &str, env_var: &str) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(env_var) {
        return which::which(path).ok();
    }
    let sibling = std::env::current_exe()
        .ok()?
        .parent()?
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    if sibling.is_file() {
        Some(sibling)
    } else {
        which::which(name).ok()
    }
}

fn adapters() -> Vec<String> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(path) = which::which("lspci")
            && let Ok(output) = std::process::Command::new(path).arg("-mm").output()
        {
            return String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| {
                    let fields: Vec<_> = line.split('"').collect();
                    let kind = *fields.get(1)?;
                    (kind.contains("VGA")
                        || kind.contains("Display")
                        || kind.contains("3D controller"))
                    .then(|| format!("{} {}", fields[3], fields[5]))
                })
                .collect();
        }
        std::fs::read_dir("/sys/class/drm")
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name();
                let suffix = name.to_str()?.strip_prefix("card")?;
                if !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
                    return None;
                }
                let device = entry.path().join("device");
                Some(format!(
                    "PCI {} {}",
                    std::fs::read_to_string(device.join("vendor")).ok()?.trim(),
                    std::fs::read_to_string(device.join("device")).ok()?.trim()
                ))
            })
            .collect()
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let output = std::process::Command::new("powershell.exe")
            .creation_flags(0x08000000)
            .args(["-NoProfile", "-NonInteractive", "-Command",
                "[Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-CimInstance Win32_VideoController -OperationTimeoutSec 3 | ForEach-Object Name"])
            .output();
        output
            .ok()
            .filter(|result| result.status.success())
            .map(|result| {
                String::from_utf8_lossy(&result.stdout)
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precision_falls_back_when_small_does_not_fit() {
        let hardware = Hardware {
            cpu: "test".into(),
            arch: "x86_64",
            logical: 4,
            physical: Some(2),
            ram_mib: 4096,
            available_mib: 1500,
            adapters: vec![],
            avx2: false,
            engine: "CPU".into(),
            calibration: vec![
                Trial {
                    threads: 1,
                    million_ops_per_second: 10.0,
                },
                Trial {
                    threads: 2,
                    million_ops_per_second: 18.0,
                },
            ],
        };
        let plan = hardware
            .plan(Profile::Precision, None, &Settings::default())
            .unwrap();
        assert_eq!(plan.model, "base-q5_1");
        assert_eq!(plan.threads, 2);
        assert_eq!(plan.workers, 1);
        assert!(
            hardware
                .plan(Profile::Eco, Some(&CATALOG[2]), &Settings::default())
                .is_none()
        );
    }
}
