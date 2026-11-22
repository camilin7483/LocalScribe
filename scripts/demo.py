#!/usr/bin/env python3
"""Record the real app and render the eight chapters of the 30-second demo."""
import argparse
import base64
import bisect
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tests"))
from browser import browser_path, drop_file, nav
from local_support import FIXTURES, TestServer, arguments
from urllib.parse import urlsplit
from playwright.sync_api import expect, sync_playwright

FPS = 30
WIDTH = 1600
HEIGHT = 820
RECORDING = "Prueba de transcripción.wav"


class Capture:
    """Keep browser screencast frames and the time they were visible."""
    def __init__(self, page, directory, name):
        self.page = page
        self.directory = directory / name
        self.directory.mkdir()
        self.frames = []
        self.session = page.context.new_cdp_session(page)

    def record_frame(self, event):
        target = self.directory / f"capture-{len(self.frames):04d}.jpg"
        target.write_bytes(base64.b64decode(event["data"]))
        self.frames.append((time.monotonic() - self.start, target))
        self.session.send("Page.screencastFrameAck", {"sessionId": event["sessionId"]})

    def __enter__(self):
        still = self.directory / "start.jpg"
        self.page.screenshot(path=still, type="jpeg", quality=96)
        self.frames.append((0, still))
        self.start = time.monotonic()
        self.session.on("Page.screencastFrame", self.record_frame)
        self.session.send("Page.startScreencast", {
            "format": "jpeg", "quality": 93, "maxWidth": WIDTH, "maxHeight": HEIGHT, "everyNthFrame": 1,
        })
        return self

    def __exit__(self, *_):
        self.session.send("Page.stopScreencast")
        self.session.detach()
        still = self.directory / "end.jpg"
        self.page.screenshot(path=still, type="jpeg", quality=96)
        self.frames.append((time.monotonic() - self.start, still))

    def sequence(self, count, hold=0.35):
        stamps = [stamp for stamp, _ in self.frames]
        stop = stamps[-1] + hold
        for index in range(count):
            tick = stop * index / max(1, count - 1)
            _, path = self.frames[max(0, bisect.bisect_right(stamps, tick) - 1)]
            yield path


def shot(page, directory, name, delay=400, action=None):
    with Capture(page, directory, name) as capture:
        if action:
            action()
        page.wait_for_timeout(delay)
    return capture


def real_chapters(server, page, directory, include_ai, windows):
    page.goto(server.url)
    expect(page.get_by_role("heading", name="Tu estudio de transcripción")).to_be_visible()
    page.wait_for_load_state("networkidle")
    first = shot(page, directory, "overview")

    def import_media():
        drop_file(page, FIXTURES / "es.wav", RECORDING, show_drag=True)
        expect(page.get_by_text(RECORDING, exact=True)).to_be_visible()
        page.get_by_role("combobox", name="Modelo Whisper").select_option("base-q5_1")
        page.wait_for_timeout(420)
        page.get_by_role("button", name="Iniciar transcripción").click()
        expect(page.get_by_role("heading", name="Cola de procesamiento", exact=True)).to_be_visible()
        expect(page.get_by_text(RECORDING, exact=True)).to_be_visible()
    imported = shot(page, directory, "import", 350, import_media)
    recording = next(job for job in server.api("/jobs")["jobs"] if job["name"] == RECORDING)
    queue = shot(page, directory, "queue", 500)

    nav(page, "Hardware")
    expect(page.get_by_role("heading", name="Diagnóstico de dependencias")).to_be_visible()
    hardware = shot(page, directory, "tune", 350, lambda: page.get_by_role("button", name="Equilibrado").click())

    detail = server.wait(recording)
    assert detail["job"]["status"] == "completed" and detail["segments"], detail
    editor = server.url + "/#editor/" + recording["id"]
    page.goto(editor)
    expect(page.get_by_role("textbox", name="Segmento 1", exact=True)).to_be_visible()
    page.wait_for_function("() => document.querySelector('audio')?.readyState >= 1")
    expect(page.get_by_test_id("studio-waveform")).to_be_visible()

    def play_caption():
        page.locator(".cue-time").first.click()
        page.get_by_role("button", name="Reproducir", exact=True).click()
        expect(page.locator(".studio-active-cue")).to_be_visible()
        page.wait_for_timeout(450)
        page.get_by_role("button", name="Pausar", exact=True).click()

    transcript = shot(page, directory, "transcript", 350, play_caption)

    if include_ai:
        nav(page, "Privacidad")
        page.get_by_role("switch", name="Permitir Ollama local").click()
        expect(page.get_by_role("switch")).to_have_attribute("aria-checked", "true")
        nav(page, "IA local")
        page.get_by_role("combobox", name="Elige una transcripción terminada").select_option(label=RECORDING)
        page.get_by_role("button", name="Ejecutar localmente").click()
        expect(page.get_by_test_id("analysis-result")).to_contain_text("Ollama", timeout=245_000)
        summary = shot(page, directory, "summary")
        page.get_by_role("combobox", name="Acción").select_option("translate")
        page.get_by_role("button", name="Ejecutar localmente").click()
        expect(page.get_by_test_id("analysis-result")).to_contain_text("Ollama", timeout=245_000)
        translation = shot(page, directory, "translation")
        if server.api("/engines")["diarization"]:
            page.get_by_role("combobox", name="Acción").select_option("speakers")
            page.get_by_role("button", name="Ejecutar localmente").click()
            expect(page.get_by_test_id("analysis-result")).to_contain_text("Hablante", timeout=140_000)
            speakers = shot(page, directory, "speakers")
            intelligence = ([(summary, 42), (speakers, 36), (translation, 42)], "Resúmenes, hablantes y traducción")
        else:
            intelligence = ([(summary, 60), (translation, 60)], "Resumen y traducción en el equipo")
        nav(page, "Privacidad")
        page.get_by_role("switch", name="Permitir Ollama local").click()
        expect(page.get_by_role("switch")).to_have_attribute("aria-checked", "false")
    else:
        nav(page, "Subtítulos")
        page.get_by_role("combobox", name="Elige una transcripción terminada").select_option(label=RECORDING)
        expect(page.locator(".caption-preview")).to_contain_text(" --> ")
        intelligence = ([(shot(page, directory, "captions"), 120)], "Subtítulos sincronizados con tu audio")

    page.goto(editor)
    first_line = page.get_by_role("textbox", name="Segmento 1", exact=True)
    expect(first_line).to_be_visible()

    def edit_and_export():
        first_cue = detail["segments"][0]
        cue = page.locator(f"#segment-{first_cue['id']}")
        cue.get_by_role("button", name="Ajustar tiempos").click()
        start = cue.get_by_role("textbox", name="Inicio", exact=True)
        moved = first_cue["start_ms"] + 120
        clock = f"{moved // 3600000:02d}:{moved // 60000 % 60:02d}:{moved // 1000 % 60:02d}.{moved % 1000:03d}"
        start.fill(clock)
        page.wait_for_timeout(450)
        cue.get_by_role("button", name="Aplicar tiempos").click()
        last = page.locator(".segment-list textarea").last
        before = last.input_value()
        fixed = before.replace("A un activo", "Cada archivo")
        if fixed == before:
            fixed = before.rstrip(".") + "."
            if fixed == before:
                first_line.fill(first_line.input_value().replace("informe, el", "informe el") + " Grabación de prueba.")
            else:
                last.fill(fixed)
        else:
            last.fill(fixed)
        expect(page.get_by_text("Guardado en este equipo", exact=True)).to_be_visible()
        with page.expect_download() as download:
            page.get_by_role("combobox", name="Exportar").select_option("srt")
            review = page.get_by_role("dialog", name="Revisar antes de exportar")
            if review.is_visible():
                review.get_by_role("button", name="Exportar SRT").click()
        output = pathlib.Path(download.value.path()).read_text(encoding="utf-8")
        assert clock.replace(".", ",") + " --> " in output and "subtítulo" in output.lower(), output
        expect(page.get_by_text("Exportación descargada · SRT")).to_be_visible()
    edited = shot(page, directory, "edit", 500, edit_and_export)

    nav(page, "Historial")
    expect(page.get_by_role("link", name=RECORDING, exact=True)).to_be_visible()
    history = shot(page, directory, "history")
    nav(page, "Privacidad")
    expect(page.get_by_role("heading", name="Funciona sin conexión")).to_be_visible()
    privacy = shot(page, directory, "privacy")
    nav(page, "Inicio")
    expect(page.get_by_text(RECORDING, exact=True)).to_be_visible()
    closing = shot(page, directory, "closing")
    platforms = "Windows · Linux" if windows else "Linux · Windows en pruebas"

    return [
        ([(first, 90)], "Transcribe audio y video con IA local"),
        ([(imported, 120)], "Importa audio y video fácilmente"),
        ([(hardware, 120)], "Optimizado automáticamente para tu PC"),
        ([(queue, 53), (transcript, 97)], "Transcripciones con marcas de tiempo"),
        intelligence,
        ([(edited, 120)], "Edita y exporta tus subtítulos"),
        ([(history, 40), (privacy, 50)], "Privado, gratuito y sin suscripción"),
        ([(closing, 90)], "LocalScribe — Open Source\n" + platforms),
    ]


def font_path(explicit):
    candidates = (explicit, "/usr/share/fonts/TTF/DejaVuSans.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                  "C:/Windows/Fonts/segoeui.ttf")
    return next(pathlib.Path(item) for item in candidates if item and pathlib.Path(item).is_file())


def render(chapters, directory, output, font):
    font_to = directory / "caption.ttf"
    shutil.copyfile(font_path(font), font_to)
    filters = [
        "scale=1920:984:flags=lanczos", "pad=1920:1080:0:0:color=0x10151d", "setsar=1",
        "drawbox=x=0:y=984:w=iw:h=2:color=0x79e2c5@0.6:t=fill",
    ]
    frame = 0
    for index, (pieces, caption) in enumerate(chapters):
        start = frame
        for source, count in pieces:
            for image in source.sequence(count, 1.1 if index in {1, 5} else .45):
                target = directory / f"frame-{frame:04d}.jpg"
                try: os.link(image, target)
                except OSError: shutil.copyfile(image, target)
                frame += 1
        caption_file = f"caption-{index}.txt"
        (directory / caption_file).write_text(caption, encoding="utf-8")
        filters.append(
            f"drawtext=fontfile=caption.ttf:textfile={caption_file}:fontsize={34 if index == 7 else 39}:"
            f"fontcolor=0xf1f5f7:line_spacing=6:x=(w-tw)/2:y=984+(96-th)/2:enable='between(n,{start},{frame - 1})'"
        )
    assert frame == 900
    subprocess.run([
        "ffmpeg", "-v", "error", "-y", "-framerate", str(FPS), "-i", "frame-%04d.jpg",
        "-vf", ",".join(filters), "-frames:v", "900", "-an", "-c:v", "libx264", "-threads", "3",
        "-preset", "veryfast", "-crf", "17", "-pix_fmt", "yuv420p", "-movflags", "+faststart", str(output),
    ], cwd=directory, check=True)


def main():
    parser = arguments(argparse.ArgumentParser(description=__doc__))
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--recordings", type=pathlib.Path)
    parser.add_argument("--font")
    parser.add_argument("--ai", action="store_true", help="Show actual output from the configured AI engines.")
    parser.add_argument("--windows", action="store_true", help="Use the Windows and Linux closing title after verifying both builds.")
    args = parser.parse_args()
    directory = args.recordings or pathlib.Path(tempfile.mkdtemp(prefix="localscribe-demo-"))
    directory.mkdir(parents=True, exist_ok=True)
    directory = directory.resolve()
    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    try:
        with TestServer(args.binary, args.models, args.web, args.data) as server, sync_playwright() as playwright:
            browser = playwright.chromium.launch(executable_path=browser_path(), headless=True, args=["--disable-background-networking"])
            page = browser.new_page(locale="es-CO", viewport={"width": WIDTH, "height": HEIGHT}, device_scale_factor=1,
                                    reduced_motion="reduce", accept_downloads=True)
            try:
                external = []
                page.on("request", lambda request: external.append(request.url)
                        if urlsplit(request.url).hostname not in (None, "127.0.0.1") else None)
                chapters = real_chapters(server, page, directory, args.ai, args.windows)
                assert not external, external
            finally:
                browser.close()
        render(chapters, directory, output, args.font)
    finally:
        if not args.recordings:
            shutil.rmtree(directory)
    print(output)


if __name__ == "__main__":
    main()
