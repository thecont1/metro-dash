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
    check(page.locator(".source-links svg").count() == 1, "GitHub logo is missing from source links")
    check(
        page.get_by_role("link", name="NammaMetro in Charts source code on GitHub").get_attribute("href")
        == "https://github.com/thecont1/metro-dash",
        "source code link does not point at metro-dash",
    )
    check(
        page.get_by_role("link", name="Namma Metro ridership source data on GitHub").get_attribute("href")
        == "https://github.com/thecont1/namma-metro-ridership-tracker",
        "source data link does not point at the ridership tracker",
    )
    header_alignment = page.evaluate(
        """() => {
          const title = document.querySelector('.site-header h1').getBoundingClientRect();
          const source = document.querySelector('.source-links').getBoundingClientRect();
          return (title.top + title.height / 2) - (source.top + source.height / 2);
        }"""
    )
    check(abs(header_alignment) <= 2, f"source links are not vertically aligned with the site title: {header_alignment}px")
    check(page.get_by_role("heading", name="Today's Ridership").is_visible(), "default data-card chart is not visible")
    check(page.locator(".arrow-button.prev").get_attribute("aria-disabled") == "true", "previous boundary is focusable")
    check(page.locator(".arrow-button.next").get_attribute("aria-disabled") == "false", "next chart is not focusable")
    chart_jump = page.locator(".chart-jump")
    check(chart_jump.count() == 1, "chart jump select is missing")
    check(chart_jump.locator("option").count() == 6, "chart jump select does not list every chart")
    check(chart_jump.input_value() == "data-card", "chart jump select is not synced to the active chart")
    check(page.locator("#chart-data").count() == 1, "embedded Rust chart payload is missing")
    check(page.locator(".chart-shell.d3-enhanced").count() == 1, "D3 enhancement did not initialise")
    check(page.locator(".d3-datacard-scene").count() == 1, "D3 data-card scene is missing")
    data_card_widths = page.evaluate(
        """() => ({
          shell: document.querySelector('.chart-shell').getBoundingClientRect().width,
          card: document.querySelector('.data-card-wrap').getBoundingClientRect().width,
        })"""
    )
    check(abs(data_card_widths["shell"] - data_card_widths["card"]) <= 1.5, "data card does not fill the chart shell width")
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
    cell_dims = page.evaluate(
        """() => [...document.querySelectorAll('.d3-cell rect')].slice(0, 20).map(r => {
          const b = r.getBoundingClientRect();
          return {w: b.width, h: b.height};
        })"""
    )
    check(len(cell_dims) == 20, "calendar cells are not rendered as rects")
    check(all(abs(d["w"] - d["h"]) <= 0.5 for d in cell_dims), f"calendar cells are not square: {cell_dims[0]}")
    check(all(abs(d["w"] - 22) <= 1 for d in cell_dims), f"calendar cells are not the fixed 22px size: {cell_dims[0]}")

    # Nine months must fit in one frame without scrolling — the cell size
    # was chosen so ~39 week-columns plus month gaps fit the desktop wrap.
    page.click('[data-preset="last-9-months"]')
    page.wait_for_timeout(800)
    nine_month_fit = page.evaluate(
        """() => {
          const wrap = document.querySelector('.calendar-scroll-wrap');
          return {client: wrap.clientWidth, scroll: wrap.scrollWidth};
        }"""
    )
    check(nine_month_fit["scroll"] <= nine_month_fit["client"] + 2, f"9-month range does not fit in one frame: {nine_month_fit}")

    # Longer ranges must remain internally scrollable: jumping to the end
    # and back to the beginning changes scrollLeft.
    page.click('[data-preset="use-all-data"]')
    page.wait_for_timeout(800)
    calendar_geometry = page.evaluate(
        """() => {
          const wrap = document.querySelector('.calendar-scroll-wrap');
          const svg = document.querySelector('.d3-calendar-svg').getBoundingClientRect();
          return {wrapClient: wrap.clientWidth, wrapScroll: wrap.scrollWidth, svgW: svg.width};
        }"""
    )
    check(calendar_geometry["wrapScroll"] > calendar_geometry["wrapClient"], "calendar is not internally scrollable for long ranges")
    check(abs(calendar_geometry["svgW"] - calendar_geometry["wrapScroll"]) <= 2, "calendar SVG does not size to its content")
    scroll_test = page.evaluate(
        """() => {
          const wrap = document.querySelector('.calendar-scroll-wrap');
          wrap.scrollLeft = wrap.scrollWidth;
          const end = wrap.scrollLeft;
          wrap.scrollLeft = 0;
          return {end, start: wrap.scrollLeft};
        }"""
    )
    check(scroll_test["end"] > 100, "calendar cannot scroll to the end of the range")
    check(scroll_test["start"] == 0, "calendar cannot scroll back to the beginning")
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
    page.wait_for_timeout(350)
    line_widths = page.evaluate(
        """() => ({
          shell: document.querySelector('.chart-shell').getBoundingClientRect().width,
          svg: document.querySelector('.d3-line-svg').getBoundingClientRect().width,
        })"""
    )
    check(abs(line_widths["shell"] - line_widths["svg"]) <= 1.5, "line chart does not fill the chart shell width")
    line_plot_widths = page.evaluate(
        """() => {
          const svg = document.querySelector('.d3-line-svg').getBoundingClientRect();
          const grid = [...document.querySelectorAll('.d3-grid-line')].map(n => n.getBoundingClientRect());
          const left = Math.min(...grid.map(r => r.left));
          const right = Math.max(...grid.map(r => r.right));
          return {ratio: (right - left) / svg.width, rightGap: svg.right - right};
        }"""
    )
    check(line_plot_widths["ratio"] >= 0.85, f"line plot is letterboxed inside the SVG: {line_plot_widths['ratio']}")
    check(line_plot_widths["rightGap"] <= 40, f"line plot leaves too much right whitespace: {line_plot_widths['rightGap']}px")
    page.set_viewport_size({"width": 1000, "height": 900})
    page.wait_for_timeout(350)
    resized_line_plot_widths = page.evaluate(
        """() => {
          const svg = document.querySelector('.d3-line-svg').getBoundingClientRect();
          const grid = [...document.querySelectorAll('.d3-grid-line')].map(n => n.getBoundingClientRect());
          const left = Math.min(...grid.map(r => r.left));
          const right = Math.max(...grid.map(r => r.right));
          return {ratio: (right - left) / svg.width, rightGap: svg.right - right};
        }"""
    )
    check(resized_line_plot_widths["ratio"] >= 0.85, "line plot does not fill the width after viewport resize")
    page.set_viewport_size({"width": 1280, "height": 900})
    page.wait_for_timeout(350)
    check(page.locator(".arrow-button.next").get_attribute("aria-disabled") == "false", "next chart is not focusable after the weekday chart")

    line_paint = page.evaluate(
        """() => {
          const path = document.querySelector('.d3-series-path.commute');
          const style = getComputedStyle(path);
          return {fillOpacity: style.fillOpacity, strokeOpacity: style.strokeOpacity};
        }"""
    )
    check(float(line_paint["fillOpacity"]) < 1, "commute fill is not translucent")
    check(float(line_paint["strokeOpacity"]) == 1, "commute line is not solid")
    edge_gap = page.evaluate(
        """() => {
          const xOf = g => parseFloat((g.getAttribute('transform') || 'translate(0,0)').slice(10));
          const groups = [...document.querySelectorAll('.d3-point-group')];
          const minX = Math.min(...groups.map(xOf));
          const edgeLabels = groups.filter(g => xOf(g) < minX + 1).map(g => g.querySelector('text').getBoundingClientRect());
          const yRight = Math.max(...[...document.querySelectorAll('.d3-y-label')].map(n => n.getBoundingClientRect().right));
          return Math.min(...edgeLabels.map(r => r.left - yRight));
        }"""
    )
    check(edge_gap >= 4, f"edge value labels are too close to y-axis labels: {edge_gap}px")

    page.locator(".d3-point-group").first.focus()
    check(page.locator("#chart-tooltip").is_visible(), "point focus tooltip is not visible")
    check("mean" in page.locator("#chart-tooltip").text_content(), "point tooltip lacks statistics")

    chart_jump.select_option("calendar")
    page.get_by_role("heading", name="Daily Total Ridership").wait_for(state="visible", timeout=10000)
    page.locator(".d3-calendar-scene").wait_for(state="visible", timeout=10000)
    check(chart_jump.input_value() == "calendar", "chart jump select did not follow direct navigation")
    chart_jump.select_option("commute-casual")
    page.get_by_role("heading", name="Commute vs Casual by Weekday").wait_for(state="visible", timeout=10000)
    page.locator(".d3-line-scene").wait_for(state="visible", timeout=10000)
    check(chart_jump.input_value() == "commute-casual", "chart jump select did not restore the active chart")
    insight_box = page.locator(".d3-insight").bounding_box()
    method_box = page.locator(".chart-method").bounding_box()
    check(insight_box is not None and method_box is not None, "line insight or method note is missing")
    check(insight_box["y"] + insight_box["height"] <= method_box["y"] + 1, "line insight overlaps the method note")

    page.evaluate("() => document.activeElement.blur()")
    page.keyboard.press("ArrowRight")
    page.get_by_role("heading", name="Unusual for its Weekday?").wait_for(state="visible", timeout=10000)
    page.locator(".d3-baseline-scene").wait_for(state="visible", timeout=10000)
    page.wait_for_function("() => document.querySelectorAll('.d3-baseline-point-group').length > 0", timeout=10000)
    check(page.locator(".baseline-svg-legend").count() >= 1, "baseline legend is missing")

    page.locator(".d3-baseline-point-group").first.focus()
    check(page.locator("#chart-tooltip").is_visible(), "baseline point focus tooltip is not visible")
    check("total" in page.locator("#chart-tooltip").text_content(), "baseline tooltip lacks total")

    page.evaluate("() => document.activeElement.blur()")
    page.keyboard.press("ArrowRight")
    page.get_by_role("heading", name="What Accounts for the Difference?").wait_for(state="visible", timeout=10000)
    page.locator(".d3-waterfall-scene").wait_for(state="visible", timeout=10000)
    page.wait_for_function("() => document.querySelectorAll('.d3-waterfall-group').length === 5", timeout=10000)
    check(page.locator(".waterfall-svg-legend").count() >= 1, "waterfall legend is missing")
    check(page.locator(".waterfall-total-anchor").count() == 2, "waterfall total anchors are missing")

    page.locator(".d3-waterfall-group").first.focus()
    check(page.locator("#chart-tooltip").is_visible(), "waterfall component focus tooltip is not visible")
    check("delta" in page.locator("#chart-tooltip").text_content(), "waterfall tooltip lacks delta")

    page.evaluate("() => document.activeElement.blur()")
    page.keyboard.press("ArrowRight")
    page.get_by_role("heading", name="How is the Fare-Media Mix Shifting?").wait_for(state="visible", timeout=10000)
    page.locator(".d3-mix-scene").wait_for(state="visible", timeout=10000)
    page.wait_for_function("() => document.querySelectorAll('.d3-mix-segment').length > 0", timeout=10000)
    check(page.locator(".mix-svg-legend").count() >= 1, "fare mix legend is missing")
    check(page.locator(".mix-x-label").count() >= 2, "fare mix month labels are missing")
    check(page.locator(".mix-coverage-label").count() >= 2, "fare mix coverage labels are missing")
    check(page.locator(".arrow-button.next").get_attribute("aria-disabled") == "true", "next boundary is focusable on last chart")

    page.locator(".d3-mix-segment").first.focus()
    check(page.locator("#chart-tooltip").is_visible(), "fare mix segment focus tooltip is not visible")
    check("complete" in page.locator("#chart-tooltip").text_content(), "fare mix tooltip lacks coverage")


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
    check(state(page).get("chart") == "fare-media-mix", "range change reset the active chart")

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

    compact = browser.new_page(viewport={"width": 320, "height": 760})
    compact.goto(f"{BASE}/", wait_until="networkidle")
    compact_source = compact.locator(".source-links").bounding_box()
    check(compact_source is not None, "source links are missing on compact viewport")
    check(compact_source["x"] + compact_source["width"] <= 320, "source links overflow compact viewport")
    compact.close()

    browser.close()
    print({"ok": True, "errors": errors, "wasm": has_wasm, "desktop": str(desktop), "mobile": str(mobile_path)})
