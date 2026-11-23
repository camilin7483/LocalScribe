#!/usr/bin/env python3
"""Assemble a standalone LocalScribe archive from actual build outputs."""
import argparse
import pathlib
import shutil
import tarfile
import tempfile
import tomllib
import zipfile

from notices import collect

ROOT = pathlib.Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", choices=("linux", "windows"), required=True)
    parser.add_argument("--native", type=pathlib.Path, required=True, help="Directory with localscribe, both CPU engines and the whisper.cpp license.")
    parser.add_argument("--output", type=pathlib.Path, default=ROOT / ".release")
    parser.add_argument("--target-triple", help="Use the exact Cargo target when collecting dependency license texts.")
    parser.add_argument("--extra-license", type=pathlib.Path, action="append", default=[])
    args = parser.parse_args()
    version = tomllib.loads((ROOT / "backend" / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    stem = f"LocalScribe-{version}-{args.platform}-x86_64"
    args.output.mkdir(exist_ok=True, parents=True)
    suffix = ".exe" if args.platform == "windows" else ""
    archive_path = args.output / (stem + (".zip" if suffix else ".tar.gz"))
    with tempfile.TemporaryDirectory() as temp:
        package = pathlib.Path(temp) / stem
        package.mkdir()
        for name in [f"localscribe{suffix}", f"whisper-cli{suffix}", f"whisper-cli-avx2{suffix}", "whisper.cpp-LICENSE.txt"]:
            shutil.copy2(args.native / name, package / name)
        shutil.copytree(ROOT / "frontend" / "dist", package / "web")
        for name in ("README.md", "LICENSE", "CHANGELOG.md"):
            shutil.copy2(ROOT / name, package / name)
        shutil.copytree(ROOT / "docs", package / "docs")
        for file in (ROOT / "packaging" / args.platform).iterdir():
            shutil.copy2(file, package / file.name)
        licenses = package / "third-party-licenses"
        collect(licenses, args.target_triple)
        toolchain_license = args.native / "llvm-mingw-LICENSE.txt"
        if args.platform == "windows" and toolchain_license.is_file():
            shutil.copy2(toolchain_license, licenses / toolchain_license.name)
        for file in args.extra_license:
            shutil.copy2(file, licenses / file.name)
        with tempfile.TemporaryDirectory(dir=args.output) as staging:
            pending = pathlib.Path(staging) / archive_path.name
            if suffix:
                with zipfile.ZipFile(pending, "w", zipfile.ZIP_DEFLATED, compresslevel=6, strict_timestamps=False) as archive:
                    for file in sorted(package.rglob("*")):
                        if file.is_file():
                            archive.write(file, file.relative_to(package.parent))
            else:
                with tarfile.open(pending, "w:gz") as archive:
                    archive.add(package, arcname=stem)
            pending.replace(archive_path)
    print(archive_path)


if __name__ == "__main__":
    main()
