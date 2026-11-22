use std::{ffi::OsStr, path::Path};

use crate::{
    State,
    domain::{Job, JobOptions, Profile, SegmentEdit, Settings, SettingsChange},
    error::{ApiError, ApiResult},
    export, media, models,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path as RoutePath, Query, Request, State as AxumState},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};
use uuid::Uuid;

pub const LANGUAGES: &str = "en zh de es ru ko fr ja pt tr pl ca nl ar sv it id hi fi vi he uk el ms cs ro da hu ta no th ur hr bg lt la mi ml cy sk te fa lv bn sr az sl kn et mk br eu is hy ne mn bs kk sq sw gl mr pa si km sn yo so af oc ka be tg sd gu am yi lo uz fo ht ps tk nn mt sa lb my bo tl mg as tt haw ln ha ba jw su yue";

pub fn router(state: State, web_root: &Path) -> Router {
    let limit = DefaultBodyLimit::max(state.upload_limit + 256 * 1024);
    Router::new()
        .route("/api/health", get(health))
        .route("/api/system", get(system))
        .route("/api/settings", patch(settings))
        .route("/api/jobs", get(list).post(upload).layer(limit))
        .route("/api/jobs/{id}", get(detail).delete(delete_job))
        .route("/api/jobs/{id}/cancel", post(cancel))
        .route("/api/jobs/{id}/resume", post(resume))
        .route("/api/jobs/{id}/segments", patch(edit))
        .route("/api/jobs/{id}/export/{kind}", get(download_export))
        .route("/api/jobs/{id}/preview", post(prepare_preview))
        .route("/api/jobs/{id}/waveform", get(waveform))
        .route("/api/media/{id}/{variant}", get(play))
        .route(
            "/api/models/{id}",
            post(download_model).delete(delete_model),
        )
        .merge(crate::ai::routes())
        .fallback_service(ServeDir::new(web_root))
        .layer(middleware::from_fn_with_state(state.clone(), privacy))
        .with_state(state)
}

async fn privacy(AxumState(state): AxumState<State>, request: Request, next: Next) -> Response {
    let (api, allowed) = check_request(&request, &state.authority);
    let mut response = if allowed {
        next.run(request).await
    } else {
        StatusCode::FORBIDDEN.into_response()
    };
    let headers = response.headers_mut();
    for (name, value) in [
        (
            "content-security-policy",
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; media-src 'self' blob:; frame-ancestors 'none'; object-src 'none'; base-uri 'none'; form-action 'self'",
        ),
        ("x-content-type-options", "nosniff"),
        ("x-frame-options", "DENY"),
        ("referrer-policy", "no-referrer"),
        ("cross-origin-resource-policy", "same-origin"),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
    ] {
        headers.insert(name, HeaderValue::from_static(value));
    }
    if api {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    response
}

fn check_request(request: &Request, authority: &str) -> (bool, bool) {
    let supplied = |name| {
        request
            .headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
    };
    let host_ok = supplied("host") == Some(authority);
    let origin = format!("http://{authority}");
    let origin_ok = supplied("origin").is_none_or(|value| value == origin)
        && !matches!(supplied("sec-fetch-site"), Some("cross-site" | "same-site"));
    let path = request.uri().path();
    let api = path.starts_with("/api/");
    let media = path.starts_with("/api/media/")
        && matches!(*request.method(), Method::GET | Method::HEAD)
        && supplied("sec-fetch-site") == Some("same-origin");
    (
        api,
        host_ok && (!api || (origin_ok && (supplied("x-localscribe") == Some("1") || media))),
    )
}

async fn health() -> Json<Value> {
    Json(json!({"status":"ok","version":env!("CARGO_PKG_VERSION")}))
}

async fn system(AxumState(state): AxumState<State>) -> ApiResult<Json<Value>> {
    let config = state.db.settings()?;
    let hardware = state.hardware.live();
    let plans = [Profile::Eco, Profile::Balanced, Profile::Precision]
        .map(|profile| hardware.plan(profile, None, &config));
    let [files, active, completed] = state.db.counts()?;
    Ok(Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "hardware": hardware,
        "plans": plans,
        "settings": config,
        "models": state.models.status(),
        "diagnostics": {
            "ffmpeg":state.programs.ffmpeg.is_some(),
            "ffprobe":state.programs.ffprobe.is_some(),
            "whisper":state.programs.whisper.is_some(),
        },
        "counts": { "files":files, "active":active, "completed":completed },
        "upload_limit": state.upload_limit,
        "languages": LANGUAGES.split_whitespace().collect::<Vec<_>>(),
    })))
}

async fn settings(
    AxumState(state): AxumState<State>,
    Json(change): Json<SettingsChange>,
) -> ApiResult<Json<Settings>> {
    if change
        .memory_mib
        .is_some_and(|value| value != 0 && !(512..=24_576).contains(&value))
        || change
            .max_workers
            .is_some_and(|value| !(1..=2).contains(&value))
    {
        return Err(ApiError::input(
            "Choose a memory budget of 512–24576 MiB (or 0 for Auto) and 1–2 workers.",
        ));
    }
    let settings = state.db.update_settings(change)?;
    state.wake.notify_one();
    Ok(Json(settings))
}

#[derive(Default, Deserialize)]
struct ListQuery {
    before: Option<i64>,
    view: Option<String>,
}

async fn list(
    AxumState(state): AxumState<State>,
    Query(query): Query<ListQuery>,
) -> ApiResult<Json<Value>> {
    let jobs = state.db.list(
        query.before.unwrap_or(i64::MAX),
        query.view.as_deref().unwrap_or(""),
    )?;
    let cursor = if jobs.len() == 60 {
        state.db.cursor(&jobs[59].id)?
    } else {
        None
    };
    Ok(Json(json!({ "jobs": jobs, "cursor": cursor })))
}

pub fn find_job(state: &State, id: Uuid) -> ApiResult<Job> {
    state.db.get(&id.to_string())?.ok_or_else(ApiError::missing)
}

async fn detail(
    AxumState(state): AxumState<State>,
    RoutePath(id): RoutePath<Uuid>,
) -> ApiResult<Json<Value>> {
    let (job, segments) = state
        .db
        .snapshot(&id.to_string())?
        .ok_or_else(ApiError::missing)?;
    Ok(Json(json!({
        "job": job,
        "segments": segments,
        "original_playback": media::browser_type(&job.file_extension).is_some(),
        "video": matches!(job.file_extension.as_str(), "mp4" | "webm"),
        "preview_ready": state.preview(&job.id).is_file()
    })))
}

fn plan_for(state: &State, options: &JobOptions) -> ApiResult<crate::domain::Plan> {
    if !state.programs.ready() {
        return Err(ApiError::unavailable(
            "Install the tools shown in System diagnostics before transcribing.",
        ));
    }
    if options.language != "auto"
        && !LANGUAGES
            .split_whitespace()
            .any(|language| language == options.language)
    {
        return Err(ApiError::input("Unsupported language code."));
    }
    let config = state.db.settings()?;
    let model = options
        .model
        .as_deref()
        .map(|id| {
            models::find(id)
                .filter(|model| !model.vad)
                .ok_or_else(|| ApiError::input("Unknown transcription model."))
        })
        .transpose()?;
    let plan = state
        .hardware
        .live()
        .plan(options.profile, model, &config)
        .ok_or_else(|| ApiError::conflict("The model does not fit the available memory budget."))?;
    if !state.models.installed(models::find(&plan.model).unwrap()) {
        return Err(ApiError::conflict(
            "Install the selected Whisper model in Models first.",
        ));
    }
    if options.vad && !state.models.installed(models::find("silero").unwrap()) {
        return Err(ApiError::conflict(
            "Install Silero in Models or turn off voice activity detection.",
        ));
    }
    if options
        .reference
        .as_ref()
        .is_some_and(|reference| !options.benchmark || reference.len() > 12_000)
    {
        return Err(ApiError::input(
            "A benchmark reference must be under 12,000 UTF-8 bytes.",
        ));
    }
    Ok(plan)
}

fn safe_filename(name: &str) -> ApiResult<(String, String)> {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("").trim();
    if base.is_empty()
        || base.len() > 255
        || base.chars().any(|character| {
            character.is_control()
                || ('\u{202a}'..='\u{202e}').contains(&character)
                || ('\u{2066}'..='\u{2069}').contains(&character)
        })
    {
        return Err(ApiError::input(
            "Use a file name between 1 and 255 UTF-8 bytes without control characters.",
        ));
    }
    let extension = Path::new(base)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !media::EXTENSIONS.contains(&extension.as_str()) {
        return Err(ApiError::input("This file extension is unsupported."));
    }
    Ok((base.to_owned(), extension))
}

async fn upload(
    AxumState(state): AxumState<State>,
    mut form: Multipart,
) -> ApiResult<impl IntoResponse> {
    let _permit = state.uploads.try_acquire().map_err(|_| {
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "Wait for an active import to finish.",
        )
    })?;
    let parse_error = |_| ApiError::input("Upload body is invalid or reached the file limit.");
    let mut field = form
        .next_field()
        .await
        .map_err(parse_error)?
        .filter(|field| field.name() == Some("options"))
        .ok_or_else(|| ApiError::input("Missing import options."))?;
    let mut raw = Vec::new();
    while let Some(bytes) = field.chunk().await.map_err(parse_error)? {
        if raw.len() + bytes.len() > 24_000 {
            return Err(ApiError::input("Import options are too large."));
        }
        raw.extend_from_slice(&bytes);
    }
    drop(field);
    let options: JobOptions =
        serde_json::from_slice(&raw).map_err(|_| ApiError::input("Invalid import options."))?;
    plan_for(&state, &options)?;
    let mut field = form
        .next_field()
        .await
        .map_err(parse_error)?
        .filter(|field| field.name() == Some("file"))
        .ok_or_else(|| ApiError::input("Missing audio or video file."))?;
    let (name, extension) = safe_filename(field.file_name().unwrap_or_default())?;
    let temp = tempfile::NamedTempFile::new_in(&state.media)?.into_temp_path();
    let mut file = tokio::fs::File::create(&temp).await?;
    let mut size = 0;
    while let Some(bytes) = field.chunk().await.map_err(parse_error)? {
        size += bytes.len();
        if size > state.upload_limit {
            return Err(ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "The file exceeds the local import limit.",
            ));
        }
        file.write_all(&bytes).await?;
    }
    if size == 0 {
        return Err(ApiError::input("This file is empty."));
    }
    file.sync_all().await?;
    drop(file);
    let plan = plan_for(&state, &options)?;
    let id = Uuid::new_v4().to_string();
    let destination = state.media.join(format!("{id}.{extension}"));
    temp.persist_noclobber(&destination)
        .map_err(|error| std::io::Error::from(error.error.kind()))?;
    let job = match state
        .db
        .insert(&id, &name, &extension, size as i64, &options, &plan)
    {
        Ok(job) => job,
        Err(error) => {
            let _ = tokio::fs::remove_file(destination).await;
            return Err(error.into());
        }
    };
    state.wake.notify_one();
    Ok((StatusCode::ACCEPTED, Json(job)))
}

async fn cancel(
    AxumState(state): AxumState<State>,
    RoutePath(id): RoutePath<Uuid>,
) -> ApiResult<StatusCode> {
    let id = id.to_string();
    let active = state.active.lock().unwrap();
    if !state.db.cancel(&id)? {
        return Err(ApiError::conflict(
            "Only queued or running jobs can be cancelled.",
        ));
    }
    if let Some((token, _, _)) = active.get(&id) {
        token.cancel();
    }
    state.wake.notify_one();
    Ok(StatusCode::NO_CONTENT)
}

async fn resume(
    AxumState(state): AxumState<State>,
    RoutePath(id): RoutePath<Uuid>,
) -> ApiResult<StatusCode> {
    let job = find_job(&state, id)?;
    let active = state.active.lock().unwrap();
    if active.contains_key(&job.id) {
        return Err(ApiError::conflict(
            "Wait for the previous process to exit, then resume.",
        ));
    }
    if !state.db.resume(&job.id)? {
        return Err(ApiError::conflict(
            "This job is already queued, running or complete.",
        ));
    }
    state.wake.notify_one();
    Ok(StatusCode::NO_CONTENT)
}

async fn edit(
    AxumState(state): AxumState<State>,
    RoutePath(id): RoutePath<Uuid>,
    Json(mut edit): Json<SegmentEdit>,
) -> ApiResult<Json<Value>> {
    if edit.text.len() > 20_000 || edit.text.contains('\0') || edit.id < 1 {
        return Err(ApiError::input(
            "A segment must use at most 20,000 UTF-8 bytes.",
        ));
    }
    edit.speaker = edit
        .speaker
        .take()
        .map(|speaker| speaker.trim().to_owned())
        .filter(|speaker| !speaker.is_empty());
    if edit
        .speaker
        .as_ref()
        .is_some_and(|speaker| speaker.len() > 120 || speaker.chars().any(char::is_control))
    {
        return Err(ApiError::input(
            "A speaker label must use at most 120 UTF-8 bytes.",
        ));
    }
    let revision = state.db.edit(&id.to_string(), &edit)?.ok_or_else(|| {
        ApiError::conflict("This transcript changed. Reload before editing again.")
    })?;
    Ok(Json(json!({"revision":revision})))
}

async fn delete_job(
    AxumState(state): AxumState<State>,
    RoutePath(id): RoutePath<Uuid>,
) -> ApiResult<StatusCode> {
    let job = find_job(&state, id)?;
    let _permit = state.auxiliary.try_acquire().map_err(|_| {
        ApiError::conflict("Wait for local analysis or preview preparation to finish.")
    })?;
    {
        let active = state.active.lock().unwrap();
        if active.contains_key(&job.id) || !state.db.delete(&job.id)? {
            return Err(ApiError::conflict(
                "Cancel the job and wait for the process to exit before deleting.",
            ));
        }
    }
    for path in [state.source(&job), state.preview(&job.id)] {
        match tokio::fs::remove_file(path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(ApiError::conflict(
                    "History was cleared. The media file is locked and will be removed next time LocalScribe starts.",
                ));
            }
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn download_export(
    AxumState(state): AxumState<State>,
    RoutePath((id, kind)): RoutePath<(Uuid, String)>,
) -> ApiResult<Response> {
    let (job, segments) = state
        .db
        .snapshot(&id.to_string())?
        .ok_or_else(ApiError::missing)?;
    if job.status != "completed" && segments.is_empty() {
        return Err(ApiError::conflict(
            "This job has no exportable segments yet.",
        ));
    }
    let (content, mime) = export::make(&job, &segments, &kind)?.ok_or_else(ApiError::missing)?;
    let disposition = format!(
        "attachment; filename=\"localscribe-{}.{}\"",
        &job.id[..8],
        kind
    );
    Ok((
        [
            (header::CONTENT_TYPE, mime),
            (header::CONTENT_DISPOSITION, disposition.as_str()),
        ],
        content,
    )
        .into_response())
}

async fn prepare_preview(
    AxumState(state): AxumState<State>,
    RoutePath(id): RoutePath<Uuid>,
) -> ApiResult<StatusCode> {
    let _permit = state.auxiliary.try_acquire().map_err(|_| {
        ApiError::conflict("Wait for local analysis or preview preparation to finish.")
    })?;
    let job = find_job(&state, id)?;
    if state.preview(&job.id).is_file() {
        return Ok(StatusCode::NO_CONTENT);
    }
    let duration = job
        .duration_ms
        .ok_or_else(|| ApiError::conflict("Wait for the file to be inspected."))?;
    let engine = state
        .programs
        .ffmpeg
        .as_deref()
        .ok_or_else(|| ApiError::unavailable("FFmpeg is unavailable."))?;
    let temporary = tempfile::NamedTempFile::new_in(&state.media)?.into_temp_path();
    media::preview(
        engine,
        &state.source(&job),
        &temporary,
        duration,
        &state.shutdown,
    )
    .await
    .map_err(|error| ApiError::unavailable(error.to_string()))?;
    temporary
        .persist(state.preview(&job.id))
        .map_err(|err| err.error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn play(
    AxumState(state): AxumState<State>,
    RoutePath((id, variant)): RoutePath<(Uuid, String)>,
    request: Request,
) -> ApiResult<Response> {
    let job = find_job(&state, id)?;
    let (path, mime) = match variant.as_str() {
        "preview" => (state.preview(&job.id), "audio/mpeg"),
        "original" => (
            state.source(&job),
            media::browser_type(&job.file_extension).ok_or_else(ApiError::missing)?,
        ),
        _ => return Err(ApiError::missing()),
    };
    let mut response = ServeFile::new(path)
        .oneshot(request)
        .await
        .unwrap()
        .into_response();
    if response.status().is_success() {
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
    }
    Ok(response)
}

async fn download_model(
    AxumState(state): AxumState<State>,
    RoutePath(id): RoutePath<String>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    if headers
        .get("x-localscribe-download")
        .and_then(|v| v.to_str().ok())
        != Some("1")
    {
        return Err(ApiError::input(
            "Confirm that this download can connect to Hugging Face.",
        ));
    }
    let model = models::find(&id).ok_or_else(ApiError::missing)?;
    if !state.models.reserve(model) {
        return Err(ApiError::conflict(
            "The model is already installed or two downloads are in progress.",
        ));
    }
    tokio::spawn(async move {
        state.models.download(model).await;
    });
    Ok(StatusCode::ACCEPTED)
}

async fn delete_model(
    AxumState(state): AxumState<State>,
    RoutePath(id): RoutePath<String>,
) -> ApiResult<StatusCode> {
    let model = models::find(&id).ok_or_else(ApiError::missing)?;
    let active = state.active.lock().unwrap();
    if !active.is_empty() || state.db.counts()?[1] > 0 || !state.models.remove(model)? {
        return Err(ApiError::conflict(
            "Wait for model downloads and queued jobs to finish before removing a model.",
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn waveform(
    AxumState(state): AxumState<State>,
    RoutePath(id): RoutePath<Uuid>,
) -> ApiResult<Json<Vec<u16>>> {
    let job = find_job(&state, id)?;
    let duration = job
        .duration_ms
        .filter(|duration| *duration > 0)
        .ok_or_else(|| ApiError::conflict("Wait for the file to be inspected."))?;
    let _permit = state
        .waveform
        .try_acquire()
        .map_err(|_| ApiError::conflict("Another waveform is being generated."))?;
    let engine = state
        .programs
        .ffmpeg
        .as_deref()
        .ok_or_else(|| ApiError::unavailable("FFmpeg is unavailable."))?;
    let peaks = media::waveform(engine, &state.source(&job), duration, &state.shutdown)
        .await
        .map_err(|_| ApiError::unavailable("Could not read the audio waveform."))?;
    Ok(Json(peaks))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_used_only_as_display_names() {
        assert_eq!(
            safe_filename("C:\\recordings\\meeting.MP3").unwrap(),
            ("meeting.MP3".into(), "mp3".into())
        );
        assert!(safe_filename("document.html").is_err());
        assert!(safe_filename("invalid\n.wav").is_err());
        assert!(safe_filename("fake\u{202e}.wav").is_err());
    }
}
