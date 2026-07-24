# /// script
# requires-python = ">=3.11"
# dependencies = ["playwright>=1.54,<2"]
# ///

from pathlib import Path
from urllib.parse import parse_qs, urlparse

from playwright.sync_api import sync_playwright

BASE = "http://127.0.0.1:3000"


def check(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


with sync_playwright() as playwright:
    browser = playwright.chromium.launch(headless=True)
    page = browser.new_page(viewport={"width": 1280, "height": 900})
    errors: list[str] = []
    page.on("console", lambda message: errors.append(message.text) if message.type == "error" else None)
    page.on("pageerror", lambda error: errors.append(str(error)))

    page.goto(f"{BASE}/", wait_until="networkidle")
    check(page.get_by_role("heading", name="Daily ridership calendar").is_visible(), "calendar is not visible")
    check(page.locator('.arrow-button[aria-label^="Previous"]').get_attribute("tabindex") == "-1", "previous boundary is focusable")
    check(page.locator('.arrow-button[aria-label^="Next"]').get_attribute("tabindex") == "0", "next chart is not focusable")
    check(page.locator(".calendar-cell.missing").count() > 0, "missing cells are absent")
    check(page.locator("#chart-data").count() == 1, "embedded Rust chart payload is missing")
    check(page.locator(".chart-shell.d3-enhanced").count() == 1, "D3 enhancement did not initialise")
    check(page.locator(".d3-calendar-scene").count() == 1, "D3 calendar scene is missing")

    page.locator(".d3-cell").first.focus()
    check(page.locator("#chart-tooltip").is_visible(), "calendar focus tooltip is not visible")
    check("2026" in page.locator("#chart-tooltip").text_content(), "calendar tooltip has no date")

    page.evaluate("() => document.activeElement.blur()")
    page.keyboard.press("ArrowRight")
    page.wait_for_url("**chart=commute-casual")
    check(page.get_by_role("heading", name="Average daily ridership: Commute vs casual").is_visible(), "line chart navigation failed")
    check(page.locator(".d3-line-scene").count() == 1, "D3 line scene is missing")
    check(page.locator(".d3-point").count() == 14, "line chart does not contain 14 valid points")
    check(page.locator('.arrow-button[aria-label^="Next"]').get_attribute("tabindex") == "-1", "next boundary is focusable")

    page.locator(".d3-point-group").first.focus()
    check(page.locator("#chart-tooltip").is_visible(), "point focus tooltip is not visible")
    check("mean" in page.locator("#chart-tooltip").text_content(), "point tooltip lacks statistics")

    page.goto(f"{BASE}/?start=2026-03-01&end=2026-03-31&chart=calendar", wait_until="networkidle")
    start_picker = page.locator('[data-date-type="start"]')
    end_picker = page.locator('[data-date-type="end"]')
    check(start_picker.locator(".date-day").input_value() == "1" and start_picker.locator(".date-month").input_value() == "3" and start_picker.locator(".date-year").input_value() == "2026", "custom start range is not restored")
    check(end_picker.locator(".date-day").input_value() == "31" and end_picker.locator(".date-month").input_value() == "3" and end_picker.locator(".date-year").input_value() == "2026", "custom end range is not restored")
    check(page.locator(".calendar-cell:not(.structural)").count() == 31, "custom calendar does not include every calendar day")
    check(page.locator(".d3-cell").count() == 31, "D3 calendar does not include every calendar day")

    start_picker.locator(".date-day").select_option("8")
    start_picker.dispatch_event("change")
    page.wait_for_url("**start=2026-03-08**")
    check(parse_qs(urlparse(page.url).query).get("start") == ["2026-03-08"], "text date did not update URL")

    check(not errors, f"browser console errors: {'; '.join(errors)}")
    desktop = Path("/tmp/metro-final-desktop.png")
    page.screenshot(path=str(desktop), full_page=True)

    mobile = browser.new_page(viewport={"width": 390, "height": 844}, is_mobile=True, has_touch=True)
    mobile.goto(f"{BASE}/", wait_until="networkidle")
    check(mobile.locator("body").evaluate("el => el.scrollWidth") <= 390, "page itself overflows mobile viewport")
    check(mobile.locator(".d3-scene:not([hidden])").evaluate("el => el.scrollWidth > el.clientWidth"), "calendar lacks contained mobile scrolling")
    mobile_path = Path("/tmp/metro-final-mobile.png")
    mobile.screenshot(path=str(mobile_path), full_page=True)

    browser.close()
    print({"ok": True, "errors": errors, "desktop": str(desktop), "mobile": str(mobile_path)})
