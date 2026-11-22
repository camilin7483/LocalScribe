"""Collect third-party licenses from resolved Cargo and npm packages."""
import json
import pathlib
import shutil
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]


def license_paths(directory):
    stems = ("license", "licence", "copying", "copyright", "notice", "unlicense")
    return [path for path in directory.iterdir() if path.is_file() and path.name.lower().startswith(stems)]


def collect(target, platform=None):
    target.mkdir(parents=True, exist_ok=True)
    args = ["cargo", "metadata", "--locked", "--format-version", "1"]
    if platform:
        args += ["--filter-platform", platform]
    metadata = json.loads(subprocess.check_output(args, cwd=ROOT))
    frontend = ROOT / "frontend"
    npm = json.loads((frontend / "package-lock.json").read_text(encoding="utf-8"))["packages"]
    packages = [
        ("cargo", package["name"], package["version"], package["license"], pathlib.Path(package["manifest_path"]).parent)
        for package in metadata["packages"] if package.get("source")
    ]
    packages += [
        ("npm", key.removeprefix("node_modules/"), value["version"], value.get("license"), frontend / key)
        for key, value in npm.items() if key and not value.get("dev")
    ]
    index = ["# Third-party component licenses", "", "Component | Version | SPDX metadata", "---|---|---"]
    for folder, name, version, license_name, directory in sorted(packages):
        if not directory.is_dir():
            continue
        index.append(f"{folder}/{name} | {version} | {license_name or 'See included notice'}")
        licenses = license_paths(directory)
        for license_path in licenses:
            destination = target / folder / name / version
            destination.mkdir(parents=True, exist_ok=True)
            shutil.copy2(license_path, destination / license_path.name)
    (target / "INDEX.md").write_text("\n".join(index) + "\n", encoding="utf-8")
