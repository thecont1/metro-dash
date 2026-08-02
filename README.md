# NammaMetro in Charts

**Live site:** <https://metrodash.fly.dev>

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
- **Refresh:** the server fetches the CSV when it starts, then checks again every six hours.
- **Missing days:** BMRCL does not publish a total every day. Those days appear as crossed cells on the calendar and are excluded from weekday averages.
- **Percentiles:** each calendar day's colour is based on its percentile within your selected range, so the palette is relative to the dates you have chosen.

## Notes

- The URL stays as `/`. The selected chart and range are kept only for the current pageview and reset on refresh, so specific views are not shareable by link.
- The app is designed to work without JavaScript: the basic charts are rendered on the server, and D3/GSAP only enhance the experience when available.
- It respects `prefers-reduced-motion` and works on mobile.

---

*For build, deployment, and developer documentation, see [TECH_SPEC.md](TECH_SPEC.md).*
