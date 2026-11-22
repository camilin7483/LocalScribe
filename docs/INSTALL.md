# Install LocalScribe

LocalScribe is packaged for x86-64. The browser UI runs on the same machine as the executable. Keep the extracted `web` folder and both Whisper executables beside LocalScribe.

## Windows 10 and 11

1. Extract the Windows ZIP. Open its folder and double-click `localscribe.exe`.
2. If **Hardware → Runtime diagnostics** reports missing FFmpeg tools, close LocalScribe. Run the included `Setup-FFmpeg.cmd` to install the tools with WinGet, then launch LocalScribe again. On a PC without WinGet, obtain FFmpeg from the linked Windows builds and place `ffmpeg.exe` and `ffprobe.exe` next to `localscribe.exe`.
3. Download the recommended Whisper model and Silero in **Models**.
4. Keep the LocalScribe process running while the browser is open. Close its console to stop the server.

The ZIP contains an unsigned portable build. Check [verification](VERIFICATION.md) for the platforms and environments that completed the smoke tests. [FFmpeg downloads](https://ffmpeg.org/download.html)

## Ubuntu 22.04 or newer

Install the runtime tools once in a terminal.

```sh
sudo apt update
sudo apt install ffmpeg xdg-utils ca-certificates
```

Extract the Linux archive, open a terminal in the extracted directory and run `./localscribe`. The optional `./install-menu.sh` adds a launcher for this directory to the desktop applications menu. Keep the extracted folder in place after adding the launcher.

## Arch Linux

```sh
sudo pacman -S ffmpeg xdg-utils ca-certificates
```

Extract and launch the Linux archive as described above. Installing `pciutils` gives AutoTune readable PCI graphics adapter names. The app will also run using its built-in CPU backend when a graphics adapter cannot be identified.

## First run and diagnostics

- Visit **Models** and download the recommended model. The Whisper download is about 32 MB to 190 MB. Silero requires less than 1 MB. Turn off voice activity detection for a job if you want to transcribe without Silero.
- **Hardware** shows missing executables and the chosen profile. Restart the server after installing tools. If the default port is taken, use `--port 0` and open the loopback address printed in the console.
- After the weights arrive, disconnect the network if desired. Use the same model files for offline installs by copying verified weights into the `models` directory before launch. See [model sources](ATTRIBUTIONS.md).
- **History** provides a delete action for each job. Stop the server before backing up its data directory. The default Linux location is `$XDG_DATA_HOME/localscribe` or `~/.local/share/localscribe`. On Windows it is under `%LOCALAPPDATA%\LocalScribe\LocalScribe\data` with this build's application-directory naming. Use `--data-dir PATH` to choose an explicit path.

Optional [Ollama and speaker identification setup](LOCAL_AI.md) is independent of transcription.
