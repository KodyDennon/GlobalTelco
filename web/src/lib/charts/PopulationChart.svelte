<script lang="ts">
	import { onMount } from 'svelte';
	import { cities, formatPopulation } from '$lib/stores/gameState';
	import * as d3 from 'd3';

	let svgElement: SVGSVGElement;
	const width = 360;
	const height = 160;
	const margin = { top: 8, right: 8, bottom: 20, left: 50 };
	const innerW = width - margin.left - margin.right;
	const innerH = height - margin.top - margin.bottom;

	let gRef: d3.Selection<SVGGElement, unknown, null, undefined> | null = null;
	let xAxisRef: d3.Selection<SVGGElement, unknown, null, undefined> | null = null;
	let yAxisRef: d3.Selection<SVGGElement, unknown, null, undefined> | null = null;
	let gridRef: d3.Selection<SVGGElement, unknown, null, undefined> | null = null;

	function initChart() {
		if (!svgElement) return;
		const svg = d3.select(svgElement);
		svg.selectAll('*').remove();

		gRef = svg.append('g').attr('transform', `translate(${margin.left},${margin.top})`);
		gridRef = gRef.append('g').attr('class', 'grid');
		yAxisRef = gRef.append('g').attr('class', 'y-axis');
		xAxisRef = gRef.append('g').attr('class', 'x-axis').attr('transform', `translate(0,${innerH})`);
	}

	function draw(citiesData: typeof $cities) {
		if (!svgElement || citiesData.length === 0) return;

		if (!gRef) initChart();
		if (!gRef || !xAxisRef || !yAxisRef || !gridRef) return;

		// Sort cities by population descending, take top 10
		const sorted = [...citiesData].sort((a, b) => b.population - a.population).slice(0, 10);

		const xScale = d3
			.scaleBand()
			.domain(sorted.map((c) => c.name))
			.range([0, innerW])
			.padding(0.3);

		const maxPop = d3.max(sorted, (d) => d.population) ?? 1;
		const yScale = d3
			.scaleLinear()
			.domain([0, maxPop])
			.range([innerH, 0])
			.nice();

		// Update grid
		gridRef.call(
			d3.axisLeft(yScale).ticks(4).tickSize(-innerW).tickFormat(() => '') as any
		);
		gridRef.selectAll('line').attr('stroke', 'rgba(55, 65, 81, 0.3)');
		gridRef.selectAll('.domain').remove();

		// Update Y axis
		yAxisRef.call(
			d3.axisLeft(yScale).ticks(4).tickFormat((d) => {
				const n = d as number;
				if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(0)}M`;
				if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
				return `${n}`;
			}) as any
		);
		yAxisRef.selectAll('text').attr('fill', '#6b7280').attr('font-size', '9px');
		yAxisRef.selectAll('.domain').attr('stroke', '#374151');
		yAxisRef.selectAll('.tick line').attr('stroke', '#374151');

		// D3 data join for bars — incremental update
		type CityData = (typeof sorted)[0];
		const bars = gRef.selectAll<SVGRectElement, CityData>('.bar')
			.data(sorted, (d) => d.name);

		bars.enter()
			.append('rect')
			.attr('class', 'bar')
			.attr('fill', '#3b82f6')
			.attr('rx', 2)
			.merge(bars)
			.attr('x', (d) => xScale(d.name) ?? 0)
			.attr('y', (d) => yScale(d.population))
			.attr('width', xScale.bandwidth())
			.attr('height', (d) => innerH - yScale(d.population));

		bars.exit().remove();

		// Update X axis
		xAxisRef.call(d3.axisBottom(xScale).tickSize(0) as any);
		xAxisRef.selectAll('text')
			.attr('fill', '#6b7280')
			.attr('font-size', '8px')
			.attr('transform', 'rotate(-35)')
			.attr('text-anchor', 'end');
		xAxisRef.selectAll('.domain').attr('stroke', '#374151');

	}

	$effect(() => {
		draw($cities);
	});

	onMount(() => {
		draw($cities);
	});
</script>

<svg bind:this={svgElement} {width} {height} class="chart"></svg>

{#if $cities.length === 0}
	<div class="empty">No city data available</div>
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
