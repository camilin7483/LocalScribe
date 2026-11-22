# Licenses and attribution

LocalScribe's source, its original synthetic test scripts, and the original test phrases and recordings in `tests/fixtures` are covered by [MIT](../LICENSE). eSpeak NG generated the English and Spanish speech. The demo uses the real app and contains no background music.

| Component | Attribution |
|---|---|
| whisper.cpp and ggml | MIT, Georgi Gerganov and contributors. The packages include the source release's license. [Source and license](https://github.com/ggml-org/whisper.cpp/tree/v1.9.5) |
| Whisper Q5_1 | Whisper weights by OpenAI under MIT, converted models from the whisper.cpp project. [OpenAI Whisper license](https://github.com/openai/whisper/blob/main/LICENSE) · [GGML files](https://huggingface.co/ggerganov/whisper.cpp/tree/5359861c739e955e79d9a303bcbc70fb988958b1) |
| Silero VAD | MIT, Silero Team. [Original model and license](https://github.com/snakers4/silero-vad) · [GGML files](https://huggingface.co/ggml-org/whisper-vad/tree/9ffd54a1e1ee413ddf265af9913beaf518d1639b) |
| FFmpeg and FFprobe | External executables installed separately. Licensing depends on the chosen build. [FFmpeg licensing and source](https://ffmpeg.org/legal.html) |
| SQLite | Public domain. [SQLite copyright](https://www.sqlite.org/copyright.html) |
| Manrope and IBM Plex Mono | SIL Open Font License 1.1. Browser files are served locally. [Manrope](https://github.com/sharanda/manrope) · [IBM Plex](https://github.com/IBM/plex) |
| Lucide icons | ISC, with MIT attribution for inherited Feather icons. [Lucide license](https://github.com/lucide-icons/lucide/blob/main/LICENSE) |
| Ollama and optional model | Ollama is MIT. Example Qwen2.5 3B weights have their own Qwen Research license. [Ollama license](https://github.com/ollama/ollama/blob/main/LICENSE) · [Qwen2.5 3B model license](https://huggingface.co/Qwen/Qwen2.5-3B-Instruct/blob/main/LICENSE) |
| sherpa-onnx and optional models | sherpa-onnx is Apache 2.0. Pyannote segmentation v3.0 is MIT. Use the redistribution terms of the chosen embedding model. [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx/blob/master/LICENSE) · [Pyannote segmentation](https://huggingface.co/pyannote/segmentation-3.0) · [WeSpeaker](https://github.com/wenet-e2e/wespeaker) |

Portable packages include resolved Cargo and production npm component notices. FFmpeg, Ollama and optional model weights are installed separately. Checksums and immutable revisions for the four download options are in `backend/src/models.rs` and the fixture downloader.
