pub(crate) const WRAPPER_OPEN: &str = "<script>";
pub(crate) const WRAPPER_CLOSE: &str = "</script>";
pub(crate) const CLIENT_SCRIPT: &str = r#"(() => {
  const D3 = window.d3;
  const GSAP = window.gsap;
  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const canAnimate = !reducedMotion;
  // Chart titles are rendered server-side from ChartDefinition. The
  // client owns chart and range state in sessionStorage and always keeps
  // the URL at "/"; labels are read from the live DOM.
  const startPicker = document.querySelector('[data-date-type="start"]');
  const endPicker = document.querySelector('[data-date-type="end"]');
  const startSlider = document.querySelector('#range-start');
  const endSlider = document.querySelector('#range-end');
  const tooltip = document.querySelector('#chart-tooltip');
  const chartShell = document.querySelector('.chart-shell');
  const chartTitle = document.querySelector('#chart-title');
  const chartDeck = document.querySelector('.chart-deck');
  const chartMethod = document.querySelector('.chart-method');
  const chartEyebrow = document.querySelector('.stage-topline .eyebrow');
  const priorButton = document.querySelector('.arrow-button.prev');
  const nextButton = document.querySelector('.arrow-button.next');
  const chartJump = document.querySelector('.chart-jump');
  const payloadNode = document.querySelector('#chart-data');
  const computeNote = document.getElementById('compute-note');
  if (!startPicker || !endPicker || !startSlider || !endSlider || !payloadNode || !chartShell) return;

  let payload = JSON.parse(payloadNode.textContent || '{}');
  let dates = payload.dataset?.availableDates || (document.querySelector('.range-controls')?.dataset.availableDates || '').split(',').filter(Boolean);
  if (!dates.length) return;

  let payloadDirty = false;
  let rangeTimer;
  let chartResizeTimer;
  // Compute-core lifecycle: 'pending' until the wasm boot settles, 'ready'
  // once chartPayload can answer locally, 'server' when the page ships no
  // wasm assets (self-hosted build → /api/chart computes), 'failed' when
  // wasm exists but the live dataset could not be loaded.
  let computeState = 'pending';
  let computeSettled = null;

  const chartRegistry = { 'data-card': true, 'calendar': true, 'commute-casual': true, 'weekday-baseline': true, 'component-waterfall': true, 'fare-media-mix': true, 'ncmc-share': true, 'qr-channel-mix': true };
  const chartOrder = ['data-card', 'calendar', 'commute-casual', 'weekday-baseline', 'component-waterfall', 'fare-media-mix', 'ncmc-share', 'qr-channel-mix'];
  if (chartJump) {
    chartJump.querySelectorAll('option').forEach((option) => {
      if (!chartRegistry[option.value]) option.remove();
    });
  }

  const SS_KEY = 'metroDashState';
  // Reset to the latest datapoint on every page load; sessionStorage is only
  // used to keep transient navigation state within a single pageview.
  try { sessionStorage.removeItem(SS_KEY); } catch {}
  const saveState = (state) => {
    try { sessionStorage.setItem(SS_KEY, JSON.stringify(state)); } catch {}
    history.replaceState(state, '', '/');
  };

  let activeChart = 'data-card';

  const navigateToChart = (chart) => {
    if (chart === activeChart || !chartRegistry[chart]) return;
    activeChart = chart;
    syncChartChrome();
    if (chart === 'calendar') {
      const quickFilter = document.querySelector('.range-actions [data-preset="last-6-months"]');
      if (quickFilter) {
        return fetchRange(quickFilter.dataset.start, quickFilter.dataset.end, {animate: true});
      }
    }
    saveState({start: payload.range.start, end: payload.range.end, chart: activeChart});
    renderActiveChart(true);
  };

  const dateFormat = (value) => new Intl.DateTimeFormat('en-IN', {day: 'numeric', month: 'short', year: 'numeric'}).format(new Date(`${value}T00:00:00`));
  const pad = (n) => String(n).padStart(2, '0');
  const getPickerValue = (picker) => {
    const d = picker.querySelector('.date-day').value;
    const m = picker.querySelector('.date-month').value;
    const y = picker.querySelector('.date-year').value;
    if (!d || !m || !y) return '';
    return `${y}-${pad(m)}-${pad(d)}`;
  };
  const setPickerValue = (picker, value) => {
    if (!value) return;
    const [y, m, d] = value.split('-');
    picker.querySelector('.date-day').value = Number(d);
    picker.querySelector('.date-month').value = Number(m);
    picker.querySelector('.date-year').value = y;
  };
  const nearestIndex = (value, fallback) => {
    if (!value) return fallback;
    const wanted = new Date(`${value}T00:00:00`).getTime();
    if (!Number.isFinite(wanted)) return fallback;
    let best = 0;
    let distance = Infinity;
    dates.forEach((date, index) => {
      const next = Math.abs(new Date(`${date}T00:00:00`).getTime() - wanted);
      if (next < distance) { best = index; distance = next; }
    });
    return best;
  };
  const compact = (value) => {
    if (value == null || Number.isNaN(value)) return '—';
    const abs = Math.abs(value);
    if (abs >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
    if (abs >= 1_000) return `${Math.round(value / 1_000)}k`;
    return `${Math.round(value)}`;
  };
  const compactAxis = (value, spread) => {
    if (value == null || Number.isNaN(value)) return '—';
    const abs = Math.abs(value);
    if (abs >= 1_000_000) {
      const decimals = spread < 250_000 ? 2 : 1;
      return `${(value / 1_000_000).toFixed(decimals)}M`;
    }
    if (abs >= 1_000) return `${Math.round(value / 1_000)}k`;
    return `${Math.round(value)}`;
  };
  const cssVar = (name) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  const _bandColors = Array.from({length: 10}, (_, i) => cssVar(`--band-${i}`));
  const _paperColor = cssVar('--paper');
  const _gridColor = cssVar('--hairline');
  const colourFor = (cell) => cell.missing ? _paperColor : cell.bandIndex == null ? _gridColor : _bandColors[cell.bandIndex];
  const setDisabled = (el, disabled) => {
    if (!el) return;
    el.setAttribute('aria-disabled', String(disabled));
    el.setAttribute('tabindex', disabled ? '-1' : '0');
  };
  const updateUrl = ({start, end, chart = activeChart}) => {
    saveState({start, end, chart});
  };
  const showTooltip = (target) => {
    const text = target.dataset.tooltip || target.getAttribute('aria-label');
    if (!tooltip || !text) return;
    const bounds = target.getBoundingClientRect();
    tooltip.textContent = text;
    tooltip.hidden = false;
    tooltip.style.left = `${Math.min(window.innerWidth - 128, Math.max(128, bounds.left + bounds.width / 2))}px`;
    tooltip.style.top = `${Math.max(12, bounds.top - 10)}px`;
  };
  let _tooltipTimer = null;
  const showTooltipHTML = (html, target) => {
    if (!tooltip || !html) return;
    const bounds = target.getBoundingClientRect();
    if (_tooltipTimer) { clearTimeout(_tooltipTimer); _tooltipTimer = null; }
    tooltip.innerHTML = html;
    tooltip.hidden = false;
    tooltip.style.opacity = '1';
    tooltip.style.transition = 'none';
    tooltip.style.left = `${Math.min(window.innerWidth - 128, Math.max(128, bounds.left + bounds.width / 2))}px`;
    tooltip.style.top = `${Math.max(12, bounds.top - 10)}px`;
  };
  const hideTooltip = () => {
    if (!tooltip) return;
    tooltip.style.transition = 'opacity .4s ease-in';
    tooltip.style.opacity = '0';
    _tooltipTimer = setTimeout(() => { tooltip.hidden = true; }, 400);
  };
  const hideTooltipImmediately = () => {
    if (!tooltip) return;
    if (_tooltipTimer) { clearTimeout(_tooltipTimer); _tooltipTimer = null; }
    tooltip.style.transition = 'none';
    tooltip.style.opacity = '0';
    tooltip.hidden = true;
  };
  const bindTooltip = (selection) => {
    selection
      .attr('tabindex', 0)
      .attr('role', 'img')
      .on('mouseenter focus click', function () { showTooltip(this); })
      .on('mouseleave blur', hideTooltip);
  };
  const choreograph = (node) => {
    if (canAnimate && GSAP && node) GSAP.fromTo(node, {autoAlpha: 0, y: 8}, {autoAlpha: 1, y: 0, duration: 0.32, ease: 'power2.out'});
  };

  const updateActionLinks = (start, end) => {
    document.querySelectorAll('.range-actions .text-button').forEach((link) => {
      const linkStart = link.dataset.start;
      const linkEnd = link.dataset.end;
      setDisabled(link, start === linkStart && end === linkEnd);
    });
  };

  // Quick-filter links carry data-start/data-end baked at render time. Every
  // payload recomputes them (ChartPayload.presets), so re-stamp the links
  // whenever a new payload arrives — e.g. after the live dataset is adopted.
  const syncPresetTargets = () => {
    const presets = payload.presets || [];
    document.querySelectorAll('.range-actions [data-preset]').forEach((link) => {
      const preset = presets.find((entry) => entry.slug === link.dataset.preset);
      if (preset) {
        link.dataset.start = preset.start;
        link.dataset.end = preset.end;
      }
    });
  };

  // The year <select>s are rendered with options for the build-time dataset
  // bounds. A live dataset can span a year the snapshot does not, which would
  // leave setPickerValue selecting a year with no matching <option>.
  const syncYearOptions = () => {
    const minYear = Number((payload.dataset?.minDate || dates[0] || '').slice(0, 4)) || NaN;
    const maxYear = Number((payload.dataset?.maxDate || dates[dates.length - 1] || '').slice(0, 4)) || NaN;
    if (!Number.isFinite(minYear) || !Number.isFinite(maxYear) || minYear > maxYear) return;
    [startPicker, endPicker].forEach((picker) => {
      const select = picker.querySelector('.date-year');
      if (!select) return;
      const optionCount = select.options.length;
      if (optionCount &&
          Number(select.options[0].value) === minYear &&
          Number(select.options[optionCount - 1].value) === maxYear) return;
      const current = select.value;
      select.innerHTML = '';
      for (let year = minYear; year <= maxYear; year++) {
        const option = document.createElement('option');
        option.value = String(year);
        option.textContent = String(year);
        select.appendChild(option);
      }
      select.value = current;
    });
  };

  const syncControls = (nextPayload) => {
    payload = nextPayload;
    dates = payload.dataset.availableDates;
    startSlider.max = `${dates.length - 1}`;
    endSlider.max = `${dates.length - 1}`;
    startSlider.value = payload.range.startIndex;
    endSlider.value = payload.range.endIndex;
    syncYearOptions();
    setPickerValue(startPicker, payload.range.start);
    setPickerValue(endPicker, payload.range.end);
    syncPresetTargets();
    updateActionLinks(payload.range.start, payload.range.end);
  };

  const syncChartChrome = () => {
    const definition = payload.definitions?.find(def => def.slug === activeChart);
    if (definition) {
      if (chartTitle) chartTitle.textContent = definition.title;
      if (chartDeck) chartDeck.textContent = definition.deck;
      if (chartMethod) chartMethod.textContent = definition.method;
    }
    const eyebrowIndex = chartEyebrow?.querySelector('.chart-index');
    if (eyebrowIndex) {
      eyebrowIndex.textContent = String(chartOrder.indexOf(activeChart) + 1);
    }
    const idx = chartOrder.indexOf(activeChart);
    const prevSlug = idx > 0 ? chartOrder[idx - 1] : null;
    const nextSlug = idx < chartOrder.length - 1 ? chartOrder[idx + 1] : null;
    const prevDef = prevSlug ? payload.definitions?.find(def => def.slug === prevSlug) : null;
    const nextDef = nextSlug ? payload.definitions?.find(def => def.slug === nextSlug) : null;
    if (priorButton) {
      priorButton.setAttribute('data-chart', prevSlug || activeChart);
      priorButton.setAttribute('aria-label', prevDef ? `Previous chart: ${prevDef.title}` : 'Previous chart');
      setDisabled(priorButton, prevSlug === null);
    }
    if (nextButton) {
      nextButton.setAttribute('data-chart', nextSlug || activeChart);
      nextButton.setAttribute('aria-label', nextDef ? `Next chart: ${nextDef.title}` : 'Next chart');
      setDisabled(nextButton, nextSlug === null);
    }
    if (chartJump) chartJump.value = activeChart;
    document.body.className = document.body.className
      .replace(/chart-\S+/g, '')
      .trim() + ' chart-' + activeChart;
  };

  const ensureScene = (className) => {
    chartShell.classList.add('d3-enhanced');
    let scene = chartShell.querySelector(`.${className}`);
    if (!scene) {
      scene = document.createElement('div');
      scene.className = `d3-scene ${className}`;
      chartShell.prepend(scene);
    }
    chartShell.querySelectorAll('.d3-scene').forEach((candidate) => {
      candidate.hidden = candidate !== scene;
    });
    return D3.select(scene);
  };

  let _calSvg = null;
  let _swatchBound = false;

  const applyLegendFilter = (bandIdx) => {
    if (!_calSvg) return;
    _calSvg.selectAll('.d3-cell')
      .style('opacity', d => d.bandIndex === bandIdx ? 1 : 0)
      .style('pointer-events', d => d.bandIndex === bandIdx ? 'all' : 'none');
  };

  const clearLegendFilter = () => {
    if (!_calSvg) return;
    _calSvg.selectAll('.d3-cell')
      .style('opacity', 1)
      .style('pointer-events', 'all');
  };

  const syncLegend = () => {
    const summary = payload.summary;
    const caption = document.querySelector('.legend-caption');
    const meta = document.querySelector('.legend-meta:not(.legend-missing-note)');
    if (!caption || !meta) return;

    if (summary.validTotalCount === 0 || summary.totalMin == null || summary.totalMax == null) {
      caption.textContent = 'Observed range unavailable for this selection';
    } else if (summary.totalMin === summary.totalMax) {
      caption.textContent = `Observed: ${summary.totalMinLabel}`;
    } else {
      caption.textContent = `Observed: ${summary.totalMinLabel} \u2013 ${summary.totalMaxLabel}`;
    }

    if (summary.validTotalCount === 0) {
      meta.textContent = 'Buckets unavailable: no published totals in this range.';
    } else if (summary.validTotalCount < 10) {
      const s = summary.validTotalCount === 1 ? '' : 's';
      meta.textContent = `Buckets are approximate (${summary.validTotalCount} day${s} in range; tint carries limited signal).`;
    } else {
      meta.textContent = "Each cell's shade maps to its percentile of daily ridership within your selection.";
    }

    const pctMap = {};
    (summary.percentileValues || []).forEach(([p, v]) => { pctMap[p] = v; });
    const get = (p) => pctMap[p] || '';
    const titles = [
      `< p2  (< ${get(2)})`,
      `p2 \u2013 p5  (${get(2)} \u2013 ${get(5)})`,
      `p5 \u2013 p10  (${get(5)} \u2013 ${get(10)})`,
      `p10 \u2013 p25  (${get(10)} \u2013 ${get(25)})`,
      `p25 \u2013 p50  (${get(25)} \u2013 ${get(50)})`,
      `p50 \u2013 p75  (${get(50)} \u2013 ${get(75)})`,
      `p75 \u2013 p90  (${get(75)} \u2013 ${get(90)})`,
      `p90 \u2013 p95  (${get(90)} \u2013 ${get(95)})`,
      `p95 \u2013 p98  (${get(95)} \u2013 ${get(98)})`,
      `> p98  (> ${get(98)})`,
    ];
    const swatches = document.querySelectorAll('.legend-swatch');
    swatches.forEach((sw, i) => {
      if (titles[i]) sw.setAttribute('title', titles[i]);
    });
  };

  const renderCalendar = (animate = true) => {
      if (!D3) return;
      if (!chartShell.querySelector('.legend') && payload.charts.calendar?.legendHtml) {
        chartShell.insertAdjacentHTML('beforeend', payload.charts.calendar.legendHtml);
      }
      const scene = ensureScene('d3-calendar-scene');
      const cells = payload.charts.calendar.cells;
      const maxCol = D3.max(cells, d => d.col) || 0;
      const top = 28, rightPad = 4, bottomPad = 12;
      // Count how many month boundaries exist to compute total width
      const monthBoundaries = new Set();
      let prevMonth = null;
      cells.forEach(d => {
        const m = d.date.substring(0, 7);
        if (prevMonth !== null && m !== prevMonth) monthBoundaries.add(d.col);
        prevMonth = m;
      });
      const totalCols = maxCol + 1;

      // Layout: frozen weekday labels on the left + scrollable calendar
      let layout = scene.select('.calendar-layout');
      if (layout.empty()) {
        layout = scene.append('div').attr('class', 'calendar-layout');
        const wd = layout.append('div').attr('class', 'weekday-fixed');
        ['Mon','Tue','Wed','Thu','Fri','Sat','Sun'].forEach((day) => {
          wd.append('span').text(day);
        });
        layout.append('div').attr('class', 'calendar-scroll-wrap');
      }
      const wrap = layout.select('.calendar-scroll-wrap');
      // Fixed-size square cells; the scroll wrap handles overflow so the
      // user can scroll horizontally to the beginning or end of the range.
      const cell = 22, gap = 2, monthGap = 6;
      const boundaryWidth = monthBoundaries.size * monthGap;
      const columnGapWidth = Math.max(0, totalCols - 1) * gap;
      const width = totalCols * cell + columnGapWidth + boundaryWidth + rightPad;
      const height = top + 7 * cell + 6 * gap + bottomPad;
      layout.select('.weekday-fixed')
        .style('gap', `${gap}px`)
        .style('padding-top', `${top}px`);
      layout.selectAll('.weekday-fixed span')
        .style('height', `${cell}px`)
        .style('line-height', `${cell}px`);

      const svg = wrap.selectAll('svg').data([null]).join('svg')
        .attr('class', 'd3-calendar-svg')
        .attr('width', width)
        .attr('height', height)
        .attr('viewBox', `0 0 ${width} ${height}`)
        .attr('role', 'img')
        .attr('aria-label', 'Daily ridership calendar heatmap, enhanced with D3');
      _calSvg = svg;

      // Month boundary columns — extra gap inserted at each month change
      const monthBoundaryCols = [...monthBoundaries].sort((a, b) => a - b);

      // Calendar cells positioned by staircase col, with extra gap at month boundaries
      const xPos = d => {
        let x = d.col * (cell + gap);
        monthBoundaryCols.forEach(c => { if (d.col >= c) x += monthGap; });
        return x;
      };

      // Build month blocks, draw separators, and place right-aligned labels
      const monthGroups = {};
      cells.forEach(d => {
        const m = d.date.substring(0, 7);
        if (!monthGroups[m]) monthGroups[m] = { month: m, min: Infinity, max: -Infinity, label: null };
        monthGroups[m].min = Math.min(monthGroups[m].min, d.col);
        monthGroups[m].max = Math.max(monthGroups[m].max, d.col);
        if (d.monthLabel) monthGroups[m].label = d.monthLabel;
      });
      const monthList = Object.values(monthGroups)
        .filter(g => g.label)
        .sort((a, b) => a.min - b.min);

      const monthMarkers = monthList.map((g, i) => {
        const isLast = i === monthList.length - 1;
        const leftX = xPos({col: g.min});
        const rightX = xPos({col: g.max}) + cell;
        const nextLeftX = isLast ? null : xPos({col: monthList[i + 1].min});
        const lineX = isLast ? null : (rightX + nextLeftX) / 2;
        return { key: g.month, label: g.label, leftX, lineX };
      });
      monthMarkers.forEach((m, i) => {
        m.leftLineX = i === 0 ? m.leftX : monthMarkers[i - 1].lineX;
      });

      // Month separator lines (0.7px solid #333333)
      svg.selectAll('.d3-month-line').data(monthMarkers.filter(d => d.lineX != null), d => d.key).join(
        enter => enter.append('line').attr('class', 'd3-month-line'),
        update => update,
        exit => exit.remove()
      )
        .attr('x1', d => d.lineX)
        .attr('x2', d => d.lineX)
        .attr('y1', top)
        .attr('y2', height)
        .attr('stroke', '#333333')
        .attr('stroke-width', 0.7)
        .attr('vector-effect', 'non-scaling-stroke');

      // Month labels — left-aligned against the separator line on the left edge
      svg.selectAll('.d3-month-label').data(monthMarkers, d => d.key).join(
        enter => enter.append('text').attr('class', 'd3-month-label').attr('opacity', 0).text(d => d.label),
        update => update.text(d => d.label),
        exit => exit.remove()
      )
        .attr('x', d => d.leftLineX)
        .attr('y', 18)
        .attr('text-anchor', 'start')
        .transition().duration(canAnimate && animate ? 220 : 0)
        .attr('opacity', 1);

      const joined = svg.selectAll('.d3-cell').data(cells, d => d.date).join(
        enter => {
          const g = enter.append('g').attr('class', d => `d3-cell ${d.missing ? 'missing' : `band-${d.bandIndex ?? 'neutral'}`}`).attr('opacity', 0);
          g.append('rect').attr('width', cell).attr('height', cell);
          g.append('line').attr('class', 'missing-cross a').attr('x1', 2).attr('y1', 2).attr('x2', cell - 2).attr('y2', cell - 2);
          g.append('line').attr('class', 'missing-cross b').attr('x1', cell - 2).attr('y1', 2).attr('x2', 2).attr('y2', cell - 2);
          return g;
        },
        update => update,
        exit => exit.transition().duration(canAnimate && animate ? 160 : 0).attr('opacity', 0).remove()
      );

      joined
        .attr('class', d => `d3-cell ${d.missing ? 'missing' : `band-${d.bandIndex ?? 'neutral'}`}`)
        .attr('aria-label', d => d.label)
        .attr('data-tooltip', d => d.label)
        .transition().duration(canAnimate && animate ? 260 : 0)
        .attr('opacity', 1)
        .attr('transform', d => `translate(${xPos(d)},${top + d.weekday * (cell + gap)})`);
      joined.select('rect')
        .attr('width', cell)
        .attr('height', cell)
        .transition().duration(canAnimate && animate ? 260 : 0).attr('fill', colourFor);
      joined.select('.missing-cross.a')
        .attr('x2', cell - 2)
        .attr('y2', cell - 2);
      joined.select('.missing-cross.b')
        .attr('x1', cell - 2)
        .attr('y2', cell - 2);
      joined.selectAll('.missing-cross').attr('display', d => d.missing ? null : 'none');

      // Structured HTML tooltip for calendar cells
      const fmtDate = (dateStr) => {
        const d = new Date(dateStr + 'T00:00');
        return d.toLocaleDateString('en-GB', { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' });
      };
      const fmtNum = (v) => v != null ? v.toLocaleString('en-IN') : '—';
      const parseBreakdown = (str) => {
        if (!str) return [];
        const cleaned = str.replace(/^\s*—\s*/, '');
        return cleaned.split(', ').map(part => {
          const m = part.trim().match(/^(.+?)\s+([\d,]+)$/);
          return m ? { label: m[1], value: m[2] } : null;
        }).filter(Boolean);
      };
      joined
        .attr('tabindex', 0)
        .attr('role', 'img')
        .attr('aria-label', d => d.label)
        .on('mouseenter focus click', function (event, d) {
          let html;
          if (d.missing) {
            html = `<div class="tt-head"><span class="tt-date">${fmtDate(d.date)}</span></div><div class="tt-rides tt-missing-inline">No data published</div><div class="tt-spacer"></div>`;
          } else {
            const bandColor = d.bandIndex != null ? _bandColors[d.bandIndex] : 'transparent';
            const rows = parseBreakdown(d.breakdown);
            const tableHtml = rows.length
              ? `<div class="tt-table"><div class="tt-thead"><span>Payment method</span></div><div class="tt-tbody">${rows.map(r => `<div class="tt-row"><span class="tt-row-label">${r.label}</span><span class="tt-row-value">${r.value} rides</span></div>`).join('')}</div></div>`
              : '<div class="tt-spacer"></div>';
            html = `<div class="tt-head"><span class="tt-date">${fmtDate(d.date)}</span></div><div class="tt-rides"><span class="tt-color" style="background:${bandColor}"></span>${fmtNum(d.total)} rides</div>${d.bandLabel ? `<div class="tt-band">${d.bandLabel}</div>` : ''}${tableHtml}`;
          }
          showTooltipHTML(html, this);
        })
        .on('mouseleave blur', hideTooltip);

      // Legend swatch click — bind once, use shared _calSvg reference
      if (!_swatchBound) {
        _swatchBound = true;
        const swatches = document.querySelectorAll('.legend-swatch');
        swatches.forEach(sw => {
          sw.style.cursor = 'pointer';
          sw.addEventListener('click', () => {
            const bandIdx = parseInt(sw.className.match(/band-(\d+)/)?.[1] ?? -1);
            const isActive = sw.classList.toggle('active');
            swatches.forEach(s => { if (s !== sw) s.classList.remove('active'); });
            if (!isActive) {
              clearLegendFilter();
            } else {
              applyLegendFilter(bandIdx);
            }
          });
        });
      }

      syncLegend();
      const activeSwatch = document.querySelector('.legend-swatch.active');
      if (activeSwatch) {
        const bandIdx = parseInt(activeSwatch.className.match(/band-(\d+)/)?.[1] ?? -1);
        applyLegendFilter(bandIdx);
      }

      choreograph(scene.node());
    };

  const renderLine = (animate = true) => {
    if (!D3) return;
    const scene = ensureScene('d3-line-scene');
    const height = 410, left = 66, right = 10, top = 24, bottom = 66, xInset = 18;
    const allPoints = payload.charts.commuteCasual.series.flatMap(series => series.points);
    const maxValue = Math.max(1, D3.max(allPoints, p => (p.mean || 0) + (p.standardDeviation || 0)) || 1);
    const svg = scene.selectAll('svg').data([null]).join('svg')
      .attr('class', 'd3-line-svg line-chart')
      .attr('role', 'img')
      .attr('aria-label', 'Average daily ridership by weekday, enhanced with D3');
    scene.selectAll('.d3-insight').data(payload.charts.commuteCasual.insight ? [payload.charts.commuteCasual.insight] : []).join('p')
      .attr('class', 'd3-insight insight')
      .text(d => d);
    const viewport = svg.node()?.getBoundingClientRect();
    const width = height * Math.max(1, viewport?.width || 900) / Math.max(1, viewport?.height || 410);
    svg.attr('viewBox', `0 0 ${width} ${height}`);
    const plotWidth = width - left - right;
    const plotHeight = height - top - bottom;
    const x = D3.scalePoint().domain(payload.charts.commuteCasual.weekdays.map(d => d.index)).range([left + xInset, left + plotWidth - xInset]);
    const y = D3.scaleLinear().domain([0, maxValue]).nice(4).range([top + plotHeight, top]);

    svg.selectAll('.d3-grid-line').data(y.ticks(4)).join('line')
      .attr('class', 'd3-grid-line')
      .attr('x1', left).attr('x2', width - right)
      .transition().duration(canAnimate && animate ? 220 : 0)
      .attr('y1', d => y(d)).attr('y2', d => y(d));
    svg.selectAll('.d3-y-label').data(y.ticks(4)).join('text')
      .attr('class', 'd3-y-label')
      .attr('x', left - 10).attr('text-anchor', 'end')
      .transition().duration(canAnimate && animate ? 220 : 0)
      .attr('y', d => y(d) + 4)
      .text(d => compact(d));
    svg.selectAll('.d3-x-label').data(payload.charts.commuteCasual.weekdays, d => d.index).join('text')
      .attr('class', 'd3-x-label')
      .attr('x', d => x(d.index)).attr('y', height - 26).attr('text-anchor', 'middle')
      .text(d => d.short);

    const line = D3.line().defined(d => d.mean != null).x(d => x(d.index)).y(d => y(d.mean));
    const series = svg.selectAll('.d3-series').data(payload.charts.commuteCasual.series, d => d.key).join('g').attr('class', d => `d3-series ${d.key}`);
    series.selectAll('.d3-series-path').data(d => [d]).join('path')
      .attr('class', d => `d3-series-path series ${d.key}`)
      .transition().duration(canAnimate && animate ? 320 : 0)
      .attr('d', d => line(d.points));

    const pointGroups = series.selectAll('.d3-point-group').data(d => d.points.map(point => ({...point, seriesKey: d.key})), d => `${d.seriesKey}-${d.index}`).join('g')
      .attr('class', d => `d3-point-group ${d.seriesKey}`)
      .attr('aria-label', d => d.tooltip)
      .attr('data-tooltip', d => d.tooltip);
    pointGroups.transition().duration(canAnimate && animate ? 320 : 0)
      .attr('opacity', d => d.mean == null ? 0 : 1)
      .attr('transform', d => `translate(${x(d.index)},${d.mean == null ? y(0) : y(d.mean)})`);
    pointGroups.selectAll('line').data(d => d.mean != null && d.standardDeviation != null ? [d] : []).join('line')
      .attr('class', d => `d3-whisker whisker ${d.seriesKey}`)
      .attr('x1', 0).attr('x2', 0)
      .transition().duration(canAnimate && animate ? 320 : 0)
      .attr('y1', d => y(Math.min(maxValue, d.mean + d.standardDeviation)) - y(d.mean))
      .attr('y2', d => y(Math.max(0, d.mean - d.standardDeviation)) - y(d.mean));
    pointGroups.selectAll('circle.d3-point-hit').data(d => d.mean == null ? [] : [d]).join('circle')
      .attr('class', 'd3-point-hit')
      .attr('r', 12)
      .attr('fill', 'transparent');
    pointGroups.selectAll('circle.d3-point').data(d => d.mean == null ? [] : [d]).join('circle')
      .attr('class', d => `d3-point point ${d.seriesKey}`)
      .attr('r', 6);
    pointGroups.selectAll('text').data(d => d.mean == null ? [] : [d]).join('text')
      .attr('class', d => `d3-point-label point-label ${d.seriesKey}`)
      .attr('text-anchor', 'middle')
      .attr('y', -14)
      .text(d => compact(d.mean));
    bindTooltip(pointGroups);

    svg.selectAll('.d3-series-label').data(payload.charts.commuteCasual.series, d => d.key).join('text')
      .attr('class', d => `d3-series-label series-label ${d.key}`)
      .attr('x', width - 210)
      .attr('y', d => d.key === 'commute' ? 25 : 48)
      .text(d => d.label);
    choreograph(scene.node());
  };

  const renderBaseline = (animate = true) => {
    if (!D3) return;
    const scene = ensureScene('d3-baseline-scene');
    const baseline = payload.charts.weekdayBaseline;
    const points = (baseline?.points || []).map(point => ({...point, dateObject: new Date(`${point.date}T00:00:00`)}));
    scene.selectAll('.baseline-empty').data(points.length ? [] : [true]).join('p')
      .attr('class', 'baseline-empty')
      .text('No published totals are available in this range.');
    if (!points.length) return;

    const width = 1100, height = 360, left = 70, right = 34, top = 36, bottom = 52;
    const plotWidth = width - left - right;
    const plotHeight = height - top - bottom;
    const startDate = new Date(`${payload.range.start}T00:00:00`);
    const endDate = new Date(`${payload.range.end}T00:00:00`);
    const x = D3.scaleTime().domain([startDate, endDate]).range([left, left + plotWidth]);
    const values = points.flatMap(point => [point.total, point.lowerQuartile, point.upperQuartile].filter(value => value != null));
    const minValue = Math.min(...values);
    const maxValue = Math.max(...values, 1);
    const spread = Math.max(1, maxValue - minValue);
    const yMin = Math.max(0, minValue - spread * 0.08);
    const yMax = maxValue + spread * 0.08;
    const y = D3.scaleLinear().domain([yMin, yMax]).range([top + plotHeight, top]);
    const svg = scene.selectAll('svg').data([null]).join('svg')
      .attr('class', 'd3-baseline-svg baseline-chart')
      .attr('viewBox', `0 0 ${width} ${height}`)
      .attr('role', 'img')
      .attr('aria-label', 'Daily ridership compared with rolling same-weekday baseline, enhanced with D3');

    svg.selectAll('.d3-grid-line').data(y.ticks(4)).join('line')
      .attr('class', 'd3-grid-line')
      .attr('x1', left).attr('x2', width - right)
      .transition().duration(canAnimate && animate ? 220 : 0)
      .attr('y1', d => y(d)).attr('y2', d => y(d));
    svg.selectAll('.d3-y-label').data(y.ticks(4)).join('text')
      .attr('class', 'd3-y-label')
      .attr('x', left - 10).attr('text-anchor', 'end')
      .transition().duration(canAnimate && animate ? 220 : 0)
      .attr('y', d => y(d) + 4)
      .text(d => compact(d));

    const spanMs = Math.max(1, endDate.getTime() - startDate.getTime());
    const monthCount = Math.max(1, (endDate.getFullYear() - startDate.getFullYear()) * 12 + endDate.getMonth() - startDate.getMonth());
    const monthStep = Math.max(1, Math.ceil(monthCount / 6));
    let xTicks = x.ticks(D3.timeMonth.every(monthStep))
      .filter(date => date >= startDate && date <= endDate);
    if (xTicks.length < 2) {
      xTicks = Array.from({length: 4}, (_, index) => new Date(startDate.getTime() + spanMs * index / 3));
    }
    const spacedTicks = [];
    xTicks.forEach(date => {
      const xx = x(date);
      if (!spacedTicks.length || xx - x(spacedTicks[spacedTicks.length - 1]) >= 56) spacedTicks.push(date);
    });
    xTicks = spacedTicks.length >= 2 ? spacedTicks : xTicks.slice(0, 2);
    const longSpan = spanMs / 86400000 > 500;
    const monthTickFormat = D3.timeFormat('%b');
    const yearTickFormat = D3.timeFormat('%b \u2019%y');
    const dayTickFormat = D3.timeFormat('%-d %b');
    svg.selectAll('.d3-x-label').data(xTicks, d => d.getTime()).join('text')
      .attr('class', 'd3-x-label')
      .attr('text-anchor', 'middle')
      .attr('y', height - 18)
      .transition().duration(canAnimate && animate ? 220 : 0)
      .attr('x', d => x(d))
      .text((d, index) => {
        if (xTicks.length < 4 || d.getDate() !== 1) return dayTickFormat(d);
        return index === 0 || d.getMonth() === 0 || longSpan ? yearTickFormat(d) : monthTickFormat(d);
      });

    const bandPoints = points.filter(point => point.lowerQuartile != null && point.upperQuartile != null);
    const band = D3.area()
      .defined(point => point.lowerQuartile != null && point.upperQuartile != null)
      .x(point => x(point.dateObject))
      .y0(point => y(point.lowerQuartile))
      .y1(point => y(point.upperQuartile));
    svg.selectAll('.d3-baseline-band').data([bandPoints]).join('path')
      .attr('class', 'baseline-band d3-baseline-band')
      .transition().duration(canAnimate && animate ? 320 : 0)
      .attr('d', band);

    const medianLine = D3.line()
      .defined(point => point.median != null)
      .x(point => x(point.dateObject))
      .y(point => y(point.median));
    svg.selectAll('.d3-baseline-median').data([points]).join('path')
      .attr('class', 'baseline-median d3-baseline-median')
      .attr('fill', 'none')
      .transition().duration(canAnimate && animate ? 320 : 0)
      .attr('d', medianLine);

    const actualPath = points.map((point, index) => {
      const gap = index === 0 ? 0 : (point.dateObject.getTime() - points[index - 1].dateObject.getTime()) / 86400000;
      const command = index === 0 || gap > 7 ? 'M' : 'L';
      return `${command}${x(point.dateObject)},${y(point.total)}`;
    }).join(' ');
    svg.selectAll('.d3-baseline-actual').data([points]).join('path')
      .attr('class', 'baseline-actual d3-baseline-actual')
      .attr('fill', 'none')
      .transition().duration(canAnimate && animate ? 320 : 0)
      .attr('d', actualPath);

    const pointGroups = svg.selectAll('.d3-baseline-point-group').data(points, d => d.date).join('g')
      .attr('class', d => `d3-baseline-point-group ${d.status}`)
      .attr('aria-label', d => d.tooltip)
      .attr('data-tooltip', d => d.tooltip);
    pointGroups.transition().duration(canAnimate && animate ? 320 : 0)
      .attr('transform', d => `translate(${x(d.dateObject)},${y(d.total)})`);
    pointGroups.selectAll('circle.d3-point-hit').data(d => [d]).join('circle')
      .attr('class', 'd3-point-hit')
      .attr('r', 12)
      .attr('fill', 'transparent');
    pointGroups.selectAll('circle.d3-baseline-point').data(d => [d]).join('circle')
      .attr('class', d => `baseline-point d3-baseline-point ${d.status}`)
      .attr('r', d => d.status === 'high' || d.status === 'low' ? 5.5 : 4);
    bindTooltip(pointGroups);

    svg.selectAll('.baseline-svg-insight').data(baseline.insight ? [baseline.insight] : []).join('text')
      .attr('class', 'baseline-svg-insight')
      .attr('x', left)
      .attr('y', 16)
      .text(d => d);

    const legendData = [
      {key: 'band', label: 'IQR band', x: 0},
      {key: 'median', label: 'Median', x: 92},
      {key: 'within', label: 'Within', x: 166},
      {key: 'high', label: 'Above', x: 240},
      {key: 'low', label: 'Below', x: 312},
    ];
    const legend = svg.selectAll('.baseline-svg-legend').data([legendData]).join('g')
      .attr('class', 'baseline-svg-legend')
      .attr('transform', `translate(${width - right - 380},8)`);
    const legendItems = legend.selectAll('g').data(d => d, d => d.key).join('g')
      .attr('transform', d => `translate(${d.x},0)`);
    // Draw the compact legend imperatively because its mark type varies.
    legendItems.each(function(d) {
      const node = D3.select(this);
      node.selectAll('*').remove();
      if (d.key === 'band') node.append('rect').attr('class', 'legend-mark-band').attr('width', 18).attr('height', 10).attr('y', -3);
      else if (d.key === 'median') node.append('line').attr('class', 'legend-mark-median').attr('x1', 0).attr('x2', 18).attr('y1', 2).attr('y2', 2);
      else node.append('circle').attr('class', `legend-mark-dot ${d.key}`).attr('cx', 7).attr('cy', 2).attr('r', 4.5);
      node.append('text').attr('x', 24).attr('y', 6).text(d.label);
    });

    choreograph(scene.node());
  };

  const renderWaterfall = (animate = true) => {
    if (!D3) return;
    const scene = ensureScene('d3-waterfall-scene');
    const waterfall = payload.charts.componentWaterfall;
    const components = waterfall?.components || [];
    scene.selectAll('svg').remove();
    scene.selectAll('.waterfall-empty').remove();
    if (!waterfall?.available || !components.length) {
      scene.append('p').attr('class', 'waterfall-empty').text(waterfall?.message || 'Waterfall unavailable for the selected day.');
      choreograph(scene.node());
      return;
    }

    const width = 1100, height = 360, left = 70, right = 34, top = 38, bottom = 62;
    const plotWidth = width - left - right;
    const plotHeight = height - top - bottom;
    const values = [waterfall.referenceTotal, waterfall.actualTotal, ...components.flatMap(d => [d.cumulativeStart, d.cumulativeEnd])].filter(v => v != null);
    const minValue = Math.min(...values);
    const maxValue = Math.max(...values, 1);
    const spread = Math.max(1, maxValue - minValue);
    const yMin = Math.max(0, minValue - spread * 0.12);
    const yMax = maxValue + spread * 0.12;
    const y = D3.scaleLinear().domain([yMin, yMax]).range([top + plotHeight, top]);
    const slots = components.length + 2;
    const step = plotWidth / slots;
    const x = index => left + step * (index + 0.5);
    const barWidth = Math.max(44, Math.min(78, step * 0.56));
    const svg = scene.append('svg')
      .attr('class', 'd3-waterfall-svg waterfall-chart')
      .attr('viewBox', `0 0 ${width} ${height}`)
      .attr('role', 'img')
      .attr('aria-label', `Fare-media component waterfall for ${waterfall.dateDisplay}`);

    const axisTicks = D3.range(5).map(i => yMin + (yMax - yMin) * i / 4);
    svg.selectAll('.d3-grid-line').data(axisTicks).join('line')
      .attr('class', 'd3-grid-line')
      .attr('x1', left).attr('x2', width - right)
      .attr('y1', d => y(d)).attr('y2', d => y(d));
    svg.selectAll('.d3-y-label').data(axisTicks).join('text')
      .attr('class', 'd3-y-label')
      .attr('x', left - 10).attr('text-anchor', 'end')
      .attr('y', d => y(d) + 4)
      .text(d => compactAxis(d, yMax - yMin));

    const referenceX = x(0);
    const actualX = x(slots - 1);
    const referenceY = y(waterfall.referenceTotal);
    const actualY = y(waterfall.actualTotal);
    svg.append('line').attr('class', 'waterfall-total-anchor')
      .attr('x1', referenceX - barWidth / 2).attr('x2', referenceX + barWidth / 2)
      .attr('y1', referenceY).attr('y2', referenceY);
    svg.append('text').attr('class', 'waterfall-total-label')
      .attr('x', referenceX).attr('y', referenceY - 10).attr('text-anchor', 'middle')
      .text(compact(waterfall.referenceTotal));
    svg.append('text').attr('class', 'd3-x-label waterfall-x-label total')
      .attr('x', referenceX).attr('y', height - 24).attr('text-anchor', 'middle').text('Reference');

    const groups = svg.selectAll('.d3-waterfall-group').data(components, d => d.key).join('g')
      .attr('class', d => `d3-waterfall-group ${d.positive ? 'positive' : 'negative'}`)
      .attr('aria-label', d => d.tooltip)
      .attr('data-tooltip', d => d.tooltip)
      .attr('transform', (d, i) => `translate(${x(i + 1)},0)`);
    groups.selectAll('rect.waterfall-hit').data(d => [d]).join('rect')
      .attr('class', 'waterfall-hit')
      .attr('x', -barWidth / 2).attr('y', top)
      .attr('width', barWidth).attr('height', plotHeight)
      .attr('fill', 'transparent');
    groups.selectAll('rect.waterfall-bar').data(d => [d]).join('rect')
      .attr('class', d => `waterfall-bar ${d.positive ? 'positive' : 'negative'}`)
      .attr('x', -barWidth / 2)
      .attr('width', barWidth)
      .transition().duration(canAnimate && animate ? 360 : 0)
      .attr('y', d => Math.min(y(d.cumulativeStart), y(d.cumulativeEnd)))
      .attr('height', d => Math.max(2, Math.abs(y(d.cumulativeStart) - y(d.cumulativeEnd))));
    groups.selectAll('text.waterfall-delta').data(d => [d]).join('text')
      .attr('class', d => `waterfall-delta ${d.positive ? 'positive' : 'negative'}`)
      .attr('text-anchor', 'middle')
      .transition().duration(canAnimate && animate ? 360 : 0)
      .attr('y', d => {
        const topY = Math.min(y(d.cumulativeStart), y(d.cumulativeEnd));
        const barHeight = Math.max(2, Math.abs(y(d.cumulativeStart) - y(d.cumulativeEnd)));
        return d.positive ? topY - 8 : topY + barHeight + 15;
      })
      .text(d => d.deltaLabel);
    groups.selectAll('text.waterfall-x-label').data(d => [d]).join('text')
      .attr('class', 'd3-x-label waterfall-x-label')
      .attr('text-anchor', 'middle')
      .attr('y', height - 24)
      .text(d => d.label);
    bindTooltip(groups);

    svg.selectAll('.waterfall-connector').data(components.slice(0, -1), d => d.key).join('line')
      .attr('class', 'waterfall-connector')
      .attr('x1', (d, i) => x(i + 1) + barWidth / 2)
      .attr('x2', (d, i) => x(i + 2) - barWidth / 2)
      .attr('y1', d => y(d.cumulativeEnd))
      .attr('y2', d => y(d.cumulativeEnd));
    const last = components[components.length - 1];
    svg.append('line').attr('class', 'waterfall-connector')
      .attr('x1', x(components.length) + barWidth / 2)
      .attr('x2', actualX - barWidth / 2)
      .attr('y1', y(last.cumulativeEnd)).attr('y2', y(last.cumulativeEnd));
    svg.append('line').attr('class', 'waterfall-total-anchor actual')
      .attr('x1', actualX - barWidth / 2).attr('x2', actualX + barWidth / 2)
      .attr('y1', actualY).attr('y2', actualY);
    svg.append('text').attr('class', 'waterfall-total-label')
      .attr('x', actualX).attr('y', actualY - 10).attr('text-anchor', 'middle')
      .text(compact(waterfall.actualTotal));
    svg.append('text').attr('class', 'd3-x-label waterfall-x-label total')
      .attr('x', actualX).attr('y', height - 24).attr('text-anchor', 'middle').text('Selected day');

    svg.append('text')
      .attr('class', 'baseline-svg-insight')
      .attr('x', left).attr('y', 16)
      .text(waterfall.insight || '');
    const legend = svg.append('g').attr('class', 'baseline-svg-legend waterfall-svg-legend').attr('transform', `translate(${width - right - 310},8)`);
    [['positive', 'Increase', 0], ['negative', 'Decrease', 100], ['n', `n = ${waterfall.baselineN}`, 200]].forEach(([key, label, dx]) => {
      const item = legend.append('g').attr('transform', `translate(${dx},0)`);
      if (key !== 'n') item.append('circle').attr('class', `legend-mark-dot ${key === 'positive' ? 'high' : 'low'}`).attr('cx', 7).attr('cy', 2).attr('r', 4.5);
      item.append('text').attr('x', key === 'n' ? 0 : 18).attr('y', 6).text(label);
    });
    choreograph(scene.node());
  };

  const renderFareMix = (animate = true) => {
    if (!D3) return;
    const scene = ensureScene('d3-mix-scene');
    const months = payload.charts.fareMediaMix?.months || [];
    scene.selectAll('svg').remove();
    scene.selectAll('.mix-empty').remove();
    if (!months.length) {
      scene.append('p').attr('class', 'mix-empty').text('No months are available in the selected range.');
      choreograph(scene.node());
      return;
    }
    const height = 400, left = 48, right = 22, top = 42;
    const sceneWidth = scene.node()?.getBoundingClientRect().width || 1100;
    const narrow = sceneWidth < 700;
    const width = narrow ? Math.max(sceneWidth, months.length * 78) : 1100;
    const shareH = 178, gap = 34, totalH = 58, labelY = 366;
    const plotWidth = width - left - right;
    const step = plotWidth / months.length;
    const barWidth = Math.max(14, Math.min(52, step * 0.58));
    const maxTotal = Math.max(1, ...months.map(d => d.total || 0));
    const totalBase = top + shareH + gap + totalH;
    const x = index => left + step * (index + 0.5);
    scene.selectAll('.d3-insight').data(payload.charts.fareMediaMix?.insight ? [payload.charts.fareMediaMix.insight] : []).join('p')
      .attr('class', 'd3-insight insight')
      .text(d => d);
    const svg = scene.append('svg')
      .attr('class', `d3-mix-svg mix-chart${narrow ? ' mix-narrow' : ''}`)
      .attr('viewBox', `0 0 ${width} ${height}`)
      .attr('role', 'img')
      .attr('aria-label', 'Monthly fare-media share mix with observed totals, enhanced with D3');
    if (narrow) svg.style('width', `${width}px`).style('height', `${height}px`);

    [[0, '0%'], [0.5, '50%'], [1, '100%']].forEach(([value, label]) => {
      const yy = top + shareH - value * shareH;
      svg.append('line').attr('class', 'd3-grid-line').attr('x1', left).attr('x2', width - right).attr('y1', yy).attr('y2', yy);
      svg.append('text').attr('class', 'd3-y-label').attr('x', left - 9).attr('y', yy + 4).attr('text-anchor', 'end').text(label);
    });
    svg.append('line').attr('class', 'd3-grid-line').attr('x1', left).attr('x2', width - right).attr('y1', totalBase).attr('y2', totalBase);
    svg.append('text').attr('class', 'd3-y-label').attr('x', left - 9).attr('y', totalBase - totalH + 4).attr('text-anchor', 'end').text(compact(maxTotal));
    svg.append('text').attr('class', 'd3-y-label').attr('x', left - 9).attr('y', totalBase + 4).attr('text-anchor', 'end').text('0');

    const segments = [];
    const totals = [];
    const gaps = [];
    months.forEach((month, monthIndex) => {
      if (month.total > 0) {
        let cumulative = 0;
        month.components.forEach(component => {
          const share = component.share || 0;
          segments.push({month, monthIndex, component, share, y: top + cumulative * shareH, height: Math.max(component.value > 0 ? 1 : 0, share * shareH)});
          cumulative += share;
        });
        const barH = month.total / maxTotal * totalH;
        totals.push({month, monthIndex, height: barH, y: totalBase - barH});
      } else {
        gaps.push({month, monthIndex});
      }
    });

    const segmentSelection = svg.selectAll('.d3-mix-segment').data(segments, d => `${d.month.month}-${d.component.key}`).join('rect')
      .attr('class', d => `d3-mix-segment mix-segment mix-${d.component.key}`)
      .attr('x', d => x(d.monthIndex) - barWidth / 2)
      .attr('width', barWidth)
      .attr('aria-label', d => d.component.tooltip)
      .attr('data-tooltip', d => d.component.tooltip);
    segmentSelection.transition().duration(canAnimate && animate ? 340 : 0)
      .attr('y', d => d.y)
      .attr('height', d => d.height);
    bindTooltip(segmentSelection);

    const totalSelection = svg.selectAll('.d3-mix-total').data(totals, d => d.month.month).join('rect')
      .attr('class', 'd3-mix-total mix-total-bar')
      .attr('x', d => x(d.monthIndex) - barWidth / 2)
      .attr('width', barWidth)
      .attr('aria-label', d => `${d.month.label} observed total: ${d.month.totalLabel} across ${d.month.completeDays} complete days`)
      .attr('data-tooltip', d => `${d.month.label} observed total: ${d.month.totalLabel} across ${d.month.completeDays} complete days`);
    totalSelection.transition().duration(canAnimate && animate ? 340 : 0)
      .attr('y', d => d.y)
      .attr('height', d => d.height);
    bindTooltip(totalSelection);

    svg.selectAll('.mix-gap').data(gaps, d => d.month.month).join('rect')
      .attr('class', 'mix-gap')
      .attr('x', d => x(d.monthIndex) - barWidth / 2)
      .attr('y', top)
      .attr('width', barWidth)
      .attr('height', shareH);
    svg.selectAll('.mix-coverage-label').data(months.filter(d => d.total > 0), d => d.month).join('text')
      .attr('class', 'mix-coverage-label')
      .attr('x', d => x(months.indexOf(d)))
      .attr('y', totalBase + 17)
      .attr('text-anchor', 'middle')
      .text(d => `${d.completeDays}/${d.publishedDays}d`);

    svg.selectAll('.mix-x-label').data(months, d => d.month).join('text')
      .attr('class', 'd3-x-label mix-x-label')
      .attr('x', (d, i) => x(i))
      .attr('y', labelY)
      .attr('text-anchor', 'middle')
      .text(d => d.label);

    const legendItems = narrow
      ? [['smart-card','Smart',0],['ncmc','NCMC',68],['token','Token',136],['qr','QR',200],['group-ticket','Group',242]]
      : [['smart-card','Smart Cards',0],['ncmc','NCMC',100],['token','Tokens',178],['qr','QR',258],['group-ticket','Group',318]];
    const legendWidth = narrow ? 292 : 390;
    const legend = svg.append('g').attr('class', 'baseline-svg-legend mix-svg-legend').attr('transform', `translate(${Math.max(left, width - right - legendWidth)},8)`);
    legendItems.forEach(([key,label,dx]) => {
      const item = legend.append('g').attr('transform', `translate(${dx},0)`);
      item.append('rect').attr('class', `mix-key mix-${key}`).attr('x', 2).attr('y', -4).attr('width', 10).attr('height', 10).attr('rx', 2);
      item.append('text').attr('x', 18).attr('y', 6).text(label);
    });
    choreograph(scene.node());
  };

  const renderNcmcShare = (animate = true) => {
    if (!D3) return;
    const scene = ensureScene('d3-ncmc-scene');
    const weeks = payload.charts.ncmcShare?.weeks || [];
    scene.selectAll('svg').remove();
    scene.selectAll('.ncmc-empty').remove();
    if (!weeks.length) {
      scene.append('p').attr('class', 'ncmc-empty').text('No weeks are available in the selected range.');
      choreograph(scene.node());
      return;
    }
    const height = 400, left = 48, right = 22, top = 42;
    const sceneWidth = scene.node()?.getBoundingClientRect().width || 1100;
    const narrow = sceneWidth < 700;
    const width = narrow ? Math.max(sceneWidth, weeks.length * 62) : 1100;
    const shareH = 178, gap = 34, totalH = 58, labelY = 366;
    const plotWidth = width - left - right;
    const step = plotWidth / weeks.length;
    const barWidth = Math.max(10, Math.min(42, step * 0.52));
    const maxNcmc = Math.max(1, ...weeks.map(d => d.ncmc || 0));
    const totalBase = top + shareH + gap + totalH;
    const x = index => left + step * (index + 0.5);
    const shareY = share => top + shareH - share * shareH;
    scene.selectAll('.d3-insight').data(payload.charts.ncmcShare?.insight ? [payload.charts.ncmcShare.insight] : []).join('p')
      .attr('class', 'd3-insight insight')
      .text(d => d);
    const svg = scene.append('svg')
      .attr('class', `d3-ncmc-svg ncmc-chart${narrow ? ' mix-narrow' : ''}`)
      .attr('viewBox', `0 0 ${width} ${height}`)
      .attr('role', 'img')
      .attr('aria-label', 'Weekly NCMC share of Smart Cards plus NCMC, enhanced with D3');
    if (narrow) svg.style('width', `${width}px`).style('height', `${height}px`);

    [[0, '0%'], [0.5, '50%'], [1, '100%']].forEach(([value, label]) => {
      const yy = shareY(value);
      svg.append('line').attr('class', 'd3-grid-line').attr('x1', left).attr('x2', width - right).attr('y1', yy).attr('y2', yy);
      svg.append('text').attr('class', 'd3-y-label').attr('x', left - 9).attr('y', yy + 4).attr('text-anchor', 'end').text(label);
    });
    svg.append('line').attr('class', 'd3-grid-line').attr('x1', left).attr('x2', width - right).attr('y1', totalBase).attr('y2', totalBase);
    svg.append('text').attr('class', 'd3-y-label').attr('x', left - 9).attr('y', totalBase - totalH + 4).attr('text-anchor', 'end').text(compact(maxNcmc));
    svg.append('text').attr('class', 'd3-y-label').attr('x', left - 9).attr('y', totalBase + 4).attr('text-anchor', 'end').text('0');

    const usable = weeks.map((week, index) => ({...week, index})).filter(week => week.share != null);
    const segments = usable.slice(1).map((point, i) => ({a: usable[i], b: point})).filter(pair => pair.b.index === pair.a.index + 1);
    svg.selectAll('.ncmc-share-line').data(segments).join('line')
      .attr('class', 'ncmc-share-line')
      .attr('x1', d => x(d.a.index)).attr('y1', d => shareY(d.a.share))
      .attr('x2', d => x(d.b.index)).attr('y2', d => shareY(d.b.share));

    const pointSelection = svg.selectAll('.d3-ncmc-point').data(usable, d => d.week).join('circle')
      .attr('class', 'd3-ncmc-point ncmc-share-point')
      .attr('cx', d => x(d.index))
      .attr('cy', d => shareY(d.share))
      .attr('r', 5)
      .attr('aria-label', d => d.tooltip)
      .attr('data-tooltip', d => d.tooltip);
    bindTooltip(pointSelection);

    const bars = usable.map(week => ({...week, height: week.ncmc / maxNcmc * totalH}));
    const barSelection = svg.selectAll('.d3-ncmc-count').data(bars, d => d.week).join('rect')
      .attr('class', 'd3-ncmc-count ncmc-count-bar')
      .attr('x', d => x(d.index) - barWidth / 2)
      .attr('width', barWidth)
      .attr('aria-label', d => `Week of ${d.label}: NCMC ${d.ncmcLabel} across ${d.completeDays} complete days`)
      .attr('data-tooltip', d => `Week of ${d.label}: NCMC ${d.ncmcLabel} across ${d.completeDays} complete days`);
    barSelection.transition().duration(canAnimate && animate ? 340 : 0)
      .attr('y', d => totalBase - d.height)
      .attr('height', d => d.height);
    bindTooltip(barSelection);

    svg.selectAll('.mix-gap').data(weeks.filter(d => d.share == null), d => d.week).join('rect')
      .attr('class', 'mix-gap')
      .attr('x', d => x(weeks.indexOf(d)) - barWidth / 2)
      .attr('y', top).attr('width', barWidth).attr('height', shareH);
    svg.selectAll('.mix-coverage-label').data(step >= 46 ? usable : [], d => d.week).join('text')
      .attr('class', 'mix-coverage-label')
      .attr('x', d => x(d.index))
      .attr('y', totalBase + 17)
      .attr('text-anchor', 'middle')
      .text(d => `${d.completeDays}/${d.publishedDays}d`);
    const labelEvery = Math.max(1, Math.floor(weeks.length / 10));
    const labelledWeeks = weeks.filter((d, i) => i % labelEvery === 0 || i + 1 === weeks.length);
    svg.selectAll('.mix-x-label').data(labelledWeeks, d => d.week).join('text')
      .attr('class', 'd3-x-label mix-x-label')
      .attr('x', d => x(weeks.indexOf(d)))
      .attr('y', labelY)
      .attr('text-anchor', 'middle')
      .text(d => d.label);

    const legend = svg.append('g').attr('class', 'baseline-svg-legend mix-svg-legend').attr('transform', `translate(${Math.max(left, width - right - 190)},8)`);
    const shareItem = legend.append('g');
    shareItem.append('line').attr('class', 'ncmc-line-key').attr('x1', 0).attr('x2', 14).attr('y1', 2).attr('y2', 2);
    shareItem.append('text').attr('x', 20).attr('y', 6).text('Share');
    const countItem = legend.append('g').attr('transform', 'translate(92,0)');
    countItem.append('rect').attr('class', 'ncmc-count-key').attr('x', 2).attr('y', -4).attr('width', 10).attr('height', 10).attr('rx', 2);
    countItem.append('text').attr('x', 18).attr('y', 6).text('NCMC count');
    choreograph(scene.node());
  };

  const renderQrMix = (animate = true) => {
    if (!D3) return;
    const scene = ensureScene('d3-qr-scene');
    const months = payload.charts.qrMix?.months || [];
    scene.selectAll('svg').remove();
    scene.selectAll('.qr-empty').remove();
    if (!months.length) {
      scene.append('p').attr('class', 'qr-empty').text('No months are available in the selected range.');
      choreograph(scene.node());
      return;
    }
    const height = 400, left = 48, right = 22, top = 42;
    const sceneWidth = scene.node()?.getBoundingClientRect().width || 1100;
    const narrow = sceneWidth < 700;
    const width = narrow ? Math.max(sceneWidth, months.length * 78) : 1100;
    const shareH = 178, gap = 34, totalH = 58, labelY = 366;
    const plotWidth = width - left - right;
    const step = plotWidth / months.length;
    const barWidth = Math.max(14, Math.min(52, step * 0.58));
    const maxTotal = Math.max(1, ...months.map(d => d.total || 0));
    const totalBase = top + shareH + gap + totalH;
    const x = index => left + step * (index + 0.5);
    scene.selectAll('.d3-insight').data(payload.charts.qrMix?.insight ? [payload.charts.qrMix.insight] : []).join('p')
      .attr('class', 'd3-insight insight')
      .text(d => d);
    const svg = scene.append('svg')
      .attr('class', `d3-qr-svg mix-chart${narrow ? ' mix-narrow' : ''}`)
      .attr('viewBox', `0 0 ${width} ${height}`)
      .attr('role', 'img')
      .attr('aria-label', 'Monthly QR channel share mix with parent Total QR, enhanced with D3');
    if (narrow) svg.style('width', `${width}px`).style('height', `${height}px`);

    [[0, '0%'], [0.5, '50%'], [1, '100%']].forEach(([value, label]) => {
      const yy = top + shareH - value * shareH;
      svg.append('line').attr('class', 'd3-grid-line').attr('x1', left).attr('x2', width - right).attr('y1', yy).attr('y2', yy);
      svg.append('text').attr('class', 'd3-y-label').attr('x', left - 9).attr('y', yy + 4).attr('text-anchor', 'end').text(label);
    });
    svg.append('line').attr('class', 'd3-grid-line').attr('x1', left).attr('x2', width - right).attr('y1', totalBase).attr('y2', totalBase);
    svg.append('text').attr('class', 'd3-y-label').attr('x', left - 9).attr('y', totalBase - totalH + 4).attr('text-anchor', 'end').text(compact(maxTotal));
    svg.append('text').attr('class', 'd3-y-label').attr('x', left - 9).attr('y', totalBase + 4).attr('text-anchor', 'end').text('0');

    const segments = [];
    const totals = [];
    const gaps = [];
    months.forEach((month, monthIndex) => {
      if (month.total > 0) {
        let cumulative = 0;
        month.components.forEach(component => {
          const share = component.share || 0;
          segments.push({month, monthIndex, component, share, y: top + cumulative * shareH, height: Math.max(component.value > 0 ? 1 : 0, share * shareH)});
          cumulative += share;
        });
        const barH = month.total / maxTotal * totalH;
        totals.push({month, monthIndex, height: barH, y: totalBase - barH});
      } else {
        gaps.push({month, monthIndex});
      }
    });

    const segmentSelection = svg.selectAll('.d3-qr-segment').data(segments, d => `${d.month.month}-${d.component.key}`).join('rect')
      .attr('class', d => `d3-qr-segment mix-segment mix-${d.component.key}`)
      .attr('x', d => x(d.monthIndex) - barWidth / 2)
      .attr('width', barWidth)
      .attr('aria-label', d => d.component.tooltip)
      .attr('data-tooltip', d => d.component.tooltip);
    segmentSelection.transition().duration(canAnimate && animate ? 340 : 0)
      .attr('y', d => d.y)
      .attr('height', d => d.height);
    bindTooltip(segmentSelection);

    const totalSelection = svg.selectAll('.d3-qr-total').data(totals, d => d.month.month).join('rect')
      .attr('class', 'd3-qr-total mix-total-bar')
      .attr('x', d => x(d.monthIndex) - barWidth / 2)
      .attr('width', barWidth)
      .attr('aria-label', d => `${d.month.label} observed total: ${d.month.totalLabel} across ${d.month.completeDays} complete days`)
      .attr('data-tooltip', d => `${d.month.label} observed total: ${d.month.totalLabel} across ${d.month.completeDays} complete days`);
    totalSelection.transition().duration(canAnimate && animate ? 340 : 0)
      .attr('y', d => d.y)
      .attr('height', d => d.height);
    bindTooltip(totalSelection);

    svg.selectAll('.mix-gap').data(gaps, d => d.month.month).join('rect')
      .attr('class', 'mix-gap')
      .attr('x', d => x(d.monthIndex) - barWidth / 2)
      .attr('y', top)
      .attr('width', barWidth)
      .attr('height', shareH);
    svg.selectAll('.mix-coverage-label').data(months.filter(d => d.total > 0), d => d.month).join('text')
      .attr('class', 'mix-coverage-label')
      .attr('x', d => x(months.indexOf(d)))
      .attr('y', totalBase + 17)
      .attr('text-anchor', 'middle')
      .text(d => `${d.completeDays}/${d.publishedDays}d`);

    svg.selectAll('.mix-x-label').data(months, d => d.month).join('text')
      .attr('class', 'd3-x-label mix-x-label')
      .attr('x', (d, i) => x(i))
      .attr('y', labelY)
      .attr('text-anchor', 'middle')
      .text(d => d.label);

    const legendItems = narrow
      ? [['qr-namma','Namma',0],['qr-whatsapp','WhatsApp',72],['qr-paytm','Paytm',158],['qr-ondc','ONDC',218]]
      : [['qr-namma','Namma Metro',0],['qr-whatsapp','WhatsApp',108],['qr-paytm','Paytm',202],['qr-ondc','ONDC',264]];
    const legendWidth = narrow ? 270 : 330;
    const legend = svg.append('g').attr('class', 'baseline-svg-legend mix-svg-legend').attr('transform', `translate(${Math.max(left, width - right - legendWidth)},8)`);
    legendItems.forEach(([key,label,dx]) => {
      const item = legend.append('g').attr('transform', `translate(${dx},0)`);
      item.append('rect').attr('class', `mix-key mix-${key}`).attr('x', 2).attr('y', -4).attr('width', 10).attr('height', 10).attr('rx', 2);
      item.append('text').attr('x', 18).attr('y', 6).text(label);
    });
    choreograph(scene.node());
  };

  const buildDataCardHTML = (card) => {
    const endIdx = dates.indexOf(payload.range.end);
    const prevDisabled = endIdx <= 0;
    const nextDisabled = endIdx < 0 || endIdx >= dates.length - 1;
    const prevDate = !prevDisabled ? dates[endIdx - 1] : '';
    const nextDate = !nextDisabled ? dates[endIdx + 1] : '';

    if (!card || !card.fareMedia || !card.fareMedia.length) {
      return `<div class="data-card-missing">
    <p class="data-card-missing-title">No data for this date.</p>
    <p class="data-card-missing-hint">Pick another end date to see a single day's breakdown.</p>
  </div>`;
    }

    let barSegments = '';
    let legendItems = '';
    let paymentBoxes = '';
    card.fareMedia.forEach((item) => {
      const pct = item.percentage;
      const pctLabel = pct.toFixed(1);
      if (pct > 0) {
        barSegments += `<div class="dc-bar-segment dc-bar-${item.key}" style="width:${pct}%" role="img" aria-label="${item.label}: ${pctLabel}%"></div>`;
      }
      legendItems += `<div class="dc-legend-item"><span class="dc-legend-color dc-bar-${item.key}"></span>${item.label} ${pctLabel}%</div>`;
      let breakdown = '';
      if (item.breakdown && item.breakdown.length) {
        const subHtml = item.breakdown.map((sub) =>
          `<div class="dc-sub-row"><span class="dc-sub-label">${sub.label}</span><span class="dc-sub-value">${sub.valueLabel}</span></div>`
        ).join('');
        breakdown = `<div class="dc-breakdown">${subHtml}</div>`;
      }
      paymentBoxes += `<div class="dc-payment-box dc-bar-${item.key}">
  <div class="dc-payment-value">${item.valueLabel}</div>
  <div class="dc-payment-label">${item.label}</div>
  ${breakdown}
</div>`;
    });
    const missingNote = card.hasMissing ? `<p class="dc-missing-note">Some fare media values are missing for this date. The total may be incomplete.</p>` : '';

    return `<div class="data-card-inner">
    <div class="dc-top-story">
      <div class="dc-date-row">
        <button class="dc-day-nav dc-day-prev" type="button" aria-label="Previous day" data-target-date="${prevDate}" aria-disabled="${prevDisabled}" tabindex="${prevDisabled ? -1 : 0}"><span aria-hidden="true">\u2190</span></button>
        <span class="dc-date-value">${card.dateDisplay}</span>
        <button class="dc-day-nav dc-day-next" type="button" aria-label="Next day" data-target-date="${nextDate}" aria-disabled="${nextDisabled}" tabindex="${nextDisabled ? -1 : 0}"><span aria-hidden="true">\u2192</span></button>
      </div>
      <div class="dc-total-container">
        <div class="dc-total-value">${card.totalLabel}</div>
        <div class="dc-total-label">Total Rides</div>
      </div>
      <div class="dc-bar-chart" role="img" aria-label="Fare media proportion bar">
        ${barSegments}
      </div>
      <div class="dc-legend">${legendItems}</div>
      ${missingNote}
    </div>
    <div class="dc-bottom-story">
      <div class="dc-payment-grid">
        ${paymentBoxes}
      </div>
    </div>
  </div>`;
  };

  const renderDataCard = (animate = true) => {
    if (!D3) return;
    const scene = ensureScene('d3-datacard-scene');
    const card = payload.charts.dataCard;

    // Rebuild the card HTML from the payload so day navigation updates content
    const wrap = chartShell.querySelector('.data-card-wrap');
    if (wrap) {
      wrap.setAttribute('aria-label', `Ridership data card for ${card?.dateDisplay || payload.range.end}`);
      wrap.innerHTML = buildDataCardHTML(card);
    }

    if (!card || !card.fareMedia || !card.fareMedia.length) return;

    // Animate bar segments from 0 to their target width
    const segments = D3.select(chartShell).selectAll('.dc-bar-segment');
    if (canAnimate && animate && segments.size()) {
      segments.each(function() {
        const el = this;
        const targetWidth = el.style.width;
        GSAP.fromTo(el, { width: '0%' }, { width: targetWidth, duration: 0.6, ease: 'power2.out' });
      });
    }

    // Fade in payment boxes
    const boxes = D3.select(chartShell).selectAll('.dc-payment-box');
    if (canAnimate && animate && boxes.size()) {
      GSAP.fromTo(boxes.nodes(), { opacity: 0, y: 12 }, { opacity: 1, y: 0, duration: 0.4, stagger: 0.06, ease: 'power2.out' });
    }

    choreograph(scene.node());
  };

  const renderActiveChart = (animate = true) => {
    hideTooltipImmediately();
    syncChartChrome();
    if (activeChart === 'data-card') renderDataCard(animate);
    else if (activeChart === 'calendar') renderCalendar(animate);
    else if (activeChart === 'commute-casual') renderLine(animate);
    else if (activeChart === 'weekday-baseline') renderBaseline(animate);
    else if (activeChart === 'component-waterfall') renderWaterfall(animate);
    else if (activeChart === 'fare-media-mix') renderFareMix(animate);
    else if (activeChart === 'ncmc-share') renderNcmcShare(animate);
    else renderQrMix(animate);
  };

  // The static site ships a wasm compute core; when present, range changes
  // are computed locally from the CSV fetched at boot. The /api/chart
  // endpoint remains the fallback for the self-hosted server build.
  const wasmPayload = (start, end) => {
    const compute = window.__metroCompute;
    if (!compute) return null;
    try {
      return JSON.parse(compute.chartPayload(start, end));
    } catch (error) {
      console.warn('wasm chart payload failed', error);
      return null;
    }
  };
  const showComputeNote = () => {
    if (computeNote) computeNote.hidden = false;
  };
  const fetchRange = async (start, end, {animate = true} = {}) => {
    // Any scheduled range request marks the view as user-owned immediately,
    // so a late-arriving live dataset cannot reset controls mid-interaction.
    payloadDirty = true;
    // Static exports have no /api/chart: hold range changes until the wasm
    // boot settles, then compute locally — or keep the snapshot and say so.
    if (computeState === 'pending' && computeSettled) await computeSettled;
    let nextPayload = wasmPayload(start, end);
    if (!nextPayload) {
      if (computeState !== 'server') {
        syncControls(payload);
        showComputeNote();
        return;
      }
      const params = new URLSearchParams({start, end});
      const response = await fetch(`/api/chart?${params.toString()}`, {headers: {'Accept': 'application/json'}});
      if (!response.ok) throw new Error(`chart payload failed: ${response.status}`);
      nextPayload = await response.json();
    }
    syncControls(nextPayload);
    updateUrl({start: nextPayload.range.start, end: nextPayload.range.end});
    renderActiveChart(animate);
  };

  const sync = (source, delay = 280) => {
    let startIndex = Number(startSlider.value);
    let endIndex = Number(endSlider.value);
    if (startIndex > endIndex) {
      if (source === 'start') endSlider.value = startIndex;
      else startSlider.value = endIndex;
    }
    const start = dates[Number(startSlider.value)];
    const end = dates[Number(endSlider.value)];
    setPickerValue(startPicker, start);
    setPickerValue(endPicker, end);
    updateActionLinks(start, end);
    payloadDirty = true;
    clearTimeout(rangeTimer);
    rangeTimer = setTimeout(() => fetchRange(start, end).catch(console.error), delay);
  };
  const syncDateInput = (source) => {
    let start = getPickerValue(startPicker);
    let end = getPickerValue(endPicker);
    if (!start || !end) return;
    if (start > end) {
      if (source === 'start') {
        end = start;
        setPickerValue(endPicker, end);
      } else {
        start = end;
        setPickerValue(startPicker, start);
      }
    }
    startSlider.value = nearestIndex(start, 0);
    endSlider.value = nearestIndex(end, dates.length - 1);
    updateActionLinks(start, end);
    payloadDirty = true;
    clearTimeout(rangeTimer);
    rangeTimer = setTimeout(() => fetchRange(start, end).catch(console.error), 0);
  };

  startSlider.addEventListener('input', () => sync('start'));
  endSlider.addEventListener('input', () => sync('end'));
  startPicker.addEventListener('change', () => syncDateInput('start'));
  endPicker.addEventListener('change', () => syncDateInput('end'));

  document.querySelectorAll('[aria-disabled="true"]').forEach((control) => {
    control.addEventListener('click', (event) => event.preventDefault());
  });
  document.querySelector('.range-actions')?.addEventListener('click', (event) => {
    const link = event.target.closest('a[href]');
    if (!link) return;
    event.preventDefault();
    if (link.getAttribute('aria-disabled') === 'true') return;
    const start = link.dataset.start;
    const end = link.dataset.end;
    if (!start || !end) return;
    fetchRange(start, end).catch(console.error);
  });
  document.querySelector('.chart-pager')?.addEventListener('click', (event) => {
    const link = event.target.closest('a[href]');
    if (!link) return;
    event.preventDefault();
    if (link.getAttribute('aria-disabled') === 'true') return;
    const chart = link.dataset.chart;
    if (!chart || !chartRegistry[chart]) return;
    navigateToChart(chart);
  });
  chartJump?.addEventListener('change', () => {
    const chart = chartJump.value;
    if (!chart || !chartRegistry[chart]) return;
    navigateToChart(chart);
  });
  window.addEventListener('resize', () => {
    clearTimeout(chartResizeTimer);
    chartResizeTimer = setTimeout(() => {
      if (activeChart === 'commute-casual') renderLine(false);
      if (activeChart === 'fare-media-mix') renderFareMix(false);
      if (activeChart === 'ncmc-share') renderNcmcShare(false);
      if (activeChart === 'qr-channel-mix') renderQrMix(false);
    }, 120);
  });
  // Day cycling buttons on the data card — change the end date by one
  // available date. Uses event delegation because renderDataCard rebuilds
  // the card HTML on every range change.
  chartShell.addEventListener('click', (event) => {
    const btn = event.target.closest('.dc-day-nav');
    if (!btn) return;
    event.preventDefault();
    if (btn.getAttribute('aria-disabled') === 'true') return;
    const targetDate = btn.dataset.targetDate;
    if (!targetDate) return;
    fetchRange(payload.range.start, targetDate).catch(console.error);
  });
  document.addEventListener('keydown', (event) => {
    if (['INPUT', 'TEXTAREA', 'SELECT', 'BUTTON'].includes(document.activeElement?.tagName)) return;
    const idx = chartOrder.indexOf(activeChart);
    if (event.key === 'ArrowLeft' && idx > 0) {
      navigateToChart(chartOrder[idx - 1]);
    }
    if (event.key === 'ArrowRight' && idx < chartOrder.length - 1) {
      navigateToChart(chartOrder[idx + 1]);
    }
  });

  // Touch swipe for mobile chart navigation
  const chartStage = document.querySelector('.chart-stage');
  if (chartStage) {
    let touchStartX = 0;
    let touchStartY = 0;
    let touchActive = false;
    const SWIPE_THRESHOLD = 50;

    chartStage.addEventListener('touchstart', (e) => {
      if (e.touches.length !== 1) return;
      touchStartX = e.touches[0].clientX;
      touchStartY = e.touches[0].clientY;
      touchActive = true;
    }, { passive: true });

    chartStage.addEventListener('touchmove', (e) => {
      if (!touchActive || e.touches.length !== 1) return;
      // If vertical scroll dominates, cancel swipe tracking
      const dy = Math.abs(e.touches[0].clientY - touchStartY);
      const dx = Math.abs(e.touches[0].clientX - touchStartX);
      if (dy > dx && dy > 20) touchActive = false;
    }, { passive: true });

    chartStage.addEventListener('touchend', (e) => {
      if (!touchActive) return;
      touchActive = false;
      const dx = e.changedTouches[0].clientX - touchStartX;
      if (Math.abs(dx) < SWIPE_THRESHOLD) return;
      const idx = chartOrder.indexOf(activeChart);
      if (dx < 0 && idx < chartOrder.length - 1) {
        navigateToChart(chartOrder[idx + 1]);
      } else if (dx > 0 && idx > 0) {
        navigateToChart(chartOrder[idx - 1]);
      }
    }, { passive: true });
  }

  // The URL is always "/". Each page load resets to the default data-card
  // view so the latest datapoint is shown.

  // Boot the wasm compute core: fetch the canonical CSV straight from
  // GitHub (the source sends CORS `*`), load it into wasm memory, and —
  // when it is newer than the SSR snapshot embedded above — adopt a fresh
  // default payload so visitors see live data without a redeploy. If the
  // user already touched a control, only the compute path is upgraded;
  // their in-flight range is left alone. A missing /wasm/ module means the
  // self-hosted server build, where /api/chart stays the compute path.
  const bootCompute = async () => {
    let compute;
    try {
      compute = await import('/wasm/metro_wasm.js');
    } catch {
      computeState = 'server';
      return;
    }
    try {
      await compute.default();
      const sourceUrl = payload.dataset?.sourceUrl;
      if (!sourceUrl) throw new Error('dataset source URL missing');
      const response = await fetch(sourceUrl, {headers: {Accept: 'text/csv'}});
      if (!response.ok) throw new Error(`dataset fetch failed: ${response.status}`);
      const info = JSON.parse(compute.loadDataset(await response.text(), new Date().toISOString()));
      window.__metroCompute = compute;
      computeState = 'ready';
      const embedded = payload.dataset || {};
      const changed = info.revision !== embedded.revision
        || info.maxDate !== embedded.maxDate
        || info.rowCount !== embedded.rowCount;
      if (!changed) return;
      // New rows only ever append, so existing date indices stay valid;
      // grow the slider domain even if the user already moved a control.
      dates = info.availableDates || dates;
      startSlider.max = `${dates.length - 1}`;
      endSlider.max = `${dates.length - 1}`;
      if (!payloadDirty && D3) {
        const fresh = JSON.parse(compute.chartPayload('', ''));
        syncControls(fresh);
        updateUrl({start: fresh.range.start, end: fresh.range.end});
        renderActiveChart(false);
      }
    } catch (error) {
      computeState = 'failed';
      console.warn('local chart compute unavailable; using embedded snapshot', error);
      showComputeNote();
    }
  };
  computeSettled = bootCompute();

  if (D3) {
    syncControls(payload);
    syncChartChrome();
    renderActiveChart(false);
    updateUrl({start: payload.range.start, end: payload.range.end});
  }
})();
"#;
