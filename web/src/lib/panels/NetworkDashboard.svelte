<script lang="ts">
	import { onMount } from 'svelte';
	import { playerCorp, worldInfo, regions, formatMoney } from '$lib/stores/gameState';
	import { chartHistory, networkHistory } from '$lib/stores/networkHistory';
	import { gameCommand } from '$lib/game/commandRouter';
	import * as bridge from '$lib/wasm/bridge';
	import * as d3 from 'd3';
	import type { TrafficFlows, InfrastructureList, InfraEdge, InfraNode, ContractInfo, AllInfrastructure } from '$lib/wasm/types';
	import type { NetworkSnapshot } from '$lib/stores/networkHistory';

	// -- Data state --
	let traffic: TrafficFlows = $state({
		edge_flows: [], node_flows: [], total_served: 0, total_dropped: 0,
		total_demand: 0, player_served: 0, player_dropped: 0, top_congested: []
	});
	let infra: InfrastructureList = $state({ nodes: [], edges: [] });
	let contracts: ContractInfo[] = $state([]);
	let allInfra: AllInfrastructure = $state({ nodes: [], edges: [] });

	// Refresh data each tick
	$effect(() => {
		const _tick = $worldInfo.tick;
		const corp = $playerCorp;
		if (corp) {
			traffic = bridge.getTrafficFlows();
			infra = bridge.getInfrastructureList(corp.id);
			contracts = bridge.getContracts(corp.id);
			allInfra = bridge.getAllInfrastructure();
		}
	});

	// -- Derived stats --
	let utilPct = $derived(
		traffic.total_demand > 0
			? (traffic.total_served / traffic.total_demand) * 100
			: 0
	);

	let dropPct = $derived(
		traffic.total_demand > 0
			? (traffic.total_dropped / traffic.total_demand) * 100
			: 0
	);

	let avgHealth = $derived(() => {
		if (infra.edges.length === 0) return 1;
		let sum = 0;
		for (const e of infra.edges) sum += e.health;
		return sum / infra.edges.length;
	});

	let alertCount = $derived(() => {
		let count = 0;
		for (const e of infra.edges) {
			if (e.utilization > 0.9 || e.health < 0.5) count++;
		}
		return count;
	});

	// ── Revenue by Infrastructure (Gap #19a) ─────────────────────────────────
	// Since per-infrastructure revenue isn't directly available, we estimate it
	// from utilization * throughput/bandwidth (the simulation generates revenue
	// proportionally to traffic served). We use maintenance_cost for profitability.

	interface RevenueRow {
		type: string;
		category: 'node' | 'edge';
		count: number;
		totalRevenue: number;
		avgRevenue: number;
		totalMaintenance: number;
		profitable: boolean;
	}

	let revenueRows = $derived.by((): RevenueRow[] => {
		const rows: RevenueRow[] = [];
		// Node revenue: estimate from utilization * max_throughput * revenue factor
		const nodeByType: Record<string, { count: number; totalRev: number; totalMaint: number }> = {};
		for (const n of infra.nodes) {
			if (n.under_construction) continue;
			const key = n.node_type;
			if (!nodeByType[key]) nodeByType[key] = { count: 0, totalRev: 0, totalMaint: 0 };
			nodeByType[key].count++;
			// Estimate revenue: traffic served proportional to utilization * throughput
			const estRevenue = n.utilization * n.max_throughput * 0.01;
			nodeByType[key].totalRev += estRevenue;
			nodeByType[key].totalMaint += n.maintenance_cost;
		}
		for (const [type, data] of Object.entries(nodeByType)) {
			rows.push({
				type,
				category: 'node',
				count: data.count,
				totalRevenue: data.totalRev,
				avgRevenue: data.count > 0 ? data.totalRev / data.count : 0,
				totalMaintenance: data.totalMaint,
				profitable: data.totalRev > data.totalMaint,
			});
		}
		// Edge revenue: estimate from utilization * bandwidth
		const edgeByType: Record<string, { count: number; totalRev: number; totalMaint: number }> = {};
		for (const e of infra.edges) {
			const key = e.edge_type;
			if (!edgeByType[key]) edgeByType[key] = { count: 0, totalRev: 0, totalMaint: 0 };
			edgeByType[key].count++;
			const estRevenue = e.utilization * e.bandwidth * 0.005;
			edgeByType[key].totalRev += estRevenue;
			// Edges don't have maintenance_cost directly; estimate from length
			const estMaint = e.length_km * 0.5;
			edgeByType[key].totalMaint += estMaint;
		}
		for (const [type, data] of Object.entries(edgeByType)) {
			rows.push({
				type,
				category: 'edge',
				count: data.count,
				totalRevenue: data.totalRev,
				avgRevenue: data.count > 0 ? data.totalRev / data.count : 0,
				totalMaintenance: data.totalMaint,
				profitable: data.totalRev > data.totalMaint,
			});
		}
		// Sort by total revenue descending
		rows.sort((a, b) => b.totalRevenue - a.totalRevenue);
		return rows;
	});

	// ── SLA Monitoring (Gap #19b) ────────────────────────────────────────────

	interface SLARow {
		id: number;
		partner: string;
		slaTarget: number; // uptime percentage target
		currentPerformance: number; // current uptime
		status: 'ok' | 'at_risk' | 'breach';
		penalty: number;
		pricePerTick: number;
	}

	let slaRows = $derived.by((): SLARow[] => {
		return contracts
			.filter(c => c.status === 'Active')
			.map(c => {
				// SLA target: derive from contract type (capacity contracts have higher SLA)
				const slaTarget = c.capacity > 5000 ? 99.5 : c.capacity > 1000 ? 99.0 : 98.0;
				// Current performance: estimate from network health + drop rate
				const healthPct = avgHealth() * 100;
				const dropPenalty = dropPct * 0.5;
				const currentPerformance = Math.max(0, Math.min(100, healthPct - dropPenalty));
				const diff = currentPerformance - slaTarget;
				let status: 'ok' | 'at_risk' | 'breach';
				if (diff >= 0) status = 'ok';
				else if (diff >= -5) status = 'at_risk';
				else status = 'breach';
				return {
					id: c.id,
					partner: c.from === ($playerCorp?.id ?? 0) ? c.to_name : c.from_name,
					slaTarget,
					currentPerformance,
					status,
					penalty: status === 'breach' ? c.penalty : 0,
					pricePerTick: c.price_per_tick,
				};
			});
	});

	function slaStatusColor(status: 'ok' | 'at_risk' | 'breach'): string {
		if (status === 'ok') return '#10b981';
		if (status === 'at_risk') return '#f59e0b';
		return '#ef4444';
	}

	function slaStatusLabel(status: 'ok' | 'at_risk' | 'breach'): string {
		if (status === 'ok') return 'OK';
		if (status === 'at_risk') return 'AT RISK';
		return 'BREACH';
	}

	// ── Maintenance Queue (Gap #19c) ─────────────────────────────────────────

	interface MaintenanceItem {
		id: number;
		type: string;
		category: 'node' | 'edge';
		health: number;
		x: number;
		y: number;
		isRepairing: boolean;
		estCost: number;
	}

	let maintenanceQueue = $derived.by((): MaintenanceItem[] => {
		const items: MaintenanceItem[] = [];
		for (const n of infra.nodes) {
			if (n.health < 1.0 && !n.under_construction) {
				items.push({
					id: n.id,
					type: n.node_type,
					category: 'node',
					health: n.health,
					x: n.x,
					y: n.y,
					isRepairing: false, // would need repair state from sim
					estCost: n.maintenance_cost * (1 - n.health) * 10,
				});
			}
		}
		for (const e of infra.edges) {
			if (e.health < 1.0) {
				items.push({
					id: e.id,
					type: e.edge_type,
					category: 'edge',
					health: e.health,
					x: (e.src_x + e.dst_x) / 2,
					y: (e.src_y + e.dst_y) / 2,
					isRepairing: false,
					estCost: e.length_km * (1 - e.health) * 5,
				});
			}
		}
		// Sort by health ascending (worst first)
		items.sort((a, b) => a.health - b.health);
		return items;
	});

	let totalMaintenanceBacklog = $derived(
		maintenanceQueue.reduce((sum, item) => sum + item.estCost, 0)
	);

	function repairItem(item: MaintenanceItem) {
		gameCommand({ RepairNode: { entity: item.id } });
	}

	function repairAllCritical() {
		for (const item of maintenanceQueue) {
			if (item.health < 0.5) {
				gameCommand({ RepairNode: { entity: item.id } });
			}
		}
	}

	function viewLocation(x: number, y: number) {
		window.dispatchEvent(new CustomEvent('map-fly-to', {
			detail: { lon: x, lat: y, zoom: 8 }
		}));
	}

	// ── Capacity Planning (Gap #19d + #28) ───────────────────────────────────

	let growthSlider = $state(1.0); // What-if growth multiplier (1.0x to 3.0x)
	let whatIfGrowthPct = $state(10); // What-if traffic growth rate (0% to 50%, step 5%)

	// Edges nearing capacity (>70% utilization)
	let nearCapacityEdges = $derived.by(() => {
		return infra.edges
			.filter(e => e.utilization > 0.7)
			.sort((a, b) => b.utilization - a.utilization)
			.slice(0, 10);
	});

	// Estimate ticks until full capacity for an edge
	function ticksUntilFull(edge: InfraEdge, growthRate: number): number {
		if (edge.utilization >= 1.0) return 0;
		if (growthRate <= 0) return Infinity;
		const remaining = 1.0 - edge.utilization;
		// Each tick, utilization grows by growthRate fraction
		return Math.ceil(remaining / growthRate);
	}

	// Build node ID → node type lookup for From→To display
	let nodeTypeMap = $derived.by((): Map<number, string> => {
		const m = new Map<number, string>();
		for (const n of infra.nodes) {
			m.set(n.id, n.node_type);
		}
		return m;
	});

	/** Shorten a node type name for table display. */
	function shortNodeType(nodeId: number): string {
		const t = nodeTypeMap.get(nodeId);
		if (!t) return `#${nodeId}`;
		// Convert CamelCase to short readable form
		return t.replace(/([a-z])([A-Z])/g, '$1 $2');
	}

	/** Color for ticks-to-exceed threshold. */
	function ttxColor(ticks: number): string {
		if (ticks < 50) return '#ef4444';   // red
		if (ticks < 200) return '#f59e0b';  // amber
		return '#10b981';                     // green
	}

	// ── What-If Analysis: Per-edge capacity stress test ──────────────────────
	// Uses the whatIfGrowthPct slider (0-50% increase) to project which edges
	// would exceed capacity at that growth rate. Uses linear regression data
	// when available, falls back to uniform growth assumption.

	interface WhatIfRow {
		id: number;
		edgeType: string;
		fromLabel: string;
		toLabel: string;
		currentUtil: number;
		ticksToExceed: number;
		src_x: number;
		src_y: number;
		dst_x: number;
		dst_y: number;
	}

	let whatIfAnalysis = $derived.by((): WhatIfRow[] => {
		const growthFraction = whatIfGrowthPct / 100;
		if (growthFraction <= 0) return [];
		if (infra.edges.length === 0) return [];

		// Determine per-tick growth rate for edges.
		// If we have linear regression data, scale its rate by the slider.
		// Otherwise, assume the slider percentage applies each tick.
		let perTickRate: number;
		if (capacityProjection && capacityProjection.growthPctPerTick > 0) {
			// Base rate from regression, scaled by the what-if growth multiplier
			const baseRatePerTick = capacityProjection.growthPctPerTick / 100;
			perTickRate = baseRatePerTick * (1 + growthFraction);
		} else {
			// No regression data: use the slider as a flat per-tick fraction
			perTickRate = growthFraction * 0.01; // scale down for per-tick
		}

		if (perTickRate <= 0) return [];

		return infra.edges
			.filter(e => e.utilization > 0.1) // only edges with some load
			.map(e => {
				const remaining = 1.0 - e.utilization;
				const ticks = e.utilization >= 1.0 ? 0 : Math.ceil(remaining / perTickRate);
				return {
					id: e.id,
					edgeType: e.edge_type,
					fromLabel: shortNodeType(e.source),
					toLabel: shortNodeType(e.target),
					currentUtil: e.utilization,
					ticksToExceed: ticks,
					src_x: e.src_x,
					src_y: e.src_y,
					dst_x: e.dst_x,
					dst_y: e.dst_y,
				};
			})
			.filter(e => e.ticksToExceed >= 0 && e.ticksToExceed < 500)
			.sort((a, b) => a.ticksToExceed - b.ticksToExceed)
			.slice(0, 12);
	});

	// Linear regression on networkHistory for capacity projections (Gap #28)
	interface CapacityProjection {
		currentThroughput: number;
		projectedThroughput: number;
		growthRatePerTick: number;
		growthPctPerTick: number;
		exceedsCapacity: boolean;
		ticksToExceed: number;
	}

	let capacityProjection = $derived.by((): CapacityProjection | null => {
		const history = $networkHistory;
		if (history.length < 10) return null;

		// Use last 50 snapshots (or all available)
		const data = history.slice(-50);
		const n = data.length;
		if (n < 2) return null;

		// Linear regression: served = a + b * tick
		let sumX = 0, sumY = 0, sumXY = 0, sumXX = 0;
		for (const snap of data) {
			sumX += snap.tick;
			sumY += snap.served;
			sumXY += snap.tick * snap.served;
			sumXX += snap.tick * snap.tick;
		}
		const b = (n * sumXY - sumX * sumY) / (n * sumXX - sumX * sumX);
		const a = (sumY - b * sumX) / n;

		const currentThroughput = data[n - 1].served;
		const currentTick = data[n - 1].tick;
		const projectedThroughput = a + b * (currentTick + 50);
		const growthRatePerTick = b;
		const growthPctPerTick = currentThroughput > 0 ? (b / currentThroughput) * 100 : 0;

		// Check if projected demand exceeds total demand capacity
		const currentDemand = data[n - 1].demand;
		const exceedsCapacity = projectedThroughput > currentDemand * 1.2;
		const ticksToExceed = b > 0 && currentDemand > currentThroughput
			? Math.ceil((currentDemand - currentThroughput) / b)
			: -1;

		return {
			currentThroughput,
			projectedThroughput,
			growthRatePerTick,
			growthPctPerTick,
			exceedsCapacity,
			ticksToExceed,
		};
	});

	// What-if analysis: edges that would hit capacity at given growth multiplier
	let whatIfEdges = $derived.by(() => {
		if (!capacityProjection || capacityProjection.growthRatePerTick <= 0) return [];
		// growthRate per edge per tick: approximate from overall growth
		const baseRate = capacityProjection.growthPctPerTick / 100;
		const adjRate = baseRate * growthSlider;
		return infra.edges
			.filter(e => e.utilization > 0.3)
			.map(e => ({
				id: e.id,
				type: e.edge_type,
				utilization: e.utilization,
				ticksToFull: ticksUntilFull(e, adjRate),
				src_x: e.src_x,
				src_y: e.src_y,
				dst_x: e.dst_x,
				dst_y: e.dst_y,
			}))
			.filter(e => e.ticksToFull < 200 && e.ticksToFull >= 0)
			.sort((a, b) => a.ticksToFull - b.ticksToFull)
			.slice(0, 8);
	});

	// Capacity sparkline chart (D3) - Gap #28
	let sparkSvg: SVGSVGElement;
	const spW = 460;
	const spH = 80;
	const spMargin = { top: 8, right: 16, bottom: 20, left: 52 };

	function drawCapacitySparkline(data: NetworkSnapshot[], projection: CapacityProjection | null) {
		if (!sparkSvg || data.length < 2) return;
		const svg = d3.select(sparkSvg);
		svg.selectAll('*').remove();

		const innerW = spW - spMargin.left - spMargin.right;
		const innerH = spH - spMargin.top - spMargin.bottom;
		const g = svg.append('g').attr('transform', `translate(${spMargin.left},${spMargin.top})`);

		const lastTick = data[data.length - 1].tick;
		const firstTick = data[0].tick;

		// Extend x domain 50 ticks into the future for projection
		const xScale = d3.scaleLinear()
			.domain([firstTick, lastTick + 50])
			.range([0, innerW]);

		const allVals = data.map(d => d.served);
		const projVal = projection ? projection.projectedThroughput : 0;
		const yMax = Math.max(d3.max(allVals) ?? 1, projVal) * 1.1;
		const yScale = d3.scaleLinear()
			.domain([0, yMax])
			.range([innerH, 0])
			.nice();

		// Gridlines
		g.append('g')
			.call(d3.axisLeft(yScale).ticks(3).tickSize(-innerW).tickFormat(() => ''))
			.selectAll('line').attr('stroke', 'rgba(55, 65, 81, 0.2)');
		g.selectAll('.domain').remove();

		// Historical line (green)
		const line = d3.line<NetworkSnapshot>()
			.x(d => xScale(d.tick))
			.y(d => yScale(d.served))
			.curve(d3.curveMonotoneX);
		g.append('path').datum(data)
			.attr('fill', 'none')
			.attr('stroke', '#10b981')
			.attr('stroke-width', 1.5)
			.attr('d', line);

		// Projection line (dashed amber/red)
		if (projection && data.length > 0) {
			const lastPoint = data[data.length - 1];
			const projColor = projection.exceedsCapacity ? '#ef4444' : '#f59e0b';
			g.append('line')
				.attr('x1', xScale(lastPoint.tick))
				.attr('y1', yScale(lastPoint.served))
				.attr('x2', xScale(lastPoint.tick + 50))
				.attr('y2', yScale(projection.projectedThroughput))
				.attr('stroke', projColor)
				.attr('stroke-width', 1.5)
				.attr('stroke-dasharray', '4,3');

			// Future zone marker
			g.append('rect')
				.attr('x', xScale(lastPoint.tick))
				.attr('y', 0)
				.attr('width', innerW - xScale(lastPoint.tick))
				.attr('height', innerH)
				.attr('fill', 'rgba(245, 158, 11, 0.05)');
		}

		// X axis
		g.append('g')
			.attr('transform', `translate(0,${innerH})`)
			.call(d3.axisBottom(xScale).ticks(5).tickFormat(d => `${d}`))
			.selectAll('text').attr('fill', '#6b7280').attr('font-size', '8px');
		g.selectAll('.tick line').attr('stroke', '#374151');

		// Y axis
		g.append('g')
			.call(d3.axisLeft(yScale).ticks(3).tickFormat(d => {
				const n = d as number;
				if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(0)}M`;
				if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
				return `${n}`;
			}))
			.selectAll('text').attr('fill', '#6b7280').attr('font-size', '8px');
	}

	$effect(() => {
		drawCapacitySparkline($chartHistory, capacityProjection);
	});

	// ── Traffic OD Matrix (Gap #26) ──────────────────────────────────────────

	interface ODEntry {
		srcRegion: string;
		dstRegion: string;
		volume: number;
	}

	// Map a coordinate (lon, lat) to the nearest region name
	function findRegionForCoord(lon: number, lat: number, regionList: typeof $regions): string {
		let bestDist = Infinity;
		let bestName = 'Unknown';
		for (const r of regionList) {
			const dx = r.center_lon - lon;
			const dy = r.center_lat - lat;
			const dist = dx * dx + dy * dy;
			if (dist < bestDist) {
				bestDist = dist;
				bestName = r.name;
			}
		}
		return bestName;
	}

	let odMatrix = $derived.by(() => {
		const regionList = $regions;
		if (regionList.length === 0 || traffic.edge_flows.length === 0) return { regionNames: [] as string[], matrix: [] as number[][] };

		// Build region traffic map from edge flows
		const pairMap: Record<string, number> = {};
		const regionTraffic: Record<string, number> = {};

		for (const flow of traffic.edge_flows) {
			if (flow.traffic <= 0) continue;
			const srcRegion = findRegionForCoord(flow.src_x, flow.src_y, regionList);
			const dstRegion = findRegionForCoord(flow.dst_x, flow.dst_y, regionList);
			const key = `${srcRegion}|${dstRegion}`;
			pairMap[key] = (pairMap[key] ?? 0) + flow.traffic;
			regionTraffic[srcRegion] = (regionTraffic[srcRegion] ?? 0) + flow.traffic;
			regionTraffic[dstRegion] = (regionTraffic[dstRegion] ?? 0) + flow.traffic;
		}

		// Get top 10 regions by traffic volume
		const topRegions = Object.entries(regionTraffic)
			.sort((a, b) => b[1] - a[1])
			.slice(0, 10)
			.map(e => e[0]);

		if (topRegions.length === 0) return { regionNames: [] as string[], matrix: [] as number[][] };

		// Build matrix
		const matrix: number[][] = [];
		for (const src of topRegions) {
			const row: number[] = [];
			for (const dst of topRegions) {
				const key = `${src}|${dst}`;
				row.push(pairMap[key] ?? 0);
			}
			matrix.push(row);
		}

		return { regionNames: topRegions, matrix };
	});

	// Get max value for color scaling in OD matrix
	let odMaxValue = $derived.by(() => {
		let max = 0;
		for (const row of odMatrix.matrix) {
			for (const val of row) {
				if (val > max) max = val;
			}
		}
		return max;
	});

	function odCellColor(value: number): string {
		if (odMaxValue <= 0 || value <= 0) return 'rgba(55, 65, 81, 0.2)';
		const ratio = value / odMaxValue;
		if (ratio < 0.33) return `rgba(16, 185, 129, ${0.2 + ratio * 1.5})`;
		if (ratio < 0.66) return `rgba(245, 158, 11, ${0.3 + (ratio - 0.33) * 1.5})`;
		return `rgba(239, 68, 68, ${0.4 + (ratio - 0.66) * 1.5})`;
	}

	function truncRegion(name: string): string {
		return name.length > 10 ? name.slice(0, 9) + '...' : name;
	}

	function flyToRegion(regionName: string) {
		const region = $regions.find(r => r.name === regionName);
		if (region) {
			window.dispatchEvent(new CustomEvent('map-fly-to', {
				detail: { lon: region.center_lon, lat: region.center_lat, zoom: 5 }
			}));
		}
	}

	// -- Color helpers --
	function dropColor(pct: number): string {
		if (pct < 1) return 'var(--green)';
		if (pct <= 5) return 'var(--amber)';
		return 'var(--red)';
	}

	function healthColor(h: number): string {
		if (h > 0.8) return 'var(--green)';
		if (h >= 0.5) return 'var(--amber)';
		return 'var(--red)';
	}

	function utilColor(u: number): string {
		if (u < 0.6) return '#10b981';
		if (u < 0.8) return '#f59e0b';
		return '#ef4444';
	}

	// -- Format helpers --
	function fmtTraffic(n: number): string {
		if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
		return n.toFixed(0);
	}

	function fmtMoney(n: number): string {
		return formatMoney(Math.round(n));
	}

	// -- Traffic Overview Chart (D3) --
	let trafficSvg: SVGSVGElement;
	const tW = 460;
	const tH = 160;
	const tMargin = { top: 24, right: 16, bottom: 24, left: 52 };

	function drawTrafficChart(data: NetworkSnapshot[]) {
		if (!trafficSvg || data.length < 2) return;
		const svg = d3.select(trafficSvg);
		svg.selectAll('*').remove();

		const innerW = tW - tMargin.left - tMargin.right;
		const innerH = tH - tMargin.top - tMargin.bottom;

		const g = svg.append('g').attr('transform', `translate(${tMargin.left},${tMargin.top})`);

		const xScale = d3.scaleLinear()
			.domain(d3.extent(data, d => d.tick) as [number, number])
			.range([0, innerW]);

		const allVals = data.flatMap(d => [d.served, d.demand, d.dropped]);
		const yMax = d3.max(allVals) ?? 1;
		const yScale = d3.scaleLinear()
			.domain([0, yMax * 1.1])
			.range([innerH, 0])
			.nice();

		// Gridlines
		g.append('g')
			.call(d3.axisLeft(yScale).ticks(4).tickSize(-innerW).tickFormat(() => ''))
			.selectAll('line').attr('stroke', 'rgba(55, 65, 81, 0.3)');
		g.selectAll('.domain').remove();

		// X axis
		g.append('g')
			.attr('transform', `translate(0,${innerH})`)
			.call(d3.axisBottom(xScale).ticks(6).tickFormat(d => `${d}`))
			.selectAll('text').attr('fill', '#6b7280').attr('font-size', '9px');
		g.selectAll('.tick line').attr('stroke', '#374151');

		// Y axis
		g.append('g')
			.call(d3.axisLeft(yScale).ticks(4).tickFormat(d => {
				const n = d as number;
				if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(0)}M`;
				if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`;
				return `${n}`;
			}))
			.selectAll('text').attr('fill', '#6b7280').attr('font-size', '9px');

		// Demand line (blue dashed)
		const demandLine = d3.line<NetworkSnapshot>()
			.x(d => xScale(d.tick))
			.y(d => yScale(d.demand))
			.curve(d3.curveMonotoneX);
		g.append('path').datum(data)
			.attr('fill', 'none')
			.attr('stroke', '#3b82f6')
			.attr('stroke-width', 1.5)
			.attr('stroke-dasharray', '4,3')
			.attr('d', demandLine);

		// Served line (green solid)
		const servedLine = d3.line<NetworkSnapshot>()
			.x(d => xScale(d.tick))
			.y(d => yScale(d.served))
			.curve(d3.curveMonotoneX);
		g.append('path').datum(data)
			.attr('fill', 'none')
			.attr('stroke', '#10b981')
			.attr('stroke-width', 2)
			.attr('d', servedLine);

		// Dropped dots (red)
		const droppedData = data.filter(d => d.dropped > 0);
		g.selectAll('.drop-dot')
			.data(droppedData)
			.join('circle')
			.attr('cx', d => xScale(d.tick))
			.attr('cy', d => yScale(d.dropped))
			.attr('r', 3)
			.attr('fill', '#ef4444')
			.attr('opacity', 0.8);

		// Legend
		const legend = svg.append('g').attr('transform', `translate(${tMargin.left + 4}, 8)`);
		// Served
		legend.append('line').attr('x1', 0).attr('y1', 5).attr('x2', 14).attr('y2', 5)
			.attr('stroke', '#10b981').attr('stroke-width', 2);
		legend.append('text').attr('x', 18).attr('y', 8).attr('fill', '#9ca3af')
			.attr('font-size', '9px').text('Served');
		// Demand
		legend.append('line').attr('x1', 60).attr('y1', 5).attr('x2', 74).attr('y2', 5)
			.attr('stroke', '#3b82f6').attr('stroke-width', 1.5).attr('stroke-dasharray', '4,3');
		legend.append('text').attr('x', 78).attr('y', 8).attr('fill', '#9ca3af')
			.attr('font-size', '9px').text('Demand');
		// Dropped
		legend.append('circle').attr('cx', 130).attr('cy', 5).attr('r', 3).attr('fill', '#ef4444');
		legend.append('text').attr('x', 137).attr('y', 8).attr('fill', '#9ca3af')
			.attr('font-size', '9px').text('Dropped');
	}

	$effect(() => {
		drawTrafficChart($chartHistory);
	});

	// -- Node Utilization Distribution Chart (D3) --
	let distSvg: SVGSVGElement;
	const dW = 460;
	const dH = 120;
	const dMargin = { top: 8, right: 16, bottom: 24, left: 40 };

	function drawDistribution(nodes: InfraNode[]) {
		if (!distSvg || nodes.length === 0) return;
		const svg = d3.select(distSvg);
		svg.selectAll('*').remove();

		const bins = [0, 0, 0, 0, 0]; // 0-20, 20-40, 40-60, 60-80, 80-100
		for (const node of nodes) {
			const pct = node.utilization * 100;
			if (pct < 20) bins[0]++;
			else if (pct < 40) bins[1]++;
			else if (pct < 60) bins[2]++;
			else if (pct < 80) bins[3]++;
			else bins[4]++;
		}

		const innerW = dW - dMargin.left - dMargin.right;
		const innerH = dH - dMargin.top - dMargin.bottom;
		const g = svg.append('g').attr('transform', `translate(${dMargin.left},${dMargin.top})`);

		const labels = ['0-20%', '20-40%', '40-60%', '60-80%', '80-100%'];
		const colors = ['#10b981', '#10b981', '#f59e0b', '#f59e0b', '#ef4444'];

		const xScale = d3.scaleBand()
			.domain(labels)
			.range([0, innerW])
			.padding(0.25);

		const yMax = d3.max(bins) ?? 1;
		const yScale = d3.scaleLinear()
			.domain([0, yMax])
			.range([innerH, 0])
			.nice();

		// Bars
		g.selectAll('.bar')
			.data(bins)
			.join('rect')
			.attr('x', (_d, i) => xScale(labels[i])!)
			.attr('y', d => yScale(d))
			.attr('width', xScale.bandwidth())
			.attr('height', d => innerH - yScale(d))
			.attr('fill', (_d, i) => colors[i])
			.attr('opacity', 0.75)
			.attr('rx', 2);

		// Count labels above bars
		g.selectAll('.bar-label')
			.data(bins)
			.join('text')
			.attr('x', (_d, i) => xScale(labels[i])! + xScale.bandwidth() / 2)
			.attr('y', d => yScale(d) - 4)
			.attr('text-anchor', 'middle')
			.attr('fill', '#9ca3af')
			.attr('font-size', '9px')
			.text(d => d > 0 ? d : '');

		// X axis
		g.append('g')
			.attr('transform', `translate(0,${innerH})`)
			.call(d3.axisBottom(xScale).tickSize(0))
			.selectAll('text').attr('fill', '#6b7280').attr('font-size', '9px');
		g.selectAll('.domain').attr('stroke', '#374151');

		// Y axis
		g.append('g')
			.call(d3.axisLeft(yScale).ticks(3).tickFormat(d => `${d}`))
			.selectAll('text').attr('fill', '#6b7280').attr('font-size', '9px');
		g.selectAll('.tick line').attr('stroke', '#374151');
	}

	$effect(() => {
		drawDistribution(infra.nodes.filter(n => !n.under_construction));
	});

	// -- Infrastructure Summary Chart (D3) --
	let infraSvg: SVGSVGElement;
	const iW = 460;
	const iH = 100;
	const iMargin = { top: 8, right: 16, bottom: 24, left: 60 };

	function drawInfraSummary(nodes: InfraNode[], edges: InfraEdge[]) {
		if (!infraSvg) return;
		const svg = d3.select(infraSvg);
		svg.selectAll('*').remove();

		// Count nodes by type
		const nodeCounts: Record<string, number> = {};
		for (const n of nodes) {
			nodeCounts[n.node_type] = (nodeCounts[n.node_type] ?? 0) + 1;
		}

		const entries = Object.entries(nodeCounts).sort((a, b) => b[1] - a[1]).slice(0, 6);
		if (entries.length === 0) return;

		const innerW = iW - iMargin.left - iMargin.right;
		const innerH = iH - iMargin.top - iMargin.bottom;
		const g = svg.append('g').attr('transform', `translate(${iMargin.left},${iMargin.top})`);

		const yScale = d3.scaleBand()
			.domain(entries.map(e => e[0]))
			.range([0, innerH])
			.padding(0.2);

		const xMax = d3.max(entries, e => e[1]) ?? 1;
		const xScale = d3.scaleLinear()
			.domain([0, xMax])
			.range([0, innerW])
			.nice();

		g.selectAll('.bar')
			.data(entries)
			.join('rect')
			.attr('x', 0)
			.attr('y', d => yScale(d[0])!)
			.attr('width', d => xScale(d[1]))
			.attr('height', yScale.bandwidth())
			.attr('fill', '#3b82f6')
			.attr('opacity', 0.7)
			.attr('rx', 2);

		g.selectAll('.count-label')
			.data(entries)
			.join('text')
			.attr('x', d => xScale(d[1]) + 4)
			.attr('y', d => yScale(d[0])! + yScale.bandwidth() / 2 + 1)
			.attr('dominant-baseline', 'middle')
			.attr('fill', '#9ca3af')
			.attr('font-size', '9px')
			.attr('font-family', 'var(--font-mono)')
			.text(d => d[1]);

		// Y axis labels (node type names)
		g.append('g')
			.call(d3.axisLeft(yScale).tickSize(0))
			.selectAll('text').attr('fill', '#d1d5db').attr('font-size', '9px');
		g.selectAll('.domain').attr('stroke', '#374151');
	}

	$effect(() => {
		drawInfraSummary(infra.nodes, infra.edges);
	});

	// -- View edge action (fly to edge midpoint) --
	function viewEdge(edgeId: number) {
		// Find edge from all infrastructure (to get coordinates)
		const allInfraSnap = bridge.getAllInfrastructure();
		const edge = allInfraSnap.edges.find(e => e.id === edgeId);
		if (!edge) return;
		const midLon = (edge.src_x + edge.dst_x) / 2;
		const midLat = (edge.src_y + edge.dst_y) / 2;
		window.dispatchEvent(new CustomEvent('map-fly-to', {
			detail: { lon: midLon, lat: midLat, zoom: 8 }
		}));
	}

	// ── Bottleneck Upgrade Suggestions (Phase 10.2.3) ────────────────────
	function upgradeSuggestion(utilization: number): { text: string; color: string; level: 'upgrade' | 'monitor' } | null {
		if (utilization > 0.9) return { text: 'UPGRADE: Add parallel edge', color: '#ef4444', level: 'upgrade' };
		if (utilization > 0.8) return { text: 'MONITOR: Consider capacity upgrade', color: '#f59e0b', level: 'monitor' };
		return null;
	}

	// ── Per-Corporation Coverage Comparison (Phase 10.2.6) ───────────────

	interface CorpCoverage {
		id: number;
		name: string;
		color: string;
		nodeCount: number;
		edgeCount: number;
		totalBandwidth: number;
		totalThroughput: number;
		avgUtilization: number;
		isPlayer: boolean;
	}

	const CORP_COLORS = ['#3b82f6', '#ef4444', '#10b981', '#f59e0b', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16'];

	let corpCoverage = $derived.by((): CorpCoverage[] => {
		if (allInfra.nodes.length === 0 && allInfra.edges.length === 0) return [];

		const corpMap = new Map<number, CorpCoverage>();
		const playerId = $playerCorp?.id ?? 0;

		for (const n of allInfra.nodes) {
			if (!corpMap.has(n.owner)) {
				corpMap.set(n.owner, {
					id: n.owner,
					name: n.owner_name || `Corp #${n.owner}`,
					color: '',
					nodeCount: 0,
					edgeCount: 0,
					totalBandwidth: 0,
					totalThroughput: n.max_throughput,
					avgUtilization: 0,
					isPlayer: n.owner === playerId,
				});
			}
			const entry = corpMap.get(n.owner)!;
			entry.nodeCount++;
			entry.totalThroughput += n.max_throughput;
			entry.avgUtilization += n.utilization;
		}

		for (const e of allInfra.edges) {
			if (!corpMap.has(e.owner)) {
				corpMap.set(e.owner, {
					id: e.owner,
					name: e.owner_name || `Corp #${e.owner}`,
					color: '',
					nodeCount: 0,
					edgeCount: 0,
					totalBandwidth: 0,
					totalThroughput: 0,
					avgUtilization: 0,
					isPlayer: e.owner === playerId,
				});
			}
			const entry = corpMap.get(e.owner)!;
			entry.edgeCount++;
			entry.totalBandwidth += e.bandwidth;
		}

		// Finalize averages and assign colors
		const corps = [...corpMap.values()];
		// Sort: player first, then by node count descending
		corps.sort((a, b) => {
			if (a.isPlayer && !b.isPlayer) return -1;
			if (!a.isPlayer && b.isPlayer) return 1;
			return (b.nodeCount + b.edgeCount) - (a.nodeCount + a.edgeCount);
		});

		for (let i = 0; i < corps.length; i++) {
			const c = corps[i];
			c.color = c.isPlayer ? '#3b82f6' : CORP_COLORS[(i) % CORP_COLORS.length];
			if (c.nodeCount > 0) {
				c.avgUtilization = c.avgUtilization / c.nodeCount;
			}
		}

		return corps;
	});

	let totalInfraCount = $derived(
		corpCoverage.reduce((sum, c) => sum + c.nodeCount + c.edgeCount, 0)
	);

	onMount(() => {
		drawTrafficChart($chartHistory);
		drawDistribution(infra.nodes.filter(n => !n.under_construction));
		drawInfraSummary(infra.nodes, infra.edges);
		drawCapacitySparkline($chartHistory, capacityProjection);
	});
</script>

<div class="panel" aria-label="Network Dashboard">
	<!-- Header Stats Row -->
	<div class="stats-row">
		<div class="stat-card">
			<div class="stat-label">Total Throughput</div>
			<div class="stat-value mono">
				{fmtTraffic(traffic.total_served)} <span class="stat-sep">/</span> {fmtTraffic(traffic.total_demand)}
			</div>
			<div class="stat-sub" style="color: {utilPct >= 80 ? 'var(--green)' : utilPct >= 50 ? 'var(--amber)' : 'var(--red)'}">
				{utilPct.toFixed(1)}% utilization
			</div>
		</div>

		<div class="stat-card">
			<div class="stat-label">Packet Loss</div>
			<div class="stat-value mono" style="color: {dropColor(dropPct)}">
				{fmtTraffic(traffic.total_dropped)}
			</div>
			<div class="stat-sub" style="color: {dropColor(dropPct)}">
				{dropPct.toFixed(1)}% drop rate
			</div>
		</div>

		<div class="stat-card">
			<div class="stat-label">Network Health</div>
			<div class="stat-value mono" style="color: {healthColor(avgHealth())}">
				{(avgHealth() * 100).toFixed(0)}%
			</div>
			<div class="stat-sub" style="color: {healthColor(avgHealth())}">
				avg edge health
			</div>
		</div>

		<div class="stat-card">
			<div class="stat-label">Active Alerts</div>
			<div class="stat-value mono" style="color: {alertCount() > 0 ? 'var(--red)' : 'var(--green)'}">
				{alertCount()}
			</div>
			<div class="stat-sub muted">
				{alertCount() === 0 ? 'all clear' : 'edges stressed'}
			</div>
		</div>
	</div>

	<!-- Traffic Overview Chart -->
	<div class="section">
		<h3>TRAFFIC OVERVIEW</h3>
		<div class="chart-wrapper">
			<svg bind:this={trafficSvg} width={tW} height={tH} class="chart"></svg>
			{#if $chartHistory.length < 2}
				<div class="chart-empty">Collecting traffic data...</div>
			{/if}
		</div>
	</div>

	<!-- Revenue by Infrastructure Widget (Gap #19a) -->
	<section class="section">
		<h3>REVENUE BY INFRASTRUCTURE</h3>
		{#if revenueRows.length > 0}
			<div class="rev-table-header">
				<span class="rev-col-type">Type</span>
				<span class="rev-col-count">Count</span>
				<span class="rev-col-rev">Total Revenue</span>
				<span class="rev-col-avg">Avg/Unit</span>
			</div>
			{#each revenueRows as row}
				<div class="rev-table-row" style="border-left: 3px solid {row.profitable ? '#10b981' : '#ef4444'};">
					<span class="rev-col-type">
						<span class="rev-category-badge" class:rev-node={row.category === 'node'} class:rev-edge={row.category === 'edge'}>
							{row.category === 'node' ? 'N' : 'E'}
						</span>
						{row.type}
					</span>
					<span class="rev-col-count mono">{row.count}</span>
					<span class="rev-col-rev mono" style="color: {row.profitable ? '#10b981' : '#ef4444'}">
						{fmtMoney(row.totalRevenue)}
					</span>
					<span class="rev-col-avg mono">{fmtMoney(row.avgRevenue)}</span>
				</div>
			{/each}
		{:else}
			<div class="chart-empty">No operational infrastructure</div>
		{/if}
	</section>

	<!-- SLA Monitoring Widget (Gap #19b) -->
	<section class="section">
		<h3>SLA MONITORING</h3>
		{#if slaRows.length > 0}
			<div class="sla-table-header">
				<span class="sla-col-partner">Partner</span>
				<span class="sla-col-target">Target</span>
				<span class="sla-col-perf">Performance</span>
				<span class="sla-col-status">Status</span>
				<span class="sla-col-penalty">Penalty</span>
			</div>
			{#each slaRows as row}
				<div class="sla-table-row">
					<span class="sla-col-partner">{row.partner}</span>
					<span class="sla-col-target mono">{row.slaTarget.toFixed(1)}%</span>
					<span class="sla-col-perf mono" style="color: {slaStatusColor(row.status)}">
						{row.currentPerformance.toFixed(1)}%
					</span>
					<span class="sla-col-status">
						<span class="sla-badge" style="background: {slaStatusColor(row.status)}20; color: {slaStatusColor(row.status)}; border: 1px solid {slaStatusColor(row.status)}40;">
							{slaStatusLabel(row.status)}
						</span>
					</span>
					<span class="sla-col-penalty mono" style="color: {row.penalty > 0 ? '#ef4444' : 'var(--text-dim)'}">
						{row.penalty > 0 ? fmtMoney(row.penalty) : '--'}
					</span>
				</div>
			{/each}
		{:else}
			<div class="chart-empty">No active contracts with SLA requirements</div>
		{/if}
	</section>

	<!-- Maintenance Queue Widget (Gap #19c) -->
	<section class="section">
		<h3>MAINTENANCE QUEUE</h3>
		{#if maintenanceQueue.length > 0}
			<div class="maint-summary">
				<span>{maintenanceQueue.length} items need repair</span>
				<span class="dot"></span>
				<span>Backlog: <span class="mono" style="color: #ef4444">{fmtMoney(totalMaintenanceBacklog)}</span></span>
				<button class="repair-all-btn" onclick={repairAllCritical}>
					Repair All &lt;50%
				</button>
			</div>
			<div class="maint-table-header">
				<span class="maint-col-type">Type</span>
				<span class="maint-col-health">Health</span>
				<span class="maint-col-status">Status</span>
				<span class="maint-col-cost">Est. Cost</span>
				<span class="maint-col-action">Action</span>
			</div>
			{#each maintenanceQueue.slice(0, 12) as item}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="maint-table-row clickable-row" onclick={() => viewLocation(item.x, item.y)}>
					<span class="maint-col-type">
						<span class="rev-category-badge" class:rev-node={item.category === 'node'} class:rev-edge={item.category === 'edge'}>
							{item.category === 'node' ? 'N' : 'E'}
						</span>
						{item.type}
					</span>
					<span class="maint-col-health">
						<div class="health-bar-bg">
							<div class="health-bar-fill" style="width: {item.health * 100}%; background: {healthColor(item.health)};"></div>
						</div>
						<span class="mono" style="color: {healthColor(item.health)}; font-size: 10px;">
							{(item.health * 100).toFixed(0)}%
						</span>
					</span>
					<span class="maint-col-status" style="color: {item.isRepairing ? '#3b82f6' : '#f59e0b'}; font-size: 10px;">
						{item.isRepairing ? 'Repairing' : 'Needs Repair'}
					</span>
					<span class="maint-col-cost mono" style="font-size: 10px;">
						{fmtMoney(item.estCost)}
					</span>
					<span class="maint-col-action">
						<button class="view-btn" onclick={(e: MouseEvent) => { e.stopPropagation(); viewLocation(item.x, item.y); }} style="margin-right: 2px;">View</button>
						<button class="repair-btn" onclick={(e: MouseEvent) => { e.stopPropagation(); repairItem(item); }}>Repair</button>
					</span>
				</div>
			{/each}
			{#if maintenanceQueue.length > 12}
				<div class="chart-empty">...and {maintenanceQueue.length - 12} more items</div>
			{/if}
		{:else}
			<div class="chart-empty">All infrastructure at full health</div>
		{/if}
	</section>

	<!-- Capacity Planning Widget (Gap #19d + #28) -->
	<section class="section">
		<h3>CAPACITY PLANNING</h3>
		{#if capacityProjection}
			<div class="cap-stats-row">
				<div class="cap-stat">
					<span class="cap-stat-label">Current</span>
					<span class="cap-stat-value mono">{fmtTraffic(capacityProjection.currentThroughput)}</span>
				</div>
				<div class="cap-stat">
					<span class="cap-stat-label">Projected (50t)</span>
					<span class="cap-stat-value mono" style="color: {capacityProjection.exceedsCapacity ? '#ef4444' : '#f59e0b'}">
						{fmtTraffic(capacityProjection.projectedThroughput)}
					</span>
				</div>
				<div class="cap-stat">
					<span class="cap-stat-label">Growth</span>
					<span class="cap-stat-value mono" style="color: {capacityProjection.growthPctPerTick > 0 ? '#10b981' : '#6b7280'}">
						{capacityProjection.growthPctPerTick.toFixed(2)}%/tick
					</span>
				</div>
			</div>
			{#if capacityProjection.exceedsCapacity}
				<div class="cap-warning">
					Projected demand may exceed capacity within 50 ticks
				</div>
			{/if}
			<div class="chart-wrapper">
				<svg bind:this={sparkSvg} width={spW} height={spH} class="chart"></svg>
			</div>
		{:else}
			<div class="chart-empty">Collecting data for projections (need 10+ snapshots)...</div>
		{/if}

		<!-- Edges nearing capacity -->
		{#if nearCapacityEdges.length > 0}
			<div class="cap-subtitle">Edges Nearing Capacity (&gt;70%)</div>
			{#each nearCapacityEdges.slice(0, 5) as edge}
				<div class="cap-edge-row">
					<span class="cap-edge-type">{edge.edge_type}</span>
					<span class="cap-edge-util">
						<div class="util-bar-bg" style="width: 60px;">
							<div class="util-bar-fill" style="width: {edge.utilization * 100}%; background: {utilColor(edge.utilization)};"></div>
						</div>
						<span class="mono" style="color: {utilColor(edge.utilization)}; font-size: 10px;">{(edge.utilization * 100).toFixed(0)}%</span>
					</span>
					<button class="view-btn" onclick={() => viewEdge(edge.id)}>View</button>
				</div>
			{/each}
		{/if}

		<!-- What-if slider (legacy, kept for backward compat) -->
		<div class="whatif-section">
			<div class="whatif-header">
				<span class="cap-subtitle" style="margin: 0;">What-if: Traffic grows at</span>
				<span class="mono whatif-value">{growthSlider.toFixed(1)}x</span>
			</div>
			<input
				type="range"
				class="whatif-slider"
				min="1.0"
				max="3.0"
				step="0.1"
				bind:value={growthSlider}
			/>
			{#if whatIfEdges.length > 0}
				<div class="whatif-results">
					{#each whatIfEdges as edge}
						<div class="whatif-row">
							<span class="whatif-type">{edge.type}</span>
							<span class="mono" style="color: {edge.ticksToFull < 30 ? '#ef4444' : '#f59e0b'}; font-size: 10px;">
								~{edge.ticksToFull} ticks to full
							</span>
							<button class="view-btn" onclick={() => viewLocation((edge.src_x + edge.dst_x) / 2, (edge.src_y + edge.dst_y) / 2)}>View</button>
						</div>
					{/each}
				</div>
			{:else}
				<div class="chart-empty" style="padding: 6px 0;">No edges projected to hit capacity</div>
			{/if}
		</div>
	</section>

	<!-- What-If Capacity Analysis (Phase 10.2.8) -->
	<section class="section whatif-analysis-section">
		<h3>WHAT-IF CAPACITY ANALYSIS</h3>
		<div class="whatif-analysis-controls">
			<div class="whatif-analysis-label">
				<span class="whatif-analysis-label-text">Traffic growth rate</span>
				<span class="mono whatif-analysis-pct">{whatIfGrowthPct}%</span>
			</div>
			<input
				type="range"
				class="whatif-analysis-slider"
				min="0"
				max="50"
				step="5"
				aria-label="Traffic growth rate percentage"
				bind:value={whatIfGrowthPct}
			/>
			<div class="whatif-analysis-ticks-row">
				<span class="whatif-tick-label">0%</span>
				<span class="whatif-tick-label">10%</span>
				<span class="whatif-tick-label">20%</span>
				<span class="whatif-tick-label">30%</span>
				<span class="whatif-tick-label">40%</span>
				<span class="whatif-tick-label">50%</span>
			</div>
		</div>

		{#if whatIfGrowthPct > 0 && whatIfAnalysis.length > 0}
			<div class="whatif-analysis-summary">
				At <span class="mono" style="color: #3b82f6;">{whatIfGrowthPct}%</span> growth,
				<span class="mono" style="color: #ef4444;">{whatIfAnalysis.length}</span> edge{whatIfAnalysis.length !== 1 ? 's' : ''} hit capacity first:
			</div>
			<div class="whatif-table-header">
				<span class="wi-col-type">Edge Type</span>
				<span class="wi-col-route">From → To</span>
				<span class="wi-col-util">Util%</span>
				<span class="wi-col-ttx">Ticks to Exceed</span>
				<span class="wi-col-action"></span>
			</div>
			{#each whatIfAnalysis as row}
				<div class="whatif-table-row" style="border-left: 3px solid {ttxColor(row.ticksToExceed)};">
					<span class="wi-col-type">{row.edgeType}</span>
					<span class="wi-col-route mono" title="{row.fromLabel} → {row.toLabel}">
						{row.fromLabel} → {row.toLabel}
					</span>
					<span class="wi-col-util mono" style="color: {utilColor(row.currentUtil)};">
						{(row.currentUtil * 100).toFixed(0)}%
					</span>
					<span class="wi-col-ttx mono" style="color: {ttxColor(row.ticksToExceed)};">
						{row.ticksToExceed === 0 ? 'NOW' : row.ticksToExceed}
					</span>
					<span class="wi-col-action">
						<button class="view-btn" onclick={() => viewLocation((row.src_x + row.dst_x) / 2, (row.src_y + row.dst_y) / 2)}>View</button>
					</span>
				</div>
			{/each}
			<div class="whatif-legend">
				<span class="whatif-legend-item"><span class="whatif-swatch" style="background: #ef4444;"></span> &lt;50 ticks (critical)</span>
				<span class="whatif-legend-item"><span class="whatif-swatch" style="background: #f59e0b;"></span> &lt;200 ticks (warning)</span>
				<span class="whatif-legend-item"><span class="whatif-swatch" style="background: #10b981;"></span> &gt;200 ticks (safe)</span>
			</div>
		{:else if whatIfGrowthPct > 0}
			<div class="chart-empty">No edges projected to exceed capacity at {whatIfGrowthPct}% growth</div>
		{:else}
			<div class="chart-empty">Set a growth rate above 0% to see projections</div>
		{/if}
	</section>

	<!-- Top Congested Edges Table (Phase 10.2.3: Bottleneck Detection + Upgrade Suggestions) -->
	{#if traffic.top_congested.length > 0}
		<div class="section">
			<h3>TOP CONGESTED EDGES</h3>
			<div class="congested-table-header">
				<span class="cg-col-type">Edge Type</span>
				<span class="cg-col-util">Utilization</span>
				<span class="cg-col-suggest">Suggestion</span>
				<span class="cg-col-owner">Owner</span>
				<span class="cg-col-action"></span>
			</div>
			{#each traffic.top_congested.slice(0, 10) as ce}
				{@const suggestion = upgradeSuggestion(ce.utilization)}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="congested-table-row clickable-row" onclick={() => viewEdge(ce.id)}>
					<span class="cg-col-type">{ce.edge_type}</span>
					<span class="cg-col-util">
						<div class="util-bar-bg">
							<div class="util-bar-fill" style="width: {Math.min(ce.utilization * 100, 100)}%; background: {utilColor(ce.utilization)};"></div>
						</div>
						<span class="mono" style="color: {utilColor(ce.utilization)}">{(ce.utilization * 100).toFixed(0)}%</span>
					</span>
					<span class="cg-col-suggest">
						{#if suggestion}
							<span class="suggest-badge" class:suggest-upgrade={suggestion.level === 'upgrade'} class:suggest-monitor={suggestion.level === 'monitor'}>
								{suggestion.level === 'upgrade' ? 'UPGRADE' : 'MONITOR'}
							</span>
						{/if}
					</span>
					<span class="cg-col-owner mono">{ce.owner === ($playerCorp?.id ?? 0) ? 'You' : `#${ce.owner}`}</span>
					<span class="cg-col-action">
						<button class="view-btn" onclick={(e: MouseEvent) => { e.stopPropagation(); viewEdge(ce.id); }}>View</button>
					</span>
				</div>
				{#if suggestion}
					<div class="suggest-detail" style="color: {suggestion.color};">
						{suggestion.text}
					</div>
				{/if}
			{/each}
		</div>
	{:else}
		<div class="section">
			<h3>TOP CONGESTED EDGES</h3>
			<div class="chart-empty">No congestion detected</div>
		</div>
	{/if}

	<!-- Traffic OD Matrix (Gap #26) -->
	<section class="section">
		<h3>TRAFFIC ORIGIN-DESTINATION MATRIX</h3>
		{#if odMatrix.regionNames.length > 0}
			<div class="od-matrix-wrapper">
				<table class="od-matrix">
					<thead>
						<tr>
							<th class="od-corner"></th>
							{#each odMatrix.regionNames as name}
								<th class="od-col-header od-clickable" title="Fly to {name}" onclick={() => flyToRegion(name)}>{truncRegion(name)}</th>
							{/each}
						</tr>
					</thead>
					<tbody>
						{#each odMatrix.matrix as row, i}
							<tr>
								<td class="od-row-header od-clickable" title="Fly to {odMatrix.regionNames[i]}" onclick={() => flyToRegion(odMatrix.regionNames[i])}>{truncRegion(odMatrix.regionNames[i])}</td>
								{#each row as val}
									<td
										class="od-cell"
										style="background: {odCellColor(val)};"
										title={fmtTraffic(val)}
									>
										{val > 0 ? fmtTraffic(val) : ''}
									</td>
								{/each}
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
			<div class="od-legend">
				<span class="od-legend-item"><span class="od-swatch" style="background: rgba(16, 185, 129, 0.5);"></span> Low</span>
				<span class="od-legend-item"><span class="od-swatch" style="background: rgba(245, 158, 11, 0.6);"></span> Medium</span>
				<span class="od-legend-item"><span class="od-swatch" style="background: rgba(239, 68, 68, 0.7);"></span> High</span>
			</div>
		{:else}
			<div class="chart-empty">No inter-region traffic flows detected</div>
		{/if}
	</section>

	<!-- Node Utilization Distribution -->
	<div class="section">
		<h3>NODE UTILIZATION DISTRIBUTION</h3>
		<div class="chart-wrapper">
			<svg bind:this={distSvg} width={dW} height={dH} class="chart"></svg>
			{#if infra.nodes.filter(n => !n.under_construction).length === 0}
				<div class="chart-empty">No operational nodes</div>
			{/if}
		</div>
	</div>

	<!-- Infrastructure Summary -->
	<div class="section">
		<h3>INFRASTRUCTURE SUMMARY</h3>
		<div class="infra-summary-stats">
			<span>{infra.nodes.length} nodes</span>
			<span class="dot"></span>
			<span>{infra.edges.length} edges</span>
			<span class="dot"></span>
			<span>{infra.nodes.filter(n => n.under_construction).length} under construction</span>
		</div>
		<div class="chart-wrapper">
			<svg bind:this={infraSvg} width={iW} height={iH} class="chart"></svg>
			{#if infra.nodes.length === 0}
				<div class="chart-empty">No infrastructure built yet</div>
			{/if}
		</div>
	</div>

	<!-- Per-Corporation Coverage Comparison (Phase 10.2.6) -->
	<section class="section">
		<h3>CORPORATION COVERAGE COMPARISON</h3>
		{#if corpCoverage.length > 0}
			<div class="corp-cov-header">
				<span class="cc-col-name">Corporation</span>
				<span class="cc-col-nodes">Nodes</span>
				<span class="cc-col-edges">Edges</span>
				<span class="cc-col-bw">Bandwidth</span>
				<span class="cc-col-util">Avg Util</span>
				<span class="cc-col-share">Share</span>
			</div>
			{#each corpCoverage as corp}
				<div class="corp-cov-row" class:corp-cov-player={corp.isPlayer}>
					<span class="cc-col-name">
						<span class="corp-color-dot" style="background: {corp.color};"></span>
						<span class="corp-name-text" class:corp-name-player={corp.isPlayer}>{corp.name}</span>
						{#if corp.isPlayer}
							<span class="corp-you-badge">YOU</span>
						{/if}
					</span>
					<span class="cc-col-nodes mono">{corp.nodeCount}</span>
					<span class="cc-col-edges mono">{corp.edgeCount}</span>
					<span class="cc-col-bw mono">{fmtTraffic(corp.totalBandwidth)}</span>
					<span class="cc-col-util mono" style="color: {utilColor(corp.avgUtilization)};">
						{(corp.avgUtilization * 100).toFixed(0)}%
					</span>
					<span class="cc-col-share">
						{#if totalInfraCount > 0}
							<div class="share-bar-bg">
								<div class="share-bar-fill" style="width: {((corp.nodeCount + corp.edgeCount) / totalInfraCount) * 100}%; background: {corp.color};"></div>
							</div>
							<span class="mono" style="color: {corp.color}; font-size: 9px;">
								{(((corp.nodeCount + corp.edgeCount) / totalInfraCount) * 100).toFixed(0)}%
							</span>
						{/if}
					</span>
				</div>
			{/each}
		{:else}
			<div class="chart-empty">No infrastructure data available</div>
		{/if}
	</section>
</div>

<style>
	.panel {
		padding: 0;
		color: var(--text-secondary);
		font-family: var(--font-sans);
		font-size: 13px;
	}

	/* -- Header Stats Row -- */
	.stats-row {
		display: grid;
		grid-template-columns: 1fr 1fr 1fr 1fr;
		gap: 8px;
		padding: 12px 16px;
		border-bottom: 1px solid var(--border);
	}

	.stat-card {
		background: rgba(30, 41, 59, 0.8);
		border: 1px solid rgba(255, 255, 255, 0.08);
		border-radius: var(--radius-md);
		padding: 10px 12px;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.stat-label {
		font-size: 10px;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: var(--text-dim);
		font-weight: 600;
	}

	.stat-value {
		font-size: 18px;
		font-weight: 700;
		color: var(--text-primary);
		line-height: 1.2;
	}

	.stat-sep {
		color: var(--text-dim);
		font-weight: 400;
		font-size: 14px;
	}

	.stat-sub {
		font-size: 10px;
		font-weight: 500;
	}

	/* -- Sections -- */
	.section {
		padding: 12px 16px;
		border-bottom: 1px solid var(--border);
	}

	h3 {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-dim);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin-bottom: 8px;
	}

	.mono {
		font-family: var(--font-mono);
	}

	.muted {
		color: var(--text-dim);
	}

	/* -- Charts -- */
	.chart-wrapper {
		position: relative;
	}

	.chart {
		width: 100%;
		max-width: 100%;
		height: auto;
	}

	.chart-empty {
		color: var(--text-dim);
		font-size: 11px;
		text-align: center;
		padding: 12px 0;
	}

	.util-bar-bg {
		flex: 1;
		height: 6px;
		background: rgba(55, 65, 81, 0.4);
		border-radius: 3px;
		overflow: hidden;
	}

	.util-bar-fill {
		height: 100%;
		border-radius: 3px;
		transition: width 0.3s;
	}

	.view-btn {
		background: rgba(59, 130, 246, 0.15);
		border: 1px solid rgba(59, 130, 246, 0.3);
		color: var(--blue);
		padding: 2px 8px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 10px;
		font-weight: 600;
		transition: all 0.15s;
	}

	.view-btn:hover {
		background: rgba(59, 130, 246, 0.25);
		border-color: rgba(59, 130, 246, 0.5);
	}

	/* -- Infra Summary -- */
	.infra-summary-stats {
		display: flex;
		gap: 8px;
		align-items: center;
		font-size: 11px;
		color: var(--text-muted);
		margin-bottom: 8px;
	}

	.dot {
		width: 3px;
		height: 3px;
		border-radius: 50%;
		background: var(--text-dim);
	}

	/* ── Revenue by Infrastructure ──────────────────────────────────────── */

	.rev-table-header {
		display: grid;
		grid-template-columns: 2fr 0.6fr 1fr 0.8fr;
		gap: 6px;
		padding: 4px 0 6px;
		font-size: 10px;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: var(--text-dim);
		font-weight: 600;
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
	}

	.rev-table-row {
		display: grid;
		grid-template-columns: 2fr 0.6fr 1fr 0.8fr;
		gap: 6px;
		padding: 5px 4px;
		align-items: center;
		border-bottom: 1px solid rgba(55, 65, 81, 0.12);
		font-size: 11px;
	}

	.rev-table-row:hover {
		background: rgba(55, 65, 81, 0.12);
	}

	.rev-col-type {
		color: var(--text-primary);
		font-weight: 500;
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.rev-col-count {
		text-align: center;
		color: var(--text-muted);
		font-size: 10px;
	}

	.rev-col-rev {
		text-align: right;
		font-size: 11px;
	}

	.rev-col-avg {
		text-align: right;
		color: var(--text-muted);
		font-size: 10px;
	}

	.rev-category-badge {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 16px;
		height: 16px;
		border-radius: 3px;
		font-size: 9px;
		font-weight: 700;
		flex-shrink: 0;
	}

	.rev-node {
		background: rgba(59, 130, 246, 0.2);
		color: #3b82f6;
		border: 1px solid rgba(59, 130, 246, 0.3);
	}

	.rev-edge {
		background: rgba(168, 85, 247, 0.2);
		color: #a855f7;
		border: 1px solid rgba(168, 85, 247, 0.3);
	}

	/* ── SLA Monitoring ─────────────────────────────────────────────────── */

	.sla-table-header {
		display: grid;
		grid-template-columns: 1.5fr 0.7fr 0.8fr 0.7fr 0.8fr;
		gap: 4px;
		padding: 4px 0 6px;
		font-size: 9px;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: var(--text-dim);
		font-weight: 600;
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
	}

	.sla-table-row {
		display: grid;
		grid-template-columns: 1.5fr 0.7fr 0.8fr 0.7fr 0.8fr;
		gap: 4px;
		padding: 5px 0;
		align-items: center;
		border-bottom: 1px solid rgba(55, 65, 81, 0.12);
		font-size: 11px;
	}

	.sla-table-row:hover {
		background: rgba(55, 65, 81, 0.12);
	}

	.sla-col-partner {
		color: var(--text-primary);
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.sla-col-target, .sla-col-perf, .sla-col-penalty {
		font-size: 10px;
		text-align: center;
	}

	.sla-col-status {
		text-align: center;
	}

	.sla-badge {
		display: inline-block;
		padding: 1px 6px;
		border-radius: 3px;
		font-size: 9px;
		font-weight: 700;
		letter-spacing: 0.3px;
	}

	/* ── Maintenance Queue ──────────────────────────────────────────────── */

	.maint-summary {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 11px;
		color: var(--text-muted);
		margin-bottom: 8px;
	}

	.repair-all-btn {
		margin-left: auto;
		background: rgba(239, 68, 68, 0.15);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #ef4444;
		padding: 3px 10px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 10px;
		font-weight: 600;
		transition: all 0.15s;
	}

	.repair-all-btn:hover {
		background: rgba(239, 68, 68, 0.25);
		border-color: rgba(239, 68, 68, 0.5);
	}

	.maint-table-header {
		display: grid;
		grid-template-columns: 1.5fr 1.2fr 0.8fr 0.7fr 1fr;
		gap: 4px;
		padding: 4px 0 6px;
		font-size: 9px;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: var(--text-dim);
		font-weight: 600;
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
	}

	.maint-table-row {
		display: grid;
		grid-template-columns: 1.5fr 1.2fr 0.8fr 0.7fr 1fr;
		gap: 4px;
		padding: 4px 0;
		align-items: center;
		border-bottom: 1px solid rgba(55, 65, 81, 0.12);
		font-size: 11px;
	}

	.maint-table-row:hover {
		background: rgba(55, 65, 81, 0.12);
	}

	.maint-col-type {
		color: var(--text-primary);
		font-weight: 500;
		display: flex;
		align-items: center;
		gap: 4px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.maint-col-health {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.health-bar-bg {
		flex: 1;
		height: 5px;
		background: rgba(55, 65, 81, 0.4);
		border-radius: 3px;
		overflow: hidden;
		min-width: 40px;
	}

	.health-bar-fill {
		height: 100%;
		border-radius: 3px;
		transition: width 0.3s;
	}

	.maint-col-status {
		font-weight: 500;
	}

	.maint-col-cost {
		color: var(--text-muted);
	}

	.maint-col-action {
		display: flex;
		gap: 2px;
		justify-content: flex-end;
	}

	.repair-btn {
		background: rgba(16, 185, 129, 0.15);
		border: 1px solid rgba(16, 185, 129, 0.3);
		color: #10b981;
		padding: 2px 8px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 10px;
		font-weight: 600;
		transition: all 0.15s;
	}

	.repair-btn:hover {
		background: rgba(16, 185, 129, 0.25);
		border-color: rgba(16, 185, 129, 0.5);
	}

	/* ── Capacity Planning ──────────────────────────────────────────────── */

	.cap-stats-row {
		display: flex;
		gap: 16px;
		margin-bottom: 8px;
	}

	.cap-stat {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.cap-stat-label {
		font-size: 9px;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: var(--text-dim);
		font-weight: 600;
	}

	.cap-stat-value {
		font-size: 14px;
		font-weight: 700;
		color: var(--text-primary);
	}

	.cap-warning {
		background: rgba(239, 68, 68, 0.12);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: var(--radius-sm);
		padding: 6px 10px;
		font-size: 11px;
		font-weight: 600;
		color: #ef4444;
		margin-bottom: 8px;
	}

	.cap-subtitle {
		font-size: 10px;
		font-weight: 600;
		color: var(--text-dim);
		text-transform: uppercase;
		letter-spacing: 0.3px;
		margin: 10px 0 6px;
	}

	.cap-edge-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 3px 0;
		font-size: 11px;
	}

	.cap-edge-type {
		flex: 1;
		color: var(--text-primary);
		font-weight: 500;
	}

	.cap-edge-util {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.whatif-section {
		margin-top: 12px;
		padding-top: 10px;
		border-top: 1px solid rgba(55, 65, 81, 0.2);
	}

	.whatif-header {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 6px;
	}

	.whatif-value {
		color: #3b82f6;
		font-size: 13px;
		font-weight: 700;
	}

	.whatif-slider {
		width: 100%;
		height: 4px;
		-webkit-appearance: none;
		appearance: none;
		background: rgba(55, 65, 81, 0.4);
		border-radius: 2px;
		outline: none;
		margin-bottom: 8px;
	}

	.whatif-slider::-webkit-slider-thumb {
		-webkit-appearance: none;
		appearance: none;
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: #3b82f6;
		cursor: pointer;
		border: 2px solid #1e3a5f;
	}

	.whatif-slider::-moz-range-thumb {
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: #3b82f6;
		cursor: pointer;
		border: 2px solid #1e3a5f;
	}

	.whatif-results {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.whatif-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 3px 0;
		font-size: 11px;
	}

	.whatif-type {
		flex: 1;
		color: var(--text-primary);
		font-weight: 500;
	}

	/* ── Traffic OD Matrix ──────────────────────────────────────────────── */

	.od-matrix-wrapper {
		overflow-x: auto;
		margin-bottom: 6px;
	}

	.od-matrix {
		border-collapse: collapse;
		font-size: 9px;
		width: 100%;
	}

	.od-matrix th, .od-matrix td {
		padding: 3px 4px;
		text-align: center;
		border: 1px solid rgba(55, 65, 81, 0.2);
	}

	.od-corner {
		background: transparent;
		border-color: transparent;
	}

	.od-col-header {
		color: var(--text-dim);
		font-weight: 600;
		font-size: 8px;
		text-transform: uppercase;
		writing-mode: vertical-lr;
		transform: rotate(180deg);
		height: 60px;
		vertical-align: bottom;
		background: rgba(30, 41, 59, 0.5);
	}

	.od-row-header {
		color: var(--text-dim);
		font-weight: 600;
		font-size: 8px;
		text-transform: uppercase;
		text-align: left;
		background: rgba(30, 41, 59, 0.5);
		white-space: nowrap;
		max-width: 70px;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.od-cell {
		font-family: var(--font-mono);
		font-size: 8px;
		color: rgba(255, 255, 255, 0.8);
		min-width: 32px;
		transition: background 0.2s;
	}

	.od-cell:hover {
		outline: 1px solid rgba(255, 255, 255, 0.4);
	}

	.od-legend {
		display: flex;
		gap: 12px;
		justify-content: center;
		font-size: 9px;
		color: var(--text-dim);
	}

	.od-legend-item {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.od-swatch {
		display: inline-block;
		width: 10px;
		height: 10px;
		border-radius: 2px;
	}

	/* ── Clickable rows & OD headers (fly-to-map) ─────────────────────── */

	.clickable-row {
		cursor: pointer;
		transition: background 0.12s;
	}

	.clickable-row:hover {
		background: rgba(59, 130, 246, 0.1) !important;
	}

	.od-clickable {
		cursor: pointer;
		transition: color 0.12s, background 0.12s;
	}

	.od-clickable:hover {
		color: #60a5fa !important;
		background: rgba(59, 130, 246, 0.15) !important;
	}

	/* ── What-If Capacity Analysis (Phase 10.2.8) ─────────────────────── */

	.whatif-analysis-section {
		background: rgba(15, 23, 42, 0.6);
	}

	.whatif-analysis-controls {
		margin-bottom: 10px;
	}

	.whatif-analysis-label {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 6px;
	}

	.whatif-analysis-label-text {
		font-size: 11px;
		color: var(--text-muted);
		font-weight: 500;
	}

	.whatif-analysis-pct {
		font-size: 16px;
		font-weight: 700;
		color: #3b82f6;
	}

	.whatif-analysis-slider {
		width: 100%;
		height: 4px;
		-webkit-appearance: none;
		appearance: none;
		background: linear-gradient(to right, rgba(16, 185, 129, 0.4), rgba(245, 158, 11, 0.5), rgba(239, 68, 68, 0.5));
		border-radius: 2px;
		outline: none;
		margin-bottom: 2px;
	}

	.whatif-analysis-slider::-webkit-slider-thumb {
		-webkit-appearance: none;
		appearance: none;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: #3b82f6;
		cursor: pointer;
		border: 2px solid #0f172a;
		box-shadow: 0 0 6px rgba(59, 130, 246, 0.4);
	}

	.whatif-analysis-slider::-moz-range-thumb {
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: #3b82f6;
		cursor: pointer;
		border: 2px solid #0f172a;
		box-shadow: 0 0 6px rgba(59, 130, 246, 0.4);
	}

	.whatif-analysis-ticks-row {
		display: flex;
		justify-content: space-between;
		padding: 0 2px;
		margin-bottom: 8px;
	}

	.whatif-tick-label {
		font-size: 8px;
		color: var(--text-dim);
		font-family: var(--font-mono);
	}

	.whatif-analysis-summary {
		font-size: 11px;
		color: var(--text-muted);
		margin-bottom: 8px;
		padding: 5px 8px;
		background: rgba(30, 41, 59, 0.6);
		border-radius: var(--radius-sm);
		border: 1px solid rgba(55, 65, 81, 0.3);
	}

	.whatif-table-header {
		display: grid;
		grid-template-columns: 1.2fr 1.5fr 0.6fr 0.8fr 0.4fr;
		gap: 4px;
		padding: 4px 4px 6px;
		font-size: 9px;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: var(--text-dim);
		font-weight: 700;
		font-family: var(--font-mono);
		border-bottom: 1px solid rgba(55, 65, 81, 0.4);
		background: rgba(15, 23, 42, 0.8);
	}

	.whatif-table-row {
		display: grid;
		grid-template-columns: 1.2fr 1.5fr 0.6fr 0.8fr 0.4fr;
		gap: 4px;
		padding: 5px 4px;
		align-items: center;
		border-bottom: 1px solid rgba(55, 65, 81, 0.15);
		font-size: 11px;
		transition: background 0.12s;
	}

	.whatif-table-row:hover {
		background: rgba(55, 65, 81, 0.15);
	}

	.wi-col-type {
		color: var(--text-primary);
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.wi-col-route {
		font-size: 10px;
		color: var(--text-muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.wi-col-util {
		text-align: center;
		font-size: 11px;
		font-weight: 600;
	}

	.wi-col-ttx {
		text-align: center;
		font-size: 11px;
		font-weight: 700;
	}

	.wi-col-action {
		text-align: right;
	}

	.whatif-legend {
		display: flex;
		gap: 10px;
		justify-content: center;
		margin-top: 8px;
		font-size: 9px;
		color: var(--text-dim);
		font-family: var(--font-mono);
	}

	.whatif-legend-item {
		display: flex;
		align-items: center;
		gap: 3px;
	}

	.whatif-swatch {
		display: inline-block;
		width: 8px;
		height: 8px;
		border-radius: 2px;
	}

	/* ── Congested Edges Table (Phase 10.2.3) ─────────────────────────── */

	.congested-table-header {
		display: grid;
		grid-template-columns: 1.2fr 1.3fr 1fr 0.5fr 0.4fr;
		gap: 6px;
		padding: 4px 0 6px;
		font-size: 10px;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: var(--text-dim);
		font-weight: 600;
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
	}

	.congested-table-row {
		display: grid;
		grid-template-columns: 1.2fr 1.3fr 1fr 0.5fr 0.4fr;
		gap: 6px;
		padding: 6px 0;
		align-items: center;
		border-bottom: 1px solid rgba(55, 65, 81, 0.15);
		font-size: 12px;
	}

	.congested-table-row:hover {
		background: rgba(55, 65, 81, 0.15);
	}

	.cg-col-type {
		color: var(--text-primary);
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.cg-col-util {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.cg-col-suggest {
		display: flex;
		align-items: center;
	}

	.cg-col-owner {
		font-size: 11px;
		color: var(--text-muted);
	}

	.cg-col-action {
		text-align: right;
	}

	.suggest-badge {
		display: inline-block;
		padding: 1px 6px;
		border-radius: 3px;
		font-size: 9px;
		font-weight: 700;
		font-family: var(--font-mono);
		letter-spacing: 0.3px;
	}

	.suggest-upgrade {
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
		border: 1px solid rgba(239, 68, 68, 0.35);
	}

	.suggest-monitor {
		background: rgba(245, 158, 11, 0.15);
		color: #f59e0b;
		border: 1px solid rgba(245, 158, 11, 0.35);
	}

	.suggest-detail {
		font-size: 10px;
		font-family: var(--font-mono);
		padding: 2px 0 4px 12px;
		border-bottom: 1px solid rgba(55, 65, 81, 0.1);
		opacity: 0.85;
	}

	/* ── Corporation Coverage Comparison (Phase 10.2.6) ───────────────── */

	.corp-cov-header {
		display: grid;
		grid-template-columns: 2fr 0.5fr 0.5fr 0.7fr 0.6fr 1fr;
		gap: 4px;
		padding: 4px 0 6px;
		font-size: 9px;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: var(--text-dim);
		font-weight: 600;
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
	}

	.corp-cov-row {
		display: grid;
		grid-template-columns: 2fr 0.5fr 0.5fr 0.7fr 0.6fr 1fr;
		gap: 4px;
		padding: 5px 0;
		align-items: center;
		border-bottom: 1px solid rgba(55, 65, 81, 0.12);
		font-size: 11px;
	}

	.corp-cov-row:hover {
		background: rgba(55, 65, 81, 0.12);
	}

	.corp-cov-player {
		background: rgba(59, 130, 246, 0.06);
	}

	.cc-col-name {
		display: flex;
		align-items: center;
		gap: 6px;
		overflow: hidden;
	}

	.corp-color-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.corp-name-text {
		color: var(--text-primary);
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.corp-name-player {
		color: #60a5fa;
		font-weight: 600;
	}

	.corp-you-badge {
		display: inline-block;
		padding: 0 4px;
		border-radius: 2px;
		font-size: 8px;
		font-weight: 700;
		font-family: var(--font-mono);
		background: rgba(59, 130, 246, 0.2);
		color: #60a5fa;
		border: 1px solid rgba(59, 130, 246, 0.3);
		flex-shrink: 0;
	}

	.cc-col-nodes, .cc-col-edges {
		text-align: center;
		color: var(--text-muted);
		font-size: 10px;
	}

	.cc-col-bw {
		text-align: center;
		color: var(--text-muted);
		font-size: 10px;
	}

	.cc-col-util {
		text-align: center;
		font-size: 10px;
		font-weight: 600;
	}

	.cc-col-share {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.share-bar-bg {
		flex: 1;
		height: 6px;
		background: rgba(55, 65, 81, 0.4);
		border-radius: 3px;
		overflow: hidden;
		min-width: 30px;
	}

	.share-bar-fill {
		height: 100%;
		border-radius: 3px;
		transition: width 0.3s;
	}
</style>
