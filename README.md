# NammaMetro in Charts

**Live site:** <https://namma.thecontrarian.in> (static site on Cloudflare Pages)

A simple, public view of Bengaluru Metro ridership. It turns the daily BMRCL ridership CSV into three interactive charts, with no sign-up or downloads needed.

## What it shows

1. **Today's Ridership** — the fare media breakdown for the latest published day (the end of your selected date range). The coloured proportion bar and payment boxes mirror the official BMRCL daily ridership page.
2. **Daily Total Ridership** — a calendar heatmap. Each square is one day; the deeper the purple, the higher the total. Crossed-out squares are days BMRCL did not publish a total.
3. **Commute vs Casual by Weekday** — average journeys by day of week. *Commute* riders use Smart Card or NCMC (closed-loop fare media); *Casual* riders use Token, QR, or Group tickets.

## How to use it

- Move between charts with the **Previous / Next** buttons, the left/right arrow keys, or a swipe.
- Change the date range by dragging the **From / To** sliders or by typing/selecting dates.
- Use the one-click filters (**Last 3 months**, **Last 6 months**, etc.) to jump to common ranges. The calendar defaults to **Last 6 months** when you open it.
- Refreshing the page resets the app to the homepage and the latest datapoint.

## About the data

- **Source:** the canonical `NammaMetro_Ridership_Dataset.csv` published by BMRCL, pulled from GitHub.
- **Refresh:** the browser fetches the latest CSV from GitHub on every visit — no server involved — but only once JavaScript has loaded and initialised the WASM compute module; without it, visitors see the build-time snapshot.
- **Missing days:** BMRCL does not publish a total every day. Those days appear as crossed cells on the calendar and are excluded from weekday averages.
- **Percentiles:** each calendar day's colour is based on its percentile within your selected range, so the palette is relative to the dates you have chosen.

## Notes

- The URL stays as `/`. The selected chart and range are kept only for the current pageview and reset on refresh, so specific views are not shareable by link.
- Without JavaScript you get the landing data card only — it is rendered into the static HTML at build time. The calendar and weekday charts, chart navigation, and date-range changes all require JavaScript; D3/GSAP enhance the experience when available.
- It respects `prefers-reduced-motion` and works on mobile.

---

*For build, deployment, and developer documentation, see [TECH_SPEC.md](TECH_SPEC.md).*
