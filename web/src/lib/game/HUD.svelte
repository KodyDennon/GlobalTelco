<script lang="ts">
	import { worldInfo, playerCorp, formatMoney } from '$lib/stores/gameState';
	import { activePanel, buildMode, buildMenuParcel, buildEdgeSource, activeOverlay } from '$lib/stores/uiState';
	import type { PanelType, OverlayType } from '$lib/stores/uiState';
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
	<div class="hud-left">
		<span class="corp-name">{$playerCorp?.name ?? 'Loading...'}</span>
		<span class="cash" class:negative={($playerCorp?.cash ?? 0) < 0}>
			{formatMoney($playerCorp?.cash ?? 0)}
		</span>
		<span class="profit" class:loss={($playerCorp?.profit_per_tick ?? 0) < 0}>
			{($playerCorp?.profit_per_tick ?? 0) >= 0 ? '+' : ''}{formatMoney($playerCorp?.profit_per_tick ?? 0)}/tick
		</span>
	</div>

	<div class="hud-center">
		<div class="build-buttons">
			<button class="build-btn" class:active={currentBuild === 'node'} onclick={() => toggleBuild('node')} title="Build Node (B)">
				+ Node
			</button>
			<button class="build-btn" class:active={currentBuild === 'edge'} onclick={() => toggleBuild('edge')} title="Build Edge (E)">
				+ Edge
			</button>
		</div>
		<div class="divider"></div>
		<SpeedControls />
		<div class="divider"></div>
		<div class="panel-buttons">
			<button class="panel-btn" class:active={$activePanel === 'dashboard'} onclick={() => togglePanel('dashboard')} title="Dashboard">
				Fin
			</button>
			<button class="panel-btn" class:active={$activePanel === 'infrastructure'} onclick={() => togglePanel('infrastructure')} title="Infrastructure">
				Infra
			</button>
			<button class="panel-btn" class:active={$activePanel === 'contracts'} onclick={() => togglePanel('contracts')} title="Contracts">
				Con
			</button>
			<button class="panel-btn" class:active={$activePanel === 'research'} onclick={() => togglePanel('research')} title="Research">
				R&D
			</button>
			<button class="panel-btn" class:active={$activePanel === 'region'} onclick={() => togglePanel('region')} title="Regions">
				Reg
			</button>
			<button class="panel-btn advisor" class:active={$activePanel === 'advisor'} onclick={() => togglePanel('advisor')} title="Advisor">
				Adv
			</button>
		</div>
		<div class="divider"></div>
		<div class="overlay-buttons">
			<button class="overlay-btn" class:active={currentOverlay === 'terrain'} onclick={() => toggleOverlay('terrain')} title="Terrain">T</button>
			<button class="overlay-btn" class:active={currentOverlay === 'ownership'} onclick={() => toggleOverlay('ownership')} title="Ownership">O</button>
			<button class="overlay-btn" class:active={currentOverlay === 'demand'} onclick={() => toggleOverlay('demand')} title="Demand">D</button>
			<button class="overlay-btn" class:active={currentOverlay === 'coverage'} onclick={() => toggleOverlay('coverage')} title="Coverage">C</button>
		</div>
	</div>

	<div class="hud-right">
		<span class="tick">Tick {$worldInfo.tick}</span>
		<span class="rating">{$playerCorp?.credit_rating ?? '---'}</span>
		<span class="infra">{$playerCorp?.infrastructure_count ?? 0} nodes</span>
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
</style>
