# Optional local engines

## Ollama

Install Ollama and download a local completion model. A multilingual model such as `qwen2.5:3b` fits the reference machine when there is enough free RAM.

```sh
ollama pull qwen2.5:3b
OLLAMA_NO_CLOUD=1 ollama serve
```

If Ollama is already managed as a service, set `OLLAMA_NO_CLOUD=1` for that service and restart it. Keep Ollama bound to loopback. On Windows use the user environment variable and restart Ollama. In LocalScribe, enable **Allow local Ollama** under **Privacy**, open **Local AI**, choose a completed transcript and run the desired action.

The named Ollama model produces the six summary modes, translation and contextual corrections. Review generated analysis against the recording. Questions use extractive answers. Ollama proposes quotations and LocalScribe accepts only exact substrings from the named segments, then takes times from those segments. It reports when it could not find an explicit answer. Corrections are proposals and only take effect after **Apply suggestions**. The editor always retains the initial transcript.

The analysis request limits are 14,000 UTF-8 bytes for summaries and questions, and 5,500 for translation and correction, inclusive of segment JSON. Model capabilities and available memory affect results. [Ollama local FAQ](https://docs.ollama.com/faq) · [Ollama Chat API](https://docs.ollama.com/api/chat) · [Qwen2.5 3B](https://ollama.com/library/qwen2.5:3b)

## Speaker identification

Install a sherpa-onnx executable distribution containing `sherpa-onnx-offline-speaker-diarization`. Use its pyannote segmentation v3.0 ONNX model and a compatible embedding model such as the upstream WeSpeaker CAM++ export. On any supported OS, set:

| Environment variable | Value |
|---|---|
| `LOCALSCRIBE_DIARIZATION` | Absolute path to the sherpa-onnx CLI, with its libraries available. |
| `LOCALSCRIBE_SPEAKER_SEGMENTATION` | Absolute path to the Pyannote `model.onnx`. |
| `LOCALSCRIBE_SPEAKER_EMBEDDING` | Absolute path to the speaker embedding weights. |

Restart LocalScribe. In **Local AI**, choose **Identify speakers** and review the proposals. The engine clusters speech turns on the CPU and maps each transcript segment to the speaker with the most temporal overlap. The action is intended for recordings up to 30 minutes and the editor supports changing any speaker label. If you know the number of speakers, enter it before running. Automatic counts and labels can be sensitive to source audio and the clustering threshold.

[Engine releases](https://github.com/k2-fsa/sherpa-onnx/releases) · [Upstream diarization setup and models](https://k2-fsa.github.io/sherpa/onnx/speaker-diarization/index.html)
