#!/usr/bin/env python3
"""Regenerate the original Spanish and English test recordings with eSpeak NG."""
import argparse
import pathlib
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests" / "fixtures"


def generate_dialogue(args, directory):
    turns = (FIXTURES / "dialogue.txt").read_text(encoding="utf-8").splitlines()
    paths = []
    for index, phrase in enumerate(turns):
        voice, pitch, rate = ("en-us+m3", "26", "153") if index % 2 == 0 else ("en-gb+f3", "67", "143")
        path = directory / f"turn-{index}.wav"
        subprocess.run([args.espeak, "-v", voice, "-p", pitch, "-s", rate, "-w", str(path), phrase], check=True)
        paths += ["-i", str(path)]
    filters = "[0:a]apad=pad_dur=2.2[a];[1:a]apad=pad_dur=2.2[b];[2:a]apad=pad_dur=2.2[c];[a][b][c][3:a]concat=n=4:v=0:a=1[s]"
    subprocess.run([args.ffmpeg, "-nostdin", "-v", "error", "-y", *paths, "-filter_complex", filters,
                    "-map", "[s]", "-c:a", "pcm_s16le", "-ar", "16000", "-ac", "1", str(FIXTURES / "dialogue.wav")], check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--espeak", default="espeak-ng")
    parser.add_argument("--ffmpeg", default="ffmpeg")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory() as directory:
        raw = pathlib.Path(directory) / "speech.wav"
        for language, voice, rate in [("es", "es", "146"), ("en", "en-us", "150")]:
            text = (FIXTURES / f"{language}.txt").read_text(encoding="utf-8").strip()
            subprocess.run([args.espeak, "-v", voice, "-s", rate, "-w", str(raw), text], check=True)
            subprocess.run([args.ffmpeg, "-nostdin", "-v", "error", "-y", "-i", str(raw),
                            "-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le", str(FIXTURES / f"{language}.wav")], check=True)
        generate_dialogue(args, pathlib.Path(directory))


if __name__ == "__main__":
    main()
