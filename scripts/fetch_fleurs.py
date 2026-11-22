#!/usr/bin/env python3
"""Fetch reproducible human speech samples and references from the FLEURS test split."""
import argparse
import csv
from hashlib import sha256
import io
import json
from pathlib import Path
import tarfile
from urllib.request import urlopen

REVISION = "70bb2e84b976b7e960aa89f1c648e09c59f894dd"
BASE = f"https://huggingface.co/datasets/google/fleurs/resolve/{REVISION}/data"
CONFIGS = {"es": "es_419", "en": "en_us"}
MAX_AUDIO = 8 * 1024 * 1024


def samples(language, count, directory):
    base = f"{BASE}/{CONFIGS[language]}"
    with urlopen(f"{base}/test.tsv?download=true", timeout=45) as response:
        text = io.TextIOWrapper(response, encoding="utf-8")
        references = {row[1]: row for row in csv.reader(text, delimiter="\t", quoting=csv.QUOTE_NONE) if len(row) >= 4}
    utterances = set()
    with urlopen(f"{base}/audio/test.tar.gz?download=true", timeout=45) as handle:
        with tarfile.open(mode="r|gz", fileobj=handle) as archive:
            for member in archive:
                name = Path(member.name).name
                row = references.get(name)
                if not row or row[0] in utterances or not member.isfile() or member.size > MAX_AUDIO:
                    continue
                if not name.endswith(".wav") or not name.removesuffix(".wav").isdigit():
                    continue
                extracted = archive.extractfile(member)
                if extracted is None:
                    continue
                audio = extracted.read()
                if not audio.startswith(b"RIFF"):
                    continue
                target = f"{language}-{name}"
                (directory / target).write_bytes(audio)
                utterances.add(row[0])
                yield {"file": target, "language": language, "source_filename": name,
                       "utterance_id": row[0], "reference": row[2], "sha256": sha256(audio).hexdigest()}
                if len(utterances) == count:
                    return
    raise RuntimeError(f"Found fewer than {count} usable utterances for {language}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--per-language", type=int, default=12)
    args = parser.parse_args()
    if not 1 <= args.per_language <= 100:
        parser.error("select 1–100 samples per language")
    args.directory.mkdir(parents=True, exist_ok=True)
    entries = [sample for language in CONFIGS for sample in samples(language, args.per_language, args.directory)]
    manifest = {"dataset": "google/fleurs", "revision": REVISION,
                "license": "CC-BY-4.0", "split": "test", "samples": entries}
    (args.directory / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"Fetched {len(entries)} FLEURS test recordings ({args.per_language} per language)", flush=True)


if __name__ == "__main__":
    main()
