# Benchmarks

The benchmark runner feeds real audio to the packaged API and uses the same SQLite checkpoint measurements as the Benchmarks screen. The reference machine in this project is the 12th Gen Intel Core i3-1215U with 6 physical cores, 8 logical CPUs, 16 GB RAM and Intel UHD graphics. Whisper uses the CPU on this machine.

## Reproduce

```sh
python scripts/fetch_models.py .release/models --all
python scripts/benchmark.py --binary PATH_TO_LOCALSCRIBE --models .release/models --repetitions 2 --output results.json
```

Defaults are the included Spanish synthetic recording, its reference text, VAD on, automatic language detection and one job at a time. The runner pairs Tiny Q5_1 with Eco, Base Q5_1 with Balanced and Small Q5_1 with Precision. Use `--media`, `--reference` and `--model` to try other input. Close CPU-intensive work for a repeatable comparison. These measurements use one short clip. Longer recordings, natural speech, background noise and differing CPUs can change the results.

| Metric | Measurement |
|---|---|
| Pipeline time | FFprobe, FFmpeg and Whisper process wall times summed over committed windows, including loading a model inside Whisper. |
| Elapsed time | Client clock from upload start until the terminal result is observed, including polling delay and queue time. |
| RTF | Pipeline seconds divided by media seconds. Smaller is faster for this sample. |
| Peak child RSS | Maximum sampled resident set of a single media or Whisper child, not the total memory of simultaneous jobs or the browser. |
| CPU time | Sampled cumulative CPU milliseconds of measured processes. Dividing by pipeline time gives percent of one core. Short final intervals may be missed. |
| WER | `(substitutions + insertions + deletions) / reference words`, case folded with words split on non-alphanumeric characters. Original Whisper output is scored. |

Startup AutoTune runs a separate fixed dot-product microbenchmark at 1, 2, 4 and up to 8 usable cores. It selects the smallest thread count within 86% of the highest eligible measured rate for the chosen profile. The calibration is a CPU scheduling heuristic. The transcription timings above are the real workload measurement.

## Recorded run

Linux x86-64, CPU AVX2 build, 11.786-second Spanish test recording, two trials per model. The workstation was also running other project verification processes. The timings describe this run and can vary with system load.

| Model and profile | Threads | Median processing time | Median RTF | Highest sampled RSS | Median WER |
|---|---:|---:|---:|---:|---:|
| tiny-q5_1 / eco | 2 | 3.53 s | 0.300× | 103.6 MiB | 100.0% |
| base-q5_1 / balanced | 2 | 10.63 s | 0.902× | 183.1 MiB | 7.7% |
| small-q5_1 / precision | 6 | 16.87 s | 1.432× | 463.6 MiB | 7.7% |

Tiny had the lowest measured memory use. Automatic language detection with Tiny on this clip produced 100% WER. The long-recording integration used Tiny with manually selected Spanish and completed multiple windows.

[Raw per-trial measurements](benchmark-i3.json)
