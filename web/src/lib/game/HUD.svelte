<script lang="ts">
	import { worldInfo, playerCorp, formatMoney } from '$lib/stores/gameState';
	import { activePanel, buildMode, buildMenuParcel, buildEdgeSource, activeOverlay } from '$lib/stores/uiState';
	import type { PanelType, OverlayType } from '$lib/stores/uiState';
	import { isMultiplayer, connectionState, playerList } from '$lib/stores/multiplayerState';
	import { tr } from '$lib/i18n/index';
	import SpeedControls from './SpeedControls.svelte';

	function togglePanel(panel: PanelType) {
		activePanel.update((p) => (p === panel ? 'none' : panel));
	}

	function toggleBuild(mode: string) {
		buildMode.update((m) => {
			if (m === mode) {
				buildMenuParcel.set(null);
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
</script>

<div class="hud">
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
		<div class="build-buttons">
			<button class="build-btn" class:active={currentBuild === 'node'} onclick={() => toggleBuild('node')} title={$tr('game.build_node')} aria-pressed={currentBuild === 'node'}>
				{$tr('game.build_node')}
			</button>
			<button class="build-btn" class:active={currentBuild === 'edge'} onclick={() => toggleBuild('edge')} title={$tr('game.build_edge')} aria-pressed={currentBuild === 'edge'}>
				{$tr('game.build_edge')}
			</button>
		</div>
		<div class="divider"></div>
		<SpeedControls />
		<div class="divider"></div>
		<div class="panel-buttons">
			<button class="panel-btn" class:active={$activePanel === 'dashboard'} onclick={() => togglePanel('dashboard')} title={$tr('hud.dashboard')} aria-pressed={$activePanel === 'dashboard'}>
				Fin
			</button>
			<button class="panel-btn" class:active={$activePanel === 'infrastructure'} onclick={() => togglePanel('infrastructure')} title={$tr('hud.infrastructure')} aria-pressed={$activePanel === 'infrastructure'}>
				Infra
			</button>
			<button class="panel-btn" class:active={$activePanel === 'contracts'} onclick={() => togglePanel('contracts')} title={$tr('hud.contracts')} aria-pressed={$activePanel === 'contracts'}>
				Con
			</button>
			<button class="panel-btn" class:active={$activePanel === 'research'} onclick={() => togglePanel('research')} title={$tr('hud.research')} aria-pressed={$activePanel === 'research'}>
				R&D
			</button>
			<button class="panel-btn" class:active={$activePanel === 'region'} onclick={() => togglePanel('region')} title={$tr('panels.regions')} aria-pressed={$activePanel === 'region'}>
				Reg
			</button>
			<button class="panel-btn" class:active={$activePanel === 'workforce'} onclick={() => togglePanel('workforce')} title={$tr('hud.workforce')} aria-pressed={$activePanel === 'workforce'}>
				WF
			</button>
			<button class="panel-btn advisor" class:active={$activePanel === 'advisor'} onclick={() => togglePanel('advisor')} title={$tr('hud.advisor')} aria-pressed={$activePanel === 'advisor'}>
				Adv
			</button>
			<button class="panel-btn" class:active={$activePanel === 'auctions'} onclick={() => togglePanel('auctions')} title={$tr('hud.auctions')} aria-pressed={$activePanel === 'auctions'}>
				Auc
			</button>
			<button class="panel-btn" class:active={$activePanel === 'mergers'} onclick={() => togglePanel('mergers')} title={$tr('hud.mergers')} aria-pressed={$activePanel === 'mergers'}>
				M&A
			</button>
			<button class="panel-btn" class:active={$activePanel === 'intel'} onclick={() => togglePanel('intel')} title={$tr('hud.intel')} aria-pressed={$activePanel === 'intel'}>
				Int
			</button>
			<button class="panel-btn" class:active={$activePanel === 'achievements'} onclick={() => togglePanel('achievements')} title={$tr('hud.achievements')} aria-pressed={$activePanel === 'achievements'}>
				Ach
			</button>
		</div>
		<div class="divider"></div>
		<div class="overlay-buttons">
			<button class="overlay-btn" class:active={currentOverlay === 'terrain'} onclick={() => toggleOverlay('terrain')} title={$tr('hud.terrain_overlay')} aria-pressed={currentOverlay === 'terrain'}>T</button>
			<button class="overlay-btn" class:active={currentOverlay === 'ownership'} onclick={() => toggleOverlay('ownership')} title={$tr('hud.ownership_overlay')} aria-pressed={currentOverlay === 'ownership'}>O</button>
			<button class="overlay-btn" class:active={currentOverlay === 'demand'} onclick={() => toggleOverlay('demand')} title={$tr('hud.demand_overlay')} aria-pressed={currentOverlay === 'demand'}>D</button>
			<button class="overlay-btn" class:active={currentOverlay === 'coverage'} onclick={() => toggleOverlay('coverage')} title={$tr('hud.coverage_overlay')} aria-pressed={currentOverlay === 'coverage'}>C</button>
			<button class="overlay-btn disaster" class:active={currentOverlay === 'disaster'} onclick={() => toggleOverlay('disaster')} title={$tr('hud.disaster_overlay')} aria-pressed={currentOverlay === 'disaster'}>!</button>
			<button class="overlay-btn" class:active={currentOverlay === 'congestion'} onclick={() => toggleOverlay('congestion')} title={$tr('hud.congestion_overlay')} aria-pressed={currentOverlay === 'congestion'}>~</button>
		</div>
	</div>

	<div class="hud-right" role="status">
		{#if $isMultiplayer}
			<span class="mp-status" class:connected={$connectionState === 'connected'} class:reconnecting={$connectionState === 'reconnecting'}>
				{$connectionState === 'connected' ? $tr('game.online') : $connectionState === 'reconnecting' ? $tr('game.reconnecting') : $tr('game.offline')}
			</span>
			<span class="mp-players">{$tr('game.players', { count: $playerList.filter(p => p.status === 'Connected').length })}</span>
		{/if}
		<span class="tick">{$tr('game.tick', { tick: $worldInfo.tick })}</span>
		<span class="rating">{$playerCorp?.credit_rating ?? '---'}</span>
		<span class="infra">{$tr('game.nodes', { count: $playerCorp?.infrastructure_count ?? 0 })}</span>
	</div>
</div>

<style>
	.hud {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: 48px;
		background: rgba(17, 24, 39, 0.95);
		border-bottom: 1px solid var(--border);
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 16px;
		z-index: 10;
		font-family: var(--font-mono);
		font-size: 13px;
		color: var(--text-secondary);
	}

	.hud-left, .hud-right {
		display: flex;
		align-items: center;
		gap: 16px;
	}

	.hud-center {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.divider {
		width: 1px;
		height: 24px;
		background: var(--border);
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
	}

	.overlay-btn {
		padding: 4px 6px;
		font-size: 10px;
		font-weight: 700;
	}

	.build-btn:hover, .panel-btn:hover, .overlay-btn:hover {
		background: rgba(55, 65, 81, 0.5);
		color: var(--text-primary);
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

	.panel-btn.advisor {
		color: var(--amber);
	}

	.panel-btn.advisor.active {
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
</style>
