<script lang="ts">
	import { playerCorp, formatMoney } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import type { TerminalInventory } from '$lib/wasm/bridge';

	let inventory: TerminalInventory = $state({ factories: [], warehouses: [] });

	$effect(() => {
		const corp = $playerCorp;
		if (corp) {
			inventory = bridge.getTerminalInventory(corp.id);
		}
	});

	let totalProduced = $derived(
		inventory.factories.reduce((s, f) => s + f.produced_stored, 0)
	);
	let totalInWarehouse = $derived(
		inventory.warehouses.reduce((s, w) => s + w.terminal_inventory, 0)
	);
	let totalDistRate = $derived(
		inventory.warehouses.reduce((s, w) => s + w.distribution_rate, 0)
	);
</script>

<div class="panel">
	<div class="section">
		<h3>Terminal Supply Chain</h3>
		<div class="stat-row">
			<span class="muted">Produced (at factory)</span>
			<span class="mono">{totalProduced.toLocaleString()}</span>
		</div>
		<div class="stat-row">
			<span class="muted">In warehouses</span>
			<span class="mono blue">{totalInWarehouse.toLocaleString()}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Distribution rate</span>
			<span class="mono green">{totalDistRate}/tick</span>
		</div>
	</div>

	<div class="section">
		<h3>Factories ({inventory.factories.length})</h3>
		{#each inventory.factories as f}
			<div class="item-card">
				<div class="item-header">
					<span class="item-name">Factory #{f.factory_id}</span>
					<span class="tier-badge">{f.tier}</span>
				</div>
				<div class="item-stats">
					<span><span class="muted">Stored</span> <span class="mono">{f.produced_stored}</span></span>
					<span><span class="muted">Progress</span> <span class="mono">{(f.production_progress * 100).toFixed(0)}%</span></span>
				</div>
			</div>
		{:else}
			<div class="empty">No terminal factories. Build one to start producing customer terminals.</div>
		{/each}
	</div>

	<div class="section">
		<h3>Warehouses ({inventory.warehouses.length})</h3>
		{#each inventory.warehouses as w}
			<div class="item-card">
				<div class="item-header">
					<span class="item-name">Warehouse #{w.warehouse_id}</span>
				</div>
				<div class="item-stats">
					<span><span class="muted">Stock</span> <span class="mono">{w.terminal_inventory}</span></span>
					<span><span class="muted">Dist rate</span> <span class="mono green">{w.distribution_rate}/tick</span></span>
				</div>
			</div>
		{:else}
			<div class="empty">No warehouses. Build Satellite Warehouses in regions to distribute terminals to cities.</div>
		{/each}
	</div>
</div>

<style>
	.panel { color: var(--text-secondary); font-family: var(--font-sans); font-size: 13px; }
	.section { padding: 12px 16px; border-bottom: 1px solid var(--border); }
	h3 { font-size: 12px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 8px; }
	.stat-row { display: flex; justify-content: space-between; padding: 3px 0; }
	.muted { color: var(--text-muted); }
	.mono { font-family: var(--font-mono); }
	.green { color: var(--green); }
	.blue { color: var(--blue); }

	.item-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 10px; margin-bottom: 8px; }
	.item-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px; }
	.item-name { font-weight: 600; color: var(--text-primary); }
	.tier-badge { background: rgba(139, 92, 246, 0.1); color: rgb(139, 92, 246); font-size: 10px; padding: 2px 6px; border-radius: 3px; font-family: var(--font-mono); }
	.item-stats { display: flex; gap: 16px; font-size: 11px; }
	.empty { color: var(--text-dim); text-align: center; padding: 16px; font-size: 12px; }
</style>
