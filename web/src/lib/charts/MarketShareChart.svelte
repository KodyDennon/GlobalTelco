<script lang="ts">
	import { onMount } from 'svelte';
	import { allCorporations } from '$lib/stores/gameState';
	import * as d3 from 'd3';
	import type { CorpSummary } from '$lib/wasm/types';

	let svgElement: SVGSVGElement;
	const width = 360;
	const height = 160;
	const margin = { top: 8, right: 8, bottom: 8, left: 8 };
	const innerW = width - margin.left - margin.right;
	const innerH = height - margin.top - margin.bottom;

	const COLORS = ['#10b981', '#3b82f6', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#14b8a6', '#f97316'];

	let gRef: d3.Selection<SVGGElement, unknown, null, undefined> | null = null;

	function ensureGroup() {
		if (!svgElement) return false;
		if (!gRef) {
			const svg = d3.select(svgElement);
			svg.selectAll('*').remove();
			gRef = svg.append('g').attr('transform', `translate(${margin.left},${margin.top})`);
		}
		return true;
	}

	function draw(corps: CorpSummary[]) {
		if (!ensureGroup() || corps.length === 0 || !gRef) return;

		// Sort by revenue descending
		const sorted = [...corps].sort((a, b) => b.revenue - a.revenue);
		const totalRevenue = sorted.reduce((s, c) => s + Math.max(0, c.revenue), 0);
		if (totalRevenue === 0) return;

		const barHeight = Math.min(20, (innerH - (sorted.length - 1) * 2) / sorted.length);

		// D3 data join — incremental update instead of nuke-and-rebuild
		const bars = gRef.selectAll<SVGRectElement, CorpSummary>('.bar')
			.data(sorted, (d) => String(d.id));

		bars.enter()
			.append('rect')
			.attr('class', 'bar')
			.attr('rx', 2)
			.merge(bars)
			.attr('x', 0)
			.attr('y', (_, i) => i * (barHeight + 2))
			.attr('width', (d) => (Math.max(0, d.revenue) / totalRevenue) * innerW * 0.7)
			.attr('height', barHeight)
			.attr('fill', (_, i) => COLORS[i % COLORS.length])
			.attr('opacity', (d) => d.is_player ? 1 : 0.6);

		bars.exit().remove();

		const labels = gRef.selectAll<SVGTextElement, CorpSummary>('.label')
			.data(sorted, (d) => String(d.id));

		labels.enter()
			.append('text')
			.attr('class', 'label')
			.attr('fill', '#d1d5db')
			.attr('font-size', '10px')
			.attr('dominant-baseline', 'middle')
			.merge(labels)
			.attr('x', (d) => (Math.max(0, d.revenue) / totalRevenue) * innerW * 0.7 + 6)
			.attr('y', (_, i) => i * (barHeight + 2) + barHeight / 2 + 1)
			.text((d) => `${d.name} (${((Math.max(0, d.revenue) / totalRevenue) * 100).toFixed(0)}%)`);

		labels.exit().remove();
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
