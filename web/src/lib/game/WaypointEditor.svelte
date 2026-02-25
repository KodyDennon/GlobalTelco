<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { get } from 'svelte/store';
	import { formatMoney } from '$lib/stores/gameState';
	import { buildMode, buildCategory } from '$lib/stores/uiState';
	import { gameCommand } from '$lib/game/commandRouter';
	import * as bridge from '$lib/wasm/bridge';
	import type { AllInfraEdge } from '$lib/wasm/types';

	// ── State ────────────────────────────────────────────────────────────
	let editing: boolean = $state(false);
	let edgeId: number | null = $state(null);
	let edgeData: AllInfraEdge | null = $state(null);
	let waypoints: [number, number][] = $state([]);
	let originalWaypoints: [number, number][] = $state([]);
	let draggingIndex: number | null = $state(null);
	let cursorPosition: [number, number] | null = $state(null);

	// Edge type cost per km (mirror from CableDrawingMode)
	const EDGE_COST_PER_KM: Record<string, number> = {
		Copper: 10000,
		FiberLocal: 20000,
		Microwave: 20000,
		FiberRegional: 40000,
		FiberNational: 80000,
		Satellite: 0,
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

	// ── Derived values ───────────────────────────────────────────────────

	let totalLengthKm = $derived.by(() => {
		if (waypoints.length < 2) return 0;
		let total = 0;
		for (let i = 1; i < waypoints.length; i++) {
			total += haversineKm(waypoints[i - 1], waypoints[i]);
		}
		return total;
	});

	let estimatedCost = $derived.by(() => {
		if (!edgeData) return 0;
		const costPerKm = EDGE_COST_PER_KM[edgeData.edge_type] ?? 20000;
		return Math.round(costPerKm * totalLengthKm);
	});

	// ── Helpers ──────────────────────────────────────────────────────────

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

	/** Push waypoint editor state to MapRenderer via custom event. */
	function pushStateToMap(): void {
		window.dispatchEvent(new CustomEvent('waypoint-editor-update', {
			detail: {
				editing,
				edgeId,
				waypoints: [...waypoints],
				draggingIndex,
				cursorPosition,
			}
		}));
	}

	/** Find the closest segment and point on it for inserting a new waypoint. */
	function findInsertPosition(lon: number, lat: number): number {
		if (waypoints.length < 2) return waypoints.length;
		let bestIdx = 1;
		let bestDist = Infinity;
		for (let i = 0; i < waypoints.length - 1; i++) {
			const a = waypoints[i];
			const b = waypoints[i + 1];
			const dist = pointToSegmentDist(lon, lat, a, b);
			if (dist < bestDist) {
				bestDist = dist;
				bestIdx = i + 1;
			}
		}
		return bestIdx;
	}

	function pointToSegmentDist(px: number, py: number, a: [number, number], b: [number, number]): number {
		const dx = b[0] - a[0];
		const dy = b[1] - a[1];
		const lenSq = dx * dx + dy * dy;
		if (lenSq === 0) return Math.sqrt((px - a[0]) ** 2 + (py - a[1]) ** 2);
		const t = Math.max(0, Math.min(1, ((px - a[0]) * dx + (py - a[1]) * dy) / lenSq));
		const projX = a[0] + t * dx;
		const projY = a[1] + t * dy;
		return Math.sqrt((px - projX) ** 2 + (py - projY) ** 2);
	}

	// ── Actions ──────────────────────────────────────────────────────────

	function startEditing(edge: AllInfraEdge): void {
		edgeId = edge.id;
		edgeData = edge;
		// Use existing waypoints, or fall back to source/target endpoints
		if (edge.waypoints && edge.waypoints.length >= 2) {
			waypoints = edge.waypoints.map(wp => [wp[0], wp[1]] as [number, number]);
		} else {
			waypoints = [
				[edge.src_x, edge.src_y],
				[edge.dst_x, edge.dst_y],
			];
		}
		originalWaypoints = waypoints.map(wp => [wp[0], wp[1]] as [number, number]);
		editing = true;
		pushStateToMap();
	}

	function confirmEdit(): void {
		if (edgeId === null || waypoints.length < 2) return;
		gameCommand({
			UpdateEdgeWaypoints: {
				edge: edgeId,
				waypoints: waypoints.map(wp => [wp[0], wp[1]]),
			},
		});
		cancelEdit();
		window.dispatchEvent(new CustomEvent('map-dirty'));
	}

	function cancelEdit(): void {
		editing = false;
		edgeId = null;
		edgeData = null;
		waypoints = [];
		originalWaypoints = [];
		draggingIndex = null;
		cursorPosition = null;
		pushStateToMap();
	}

	// ── Event handlers ───────────────────────────────────────────────────

	function handleEdgeEditStart(e: Event): void {
		const detail = (e as CustomEvent).detail;
		if (!detail || !detail.edge) return;
		// Do not start editing if we're in build mode
		if (get(buildMode) !== null) return;
		startEditing(detail.edge as AllInfraEdge);
	}

	function handleMapClicked(e: Event): void {
		if (!editing) return;
		const detail = (e as CustomEvent).detail;
		if (!detail) return;

		const lon = detail.lon as number;
		const lat = detail.lat as number;

		// Insert a new waypoint at the closest segment position
		const insertIdx = findInsertPosition(lon, lat);
		const newWaypoints = [...waypoints];
		newWaypoints.splice(insertIdx, 0, [lon, lat]);
		waypoints = newWaypoints;
		pushStateToMap();

		e.stopImmediatePropagation();
	}

	function handleMapMouseMove(e: Event): void {
		if (!editing) return;
		const detail = (e as CustomEvent).detail;
		if (!detail) return;

		const lon = detail.lon as number;
		const lat = detail.lat as number;
		cursorPosition = [lon, lat];

		// If dragging a waypoint, update its position
		if (draggingIndex !== null && draggingIndex >= 0 && draggingIndex < waypoints.length) {
			const newWaypoints = [...waypoints];
			newWaypoints[draggingIndex] = [lon, lat];
			waypoints = newWaypoints;
			pushStateToMap();
		}
	}

	function handleMapContextMenu(e: Event): void {
		if (!editing) return;
		e.stopImmediatePropagation();
		e.preventDefault();
	}

	function handleWaypointDragStart(e: Event): void {
		if (!editing) return;
		const detail = (e as CustomEvent).detail;
		if (!detail || detail.waypointIndex === undefined) return;
		draggingIndex = detail.waypointIndex as number;
		pushStateToMap();
	}

	function handleWaypointDragEnd(e: Event): void {
		if (!editing || draggingIndex === null) return;
		draggingIndex = null;
		pushStateToMap();
	}

	function handleWaypointDelete(e: Event): void {
		if (!editing) return;
		const detail = (e as CustomEvent).detail;
		if (!detail || detail.waypointIndex === undefined) return;
		const idx = detail.waypointIndex as number;
		// Minimum 2 waypoints (source and target)
		if (waypoints.length <= 2) return;
		const newWaypoints = [...waypoints];
		newWaypoints.splice(idx, 1);
		waypoints = newWaypoints;
		pushStateToMap();
	}

	function handleKeyDown(e: KeyboardEvent): void {
		if (!editing) return;

		if (e.key === 'Enter') {
			confirmEdit();
			e.preventDefault();
			e.stopPropagation();
		} else if (e.key === 'Escape') {
			cancelEdit();
			e.preventDefault();
			e.stopPropagation();
		}
	}

	// ── Lifecycle ────────────────────────────────────────────────────────

	onMount(() => {
		window.addEventListener('edge-edit-start', handleEdgeEditStart);
		window.addEventListener('map-clicked', handleMapClicked, true);
		window.addEventListener('map-mousemove', handleMapMouseMove);
		window.addEventListener('map-contextmenu', handleMapContextMenu, true);
		window.addEventListener('waypoint-drag-start', handleWaypointDragStart);
		window.addEventListener('waypoint-drag-end', handleWaypointDragEnd);
		window.addEventListener('waypoint-delete', handleWaypointDelete);
		window.addEventListener('keydown', handleKeyDown, true);
	});

	onDestroy(() => {
		window.removeEventListener('edge-edit-start', handleEdgeEditStart);
		window.removeEventListener('map-clicked', handleMapClicked, true);
		window.removeEventListener('map-mousemove', handleMapMouseMove);
		window.removeEventListener('map-contextmenu', handleMapContextMenu, true);
		window.removeEventListener('waypoint-drag-start', handleWaypointDragStart);
		window.removeEventListener('waypoint-drag-end', handleWaypointDragEnd);
		window.removeEventListener('waypoint-delete', handleWaypointDelete);
		window.removeEventListener('keydown', handleKeyDown, true);
		if (editing) cancelEdit();
	});

	// Cancel editing if build mode activates
	$effect(() => {
		if ($buildMode !== null && editing) {
			cancelEdit();
		}
	});
</script>

{#if editing && edgeData}
	<div class="waypoint-editor-hud">
		<div class="hud-section">
			<span class="editor-badge">EDIT</span>
			<span class="edge-name">{edgeData.edge_type}</span>
		</div>

		<div class="hud-divider"></div>

		<div class="hud-section">
			<span class="hud-label">Waypoints</span>
			<span class="hud-value mono">{waypoints.length}</span>
		</div>

		<div class="hud-divider"></div>

		<div class="hud-section">
			<span class="hud-label">Length</span>
			<span class="hud-value mono">{totalLengthKm.toFixed(1)} km</span>
		</div>

		<div class="hud-divider"></div>

		<div class="hud-section">
			<span class="hud-label">Est. Cost</span>
			<span class="hud-value mono cost">{formatMoney(estimatedCost)}</span>
		</div>

		<div class="hud-divider"></div>

		<div class="hud-section actions">
			<button class="confirm-btn" onclick={confirmEdit}>Confirm</button>
			<button class="cancel-btn" onclick={cancelEdit}>Cancel</button>
		</div>

		<div class="hud-divider"></div>

		<div class="hud-hints">
			<span class="hint">Click: insert waypoint</span>
			<span class="hint-sep">|</span>
			<span class="hint">Drag: move waypoint</span>
			<span class="hint-sep">|</span>
			<span class="hint">Right-click: delete waypoint</span>
			<span class="hint-sep">|</span>
			<span class="hint"><kbd>Enter</kbd> confirm</span>
			<span class="hint-sep">|</span>
			<span class="hint"><kbd>Esc</kbd> cancel</span>
		</div>
	</div>
{/if}

<style>
	.waypoint-editor-hud {
		position: absolute;
		top: 84px;
		left: 50%;
		transform: translateX(-50%);
		z-index: 15;
		display: flex;
		align-items: center;
		gap: 0;
		background: rgba(17, 24, 39, 0.92);
		border: 1px solid rgba(96, 165, 250, 0.3);
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

	.hud-section.actions {
		gap: 4px;
	}

	.hud-divider {
		width: 1px;
		height: 20px;
		background: rgba(75, 85, 99, 0.5);
		flex-shrink: 0;
	}

	.editor-badge {
		font-size: 9px;
		font-weight: 800;
		letter-spacing: 0.1em;
		padding: 2px 6px;
		border-radius: 3px;
		background: rgba(96, 165, 250, 0.2);
		color: #60a5fa;
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

	.confirm-btn {
		background: rgba(16, 185, 129, 0.2);
		border: 1px solid rgba(16, 185, 129, 0.4);
		color: #10b981;
		font-size: 10px;
		font-family: var(--font-mono, monospace);
		font-weight: 600;
		padding: 2px 10px;
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.12s;
	}

	.confirm-btn:hover {
		background: rgba(16, 185, 129, 0.3);
	}

	.cancel-btn {
		background: rgba(239, 68, 68, 0.15);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #ef4444;
		font-size: 10px;
		font-family: var(--font-mono, monospace);
		font-weight: 600;
		padding: 2px 10px;
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.12s;
	}

	.cancel-btn:hover {
		background: rgba(239, 68, 68, 0.25);
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
