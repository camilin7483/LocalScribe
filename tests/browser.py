#!/usr/bin/env python3
"""Drive the production UI against the native local transcription service."""
import argparse
import base64
import os
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
    before = player.evaluate("(audio) => audio.currentTime")
    page.get_by_role("button", name="Reproducir").click()
    expect(page.get_by_role("button", name="Pausar")).to_be_visible()
    page.wait_for_function(f"() => document.querySelector('audio').currentTime > {before + 0.2}", timeout=7000)
    page.get_by_role("button", name="Pausar").click()
    cue = page.locator(".cue-time").last
    cue.click()
    expected = result["segments"][-1]["start_ms"] / 1000
    assert abs(player.evaluate("(audio) => audio.currentTime") - expected) < 0.15
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
    with page.expect_download() as download:
        page.get_by_role("combobox", name="Exportar").select_option("srt")
    assert "Edición de prueba" in pathlib.Path(download.value.path()).read_text(encoding="utf-8")
    page.reload()
    expect(first).to_have_value(edited)
    expect(page.get_by_role("textbox", name="Hablante 1")).to_have_value("Hablante de prueba")
    if captures:
        page.screenshot(path=captures / "editor.png")

    nav(page, "Subtítulos")
    page.get_by_role("combobox", name="Elige una transcripción terminada").select_option(label="Prueba local.wav")
    expect(page.locator("pre.caption-preview")).to_contain_text("00:00:")
    with page.expect_download() as download:
        page.get_by_role("combobox", name="Exportar").select_option("vtt")
    assert pathlib.Path(download.value.path()).read_text(encoding="utf-8").startswith("WEBVTT")

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
    print("PASS browser navigation, drag and drop, batch import, real ASR, playback, edit, undo/redo, downloads, locale, theme and mobile layout")


if __name__ == "__main__":
    main()
