use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    Eco,
    #[default]
    Balanced,
    Precision,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct JobOptions {
    pub language: String,
    pub profile: Profile,
    pub model: Option<String>,
    pub vad: bool,
    pub benchmark: bool,
    pub reference: Option<String>,
}

impl Default for JobOptions {
    fn default() -> Self {
        Self {
            language: "auto".into(),
            profile: Profile::Balanced,
            model: None,
            vad: true,
            benchmark: false,
            reference: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Plan {
    pub profile: Profile,
    pub model: String,
    pub quantization: String,
    pub threads: usize,
    pub chunk_ms: i64,
    pub memory_mib: u64,
    pub estimated_mib: u64,
    pub workers: usize,
    pub beam_size: usize,
    pub engine: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Segment {
    pub id: i64,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub original: String,
    pub speaker: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SegmentEdit {
    pub id: i64,
    pub text: String,
    pub speaker: Option<String>,
    pub revision: i64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Metrics {
    pub wall_ms: u64,
    pub peak_rss_bytes: u64,
    pub cpu_ms: u64,
    pub wer: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stages: Option<StageMetrics>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct StageMetrics {
    pub probe_ms: u64,
    pub silence_scan_ms: u64,
    pub silent_windows: u64,
    pub decode_ms: u64,
    pub inference_ms: u64,
    pub persistent_windows: u64,
    pub cli_windows: u64,
    pub worker_fallbacks: u64,
    pub fallback_ms: u64,
    pub peak_child_tree_estimate_bytes: u64,
}

impl Metrics {
    pub fn add(&mut self, other: &Self) {
        self.wall_ms += other.wall_ms;
        self.cpu_ms += other.cpu_ms;
        self.peak_rss_bytes = self.peak_rss_bytes.max(other.peak_rss_bytes);
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub bytes: i64,
    pub status: String,
    pub created_at: i64,
    pub duration_ms: Option<i64>,
    pub processed_ms: i64,
    pub detected_language: Option<String>,
    pub error: Option<String>,
    pub revision: i64,
    pub plan: Plan,
    pub metrics: Metrics,
    pub vad: bool,
    pub benchmark: bool,
    #[serde(skip_serializing)]
    pub file_extension: String,
    #[serde(skip_serializing)]
    pub options: JobOptions,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub memory_mib: u64,
    pub max_workers: usize,
    pub ollama_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            memory_mib: 0,
            max_workers: 2,
            ollama_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Suggestion {
    pub id: i64,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[derive(Default)]
pub struct SettingsChange {
    pub memory_mib: Option<u64>,
    pub max_workers: Option<usize>,
    pub ollama_enabled: Option<bool>,
}
