<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { playerCorp } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import * as d3 from 'd3';

	let svgElement: SVGSVGElement;
	const width = 360;
	const height = 240;

	interface TopoNode extends d3.SimulationNodeDatum {
		id: number;
		node_type: string;
		health: number;
		utilization: number;
		under_construction: boolean;
	}

	interface TopoLink extends d3.SimulationLinkDatum<TopoNode> {
		id: number;
		edge_type: string;
		utilization: number;
	}

	let simulation: d3.Simulation<TopoNode, TopoLink> | null = null;

	function draw(corpId: number) {
		if (!svgElement || !bridge.isInitialized()) return;

		const infra = bridge.getInfrastructureList(corpId);

		const nodes: TopoNode[] = infra.nodes.map((n) => ({
			id: n.id,
			node_type: n.node_type,
			health: n.health,
			utilization: n.utilization,
			under_construction: n.under_construction
		}));

		const nodeIds = new Set(nodes.map((n) => n.id));
		const links: TopoLink[] = infra.edges
			.filter((e) => nodeIds.has(e.source) && nodeIds.has(e.target))
			.map((e) => ({
				id: e.id,
				source: e.source,
				target: e.target,
				edge_type: e.edge_type,
				utilization: e.utilization
			}));

		const svg = d3.select(svgElement);
		svg.selectAll('*').remove();

		if (nodes.length === 0) return;

		const nodeColorMap: Record<string, string> = {
			CentralOffice: '#3b82f6',
			ExchangePoint: '#8b5cf6',
			CellTower: '#10b981',
			DataCenter: '#f59e0b',
			SatelliteGround: '#ec4899',
			SubmarineLanding: '#14b8a6',
			WirelessRelay: '#6366f1'
		};

		const nodeSizeMap: Record<string, number> = {
			CentralOffice: 8,
			ExchangePoint: 10,
			CellTower: 5,
			DataCenter: 12,
			SatelliteGround: 9,
			SubmarineLanding: 11,
			WirelessRelay: 4
		};

		// Stop any previous simulation
		if (simulation) simulation.stop();

		simulation = d3
			.forceSimulation<TopoNode>(nodes)
			.force(
				'link',
				d3
					.forceLink<TopoNode, TopoLink>(links)
					.id((d) => d.id)
					.distance(40)
			)
			.force('charge', d3.forceManyBody().strength(-80))
			.force('center', d3.forceCenter(width / 2, height / 2))
			.force('collision', d3.forceCollide().radius(12));

		// Links
		const link = svg
			.append('g')
			.selectAll('line')
			.data(links)
			.enter()
			.append('line')
			.attr('stroke', '#374151')
			.attr('stroke-width', 1.5)
			.attr('stroke-opacity', 0.6);

		// Nodes
		const node = svg
			.append('g')
			.selectAll('circle')
			.data(nodes)
			.enter()
			.append('circle')
			.attr('r', (d) => nodeSizeMap[d.node_type] ?? 6)
			.attr('fill', (d) => {
				if (d.under_construction) return '#6b7280';
				return nodeColorMap[d.node_type] ?? '#9ca3af';
			})
			.attr('stroke', (d) => {
				if (d.health < 0.5) return '#ef4444';
				return 'none';
			})
			.attr('stroke-width', 2)
			.attr('opacity', (d) => (d.under_construction ? 0.5 : 1));

		// Labels
		const label = svg
			.append('g')
			.selectAll('text')
			.data(nodes)
			.enter()
			.append('text')
			.text((d) => d.node_type.replace(/([A-Z])/g, ' $1').trim().split(' ')[0])
			.attr('font-size', '8px')
			.attr('fill', '#6b7280')
			.attr('text-anchor', 'middle')
			.attr('dy', (d) => (nodeSizeMap[d.node_type] ?? 6) + 10);

		simulation.on('tick', () => {
			link
				.attr('x1', (d) => ((d.source as TopoNode).x ?? 0))
				.attr('y1', (d) => ((d.source as TopoNode).y ?? 0))
				.attr('x2', (d) => ((d.target as TopoNode).x ?? 0))
				.attr('y2', (d) => ((d.target as TopoNode).y ?? 0));

			node
				.attr('cx', (d) => (d.x ?? 0))
				.attr('cy', (d) => (d.y ?? 0));

			label
				.attr('x', (d) => (d.x ?? 0))
				.attr('y', (d) => (d.y ?? 0));
		});
	}

	$effect(() => {
		const corp = $playerCorp;
		if (corp) {
			draw(corp.id);
		}
	});

	onMount(() => {
		if ($playerCorp) {
			draw($playerCorp.id);
		}
	});

	onDestroy(() => {
		if (simulation) simulation.stop();
	});
</script>

<svg bind:this={svgElement} {width} {height} class="chart"></svg>

{#if !$playerCorp || ($playerCorp.infrastructure_count ?? 0) === 0}
	<div class="empty">Build infrastructure to see network topology</div>
{/if}

<div class="legend">
	<span class="legend-item"><span class="dot" style="background: #3b82f6"></span>Central Office</span>
	<span class="legend-item"><span class="dot" style="background: #10b981"></span>Cell Tower</span>
	<span class="legend-item"><span class="dot" style="background: #f59e0b"></span>Data Center</span>
	<span class="legend-item"><span class="dot" style="background: #8b5cf6"></span>Exchange</span>
</div>

<style>
	.chart {
		width: 100%;
		max-width: 100%;
		height: auto;
		background: rgba(10, 14, 23, 0.5);
		border-radius: var(--radius-sm);
	}

	.empty {
		color: var(--text-dim);
		font-size: 11px;
		text-align: center;
		padding: 8px;
	}

	.legend {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		padding: 4px 0;
	}

	.legend-item {
		display: flex;
		align-items: center;
		gap: 4px;
		font-size: 10px;
		color: var(--text-muted);
	}

	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		display: inline-block;
	}
</style>
