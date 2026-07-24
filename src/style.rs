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
.source-link{align-self:flex-start;font-family:var(--font-header);font-weight:700;font-size:.72rem;letter-spacing:.1em;text-transform:uppercase;color:var(--ink);text-decoration:none;border:1px solid var(--ink);padding:.6rem .8rem;min-height:24px;display:inline-flex;align-items:center;background:transparent}
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
.date-select{border:0;background:transparent;color:var(--ink);font:600 .85rem var(--font-mono);padding:6px 8px;min-height:24px;cursor:pointer;-webkit-appearance:none;appearance:none;text-align:center}
.date-select + .date-select{border-left:1px solid var(--hairline)}
.date-select:focus{outline:2px solid var(--accent);outline-offset:-1px}
.date-select:hover{background:var(--surface)}
.date-day{min-width:30px}
.date-month{min-width:42px}
.date-year{min-width:52px}
.range-slider{position:relative;height:30px}
.slider-track{position:absolute;top:14px;left:0;right:0;height:3px;background:var(--hairline)}
.range-slider input{position:absolute;top:0;left:0;width:100%;height:30px;margin:0;background:none;pointer-events:none;-webkit-appearance:none;appearance:none}
.range-slider input::-webkit-slider-thumb{appearance:none;pointer-events:auto;width:24px;height:24px;border:2px solid var(--paper);border-radius:50%;background:var(--accent);box-shadow:0 0 0 1px var(--accent);cursor:grab;margin-top:3px}
.range-slider input::-moz-range-thumb{pointer-events:auto;width:20px;height:20px;border:2px solid var(--paper);border-radius:50%;background:var(--accent);box-shadow:0 0 0 1px var(--accent);cursor:grab}
.range-actions{display:flex;gap:14px;align-items:center;justify-content:center;padding-top:4px}
.text-button{font-family:var(--font-body);font-size:.74rem;font-weight:700;letter-spacing:.02em;color:var(--accent);text-decoration:underline;text-decoration-color:var(--accent);text-underline-offset:3px;display:inline-flex;align-items:center;min-height:24px;padding:4px 2px}
.text-button[aria-disabled="true"]{color:var(--muted-2);text-decoration-color:var(--muted-2);cursor:not-allowed;pointer-events:none}

/* ---------- Chart stage ---------- */
.chart-stage{display:flex;flex-direction:column;min-height:0;overflow:hidden}
.stage-topline{display:flex;justify-content:space-between;align-items:baseline;gap:20px;margin-bottom:4px}
.eyebrow .chart-index,.eyebrow .chart-total{color:var(--ink)}
.chart-title-row{display:flex;justify-content:space-between;align-items:center;gap:18px;margin:2px 0 0}
.chart-stage h2{font-family:var(--font-body);font-size:clamp(1.3rem,2.4vw,1.85rem);font-weight:800;letter-spacing:-.02em;color:var(--ink);margin:0}
.chart-deck{font-family:var(--font-display);font-style:italic;font-size:.95rem;color:var(--ink-muted);max-width:62ch;margin:0 0 8px}
.chart-shell{flex:1 1 auto;min-height:0;min-width:0;display:flex;flex-direction:column}
.chart-shell.d3-enhanced .calendar-wrap,.chart-shell.d3-enhanced .line-chart-wrap,.chart-shell.d3-enhanced .accessible-data{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap}
.chart-shell.has-render{position:relative}
.chart-pager{display:flex;gap:16px}
.chart-pager .arrow-button{display:inline-flex;align-items:center;gap:4px;width:max-content;padding:8px 4px;min-height:24px;border:0;background:transparent;color:var(--ink);font-family:var(--font-body);font-weight:800;font-size:.85rem;letter-spacing:.01em;text-decoration:none}
.chart-pager .arrow-button[aria-disabled="false"]:hover{color:var(--accent)}
.chart-pager .arrow-button[aria-disabled="true"]{color:var(--muted-2);cursor:not-allowed;pointer-events:none}
.chart-pager .arrow-glyph{font-size:1.2rem;font-weight:300;line-height:.5}
.chart-pager .arrow-glyph-label{font-size:.7rem;letter-spacing:.06em;text-transform:uppercase;line-height:1}

/* Calendar scroll container — fixed 7-row height, hides scrollbar. */
.calendar-scroll-wrap{overflow-x:auto;overflow-y:hidden;max-width:100%;scrollbar-width:none;-ms-overflow-style:none}
.calendar-scroll-wrap::-webkit-scrollbar{display:none}
.calendar-scroll-wrap .d3-calendar-svg{display:block;height:220px;width:auto;max-width:none}

/* SSR fallback for the calendar — must match the D3 viewbox metrics so
   the .d3-enhanced clip is the only switch between SSR and D3 scenes.
   Content sizes itself to fit inside the parent so a wide calendar
   shrinks on narrow viewports instead of forcing a horizontal scrollbar. */
.calendar-wrap,.line-chart-wrap{flex:1 1 auto;min-height:0}
.d3-scene{flex:0 1 auto;min-height:0;min-width:0}
.d3-scene[hidden]{display:none}
.d3-calendar-svg,.d3-line-svg{display:block;width:100%;height:100%;max-width:100%;max-height:100%}

/* D3 calendar layout — frozen weekday column + scrollable SVG */
.calendar-layout{display:flex;gap:4px;align-items:stretch;width:100%}
.weekday-fixed{display:flex;flex-direction:column;justify-content:flex-start;gap:2px;padding-top:28px;flex:0 0 auto;width:32px}
.weekday-fixed span{font:300 10px/24px SFMono,Monaco,Menlo,monospace;letter-spacing:.04em;color:var(--muted);height:24px;display:flex;align-items:center}
.calendar-scroll-wrap{flex:1 1 auto;min-width:0;overflow-x:auto;overflow-y:hidden}
.d3-month-label{font:600 12px/1 var(--font-display);letter-spacing:.01em;fill:var(--ink);text-anchor:middle}

/* SSR calendar layout — staircase grid.
   Each cell placed by grid-column (col) and grid-row (weekday+2).
   Row 1 reserved for month labels. */
.calendar-grid{display:grid;grid-template-columns:42px 1fr;width:100%;padding:6px 0 0}
.weekday-labels{display:grid;grid-template-rows:repeat(7,24px);gap:1px;padding-top:20px;color:var(--muted);font-size:.6rem;font-weight:700;letter-spacing:.06em;text-transform:uppercase;font-family:var(--font-body)}
.weekday-labels span{align-self:center}
.calendar-canvas{display:grid;grid-template-columns:repeat(var(--max-col,80),1fr);grid-template-rows:auto repeat(7,24px);gap:1px;position:relative;width:100%}
.calendar-cell{width:100%;aspect-ratio:1;min-width:0;min-height:24px;padding:0;border:0;background:var(--hairline);position:relative;cursor:pointer}
.calendar-cell:hover,.calendar-cell:focus-visible{outline:2px solid var(--ink);outline-offset:1px;z-index:2}
.calendar-cell.structural{background:transparent;cursor:default}
.calendar-cell.missing{background:var(--paper);border:1px solid var(--hairline);background-image:linear-gradient(45deg,transparent 47%,var(--missing) 47.5%,var(--missing) 52.5%,transparent 53%),linear-gradient(-45deg,transparent 47%,var(--missing) 47.5%,var(--missing) 52.5%,transparent 53%);background-size:100% 100%}
.calendar-cell.neutral{background:var(--hairline)}
.band-0{background:var(--band-0)}.band-1{background:var(--band-1)}.band-2{background:var(--band-2)}.band-3{background:var(--band-3)}.band-4{background:var(--band-4)}.band-5{background:var(--band-5)}.band-6{background:var(--band-6)}.band-7{background:var(--band-7)}.band-8{background:var(--band-8)}.band-9{background:var(--band-9)}
.month-label{color:var(--ink);font-size:.7rem;font-weight:600;letter-spacing:.01em;white-space:nowrap;font-family:var(--font-display);pointer-events:none;grid-row:1;text-align:center;align-self:end;padding-bottom:4px}
.legend{display:flex;flex-direction:column;gap:6px;margin:2px 0 0;color:var(--muted);font-size:.7rem;line-height:1.45;letter-spacing:.02em;font-family:var(--font-body)}
.legend-caption{margin:0;font-weight:700;color:var(--ink);font-variant-numeric:tabular-nums;letter-spacing:.005em}
.legend-meta{margin:0;color:var(--muted)}
.legend-missing-note{color:var(--missing)}
.legend-gradient-wrap{position:relative;width:100%}
.legend-swatches{display:flex;width:100%;height:14px;border:1px solid var(--hairline);border-radius:2px;overflow:hidden}
.legend-swatch{flex:1 1 0;min-width:0;height:100%;cursor:pointer;transition:outline .15s,transform .15s}
.legend-swatch.active{outline:2px solid var(--ink);outline-offset:-1px;transform:scaleY(1.15)}
.legend-swatches .band-0{background:var(--band-0)}.legend-swatches .band-1{background:var(--band-1)}.legend-swatches .band-2{background:var(--band-2)}.legend-swatches .band-3{background:var(--band-3)}.legend-swatches .band-4{background:var(--band-4)}.legend-swatches .band-5{background:var(--band-5)}.legend-swatches .band-6{background:var(--band-6)}.legend-swatches .band-7{background:var(--band-7)}.legend-swatches .band-8{background:var(--band-8)}.legend-swatches .band-9{background:var(--band-9)}
.legend-ticks{position:relative;height:16px;margin-top:2px}
.legend-tick{position:absolute;transform:translateX(-50%);font:400 9px/1 SFMono,Monaco,Menlo,monospace;color:var(--muted);white-space:nowrap}

/* SSR line chart */
.line-chart-wrap{width:100%}
.line-chart{display:block;width:100%;max-width:100%;height:auto}
.grid line{stroke:var(--hairline);stroke-width:1}
.grid text,.x-label{fill:var(--muted);font:11px var(--font-body)}
.series{stroke-width:3;fill:none}
.series.commute,.point.commute,.whisker.commute{stroke:var(--accent);fill:var(--accent)}
.series.casual,.point.casual,.whisker.casual{stroke:var(--gold);fill:var(--gold)}
.point{stroke:var(--paper);stroke-width:3;cursor:pointer}
.point-hit{cursor:pointer}
.point-hit:focus{outline:none}
.point-hit:focus + .point{stroke:var(--ink);stroke-width:4}
.point:focus{outline:none;stroke:var(--ink);stroke-width:4}
.whisker{stroke-width:2;opacity:.45}
.point-label{font:700 11px var(--font-body)}
.point-label.commute,.series-label.commute{fill:var(--accent)}
.point-label.casual,.series-label.casual{fill:var(--gold)}
.series-label{font:700 11px var(--font-body)}
.insight{margin:6px 0 8px;color:var(--ink);font:600 1rem var(--font-display);letter-spacing:-.01em;max-width:60ch}

/* SSR fallback tables (the accessible-data details block) */
.accessible-data{margin-top:6px;font-size:.7rem;color:var(--muted);position:relative;z-index:1;background:var(--paper)}
.accessible-data summary{color:var(--accent);font-weight:800;cursor:pointer;letter-spacing:.04em;min-height:24px;display:flex;align-items:center;padding:2px 0}
.table-scroll{max-height:400px;overflow-y:auto;border:1px solid var(--hairline);border-radius:2px}
table{width:100%;border-collapse:collapse;font-size:.7rem}
th,td{padding:4px 6px;border-bottom:1px solid var(--hairline);text-align:left;white-space:nowrap}
th{color:var(--muted);font-size:.62rem;letter-spacing:.06em;text-transform:uppercase}

/* Footer */
.site-footer{padding:8px 0 0;border-top:1px solid var(--hairline);color:var(--muted);font-size:.66rem;line-height:1.4;display:flex;justify-content:space-between;align-items:flex-end;gap:18px}
.site-footer p{margin-bottom:3px}
.site-footer .chart-method{color:var(--muted);font-size:.7rem;line-height:1.45;border-left:2px solid var(--accent);padding:2px 0 2px 9px;max-width:62ch}
.site-footer .copyright{font-family:var(--font-display);font-style:italic;font-size:.8rem;font-weight:400;color:var(--ink-muted);letter-spacing:.01em;margin-top:6px}

/* Tooltip (D3 hover) */
.chart-tooltip{position:fixed;z-index:30;width:360px;min-height:120px;transform:translate(-50%,-100%);padding:0;background:var(--paper);color:var(--ink);font-size:.78rem;line-height:1.4;pointer-events:none;box-shadow:0 6px 20px rgba(10,10,10,.2);font-family:var(--font-body);border-radius:6px;border:1px solid var(--hairline);overflow:hidden}
.chart-tooltip .tt-head{display:flex;align-items:center;gap:8px;padding:10px 14px 6px}
.chart-tooltip .tt-color{display:inline-block;width:12px;height:12px;border-radius:2px;flex-shrink:0;border:1px solid rgba(0,0,0,.1);vertical-align:middle;margin-right:6px}
.chart-tooltip .tt-date{font-family:var(--font-display);font-weight:600;font-size:.88rem;color:var(--ink)}
.chart-tooltip .tt-rides{font-weight:700;color:var(--ink);font-variant-numeric:tabular-nums;padding:0 14px 6px;font-size:.82rem;display:flex;align-items:center}
.chart-tooltip .tt-band{color:var(--muted);font-size:.68rem;padding:0 14px 4px}
.chart-tooltip .tt-table{width:100%;border-collapse:collapse;font-size:.7rem;margin-top:2px}
.chart-tooltip .tt-table th{padding:5px 14px 3px;color:var(--muted);font-size:.62rem;letter-spacing:.04em;text-transform:uppercase;text-align:left;font-weight:700;border-bottom:1px solid var(--hairline)}
.chart-tooltip .tt-table td{padding:4px 14px;color:var(--ink);font-variant-numeric:tabular-nums}
.chart-tooltip .tt-table td:last-child{text-align:right;font-weight:600}
.chart-tooltip .tt-missing-inline{color:var(--muted);font-style:italic;font-weight:400}
.chart-tooltip .tt-spacer{height:8px}
.chart-tooltip .tt-missing{color:var(--muted);font-style:italic;padding:10px 14px}
.chart-tooltip[hidden]{display:none}

/* D3 progressive-enhancement layers (calendar + line scenes) */
.d3-cell rect{stroke:transparent}
.d3-cell:focus{outline:none}
.d3-cell:focus rect,.d3-cell:hover rect{stroke:var(--ink);stroke-width:2;stroke-opacity:.5}
.missing-cross{stroke:var(--missing);stroke-width:0.8}
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
.d3-y-label,.d3-x-label{fill:var(--muted);font:700 11px var(--font-body)}
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
