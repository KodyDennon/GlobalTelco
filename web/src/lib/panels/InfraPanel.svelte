<script lang="ts">
	import { playerCorp, formatMoney } from '$lib/stores/gameState';
	import { activePanel, selectedEntityId, selectedEntityType } from '$lib/stores/uiState';
	import * as bridge from '$lib/wasm/bridge';
	import type { InfraNode, InfraEdge, InfrastructureList } from '$lib/wasm/types';
	import NetworkDiagram from '$lib/charts/NetworkDiagram.svelte';
	import { tr } from '$lib/i18n/index';

	let infra: InfrastructureList = $state({ nodes: [], edges: [] });

	$effect(() => {
		const corp = $playerCorp;
		if (corp) {
			infra = bridge.getInfrastructureList(corp.id);
		}
	});

	function selectNode(id: number) {
		selectedEntityId.set(id);
		selectedEntityType.set('node');
	}

	function upgradeNode(id: number) {
		bridge.processCommand({ UpgradeNode: { entity: id } });
	}

	function decommission(id: number) {
		bridge.processCommand({ DecommissionNode: { entity: id } });
	}

	function close() {
		activePanel.set('none');
	}

	let operationalNodes = $derived(infra.nodes.filter((n) => !n.under_construction));
	let constructingNodes = $derived(infra.nodes.filter((n) => n.under_construction));
	let totalRevenue = $derived(
		operationalNodes.reduce((s, n) => s + n.max_throughput * n.utilization * 5, 0)
	);
	let totalMaintenance = $derived(operationalNodes.reduce((s, n) => s + n.maintenance_cost, 0));
</script>

<div class="panel" aria-label={$tr('panels.infrastructure')}>
	<div class="panel-header">
		<span class="title">{$tr('panels.infrastructure')}</span>
		<button class="close" onclick={close}>x</button>
	</div>

	<div class="section">
		<h3>{$tr('panels.summary')}</h3>
		<div class="summary-row">
			<span>{operationalNodes.length} operational</span>
			<span>{constructingNodes.length} building</span>
			<span>{infra.edges.length} edges</span>
		</div>
		<div class="summary-row">
			<span>{$tr('panels.maintenance')}: <span class="mono red">{formatMoney(totalMaintenance)}/tick</span></span>
		</div>
	</div>

	<div class="section">
		<h3>{$tr('panels.network_topology')}</h3>
		<NetworkDiagram />
	</div>

	{#if constructingNodes.length > 0}
		<div class="section">
			<h3>{$tr('panels.under_construction')}</h3>
			{#each constructingNodes as node}
				<button class="node-row construction" onclick={() => selectNode(node.id)}>
					<div class="node-info">
						<span class="node-type">{node.node_type}</span>
						<span class="node-level">{node.network_level}</span>
					</div>
					<span class="badge building">{$tr('panels.building')}</span>
				</button>
			{/each}
		</div>
	{/if}

	<div class="section">
		<h3>{$tr('panels.operational_nodes', { count: operationalNodes.length })}</h3>
		{#each operationalNodes as node}
			<div class="node-row" role="button" tabindex="0" onclick={() => selectNode(node.id)} onkeydown={(e) => { if (e.key === 'Enter') selectNode(node.id); }}>
				<div class="node-info">
					<span class="node-type">{node.node_type}</span>
					<div class="node-stats">
						<span>
							<span class="muted">HP</span>
							<span class="mono" class:warn={node.health < 0.5}>{(node.health * 100).toFixed(0)}%</span>
						</span>
						<span>
							<span class="muted">{$tr('panels.utilization')}</span>
							<span class="mono">{(node.utilization * 100).toFixed(0)}%</span>
						</span>
						<span>
							<span class="muted">Maint</span>
							<span class="mono">{formatMoney(node.maintenance_cost)}</span>
						</span>
					</div>
				</div>
				<div class="node-actions">
					<button class="tiny-btn" onclick={(e) => { e.stopPropagation(); upgradeNode(node.id); }} title="Upgrade">U</button>
					<button class="tiny-btn danger" onclick={(e) => { e.stopPropagation(); decommission(node.id); }} title="Decommission">X</button>
				</div>
			</div>
		{/each}
	</div>

	{#if infra.edges.length > 0}
		<div class="section">
			<h3>{$tr('panels.connections', { count: infra.edges.length })}</h3>
			{#each infra.edges as edge}
				<div class="edge-row">
					<span class="edge-type">{edge.edge_type}</span>
					<span class="mono">{edge.bandwidth.toFixed(0)} bw</span>
					<span class="mono">{(edge.utilization * 100).toFixed(0)}% util</span>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.panel {
		color: var(--text-secondary);
		font-family: var(--font-sans);
		font-size: 13px;
	}

	.panel-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 12px 16px;
		border-bottom: 1px solid var(--border);
		position: sticky;
		top: 0;
		background: var(--bg-panel);
		z-index: 1;
	}

	.title {
		font-weight: 700;
		font-size: 14px;
		color: var(--text-primary);
	}

	.close {
		background: none;
		border: none;
		color: var(--text-dim);
		cursor: pointer;
		font-size: 16px;
	}

	.section {
		padding: 12px 16px;
		border-bottom: 1px solid var(--border);
	}

	h3 {
		font-size: 12px;
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin-bottom: 8px;
	}

	.summary-row {
		display: flex;
		gap: 16px;
		font-size: 12px;
		color: var(--text-muted);
		margin-bottom: 4px;
	}

	.mono {
		font-family: var(--font-mono);
	}

	.red {
		color: var(--red);
	}

	.muted {
		color: var(--text-dim);
		font-size: 11px;
	}

	.node-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition: background 0.15s;
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
		background: transparent;
		border-top: none;
		border-left: none;
		border-right: none;
		width: 100%;
		color: inherit;
		font: inherit;
		text-align: left;
	}

	.node-row:hover {
		background: var(--bg-surface);
	}

	.node-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.node-type {
		font-weight: 600;
		color: var(--text-primary);
	}

	.node-level {
		font-size: 11px;
		color: var(--text-dim);
	}

	.node-stats {
		display: flex;
		gap: 12px;
		font-size: 11px;
	}

	.warn {
		color: var(--red);
	}

	.badge {
		font-size: 10px;
		padding: 2px 6px;
		border-radius: var(--radius-sm);
		font-weight: 600;
	}

	.badge.building {
		background: var(--amber-bg);
		color: var(--amber);
	}

	.node-actions {
		display: flex;
		gap: 4px;
	}

	.tiny-btn {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--text-muted);
		width: 24px;
		height: 24px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 11px;
		font-weight: 600;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.tiny-btn:hover {
		color: var(--blue);
		border-color: var(--blue);
	}

	.tiny-btn.danger:hover {
		color: var(--red);
		border-color: var(--red);
	}

	.edge-row {
		display: flex;
		gap: 12px;
		padding: 4px 0;
		font-size: 12px;
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
	}

	.edge-type {
		color: var(--text-primary);
		min-width: 80px;
	}
</style>
