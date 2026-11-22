PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA secure_delete = ON;
CREATE TABLE IF NOT EXISTS jobs (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    extension TEXT NOT NULL,
    bytes INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'queued',
    created_at INTEGER NOT NULL DEFAULT (1000 * unixepoch()),
    duration_ms INTEGER,
    processed_ms INTEGER NOT NULL DEFAULT 0,
    detected_language TEXT,
    error TEXT,
    revision INTEGER NOT NULL DEFAULT 0,
    options TEXT NOT NULL,
    plan TEXT NOT NULL,
    metrics TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS jobs_status ON jobs(status,sequence);
CREATE TABLE IF NOT EXISTS segments (
    id INTEGER PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL,
    text TEXT NOT NULL,
    original TEXT NOT NULL,
    speaker TEXT
);
CREATE INDEX IF NOT EXISTS segments_job ON segments(job_id,start_ms,id);
CREATE TABLE IF NOT EXISTS segment_original_timing (
    segment_id INTEGER PRIMARY KEY REFERENCES segments(id) ON DELETE CASCADE,
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
PRAGMA user_version = 1;
