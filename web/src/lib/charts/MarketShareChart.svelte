<script lang="ts">
	import { onMount } from 'svelte';
	import { allCorporations } from '$lib/stores/gameState';
	import * as d3 from 'd3';
	import type { CorpSummary } from '$lib/wasm/types';

	let svgElement: SVGSVGElement;
	const width = 360;
	const height = 160;
	const margin = { top: 8, right: 8, bottom: 8, left: 8 };

	const COLORS = ['#10b981', '#3b82f6', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#14b8a6', '#f97316'];

	function draw(corps: CorpSummary[]) {
		if (!svgElement || corps.length === 0) return;

		const svg = d3.select(svgElement);
		svg.selectAll('*').remove();

		const innerW = width - margin.left - margin.right;
		const innerH = height - margin.top - margin.bottom;

		const g = svg
			.append('g')
			.attr('transform', `translate(${margin.left},${margin.top})`);

		// Sort by revenue descending
		const sorted = [...corps].sort((a, b) => b.revenue - a.revenue);
		const totalRevenue = sorted.reduce((s, c) => s + Math.max(0, c.revenue), 0);
		if (totalRevenue === 0) return;

		const barHeight = Math.min(20, (innerH - (sorted.length - 1) * 2) / sorted.length);

		sorted.forEach((corp, i) => {
			const share = Math.max(0, corp.revenue) / totalRevenue;
			const barW = share * innerW * 0.7;

			g.append('rect')
				.attr('x', 0)
				.attr('y', i * (barHeight + 2))
				.attr('width', barW)
				.attr('height', barHeight)
				.attr('fill', COLORS[i % COLORS.length])
				.attr('opacity', corp.is_player ? 1 : 0.6)
				.attr('rx', 2);

			g.append('text')
				.attr('x', barW + 6)
				.attr('y', i * (barHeight + 2) + barHeight / 2 + 1)
				.attr('fill', '#d1d5db')
				.attr('font-size', '10px')
				.attr('dominant-baseline', 'middle')
				.text(`${corp.name} (${(share * 100).toFixed(0)}%)`);
		});
	}

	$effect(() => {
		draw($allCorporations);
	});

	onMount(() => {
		draw($allCorporations);
	});
</script>

<svg bind:this={svgElement} {width} {height} class="chart"></svg>

<style>
	.chart {
		width: 100%;
		max-width: 100%;
		height: auto;
	}
</style>
