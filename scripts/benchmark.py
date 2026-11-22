#!/usr/bin/env python3
"""Run fresh, sequential transcription measurements for each installed fixture model."""
import argparse
import json
import pathlib
import statistics
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tests"))
from local_support import FIXTURES, TestServer, arguments

PROFILE_BY_MODEL = {"tiny-q5_1": "eco", "base-q5_1": "balanced", "small-q5_1": "precision"}


def main():
    cli = arguments(argparse.ArgumentParser(description=__doc__))
    cli.add_argument("--media", type=pathlib.Path, default=FIXTURES / "es.wav")
    cli.add_argument("--reference", type=pathlib.Path, default=FIXTURES / "es.txt")
    cli.add_argument("--model", choices=tuple(PROFILE_BY_MODEL), action="append")
    cli.add_argument("--repetitions", type=int, default=2)
    cli.add_argument("--output", type=pathlib.Path)
    args = cli.parse_args()
    reference = args.reference.read_text(encoding="utf-8")
    rows = []
    with TestServer(args.binary, args.models, args.web, args.data) as server:
        hardware = server.api("/system")["hardware"]
        for model in args.model or list(PROFILE_BY_MODEL):
            for trial in range(1, args.repetitions + 1):
                started = time.monotonic()
                job = server.import_file(args.media, {"profile": PROFILE_BY_MODEL[model], "model": model,
                                                     "language": "auto", "vad": True, "benchmark": True, "reference": reference})
                detail = server.wait(job)
                job = detail["job"]
                if job["status"] != "completed":
                    raise RuntimeError(job["error"])
                metrics = job["metrics"]
                rows.append({"model": model, "trial": trial, "profile": job["plan"]["profile"], "threads": job["plan"]["threads"],
                             "audio_ms": job["duration_ms"], "pipeline_ms": metrics["wall_ms"],
                             "elapsed_ms": round((time.monotonic() - started) * 1000), "stages": metrics.get("stages"),
                             "peak_rss_bytes": metrics["peak_rss_bytes"], "cpu_ms": metrics["cpu_ms"],
                             "rtf": round(metrics["wall_ms"] / job["duration_ms"], 4), "wer": metrics["wer"]})
        aggregates = [{"model": model, "median_rtf": round(statistics.median(row["rtf"] for row in rows if row["model"] == model), 4)}
                      for model in dict.fromkeys(row["model"] for row in rows)]
    result = json.dumps({"hardware": hardware, "sample": args.media.name, "trials": rows, "summary": aggregates},
                        indent=2, ensure_ascii=False) + "\n"
    if args.output:
        args.output.write_text(result, encoding="utf-8")
    print(result, end="")


if __name__ == "__main__":
    main()
