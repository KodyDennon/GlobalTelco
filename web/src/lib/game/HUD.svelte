<script lang="ts">
	import { worldInfo, playerCorp, formatMoney } from '$lib/stores/gameState';
	import {
		activePanelGroup,
		buildMode,
		buildMenuLocation,
		buildEdgeSource,
		activeOverlay,
		selectedEdgeType,
		openPanelGroup,
		closePanelGroup,
	} from '$lib/stores/uiState';
	import type { PanelGroupType, OverlayType } from '$lib/stores/uiState';
	import { isMultiplayer, connectionState, playerList } from '$lib/stores/multiplayerState';
	import { tr } from '$lib/i18n/index';
	import SpeedControls from './SpeedControls.svelte';
	import TierGuide from '$lib/ui/TierGuide.svelte';

	function toggleGroup(group: PanelGroupType) {
		if ($activePanelGroup === group) {
			closePanelGroup();
		} else {
			openPanelGroup(group);
		}
	}

	function toggleBuild(mode: string) {
		buildMode.update((m) => {
			if (m === mode) {
				buildMenuLocation.set(null);
				buildEdgeSource.set(null);
				return null;
			}
			return mode;
		});
	}

	function toggleOverlay(overlay: OverlayType) {
		activeOverlay.update((o) => (o === overlay ? 'none' : overlay));
	}

	let currentBuild = $derived($buildMode);
	let currentOverlay = $derived($activeOverlay);
	let currentGroup = $derived($activePanelGroup);
	let showTierGuide = $state(false);

	// Edge types with distance multipliers (must match Rust EdgeType::distance_multiplier())
	const EDGE_TYPES = [
		{ key: 'Copper', name: 'Copper', mult: 2, tiers: 'T1-T1/T2' },
		{ key: 'FiberLocal', name: 'Fiber Local', mult: 5, tiers: 'T1-T1/T2, T2-T2' },
		{ key: 'Microwave', name: 'Microwave', mult: 8, tiers: 'T1-T2, T2-T2/T3' },
		{ key: 'FiberRegional', name: 'Fiber Reg.', mult: 15, tiers: 'T2-T2/T3, T3-T3' },
		{ key: 'FiberNational', name: 'Fiber Nat.', mult: 40, tiers: 'T3-T3/T4, T4-T4' },
		{ key: 'Satellite', name: 'Satellite', mult: Infinity, tiers: 'T3/T4-T5' },
		{ key: 'Submarine', name: 'Submarine', mult: 60, tiers: 'T5-T5' },
	];

	let spacing = $derived($worldInfo.cell_spacing_km || 100);

	function fmtRange(mult: number): string {
		if (!isFinite(mult)) return '∞';
		const km = Math.round(spacing * mult);
		return km >= 1000 ? `${(km / 1000).toFixed(1)}k km` : `${km}km`;
	}

	const PANEL_GROUPS: Array<{ key: PanelGroupType; label: string }> = [
		{ key: 'finance', label: 'Finance' },
		{ key: 'operations', label: 'Operations' },
		{ key: 'diplomacy', label: 'Diplomacy' },
		{ key: 'research', label: 'Research' },
		{ key: 'market', label: 'Market' },
		{ key: 'info', label: 'Info' },
	];

	const OVERLAYS: Array<{ key: OverlayType; label: string; cls?: string }> = [
		{ key: 'terrain', label: 'Terrain' },
		{ key: 'ownership', label: 'Own' },
		{ key: 'demand', label: 'Demand' },
		{ key: 'coverage', label: 'Cover' },
		{ key: 'disaster', label: 'Risk', cls: 'disaster' },
		{ key: 'traffic', label: 'Traffic', cls: 'traffic' },
	];
</script>

<div class="hud">
	<!-- Row 1: Status bar -->
	<div class="hud-row row-1">
		<div class="hud-left" role="status">
			<span class="corp-name">{$playerCorp?.name ?? 'Loading...'}</span>
			<span class="cash" class:negative={($playerCorp?.cash ?? 0) < 0}>
				{formatMoney($playerCorp?.cash ?? 0)}
			</span>
			<span class="profit" class:loss={($playerCorp?.profit_per_tick ?? 0) < 0}>
				{($playerCorp?.profit_per_tick ?? 0) >= 0 ? '+' : ''}{formatMoney($playerCorp?.profit_per_tick ?? 0)}/tick
			</span>
		</div>

		<div class="hud-center" role="toolbar" aria-label="Game controls">
			<SpeedControls />
			<div class="divider"></div>
			<span class="tick">{$tr('game.tick', { tick: $worldInfo.tick })}</span>
			<span class="rating">{$playerCorp?.credit_rating ?? '---'}</span>
			<span class="infra">{$tr('game.nodes', { count: $playerCorp?.infrastructure_count ?? 0 })}</span>
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
		<div class="build-buttons">
			<button class="build-btn" class:active={currentBuild === 'node'} onclick={() => toggleBuild('node')} title={$tr('game.build_node')} aria-pressed={currentBuild === 'node'}>
				{$tr('game.build_node')}
			</button>
			<button class="build-btn" class:active={currentBuild === 'edge'} onclick={() => toggleBuild('edge')} title={$tr('game.build_edge')} aria-pressed={currentBuild === 'edge'}>
				{$tr('game.build_edge')}
			</button>
			{#if currentBuild === 'edge'}
				<select class="edge-type-select" bind:value={$selectedEdgeType} aria-label="Edge type">
					{#each EDGE_TYPES as et}
						<option value={et.key}>{et.name} ({fmtRange(et.mult)}) {et.tiers}</option>
					{/each}
				</select>
				<button class="tier-help-btn" onclick={() => showTierGuide = !showTierGuide} title="Tier Guide">?</button>
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
					class:disaster={overlay.cls === 'disaster'}
					class:traffic={overlay.cls === 'traffic'}
					onclick={() => toggleOverlay(overlay.key)}
					title={overlay.label}
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

	.build-buttons, .panel-buttons, .overlay-buttons {
		display: flex;
		gap: 2px;
		background: rgba(31, 41, 55, 0.8);
		border-radius: var(--radius-sm);
		padding: 2px;
	}

	.build-btn, .panel-btn, .overlay-btn {
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

	.build-btn:hover, .panel-btn:hover, .overlay-btn:hover {
		background: rgba(55, 65, 81, 0.5);
		color: var(--text-primary);
	}

	.edge-type-select {
		background: rgba(31, 41, 55, 0.9);
		border: 1px solid var(--border);
		color: var(--text-primary);
		font-size: 11px;
		font-family: var(--font-mono);
		padding: 3px 6px;
		border-radius: 3px;
		cursor: pointer;
	}

	.build-btn.active {
		background: rgba(59, 130, 246, 0.2);
		color: var(--blue);
	}

	.panel-btn.active {
		background: var(--green-bg);
		color: var(--green);
	}

	.overlay-btn.active {
		background: rgba(245, 158, 11, 0.2);
		color: var(--amber);
	}

	.overlay-btn.disaster {
		color: var(--red);
		font-weight: 900;
	}

	.overlay-btn.disaster.active {
		background: rgba(239, 68, 68, 0.2);
		color: var(--red);
	}

	.overlay-btn.traffic {
		color: var(--green);
	}

	.overlay-btn.traffic.active {
		background: rgba(16, 185, 129, 0.2);
		color: var(--green);
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

</style>
