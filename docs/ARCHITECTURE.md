# Architecture

| Layer | Responsibility |
|---|---|
| `frontend/src` | React views, hash navigation, same-origin API client, batch import, transcript editing and local preferences. |
| `backend/src/api.rs` | Axum routes, upload validation, loopback request policy, file playback and exports. |
| `backend/src/db.rs` and `schema.sql` | SQLite jobs, immutable original segments, editable text and settings. |
| `backend/src/queue.rs` | Scheduling with live memory headroom, worker reservations, cancellation and recovery. |
| `backend/src/media.rs` and `transcribe.rs` | FFmpeg/FFprobe media inspection and decode, whisper.cpp invocation, timestamp normalization and checkpointing. |
| `backend/src/hardware.rs` and `models.rs` | AutoTune calibration and policies, model catalog, pinned downloads and integrity checks. |
| `backend/src/ai.rs`, `ollama.rs` and `diarization.rs` | Explicit requests to the optional local engines and validated evidence or edit proposals. |

## Import and transcribe

1. Axum streams multipart file data into a private temporary file. The displayed name is treated as metadata. A generated UUID and validated extension identify the stored file.
2. The queue inspects the first audio track and determines duration. FFmpeg decodes bounded 16 kHz mono PCM windows to a temporary file. The queue updates its reservations using live available memory.
3. whisper.cpp processes each window in a cancellable child process. VAD uses Silero when enabled. A small overlap gives context at boundaries. Segment ownership follows each segment's midpoint on the global timeline.
4. A single SQLite transaction writes segment text and updates the committed audio checkpoint. Progress is processed duration divided by inspected audio duration. Before a checkpoint, the UI shows the real queue state without estimating the remaining time.
5. Cancellation terminates the current child. Restart changes unfinished running jobs to **Interrupted**. Resuming reuses the completed windows and the original job settings. The process lease keeps two instances from modifying the same workspace concurrently.

The Generic and AVX2 CPU variants ship separately. A job uses one ASR model at a time. Native inference and FFmpeg processes exit after each use. This bounds model retention and enables cancellation, with a model-load cost per audio window. The memory guard samples child RSS about every 75 ms. Its ceiling has the resolution of that sampler. A queue may wait for memory to become available.

## Editing and exports

A revision counter orders transcript writes. Browser autosave submits edits sequentially. A stale revision returns a conflict, safeguarding concurrent work. Original segments stay in a separate column and can be compared or exported as JSON. SRT and VTT export the current timed cues. Playback uses HTTP range requests on allowlisted original formats. When the browser cannot play an original, it can request a compact local MP3 preview.

## Privacy and storage

- Axum binds to `127.0.0.1` and verifies the exact HTTP Host. API requests use a custom same-origin header. Cross-origin requests, unexpected origins and framing are rejected. Same-origin HTTP media requests use the browser fetch metadata.
- Child processes receive structured argument arrays. FFmpeg only reads local files using an allowlist of recording demuxers. Playlist and network protocols are unavailable in the media pipeline.
- The workspace is private to the OS account and contains SQLite data, media, weights and temporary PCM. SQLite can contain transcript contents, its write-ahead log can persist until checkpointing, and the application relies on OS access controls. A local process with the user's privileges is within the trust boundary.
- Model downloads use version-pinned HTTPS URLs and verify length and SHA‑256 before publishing weights to the local catalog. Downloads send no recordings or transcript fields.
- The optional Ollama client uses a fixed loopback address, no proxy and no redirect. The app screens hosted model variants. Operate the separately installed server in local-only mode. The app has no telemetry or cloud adapter. [Ollama local FAQ](https://docs.ollama.com/faq)
- Backups and app-generated downloads are independent files. Deleting a job removes its managed media, playback preview and rows from the active workspace. OS-level secure erasure and rolling back another user's edit session are outside the design.

Upstream references [whisper.cpp](https://github.com/ggml-org/whisper.cpp/tree/v1.9.5) · [FFmpeg protocol allowlists](https://www.ffmpeg.org/ffmpeg-protocols.html) · [Axum](https://docs.rs/axum/0.8.9/axum/)
