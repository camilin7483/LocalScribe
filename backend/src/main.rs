#![forbid(unsafe_code)]

mod ai;
mod api;
mod db;
mod diarization;
mod domain;
mod error;
mod export;
mod hardware;
mod media;
mod models;
mod ollama;
mod queue;
mod state;
mod subprocess;
mod transcribe;
mod wer;

use anyhow::{Context, Result, bail, ensure};
use directories::ProjectDirs;
use state::{Runtime, State};
use std::{
    collections::HashMap,
    fs::OpenOptions,
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
    sync::Mutex,
};
use tokio::{
    net::TcpListener,
    sync::{Notify, Semaphore},
};
use tokio_util::sync::CancellationToken;

struct Args {
    port: u16,
    data: PathBuf,
    web: PathBuf,
    open: bool,
}

fn arguments() -> Result<Option<Args>> {
    let default_data = ProjectDirs::from("", "LocalScribe", "LocalScribe")
        .context("No user data directory was found.")?
        .data_local_dir()
        .to_owned();
    let exe = std::env::current_exe()?;
    let packaged_web = exe
        .parent()
        .context("No executable directory.")?
        .join("web");
    let mut args = Args {
        port: 7676,
        data: std::env::var_os("LOCALSCRIBE_DATA")
            .map(PathBuf::from)
            .unwrap_or(default_data),
        web: if packaged_web.is_dir() {
            packaged_web
        } else {
            PathBuf::from("frontend/dist")
        },
        open: true,
    };
    let mut input = std::env::args_os().skip(1);
    while let Some(option) = input.next() {
        match option.to_str() {
            Some("--port") => {
                args.port = input
                    .next()
                    .context("Missing port.")?
                    .to_string_lossy()
                    .parse()?
            }
            Some("--data-dir") => {
                args.data = input.next().context("Missing data directory.")?.into()
            }
            Some("--web-root") => args.web = input.next().context("Missing web root.")?.into(),
            Some("--no-open") => args.open = false,
            Some("--version") => {
                println!("LocalScribe {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            Some("--help" | "-h") => {
                println!(
                    "LocalScribe {}\n\n--no-open           Do not open the browser\n--port PORT         Loopback port (default 7676, 0 for an ephemeral port)\n--data-dir PATH     Private data directory\n--web-root PATH     Built frontend directory\n--version           Print the version",
                    env!("CARGO_PKG_VERSION")
                );
                return Ok(None);
            }
            _ => bail!("Unknown option. Use --help."),
        }
    }
    Ok(Some(args))
}

fn open_browser(url: &str) {
    #[cfg(target_os = "linux")]
    let command = hardware::program("xdg-open", "LOCALSCRIBE_BROWSER").map(|path| {
        let mut command = std::process::Command::new(path);
        command.arg(url);
        command
    });
    #[cfg(target_os = "windows")]
    let command = {
        let mut command = std::process::Command::new("explorer.exe");
        command.arg(url);
        Some(command)
    };
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let command: Option<std::process::Command> = None;
    if let Some(mut command) = command {
        use std::process::Stdio;
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        std::thread::spawn(move || {
            let _ = command.status();
        });
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let Some(args) = arguments()? else {
        return Ok(());
    };
    ensure!(
        args.web.join("index.html").is_file(),
        "Frontend not found. Build it or use --web-root."
    );
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, args.port))
        .await
        .context("Could not bind to loopback. Try --port 0 or close the existing instance.")?;
    let authority = listener.local_addr()?.to_string();
    state::private_directory(&args.data)?;
    let data = args.data.canonicalize()?;
    let _lease = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(data.join("instance.lock"))?;
    _lease
        .try_lock()
        .context("This data directory is already open in LocalScribe.")?;
    let media = data.join("media");
    let model_dir = data.join("models");
    let scratch = data.join("scratch");
    for directory in [&media, &model_dir, &scratch] {
        state::private_directory(directory)?;
    }
    for entry in std::fs::read_dir(&scratch)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with(".tmp") && entry.file_type()?.is_dir() {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
    for entry in std::fs::read_dir(&model_dir)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with(".tmp") && entry.file_type()?.is_file() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    let db = db::Database::open(&data.join("localscribe.sqlite3"))?;
    db.recover()?;
    let mut programs = media::Programs::locate();
    programs.validate().await;
    let engine = if programs.optimized && programs.whisper.is_some() {
        "CPU (AVX2)"
    } else {
        "CPU"
    };
    let hardware = hardware::Hardware::inspect(engine.into());
    let upload_limit = std::env::var("LOCALSCRIBE_MAX_UPLOAD_MIB")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(4096)
        .clamp(1, 16_384)
        * 1024
        * 1024;
    let state = State::new(Runtime {
        db,
        programs,
        hardware,
        models: models::Models::new(model_dir),
        media,
        scratch,
        authority,
        upload_limit,
        uploads: Semaphore::new(2),
        auxiliary: Semaphore::new(1),
        wake: Notify::new(),
        active: Mutex::new(HashMap::new()),
        shutdown: CancellationToken::new(),
    });
    state.clean_media()?;
    tokio::spawn(queue::run(state.clone()));
    let router = api::router(state.clone(), &args.web);
    let url = format!("http://{}", state.authority);
    println!("LocalScribe listening at {url}");
    if args.open {
        open_browser(&url);
    }
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            state.shutdown.cancel();
        })
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    if let Ok(mut term) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
    {
        tokio::select! { _ = term.recv() => {}, _ = tokio::signal::ctrl_c() => {} }
        return;
    }
    let _ = tokio::signal::ctrl_c().await;
}
