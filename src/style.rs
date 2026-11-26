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
.site-header{display:grid;grid-template-columns:minmax(0,1fr) auto;grid-template-areas:"eyebrow eyebrow" "title source" "subtitle subtitle";column-gap:24px;padding-bottom:10px;border-bottom:2px solid var(--ink)}
.site-header>.eyebrow{grid-area:eyebrow}
.site-header h1{grid-area:title;font-family:var(--font-body);font-size:clamp(1.6rem,3.3vw,2.8rem);font-weight:900;letter-spacing:-.035em;line-height:.95;margin:0;max-width:18ch}
.site-header .subtitle{grid-area:subtitle;font-family:var(--font-display);font-style:italic;font-weight:300;font-size:clamp(.95rem,1.4vw,1.15rem);line-height:1.35;color:#000000a6;max-width:55ch;margin:.35rem 0 0}
.source-links{grid-area:source;align-self:center;justify-self:end;display:inline-flex;align-items:center;gap:.45rem;min-height:24px;border:1px solid var(--ink);padding:.6rem .8rem;background:transparent;color:var(--ink);font-family:var(--font-header);font-weight:700;font-size:.72rem;letter-spacing:.1em;line-height:1;text-transform:uppercase;white-space:nowrap}
.source-links svg{display:block;width:1rem;height:1rem;fill:currentColor;flex:0 0 auto}
.source-links a{color:inherit;text-decoration:none}
.source-links a:hover{color:var(--accent);text-decoration:underline;text-decoration-thickness:1px;text-underline-offset:3px}
.source-links a:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
.source-separator{color:var(--muted-2)}
@media (max-width:480px){
  .site-header{grid-template-columns:1fr;grid-template-areas:"eyebrow" "title" "subtitle" "source"}
  .source-links{justify-self:start;margin-top:.6rem}
}

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
.chart-title-row{display:flex;justify-content:space-between;align-items:center;gap:18px;row-gap:10px;flex-wrap:wrap;margin:2px 0 0}
.chart-stage h2{font-family:var(--font-body);font-size:clamp(1.3rem,2.4vw,1.85rem);font-weight:800;letter-spacing:-.02em;color:var(--ink);margin:0}
.chart-deck{font-family:var(--font-display);font-style:italic;font-size:.95rem;color:var(--ink-muted);max-width:62ch;margin:0 0 8px}
.chart-shell{flex:1 1 auto;min-height:0;min-width:0;display:flex;flex-direction:column}
.chart-shell.d3-enhanced .calendar-wrap,.chart-shell.d3-enhanced .line-chart-wrap,.chart-shell.d3-enhanced .baseline-wrap,.chart-shell.d3-enhanced .waterfall-wrap,.chart-shell.d3-enhanced .mix-wrap,.chart-shell.d3-enhanced .ncmc-wrap,.chart-shell.d3-enhanced .qr-ssr-wrap,.chart-shell.d3-enhanced .accessible-data{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap}
.chart-shell.has-render{position:relative}
.chart-pager{display:flex;gap:16px;align-items:center;justify-content:flex-end;flex-wrap:wrap;min-width:0}
.chart-pager .arrow-button{display:inline-flex;align-items:center;gap:4px;width:max-content;padding:8px 4px;min-height:24px;border:0;background:transparent;color:var(--ink);font-family:var(--font-body);font-weight:800;font-size:.85rem;letter-spacing:.01em;text-decoration:none}
.chart-pager .arrow-button[aria-disabled="false"]:hover{color:var(--accent)}
.chart-pager .arrow-button[aria-disabled="true"]{color:var(--muted-2);cursor:not-allowed;pointer-events:none}
.chart-pager .chart-jump{width:clamp(8rem,22vw,14rem);max-width:100%;min-height:32px;border:1px solid var(--ink);border-radius:2px;background:var(--paper);color:var(--ink);font:800 .68rem/1 var(--font-body);letter-spacing:.06em;text-transform:uppercase;padding:.55rem .7rem;cursor:pointer}
.chart-pager .chart-jump:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
.chart-pager .arrow-glyph{font-size:1.2rem;font-weight:300;line-height:.5}
.chart-pager .arrow-glyph-label{font-size:.7rem;letter-spacing:.06em;text-transform:uppercase;line-height:1}

/* Calendar scroll container — fixed 7-row height, hides scrollbar. */
.calendar-scroll-wrap{overflow-x:auto;overflow-y:hidden;max-width:100%;scrollbar-width:none;-ms-overflow-style:none}
.calendar-scroll-wrap::-webkit-scrollbar{display:none}
.calendar-scroll-wrap .d3-calendar-svg{display:block;max-width:none}

/* SSR fallback for the calendar — must match the D3 viewbox metrics so
   the .d3-enhanced clip is the only switch between SSR and D3 scenes.
   Content sizes itself to fit inside the parent so a wide calendar
   shrinks on narrow viewports instead of forcing a horizontal scrollbar. */
.calendar-wrap,.line-chart-wrap,.baseline-wrap,.waterfall-wrap,.mix-wrap,.ncmc-wrap,.qr-ssr-wrap{flex:1 1 auto;min-height:0}
.d3-scene{flex:0 1 auto;min-height:0;min-width:0}
.d3-scene[hidden]{display:none}
.d3-line-svg,.d3-baseline-svg,.d3-waterfall-svg,.d3-mix-svg,.d3-ncmc-svg,.d3-qr-svg{display:block;width:100%;height:100%;max-width:100%;max-height:100%}
.d3-line-scene{display:flex;flex-direction:column;flex:1 1 auto;min-height:240px}
.d3-line-scene .d3-line-svg{flex:1 1 0;min-height:0;height:auto;max-height:none}
.d3-line-scene .d3-insight{flex:0 0 auto}

/* D3 calendar layout — frozen weekday column + scrollable SVG */
.calendar-layout{display:flex;gap:4px;align-items:stretch;width:100%}
.weekday-fixed{display:flex;flex-direction:column;justify-content:flex-start;gap:2px;padding-top:28px;flex:0 0 auto;width:32px}
.weekday-fixed span{font:300 10px/24px SFMono,Monaco,Menlo,monospace;letter-spacing:.04em;color:var(--muted);height:24px;display:flex;align-items:center}
.calendar-scroll-wrap{flex:1 1 auto;min-width:0;overflow-x:auto;overflow-y:hidden}
.d3-month-label{font:600 12px/1 var(--font-display);letter-spacing:.01em;fill:var(--ink);text-anchor:start}
.d3-month-line{shape-rendering:crispEdges}

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
.series.commute,.series.casual{fill-opacity:.18}
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
/* Weekday-normalised baseline chart */
.baseline-wrap{width:100%}
.baseline-chart{display:block;width:100%;max-width:100%;height:auto}
.baseline-band{fill:var(--accent);fill-opacity:.11;stroke:none}
.baseline-median{stroke:var(--accent);stroke-width:1.8;stroke-dasharray:5 4;opacity:.78}
.baseline-actual{stroke:var(--ink);stroke-width:2.2;opacity:.88}
.baseline-point{stroke:var(--paper);stroke-width:2.4;cursor:pointer}
.baseline-point.within,.legend-dot.within{fill:#6f7480}
.baseline-point.high,.legend-dot.high{fill:var(--accent)}
.baseline-point.low,.legend-dot.low{fill:var(--missing)}
.baseline-point.insufficient{fill:var(--muted-2);opacity:.65}
.baseline-point-hit:focus{outline:none}
.baseline-point-hit:focus + .baseline-point{stroke:var(--ink);stroke-width:3.4}
.baseline-legend{display:flex;flex-wrap:wrap;gap:8px 18px;align-items:center;margin-top:4px;color:var(--muted);font:700 .68rem var(--font-body);letter-spacing:.02em}
.baseline-legend span{display:inline-flex;align-items:center;gap:6px;min-height:20px}
.legend-swatch-band{display:inline-block;width:18px;height:10px;background:var(--accent);opacity:.16;border:1px solid var(--accent)}
.legend-line-median{display:inline-block;width:18px;height:0;border-top:2px dashed var(--ink)}
.legend-dot{display:inline-block;width:9px;height:9px;border-radius:50%;border:1px solid var(--paper)}
.baseline-empty{margin:32px auto;color:var(--muted);font:600 .95rem var(--font-body);text-align:center}
.baseline-svg-insight{fill:var(--ink);font:600 13px var(--font-display);letter-spacing:-.01em}
/* Fare-media mix */
.mix-wrap{width:100%}
.mix-chart{display:block;width:100%;max-width:100%;height:auto}
.mix-smart-card{fill:var(--accent);background:var(--accent)}
.mix-ncmc{fill:#d95f4f;background:#d95f4f}
.mix-token{fill:#3f9d69;background:#3f9d69}
.mix-qr{fill:#3182ce;background:#3182ce}
.mix-group-ticket{fill:#c8a44d;background:#c8a44d}
.mix-segment{stroke:var(--paper);stroke-width:.7}
.mix-total-bar{fill:var(--ink);fill-opacity:.34}
.mix-gap{fill:none;stroke:var(--missing);stroke-width:1;stroke-dasharray:4 3;opacity:.55}
.mix-key{display:inline-block;width:10px;height:10px;border-radius:2px}
.mix-legend{display:flex;flex-wrap:wrap;gap:8px 16px;align-items:center;margin-top:4px;color:var(--muted);font:700 .68rem var(--font-body)}
.mix-legend span{display:inline-flex;align-items:center;gap:6px}
.mix-x-label{font-size:10px}
.mix-coverage-label{fill:var(--muted);font:700 9.5px var(--font-mono);letter-spacing:.02em}
.mix-empty{margin:32px auto;color:var(--muted);font:600 .95rem var(--font-body);text-align:center;max-width:52ch}
.d3-mix-scene{display:flex;flex-direction:column;flex:1 1 auto;min-height:240px;overflow-x:auto;overflow-y:hidden;scrollbar-width:thin}
.d3-mix-scene .d3-mix-svg{flex:1 1 0;min-height:0;height:auto;max-height:none}
.d3-mix-scene .d3-insight{flex:0 0 auto;margin-bottom:4px}
.d3-mix-scene .d3-mix-svg.mix-narrow{flex:0 0 auto}
.d3-mix-segment:focus,.d3-mix-total:focus{outline:none;stroke:var(--ink);stroke-width:2}
/* NCMC share */
.ncmc-wrap{width:100%}
.ncmc-chart{display:block;width:100%;max-width:100%;height:auto}
.ncmc-share-line{stroke:var(--accent);stroke-width:2.6;fill:none}
.ncmc-share-point{fill:var(--accent);stroke:var(--paper);stroke-width:2.2;cursor:pointer}
.ncmc-count-bar{fill:var(--gold);fill-opacity:.72}
.ncmc-line-key{stroke:var(--accent);stroke-width:3}
.ncmc-count-key{fill:var(--gold);fill-opacity:.72}
.ncmc-empty{margin:32px auto;color:var(--muted);font:600 .95rem var(--font-body);text-align:center;max-width:52ch}
.d3-ncmc-scene{display:flex;flex-direction:column;flex:1 1 auto;min-height:240px;overflow-x:auto;overflow-y:hidden;scrollbar-width:thin}
.d3-ncmc-scene .d3-ncmc-svg{flex:1 1 0;min-height:0;height:auto;max-height:none}
.d3-ncmc-scene .d3-insight{flex:0 0 auto;margin-bottom:4px}
.d3-ncmc-scene .d3-ncmc-svg.mix-narrow{flex:0 0 auto}
.d3-ncmc-point:focus,.d3-ncmc-count:focus{outline:none;stroke:var(--ink);stroke-width:2}
/* QR channel mix */
.mix-qr-namma,.qr-qr-namma{fill:#2f7fbc;background:#2f7fbc}
.mix-qr-whatsapp,.qr-qr-whatsapp{fill:#4c9a63;background:#4c9a63}
.mix-qr-paytm,.qr-qr-paytm{fill:#6f6ab8;background:#6f6ab8}
.mix-qr-ondc,.qr-qr-ondc{fill:#c8a44d;background:#c8a44d}
.qr-ssr-months{display:grid;gap:8px;margin:16px 0}
.qr-ssr-month{display:grid;grid-template-columns:64px 1fr minmax(100px,auto);gap:10px;align-items:center;font-size:.78rem}
.qr-ssr-label{font-weight:700}
.qr-ssr-bar{display:flex;height:16px;overflow:hidden;border-radius:2px;background:var(--surface)}
.qr-segment{display:block;height:100%}
.qr-ssr-gap{color:var(--muted);font-size:.72rem}
.qr-ssr-meta{color:var(--muted);font-family:var(--font-mono);font-size:.68rem;text-align:right}
.qr-empty{margin:32px auto;color:var(--muted);font:600 .95rem var(--font-body);text-align:center;max-width:52ch}
.d3-qr-scene{display:flex;flex-direction:column;flex:1 1 auto;min-height:240px;overflow-x:auto;overflow-y:hidden;scrollbar-width:thin}
.d3-qr-scene .d3-qr-svg{flex:1 1 0;min-height:0;height:auto;max-height:none}
.d3-qr-scene .d3-insight{flex:0 0 auto;margin-bottom:4px}
.d3-qr-scene .d3-qr-svg.mix-narrow{flex:0 0 auto}
.d3-qr-segment:focus,.d3-qr-total:focus{outline:none;stroke:var(--ink);stroke-width:2}
/* Component-delta waterfall */
.waterfall-wrap{width:100%}
.waterfall-chart{display:block;width:100%;max-width:100%;height:auto}
.waterfall-bar.positive{fill:var(--accent);fill-opacity:.88}
.waterfall-bar.negative{fill:var(--missing);fill-opacity:.88}
.waterfall-connector{stroke:var(--muted);stroke-width:1.2;stroke-dasharray:3 3;opacity:.75}
.waterfall-total-anchor{stroke:var(--ink);stroke-width:4;stroke-linecap:round}
.waterfall-total-anchor.actual{stroke:var(--accent)}
.waterfall-total-label{font:800 12px var(--font-body);fill:var(--ink)}
.waterfall-delta{font:800 11px var(--font-body)}
.waterfall-delta.positive{fill:var(--accent)}
.waterfall-delta.negative{fill:var(--missing)}
.waterfall-hit:focus{outline:none}
.waterfall-hit:focus + .waterfall-bar{stroke:var(--ink);stroke-width:2}
.waterfall-x-label{font-size:10px}
.waterfall-legend{display:flex;flex-wrap:wrap;gap:8px 18px;align-items:center;margin-top:4px;color:var(--muted);font:700 .68rem var(--font-body)}
.waterfall-legend span{display:inline-flex;align-items:center;gap:6px}
.waterfall-empty{margin:32px auto;color:var(--muted);font:600 .95rem var(--font-body);text-align:center;max-width:52ch}
.baseline-svg-legend text{fill:var(--muted);font:700 11px var(--font-body)}
.legend-mark-band{fill:var(--accent);fill-opacity:.14;stroke:var(--accent);stroke-opacity:.45}
.legend-mark-median{stroke:var(--accent);stroke-width:1.8;stroke-dasharray:5 4}
.legend-mark-dot{stroke:var(--paper);stroke-width:1.4}
.legend-mark-dot.within{fill:#6f7480}
.legend-mark-dot.high{fill:var(--accent)}
.legend-mark-dot.low{fill:var(--missing)}

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
.site-footer .compute-note{color:var(--muted);font-size:.7rem;line-height:1.45;border-left:2px solid var(--accent);padding:2px 0 2px 9px;max-width:62ch}
.site-footer .compute-note[hidden]{display:none}
.site-footer .copyright{font-family:var(--font-display);font-style:italic;font-size:.8rem;font-weight:400;color:var(--ink-muted);letter-spacing:.01em;margin-top:6px}

/* Tooltip (D3 hover) */
.chart-tooltip{position:fixed;z-index:30;width:240px;min-height:120px;transform:translate(-50%,-100%);padding:0;background:var(--paper);color:var(--ink);font-size:.78rem;line-height:1.4;pointer-events:none;box-shadow:0 6px 20px rgba(10,10,10,.2);font-family:var(--font-body);border-radius:6px;border:1px solid var(--hairline);overflow:hidden}
.chart-tooltip .tt-head{display:flex;align-items:center;gap:8px;padding:10px 14px 6px}
.chart-tooltip .tt-color{display:inline-block;width:12px;height:12px;border-radius:2px;flex-shrink:0;border:1px solid rgba(0,0,0,.1);vertical-align:middle;margin-right:6px}
.chart-tooltip .tt-date{font-family:var(--font-display);font-weight:600;font-size:.88rem;color:var(--ink)}
.chart-tooltip .tt-rides{font-weight:700;color:var(--ink);font-variant-numeric:tabular-nums;padding:0 14px 6px;font-size:.82rem;display:flex;align-items:center}
.chart-tooltip .tt-band{color:var(--muted);font-size:.68rem;padding:0 14px 4px}
.chart-tooltip .tt-table{margin-top:2px}
.chart-tooltip .tt-thead{padding:5px 14px 3px;color:var(--muted);font-size:.62rem;letter-spacing:.04em;text-transform:uppercase;font-weight:700;border-bottom:1px solid var(--hairline)}
.chart-tooltip .tt-tbody{padding:2px 0 6px}
.chart-tooltip .tt-row{display:flex;flex-direction:column;padding:4px 14px}
.chart-tooltip .tt-row + .tt-row{border-top:1px solid var(--hairline)}
.chart-tooltip .tt-row-label{color:var(--muted);font-size:.62rem;letter-spacing:.04em;text-transform:uppercase;font-weight:700}
.chart-tooltip .tt-row-value{color:var(--ink);font-variant-numeric:tabular-nums;font-weight:600;font-size:.78rem}
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
.d3-baseline-point{stroke:var(--paper);stroke-width:2.4;cursor:pointer}
.d3-baseline-point-group:focus .d3-baseline-point{stroke:var(--ink);stroke-width:3.4}
.d3-waterfall-group:focus .waterfall-bar{stroke:var(--ink);stroke-width:2}
.d3-whisker{stroke-width:2;opacity:.45}
.d3-point-label{font:700 11px var(--font-body)}
.d3-point-label.commute,.d3-series-label.commute{fill:var(--accent)}
.d3-point-label.casual,.d3-series-label.casual{fill:var(--gold)}
.d3-series-label{font:700 11px var(--font-body)}
.d3-y-label,.d3-x-label{fill:var(--muted);font:700 11px var(--font-body)}
.weekday-label{fill:var(--muted);font:700 10px var(--font-body);letter-spacing:.05em;text-transform:uppercase}

/* Fallback: when the viewport is too short to fit everything, the
   whole shell becomes scrollable. This is the ONLY scroll the page
   ever uses, and it kicks in below the budget breakpoint (~835px tall
   for the 1280-wide shell, or below 880px wide for narrower devices). */
@media (max-height:835px),(max-width:880px){
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
body.chart-calendar .data-card-wrap,body.chart-commute-casual .data-card-wrap,body.chart-weekday-baseline .data-card-wrap,body.chart-component-waterfall .data-card-wrap,body.chart-fare-media-mix .data-card-wrap,body.chart-ncmc-share .data-card-wrap,body.chart-qr-channel-mix .data-card-wrap{display:none}
body.chart-data-card .legend,body.chart-commute-casual .legend,body.chart-weekday-baseline .legend,body.chart-component-waterfall .legend,body.chart-fare-media-mix .legend,body.chart-ncmc-share .legend,body.chart-qr-channel-mix .legend{display:none}
.data-card-wrap{width:100%}
.data-card-inner{background:var(--paper);border:1px solid var(--hairline);border-radius:12px;padding:28px 32px;display:flex;flex-direction:column;gap:24px}
.dc-top-story{display:flex;flex-direction:column;gap:20px}
.dc-bottom-story{display:flex;flex-direction:column;gap:12px}
.dc-date-row{display:flex;justify-content:center;align-items:center;gap:16px}
.dc-day-nav{display:inline-flex;align-items:center;justify-content:center;width:44px;height:44px;border-radius:50%;border:1px solid var(--hairline);background:var(--surface);color:var(--ink);font-size:1.2rem;font-weight:300;line-height:1;cursor:pointer;transition:background .15s,color .15s,border-color .15s,opacity .15s;flex-shrink:0}
.dc-day-nav:hover:not([aria-disabled="true"]){background:var(--accent);color:var(--paper);border-color:var(--accent)}
.dc-day-nav:focus-visible{outline:2px solid var(--accent);outline-offset:2px}
.dc-day-nav[aria-disabled="true"]{color:var(--muted-2);cursor:not-allowed;opacity:.4}
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
