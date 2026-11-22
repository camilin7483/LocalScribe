use anyhow::{Context, Result, ensure};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
};

#[derive(Clone, Copy, Serialize)]
pub struct CatalogModel {
    pub id: &'static str,
    pub label: &'static str,
    pub file: &'static str,
    pub size: u64,
    pub estimated_mib: u64,
    pub vad: bool,
    #[serde(skip)]
    pub sha256: &'static str,
}

pub const CATALOG: [CatalogModel; 4] = [
    CatalogModel {
        id: "tiny-q5_1",
        label: "Whisper Tiny",
        file: "ggml-tiny-q5_1.bin",
        size: 32_152_673,
        estimated_mib: 512,
        vad: false,
        sha256: "818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7",
    },
    CatalogModel {
        id: "base-q5_1",
        label: "Whisper Base",
        file: "ggml-base-q5_1.bin",
        size: 59_707_625,
        estimated_mib: 720,
        vad: false,
        sha256: "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898",
    },
    CatalogModel {
        id: "small-q5_1",
        label: "Whisper Small",
        file: "ggml-small-q5_1.bin",
        size: 190_085_487,
        estimated_mib: 1550,
        vad: false,
        sha256: "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb",
    },
    CatalogModel {
        id: "silero",
        label: "Silero VAD 6.2",
        file: "ggml-silero-v6.2.0.bin",
        size: 885_098,
        estimated_mib: 0,
        vad: true,
        sha256: "2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987",
    },
];

pub fn find(id: &str) -> Option<&'static CatalogModel> {
    CATALOG.iter().find(|model| model.id == id)
}

#[derive(Clone, Default, Serialize)]
pub struct Download {
    pub downloaded: u64,
    pub downloading: bool,
    pub error: Option<&'static str>,
}

#[derive(Serialize)]
pub struct ModelStatus {
    #[serde(flatten)]
    pub model: CatalogModel,
    pub installed: bool,
    #[serde(flatten)]
    pub download: Download,
}

pub struct Models {
    directory: PathBuf,
    transfers: Mutex<HashMap<&'static str, Download>>,
}

impl Models {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            transfers: Mutex::new(HashMap::new()),
        }
    }

    pub fn path(&self, model: &CatalogModel) -> PathBuf {
        self.directory.join(model.file)
    }

    pub fn installed(&self, model: &CatalogModel) -> bool {
        self.path(model)
            .symlink_metadata()
            .is_ok_and(|meta| meta.is_file() && meta.len() == model.size)
    }

    pub fn status(&self) -> Vec<ModelStatus> {
        let transfers = self.transfers.lock().unwrap();
        CATALOG
            .iter()
            .map(|model| ModelStatus {
                model: *model,
                installed: self.installed(model),
                download: transfers.get(model.id).cloned().unwrap_or_default(),
            })
            .collect()
    }

    pub fn reserve(&self, model: &CatalogModel) -> bool {
        let mut transfers = self.transfers.lock().unwrap();
        if self.installed(model)
            || transfers
                .get(model.id)
                .is_some_and(|value| value.downloading)
            || transfers.values().filter(|value| value.downloading).count() >= 2
        {
            return false;
        }
        transfers.insert(
            model.id,
            Download {
                downloading: true,
                ..Default::default()
            },
        );
        true
    }

    pub fn remove(&self, model: &CatalogModel) -> Result<bool> {
        let mut transfers = self.transfers.lock().unwrap();
        if transfers
            .get(model.id)
            .is_some_and(|value| value.downloading)
        {
            return Ok(false);
        }
        match std::fs::remove_file(self.path(model)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        transfers.remove(model.id);
        Ok(true)
    }

    pub async fn download(&self, model: &'static CatalogModel) {
        let outcome = self.fetch(model).await;
        let mut transfers = self.transfers.lock().unwrap();
        let transfer = transfers.entry(model.id).or_default();
        transfer.downloading = false;
        transfer.error = outcome
            .err()
            .map(|_| "Model download failed. Check connectivity and free disk space, then retry.");
    }

    async fn fetch(&self, model: &'static CatalogModel) -> Result<()> {
        let (repository, revision) = if model.vad {
            (
                "ggml-org/whisper-vad",
                "9ffd54a1e1ee413ddf265af9913beaf518d1639b",
            )
        } else {
            (
                "ggerganov/whisper.cpp",
                "5359861c739e955e79d9a303bcbc70fb988958b1",
            )
        };
        let url = format!(
            "https://huggingface.co/{repository}/resolve/{revision}/{}?download=true",
            model.file
        );
        let client = reqwest::Client::builder()
            .https_only(true)
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(45))
            .timeout(Duration::from_secs(1800))
            .build()?;
        let mut response = client.get(url).send().await?.error_for_status()?;
        let temporary = tempfile::NamedTempFile::new_in(&self.directory)?.into_temp_path();
        let mut file = fs::File::create(&temporary).await?;
        let mut digest = Sha256::new();
        let mut total = 0;
        while let Some(bytes) = response.chunk().await? {
            total += bytes.len() as u64;
            ensure!(total <= model.size, "model response too large");
            digest.update(&bytes);
            file.write_all(&bytes).await?;
            self.transfers
                .lock()
                .unwrap()
                .entry(model.id)
                .or_default()
                .downloaded = total;
        }
        file.sync_all().await?;
        drop(file);
        ensure!(
            total == model.size && format!("{:x}", digest.finalize()) == model.sha256,
            "model integrity check failed"
        );
        temporary.persist(self.path(model))?;
        Ok(())
    }
}

pub async fn verify_file(path: &Path, model: &CatalogModel) -> Result<()> {
    let mut file = fs::File::open(path)
        .await
        .context("Model is missing. Open Models to install it.")?;
    ensure!(
        file.metadata().await?.len() == model.size,
        "Model size is incorrect. Reinstall it from Models."
    );
    let mut digest = Sha256::new();
    let mut buffer = vec![0; 128 * 1024];
    loop {
        let size = file.read(&mut buffer).await?;
        if size == 0 {
            break;
        }
        digest.update(&buffer[..size]);
    }
    ensure!(
        format!("{:x}", digest.finalize()) == model.sha256,
        "Model checksum failed. Reinstall it from Models."
    );
    Ok(())
}
