# NammaMetro in Charts

**Live site:** <https://namma.thecontrarian.in> (static site on Cloudflare Pages)

A simple, public view of Bengaluru Metro ridership. It turns the daily BMRCL ridership CSV into nine interactive charts, with no sign-up or downloads needed.

## What it shows

1. **Today's Ridership** — the fare media breakdown for the latest published day (the end of your selected date range). The coloured proportion bar and payment boxes mirror the official BMRCL daily ridership page. *Method note:* this is a single-day snapshot, so the end date is the anchor; missing fare-media components are not treated as zero.
2. **Daily Total Ridership** — a calendar heatmap. Each cell is one day; the deeper the purple, the higher the total. Crossed-out cells are days BMRCL did not publish a total. *Method note:* colours use percentile bands within the selected range rather than a fixed ridership scale, making local variation visible while keeping the scale explicitly relative.
3. **Commute vs Casual by Weekday** — average journeys by day of week. *Commute* riders use Smart Card or NCMC (closed-loop fare media); *Casual* riders use Token, QR, or Group tickets. *Method note:* the chart uses arithmetic means with sample-standard-deviation whiskers, and a day contributes to a group only when every component of that group is present.
4. **Unusual for its Weekday?** — each derived daily total compared with a rolling same-weekday median and interquartile band. Mondays are compared with nearby Mondays, Tuesdays with nearby Tuesdays, and so on. *Method note:* the centred baseline uses up to 6 earlier and 6 later published reports from the same weekday, excludes the current day, requires at least 4 nearby reports, and omits missing reports rather than treating them as zero.
5. **What Accounts for the Difference?** — a waterfall showing how Smart Cards, NCMC, Tokens, QR, and Group Ticket contribute to the selected day's difference from its same-weekday reference. *Method note:* each category reference is the median of the same set of nearby same-weekday reports with complete published parent components. The chart intentionally uses parent-category totals rather than leaf/provider sums because QR provider splits do not always reconcile with their parent; category deltas add exactly to the total difference, and this is arithmetic decomposition, not causal attribution.
6. **How is the Fare-Media Mix Shifting?** — monthly 100% share columns for the five fare-media categories, with a separate lower strip of observed absolute totals. *Method note:* a month uses only published days with complete parent components. The lower strip is the observed total for those complete days, not a full-month total; months without usable observations remain visible as gaps, and complete-day counts are printed beneath the observed-total bars. Shares describe aggregate fare-media composition, not individual payment switching.
7. **How is NCMC’s Share Changing?** — weekly NCMC share of Smart Cards + NCMC, with NCMC counts in a separate lower strip. *Method note:* a day contributes only when both Smart Card and NCMC are present, so the count and share use the same evidence base. Complete/published day counts make uneven week coverage visible. This is fare-media arithmetic, not proof that Smart Card users converted to NCMC.
8. **Which QR Channels Make Up QR Use?** — monthly Namma Metro, WhatsApp, Paytm, and ONDC shares with parent Total QR shown separately. *Method note:* shares use the provider sum because provider splits do not always reconcile with parent `Total QR`; only days with all provider fields present contribute. Zeros are reported zeros, missing fields exclude the day, and the mix does not identify individual app use.
9. **What is Inside Smart Cards?** — monthly Stored Value, One Day Pass, Three Day Pass, and Five Day Pass shares with parent Smart Cards shown separately. *Method note:* shares use the subtype sum and only days with all four subtype fields present contribute. Missing fields exclude the day rather than becoming zero; subtype counts are fare-media counts, not unique people or trip purposes.

## How to use it

- Move between charts with the **Previous / Next** buttons, the chart jump menu, the left/right arrow keys, or a swipe.
- Change the date range by dragging the **From / To** sliders or by typing/selecting dates.
- Use the one-click filters (**Last 3 months**, **Last 6 months**, etc.) to jump to common ranges. The calendar defaults to **Last 6 months** when you open it.
- Refreshing the page resets the app to the homepage and the latest datapoint.

## About the data

- **Source:** the canonical `NammaMetro_Ridership_Dataset.csv` published by BMRCL, pulled from GitHub.
- **Refresh:** the browser fetches the latest CSV from GitHub on every visit — no server involved — but only once JavaScript has loaded and initialised the WASM compute module; without it, visitors see the build-time snapshot.
- **Missing days:** BMRCL does not publish a total every day. Those days appear as crossed cells on the calendar, are excluded from weekday averages, and are not treated as zero in the unusual-day baseline.
- **Percentiles:** each calendar day's colour is based on its percentile within your selected range, so the palette is relative to the dates you have chosen.

## Notes

- The URL stays as `/`. The selected chart and range are kept only for the current pageview and reset on refresh, so specific views are not shareable by link.
- Without JavaScript you get the landing data card only — it is rendered into the static HTML at build time. The other charts, chart navigation, and date-range changes all require JavaScript; D3/GSAP enhance the experience when available.
- It respects `prefers-reduced-motion` and works on mobile.

---

*For build, deployment, and developer documentation, see [TECH_SPEC.md](TECH_SPEC.md).*
