#!/usr/bin/env python3
"""Drive the production UI against the native local transcription service."""
import argparse
import base64
import os
import sqlite3
import pathlib
from urllib.parse import urlsplit

from playwright.sync_api import expect, sync_playwright

from local_support import FIXTURES, TestServer, arguments


def browser_path():
    return os.environ.get("LOCALSCRIBE_CHROMIUM") or None


def drop_file(page, path, name=None, show_drag=False):
    transfer = page.evaluate_handle(
        """({name, encoded}) => {
          const data = new DataTransfer();
          const bytes = Uint8Array.from(atob(encoded), c => c.charCodeAt(0));
          data.items.add(new File([bytes], name, {type:'audio/wav'}));
          return data;
        }""",
        {"name": name or path.name, "encoded": base64.b64encode(path.read_bytes()).decode()},
    )
    zone = page.get_by_test_id("dropzone")
    zone.dispatch_event("dragenter", {"dataTransfer": transfer})
    zone.dispatch_event("dragover", {"dataTransfer": transfer})
    if show_drag:
        page.wait_for_timeout(350)
    zone.dispatch_event("drop", {"dataTransfer": transfer})
    transfer.dispose()


def nav(page, title):
    page.get_by_role("navigation").get_by_role("link", name=title, exact=True).click()


def run(page, server, captures):
    page.goto(server.url)
    page.wait_for_load_state("networkidle")
    expect(page.get_by_role("heading", name="Tu estudio de transcripción")).to_be_visible()
    if captures:
        page.screenshot(path=captures / "dashboard.png")
    nav(page, "Hardware")
    expect(page.get_by_role("heading", name="Hardware", exact=True)).to_be_visible()
    expect(page.get_by_role("button", name="Equilibrado")).to_have_attribute("aria-pressed", "true")
    expect(page.get_by_role("heading", name="Diagnóstico de dependencias")).to_be_visible()
    nav(page, "Inicio")
    drop_file(page, FIXTURES / "es.wav", "Prueba local.wav")
    expect(page.get_by_text("Prueba local.wav", exact=True)).to_be_visible()
    page.locator('input[type="file"]').set_input_files(FIXTURES / "en.wav")
    expect(page.get_by_text("en.wav", exact=True)).to_be_visible()
    page.get_by_role("combobox", name="Modelo Whisper").select_option("base-q5_1")
    page.get_by_role("button", name="Iniciar transcripción").click()
    expect(page.get_by_role("heading", name="Cola de procesamiento", exact=True)).to_be_visible()
    jobs = server.api("/jobs")["jobs"]
    assert len(jobs) == 2
    media_job = next(job for job in jobs if job["name"] == "Prueba local.wav")
    result = server.wait(media_job)
    assert result["job"]["status"] == "completed" and result["segments"]
    for job in jobs:
        assert server.wait(job)["job"]["status"] == "completed"

    nav(page, "Historial")
    page.get_by_role("link", name="Prueba local.wav", exact=True).click()
    expect(page.get_by_role("heading", name="Prueba local.wav")).to_be_visible()
    player = page.locator("audio")
    page.wait_for_function("() => document.querySelector('audio')?.readyState >= 1", timeout=12_000)
    wave = page.get_by_test_id("studio-waveform")
    expect(wave).to_be_visible()
    assert wave.evaluate("(node) => node.getContext('2d').getImageData(0, 0, node.width, node.height).data.some(value => value > 0)")
    expect(page.get_by_role("complementary", name="Revisión de subtítulos")).to_be_visible()
    page.get_by_label("Velocidad").select_option("1.25")
    assert player.evaluate("(audio) => audio.playbackRate") == 1.25
    before = player.evaluate("(audio) => audio.currentTime")
    page.get_by_role("button", name="Reproducir").click()
    expect(page.get_by_role("button", name="Pausar")).to_be_visible()
    page.wait_for_function(f"() => document.querySelector('audio').currentTime > {before + 0.2}", timeout=7000)
    page.get_by_role("button", name="Pausar").click()
    cue = page.locator(".cue-time").last
    cue.click()
    expected = result["segments"][-1]["start_ms"] / 1000
    assert abs(player.evaluate("(audio) => audio.currentTime") - expected) < 0.15
    expect(page.locator(".studio-active-cue")).to_contain_text(result["segments"][-1]["text"])
    follow = page.get_by_role("button", name="Seguir reproducción")
    follow.click()
    expect(follow).to_have_attribute("aria-pressed", "true")
    follow.click()
    loop = page.get_by_role("button", name="Repetir segmento", exact=False)
    loop.click()
    expect(loop).to_have_attribute("aria-pressed", "true")
    loop.click()
    expect(loop).to_have_attribute("aria-pressed", "false")
    original_cue = result["segments"][0]
    cue_id = original_cue["id"]
    route = "/jobs/" + media_job["id"]
    row = page.locator(f"#segment-{cue_id}")
    edit_timing = row.get_by_role("button", name="Ajustar tiempos")
    edit_timing.click()
    start_field = row.get_by_role("textbox", name="Inicio", exact=True)
    end_field = row.get_by_role("textbox", name="Final", exact=True)
    apply_timing = row.get_by_role("button", name="Aplicar tiempos")
    expect(start_field).to_be_focused()
    original_start = start_field.input_value()
    original_end = end_field.input_value()
    row.get_by_role("button", name="Inicio en posición actual").click()
    expect(start_field).to_have_value(original_start if len(result["segments"]) == 1 else
                                     "00:" + page.locator(".cue-time").last.inner_text())
    start_field.fill(original_start)
    end_field.fill(original_start)
    apply_timing.click()
    expect(row.get_by_role("alert")).to_have_text("El final debe ser posterior al inicio.")
    expect(end_field).to_have_attribute("aria-invalid", "true")
    assert server.api(route)["job"]["revision"] == result["job"]["revision"]
    changed_start_ms = original_cue["start_ms"] + 120
    changed_end_ms = min(original_cue["end_ms"], changed_start_ms + 600)

    def timecode(ms):
        return f"{ms // 3600000:02d}:{ms // 60000 % 60:02d}:{ms // 1000 % 60:02d}.{ms % 1000:03d}"

    start_field.fill(timecode(changed_start_ms).replace(".", ","))
    end_field.fill(timecode(changed_end_ms))
    if captures:
        row.screenshot(path=captures / "cue-timing.png")
    apply_timing.click()
    expect(edit_timing).to_be_focused()
    expect(page.get_by_text("Guardado en este equipo", exact=True)).to_be_visible()
    cue = next(segment for segment in server.api(route)["segments"] if segment["id"] == cue_id)
    assert (cue["start_ms"], cue["end_ms"]) == (changed_start_ms, changed_end_ms)
    assert (cue["original_start_ms"], cue["original_end_ms"]) == (original_cue["start_ms"], original_cue["end_ms"])
    page.get_by_role("button", name="Deshacer", exact=True).click()
    expect(row.locator(".cue-time")).to_have_text(original_start[3:])
    page.get_by_role("button", name="Rehacer", exact=True).click()
    expect(row.locator(".cue-time")).to_have_text(timecode(changed_start_ms)[3:])
    edit_timing.click()
    row.get_by_role("button", name="Recuperar tiempos originales").click()
    expect(start_field).to_have_value(original_start)
    expect(end_field).to_have_value(original_end)
    end_field.press("Escape")
    expect(edit_timing).to_be_focused()
    first = page.get_by_role("textbox", name="Segmento 1", exact=True)
    original = first.input_value()
    assert "transcripción" in original.lower()
    edited = original + " Edición de prueba."
    first.fill(edited)
    expect(page.get_by_text("Guardado en este equipo", exact=True)).to_be_visible()
    page.get_by_role("button", name="Deshacer", exact=True).click()
    expect(first).to_have_value(original)
    page.get_by_role("button", name="Rehacer", exact=True).click()
    expect(first).to_have_value(edited)
    page.get_by_role("textbox", name="Hablante 1").fill("Hablante de prueba")
    page.get_by_role("button", name="Ver original").click()
    expect(page.locator("blockquote").first).to_have_text(original)
    page.get_by_role("searchbox", name="Buscar en la transcripción").fill("zz-sin-coincidencia")
    expect(page.locator(".segment-list").locator("textarea")).to_have_count(0)
    page.get_by_role("searchbox", name="Buscar en la transcripción").fill("")
    export = page.get_by_role("combobox", name="Exportar")
    export.select_option("srt")
    check = page.get_by_role("dialog", name="Revisar antes de exportar")
    expect(check).to_contain_text("20 caracteres por segundo")
    if captures:
        page.screenshot(path=captures / "caption-export-check.png")
    check.get_by_role("button", name="Ir al primer segmento").click()
    expect(check).not_to_be_visible()
    expect(first).to_be_focused()
    export.select_option("srt")
    with page.expect_download() as download:
        check.get_by_role("button", name="Exportar SRT").click()
    srt = pathlib.Path(download.value.path()).read_text(encoding="utf-8")
    assert "Edición de prueba" in srt
    assert f"{timecode(changed_start_ms)} --> {timecode(changed_end_ms)}".replace(".", ",") in srt
    page.reload()
    expect(first).to_have_value(edited)
    expect(page.get_by_role("textbox", name="Hablante 1")).to_have_value("Hablante de prueba")
    if captures:
        expect(page.get_by_test_id("studio-waveform")).to_be_visible()
        page.screenshot(path=captures / "editor.png")

    nav(page, "Subtítulos")
    page.get_by_role("combobox", name="Elige una transcripción terminada").select_option(label="Prueba local.wav")
    expect(page.locator("pre.caption-preview")).to_contain_text("00:00:")
    with page.expect_download() as download:
        page.get_by_role("combobox", name="Exportar").select_option("vtt")
    vtt = pathlib.Path(download.value.path()).read_text(encoding="utf-8")
    assert vtt.startswith("WEBVTT")
    assert f"{timecode(changed_start_ms)} --> {timecode(changed_end_ms)}" in vtt

    nav(page, "IA local")
    expect(page.get_by_text("Activa Ollama local en Privacidad para usar estas acciones.")).to_be_visible()
    expect(page.get_by_role("button", name="Ejecutar localmente")).to_be_disabled()
    nav(page, "Privacidad")
    expect(page.get_by_role("switch")).to_have_attribute("aria-checked", "false")
    page.get_by_role("button", name="Tema claro").click()
    expect(page.locator("html")).to_have_attribute("data-theme", "light")
    page.get_by_role("button", name="Cambiar idioma").click()
    expect(page.get_by_role("heading", name="Privacy", exact=True)).to_be_visible()
    page.set_viewport_size({"width": 390, "height": 844})
    page.get_by_role("button", name="Menu").click()
    nav(page, "File library")
    expect(page.get_by_role("heading", name="File library")).to_be_visible()
    assert page.evaluate("document.documentElement.scrollWidth <= innerWidth"), "Horizontal overflow in the mobile layout"
    if captures:
        page.screenshot(path=captures / "mobile.png")

    # Populate the isolated workspace to exercise editor pagination separately from recognition.
    with sqlite3.connect(server.data / "localscribe.sqlite3", timeout=5) as storage:
        storage.executemany(
            "INSERT INTO segments(job_id, start_ms, end_ms, text, original) VALUES (?, ?, ?, ?, ?)",
            [(media_job["id"], n * 50, n * 50 + 49, f"Pagination fixture {n}", f"Pagination fixture {n}") for n in range(125)],
        )
    page.goto(server.url + "/#editor/" + media_job["id"])
    rows = page.locator(".segment-list textarea")
    expect(rows).to_have_count(100)
    pagination = page.get_by_role("navigation", name="Transcript pages")
    pagination.get_by_role("button").last.click()
    expect(rows).to_have_count(125 + len(result["segments"]) - 100)
    page.get_by_role("searchbox", name="Search transcript").fill("Pagination fixture 124")
    expect(rows).to_have_count(1)
    expect(rows.first).to_have_value("Pagination fixture 124")



def main():
    parser = arguments(argparse.ArgumentParser(description=__doc__))
    parser.add_argument("--captures", type=pathlib.Path)
    args = parser.parse_args()
    if args.captures:
        args.captures.mkdir(parents=True, exist_ok=True)
    with TestServer(args.binary, args.models, args.web, args.data) as server, sync_playwright() as playwright:
        browser = playwright.chromium.launch(executable_path=browser_path(), headless=True, args=["--disable-background-networking"])
        context = browser.new_context(locale="es-CO", viewport={"width": 1536, "height": 864}, reduced_motion="reduce", accept_downloads=True)
        page = context.new_page()
        failures = []
        external = []
        page.on("pageerror", lambda error: failures.append(str(error)))
        page.on("request", lambda request: external.append(request.url) if urlsplit(request.url).hostname not in {"127.0.0.1", None} else None)
        page.on("response", lambda response: failures.append(f"{response.status} {response.url}") if response.status >= 400 else None)
        try:
            run(page, server, args.captures)
            assert not failures, failures
            assert not external, external
        finally:
            browser.close()
    print("PASS browser navigation, drag and drop, batch import, real ASR, playback, timing and text edits, undo/redo, downloads, locale, theme, mobile layout and long-transcript pagination")


if __name__ == "__main__":
    main()
