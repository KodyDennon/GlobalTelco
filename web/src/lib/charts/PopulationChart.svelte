<script lang="ts">
	import { onMount } from 'svelte';
	import { cities, formatPopulation } from '$lib/stores/gameState';
	import * as d3 from 'd3';

	let svgElement: SVGSVGElement;
	const width = 360;
	const height = 160;
	const margin = { top: 8, right: 8, bottom: 20, left: 50 };

	function draw(citiesData: typeof $cities) {
		if (!svgElement || citiesData.length === 0) return;

		const svg = d3.select(svgElement);
		svg.selectAll('*').remove();

		const innerW = width - margin.left - margin.right;
		const innerH = height - margin.top - margin.bottom;

		const g = svg
			.append('g')
			.attr('transform', `translate(${margin.left},${margin.top})`);

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

		// Grid lines
		g.append('g')
			.attr('class', 'grid')
			.call(
				d3
					.axisLeft(yScale)
					.ticks(4)
					.tickSize(-innerW)
					.tickFormat(() => '')
			)
			.selectAll('line')
			.attr('stroke', 'rgba(55, 65, 81, 0.3)');
		g.selectAll('.grid .domain').remove();

		// Y axis
		g.append('g')
			.call(
				d3
					.axisLeft(yScale)
					.ticks(4)
					.tickFormat((d) => {
						const n = d as number;
						if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(0)}M`;
						if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
						return `${n}`;
					})
			)
			.selectAll('text')
			.attr('fill', '#6b7280')
			.attr('font-size', '9px');

		g.selectAll('.domain').attr('stroke', '#374151');
		g.selectAll('.tick line').attr('stroke', '#374151');

		// Bars
		g.selectAll('.bar')
			.data(sorted)
			.enter()
			.append('rect')
			.attr('class', 'bar')
			.attr('x', (d) => xScale(d.name) ?? 0)
			.attr('y', (d) => yScale(d.population))
			.attr('width', xScale.bandwidth())
			.attr('height', (d) => innerH - yScale(d.population))
			.attr('fill', '#3b82f6')
			.attr('rx', 2);

		// X axis labels (rotated for readability)
		g.append('g')
			.attr('transform', `translate(0,${innerH})`)
			.call(d3.axisBottom(xScale).tickSize(0))
			.selectAll('text')
			.attr('fill', '#6b7280')
			.attr('font-size', '8px')
			.attr('transform', 'rotate(-35)')
			.attr('text-anchor', 'end');

		g.selectAll('.domain').attr('stroke', '#374151');
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
