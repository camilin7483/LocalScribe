# LocalScribe 0.1.0 verification

Run on 2026-11-22. The primary host was Arch Linux x86-64 on an Intel Core i3‑1215U with 16 GB RAM and integrated Intel UHD graphics.

| Check | Observed coverage |
|---|---|
| Source | Rust formatting, seven unit tests and Clippy with warnings denied. TypeScript type checking and a production Vite build. |
| Audio and video | Actual whisper.cpp transcription of synthetic Spanish and English recordings. The full Arch integration run covered WAV, MP3, FLAC, M4A, OGG, AAC, MP4, MKV, MOV, WEBM and AVI. It included deterministic background noise, a silent sample, malformed input and an MKV playback preview. |
| Work queue | A 93-second fixture reached a committed 30-second checkpoint. The test cancelled the job, resumed it, stopped and restarted the server, verified the Interrupted state and completed the remaining windows while preserving the first segment IDs and timestamps. |
| Browser | Chromium drove a batch import using drag and drop and the file picker. Playback advanced and sought to a segment. Text and speaker edits survived reload, original text remained available, undo/redo worked, and both SRT and VTT downloaded. Spanish, English, both themes and a mobile viewport were exercised. The page's requests remained on loopback. |
| Privacy | Host and Origin validation, custom same-origin header, rejected cross-origin preflight, security headers, default-off Ollama and media range requests. The packaged Linux binaries also transcribed and exported inside an isolated network namespace with only loopback enabled. |
| Models | Removed Silero and installed it through the actual HTTPS download path. Verified the stored file's SHA‑256 and transcribed with it again. |
| Local analysis | Ollama with Qwen2.5 3B exercised six summary modes, translation, correction requests and extractive answers. Known and absent-answer questions exercised citation verification. Sherpa-onnx 2.0.2 exercised speaker proposals and applying them. With a known count of two, the separate alternating-voice fixture produced two labels that persisted after applying the proposals. |
| Linux package | The tarball was extracted and the browser suite passed with only loopback enabled on Arch. A basic API smoke passed in an isolated Ubuntu 22.04.5 userland with distribution FFmpeg 4.4.2. The generic non-AVX2 engine also completed a real transcript. |
| Windows executable | Built PE x64 binaries with Rust and LLVM-MinGW. Wine 11.22 ran the loopback server, the FFmpeg and FFprobe diagnostic checks and real Whisper inference, then exercised edit, playback range, history and the five exports. Native Windows 10/11 installation and browser launch still need verification. |
| Advisory scans | npm audit reported zero advisories. Cargo Audit found an advisory in the first bundled SQLite version. The lockfile was upgraded to rusqlite 0.43 and libsqlite3-sys 0.40 and rescanned without reported advisories. |
| Demo | The browser compositor captured the real interface. FFprobe reported H.264, 1920 × 1080, 30/1 fps, exactly 900 frames and 30.000000 seconds. The video decoded without errors and the scene frames were visually reviewed. |

The browser test and integration runner are in `tests`. See [BUILD](BUILD.md) for commands, [BENCHMARKS](BENCHMARKS.md) for workload measurements and [DEMO](DEMO.md) for the 30-second script.

## Limits and pending release checks

- Transcription uses CPU builds with a generic fallback. GPU inference and GPU performance comparisons remain open.
- Decode windows overlap near their boundaries. Hard cuts, VAD, mixed-language recordings and weak source audio can affect timestamps and recognition. The examples are synthetic and each benchmark covers one short recording.
- Audio-only previews require the chosen FFmpeg installation to support MP3 encoding. Browser playback of the original depends on its codecs.
- The memory ceiling uses sampled resident memory per child process. Kernel-level and systemwide memory enforcement remain open.
- Ollama is optional. Context limits and the quality of the local model affect summaries, corrections and translations. For automatic speaker clustering, a known count helps with similar voices. Mixed or overlapping speech can require manual editing.
- External AI provider adapters, DOCX and PDF exports, subtitles burned into video, and signed Windows installers remain pending.
- The source, Linux tarball and Windows ZIP are local 0.1.0 candidates. The first GitHub Actions run remains pending. Native Windows installation, the packaged setup script on both Windows versions and clean desktop-menu launch on Ubuntu are release gates.
