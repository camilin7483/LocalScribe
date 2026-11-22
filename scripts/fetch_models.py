#!/usr/bin/env python3
"""Fetch the exact Whisper and Silero models used by LocalScribe's test suite."""
import argparse
import hashlib
import pathlib
import tempfile
import urllib.request

WHISPER = "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/"
SILERO = "https://huggingface.co/ggml-org/whisper-vad/resolve/9ffd54a1e1ee413ddf265af9913beaf518d1639b/"
FILES = {
    "ggml-tiny-q5_1.bin": (WHISPER, "818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7"),
    "ggml-base-q5_1.bin": (WHISPER, "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898"),
    "ggml-small-q5_1.bin": (WHISPER, "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb"),
    "ggml-silero-v6.2.0.bin": (SILERO, "2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987"),
}


def verified(path, digest):
    if not path.is_file():
        return False
    with path.open("rb") as file:
        return hashlib.file_digest(file, "sha256").hexdigest() == digest


def fetch(destination, include_small):
    destination.mkdir(parents=True, exist_ok=True)
    for filename, (base, sha256) in FILES.items():
        if "small" in filename and not include_small:
            continue
        target = destination / filename
        if not verified(target, sha256):
            with tempfile.TemporaryDirectory(dir=destination) as directory:
                partial = pathlib.Path(directory) / filename
                with urllib.request.urlopen(base + filename + "?download=true", timeout=90) as response, partial.open("wb") as output:
                    while chunk := response.read(128 * 1024):
                        output.write(chunk)
                if not verified(partial, sha256):
                    raise RuntimeError(f"Checksum mismatch: {filename}")
                partial.replace(target)
        print(f"Verified {filename}")


if __name__ == "__main__":
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("destination", type=pathlib.Path)
    cli.add_argument("--all", action="store_true")
    params = cli.parse_args()
    fetch(params.destination, params.all)
