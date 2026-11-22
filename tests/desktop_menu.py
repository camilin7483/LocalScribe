#!/usr/bin/env python3
"""Verify the extracted Linux package's desktop registration through GIO."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
from tempfile import TemporaryDirectory
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("package", type=Path)
    args = parser.parse_args()

    with TemporaryDirectory(prefix="localscribe-menu-") as path:
        workspace = Path(path)
        data = workspace / "data"
        proof = workspace / "launched"
        env = {**os.environ, "XDG_DATA_HOME": str(data), "LOCALSCRIBE_MENU_PROOF": str(proof)}

        for name in ("LocalScribe 50%", 'LocalScribe $p "café" \\ x'):
            program = workspace / name
            program.mkdir()
            shutil.copy2(args.package / "install-menu.sh", program)
            executable = program / "localscribe"
            executable.write_text(
                '#!/bin/sh\nprintf "%s\\n" "$PWD" > "$LOCALSCRIBE_MENU_PROOF"\n',
                encoding="utf-8",
            )
            executable.chmod(0o755)
            proof.unlink(missing_ok=True)
            subprocess.run(["sh", str(program / "install-menu.sh")], env=env, check=True, timeout=10)
            entry = data / "applications" / "localscribe.desktop"
            subprocess.run(["desktop-file-validate", str(entry)], env=env, check=True, timeout=10)
            subprocess.run(["gio", "launch", str(entry)], env=env, check=True, timeout=10)
            cutoff = time.monotonic() + 5
            while not proof.exists() and time.monotonic() < cutoff:
                time.sleep(0.05)
            assert proof.read_text(encoding="utf-8").rstrip("\n") == str(program), entry.read_text()
    print("PASS desktop registration and launch from paths with spaces, percent signs and punctuation")


if __name__ == "__main__":
    main()
