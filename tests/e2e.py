# /// script
# requires-python = ">=3.11"
# dependencies = ["playwright>=1.54,<2"]
# ///

import json
import os
import re
from pathlib import Path

from playwright.sync_api import sync_playwright

# Point at the static site (e.g. METRO_DASH_BASE=http://127.0.0.1:8931 uv run tests/e2e.py
# after `python3 -m http.server -d site 8931`) or the self-hosted server on :3000.
BASE = os.environ.get("METRO_DASH_BASE", "http://127.0.0.1:3000")


def check(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def state(page) -> dict:
    raw = page.evaluate("() => sessionStorage.getItem('metroDashState')")
    return json.loads(raw) if raw else {}


with sync_playwright() as playwright:
    browser = playwright.chromium.launch(headless=True)
    page = browser.new_page(viewport={"width": 1280, "height": 900})
    errors: list[str] = []
    api_calls: list[str] = []

    def on_console(message):
        # The self-hosted server has no /wasm/ assets; the client's optional
        # WASM probe 404s there and falls back to /api/chart. Not an error.
        location = message.location or {}
        if message.type == "error" and "/wasm/" not in (location.get("url") or ""):
            errors.append(message.text)

    page.on("console", on_console)
    page.on("pageerror", lambda error: errors.append(str(error)))
    page.on("request", lambda request: api_calls.append(request.url) if "/api/" in request.url else None)

    page.goto(f"{BASE}/", wait_until="networkidle")
    check(page.get_by_role("heading", name="NammaMetro in Charts").is_visible(), "site heading is not visible")
    check(page.get_by_role("heading", name="Today's Ridership").is_visible(), "default data-card chart is not visible")
    check(page.locator(".arrow-button.prev").get_attribute("aria-disabled") == "true", "previous boundary is focusable")
    check(page.locator(".arrow-button.next").get_attribute("aria-disabled") == "false", "next chart is not focusable")
    check(page.locator("#chart-data").count() == 1, "embedded Rust chart payload is missing")
    check(page.locator(".chart-shell.d3-enhanced").count() == 1, "D3 enhancement did not initialise")
    check(page.locator(".d3-datacard-scene").count() == 1, "D3 data-card scene is missing")
    check(state(page).get("chart") == "data-card", "initial session state is not the data card")
    check("?" not in page.url, "URL gained query params")

    # Keyboard navigation to the calendar chart. Opening it applies the
    # last-6-months preset through fetchRange (WASM on the static site,
    # /api/chart on the self-hosted server). activeElement is <body> here,
    # so the document-level keydown handler accepts arrow keys.
    page.keyboard.press("ArrowRight")
    page.get_by_role("heading", name="Daily Total Ridership").wait_for(state="visible", timeout=10000)
    page.locator(".d3-calendar-scene").wait_for(state="visible", timeout=10000)
    check(page.locator(".d3-datacard-scene").get_attribute("hidden") is not None, "data-card scene is not hidden")
    page.wait_for_function("() => document.querySelectorAll('.d3-cell').length > 150", timeout=10000)
    check(page.locator(".d3-cell.missing").count() > 0, "missing calendar cells are absent")
    calendar_state = state(page)
    check(calendar_state.get("chart") == "calendar", "session state did not follow navigation")
    check(re.fullmatch(r"\d{4}-\d{2}-\d{2}", calendar_state.get("start", "")) is not None, "calendar range start is not a date")
    check(re.fullmatch(r"\d{4}-\d{2}-\d{2}", calendar_state.get("end", "")) is not None, "calendar range end is not a date")

    page.locator(".d3-cell").first.focus()
    check(page.locator("#chart-tooltip").is_visible(), "calendar focus tooltip is not visible")
    check(re.search(r"\d{4}", page.locator("#chart-tooltip").text_content() or "") is not None, "calendar tooltip has no date")

    page.evaluate("() => document.activeElement.blur()")
    page.keyboard.press("ArrowRight")
    page.get_by_role("heading", name="Commute vs Casual by Weekday").wait_for(state="visible", timeout=10000)
    page.locator(".d3-line-scene").wait_for(state="visible", timeout=10000)
    page.wait_for_function("() => document.querySelectorAll('.d3-point-group').length === 14", timeout=10000)
    check(page.locator(".arrow-button.next").get_attribute("aria-disabled") == "true", "next boundary is focusable on last chart")

    page.locator(".d3-point-group").first.focus()
    check(page.locator("#chart-tooltip").is_visible(), "point focus tooltip is not visible")
    check("mean" in page.locator("#chart-tooltip").text_content(), "point tooltip lacks statistics")

    # Range change through the date picker recomputes the payload. The target
    # end date comes from the loaded dataset's own available dates, so the
    # expectation holds even when the latest published day falls early in
    # its month (a hard-coded day would be clamped away by choose_range).
    end_picker = page.locator('[data-date-type="end"]')
    available_dates = page.evaluate(
        """() => {
          const compute = window.__metroCompute;
          if (compute) {
            try { return JSON.parse(compute.datasetInfo()).availableDates || []; } catch (e) {}
          }
          const node = document.querySelector('#chart-data');
          return node ? (JSON.parse(node.textContent).dataset?.availableDates || []) : [];
        }"""
    )
    check(len(available_dates) > 1, "dataset exposes fewer than two available dates")
    current_end = state(page).get("end")
    alternatives = [d for d in reversed(available_dates) if d != current_end]
    check(alternatives, "no alternative end date available in the dataset")
    target_end = alternatives[0]
    target_year, target_month, target_day = target_end.split("-")
    end_picker.evaluate(
        """(picker, [y, m, d]) => {
          picker.querySelector('.date-year').value = y;
          picker.querySelector('.date-month').value = String(Number(m));
          picker.querySelector('.date-day').value = String(Number(d));
          picker.dispatchEvent(new Event('change', { bubbles: true }));
        }""",
        [target_year, target_month, target_day],
    )
    page.wait_for_function(
        f"() => {{ const s = JSON.parse(sessionStorage.getItem('metroDashState') || '{{}}'); return s.end === '{target_end}'; }}",
        timeout=10000,
    )
    check(state(page).get("chart") == "commute-casual", "range change reset the active chart")

    # On the static site the WASM compute core answers every range change
    # locally; /api/* must stay untouched. The self-hosted server has no
    # /wasm/ assets, so there the client legitimately falls back to /api/chart.
    has_wasm = page.evaluate("() => !!window.__metroCompute")
    if has_wasm:
        check(not api_calls, f"static site called server APIs: {api_calls}")

    check(not errors, f"browser console errors: {'; '.join(errors)}")
    desktop = Path("/tmp/metro-final-desktop.png")
    page.screenshot(path=str(desktop), full_page=True)

    mobile = browser.new_page(viewport={"width": 390, "height": 844}, is_mobile=True, has_touch=True)
    mobile_errors: list[str] = []

    def on_mobile_console(message):
        location = message.location or {}
        if message.type == "error" and "/wasm/" not in (location.get("url") or ""):
            mobile_errors.append(message.text)

    mobile.on("console", on_mobile_console)
    mobile.on("pageerror", lambda error: mobile_errors.append(str(error)))
    mobile.goto(f"{BASE}/", wait_until="networkidle")
    check(mobile.locator("body").evaluate("el => el.scrollWidth") <= 390, "page itself overflows mobile viewport")
    mobile.locator(".arrow-button.next").tap()
    mobile.locator(".d3-calendar-scene").wait_for(state="visible", timeout=10000)
    mobile.wait_for_function("() => document.querySelectorAll('.d3-cell').length > 100", timeout=10000)
    check(
        mobile.locator(".calendar-scroll-wrap").evaluate("el => el.scrollWidth > el.clientWidth"),
        "calendar lacks contained mobile scrolling",
    )
    check(not mobile_errors, f"mobile console errors: {'; '.join(mobile_errors)}")
    mobile_path = Path("/tmp/metro-final-mobile.png")
    mobile.screenshot(path=str(mobile_path), full_page=True)

    browser.close()
    print({"ok": True, "errors": errors, "wasm": has_wasm, "desktop": str(desktop), "mobile": str(mobile_path)})
