<script lang="ts">
	import { buildMenuParcel, buildMode, buildEdgeSource } from '$lib/stores/uiState';
	import { formatMoney } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import type { BuildOption } from '$lib/wasm/types';

	let options: BuildOption[] = $state([]);

	$effect(() => {
		const parcel = $buildMenuParcel;
		if (parcel) {
			options = bridge.getBuildableNodes(parcel.id);
		} else {
			options = [];
		}
	});

	function build(opt: BuildOption) {
		const parcel = $buildMenuParcel;
		if (!parcel) return;
		bridge.processCommand({
			BuildNode: { node_type: opt.node_type, parcel: parcel.id }
		});
		close();
	}

	function close() {
		buildMenuParcel.set(null);
	}
</script>

{#if $buildMenuParcel}
	<div class="build-menu">
		<div class="build-header">
			<span>Build Infrastructure</span>
			<button class="close-btn" onclick={close}>x</button>
		</div>
		<div class="build-list">
			{#each options as opt}
				<button
					class="build-option"
					class:disabled={!opt.affordable}
					onclick={() => build(opt)}
					disabled={!opt.affordable}
				>
					<div class="opt-info">
						<span class="opt-name">{opt.label}</span>
						<span class="opt-level">{opt.network_level}</span>
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
