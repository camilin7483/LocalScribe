# Build from source

Use Rust 1.89 or newer, Node.js 22.12+ or 24 LTS, npm, Python 3.12+, CMake, a C/C++ compiler and FFmpeg with FFprobe and MP3 encoding. On Windows install Visual Studio Build Tools for Rust and put Ninja and the x64 UCRT build of LLVM-MinGW on PATH. The Windows workflow pins LLVM-MinGW 20261117 for the Whisper executables. On Linux install GCC or Clang and Make or Ninja. Python builds the packages and runs the tests. The installed core requires no Python, Node.js or Rust.

Run from the source directory.

```sh
python scripts/build_native.py
cargo test --locked
python scripts/fetch_models.py .release/models
```

The build verifies the whisper.cpp v1.9.5 source archive and builds a generic CPU engine and an AVX2 variant. It adds the standard `<algorithm>` include to the vendored example for libc++ compatibility. The browser assets are built in `frontend/dist`. For development, run that frontend build alongside the backend.

```sh
npm --prefix frontend ci --ignore-scripts
npm --prefix frontend run build
cargo run --locked -- --no-open
```

For a development launch, place the engine on PATH or set an absolute `LOCALSCRIBE_WHISPER` path first. Production builds resolve tools next to the executable and then via PATH, with the following environment overrides:

| Variable | Purpose |
|---|---|
| `LOCALSCRIBE_WHISPER`, `LOCALSCRIBE_WHISPER_AVX2` | Explicit executable path for either CPU engine. The generic override has precedence. |
| `LOCALSCRIBE_FFMPEG`, `LOCALSCRIBE_FFPROBE` | Override media tools. |
| `LOCALSCRIBE_DATA` | Data directory when `--data-dir` is absent. |
| `LOCALSCRIBE_MAX_UPLOAD_MIB` | Per-file limit. Defaults to 4096 MiB and is capped at 16384 MiB. |
| `LOCALSCRIBE_CHROMIUM` | Explicit Chromium executable for test and demo scripts. |

## Package and test

```sh
python scripts/package.py --platform linux --native .release/native
```

On Windows replace `linux` with `windows`. Extract the archive, then use the extracted executable with the test suite. With a packaged executable the server locates the adjacent `web` folder.

```sh
python -m pip install -r requirements-dev.txt
python -m playwright install chromium
python tests/integration.py --binary PATH_TO_LOCALSCRIBE --models .release/models --full
python tests/browser.py --binary PATH_TO_LOCALSCRIBE --models .release/models
```

The GitHub Actions workflow builds and smoke-tests the extracted Ubuntu and Windows packages on pushes and pull requests. Version tags and manual runs also test the full format matrix, browser workflow and recovery with both inference modes. A version tag such as `v0.1.0` must match the manifest. Passing jobs produce packages and a draft pre-release with checksums. Verify GUI launch and a clean setup on the target operating systems before publishing the draft. [GitHub hosted runner labels](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)

## Toolchains for the accompanying local artifacts

The Linux archive uses an official Rust 1.100.0 x86-64 musl target and CPU whisper.cpp linked statically using Zig 0.15.2. The Windows ZIP uses the official Rust 1.100.0 gnullvm target and LLVM-MinGW UCRT 20261117. It was smoke-tested with external FFmpeg 9.1 in Wine 11.22. CI builds Whisper with the system compiler on Linux and LLVM-MinGW UCRT 20261117 on Windows. The native Windows workflow uses Rust's default MSVC target for the server. All variants use the same frontend lockfile, backend lockfile and pinned whisper.cpp source.

### Experimental persistent Whisper worker

The native builder adds a small stdin/stdout protocol to the verified whisper.cpp release. Set `LOCALSCRIBE_WHISPER_PERSISTENT=1` on the LocalScribe process to test it. The app opens one worker for a multi-window job and exchanges a one-byte command and acknowledgment per window through private pipes. A failed worker is terminated and its current window is retried with the standard CLI. SQLite checkpoints still occur after successful windows. The default uses the standard CLI.

To exercise the packaged worker, comparison, and cancellation path:

```sh
python tests/worker.py --binary PATH_TO_LOCALSCRIBE --models .release/models --interrupt
```

See [0.2 engineering evidence](ROADMAP-0.2.md#inferencias-y-segundo-incremento) for measurement scope and acceptance criteria.

For a Vulkan benchmark CLI, install the Vulkan SDK, `glslc`, and the SPIRV-Headers CMake package. Run `python scripts/build_native.py --vulkan-lab`. For SDKs outside the system paths, include their prefixes in `CMAKE_PREFIX_PATH`. The build places `whisper-cli-vulkan` in the native output directory. Standard packaging contains the CPU variants.
