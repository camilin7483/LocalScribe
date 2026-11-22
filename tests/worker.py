#!/usr/bin/env python3
"""Verify the optional Whisper worker against isolated CLI and SQLite checkpoints."""
import argparse
from hashlib import file_digest
import json
import os
from pathlib import Path
import sys
import tempfile
from time import monotonic
from unittest.mock import patch

from integration import verify_checkpoint
from local_support import FIXTURES, TestServer, arguments, ffmpeg


def previous_run(server, source):
    def digest(path):
        with path.open("rb") as stream:
            return file_digest(stream, "sha256").digest()

    known = digest(source)
    for job in server.api("/jobs")["jobs"]:
        stored = server.data / "media" / (job["id"] + source.suffix.lower())
        plan = job["plan"]
        if (job["name"] == source.name and job["vad"] and plan["model"] == "base-q5_1"
                and plan["profile"] == "eco" and stored.is_file() and digest(stored) == known):
            return job
    return None


def run(args, mode, executable, worker, source, checkpoint_directory=None):
    env = {"LOCALSCRIBE_WHISPER_PERSISTENT": "1" if worker else "0"}
    if executable is not None:
        env["LOCALSCRIBE_WHISPER"] = str(executable.resolve())
    data = args.data / mode if args.data else None
    if data:
        data.mkdir(parents=True, exist_ok=True)
    with patch.dict(os.environ, env), TestServer(args.binary, args.models, args.web, data) as server:
        started = monotonic()
        existing = previous_run(server, source) if args.resume else None
        if existing:
            job = existing
            if job["status"] in ("interrupted", "cancelled", "failed"):
                server.api("/jobs/" + job["id"] + "/resume", "POST", expected=204)
            print(f"{mode}: reusing {job['id']} at {job['processed_ms']} ms", file=sys.stderr, flush=True)
        else:
            job = server.import_file(source, {"profile": "eco", "model": "base-q5_1", "vad": True})
            print(f"{mode}: {job['id']} is queued", file=sys.stderr, flush=True)
        processed = job["processed_ms"]

        def finished(detail):
            nonlocal processed
            current = detail["job"]
            if args.media and current["processed_ms"] > processed:
                processed = current["processed_ms"]
                print(f"{mode}: {processed}/{current['duration_ms']} ms committed", file=sys.stderr, flush=True)
            return current["status"] not in ("running", "queued")

        detail = server.wait(job, predicate=finished, timeout=args.timeout)
        elapsed_ms = round((monotonic() - started) * 1000)
        assert detail["job"]["status"] == "completed", detail
        if checkpoint_directory is not None:
            verify_checkpoint(server, checkpoint_directory, require_persistent=True)
        return detail, elapsed_ms, existing is not None


def normalized(detail):
    return [(segment["start_ms"], segment["end_ms"], segment["original"]) for segment in detail["segments"]]


def main():
    parser = arguments(argparse.ArgumentParser(description=__doc__))
    parser.add_argument("--whisper", type=Path, help="Override the packaged Whisper executable.")
    parser.add_argument("--legacy-whisper", type=Path, help="Verify that an unpatched CLI is recovered from automatically.")
    parser.add_argument("--media", type=Path, help="Optional longer comparison file.")
    parser.add_argument("--timeout", type=int, default=600)
    parser.add_argument("--resume", action="store_true", help="Resume matching benchmark jobs from --data and --media.")
    parser.add_argument("--interrupt", action="store_true", help="Cancel and restart an active work queue.")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.resume and not (args.data and args.media):
        parser.error("--resume requires --data and --media")
    with tempfile.TemporaryDirectory(prefix="localscribe-worker-test-") as path:
        directory = Path(path)
        if args.media is None:
            source = directory / "speech-silence-speech.wav"
            ffmpeg("-i", FIXTURES / "es.wav", "-i", FIXTURES / "en.wav", "-filter_complex",
                   "[0:a]apad=whole_dur=62[first];[first][1:a]concat=n=2:v=0:a=1[a]",
                   "-map", "[a]", "-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le", source)
        else:
            source = args.media
        versions = [
            ("cli", *run(args, "cli", args.whisper, False, source)),
            ("persistent", *run(args, "persistent", args.whisper, True, source, directory if args.interrupt else None)),
        ]
        if args.legacy_whisper:
            versions.append(("fallback", *run(args, "fallback", args.legacy_whisper, True, source)))
    expected = normalized(versions[0][1])
    assert expected, "The comparison fixture must produce usable text"
    for mode, detail, _, reused in versions:
        job = detail["job"]
        stages = job["metrics"]["stages"]
        assert normalized(detail) == expected, (mode, expected[:6], normalized(detail)[:6])
        assert job["processed_ms"] == job["duration_ms"]
        if mode == "persistent":
            assert stages["persistent_windows"] >= 2 and stages["worker_fallbacks"] == 0, stages
        else:
            assert stages["cli_windows"] >= 2 and stages["persistent_windows"] == 0, stages
        if mode == "fallback":
            count = stages["worker_fallbacks"]
            assert count >= 1 and (reused or count == 1), stages
    if args.media is None:
        assert all(detail["job"]["metrics"]["stages"]["silent_windows"] == 1 for _, detail, _, _ in versions)
        assert all(end < 29_000 or start > 61_000 for start, end, _ in expected), expected
        assert any(start > 61_000 for start, _, _ in expected), expected
    thread_counts = {detail["job"]["plan"]["threads"] for _, detail, _, _ in versions}
    report = {"same_thread_count": len(thread_counts) == 1, "media": source.name, "duration_ms": versions[0][1]["job"]["duration_ms"],
              "exact_segments": True, "segment_count": len(expected),
              "runs": [{"mode": mode, "elapsed_ms": elapsed, "reused": reused,
                        "threads": detail["job"]["plan"]["threads"], "metrics": detail["job"]["metrics"]}
                       for mode, detail, elapsed, reused in versions]}
    formatted = json.dumps(report, ensure_ascii=False, indent=2)
    if args.output:
        args.output.write_text(formatted + "\n", encoding="utf-8")
    print(formatted, flush=True)


if __name__ == "__main__":
    main()
