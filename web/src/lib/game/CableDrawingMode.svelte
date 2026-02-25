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

	/** Push cable drawing state to MapRenderer via custom event. */
	function pushStateToMap(): void {
		const state: CableDrawingState = {
			waypoints: [...waypoints],
			cursorPosition,
			deployment: deploymentMethod,
			sourceNodePos,
			isDrawing,
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

		// Add waypoint at click position
		waypoints = [...waypoints, [lon, lat]];
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

	function handleMapMouseMove(e: Event): void {
		if (!isActive || !isDrawing) return;
		const detail = (e as CustomEvent).detail;
		if (!detail) return;

		cursorPosition = [detail.lon as number, detail.lat as number];
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
	});

	onDestroy(() => {
		window.removeEventListener('entity-selected', handleEntitySelected, true);
		window.removeEventListener('map-clicked', handleMapClicked, true);
		window.removeEventListener('map-dblclick', handleMapDblClick);
		window.removeEventListener('map-mousemove', handleMapMouseMove);
		window.removeEventListener('map-contextmenu', handleMapContextMenu, true);
		window.removeEventListener('keydown', handleKeyDown, true);
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
