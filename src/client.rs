pub(crate) const WRAPPER_OPEN: &str = "<script>";
pub(crate) const WRAPPER_CLOSE: &str = "</script>";
pub(crate) const CLIENT_SCRIPT: &str = r#"(() => {
  const D3 = window.d3;
  const GSAP = window.gsap;
  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const canAnimate = !reducedMotion;
  // Chart titles are rendered server-side from ChartDefinition. The
  // client only needs the active-chart switch for arrow-key navigation;
  // labels are read from the live DOM, not a duplicated registry.
  const startPicker = document.querySelector('[data-date-type="start"]');
  const endPicker = document.querySelector('[data-date-type="end"]');
  const startSlider = document.querySelector('#range-start');
  const endSlider = document.querySelector('#range-end');
  const tooltip = document.querySelector('#chart-tooltip');
  const chartShell = document.querySelector('.chart-shell');
  const chartTitle = document.querySelector('#chart-title');
  const chartDeck = document.querySelector('.chart-deck');
  const chartEyebrow = document.querySelector('.stage-topline .eyebrow');
  const priorButton = document.querySelector('.arrow-button.prev');
  const nextButton = document.querySelector('.arrow-button.next');
  const resetLink = document.querySelector('.range-actions .text-button[href*="start=2026-01-01"]');
  const allDataLink = document.querySelector('.range-actions .text-button:not([href*="start=2026-01-01"])');
  const payloadNode = document.querySelector('#chart-data');
  if (!startPicker || !endPicker || !startSlider || !endSlider || !payloadNode || !chartShell) return;

  let payload = JSON.parse(payloadNode.textContent || '{}');
  let dates = payload.dataset?.availableDates || (document.querySelector('.range-controls')?.dataset.availableDates || '').split(',').filter(Boolean);
  if (!dates.length) return;
  const initialParams = new URLSearchParams(window.location.search);
  let activeChart = initialParams.get('chart') === 'commute-casual' ? 'commute-casual'
    : initialParams.get('chart') === 'calendar' ? 'calendar'
    : 'data-card';
  let rangeTimer;

  const chartRegistry = { 'data-card': true, 'calendar': true, 'commute-casual': true };
  const chartOrder = ['data-card', 'calendar', 'commute-casual'];

  const navigateToChart = (chart) => {
    if (chart === activeChart || !chartRegistry[chart]) return;
    activeChart = chart;
    updateUrl({start: payload.range.start, end: payload.range.end, chart: activeChart});
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
  const colourFor = (cell) => cell.missing ? 'var(--surface)' : cell.bandIndex == null ? 'var(--grid)' : `var(--band-${cell.bandIndex})`;
  const setDisabled = (el, disabled) => {
    if (!el) return;
    el.setAttribute('aria-disabled', String(disabled));
    el.setAttribute('tabindex', disabled ? '-1' : '0');
  };
  const currentParams = () => new URLSearchParams(window.location.search);
  const updateUrl = ({start, end, chart = activeChart, replace = false}) => {
    const params = currentParams();
    params.set('start', start);
    params.set('end', end);
    params.set('chart', chart);
    const next = `${window.location.pathname}?${params.toString()}`;
    (replace ? history.replaceState : history.pushState).call(history, {start, end, chart}, '', next);
  };
  const showTooltip = (target) => {
    const text = target.dataset.tooltip || target.getAttribute('aria-label');
    if (!tooltip || !text) return;
    const bounds = target.getBoundingClientRect();
    tooltip.textContent = text;
    tooltip.hidden = false;
    tooltip.style.left = `${Math.min(window.innerWidth - 16, Math.max(16, bounds.left + bounds.width / 2))}px`;
    tooltip.style.top = `${Math.max(12, bounds.top - 10)}px`;
  };
  const hideTooltip = () => { if (tooltip) tooltip.hidden = true; };
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
    const minDate = payload.dataset.minDate;
    const maxDate = payload.dataset.maxDate;
    const resetStart = resetLink ? new URL(resetLink.href).searchParams.get('start') : null;
    const resetEnd = resetLink ? new URL(resetLink.href).searchParams.get('end') : null;
    if (resetLink) setDisabled(resetLink, resetStart && resetEnd && start === resetStart && end === resetEnd);
    if (allDataLink) setDisabled(allDataLink, start === minDate && end === maxDate);
  };

  const syncControls = (nextPayload) => {
    payload = nextPayload;
    dates = payload.dataset.availableDates;
    startSlider.max = `${dates.length - 1}`;
    endSlider.max = `${dates.length - 1}`;
    startSlider.value = payload.range.startIndex;
    endSlider.value = payload.range.endIndex;
    setPickerValue(startPicker, payload.range.start);
    setPickerValue(endPicker, payload.range.end);
    updateActionLinks(payload.range.start, payload.range.end);
  };

  const syncChartChrome = () => {
    // Title, deck, and eyebrow are rendered server-side from
    // ChartDefinition; the client only owns disabled-state on the
    // prev/next buttons (and would re-render them on a hot
    // swap if we ever introduce one).
    setDisabled(priorButton, activeChart === 'data-card');
    setDisabled(nextButton, activeChart === 'commute-casual');
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

  const renderCalendar = (animate = true) => {
      if (!D3) return;
      const scene = ensureScene('d3-calendar-scene');
      const cells = payload.charts.calendar.cells;
      const maxWeek = D3.max(cells, d => d.week) || 0;
      const maxGap = D3.max(cells, d => d.monthGap) || 0;
      const cell = 24, gap = 2, left = 44, top = 28;
      const totalCols = maxWeek + maxGap + 1;
      const width = left + totalCols * (cell + gap) + 24;
      const height = 220;

      // Ensure scroll-wrap container around the SVG so months scroll
      // horizontally while the 7-row height stays fixed.
      let wrap = scene.select('.calendar-scroll-wrap');
      if (wrap.empty()) {
        wrap = scene.append('div').attr('class', 'calendar-scroll-wrap');
      }
      const svg = wrap.selectAll('svg').data([null]).join('svg')
        .attr('class', 'd3-calendar-svg')
        .attr('viewBox', `0 0 ${width} ${height}`)
        .attr('role', 'img')
        .attr('aria-label', 'Daily ridership calendar heatmap, enhanced with D3');

      // Thin monospace weekday labels on the y-axis
      svg.selectAll('.weekday-label').data(['Mon','Tue','Wed','Thu','Fri','Sat','Sun']).join('text')
        .attr('class', 'weekday-label')
        .attr('x', 0)
        .attr('y', (_, i) => top + i * (cell + gap) + 12)
        .text(d => d);

      // Month labels positioned above the first week of each month,
      // accounting for month-gap columns so they align with their cells.
      const xPos = d => left + (d.week + d.monthGap) * (cell + gap);
      svg.selectAll('.d3-month-label').data(cells.filter(d => d.monthLabel), d => d.date).join(
        enter => enter.append('text').attr('class', 'd3-month-label').attr('opacity', 0).text(d => d.monthLabel),
        update => update.text(d => d.monthLabel),
        exit => exit.remove()
      )
        .attr('x', xPos)
        .attr('y', 13)
        .transition().duration(canAnimate && animate ? 220 : 0)
        .attr('opacity', 1);

      // Calendar cells — x transform includes monthGap so month blocks
      // are visually separated by a column of whitespace.
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
      joined.select('rect').transition().duration(canAnimate && animate ? 260 : 0).attr('fill', colourFor);
      joined.selectAll('.missing-cross').attr('display', d => d.missing ? null : 'none');
      bindTooltip(joined);

      // Legend is rendered server-side by `legend_markup` inside the SSR
      // calendar-wrap. The D3 pass does not duplicate it.
      choreograph(scene.node());
    };

  const renderLine = (animate = true) => {
    if (!D3) return;
    const scene = ensureScene('d3-line-scene');
    const width = 900, height = 410, left = 66, right = 28, top = 24, bottom = 66;
    const plotWidth = width - left - right;
    const plotHeight = height - top - bottom;
    const allPoints = payload.charts.commuteCasual.series.flatMap(series => series.points);
    const maxValue = Math.max(1, D3.max(allPoints, p => (p.mean || 0) + (p.standardDeviation || 0)) || 1);
    const x = D3.scalePoint().domain(payload.charts.commuteCasual.weekdays.map(d => d.index)).range([left, left + plotWidth]);
    const y = D3.scaleLinear().domain([0, maxValue]).nice(4).range([top + plotHeight, top]);
    const svg = scene.selectAll('svg').data([null]).join('svg')
      .attr('class', 'd3-line-svg line-chart')
      .attr('viewBox', `0 0 ${width} ${height}`)
      .attr('role', 'img')
      .attr('aria-label', 'Average daily ridership by weekday, enhanced with D3');

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
      .attr('fill', 'none')
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
    scene.selectAll('.d3-insight').data(payload.charts.commuteCasual.insight ? [payload.charts.commuteCasual.insight] : []).join('p')
      .attr('class', 'd3-insight insight')
      .text(d => d);
    choreograph(scene.node());
  };

  const renderDataCard = (animate = true) => {
    if (!D3) return;
    const scene = ensureScene('d3-datacard-scene');
    const card = payload.charts.dataCard;
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
    syncChartChrome();
    if (activeChart === 'data-card') renderDataCard(animate);
    else if (activeChart === 'calendar') renderCalendar(animate);
    else renderLine(animate);
  };

  const fetchRange = async (start, end, {replace = false, animate = true} = {}) => {
    const params = new URLSearchParams({start, end});
    const response = await fetch(`/api/chart?${params.toString()}`, {headers: {'Accept': 'application/json'}});
    if (!response.ok) throw new Error(`chart payload failed: ${response.status}`);
    const nextPayload = await response.json();
    syncControls(nextPayload);
    updateUrl({start: nextPayload.range.start, end: nextPayload.range.end, replace});
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
    clearTimeout(rangeTimer);
    rangeTimer = setTimeout(() => fetchRange(start, end, {replace: true}).catch(console.error), delay);
  };
  const syncDateInput = (source) => {
    const value = source === 'start' ? getPickerValue(startPicker) : getPickerValue(endPicker);
    if (source === 'start') startSlider.value = nearestIndex(value, 0);
    else endSlider.value = nearestIndex(value, dates.length - 1);
    sync(source, 0);
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
    const params = new URLSearchParams(new URL(link.href).search);
    fetchRange(params.get('start'), params.get('end'), {replace: false}).catch(console.error);
  });
  document.querySelector('.navigation-row')?.addEventListener('click', (event) => {
    const link = event.target.closest('a[href]');
    if (!link) return;
    event.preventDefault();
    if (link.getAttribute('aria-disabled') === 'true') return;
    const params = new URLSearchParams(new URL(link.href).search);
    const chart = params.get('chart') === 'commute-casual' ? 'commute-casual'
      : params.get('chart') === 'calendar' ? 'calendar'
      : 'data-card';
    navigateToChart(chart);
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

  window.addEventListener('popstate', () => {
    const params = currentParams();
    activeChart = chartRegistry[params.get('chart')] ? params.get('chart') : 'data-card';
    renderActiveChart(false);
  });

  if (D3) {
    syncControls(payload);
    renderActiveChart(false);
  }
})();
"#;
