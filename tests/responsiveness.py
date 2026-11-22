#!/usr/bin/env python3
"""Measure editor search response while a real transcription is active."""
import argparse
import json
import math
from pathlib import Path
import sqlite3
from tempfile import TemporaryDirectory

from playwright.sync_api import expect, sync_playwright

from browser import browser_path
from local_support import FIXTURES, TestServer, arguments, ffmpeg


def main():
    parser = arguments(argparse.ArgumentParser(description=__doc__))
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    with TemporaryDirectory(prefix="localscribe-responsiveness-") as temporary:
        media = Path(temporary) / "background.wav"
        ffmpeg("-stream_loop", "7", "-i", FIXTURES / "es.wav", "-t", "93", "-c:a", "pcm_s16le", media)
        with TestServer(args.binary, args.models, args.web) as server, sync_playwright() as playwright:
            job = server.import_file(FIXTURES / "es.wav", {"profile": "eco"})
            detail = server.wait(job)
            assert detail["job"]["status"] == "completed"
            with sqlite3.connect(server.data / "localscribe.sqlite3", timeout=5) as storage:
                storage.executemany(
                    "INSERT INTO segments(job_id,start_ms,end_ms,text,original) VALUES (?,?,?,?,?)",
                    [(job["id"], index * 3, index * 3 + 2, f"Response fixture {index:04d}", f"Response fixture {index:04d}")
                     for index in range(3000)],
                )
            browser = playwright.chromium.launch(executable_path=browser_path(), headless=True, args=["--disable-background-networking"])
            page = browser.new_page(locale="en-US", viewport={"width": 1440, "height": 900})
            page.goto(f"{server.url}/#editor/{job['id']}")
            search = page.get_by_role("searchbox", name="Search transcript")
            expect(search).to_be_visible()
            expect(page.locator(".segment-list textarea")).to_have_count(100)
            page.evaluate("""() => {
              window.responseSamples = [];
              const field = document.querySelector('input[type="search"]');
              field.addEventListener('input', (event) => {
                requestAnimationFrame(() => requestAnimationFrame(() => {
                  window.responseSamples.push(performance.now() - event.timeStamp);
                }));
              });
            }""")
            pending = server.import_file(media, {"profile": "eco", "model": "tiny-q5_1", "language": "es"})
            server.wait(pending, lambda payload: payload["job"]["status"] == "running")
            try:
                for index in range(30):
                    value = f"Response fixture {index * 97:04d}"
                    search.fill(value)
                    expect(page.locator(".segment-list textarea")).to_have_value(value)
                    page.wait_for_function("(count) => window.responseSamples.length === count", arg=index + 1)
                timing = page.evaluate("window.responseSamples")
                remained_active = server.api("/jobs/" + pending["id"])["job"]["status"] == "running"
            finally:
                browser.close()
    assert remained_active, "The background job finished before the measurement ended"
    sorted_timing = sorted(timing)
    percentiles = {f"p{value}_ms": round(sorted_timing[math.ceil(len(timing) * value / 100) - 1], 1)
                   for value in (50, 95, 100)}
    result = {"method": "input event to second animation frame, synchronized to matching result",
              "synthetic_segments": 3000, "actuator": "Playwright Chromium", "application_jobs_active": True,
              **percentiles, "samples_ms": [round(value, 1) for value in timing]}
    report = json.dumps(result, indent=2) + "\n"
    if args.output:
        args.output.write_text(report, encoding="utf-8")
    print(report, end="")


if __name__ == "__main__":
    main()
