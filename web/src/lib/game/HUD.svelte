<script lang="ts">
	import { worldInfo, playerCorp, formatMoney } from '$lib/stores/gameState';
	import {
		activePanelGroup,
		buildMode,
		buildEdgeSource,
		activeOverlay,
		selectedEdgeType,
		edgeTargets,
		selectedBuildItem,
		buildCategory,
		openPanelGroup,
		closePanelGroup,
		exitPlacementMode,
		canEdgeConnect,
		getEdgeTypesForSource,
		ghostPreviewInfo,
	} from '$lib/stores/uiState';
	import type { PanelGroupType, OverlayType } from '$lib/stores/uiState';
	import { isMultiplayer, connectionState, playerList } from '$lib/stores/multiplayerState';
	import { tr } from '$lib/i18n/index';
	import * as bridge from '$lib/wasm/bridge';
	import SpeedControls from './SpeedControls.svelte';
	import TierGuide from '$lib/ui/TierGuide.svelte';
	import { tooltip } from '$lib/ui/tooltip';

	// When edge source is selected, find its node type for filtering
	let sourceNodeType = $derived.by(() => {
		const sourceId = $buildEdgeSource;
		if (sourceId === null) return null;
		const allInfra = bridge.getAllInfrastructure();
		const node = allInfra.nodes.find(n => n.id === sourceId);
		return node?.node_type ?? null;
	});

	// Count valid targets for selected edge type
	let validTargetCount = $derived.by(() => {
		if (!sourceNodeType || $edgeTargets.length === 0) return 0;
		return $edgeTargets.filter(t =>
			canEdgeConnect($selectedEdgeType, sourceNodeType!, t.target_type)
		).length;
	});

	function toggleGroup(group: PanelGroupType) {
		if ($activePanelGroup === group) {
			closePanelGroup();
		} else {
			openPanelGroup(group);
		}
	}

	function toggleOverlay(overlay: OverlayType) {
		activeOverlay.update((o) => (o === overlay ? 'none' : overlay));
	}

	let currentBuild = $derived($buildMode);
	let currentOverlay = $derived($activeOverlay);
	let currentGroup = $derived($activePanelGroup);
	let showTierGuide = $state(false);

	// Display name for the active build item
	const BUILD_ITEM_NAMES: Record<string, string> = {
		CellTower: 'Cell Tower',
		WirelessRelay: 'Wireless Relay',
		CentralOffice: 'Central Office',
		ExchangePoint: 'Exchange Point',
		DataCenter: 'Data Center',
		BackboneRouter: 'Backbone Router',
		SatelliteGround: 'Satellite Ground',
		SubmarineLanding: 'Submarine Landing',
		Copper: 'Copper',
		FiberLocal: 'Fiber Local',
		FiberRegional: 'Fiber Regional',
		FiberNational: 'Fiber National',
		Microwave: 'Microwave',
		Satellite: 'Satellite',
		Submarine: 'Submarine',
	};

	const PANEL_GROUPS: Array<{ key: PanelGroupType; label: string; tip: string }> = [
		{ key: 'finance', label: 'Finance', tip: 'Dashboard, loans, budgets, and market share' },
		{ key: 'operations', label: 'Operations', tip: 'Infrastructure management and workforce' },
		{ key: 'diplomacy', label: 'Diplomacy', tip: 'Espionage, sabotage, and lobbying' },
		{ key: 'research', label: 'Research', tip: 'Technology tree and R&D budget' },
		{ key: 'market', label: 'Market', tip: 'Contracts, auctions, and mergers' },
		{ key: 'info', label: 'Info', tip: 'Region details, advisor, and achievements' },
	];

	const OVERLAY_TIPS: Record<string, string> = {
		terrain: 'Show terrain types — urban, rural, mountain, desert, etc.',
		ownership: 'Show which corporation controls each area',
		population: 'Show population density — dark (sparse) to bright yellow (dense). Build near population!',
		demand: 'Show telecom demand intensity — blue (low) to red (high)',
		coverage: 'Show network coverage — red (none) to green (full)',
		disaster: 'Show disaster risk — green (safe) to red (dangerous)',
		congestion: 'Show network congestion — green (free) to red (full)',
		traffic: 'Show traffic flow — blue (low) to white (high)',
		market_share: 'Show market share — regions colored by dominant corporation',
		ocean_depth: 'Show ocean depth zones — shallow (lighter blue) to deep (dark abyss). Plan submarine cables!',
		elevation_contour: 'Show elevation contours — lighter = higher, richer = lower. Contour lines at elevation bands. Procgen only.',
		submarine_reference: 'Show real-world submarine cable routes — reference overlay for planning cable placement. Real Earth mode only.',
		coverage_overlap: 'Show coverage overlap — cells with 2+ corporations competing. Red = hotspot, darker = more competitors.',
	};

	const OVERLAYS: Array<{ key: OverlayType; label: string; cls?: string }> = [
		{ key: 'terrain', label: 'Terrain' },
		{ key: 'ownership', label: 'Own' },
		{ key: 'population', label: 'Pop', cls: 'population' },
		{ key: 'demand', label: 'Demand' },
		{ key: 'coverage', label: 'Cover' },
		{ key: 'coverage_overlap', label: 'Overlap', cls: 'overlap' },
		{ key: 'disaster', label: 'Risk', cls: 'disaster' },
		{ key: 'congestion', label: 'Congest', cls: 'congestion' },
		{ key: 'traffic', label: 'Traffic', cls: 'traffic' },
		{ key: 'market_share', label: 'Market', cls: 'market-share' },
		{ key: 'ocean_depth', label: 'Ocean', cls: 'ocean' },
		{ key: 'elevation_contour', label: 'Elev', cls: 'elevation' },
		{ key: 'submarine_reference', label: 'Cables', cls: 'cables' },
	];
</script>

<div class="hud">
	<!-- Row 1: Status bar -->
	<div class="hud-row row-1">
		<div class="hud-left" role="status">
			<span class="corp-name" use:tooltip={() => `Your corporation: ${$playerCorp?.name ?? 'Unknown'}`}>{$playerCorp?.name ?? 'Loading...'}</span>
			<span class="cash" class:negative={($playerCorp?.cash ?? 0) < 0} use:tooltip={() => `Cash on hand: ${formatMoney($playerCorp?.cash ?? 0)}\nUsed for building, hiring, and operations`}>
				{formatMoney($playerCorp?.cash ?? 0)}
			</span>
			<span class="profit" class:loss={($playerCorp?.profit_per_tick ?? 0) < 0} use:tooltip={() => `Net income per tick: revenue minus all costs\n${formatMoney($playerCorp?.revenue_per_tick ?? 0)} revenue - ${formatMoney($playerCorp?.cost_per_tick ?? 0)} costs`}>
				{($playerCorp?.profit_per_tick ?? 0) >= 0 ? '+' : ''}{formatMoney($playerCorp?.profit_per_tick ?? 0)}/tick
			</span>
		</div>

		<div class="hud-center" role="toolbar" aria-label="Game controls">
			<SpeedControls />
			<div class="divider"></div>
			<span class="tick" use:tooltip={'Current simulation tick — each tick represents one time unit'}>{$tr('game.tick', { tick: $worldInfo.tick })}</span>
			<span class="rating" use:tooltip={() => `Credit rating: ${$playerCorp?.credit_rating ?? '---'}\nAffects loan interest rates and contract trust`}>{$playerCorp?.credit_rating ?? '---'}</span>
			<span class="infra" use:tooltip={() => `Total infrastructure: ${$playerCorp?.infrastructure_count ?? 0} nodes\nIncludes towers, offices, data centers, and more`}>{$tr('game.nodes', { count: $playerCorp?.infrastructure_count ?? 0 })}</span>
			{#if $worldInfo.sandbox}
				<span class="sandbox-indicator" use:tooltip={'Sandbox mode — unlimited funds, no bankruptcy'}>SANDBOX</span>
			{/if}
		</div>

		<div class="hud-right" role="status">
			{#if $isMultiplayer}
				<span class="mp-status" class:connected={$connectionState === 'connected'} class:reconnecting={$connectionState === 'reconnecting'}>
					{$connectionState === 'connected' ? $tr('game.online') : $connectionState === 'reconnecting' ? $tr('game.reconnecting') : $tr('game.offline')}
				</span>
				<span class="mp-players">{$tr('game.players', { count: $playerList.filter(p => p.status === 'Connected').length })}</span>
			{/if}
		</div>
	</div>

	<!-- Row 2: Actions bar -->
	<div class="hud-row row-2">
		<!-- Build mode status indicator -->
		<div class="build-status">
			{#if currentBuild === 'node' && $selectedBuildItem}
				<span class="build-mode-badge node">NODE</span>
				<span class="build-item-name">{BUILD_ITEM_NAMES[$selectedBuildItem] ?? $selectedBuildItem}</span>
				{#if $ghostPreviewInfo.terrainType}
					<span class="ghost-terrain" class:ghost-invalid={!$ghostPreviewInfo.valid}>
						{$ghostPreviewInfo.terrainType}
					</span>
					<span class="ghost-multiplier" class:ghost-expensive={$ghostPreviewInfo.costMultiplier >= 2.0}>
						{$ghostPreviewInfo.costMultiplier.toFixed(1)}x
					</span>
				{/if}
				{#if $ghostPreviewInfo.cost !== null}
					<span class="ghost-cost">{formatMoney($ghostPreviewInfo.cost)}</span>
				{/if}
				<span class="ghost-cash" class:negative={($playerCorp?.cash ?? 0) < ($ghostPreviewInfo.cost ?? 0)}>
					{formatMoney($playerCorp?.cash ?? 0)}
				</span>
				{#if !$ghostPreviewInfo.valid}
					<span class="ghost-invalid-label">INVALID</span>
				{:else}
					<span class="build-hint">Click to place</span>
				{/if}
				<button class="cancel-btn" onclick={exitPlacementMode} use:tooltip={'Cancel build mode (Esc)'}>Cancel</button>
			{:else if currentBuild === 'edge'}
				<span class="build-mode-badge edge">LINK</span>
				<span class="build-item-name">{BUILD_ITEM_NAMES[$selectedEdgeType] ?? $selectedEdgeType}</span>
				{#if $buildEdgeSource !== null}
					<span class="edge-status">
						{#if validTargetCount > 0}
							<span class="target-count">{validTargetCount} target{validTargetCount > 1 ? 's' : ''}</span>
						{:else}
							<span class="no-targets">No valid targets</span>
						{/if}
						— click a green node
					</span>
				{:else}
					<span class="edge-hint">Click a source node</span>
				{/if}
				<button class="tier-help-btn" onclick={() => showTierGuide = !showTierGuide} use:tooltip={'Show tier compatibility guide — which edge types connect which node tiers'}>?</button>
				<button class="cancel-btn" onclick={exitPlacementMode} use:tooltip={'Cancel build mode (Esc)'}>Cancel</button>
			{:else if currentBuild === 'node'}
				<span class="build-mode-badge node">NODE</span>
				<span class="build-hint">Right-click map to open build menu</span>
				<button class="cancel-btn" onclick={exitPlacementMode} use:tooltip={'Cancel build mode (Esc)'}>Cancel</button>
			{:else}
				<span class="build-hint-idle" use:tooltip={'Right-click the map to open the radial build menu.\nUse keys 1-9 for hotbar shortcuts.'}>Right-click to build</span>
			{/if}
		</div>

		<div class="divider"></div>

		<div class="panel-buttons">
			{#each PANEL_GROUPS as group}
				<button
					class="panel-btn"
					class:active={currentGroup === group.key}
					onclick={() => toggleGroup(group.key)}
					aria-pressed={currentGroup === group.key}
					use:tooltip={group.tip}
				>
					{group.label}
				</button>
			{/each}
		</div>

		<div class="divider"></div>

		<div class="overlay-buttons">
			{#each OVERLAYS as overlay}
				<button
					class="overlay-btn"
					class:active={currentOverlay === overlay.key}
					class:population={overlay.cls === 'population'}
					class:disaster={overlay.cls === 'disaster'}
					class:congestion={overlay.cls === 'congestion'}
					class:traffic={overlay.cls === 'traffic'}
					class:market-share={overlay.cls === 'market-share'}
					class:ocean={overlay.cls === 'ocean'}
					class:cables={overlay.cls === 'cables'}
					class:overlap={overlay.cls === 'overlap'}
					onclick={() => toggleOverlay(overlay.key)}
					use:tooltip={OVERLAY_TIPS[overlay.key] ?? overlay.label}
					aria-pressed={currentOverlay === overlay.key}
				>
					{overlay.label}
				</button>
			{/each}
		</div>
	</div>
</div>

{#if showTierGuide}
	<TierGuide onclose={() => showTierGuide = false} />
{/if}

<style>
	.hud {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		z-index: 10;
		font-family: var(--font-mono);
		font-size: 13px;
		color: var(--text-secondary);
	}

	.hud-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 16px;
		height: 40px;
		background: rgba(17, 24, 39, 0.95);
	}

	.row-1 {
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
	}

	.row-2 {
		border-bottom: 1px solid var(--border);
		justify-content: flex-start;
		gap: 8px;
	}

	.hud-left, .hud-right, .hud-center {
		display: flex;
		align-items: center;
		gap: 16px;
	}

	.divider {
		width: 1px;
		height: 24px;
		background: var(--border);
		flex-shrink: 0;
	}

	.corp-name {
		font-weight: 600;
		color: var(--text-primary);
	}

	.cash {
		color: var(--green);
		font-weight: 600;
	}

	.cash.negative {
		color: var(--red);
	}

	.profit {
		color: var(--green);
		font-size: 11px;
	}

	.profit.loss {
		color: var(--red);
	}

	.tick {
		color: var(--text-muted);
	}

	.rating {
		color: var(--amber);
		font-weight: 600;
	}

	.infra {
		color: var(--blue);
	}

	/* ── Build status indicator ────────────────────────────────────────────── */

	.build-status {
		display: flex;
		align-items: center;
		gap: 8px;
		background: rgba(31, 41, 55, 0.8);
		border-radius: var(--radius-sm);
		padding: 2px 8px;
		min-height: 28px;
	}

	.build-mode-badge {
		font-size: 9px;
		font-weight: 800;
		letter-spacing: 0.1em;
		padding: 2px 6px;
		border-radius: 3px;
	}

	.build-mode-badge.node {
		background: rgba(16, 185, 129, 0.2);
		color: #10b981;
	}

	.build-mode-badge.edge {
		background: rgba(251, 191, 36, 0.2);
		color: #fbbf24;
	}

	.build-item-name {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-primary);
	}

	.build-hint {
		font-size: 11px;
		color: var(--text-muted);
	}

	.build-hint-idle {
		font-size: 11px;
		color: var(--text-dim, #6b7280);
		cursor: default;
	}

	.cancel-btn {
		background: rgba(239, 68, 68, 0.15);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #ef4444;
		font-size: 10px;
		font-family: var(--font-mono);
		font-weight: 600;
		padding: 2px 8px;
		border-radius: 3px;
		cursor: pointer;
		transition: all 0.12s;
	}

	.cancel-btn:hover {
		background: rgba(239, 68, 68, 0.25);
	}

	/* ── Ghost preview (build mode terrain/cost) ──────────────────────────── */

	.ghost-terrain {
		font-size: 10px;
		font-weight: 600;
		color: #a5b4fc;
		background: rgba(99, 102, 241, 0.15);
		border: 1px solid rgba(99, 102, 241, 0.3);
		padding: 1px 6px;
		border-radius: 3px;
		font-family: var(--font-mono);
	}

	.ghost-terrain.ghost-invalid {
		color: #ef4444;
		background: rgba(239, 68, 68, 0.15);
		border-color: rgba(239, 68, 68, 0.3);
	}

	.ghost-multiplier {
		font-size: 10px;
		font-weight: 700;
		color: var(--text-muted);
		font-family: var(--font-mono);
	}

	.ghost-multiplier.ghost-expensive {
		color: #f59e0b;
	}

	.ghost-cost {
		font-size: 10px;
		font-weight: 600;
		color: var(--green);
		font-family: var(--font-mono);
	}

	.ghost-cash {
		font-size: 10px;
		font-weight: 500;
		color: var(--text-dim);
		font-family: var(--font-mono);
		border-left: 1px solid var(--border);
		padding-left: 6px;
	}

	.ghost-cash.negative {
		color: #ef4444;
	}

	.ghost-invalid-label {
		font-size: 9px;
		font-weight: 800;
		letter-spacing: 0.1em;
		color: #ef4444;
		background: rgba(239, 68, 68, 0.15);
		border: 1px solid rgba(239, 68, 68, 0.3);
		padding: 1px 6px;
		border-radius: 3px;
	}

	/* ── Panel & overlay buttons ────────────────────────────────────────────── */

	.panel-buttons, .overlay-buttons {
		display: flex;
		gap: 2px;
		background: rgba(31, 41, 55, 0.8);
		border-radius: var(--radius-sm);
		padding: 2px;
	}

	.panel-btn, .overlay-btn {
		background: transparent;
		border: none;
		color: var(--text-muted);
		padding: 4px 10px;
		font-size: 11px;
		font-family: var(--font-mono);
		cursor: pointer;
		border-radius: 3px;
		transition: all 0.15s;
		white-space: nowrap;
	}

	.overlay-btn {
		padding: 4px 8px;
		font-size: 10px;
		font-weight: 600;
	}

	.panel-btn:hover, .overlay-btn:hover {
		background: rgba(55, 65, 81, 0.5);
		color: var(--text-primary);
	}

	.panel-btn.active {
		background: var(--green-bg);
		color: var(--green);
	}

	.overlay-btn.active {
		background: rgba(245, 158, 11, 0.2);
		color: var(--amber);
	}

	.overlay-btn.population {
		color: #f5d060;
	}

	.overlay-btn.population.active {
		background: rgba(245, 208, 96, 0.2);
		color: #f5d060;
	}

	.overlay-btn.disaster {
		color: var(--red);
		font-weight: 900;
	}

	.overlay-btn.disaster.active {
		background: rgba(239, 68, 68, 0.2);
		color: var(--red);
	}

	.overlay-btn.congestion {
		color: var(--amber);
	}

	.overlay-btn.congestion.active {
		background: rgba(245, 158, 11, 0.2);
		color: var(--amber);
	}

	.overlay-btn.traffic {
		color: var(--green);
	}

	.overlay-btn.traffic.active {
		background: rgba(16, 185, 129, 0.2);
		color: var(--green);
	}

	.overlay-btn.market-share {
		color: #a78bfa;
	}

	.overlay-btn.market-share.active {
		background: rgba(139, 92, 246, 0.2);
		color: #c4b5fd;
	}

	.overlay-btn.ocean {
		color: #3b82f6;
	}

	.overlay-btn.ocean.active {
		background: rgba(59, 130, 246, 0.2);
		color: #60a5fa;
	}

	.overlay-btn.cables {
		color: #8bb4d6;
	}

	.overlay-btn.cables.active {
		background: rgba(100, 180, 255, 0.2);
		color: #a8d0f0;
	}

	.overlay-btn.overlap {
		color: #f472b6;
	}

	.overlay-btn.overlap.active {
		background: rgba(244, 114, 182, 0.2);
		color: #f9a8d4;
	}

	.sandbox-indicator {
		font-size: 10px;
		font-weight: 800;
		letter-spacing: 0.1em;
		padding: 2px 8px;
		border-radius: 4px;
		background: rgba(245, 158, 11, 0.2);
		color: #f59e0b;
		border: 1px solid rgba(245, 158, 11, 0.3);
	}

	.mp-status {
		font-size: 11px;
		padding: 2px 8px;
		border-radius: 4px;
		background: rgba(239, 68, 68, 0.2);
		color: var(--red);
	}

	.mp-status.connected {
		background: rgba(16, 185, 129, 0.2);
		color: var(--green);
	}

	.mp-status.reconnecting {
		background: rgba(245, 158, 11, 0.2);
		color: var(--amber);
	}

	.mp-players {
		font-size: 11px;
		color: var(--text-muted);
	}

	/* Tier help button */
	.tier-help-btn {
		width: 24px;
		height: 24px;
		background: rgba(59, 130, 246, 0.2);
		border: 1px solid rgba(59, 130, 246, 0.4);
		color: #60a5fa;
		font-size: 12px;
		font-weight: 700;
		border-radius: 50%;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: all 0.15s;
	}

	.tier-help-btn:hover {
		background: rgba(59, 130, 246, 0.3);
		color: #93c5fd;
	}

	.edge-status, .edge-hint {
		font-size: 11px;
		color: var(--text-muted);
		white-space: nowrap;
	}

	.target-count {
		color: var(--green);
		font-weight: 600;
	}

	.no-targets {
		color: var(--red);
		font-weight: 600;
	}

</style>
