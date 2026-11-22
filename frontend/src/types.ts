export type Profile = 'eco' | 'balanced' | 'precision'
export type Status = 'queued' | 'running' | 'completed' | 'failed' | 'interrupted' | 'cancelled'
export type Format = 'txt' | 'srt' | 'vtt' | 'json' | 'md'
export type View = 'home' | 'new' | 'library' | 'editor' | 'subtitles' | 'analysis' | 'queue' | 'history' | 'benchmarks' | 'hardware' | 'models' | 'privacy'
export type Plan = {
  profile: Profile; model: string; quantization: string; threads: number
  chunk_ms: number; memory_mib: number; estimated_mib: number; workers: number
  beam_size: number; engine: string
}
export type Settings = { memory_mib: number; max_workers: number; ollama_enabled: boolean }
export type Model = {
  id: string; label: string; size: number; installed: boolean; vad: boolean
  downloaded: number; downloading: boolean; error: string | null
}
export type Hardware = {
  cpu: string; arch: string; logical: number; physical: number | null; ram_mib: number; available_mib: number
  adapters: string[]; avx2: boolean; engine: string
  calibration: { threads: number; million_ops_per_second: number }[]
}
export type System = {
  version: string; plans: (Plan | null)[]; hardware: Hardware; settings: Settings; models: Model[]
  diagnostics: Record<'ffmpeg' | 'ffprobe' | 'whisper', boolean>; upload_limit: number
  counts: { files: number; completed: number; active: number }; languages: string[]
}
export type Metrics = { wall_ms: number; peak_rss_bytes: number; cpu_ms: number; wer: number | null }
export type Job = {
  id: string; name: string; bytes: number; status: Status; created_at: number; duration_ms: number | null
  processed_ms: number; detected_language: string | null; error: string | null
  revision: number; plan: Plan; metrics: Metrics; vad: boolean; benchmark: boolean
}
export type Jobs = { jobs: Job[]; cursor: number | null }
export type Segment = { id: number; start_ms: number; end_ms: number; text: string; original: string; speaker: string | null }
export type Detail = { job: Job; segments: Segment[]; original_playback: boolean; video: boolean; preview_ready: boolean }
export type Options = { language: string; profile: Profile; model: string | null; vad: boolean; benchmark: boolean; reference: string | null }
export type Insight = {
  title: string; text: string; evidence: Segment[]
  changes: { id: number; text: string; speaker?: string }[]
}
export type AnalyzeMode = 'general' | 'executive' | 'key_points' | 'decisions' | 'tasks' | 'questions' | 'translate' | 'correct' | 'ask' | 'speakers'
export type Engines = { enabled: boolean; models: string[]; diarization: boolean }
