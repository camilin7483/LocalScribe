use crate::domain::{
    Job, JobOptions, Metrics, Plan, Segment, SegmentEdit, Settings, SettingsChange, Suggestion,
};
use anyhow::{Result, ensure};
use rusqlite::{Connection, OptionalExtension, Row, params, types::Type};
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};

pub struct Database(Mutex<Connection>);

const FIELDS: &str = "id,name,extension,bytes,status,created_at,duration_ms,processed_ms,detected_language,error,revision,options,plan,metrics";

fn decode<T: serde::de::DeserializeOwned>(row: &Row<'_>, index: usize) -> rusqlite::Result<T> {
    serde_json::from_str(&row.get::<_, String>(index)?).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })
}

fn job(row: &Row<'_>) -> rusqlite::Result<Job> {
    let options: JobOptions = decode(row, 11)?;
    Ok(Job {
        id: row.get(0)?,
        name: row.get(1)?,
        file_extension: row.get(2)?,
        bytes: row.get(3)?,
        status: row.get(4)?,
        created_at: row.get(5)?,
        duration_ms: row.get(6)?,
        processed_ms: row.get(7)?,
        detected_language: row.get(8)?,
        error: row.get(9)?,
        revision: row.get(10)?,
        vad: options.vad,
        benchmark: options.benchmark,
        options,
        plan: decode(row, 12)?,
        metrics: decode(row, 13)?,
    })
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(include_str!("schema.sql"))?;
        Ok(Self(Mutex::new(conn)))
    }

    fn conn(&self) -> Result<MutexGuard<'_, Connection>> {
        self.0
            .lock()
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))
    }

    pub fn recover(&self) -> Result<()> {
        self.conn()?.execute(
            "UPDATE jobs SET status='interrupted' WHERE status='running'",
            [],
        )?;
        Ok(())
    }

    pub fn settings(&self) -> Result<Settings> {
        let conn = self.conn()?;
        read_settings(&conn)
    }

    pub fn update_settings(&self, change: SettingsChange) -> Result<Settings> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let mut settings = read_settings(&tx)?;
        if let Some(memory_mib) = change.memory_mib {
            settings.memory_mib = memory_mib;
        }
        if let Some(max_workers) = change.max_workers {
            settings.max_workers = max_workers;
        }
        if let Some(enabled) = change.ollama_enabled {
            settings.ollama_enabled = enabled;
        }
        tx.execute(
            "INSERT INTO settings(key,value) VALUES('preferences',?1)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [serde_json::to_string(&settings)?],
        )?;
        tx.commit()?;
        Ok(settings)
    }

    pub fn insert(
        &self,
        id: &str,
        name: &str,
        extension: &str,
        bytes: i64,
        options: &JobOptions,
        plan: &Plan,
    ) -> Result<Job> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO jobs(id,name,extension,bytes,options,plan,metrics) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![id,name,extension,bytes,serde_json::to_string(options)?,serde_json::to_string(plan)?,serde_json::to_string(&Metrics::default())?],
        )?;
        Ok(conn.query_row(&format!("SELECT {FIELDS} FROM jobs WHERE id=?1"), [id], job)?)
    }

    pub fn get(&self, id: &str) -> Result<Option<Job>> {
        Ok(self
            .conn()?
            .query_row(&format!("SELECT {FIELDS} FROM jobs WHERE id=?1"), [id], job)
            .optional()?)
    }

    pub fn list(&self, before: i64, filter: &str) -> Result<Vec<Job>> {
        let conn = self.conn()?;
        let predicate = match filter {
            "queue" => "AND status IN ('queued','running','cancelled','interrupted','failed')",
            "history" => "AND status IN ('completed','cancelled','interrupted','failed')",
            "benchmarks" => "AND json_extract(options,'$.benchmark')=1",
            _ => "",
        };
        let mut statement = conn.prepare(&format!(
            "SELECT {FIELDS} FROM jobs WHERE sequence < ?1 {predicate} ORDER BY sequence DESC LIMIT 60"
        ))?;
        Ok(statement
            .query_map([before], job)?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn cursor(&self, id: &str) -> Result<Option<i64>> {
        Ok(self
            .conn()?
            .query_row("SELECT sequence FROM jobs WHERE id=?1", [id], |row| {
                row.get(0)
            })
            .optional()?)
    }

    pub fn counts(&self) -> Result<[i64; 3]> {
        let counts = self.conn()?.query_row(
            "SELECT count(*), count(CASE WHEN status IN ('queued','running') THEN 1 END),
             count(CASE WHEN status='completed' THEN 1 END) FROM jobs",
            [],
            |row| Ok([row.get(0)?, row.get(1)?, row.get(2)?]),
        )?;
        Ok(counts)
    }

    pub fn queued(&self) -> Result<Vec<Job>> {
        let conn = self.conn()?;
        let mut statement = conn.prepare(&format!(
            "SELECT {FIELDS} FROM jobs WHERE status='queued' ORDER BY sequence LIMIT 16"
        ))?;
        Ok(statement
            .query_map([], job)?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn set_running(&self, id: &str) -> Result<bool> {
        Ok(self.conn()?.execute(
            "UPDATE jobs SET status='running',error=NULL WHERE id=?1 AND status='queued'",
            [id],
        )? == 1)
    }

    pub fn set_duration(&self, id: &str, duration: i64) -> Result<()> {
        self.conn()?.execute(
            "UPDATE jobs SET duration_ms=?2 WHERE id=?1 AND status='running'",
            params![id, duration],
        )?;
        Ok(())
    }

    pub fn fail(&self, id: &str, error: &str) -> Result<()> {
        self.conn()?.execute(
            "UPDATE jobs SET status='failed',error=?2 WHERE id=?1 AND status='running'",
            params![id, error],
        )?;
        Ok(())
    }

    pub fn cancel(&self, id: &str) -> Result<bool> {
        Ok(self.conn()?.execute("UPDATE jobs SET status='cancelled',error=NULL WHERE id=?1 AND status IN ('running','queued')", [id])? == 1)
    }

    pub fn resume(&self, id: &str) -> Result<bool> {
        Ok(self.conn()?.execute("UPDATE jobs SET status='queued',error=NULL WHERE id=?1 AND status IN ('failed','cancelled','interrupted')", [id])? == 1)
    }

    pub fn checkpoint(
        &self,
        id: &str,
        segments: &[Segment],
        end: i64,
        language: Option<&str>,
        metrics: &Metrics,
    ) -> Result<bool> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let changed = tx.execute(
            "UPDATE jobs SET processed_ms=?2,detected_language=coalesce(detected_language,?3),metrics=?4
             WHERE id=?1 AND status='running' AND processed_ms < ?2",
            params![id,end,language,serde_json::to_string(metrics)?],
        )?;
        if changed == 0 {
            return Ok(false);
        }
        {
            let mut insert = tx.prepare(
                "INSERT INTO segments(job_id,start_ms,end_ms,text,original) VALUES(?1,?2,?3,?4,?4)",
            )?;
            for segment in segments {
                insert.execute(params![id, segment.start_ms, segment.end_ms, segment.text])?;
            }
        }
        tx.commit()?;
        Ok(true)
    }

    pub fn finish(&self, id: &str, metrics: &Metrics) -> Result<()> {
        self.conn()?.execute(
            "UPDATE jobs SET status='completed',metrics=?2 WHERE id=?1 AND status='running'",
            params![id, serde_json::to_string(metrics)?],
        )?;
        Ok(())
    }

    pub fn snapshot(&self, id: &str) -> Result<Option<(Job, Vec<Segment>)>> {
        let conn = self.conn()?;
        let job = conn
            .query_row(&format!("SELECT {FIELDS} FROM jobs WHERE id=?1"), [id], job)
            .optional()?;
        job.map(|job| Ok((job, read_segments(&conn, id)?)))
            .transpose()
    }

    pub fn segments(&self, id: &str) -> Result<Vec<Segment>> {
        let conn = self.conn()?;
        read_segments(&conn, id)
    }

    pub fn edit(&self, id: &str, edit: &SegmentEdit) -> Result<Option<i64>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let updated = tx.execute(
            "UPDATE jobs SET revision=revision+1 WHERE id=?1 AND revision=?2 AND status='completed'",
            params![id, edit.revision],
        )?;
        if updated == 0 {
            return Ok(None);
        }
        let changed = tx.execute(
            "UPDATE segments SET text=?3,speaker=?4 WHERE job_id=?1 AND id=?2",
            params![id, edit.id, edit.text, edit.speaker],
        )?;
        ensure!(changed == 1, "segment does not exist");
        tx.commit()?;
        Ok(Some(edit.revision + 1))
    }

    pub fn apply(&self, id: &str, revision: i64, changes: &[Suggestion]) -> Result<Option<i64>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let updated = tx.execute(
            "UPDATE jobs SET revision=revision+1 WHERE id=?1 AND revision=?2 AND status='completed'",
            params![id, revision],
        )?;
        if updated == 0 {
            return Ok(None);
        }
        for change in changes {
            let updated = tx.execute(
                "UPDATE segments SET text=?3,speaker=coalesce(?4,speaker) WHERE job_id=?1 AND id=?2",
                params![id, change.id, change.text, change.speaker],
            )?;
            if updated == 0 {
                return Ok(None);
            }
        }
        tx.commit()?;
        Ok(Some(revision + 1))
    }

    pub fn delete(&self, id: &str) -> Result<bool> {
        Ok(self.conn()?.execute(
            "DELETE FROM jobs WHERE id=?1 AND status NOT IN ('queued','running')",
            [id],
        )? == 1)
    }
}

fn read_settings(conn: &Connection) -> Result<Settings> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key='preferences'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    Ok(value
        .map(|value| serde_json::from_str(&value))
        .transpose()?
        .unwrap_or_default())
}

fn read_segments(conn: &Connection, id: &str) -> Result<Vec<Segment>> {
    let mut statement = conn.prepare("SELECT id,start_ms,end_ms,text,original,speaker FROM segments WHERE job_id=?1 ORDER BY start_ms,id")?;
    Ok(statement
        .query_map([id], |row| {
            Ok(Segment {
                id: row.get(0)?,
                start_ms: row.get(1)?,
                end_ms: row.get(2)?,
                text: row.get(3)?,
                original: row.get(4)?,
                speaker: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
}
