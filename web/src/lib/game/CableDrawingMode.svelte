<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { get } from 'svelte/store';
	import {
		buildCategory,
		selectedBuildItem,
		selectedEdgeType,
		buildMode,
		exitPlacementMode,
	} from '$lib/stores/uiState';
	import { formatMoney } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import type { CableDrawingState } from './map/layers/cablePreviewLayer';

	// ── Drawing state ────────────────────────────────────────────────────
	let waypoints: [number, number][] = $state([]);
	let sourceNodeId: number | null = $state(null);
	let sourceNodePos: [number, number] | null = $state(null);
	let deploymentMethod: 'Aerial' | 'Underground' = $state('Underground');
	let isDrawing: boolean = $state(false);
	let cursorPosition: [number, number] | null = $state(null);

	// ── Road snap state ─────────────────────────────────────────────────
	let roadSnapPosition: [number, number] | null = $state(null);
	let roadSnapSegment: [[number, number], [number, number]] | null = $state(null);
	let isSnappedToRoad: boolean = $state(false);
	let shiftHeld: boolean = $state(false);

	// Cached road segments for snap detection (loaded once when drawing starts)
	let cachedRoadSegments: bridge.RoadSegmentInfo[] | null = $state(null);

	// ── Auto-route state ────────────────────────────────────────────────
	let autoRouteWaypoints: [number, number][] | null = $state(null);
	let autoRouteCost: number | null = $state(null);
	let directRouteCost: number | null = $state(null);
	let autoRouteAvailable: boolean = $state(false);

	// Whether cable drawing mode is active (edge build mode + item selected)
	let isActive = $derived($buildCategory === 'edge' && $selectedBuildItem !== null);

	// Edge type cost per km (approximate — from Rust InfraEdge::new)
	const EDGE_COST_PER_KM: Record<string, number> = {
		Copper: 10000,
		FiberLocal: 20000,
		Microwave: 20000,
		FiberRegional: 40000,
		FiberNational: 80000,
		Satellite: 0, // flat cost
		Submarine: 200000,
		TelegraphWire: 500,
		SubseaTelegraphCable: 5000,
		CopperTrunkLine: 3000,
		LongDistanceCopper: 8000,
		CoaxialCable: 8000,
		MicrowaveLink: 15000,
		EarlySatelliteLink: 0,
		SubseaFiberCable: 250000,
		FiberMetro: 50000,
		FiberLongHaul: 100000,
		DWDM_Backbone: 150000,
		SatelliteLEOLink: 0,
		FeederFiber: 30000,
		DistributionFiber: 15000,
		DropCable: 2000,
		QuantumFiberLink: 500000,
		TerahertzBeam: 100000,
		LaserInterSatelliteLink: 0,
	};

	// Flat cost for satellite-type edges
	const EDGE_FLAT_COST: Record<string, number> = {
		Satellite: 5000000,
		EarlySatelliteLink: 2000000,
		SatelliteLEOLink: 10000000,
		LaserInterSatelliteLink: 50000000,
	};

	// Display names for edge types
	const EDGE_NAMES: Record<string, string> = {
		Copper: 'Copper',
		FiberLocal: 'Fiber Local',
		FiberRegional: 'Fiber Regional',
		FiberNational: 'Fiber National',
		Microwave: 'Microwave',
		Satellite: 'Satellite',
		Submarine: 'Submarine',
		TelegraphWire: 'Telegraph Wire',
		SubseaTelegraphCable: 'Subsea Telegraph',
		CopperTrunkLine: 'Copper Trunk',
		LongDistanceCopper: 'Long Distance Copper',
		CoaxialCable: 'Coaxial Cable',
		MicrowaveLink: 'Microwave Link',
		EarlySatelliteLink: 'Early Satellite',
		SubseaFiberCable: 'Subsea Fiber',
		FiberMetro: 'Fiber Metro',
		FiberLongHaul: 'Fiber Long Haul',
		DWDM_Backbone: 'DWDM Backbone',
		SatelliteLEOLink: 'LEO Satellite',
		FeederFiber: 'Feeder Fiber',
		DistributionFiber: 'Distribution Fiber',
		DropCable: 'Drop Cable',
		QuantumFiberLink: 'Quantum Fiber',
		TerahertzBeam: 'Terahertz Beam',
		LaserInterSatelliteLink: 'Laser Inter-Satellite',
	};

	// ── Road snap constants ──────────────────────────────────────────────
	// Snap threshold in degrees (roughly ~50px at zoom 5-6; becomes tighter at higher zoom)
	// At zoom 5: ~0.5 deg per 50px, at zoom 8: ~0.06 deg per 50px
	const SNAP_THRESHOLD_DEG = 0.15;

	// ── Derived values ───────────────────────────────────────────────────

	let totalLengthKm = $derived.by(() => {
		if (waypoints.length < 2) return 0;
		let total = 0;
		for (let i = 1; i < waypoints.length; i++) {
			total += haversineKm(waypoints[i - 1], waypoints[i]);
		}
		return total;
	});

	let runningCost = $derived.by(() => {
		const edgeType = $selectedBuildItem ?? $selectedEdgeType;
		const flatCost = EDGE_FLAT_COST[edgeType];
		if (flatCost !== undefined) return flatCost;
		const costPerKm = EDGE_COST_PER_KM[edgeType] ?? 20000;
		return Math.round(costPerKm * totalLengthKm);
	});

	let edgeDisplayName = $derived(EDGE_NAMES[$selectedBuildItem ?? $selectedEdgeType] ?? ($selectedBuildItem ?? $selectedEdgeType));

	// ── Helpers ──────────────────────────────────────────────────────────

	/** Haversine distance between two [lon, lat] points in km. */
	function haversineKm(a: [number, number], b: [number, number]): number {
		const toRad = Math.PI / 180;
		const dlat = (b[1] - a[1]) * toRad;
		const dlon = (b[0] - a[0]) * toRad;
		const lat1 = a[1] * toRad;
		const lat2 = b[1] * toRad;
		const sinDlat = Math.sin(dlat / 2);
		const sinDlon = Math.sin(dlon / 2);
		const aVal = sinDlat * sinDlat + Math.cos(lat1) * Math.cos(lat2) * sinDlon * sinDlon;
		return 6371 * 2 * Math.asin(Math.sqrt(aVal));
	}

	/** Compute cost for a given edge type and distance in km. */
	function computeRouteCost(lengthKm: number): number {
		const edgeType = $selectedBuildItem ?? $selectedEdgeType;
		const flatCost = EDGE_FLAT_COST[edgeType];
		if (flatCost !== undefined) return flatCost;
		const costPerKm = EDGE_COST_PER_KM[edgeType] ?? 20000;
		return Math.round(costPerKm * lengthKm);
	}

	/** Push cable drawing state to MapRenderer via custom event. */
	function pushStateToMap(): void {
		const state: CableDrawingState = {
			waypoints: [...waypoints],
			cursorPosition: isSnappedToRoad && roadSnapPosition ? roadSnapPosition : cursorPosition,
			deployment: deploymentMethod,
			sourceNodePos,
			isDrawing,
			roadSnapPosition,
			roadSnapSegment,
			isSnappedToRoad,
			autoRouteWaypoints,
			autoRouteCost,
			directRouteCost,
			autoRouteAvailable,
		};
		window.dispatchEvent(new CustomEvent('cable-drawing-update', { detail: state }));
	}

	/** Reset all drawing state. */
	function resetDrawing(): void {
		waypoints = [];
		sourceNodeId = null;
		sourceNodePos = null;
		isDrawing = false;
		cursorPosition = null;
		roadSnapPosition = null;
		roadSnapSegment = null;
		isSnappedToRoad = false;
		autoRouteWaypoints = null;
		autoRouteCost = null;
		directRouteCost = null;
		autoRouteAvailable = false;
		cachedRoadSegments = null;
		pushStateToMap();
	}

	// ── Road snap logic ──────────────────────────────────────────────────

	/** Load road segments for snap detection (called once when drawing starts). */
	function ensureRoadSegmentsLoaded(): void {
		if (cachedRoadSegments === null) {
			cachedRoadSegments = bridge.getRoadSegments();
		}
	}

	/**
	 * Find the nearest point on a line segment to a given point.
	 * Returns the projected point and the distance.
	 */
	function projectToSegment(
		px: number, py: number,
		ax: number, ay: number,
		bx: number, by: number,
	): { point: [number, number]; dist: number } {
		const dx = bx - ax;
		const dy = by - ay;
		const lenSq = dx * dx + dy * dy;
		if (lenSq === 0) {
			const d = Math.sqrt((px - ax) ** 2 + (py - ay) ** 2);
			return { point: [ax, ay], dist: d };
		}
		const t = Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / lenSq));
		const projX = ax + t * dx;
		const projY = ay + t * dy;
		const d = Math.sqrt((px - projX) ** 2 + (py - projY) ** 2);
		return { point: [projX, projY], dist: d };
	}

	/**
	 * Find the nearest road segment to cursor and compute snap position.
	 * Returns snap info if within threshold, or null if too far.
	 */
	function computeRoadSnap(
		lon: number, lat: number,
	): { position: [number, number]; segment: [[number, number], [number, number]]; dist: number } | null {
		if (!cachedRoadSegments || cachedRoadSegments.length === 0) return null;

		let bestDist = Infinity;
		let bestPoint: [number, number] = [0, 0];
		let bestSeg: [[number, number], [number, number]] = [[0, 0], [0, 0]];

		for (const seg of cachedRoadSegments) {
			const result = projectToSegment(
				lon, lat,
				seg.from[0], seg.from[1],
				seg.to[0], seg.to[1],
			);
			if (result.dist < bestDist) {
				bestDist = result.dist;
				bestPoint = result.point;
				bestSeg = [seg.from, seg.to];
			}
		}

		if (bestDist <= SNAP_THRESHOLD_DEG) {
			return { position: bestPoint, segment: bestSeg, dist: bestDist };
		}
		return null;
	}

	// ── Auto-route logic ─────────────────────────────────────────────────

	/** Compute auto-route along roads from the last waypoint to cursor position. */
	function computeAutoRoute(): void {
		if (!isDrawing || waypoints.length === 0 || !cursorPosition) {
			autoRouteWaypoints = null;
			autoRouteCost = null;
			directRouteCost = null;
			autoRouteAvailable = false;
			return;
		}

		const lastWp = waypoints[waypoints.length - 1];
		const target = cursorPosition;

		// Only compute auto-route when we have at least a source position
		// and the cursor is far enough away to matter (> ~1km)
		const dist = haversineKm(lastWp, target);
		if (dist < 0.5) {
			autoRouteWaypoints = null;
			autoRouteCost = null;
			directRouteCost = null;
			autoRouteAvailable = false;
			return;
		}

		// Get road pathfinding result
		const roadPath = bridge.roadPathfind(lastWp[0], lastWp[1], target[0], target[1]);
		if (roadPath.length >= 2) {
			autoRouteWaypoints = roadPath;

			// Compute road route cost using WASM fiber cost
			const roadCostKm = bridge.roadFiberRouteCost(lastWp[0], lastWp[1], target[0], target[1]);
			autoRouteCost = computeRouteCost(roadCostKm);

			// Compute direct route cost
			directRouteCost = computeRouteCost(dist);
			autoRouteAvailable = true;
		} else {
			// No road path available
			autoRouteWaypoints = null;
			autoRouteCost = null;
			directRouteCost = computeRouteCost(dist);
			autoRouteAvailable = false;
		}
	}

	/** Accept the auto-route: replace manual waypoints with road waypoints. */
	function acceptAutoRoute(): void {
		if (!autoRouteAvailable || !autoRouteWaypoints || autoRouteWaypoints.length < 2) return;

		// Add all road waypoints (skip the first since it's the current last waypoint)
		const newWaypoints = [...waypoints];
		for (let i = 1; i < autoRouteWaypoints.length; i++) {
			newWaypoints.push(autoRouteWaypoints[i]);
		}
		waypoints = newWaypoints;

		// Clear auto-route state
		autoRouteWaypoints = null;
		autoRouteCost = null;
		directRouteCost = null;
		autoRouteAvailable = false;
		pushStateToMap();
	}

	/** Complete the cable and send the BuildEdge command. */
	function completeCable(targetNodeId: number): void {
		if (sourceNodeId === null || waypoints.length < 2) return;

		const edgeType = $selectedBuildItem ?? $selectedEdgeType;
		gameCommand({
			BuildEdge: {
				edge_type: edgeType,
				from: sourceNodeId,
				to: targetNodeId,
				waypoints: waypoints.map(wp => [wp[0], wp[1]]),
				deployment: deploymentMethod,
			},
		});

		// Reset drawing but stay in cable drawing mode for rapid building
		resetDrawing();
		// Trigger map update
		window.dispatchEvent(new CustomEvent('map-dirty'));
	}

	// ── Event handlers ───────────────────────────────────────────────────

	function handleEntitySelected(e: Event): void {
		if (!isActive) return;
		const detail = (e as CustomEvent).detail;
		if (!detail || detail.type !== 'node' || detail.id === null) return;

		const nodeId = detail.id as number;
		const allInfra = bridge.getAllInfrastructure();
		const node = allInfra.nodes.find(n => n.id === nodeId);
		if (!node) return;

		if (!isDrawing) {
			// First click on a node: begin drawing
			sourceNodeId = nodeId;
			sourceNodePos = [node.x, node.y];
			waypoints = [[node.x, node.y]];
			isDrawing = true;
			// Load road segments for snap detection
			ensureRoadSegmentsLoaded();
			pushStateToMap();
			// Prevent the default entity-selected behavior
			e.stopImmediatePropagation();
		} else {
			// Already drawing — this is a node click for completion
			// Double-click on a node completes (handled separately via map-dblclick)
			// Single click on a node during drawing — ignore (waypoints are added via map-clicked)
		}
	}

	function handleMapClicked(e: Event): void {
		if (!isActive || !isDrawing) return;
		const detail = (e as CustomEvent).detail;
		if (!detail) return;

		const lon = detail.lon as number;
		const lat = detail.lat as number;

		// If snapped to road and Shift is NOT held, use the snap position
		if (isSnappedToRoad && roadSnapPosition && !shiftHeld) {
			waypoints = [...waypoints, roadSnapPosition];
		} else {
			// Add waypoint at raw click position
			waypoints = [...waypoints, [lon, lat]];
		}
		pushStateToMap();

		// Prevent default map click behavior
		e.stopImmediatePropagation();
	}

	function handleMapDblClick(e: Event): void {
		if (!isActive || !isDrawing) return;
		const detail = (e as CustomEvent).detail;
		if (!detail) return;

		const pickedNode = detail.pickedNode;
		if (pickedNode && pickedNode.id !== undefined && pickedNode.id !== sourceNodeId) {
			// Double-clicked on a target node — complete cable
			const targetId = pickedNode.id as number;
			const allInfra = bridge.getAllInfrastructure();
			const targetNode = allInfra.nodes.find(n => n.id === targetId);
			if (targetNode) {
				// Add the target node position as the final waypoint
				waypoints = [...waypoints, [targetNode.x, targetNode.y]];
				completeCable(targetId);
			}
		}
	}

	// Throttle auto-route computation to avoid calling WASM too often
	let autoRouteTimer: ReturnType<typeof setTimeout> | null = null;

	function handleMapMouseMove(e: Event): void {
		if (!isActive || !isDrawing) return;
		const detail = (e as CustomEvent).detail;
		if (!detail) return;

		const rawLon = detail.lon as number;
		const rawLat = detail.lat as number;

		cursorPosition = [rawLon, rawLat];

		// Road snap: find nearest road segment (unless Shift held for override)
		if (!shiftHeld) {
			const snap = computeRoadSnap(rawLon, rawLat);
			if (snap) {
				roadSnapPosition = snap.position;
				roadSnapSegment = snap.segment;
				isSnappedToRoad = true;
			} else {
				roadSnapPosition = null;
				roadSnapSegment = null;
				isSnappedToRoad = false;
			}
		} else {
			roadSnapPosition = null;
			roadSnapSegment = null;
			isSnappedToRoad = false;
		}

		// Throttled auto-route computation (every 200ms)
		if (autoRouteTimer) clearTimeout(autoRouteTimer);
		autoRouteTimer = setTimeout(() => {
			computeAutoRoute();
			pushStateToMap();
		}, 200);

		pushStateToMap();
	}

	function handleMapContextMenu(e: Event): void {
		if (!isActive || !isDrawing) return;

		// Right-click: undo last waypoint
		e.stopImmediatePropagation();
		e.preventDefault();

		if (waypoints.length > 1) {
			waypoints = waypoints.slice(0, -1);
			pushStateToMap();
		} else {
			// No waypoints left besides source — cancel drawing
			resetDrawing();
		}
	}

	function handleKeyDown(e: KeyboardEvent): void {
		// Track Shift key state for road snap override
		if (e.key === 'Shift') {
			shiftHeld = true;
		}

		if (!isActive) return;

		if (e.key === 'Escape') {
			if (isDrawing) {
				resetDrawing();
				e.preventDefault();
				e.stopPropagation();
			} else {
				// Exit placement mode entirely
				exitPlacementMode();
				e.preventDefault();
				e.stopPropagation();
			}
		} else if (e.key === 'Tab') {
			// Toggle deployment method
			deploymentMethod = deploymentMethod === 'Aerial' ? 'Underground' : 'Aerial';
			pushStateToMap();
			e.preventDefault();
			e.stopPropagation();
		} else if (e.key === 'Enter' && autoRouteAvailable && isDrawing) {
			// Accept auto-route along roads
			acceptAutoRoute();
			e.preventDefault();
			e.stopPropagation();
		}
	}

	function handleKeyUp(e: KeyboardEvent): void {
		if (e.key === 'Shift') {
			shiftHeld = false;
		}
	}

	// ── Lifecycle ────────────────────────────────────────────────────────

	onMount(() => {
		// Register event listeners with capture phase to intercept before MapView
		window.addEventListener('entity-selected', handleEntitySelected, true);
		window.addEventListener('map-clicked', handleMapClicked, true);
		window.addEventListener('map-dblclick', handleMapDblClick);
		window.addEventListener('map-mousemove', handleMapMouseMove);
		window.addEventListener('map-contextmenu', handleMapContextMenu, true);
		window.addEventListener('keydown', handleKeyDown, true);
		window.addEventListener('keyup', handleKeyUp, true);
	});

	onDestroy(() => {
		window.removeEventListener('entity-selected', handleEntitySelected, true);
		window.removeEventListener('map-clicked', handleMapClicked, true);
		window.removeEventListener('map-dblclick', handleMapDblClick);
		window.removeEventListener('map-mousemove', handleMapMouseMove);
		window.removeEventListener('map-contextmenu', handleMapContextMenu, true);
		window.removeEventListener('keydown', handleKeyDown, true);
		window.removeEventListener('keyup', handleKeyUp, true);
		if (autoRouteTimer) clearTimeout(autoRouteTimer);
		// Clear drawing state on unmount
		resetDrawing();
	});

	// Reset drawing when leaving edge build mode
	$effect(() => {
		if (!isActive && isDrawing) {
			resetDrawing();
		}
	});
</script>

{#if isActive && isDrawing}
	<div class="cable-drawing-hud">
		<div class="hud-section">
			<span class="edge-badge">CABLE</span>
			<span class="edge-name">{edgeDisplayName}</span>
		</div>

		<div class="hud-divider"></div>

		<div class="hud-section">
			<span class="hud-label">Length</span>
			<span class="hud-value mono">{totalLengthKm.toFixed(1)} km</span>
		</div>

		<div class="hud-divider"></div>

		<div class="hud-section">
			<span class="hud-label">Cost</span>
			<span class="hud-value mono cost">{formatMoney(runningCost)}</span>
		</div>

		<div class="hud-divider"></div>

		<div class="hud-section">
			<button
				class="deployment-toggle"
				class:aerial={deploymentMethod === 'Aerial'}
				class:underground={deploymentMethod === 'Underground'}
				onclick={() => {
					deploymentMethod = deploymentMethod === 'Aerial' ? 'Underground' : 'Aerial';
					pushStateToMap();
				}}
			>
				{deploymentMethod === 'Aerial' ? 'Aerial' : 'Underground'}
			</button>
		</div>

		<div class="hud-divider"></div>

		<div class="hud-section">
			<span class="hud-label">Waypoints</span>
			<span class="hud-value mono">{waypoints.length}</span>
		</div>

		{#if isSnappedToRoad}
			<div class="hud-divider"></div>
			<div class="hud-section">
				<span class="snap-badge">ROAD SNAP</span>
			</div>
		{/if}

		{#if autoRouteAvailable && autoRouteCost !== null && directRouteCost !== null}
			<div class="hud-divider"></div>
			<div class="hud-section route-compare">
				<span class="route-label road-route">Road: {formatMoney(autoRouteCost)}</span>
				<span class="route-sep">|</span>
				<span class="route-label direct-route">Direct: {formatMoney(directRouteCost)}</span>
			</div>
		{/if}

		<div class="hud-divider"></div>

		<div class="hud-hints">
			<span class="hint">Click: add waypoint</span>
			<span class="hint-sep">|</span>
			<span class="hint">Dbl-click node: finish</span>
			<span class="hint-sep">|</span>
			<span class="hint">Right-click: undo</span>
			<span class="hint-sep">|</span>
			<span class="hint"><kbd>Esc</kbd> cancel</span>
			<span class="hint-sep">|</span>
			<span class="hint"><kbd>Tab</kbd> toggle deploy</span>
			{#if autoRouteAvailable}
				<span class="hint-sep">|</span>
				<span class="hint autoroute-hint"><kbd>Enter</kbd> accept road route</span>
			{/if}
			<span class="hint-sep">|</span>
			<span class="hint"><kbd>Shift</kbd> off-road</span>
		</div>
	</div>
{/if}

<style>
	.cable-drawing-hud {
		position: absolute;
		top: 84px;
		left: 50%;
		transform: translateX(-50%);
		z-index: 15;
		display: flex;
		align-items: center;
		gap: 0;
		background: rgba(17, 24, 39, 0.92);
		border: 1px solid rgba(251, 191, 36, 0.3);
		border-radius: 8px;
		padding: 6px 12px;
		font-family: var(--font-mono, 'JetBrains Mono', monospace);
		font-size: 12px;
		color: #d1d5db;
		backdrop-filter: blur(8px);
		box-shadow: 0 4px 20px rgba(0, 0, 0, 0.4);
		pointer-events: auto;
		white-space: nowrap;
	}

	.hud-section {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 0 8px;
	}

	.hud-divider {
		width: 1px;
		height: 20px;
		background: rgba(75, 85, 99, 0.5);
		flex-shrink: 0;
	}

	.edge-badge {
		font-size: 9px;
		font-weight: 800;
		letter-spacing: 0.1em;
		padding: 2px 6px;
		border-radius: 3px;
		background: rgba(251, 191, 36, 0.2);
		color: #fbbf24;
	}

	.edge-name {
		font-weight: 600;
		color: #f3f4f6;
		font-size: 11px;
	}

	.hud-label {
		color: #6b7280;
		font-size: 10px;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.hud-value {
		color: #e5e7eb;
		font-weight: 600;
	}

	.hud-value.mono {
		font-family: var(--font-mono, 'JetBrains Mono', monospace);
	}

	.hud-value.cost {
		color: #fbbf24;
	}

	.deployment-toggle {
		background: rgba(55, 65, 81, 0.6);
		border: 1px solid rgba(75, 85, 99, 0.5);
		color: #d1d5db;
		font-size: 10px;
		font-weight: 600;
		font-family: var(--font-mono, 'JetBrains Mono', monospace);
		padding: 2px 10px;
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.12s;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.deployment-toggle:hover {
		background: rgba(75, 85, 99, 0.6);
	}

	.deployment-toggle.aerial {
		border-color: rgba(234, 179, 8, 0.4);
		color: #eab308;
	}

	.deployment-toggle.underground {
		border-color: rgba(59, 130, 246, 0.4);
		color: #60a5fa;
	}

	.snap-badge {
		font-size: 9px;
		font-weight: 800;
		letter-spacing: 0.1em;
		padding: 2px 6px;
		border-radius: 3px;
		background: rgba(0, 255, 200, 0.15);
		color: #00ffc8;
	}

	.route-compare {
		gap: 4px;
	}

	.route-label {
		font-size: 10px;
		font-weight: 600;
		font-family: var(--font-mono, 'JetBrains Mono', monospace);
	}

	.route-label.road-route {
		color: #10b981;
	}

	.route-label.direct-route {
		color: #fbbf24;
	}

	.route-sep {
		color: rgba(75, 85, 99, 0.5);
		font-size: 10px;
	}

	.hud-hints {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 0 4px;
	}

	.hint {
		color: #6b7280;
		font-size: 10px;
	}

	.hint.autoroute-hint {
		color: #10b981;
	}

	.hint-sep {
		color: rgba(75, 85, 99, 0.5);
		font-size: 10px;
	}

	.hint :global(kbd) {
		display: inline-block;
		background: rgba(55, 65, 81, 0.5);
		border: 1px solid rgba(75, 85, 99, 0.4);
		border-radius: 3px;
		padding: 0 4px;
		font-family: monospace;
		font-size: 9px;
		color: #d1d5db;
	}
</style>
