#!/usr/bin/env python3
"""Compare CPU and verified Vulkan execution on the same human recordings."""
import argparse
from collections import defaultdict
import hashlib
import json
from pathlib import Path
import re
import statistics
import subprocess
import tempfile
from time import monotonic


def tokens(text):
    return re.findall(r"[^\W_]+", text.lower())


def wer(reference, hypothesis):
    words = tokens(reference)
    costs = list(range(len(words) + 1))
    for count, spoken in enumerate(tokens(hypothesis), start=1):
        previous, costs = costs, [count]
        for index, expected in enumerate(words, start=1):
            costs.append(min(previous[index] + 1, costs[-1] + 1, previous[index - 1] + (spoken != expected)))
    return costs[-1] / len(words) if words else None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--whisper", type=Path, required=True, help="Build of whisper-cli with Vulkan support.")
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--vad", type=Path, required=True)
    parser.add_argument("--device", type=int, default=0)
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--clips-per-language", type=int, default=2)
    parser.add_argument("--repeats", type=int, default=2)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if min(args.repeats, args.threads, args.clips_per_language) <= 0 or args.device < 0:
        parser.error("use a nonnegative device and positive counts")
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    chosen = defaultdict(list)
    for sample in manifest["samples"]:
        group = chosen[sample["language"]]
        if len(group) < args.clips_per_language:
            group.append(sample)
    base = [str(args.whisper.resolve()), "-m", str(args.model.resolve()), "-oj", "-l", "auto",
            "-t", str(args.threads), "-bs", "3", "-bo", "1",
            "--vad", "-vm", str(args.vad.resolve()), "-vsd", "300"]
    rows = []
    with tempfile.TemporaryDirectory(prefix="localscribe-device-") as temporary:
        prefix = Path(temporary) / "result"
        for repeat in range(args.repeats):
            for index, sample in enumerate(sample for group in chosen.values() for sample in group):
                audio = args.manifest.parent / sample["file"]
                with audio.open("rb") as stream:
                    if hashlib.file_digest(stream, "sha256").hexdigest() != sample["sha256"]:
                        raise ValueError(f"Fixture checksum changed: {audio.name}")
                order = ["cpu", "vulkan"] if (repeat + index) % 2 == 0 else ["vulkan", "cpu"]
                for backend in order:
                    mode = ["-ng"] if backend == "cpu" else ["-dev", str(args.device)]
                    started = monotonic()
                    run = subprocess.run(base + mode + ["-f", str(audio), "-of", str(prefix)],
                                         stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
                                         check=True, encoding="utf-8", errors="replace", timeout=300)
                    elapsed = monotonic() - started
                    actual = re.search(r"whisper_backend_init_gpu: using (Vulkan\d+) backend", run.stderr)
                    if (actual is not None) != (backend == "vulkan"):
                        raise RuntimeError(f"Expected {backend} execution. Whisper reported:\n{run.stderr[-2500:]}")
                    track = json.loads(prefix.with_suffix(".json").read_text(encoding="utf-8"))
                    hypothesis = " ".join(segment["text"] for segment in track["transcription"])
                    rows.append({"trial": repeat + 1, "file": sample["file"], "sha256": sample["sha256"],
                                 "backend": backend, "device": actual.group(1) if actual else "CPU",
                                 "seconds": round(elapsed, 3), "language": track["result"]["language"],
                                 "wer": wer(sample["reference"], hypothesis)})
    with args.whisper.open("rb") as stream:
        engine_hash = hashlib.file_digest(stream, "sha256").hexdigest()
    summary = {backend: {"median_seconds": statistics.median(row["seconds"] for row in rows if row["backend"] == backend),
                         "macro_wer": statistics.mean(row["wer"] for row in rows if row["backend"] == backend)}
               for backend in ("cpu", "vulkan")}
    result = json.dumps({"revision": manifest["revision"], "engine_sha256": engine_hash, "model": args.model.name,
                         "threads": args.threads, "summary": summary, "runs": rows}, indent=2) + "\n"
    if args.output:
        args.output.write_text(result, encoding="utf-8")
    print(result, end="")


if __name__ == "__main__":
    main()
