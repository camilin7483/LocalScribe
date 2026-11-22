#!/usr/bin/env python3
"""Measure CLI model/context reuse on identical already-decoded WAV inputs."""
import argparse
import hashlib
import json
import pathlib
import subprocess
import tempfile
from time import monotonic


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--whisper", type=pathlib.Path, required=True)
    parser.add_argument("--model", type=pathlib.Path, required=True)
    parser.add_argument("--vad", type=pathlib.Path)
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--repeats", type=int, default=2)
    parser.add_argument("wav", type=pathlib.Path, nargs="+")
    args = parser.parse_args()
    if len(args.wav) < 2 or args.repeats < 1 or args.threads < 1:
        parser.error("provide at least two WAV files and positive threads and repeats")
    base = [str(args.whisper.resolve()), "-ng", "-m", str(args.model.resolve()), "-oj", "-l", "auto",
            "-t", str(args.threads), "-bs", "3", "-bo", "1"]
    if args.vad:
        base += ["--vad", "-vm", str(args.vad.resolve()), "-vsd", "300"]
    rows = []
    with tempfile.TemporaryDirectory(prefix="localscribe-context-") as directory:
        for trial in range(args.repeats):
            outputs = {}
            order = ("isolated", "shared") if trial % 2 == 0 else ("shared", "isolated")
            for mode in order:
                target = pathlib.Path(directory)
                prefixes = [target / f"{trial}-{mode}-{index}" for index in range(len(args.wav))]
                pairs = [["-f", str(wav.resolve()), "-of", str(prefix)]
                         for wav, prefix in zip(args.wav, prefixes, strict=True)]
                commands = pairs if mode == "isolated" else [sum(pairs, [])]
                start = monotonic()
                for arguments in commands:
                    subprocess.run(base + arguments, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, check=True, timeout=900)
                elapsed = monotonic() - start
                outputs[mode] = [json.loads(prefix.with_suffix(".json").read_text(encoding="utf-8")) for prefix in prefixes]
                rows.append({"trial": trial + 1, "mode": mode, "seconds": round(elapsed, 3), "processes": len(commands)})
            for row in rows[-2:]:
                row["exact_segments_and_language"] = all(
                    (isolated["transcription"], isolated["result"]["language"]) == (shared["transcription"], shared["result"]["language"])
                    for isolated, shared in zip(outputs["isolated"], outputs["shared"], strict=True)
                )
    with args.whisper.open("rb") as engine:
        engine_hash = hashlib.file_digest(engine, "sha256").hexdigest()
    result = {"engine_sha256": engine_hash,
              "model": args.model.name, "vad": args.vad.name if args.vad else None,
              "clips": [path.name for path in args.wav], "threads": args.threads, "trials": rows}
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
