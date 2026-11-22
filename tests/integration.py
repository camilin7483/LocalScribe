#!/usr/bin/env python3
"""Real API integration tests with FFmpeg, SQLite, whisper.cpp and optional local AI."""
import argparse
import json
import pathlib
import tempfile
import time

from local_support import FIXTURES, TestServer, arguments, ffmpeg

BASE = {"profile": "balanced", "model": "base-q5_1", "vad": True}
ECO = {"profile": "eco", "model": "tiny-q5_1", "vad": True}


def verify_privacy(server):
    assert server.raw("/system", headers={})[0] == 403
    assert server.raw("/system", headers={"Host": "unrelated.invalid", "X-LocalScribe": "1"})[0] == 403
    origin = {"Origin": "https://unrelated.invalid", "X-LocalScribe": "1"}
    assert server.raw("/settings", "PATCH", b'{"ollama_enabled":true}', {**origin, "Content-Type": "application/json"})[0] == 403
    assert server.raw("/system", "OPTIONS", headers=origin)[0] == 403
    status, headers, _ = server.raw("/system")
    assert status == 200 and headers["cache-control"] == "no-store"
    assert "frame-ancestors 'none'" in headers["content-security-policy"]
    assert "access-control-allow-origin" not in headers
    assert server.api("/engines")["models"] == []
    assert server.raw("/models/silero", "POST")[0] == 400
    print("PASS loopback Host, origin, custom header and security headers", flush=True)


def verify_model_download(server):
    import hashlib

    endpoint = "/models/silero"
    server.api(endpoint, "DELETE", expected=204)
    assert not next(model for model in server.api("/system")["models"] if model["id"] == "silero")["installed"]
    status, _, _ = server.raw(endpoint, "POST", headers={"X-LocalScribe": "1", "X-LocalScribe-Download": "1"})
    assert status == 202
    end = time.monotonic() + 110
    while time.monotonic() < end:
        model = next(model for model in server.api("/system")["models"] if model["id"] == "silero")
        if model["installed"]:
            with (server.data / "models" / "ggml-silero-v6.2.0.bin").open("rb") as local:
                assert hashlib.file_digest(local, "sha256").hexdigest() == "2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987"
            print("PASS model removal and verified Silero download over HTTPS", flush=True)
            return
        assert model["error"] is None, model
        time.sleep(0.3)
    raise TimeoutError("Silero download did not complete in the time allowed.")


def verify_language_edit_export(server, language="es", options=BASE):
    reference = (FIXTURES / f"{language}.txt").read_text(encoding="utf-8")
    imported = server.import_file(FIXTURES / f"{language}.wav", {**options, "benchmark": True, "reference": reference})
    result = server.wait(imported)
    job, segments = result["job"], result["segments"]
    assert job["status"] == "completed", job
    assert job["detected_language"] == language and job["processed_ms"] == job["duration_ms"]
    assert 0 <= job["metrics"]["wer"] < 0.5, job["metrics"]
    assert job["metrics"]["wall_ms"] > 0 and job["metrics"]["peak_rss_bytes"] > 0
    assert segments and 0 <= segments[0]["start_ms"] < segments[0]["end_ms"] <= job["duration_ms"]
    edits = {"id": segments[0]["id"], "text": "Texto comprobado en LocalScribe.", "speaker": "Hablante de prueba", "revision": 0}
    route = "/jobs/" + job["id"]
    for bad_timing in (
        {"start_ms": 0},
        {"start_ms": -1, "end_ms": 1},
        {"start_ms": 500, "end_ms": 500},
        {"start_ms": 0, "end_ms": job["duration_ms"] + 1},
    ):
        server.api(route + "/segments", "PATCH", {**edits, **bad_timing}, 400)
    assert server.api(route)["job"]["revision"] == 0
    assert server.api(route + "/segments", "PATCH", edits)["revision"] == 1
    server.api(route + "/segments", "PATCH", edits, 409)
    saved = server.api(route)["segments"][0]
    assert saved["original"] == segments[0]["original"] and saved["speaker"] == edits["speaker"]
    checks = {"txt": b"Texto comprobado", "srt": b" --> ", "vtt": b"WEBVTT", "md": b"- **", "json": b'"original"'}
    for format_name, marker in checks.items():
        status, headers, body = server.raw(route + "/export/" + format_name)
        assert status == 200 and marker in body and "attachment" in headers["content-disposition"]
    status, _, body = server.raw("/media/" + job["id"] + "/original", headers={"X-LocalScribe": "1", "Range": "bytes=0-3"})
    assert status == 206 and body == b"RIFF"
    assert any(row["id"] == job["id"] for row in server.api("/jobs?view=history")["jobs"])
    print(f"PASS {language}, timed segments, immutable original, edit revision, playback range, five exports and history", flush=True)
    return result


def verify_eco_auto(server):
    reference = (FIXTURES / "es.txt").read_text(encoding="utf-8")
    job = server.import_file(FIXTURES / "es.wav", {"profile": "eco", "benchmark": True, "reference": reference})
    result = server.wait(job)["job"]
    assert result["status"] == "completed" and result["detected_language"] == "es", result
    assert result["plan"]["model"] == "base-q5_1" and result["metrics"]["wer"] < 0.35, result
    print(f"PASS Eco automatic language regression, Base WER {result['metrics']['wer']:.1%}", flush=True)


def verify_media(server, directory, full):
    silent = directory / "silence.wav"
    ffmpeg("-f", "lavfi", "-i", "anullsrc=r=16000:cl=mono", "-t", "3", silent)
    result = server.wait(server.import_file(silent, ECO))
    assert result["job"]["status"] == "completed" and not result["segments"], result

    invalid = directory / "invalid.wav"
    invalid.write_bytes(b"This is ordinary text, not a sound file.")
    server.import_file(invalid, ECO, name="unsupported.txt", expected=400)
    malformed = server.wait(server.import_file(invalid, ECO))
    assert malformed["job"]["status"] == "failed"
    server.api("/jobs/" + malformed["job"]["id"], "DELETE", expected=204)

    cases = {
        "mp3": ["-c:a", "libmp3lame"], "flac": ["-c:a", "flac"], "m4a": ["-c:a", "aac"],
        "ogg": ["-c:a", "libopus"], "aac": ["-c:a", "aac"],
        "mp4": ["-c:v", "mpeg4", "-c:a", "aac"], "mkv": ["-c:v", "mpeg4", "-c:a", "aac"],
        "mov": ["-c:v", "mpeg4", "-c:a", "aac"], "webm": ["-c:v", "libvpx-vp9", "-c:a", "libopus"],
        "avi": ["-c:v", "mpeg4", "-c:a", "libmp3lame"],
    }
    formats = list(cases) if full else ["mp3", "mkv"]
    for ext in formats:
        path = directory / f"english.{ext}"
        video = ["-f", "lavfi", "-i", "color=c=0x203345:s=320x180:r=15", "-shortest"] if ext in ("mp4", "mkv", "mov", "webm", "avi") else []
        ffmpeg("-i", FIXTURES / "en.wav", *video, *cases[ext], path)
        detail = server.wait(server.import_file(path, {**BASE, "language": "en"}))
        assert detail["job"]["status"] == "completed" and detail["segments"], detail
        assert detail["job"]["detected_language"] == "en"
        recognized = " ".join(row["text"].lower() for row in detail["segments"])
        assert "good morning" in recognized and any(term in recognized for term in ("subtitles", "report", "thursday")), (ext, recognized)
        if ext == "mkv":
            server.api("/jobs/" + detail["job"]["id"] + "/preview", "POST", expected=204)
            assert server.raw("/media/" + detail["job"]["id"] + "/preview")[0] == 200
    if full:
        noisy = directory / "ruido-es.wav"
        ffmpeg("-i", FIXTURES / "es.wav", "-f", "lavfi", "-i", "anoisesrc=amplitude=0.035:sample_rate=16000:seed=246",
               "-filter_complex", "[0:a][1:a]amix=inputs=2:duration=first:weights=1 0.38", "-ac", "1", noisy)
        detail = server.wait(server.import_file(noisy, {**BASE, "language": "es"}))
        assert detail["job"]["status"] == "completed" and "subtítulo" in " ".join(s["text"].lower() for s in detail["segments"])
    print("PASS silence, invalid media, manual English, formats " + ", ".join(formats) + " and MKV audio preview" +
          (", plus noisy Spanish" if full else ""), flush=True)


def wait_for_resume(server, job_id):
    end = time.monotonic() + 15
    while time.monotonic() < end:
        try:
            server.api("/jobs/" + job_id + "/resume", "POST", expected=204)
            return
        except AssertionError as error:
            if "previous process" not in str(error):
                raise
            time.sleep(0.15)
    raise TimeoutError("Cancelled worker did not finish.")


def verify_checkpoint(server, directory, require_persistent=False):
    path = directory / "ventanas-largas.wav"
    speech = FIXTURES / "es.wav"
    ffmpeg("-i", speech, "-i", speech, "-filter_complex",
           "[0:a]apad=whole_dur=66[first];[first][1:a]concat=n=2:v=0:a=1,apad=whole_dur=93[a]",
           "-map", "[a]", "-t", "93", "-c:a", "pcm_s16le", path)
    job = server.import_file(path, {**ECO, "language": "es", "benchmark": True})
    route = "/jobs/" + job["id"]
    first = server.wait(job, lambda detail: detail["job"]["status"] == "running" and detail["job"]["processed_ms"] >= 30_000)
    first_ids = [(row["id"], row["start_ms"]) for row in first["segments"]]
    assert first_ids
    if require_persistent:
        assert first["job"]["metrics"]["stages"]["persistent_windows"] >= 1
    server.api(route + "/cancel", "POST", expected=204)
    stopped = server.api(route)
    assert stopped["job"]["status"] == "cancelled"
    checkpoint = stopped["job"]["processed_ms"]
    assert 30_000 <= checkpoint < 93_000
    wait_for_resume(server, job["id"])
    server.wait(job, lambda detail: detail["job"]["status"] == "running")
    server.stop()
    server.start()
    recovered = server.api(route)
    assert recovered["job"]["status"] == "interrupted"
    assert recovered["job"]["processed_ms"] >= checkpoint
    wait_for_resume(server, job["id"])
    finished = server.wait(job)
    assert finished["job"]["status"] == "completed" and finished["job"]["processed_ms"] == 93_000
    ids = {row["id"]: row["start_ms"] for row in finished["segments"]}
    assert all(ids[segment_id] == start_ms for segment_id, start_ms in first_ids)
    assert any(start_ms >= 60_000 for start_ms in ids.values()), finished
    print(f"PASS 93-second file, real checkpoint at {checkpoint} ms, cancel, server restart and resume", flush=True)


def verify_ai(server, proof, use_diarization):
    job = proof["job"]
    route = "/jobs/" + job["id"]
    edits = server.api(route)["job"]["revision"]
    body = {"mode": "ask", "model": "", "target": "en", "question": "¿Cuándo se enviará el informe?", "locale": "es", "revision": edits}
    server.api("/settings", "PATCH", {"ollama_enabled": True})
    engines = server.api("/engines")
    assert engines["models"], "Ollama test requested but no local model is installed."
    body["model"] = engines["models"][0]
    result = server.api(route + "/analyze", "POST", body, timeout=250)
    assert "jueves" in result["text"].lower() and all(item["text"] for item in result["evidence"]), result
    absent = server.api(route + "/analyze", "POST", {**body, "question": "¿Cuál es la dirección postal de la oficina?"}, timeout=250)
    assert not absent["evidence"], absent
    for mode in ("general", "executive", "key_points", "decisions", "tasks", "questions", "translate", "correct"):
        result = server.api(route + "/analyze", "POST", {**body, "mode": mode}, timeout=250)
        assert result["text"] and "Ollama" in result["title"], (mode, result)
        if mode == "translate":
            assert len(result["text"].split("\n\n")) >= len(proof["segments"]), result
    server.api("/settings", "PATCH", {"ollama_enabled": False})
    if use_diarization:
        assert engines["diarization"]
        result = server.api(route + "/analyze", "POST", {**body, "mode": "speakers"}, timeout=140)
        assert any(change["speaker"].startswith("Hablante ") for change in result["changes"]), result
        server.api(route + "/suggestions", "PATCH", {"revision": edits, "changes": result["changes"]})
        assert any(row["speaker"] for row in server.api(route)["segments"])
        dialogue = server.wait(server.import_file(FIXTURES / "dialogue.wav", {**BASE, "language": "en", "vad": False}))
        assert dialogue["job"]["status"] == "completed"
        dialogue_route = "/jobs/" + dialogue["job"]["id"]
        labeled = server.api(dialogue_route + "/analyze", "POST", {**body, "mode": "speakers", "speaker_count": 2, "revision": 0}, timeout=140)
        assert len({change["speaker"] for change in labeled["changes"]}) == 2, labeled
    print("PASS local Ollama summary modes, translation, corrections and verified quotes" +
          (", plus sherpa-onnx speaker proposals" if use_diarization else ""), flush=True)


def main():
    parser = arguments(argparse.ArgumentParser(description=__doc__))
    parser.add_argument("--eco-regression", action="store_true")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--full", action="store_true", help="All formats, noise and restart/resume test.")
    mode.add_argument("--smoke", action="store_true", help="A short real Tiny transcription with privacy, editing and export checks.")
    parser.add_argument("--download", action="store_true", help="Exercise a real model download with internet access.")
    parser.add_argument("--ai", action="store_true", help="Exercise the installed Ollama model.")
    parser.add_argument("--diarization", action="store_true", help="Exercise the configured sherpa-onnx CLI with --ai.")
    parser.add_argument("--ai-only", action="store_true", help="Run the core transcript and AI checks only.")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="localscribe-fixtures-") as temporary:
        directory = pathlib.Path(temporary)
        with TestServer(args.binary, args.models, args.web, args.data) as server:
            if not args.ai_only:
                verify_privacy(server)
            facts = server.api("/system")
            assert all(facts["diagnostics"].values()), facts["diagnostics"]
            assert facts["hardware"]["logical"] > 0 and facts["hardware"]["ram_mib"] > 0 and facts["hardware"]["calibration"]
            if args.download:
                verify_model_download(server)
            if args.eco_regression:
                verify_eco_auto(server)
            language = "en" if args.smoke else "es"
            options = {**ECO, "language": "en"} if args.smoke else BASE
            proof = verify_language_edit_export(server, language, options)
            if not args.ai_only and not args.smoke:
                verify_media(server, directory, args.full)
                if args.full:
                    verify_checkpoint(server, directory)
            if args.ai:
                # Analyze the unedited synthetic transcript to keep the expected result in the text.
                original = proof["segments"][0]
                route = "/jobs/" + proof["job"]["id"]
                server.api(route + "/segments", "PATCH", {"id": original["id"], "text": original["original"], "speaker": None, "revision": 1})
                verify_ai(server, proof, args.diarization)
            print(json.dumps({"sample": f"{language}.wav", "metrics": proof["job"]["metrics"], "hardware": facts["hardware"]}, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
