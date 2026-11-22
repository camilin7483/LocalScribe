#!/usr/bin/env python3
"""Compare local Base inference engines on identical audio with recorded settings."""
import argparse
from collections import defaultdict
from hashlib import file_digest
from importlib.metadata import version
import json
from pathlib import Path
import statistics
import subprocess
import tempfile
from time import monotonic

from compare_whisper_devices import wer


def sha256(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def cpp(engine, model, audio, language, threads, prefix):
    output = prefix.with_suffix(".json")
    output.unlink(missing_ok=True)
    subprocess.run(
        [str(engine), "-ng", "-m", str(model), "-f", str(audio), "-l", language,
         "-t", str(threads), "-bs", "1", "-bo", "1", "-nf", "-oj", "-of", str(prefix)],
        stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True, check=True, timeout=300,
    )
    result = json.loads(output.read_text(encoding="utf-8"))
    return " ".join(segment["text"] for segment in result["transcription"])


def selected(manifest, limit):
    groups = defaultdict(list)
    for sample in manifest["samples"]:
        group = groups[sample["language"]]
        if len(group) < limit:
            group.append(sample)
    return [sample for group in groups.values() for sample in group]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--whisper", type=Path, required=True)
    parser.add_argument("--ggml-model", type=Path, required=True)
    parser.add_argument("--ct2-model", type=Path, required=True, help="Local multilingual Base model directory.")
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--clips-per-language", type=int, default=2)
    parser.add_argument("--repeats", type=int, default=2)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if min(args.threads, args.clips_per_language, args.repeats) <= 0:
        parser.error("threads, clips per language and repeats must be positive")

    from faster_whisper import WhisperModel

    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    samples = selected(manifest, args.clips_per_language)
    for sample in samples:
        audio = args.manifest.parent / sample["file"]
        if sha256(audio) != sample["sha256"]:
            raise ValueError(f"Fixture checksum changed: {audio.name}")
    if not samples:
        parser.error("the manifest needs at least one recording")

    engine = args.whisper.resolve()
    ggml = args.ggml_model.resolve()
    ct2 = args.ct2_model.resolve()
    names = ("whisper.cpp", "faster-whisper")
    rows = []
    with tempfile.TemporaryDirectory(prefix="localscribe-engine-") as directory:
        prefix = Path(directory) / "caption"
        for repeat in range(args.repeats):
            for index, sample in enumerate(samples):
                audio = (args.manifest.parent / sample["file"]).resolve()
                language = sample["language"]
                for name in names if (repeat + index) % 2 == 0 else reversed(names):
                    started = monotonic()
                    if name == "whisper.cpp":
                        hypothesis = cpp(engine, ggml, audio, language, args.threads, prefix)
                    else:
                        loaded = WhisperModel(str(ct2), device="cpu", compute_type="int8", cpu_threads=args.threads, local_files_only=True)
                        segments, _ = loaded.transcribe(str(audio), language=language, beam_size=1, best_of=1,
                                                        temperature=0, vad_filter=False, condition_on_previous_text=False)
                        hypothesis = " ".join(segment.text for segment in segments)
                        del loaded
                    elapsed = monotonic() - started
                    error = wer(sample["reference"], hypothesis)
                    rows.append({"engine": name, "file": sample["file"], "language": language,
                                 "sha256": sample["sha256"], "repeat": repeat + 1,
                                 "seconds": round(elapsed, 3), "wer": error})
                    print(f"{name} {sample['file']}: {elapsed:.2f} s, WER {error:.1%}", flush=True)
    summary = {}
    for name in names:
        subset = [row for row in rows if row["engine"] == name]
        summary[name] = {"median_seconds": statistics.median(row["seconds"] for row in subset),
                         "macro_wer": statistics.mean(row["wer"] for row in subset)}
    result = {
        "dataset": manifest["dataset"], "revision": manifest["revision"], "license": manifest["license"],
        "threads": args.threads, "beam": 1, "vad": False, "language": "manual",
        "scope": "model load and decode with warm filesystem caches; Python import excluded",
        "whisper_cli_sha256": sha256(engine), "ggml_model": ggml.name, "ggml_model_sha256": sha256(ggml),
        "faster_whisper_version": version("faster-whisper"), "ctranslate2_version": version("ctranslate2"),
        "ctranslate2_model_sha256": sha256(ct2 / "model.bin"), "ctranslate2_compute_type": "int8",
        "summary": summary, "runs": rows,
    }
    if args.output:
        args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
