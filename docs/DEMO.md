# LocalScribe demo

The deliverable is `LocalScribe-Demo.mp4`. It is a silent 1920 × 1080 H.264 video at 30 fps, with 900 frames. The footage comes from the production browser application running on a local test instance. The import uses our Spanish speech fixture. The editing scene corrects a recognition mistake and downloads a generated SRT.

| Time | Real action shown | On-screen text |
|---|---|---|
| 00:00–00:03 | Open the dashboard. | Transcribe audio y video con IA local |
| 00:03–00:07 | Drop the fixture, configure the job and open its processing queue. | Importa audio y video fácilmente |
| 00:07–00:11 | Open Hardware on the i3-1215U machine and show the Balanced profile. | Optimizado automáticamente para tu PC |
| 00:11–00:16 | Show native inference in the queue and the resulting timed segments. | Transcripciones con marcas de tiempo |
| 00:16–00:20 | Show completed optional local analyses. The offline-only cut shows SRT cues in Subtitles. | Resúmenes, hablantes y traducción / Subtítulos sincronizados con tu audio |
| 00:20–00:24 | Edit the transcript, wait for autosave and download SRT. | Edita y exporta tus subtítulos |
| 00:24–00:27 | Open History and Privacy. | Privado, gratuito y sin suscripción |
| 00:27–00:30 | Return to the dashboard with the completed job. | LocalScribe — Open Source / Windows · Linux |

## Reproduce

Install the Playwright test dependency and Chromium, then build the native app and fetch the test models using [BUILD](BUILD.md). The recording host needs FFmpeg with libx264, the drawtext filter and a TrueType font.

```sh
python scripts/demo.py --binary PATH_TO_LOCALSCRIBE --models .release/models --output LocalScribe-Demo.mp4
ffprobe -v error -select_streams v:0 -show_entries stream=codec_name,width,height,avg_frame_rate,nb_frames,duration:format=duration -of json LocalScribe-Demo.mp4
```

The script drives the browser with Playwright and records its compositor through CDP. It touches only its isolated workspace. Add `--ai` when the optional engines are configured and verified on the recording machine. Add `--windows` for the two-platform closing text. Omit these flags for the core workflow cut. Raw recordings can be retained with `--recordings PATH`. The runtime wall time is edited to fit the chapter lengths.
