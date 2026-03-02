<script lang="ts">
	import { onMount } from 'svelte';
	import { financeHistory } from '$lib/stores/gameState';
	import * as d3 from 'd3';

	let svgElement: SVGSVGElement;
	const width = 360;
	const height = 140;
	const margin = { top: 8, right: 8, bottom: 20, left: 50 };
	const innerW = width - margin.left - margin.right;
	const innerH = height - margin.top - margin.bottom;

	// Persistent D3 selections — created once, updated incrementally
	let gRef: d3.Selection<SVGGElement, unknown, null, undefined> | null = null;
	let revenuePathRef: d3.Selection<SVGPathElement, unknown, null, undefined> | null = null;
	let costPathRef: d3.Selection<SVGPathElement, unknown, null, undefined> | null = null;
	let xAxisRef: d3.Selection<SVGGElement, unknown, null, undefined> | null = null;
	let yAxisRef: d3.Selection<SVGGElement, unknown, null, undefined> | null = null;
	let gridRef: d3.Selection<SVGGElement, unknown, null, undefined> | null = null;
	let lastDataLen = 0;

	function initChart() {
		if (!svgElement) return;
		const svg = d3.select(svgElement);
		svg.selectAll('*').remove();

		gRef = svg.append('g').attr('transform', `translate(${margin.left},${margin.top})`);

		gridRef = gRef.append('g').attr('class', 'grid');
		xAxisRef = gRef.append('g').attr('class', 'x-axis').attr('transform', `translate(0,${innerH})`);
		yAxisRef = gRef.append('g').attr('class', 'y-axis');

		revenuePathRef = gRef.append('path')
			.attr('fill', 'none')
			.attr('stroke', '#10b981')
			.attr('stroke-width', 1.5);

		costPathRef = gRef.append('path')
			.attr('fill', 'none')
			.attr('stroke', '#ef4444')
			.attr('stroke-width', 1.5);

		// Legend
		const legend = svg.append('g').attr('transform', `translate(${margin.left + 4}, ${margin.top})`);
		legend.append('rect').attr('width', 8).attr('height', 2).attr('fill', '#10b981').attr('y', 1);
		legend.append('text').attr('x', 12).attr('y', 4).attr('fill', '#9ca3af').attr('font-size', '9px').text('Revenue');
		legend.append('rect').attr('x', 65).attr('width', 8).attr('height', 2).attr('fill', '#ef4444').attr('y', 1);
		legend.append('text').attr('x', 77).attr('y', 4).attr('fill', '#9ca3af').attr('font-size', '9px').text('Cost');
	}

	function draw(data: typeof $financeHistory) {
		if (!svgElement || data.length < 2) return;

		if (!gRef) initChart();
		if (!gRef || !revenuePathRef || !costPathRef || !xAxisRef || !yAxisRef || !gridRef) return;

		// Skip redraw if data length hasn't changed (same snapshot count)
		if (data.length === lastDataLen) return;
		lastDataLen = data.length;

		const xScale = d3.scaleLinear()
			.domain(d3.extent(data, (d) => d.tick) as [number, number])
			.range([0, innerW]);

		const allValues = data.flatMap((d) => [d.revenue, d.cost]);
		const yScale = d3.scaleLinear()
			.domain([Math.min(0, d3.min(allValues) ?? 0), d3.max(allValues) ?? 1])
			.range([innerH, 0])
			.nice();

		// Update grid
		gridRef.call(
			d3.axisLeft(yScale).ticks(4).tickSize(-innerW).tickFormat(() => '') as any
		);
		gridRef.selectAll('line').attr('stroke', 'rgba(55, 65, 81, 0.3)');
		gridRef.selectAll('.domain').remove();

		// Update axes
		xAxisRef.call(d3.axisBottom(xScale).ticks(5).tickFormat((d) => `${d}`) as any);
		xAxisRef.selectAll('text').attr('fill', '#6b7280').attr('font-size', '9px');
		xAxisRef.selectAll('.domain').attr('stroke', '#374151');
		xAxisRef.selectAll('.tick line').attr('stroke', '#374151');

		yAxisRef.call(
			d3.axisLeft(yScale).ticks(4).tickFormat((d) => {
				const n = d as number;
				if (Math.abs(n) >= 1_000_000) return `${(n / 1_000_000).toFixed(0)}M`;
				if (Math.abs(n) >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
				return `${n}`;
			}) as any
		);
		yAxisRef.selectAll('text').attr('fill', '#6b7280').attr('font-size', '9px');
		yAxisRef.selectAll('.domain').attr('stroke', '#374151');
		yAxisRef.selectAll('.tick line').attr('stroke', '#374151');

		// Update paths (no DOM creation — just update 'd' attribute)
		const revenueLine = d3.line<(typeof data)[0]>()
			.x((d) => xScale(d.tick))
			.y((d) => yScale(d.revenue))
			.curve(d3.curveMonotoneX);

		const costLine = d3.line<(typeof data)[0]>()
			.x((d) => xScale(d.tick))
			.y((d) => yScale(d.cost))
			.curve(d3.curveMonotoneX);

		revenuePathRef.datum(data).attr('d', revenueLine);
		costPathRef.datum(data).attr('d', costLine);
	}

	$effect(() => {
		draw($financeHistory);
	});

	onMount(() => {
		draw($financeHistory);
	});
</script>

<svg bind:this={svgElement} {width} {height} class="chart"></svg>

{#if $financeHistory.length < 2}
	<div class="empty">Collecting data...</div>
{/if}

<style>
	.chart {
		width: 100%;
		max-width: 100%;
		height: auto;
	}

	.empty {
		color: var(--text-dim);
		font-size: 11px;
		text-align: center;
		padding: 8px;
	}
</style>
