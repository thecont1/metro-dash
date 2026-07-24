pub const STYLE: &str = r#"<style>
:root{
  --paper:#fff;
  --ink:#0a0a0a;
  --ink-muted:#2a2a2a;
  --surface:#f5f5f5;
  --hairline:#e5e5e5;
  --muted:#555;
  --muted-2:#999;
  --accent:#5E2D8C;
  --accent-color:#5E2D8C;
  --qr:#3a8fb7;
  --gold:#c8a44d;
  --band-0:#f0eaf3;
  --band-1:#e6d9ed;
  --band-2:#dac6e5;
  --band-3:#ccb0dc;
  --band-4:#bb95d0;
  --band-5:#a979c1;
  --band-6:#955fb4;
  --band-7:#874baa;
  --band-8:#793da2;
  --band-9:#642a96;
  --missing:#bf705d;
  --shell-max:1280px;
  --font-display:"Fraunces","Iowan Old Style","Apple Garamond","Baskerville",Georgia,serif;
  --font-body:"Inter",system-ui,-apple-system,Segoe UI,sans-serif;
  --font-header:"DM Sans",system-ui,sans-serif;
  --font-mono:"JetBrains Mono","SF Mono",Menlo,Consolas,monospace
}
*,*:before,*:after{box-sizing:border-box}
*{margin:0;padding:0}
html,body{height:100%}
html{background:var(--paper);color:var(--ink);font-family:var(--font-body);font-size:15px;line-height:1.55;-webkit-font-smoothing:antialiased;-moz-osx-font-smoothing:grayscale;text-rendering:optimizeLegibility}
body{min-height:100vh;background:var(--paper)}
h1,h2,h3,h4,h5,h6{font-family:var(--font-display);font-weight:600;line-height:1.1;letter-spacing:-.02em;color:var(--ink)}
a{color:inherit;text-decoration:underline;text-decoration-color:var(--muted-2);text-underline-offset:3px;transition:text-decoration-color .2s}
a:hover{text-decoration-color:var(--ink)}
button,input,select,textarea{font:inherit;color:inherit}
code,.mono{font-family:var(--font-mono);font-size:.92em;background:#0000000d;padding:.1em .35em;border-radius:3px}

/* Eyebrow chip + section labels */
.eyebrow{font-family:var(--font-body);font-size:.7rem;font-weight:700;letter-spacing:.22em;text-transform:uppercase;color:var(--accent);margin:0 0 .5rem}

/* ---------- Site shell (single-viewport, capped-width) ----------
   The shell is capped at 1280px and centered. On any larger monitor
   (4K, ultrawide) the content reads at the same width as a
   comfortably-sized printed page, with empty gutter on either side.
   On a small laptop the shell uses 100% of the viewport width and
   scales the calendar/line chart down inside it. */
.site-shell{
  display:grid;
  grid-template-rows:auto 1fr auto;
  gap:14px;
  width:100%;
  max-width:var(--shell-max);
  height:100vh;
  min-height:0;
  margin:0 auto;
  padding:18px clamp(16px,3vw,32px) 20px;
  /* When the viewport is tall enough, hard-cap the shell so it
     cannot push a viewport scrollbar. When short, allow the shell
     to grow past 100vh so users can keep reading. */
}
.site-header{display:grid;grid-template-columns:1fr auto;align-items:flex-end;gap:24px;padding-bottom:10px;border-bottom:2px solid var(--ink)}
.site-header h1{font-family:var(--font-body);font-size:clamp(1.6rem,3.3vw,2.8rem);font-weight:900;letter-spacing:-.035em;line-height:.95;margin:0;max-width:18ch}
.site-header .subtitle{font-family:var(--font-display);font-style:italic;font-weight:300;font-size:clamp(.95rem,1.4vw,1.15rem);line-height:1.35;color:#000000a6;max-width:55ch;margin:.35rem 0 0}
.source-link{align-self:flex-start;font-family:var(--font-header);font-weight:700;font-size:.72rem;letter-spacing:.1em;text-transform:uppercase;color:var(--ink);text-decoration:none;border:1px solid var(--ink);padding:.45rem .7rem;background:transparent}
.source-link:hover{background:var(--ink);color:var(--paper)}

/* ---------- Prepare band (inside chart-stage) ---------- */
/* The prepare band renders as a control row: From-input + slider
   + To-input on one line, with centered action links underneath.
   It lives inside .chart-stage so the title row position is
   identical across all chart views. */
.prepare-band{display:flex;flex-direction:column;gap:14px;padding:18px 0 20px;border-bottom:1px solid var(--hairline);margin-bottom:8px}
.range-controls{display:grid;grid-template-columns:auto 1fr auto;align-items:center;gap:12px;width:100%}
.range-input{display:grid;grid-template-columns:auto 1fr;align-items:center;gap:6px;color:var(--muted);font-size:.62rem;font-weight:800;letter-spacing:.1em;text-transform:uppercase;white-space:nowrap}
.range-input-label{color:inherit}
.date-picker{display:flex;align-items:stretch;border:1px solid var(--hairline);background:var(--surface);min-height:32px;overflow:hidden}
.date-select{border:0;background:transparent;color:var(--ink);font:600 .85rem var(--font-mono);padding:4px 8px;cursor:pointer;-webkit-appearance:none;appearance:none;text-align:center}
.date-select + .date-select{border-left:1px solid var(--hairline)}
.date-select:focus{outline:2px solid var(--accent);outline-offset:-1px}
.date-select:hover{background:var(--surface)}
.date-day{min-width:30px}
.date-month{min-width:42px}
.date-year{min-width:52px}
.range-slider{position:relative;height:30px}
.slider-track{position:absolute;top:14px;left:0;right:0;height:3px;background:var(--hairline)}
.range-slider input{position:absolute;top:0;left:0;width:100%;height:30px;margin:0;background:none;pointer-events:none;-webkit-appearance:none;appearance:none}
.range-slider input::-webkit-slider-thumb{appearance:none;pointer-events:auto;width:18px;height:18px;border:2px solid var(--paper);border-radius:50%;background:var(--accent);box-shadow:0 0 0 1px var(--accent);cursor:grab;margin-top:6px}
.range-slider input::-moz-range-thumb{pointer-events:auto;width:14px;height:14px;border:2px solid var(--paper);border-radius:50%;background:var(--accent);box-shadow:0 0 0 1px var(--accent);cursor:grab}
.range-actions{display:flex;gap:14px;align-items:center;justify-content:center;padding-top:4px}
.text-button{font-family:var(--font-body);font-size:.74rem;font-weight:700;letter-spacing:.02em;color:var(--accent);text-decoration:underline;text-decoration-color:var(--accent);text-underline-offset:3px}
.text-button[aria-disabled="true"]{color:var(--muted-2);text-decoration-color:var(--muted-2);cursor:not-allowed;pointer-events:none}

/* ---------- Chart stage ---------- */
.chart-stage{display:flex;flex-direction:column;min-height:0;overflow:hidden}
.stage-topline{display:flex;justify-content:space-between;align-items:baseline;gap:20px;margin-bottom:4px}
.eyebrow .chart-index,.eyebrow .chart-total{color:var(--ink)}
.chart-title-row{display:flex;justify-content:space-between;align-items:center;gap:18px;margin:2px 0 0}
.chart-stage h2{font-family:var(--font-body);font-size:clamp(1.3rem,2.4vw,1.85rem);font-weight:800;letter-spacing:-.02em;color:var(--ink);margin:0}
.chart-deck{font-family:var(--font-display);font-style:italic;font-size:.95rem;color:var(--ink-muted);max-width:62ch;margin:0 0 8px}
.chart-shell{flex:1 1 auto;min-height:0;min-width:0;display:flex;flex-direction:column}
.chart-shell.d3-enhanced .calendar-wrap,.chart-shell.d3-enhanced .line-chart-wrap{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap}
.chart-shell.has-render{position:relative}
.chart-pager{display:flex;gap:16px}
.chart-pager .arrow-button{display:inline-flex;align-items:center;gap:4px;width:max-content;padding:6px 0;border:0;background:transparent;color:var(--ink);font-family:var(--font-body);font-weight:800;font-size:.85rem;letter-spacing:.01em;text-decoration:none}
.chart-pager .arrow-button[aria-disabled="false"]:hover{color:var(--accent)}
.chart-pager .arrow-button[aria-disabled="true"]{color:var(--muted-2);cursor:not-allowed;pointer-events:none}
.chart-pager .arrow-glyph{font-size:1.2rem;font-weight:300;line-height:.5}
.chart-pager .arrow-glyph-label{font-size:.7rem;letter-spacing:.06em;text-transform:uppercase;line-height:1}

/* Calendar scroll container — fixed 7-row height, hides scrollbar,
   uses a mask to fade content at the left/right edges suggesting more
   months are available by scrolling. */
.calendar-scroll-wrap{overflow-x:auto;overflow-y:hidden;max-width:100%;scrollbar-width:none;-ms-overflow-style:none;-webkit-mask-image:linear-gradient(to right,transparent 24px,#000 48px,#000 calc(100% - 48px),transparent calc(100% - 24px));mask-image:linear-gradient(to right,transparent 24px,#000 48px,#000 calc(100% - 48px),transparent calc(100% - 24px))}
.calendar-scroll-wrap::-webkit-scrollbar{display:none}
.calendar-scroll-wrap .d3-calendar-svg{display:block;height:190px;width:auto;max-width:none}

/* SSR fallback for the calendar — must match the D3 viewbox metrics so
   the .d3-enhanced clip is the only switch between SSR and D3 scenes.
   Content sizes itself to fit inside the parent so a wide calendar
   shrinks on narrow viewports instead of forcing a horizontal scrollbar. */
.calendar-wrap,.line-chart-wrap{flex:1 1 auto;min-height:0}
.d3-scene{flex:1 1 auto;min-height:0;min-width:0}
.d3-scene[hidden]{display:none}
.d3-calendar-svg,.d3-line-svg{display:block;width:100%;height:100%;max-width:100%;max-height:100%}

/* D3 weekday / month labels — thin monospace per spec */
.weekday-label{font:300 10px/1 SFMono,Monaco,Menlo,monospace;letter-spacing:.04em;fill:var(--muted)}
.d3-month-label{font:800 10px/1 SFMono,Monaco,Menlo,monospace;letter-spacing:.01em;fill:var(--ink);text-anchor:start}

/* SSR calendar layout.
   cell/gap chosen so the full ~78-week view fits inside ~1200px
   (cell=10, gap=1, plus 42px weekday labels + 16px padding ≈ 930px).
   On narrower shells the cells stay square because the parent is
   the constraint, not the children. */
.calendar-grid{display:grid;grid-template-columns:42px 1fr;width:100%;padding:6px 0 0}
.weekday-labels{display:grid;grid-template-rows:repeat(7,10px);gap:1px;padding-top:1px;color:var(--muted);font-size:.6rem;font-weight:700;letter-spacing:.06em;text-transform:uppercase;font-family:var(--font-body)}
.weekday-labels span{align-self:center}
.calendar-canvas{display:grid;grid-template-columns:repeat(auto-fill,minmax(0,1fr));grid-auto-flow:column;grid-auto-columns:minmax(0,1fr);gap:1px;position:relative;padding-top:18px;width:100%}
.calendar-week{display:grid;grid-template-rows:repeat(7,10px);gap:1px}
.calendar-cell{width:100%;aspect-ratio:1;min-width:0;padding:0;border:0;background:var(--hairline);position:relative;cursor:pointer}
.calendar-cell:hover,.calendar-cell:focus-visible{outline:2px solid var(--ink);outline-offset:1px;z-index:2}
.calendar-cell.structural{background:transparent;cursor:default}
.calendar-cell.missing{background:var(--paper);border:1px solid var(--missing);background-image:linear-gradient(45deg,transparent 44%,var(--missing) 45%,var(--missing) 55%,transparent 56%),linear-gradient(-45deg,transparent 44%,var(--missing) 45%,var(--missing) 55%,transparent 56%);background-size:100% 100%}
.calendar-cell.neutral{background:var(--hairline)}
.band-0{background:var(--band-0)}.band-1{background:var(--band-1)}.band-2{background:var(--band-2)}.band-3{background:var(--band-3)}.band-4{background:var(--band-4)}.band-5{background:var(--band-5)}.band-6{background:var(--band-6)}.band-7{background:var(--band-7)}.band-8{background:var(--band-8)}.band-9{background:var(--band-9)}
.month-label{position:absolute;top:0;left:0;color:var(--muted);font-size:.62rem;font-weight:800;letter-spacing:.04em;text-transform:uppercase;white-space:nowrap;font-family:var(--font-body);pointer-events:none}
.legend{display:flex;flex-direction:column;gap:6px;margin:10px 0 0;color:var(--muted);font-size:.7rem;line-height:1.45;letter-spacing:.02em;font-family:var(--font-body)}
.legend-caption{margin:0;font-weight:700;color:var(--ink);font-variant-numeric:tabular-nums;letter-spacing:.005em}
.legend-meta{margin:0;color:var(--muted)}
.legend-missing-note{color:var(--missing)}
.legend-gradient{display:block;width:100%;height:14px;border:1px solid var(--hairline);border-radius:2px;background:linear-gradient(to right,var(--band-0),var(--band-3),var(--band-6),var(--band-9))}

/* SSR line chart */
.line-chart-wrap{width:100%}
.line-chart{display:block;width:100%;max-width:100%;height:auto}
.grid line{stroke:var(--hairline);stroke-width:1}
.grid text,.x-label{fill:var(--muted);font:11px var(--font-body)}
.series{stroke-width:3;fill:none}
.series.commute,.point.commute,.whisker.commute{stroke:var(--accent);fill:var(--accent)}
.series.casual,.point.casual,.whisker.casual{stroke:var(--gold);fill:var(--gold)}
.point{stroke:var(--paper);stroke-width:3;cursor:pointer}
.point:focus{outline:none;stroke:var(--ink);stroke-width:4}
.whisker{stroke-width:2;opacity:.45}
.point-label{font:700 11px var(--font-body)}
.point-label.commute,.series-label.commute{fill:var(--accent)}
.point-label.casual,.series-label.casual{fill:var(--gold)}
.series-label{font:700 11px var(--font-body)}
.insight{margin:6px 0 8px;color:var(--ink);font:600 1rem var(--font-display);letter-spacing:-.01em;max-width:60ch}

/* SSR fallback tables (the accessible-data details block) */
.accessible-data{margin-top:6px;font-size:.7rem;color:var(--muted)}
.accessible-data summary{color:var(--accent);font-weight:800;cursor:pointer;letter-spacing:.04em}
table{width:100%;border-collapse:collapse;font-size:.7rem}
th,td{padding:4px 6px;border-bottom:1px solid var(--hairline);text-align:left;white-space:nowrap}
th{color:var(--muted);font-size:.62rem;letter-spacing:.06em;text-transform:uppercase}

/* Footer */
.site-footer{padding:8px 0 0;border-top:1px solid var(--hairline);color:var(--muted);font-size:.66rem;line-height:1.4;display:flex;justify-content:space-between;align-items:flex-end;gap:18px}
.site-footer p{margin-bottom:3px}
.site-footer .chart-method{color:var(--muted);font-size:.7rem;line-height:1.45;border-left:2px solid var(--accent);padding:2px 0 2px 9px;max-width:62ch}
.site-footer .copyright{font-family:var(--font-display);font-style:italic;font-size:.8rem;font-weight:400;color:var(--ink-muted);letter-spacing:.01em;margin-top:6px}

/* Tooltip (D3 hover) */
.chart-tooltip{position:fixed;z-index:20;max-width:280px;transform:translate(-50%,-100%);padding:7px 9px;background:var(--ink);color:var(--paper);font-size:.7rem;line-height:1.35;pointer-events:none;box-shadow:0 6px 20px rgba(10,10,10,.25);font-family:var(--font-body)}
.chart-tooltip[hidden]{display:none}

/* D3 progressive-enhancement layers (calendar + line scenes) */
.d3-cell rect{stroke:transparent}
.d3-cell:focus{outline:none}
.d3-cell:focus rect,.d3-cell:hover rect{stroke:var(--ink);stroke-width:2;stroke-opacity:.5}
.missing-cross{stroke:var(--missing);stroke-width:1.4}
.d3-grid-line{stroke:var(--hairline);stroke-width:1}
.d3-series-path{stroke-width:3}
.d3-series-path.commute,.d3-point.commute,.d3-whisker.commute{stroke:var(--accent);fill:var(--accent)}
.d3-series-path.casual,.d3-point.casual,.d3-whisker.casual{stroke:var(--gold);fill:var(--gold)}
.d3-point{stroke:var(--paper);stroke-width:3;cursor:pointer}
.d3-point-group:focus .d3-point{stroke:var(--ink);stroke-width:3}
.d3-whisker{stroke-width:2;opacity:.45}
.d3-point-label{font:700 11px var(--font-body)}
.d3-point-label.commute,.d3-series-label.commute{fill:var(--accent)}
.d3-point-label.casual,.d3-series-label.casual{fill:var(--gold)}
.d3-series-label{font:700 11px var(--font-body)}
.d3-month-label,.d3-y-label,.d3-x-label{fill:var(--muted);font:700 11px var(--font-body)}
.d3-month-label{font-size:10px}
.weekday-label{fill:var(--muted);font:700 10px var(--font-body);letter-spacing:.05em;text-transform:uppercase}

/* Fallback: when the viewport is too short to fit everything, the
   whole shell becomes scrollable. This is the ONLY scroll the page
   ever uses, and it kicks in below the budget breakpoint (~820px tall
   for the 1280-wide shell, or below 880px wide for narrower devices). */
@media (max-height:820px),(max-width:880px){
  html,body{height:auto;overflow:auto}
  .site-shell{height:auto;min-height:100vh}
}

/* Reduced motion */
@media (prefers-reduced-motion:reduce){*,*:before,*:after{animation-duration:.01ms!important;transition-duration:.01ms!important}}

/* Skip link */
.sr-only{position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0}

/* Data card */
body.chart-data-card .prepare-band{display:none}
body.chart-data-card .chart-stage{justify-content:center}
body.chart-data-card .chart-shell{justify-content:center}
body.chart-data-card .d3-datacard-scene{position:absolute;pointer-events:none;width:0;height:0;overflow:visible}
.data-card-wrap{max-width:1100px;margin:0 auto;width:100%}
.data-card-inner{background:var(--paper);border:1px solid var(--hairline);border-radius:12px;padding:28px 32px;display:flex;flex-direction:column;gap:24px}
.dc-top-story{display:flex;flex-direction:column;gap:20px}
.dc-bottom-story{display:flex;flex-direction:column;gap:12px}
.dc-date-row{display:flex;justify-content:center}
.dc-date-label{font-size:12px;font-weight:600;color:var(--muted);text-transform:uppercase;letter-spacing:1px}
.dc-date-value{font-size:18px;font-weight:600;color:var(--ink);font-family:var(--mono,ui-monospace,SFMono-Regular,Menlo,monospace)}
.dc-total-container{text-align:center}
.dc-total-value{font-size:56px;font-weight:700;color:var(--ink);letter-spacing:-1px;line-height:1.1}
.dc-total-label{font-size:13px;font-weight:600;color:var(--muted);text-transform:uppercase;letter-spacing:1px;margin-top:4px}
.dc-bar-chart{height:28px;background:#edf2f7;border-radius:14px;overflow:hidden;display:flex}
.dc-bar-segment{height:100%;transition:width .4s ease}
.dc-bar-smart-card{background:var(--accent)}
.dc-bar-token{background:#2ecc71}
.dc-bar-qr{background:#3182ce}
.dc-bar-ncmc{background:#ff3333}
.dc-bar-group{background:#9b59b6}
.dc-legend{display:flex;flex-wrap:wrap;justify-content:center;gap:10px}
.dc-legend-item{display:flex;align-items:center;font-size:12px;color:var(--ink)}
.dc-legend-color{width:12px;height:12px;border-radius:3px;margin-right:6px;flex-shrink:0}
.dc-payment-grid{display:grid;grid-template-columns:repeat(5,1fr);gap:12px;align-items:stretch;grid-auto-rows:1fr}
.dc-payment-box{background:#f8fafc;border-radius:8px;padding:16px 14px;border-top:4px solid var(--accent);text-align:center}
.dc-payment-box.dc-bar-smart-card{border-top-color:var(--accent)}
.dc-payment-box.dc-bar-token{border-top-color:#2ecc71}
.dc-payment-box.dc-bar-qr{border-top-color:#3182ce}
.dc-payment-box.dc-bar-ncmc{border-top-color:#ff3333}
.dc-payment-box.dc-bar-group{border-top-color:#9b59b6}
.dc-payment-value{font-size:20px;font-weight:700;color:var(--ink);margin-bottom:2px}
.dc-payment-label{font-size:11px;text-transform:uppercase;color:var(--ink);font-weight:500}
.dc-breakdown{margin-top:8px;font-size:11px;color:var(--muted);text-align:left}
.dc-sub-row{display:flex;justify-content:space-between;margin-bottom:2px}
.dc-sub-label{color:var(--muted)}
.dc-sub-value{color:var(--muted);font-weight:500}
.dc-missing-note{margin-top:4px;font-size:13px;color:var(--muted);text-align:center}
.data-card-missing{text-align:center;padding:48px 24px;color:var(--muted)}
.data-card-missing-title{font-size:18px;font-weight:600;margin-bottom:8px}
.data-card-missing-hint{font-size:14px}
/* Narrow screens: redistribute payment grid columns */
@media (max-width:960px){
  .dc-payment-grid{grid-template-columns:repeat(3,1fr)}
}
@media (max-width:760px){
  .dc-payment-grid{grid-template-columns:repeat(2,1fr)}
  .dc-total-value{font-size:44px}
  .data-card-inner{padding:24px 20px}
}
@media (max-width:480px){
  .dc-payment-grid{grid-template-columns:1fr}
  .dc-total-value{font-size:36px}
}
</style>"#;
