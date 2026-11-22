#!/usr/bin/env python3
"""Build the frontend, server and two CPU variants of the pinned whisper.cpp release."""
import argparse
import hashlib
import os
import pathlib
import shutil
import subprocess
import tarfile
import urllib.request
from whisper_worker import add_worker

ROOT = pathlib.Path(__file__).resolve().parents[1]
VERSION = "1.9.5"
SHA = "ff1a9053feb509ff9d7729703355541ae9690073a6b1c40eb692c962e0dc1720"
URL = f"https://codeload.github.com/ggml-org/whisper.cpp/tar.gz/refs/tags/v{VERSION}"
EXE = ".exe" if os.name == "nt" else ""


def call(*args, cwd=ROOT):
    subprocess.run([str(arg) for arg in args], cwd=cwd, check=True)


def retrieve(directory):
    source = directory / f"whisper.cpp-{VERSION}"
    directory.mkdir(parents=True, exist_ok=True)
    tarball = directory / f"whisper.cpp-{VERSION}.tar.gz"
    if not tarball.is_file():
        temporary = tarball.with_suffix(".partial")
        with urllib.request.urlopen(URL, timeout=90) as response, temporary.open("wb") as output:
            shutil.copyfileobj(response, output)
        temporary.replace(tarball)
    with tarball.open("rb") as stream:
        if hashlib.file_digest(stream, "sha256").hexdigest() != SHA:
            raise RuntimeError("whisper.cpp source archive integrity check failed.")
    if not source.is_dir():
        with tarfile.open(tarball) as archive:
            archive.extractall(directory, filter="data")
    # whisper.cpp 1.9.5 uses std::partial_sort here. libc++ does not include algorithm transitively.
    common = source / "examples" / "common.cpp"
    content = common.read_bytes()
    if b"#include <algorithm>" not in content:
        common.write_bytes(b"#include <algorithm>\n" + content)
    add_worker(source / "examples" / "cli" / "cli.cpp")
    # Vulkan's generated target uses SPIR-V headers. Propagate the
    # imported include path when the SDK is installed outside /usr.
    vulkan = source / "ggml" / "src" / "ggml-vulkan" / "CMakeLists.txt"
    cmake = vulkan.read_text(encoding="utf-8")
    original = "target_link_libraries(ggml-vulkan PRIVATE Vulkan::Vulkan)"
    patched = "target_link_libraries(ggml-vulkan PRIVATE Vulkan::Vulkan SPIRV-Headers::SPIRV-Headers)"
    if patched not in cmake:
        if cmake.count(original) != 1:
            raise RuntimeError("Pinned Vulkan target changed.")
        vulkan.write_text(cmake.replace(original, patched), encoding="utf-8")
    return source


def whisper(source, target, variant, jobs):
    build = target / ("whisper-" + variant)
    flags = [
        "-DCMAKE_BUILD_TYPE=Release", "-DBUILD_SHARED_LIBS=OFF", "-DGGML_NATIVE=OFF",
        "-DGGML_OPENMP=OFF", "-DWHISPER_BUILD_TESTS=OFF", "-DWHISPER_BUILD_SERVER=OFF",
    ]
    if variant == "vulkan":
        flags.append("-DGGML_VULKAN=ON")
    if variant == "generic":
        flags += [f"-DGGML_{feature}=OFF" for feature in ("AVX", "AVX2", "SSE42", "F16C", "FMA", "BMI2")]
    if os.name == "nt":
        flags.append("-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded")
    call("cmake", "-S", source, "-B", build, *flags)
    call("cmake", "--build", build, "--config", "Release", "--target", "whisper-cli", "--parallel", jobs)
    matches = sorted((build / "bin").rglob("whisper-cli" + EXE))
    if not matches:
        raise RuntimeError("Could not find the built whisper-cli")
    return matches[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dir", type=pathlib.Path, default=ROOT / ".release")
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--vulkan-lab", action="store_true", help="Build an additional Vulkan CLI for benchmarks.")
    args = parser.parse_args()
    build = args.dir.resolve()
    stage = build / "native"
    stage.mkdir(parents=True, exist_ok=True)
    npm = shutil.which("npm.cmd" if os.name == "nt" else "npm")
    if not npm:
        raise RuntimeError("npm is required for a source build.")
    call(npm, "ci", "--ignore-scripts", cwd=ROOT / "frontend")
    call(npm, "run", "build", cwd=ROOT / "frontend")
    call("cargo", "build", "--locked", "--release", "--target-dir", build / "rust", "-j", args.jobs)
    shutil.copy2(build / "rust" / "release" / ("localscribe" + EXE), stage)
    src = retrieve(build / "vendor")
    for variant in ("generic", "avx2"):
        result = whisper(src, build, variant, args.jobs)
        target_name = ("whisper-cli" if variant == "generic" else "whisper-cli-avx2") + EXE
        shutil.copy2(result, stage / target_name)
    shutil.copy2(src / "LICENSE", stage / "whisper.cpp-LICENSE.txt")
    if args.vulkan_lab:
        result = whisper(src, build, "vulkan", args.jobs)
        shutil.copy2(result, stage / ("whisper-cli-vulkan" + EXE))


if __name__ == "__main__":
    main()
