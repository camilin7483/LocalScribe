#!/usr/bin/env python3
"""Measure word error rates and language detection on reference recordings."""
import argparse
from collections import defaultdict
import hashlib
import json
from pathlib import Path
import statistics
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tests"))
from local_support import TestServer, arguments


def main():
    parser = arguments(argparse.ArgumentParser(description=__doc__))
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    rows = []
    with TestServer(args.binary, args.models, args.web, args.data) as server:
        for entry in manifest["samples"]:
            source = args.manifest.parent / entry["file"]
            with source.open("rb") as stream:
                if hashlib.file_digest(stream, "sha256").hexdigest() != entry["sha256"]:
                    raise ValueError(f"Fixture checksum changed: {source.name}")
            variants = [("tiny-q5_1", "tiny-q5_1"), ("eco-auto", None)]
            if len(rows) % 4:
                variants.reverse()
            for variant, model in variants:
                options = {"model": model, "profile": "eco", "vad": True,
                           "benchmark": True, "reference": entry["reference"]}
                job = server.wait(server.import_file(source, options))["job"]
                stage = job["metrics"]
                rows.append({"file": entry["file"], "sha256": entry["sha256"], "expected_language": entry["language"],
                             "variant": variant, "actual_model": job["plan"]["model"],
                             "actual_language": job["detected_language"], "status": job["status"],
                             "duration_ms": job["duration_ms"], "pipeline_ms": stage["wall_ms"], "wer": stage["wer"]})
                print(f'{entry["file"]} {variant}: {job["status"]} {stage["wer"]}', file=sys.stderr, flush=True)
    groups = defaultdict(list)
    for row in rows:
        groups[(row["variant"], row["expected_language"])].append(row)
    summary = []
    for (variant, language), group in sorted(groups.items()):
        errors = [row["wer"] for row in group if row["wer"] is not None]
        summary.append({"variant": variant, "language": language, "recordings": len(group), "scored": len(errors),
                        "macro_wer": statistics.mean(errors) if errors else None,
                        "language_matches": sum(row["actual_language"] == language for row in group)})
    output = json.dumps({"dataset": manifest["dataset"], "revision": manifest["revision"], "license": manifest["license"],
                         "summary": summary, "recordings": rows}, ensure_ascii=False, indent=2)
    if args.output:
        args.output.write_text(output + "\n", encoding="utf-8")
    print(output)


if __name__ == "__main__":
    main()
