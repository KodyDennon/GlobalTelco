<script lang="ts">
	import { playerCorp, formatMoney } from '$lib/stores/gameState';
	import { closePanelGroup, selectedEntityId, selectedEntityType, buildMode, buildEdgeSource, showConfirm } from '$lib/stores/uiState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import type { InfraNode, InfraEdge, InfrastructureList, TrafficFlows } from '$lib/wasm/types';
	import NetworkDiagram from '$lib/charts/NetworkDiagram.svelte';
	import { tr } from '$lib/i18n/index';
	import { tooltip } from '$lib/ui/tooltip';

	let infra: InfrastructureList = $state({ nodes: [], edges: [] });
	let traffic: TrafficFlows = $state({ edge_flows: [], node_flows: [], total_served: 0, total_dropped: 0, total_demand: 0, player_served: 0, player_dropped: 0, top_congested: [] });

	$effect(() => {
		const corp = $playerCorp;
		if (corp) {
			infra = bridge.getInfrastructureList(corp.id);
			traffic = bridge.getTrafficFlows();
		}
	});

	function selectNode(id: number) {
		selectedEntityId.set(id);
		selectedEntityType.set('node');
	}

	function upgradeNode(id: number) {
		gameCommand({ UpgradeNode: { entity: id } });
	}

	function decommission(id: number) {
		showConfirm('Decommission this node? You will recover 20% of the build cost.', () => {
			gameCommand({ DecommissionNode: { entity: id } });
		});
	}

	function toggleConnectMode() {
		if ($buildMode === 'edge') {
			buildMode.set(null);
			buildEdgeSource.set(null);
		} else {
			buildMode.set('edge');
			closePanelGroup(); // Hide panel to focus on map
		}
	}

	let operationalNodes = $derived(infra.nodes.filter((n) => !n.under_construction));
	let constructingNodes = $derived(infra.nodes.filter((n) => n.under_construction));
	let totalMaintenance = $derived(operationalNodes.reduce((s, n) => s + n.maintenance_cost, 0));
</script>

<div class="panel" aria-label={$tr('panels.infrastructure')}>
	<div class="section" style="padding-top: 8px;">
		<button class="action-btn" class:active={$buildMode === 'edge'} onclick={toggleConnectMode} use:tooltip={$buildMode === 'edge' ? 'Cancel edge building mode and return to normal' : 'Enter edge building mode — click two nodes to connect them with a link'}>
			{$buildMode === 'edge' ? 'Cancel Connect' : 'Connect Nodes'}
		</button>
	</div>

	{#if operationalNodes.length === 0 && constructingNodes.length === 0 && infra.edges.length === 0}
		<div class="empty-state">
			<p class="empty-msg">No infrastructure yet — enter Build Mode to get started!</p>
			<button class="action-btn" onclick={() => { buildMode.set('node'); closePanelGroup(); }}>Enter Build Mode</button>
		</div>
	{/if}

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
		<h3>TRAFFIC SUMMARY</h3>
		<div class="summary-row">
			<span>Serving <span class="mono green">{traffic.player_served.toFixed(0)}</span> / <span class="mono">{(traffic.player_served + traffic.player_dropped).toFixed(0)}</span> demand</span>
		</div>
		{#if traffic.player_served + traffic.player_dropped > 0}
			<div class="summary-row">
				<span>Service rate: <span class="mono" class:green={traffic.player_served / (traffic.player_served + traffic.player_dropped) > 0.8} class:warn={traffic.player_served / (traffic.player_served + traffic.player_dropped) <= 0.8}>
					{((traffic.player_served / (traffic.player_served + traffic.player_dropped)) * 100).toFixed(1)}%
				</span></span>
			</div>
		{/if}
		{#if traffic.player_dropped > 0}
			<div class="summary-row">
				<span>Dropped: <span class="mono red">{traffic.player_dropped.toFixed(0)} units</span></span>
			</div>
		{/if}
	</div>

	{#if traffic.top_congested.length > 0}
		<div class="section">
			<h3>CONGESTION POINTS</h3>
			{#each traffic.top_congested as ce}
				<div class="edge-row">
					<span class="edge-type">{ce.edge_type}</span>
					<span class="mono" class:warn={ce.utilization > 0.8} class:red={ce.utilization > 1.0}>
						{(ce.utilization * 100).toFixed(0)}% util
					</span>
				</div>
			{/each}
		</div>
	{/if}

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
					<button class="tiny-btn" onclick={(e) => { e.stopPropagation(); upgradeNode(node.id); }} use:tooltip={() => `Upgrade ${node.node_type}\n+50% throughput`}>U</button>
					<button class="tiny-btn danger" onclick={(e) => { e.stopPropagation(); decommission(node.id); }} use:tooltip={() => `Decommission ${node.node_type}\nRecover 20% of build cost`}>X</button>
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

	.action-btn {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 4px 10px;
		border-radius: var(--radius-sm);
		font-size: 11px;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s;
	}

	.action-btn:hover {
		border-color: var(--blue);
		color: var(--blue);
	}

	.action-btn.active {
		background: var(--blue);
		color: white;
		border-color: var(--blue);
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

	.green {
		color: var(--green);
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

	.empty-state {
		text-align: center;
		padding: 24px 16px;
		border-bottom: 1px solid var(--border);
	}

	.empty-msg {
		color: var(--text-dim);
		font-size: 13px;
		margin-bottom: 12px;
	}
</style>
