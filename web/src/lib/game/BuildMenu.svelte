<script lang="ts">
	import { buildMenuLocation, buildMode, buildEdgeSource } from '$lib/stores/uiState';
	import { formatMoney } from '$lib/stores/gameState';
	import { tr } from '$lib/i18n/index';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import type { BuildOption } from '$lib/wasm/types';
	import { tooltip } from '$lib/ui/tooltip';

	// Node type → tier label
	const NODE_TIER_LABEL: Record<string, string> = {
		CellTower: 'T1 Access',
		WirelessRelay: 'T1 Access',
		CentralOffice: 'T2 Aggregation',
		ExchangePoint: 'T2 Aggregation',
		DataCenter: 'T3 Core',
		BackboneRouter: 'T4 Backbone',
		SatelliteGround: 'T5 Global',
		SubmarineLanding: 'T5 Global',
	};

	let options: BuildOption[] = $state([]);

	$effect(() => {
		const loc = $buildMenuLocation;
		if (loc) {
			options = bridge.getBuildableNodes(loc.lon, loc.lat);
		} else {
			options = [];
		}
	});

	function build(opt: BuildOption) {
		const loc = $buildMenuLocation;
		if (!loc) return;
		gameCommand({
			BuildNode: { node_type: opt.node_type, lon: loc.lon, lat: loc.lat }
		});
		close();
	}

	function close() {
		buildMenuLocation.set(null);
		buildMode.set(null);
		buildEdgeSource.set(null);
	}
</script>

{#if $buildMenuLocation}
	<div class="build-menu" role="menu" aria-label={$tr('game.build_infra')}>
		<div class="build-header">
			<span>{$tr('game.build_infra')}</span>
			<button class="close-btn" onclick={close} aria-label={$tr('common.close')}>x</button>
		</div>
		<div class="build-list">
			{#each options as opt}
				<button
					class="build-option"
					class:disabled={!opt.affordable}
					onclick={() => build(opt)}
					disabled={!opt.affordable}
					role="menuitem"
					use:tooltip={() => `${opt.label} — ${NODE_TIER_LABEL[opt.node_type] ?? opt.network_level}\nCost: ${formatMoney(opt.cost)}\nBuild time: ${opt.build_ticks} ticks${!opt.affordable ? '\nInsufficient funds!' : ''}`}
				>
					<div class="opt-info">
						<span class="opt-name">{opt.label}</span>
						<span class="opt-level">{NODE_TIER_LABEL[opt.node_type] ?? opt.network_level}</span>
					</div>
					<div class="opt-meta">
						<span class="opt-cost" class:unaffordable={!opt.affordable}>{formatMoney(opt.cost)}</span>
						<span class="opt-time">{opt.build_ticks} ticks</span>
					</div>
				</button>
			{/each}
		</div>
	</div>
{/if}

<style>
	.build-menu {
		position: absolute;
		left: 16px;
		bottom: 16px;
		width: 320px;
		background: rgba(17, 24, 39, 0.97);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		z-index: 25;
		font-family: var(--font-sans);
		color: var(--text-secondary);
	}

	.build-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 10px 14px;
		border-bottom: 1px solid var(--border);
		font-weight: 600;
		font-size: 13px;
		color: var(--text-primary);
	}

	.close-btn {
		background: none;
		border: none;
		color: var(--text-dim);
		cursor: pointer;
		font-size: 16px;
		padding: 0 4px;
	}

	.build-list {
		display: flex;
		flex-direction: column;
		padding: 4px;
		max-height: 400px;
		overflow-y: auto;
	}

	.build-option {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 10px 12px;
		background: transparent;
		border: none;
		border-radius: var(--radius-sm);
		color: var(--text-secondary);
		cursor: pointer;
		font-family: var(--font-sans);
		font-size: 13px;
		text-align: left;
		transition: background 0.15s;
	}

	.build-option:hover:not(.disabled) {
		background: var(--bg-surface);
	}

	.build-option.disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.opt-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.opt-name {
		font-weight: 600;
		color: var(--text-primary);
	}

	.opt-level {
		font-size: 11px;
		color: var(--text-muted);
	}

	.opt-meta {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 2px;
	}

	.opt-cost {
		font-family: var(--font-mono);
		font-weight: 600;
		color: var(--green);
	}

	.opt-cost.unaffordable {
		color: var(--red);
	}

	.opt-time {
		font-size: 11px;
		color: var(--text-muted);
	}
</style>
