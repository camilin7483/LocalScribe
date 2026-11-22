@echo off
setlocal
where ffmpeg.exe >nul 2>&1
if not errorlevel 1 goto verify
where winget.exe >nul 2>&1
if errorlevel 1 goto wings
echo Installing FFmpeg from the official WinGet community source.
winget install --id Gyan.FFmpeg --exact --source winget
if errorlevel 1 goto wings
echo Close and reopen LocalScribe to pick up the updated PATH.
pause
exit /b 0
:verify
where ffprobe.exe >nul 2>&1
if errorlevel 1 goto wings
echo FFmpeg and FFprobe are already available.
pause
exit /b 0
:wings
echo Install the FFmpeg tools from https://ffmpeg.org/download.html
echo Put ffmpeg.exe and ffprobe.exe next to localscribe.exe, or add them to PATH.
pause
exit /b 1
