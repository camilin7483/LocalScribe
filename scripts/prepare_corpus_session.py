#!/usr/bin/env python3
"""Assemble a timed, repeatable long session from a reference corpus."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import wave


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--seconds", type=int, default=1800)
    args = parser.parse_args()
    if args.seconds <= 0:
        parser.error("duration must be positive")
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        parser.error("ffmpeg is required")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="localscribe-corpus-session-") as directory:
        sequence = Path(directory) / "sequence.wav"
        with wave.open(str(sequence), "wb") as wav:
            wav.setparams((1, 2, 16000, 0, "NONE", "not compressed"))
            for sample in manifest["samples"]:
                clip = args.manifest.parent / sample["file"]
                with clip.open("rb") as stream:
                    if hashlib.file_digest(stream, "sha256").hexdigest() != sample["sha256"]:
                        raise ValueError(f"Incorrect source hash: {clip.name}")
                pcm = subprocess.check_output([ffmpeg, "-v", "error", "-nostdin", "-i", str(clip),
                                               "-f", "s16le", "-ar", "16000", "-ac", "1", "pipe:1"])
                wav.writeframes(pcm + b"\x00" * 16000)
            if wav.getnframes() == 0:
                parser.error("no audio available")
        subprocess.run([ffmpeg, "-v", "error", "-nostdin", "-y", "-stream_loop", "-1", "-i", str(sequence),
                        "-t", str(args.seconds), "-c:a", "flac", str(args.output)], check=True)
    print(args.output)


if __name__ == "__main__":
    main()
