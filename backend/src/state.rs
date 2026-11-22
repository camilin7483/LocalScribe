use crate::{db::Database, domain::Job, hardware::Hardware, media::Programs, models::Models};
use std::{
    collections::HashMap,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::sync::{Notify, Semaphore};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct State(Arc<Runtime>);

pub struct Runtime {
    pub db: Database,
    pub programs: Programs,
    pub hardware: Hardware,
    pub models: Models,
    pub media: PathBuf,
    pub scratch: PathBuf,
    pub authority: String,
    pub upload_limit: usize,
    pub uploads: Semaphore,
    pub auxiliary: Semaphore,
    pub wake: Notify,
    pub active: Mutex<HashMap<String, (CancellationToken, u64, usize)>>,
    pub shutdown: CancellationToken,
}

impl Deref for State {
    type Target = Runtime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl State {
    pub fn new(runtime: Runtime) -> Self {
        Self(Arc::new(runtime))
    }

    pub fn source(&self, job: &Job) -> PathBuf {
        self.media
            .join(format!("{}.{}", job.id, job.file_extension))
    }

    pub fn preview(&self, id: &str) -> PathBuf {
        self.media.join(format!("{id}.preview.mp3"))
    }

    pub fn clean_media(&self) -> anyhow::Result<()> {
        for entry in std::fs::read_dir(&self.media)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let Some((id, _)) = name.split_once('.') else {
                continue;
            };
            if ((id.is_empty() && name.starts_with(".tmp"))
                || (uuid::Uuid::parse_str(id).is_ok() && self.db.get(id)?.is_none()))
                && entry.file_type()?.is_file()
            {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        Ok(())
    }
}

pub fn private_directory(path: &Path) -> std::io::Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}
