<script lang="ts">
	import { playerCorp, formatMoney } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import type { DamagedNode } from '$lib/wasm/types';
	import { tooltip } from '$lib/ui/tooltip';

	let damagedNodes: DamagedNode[] = $state([]);

	// Track nodes currently being repaired (standard or emergency)
	let repairingNodes: Map<number, 'standard' | 'emergency'> = $state(new Map());

	$effect(() => {
		const corp = $playerCorp;
		if (corp) {
			damagedNodes = bridge.getDamagedNodes(corp.id);
		}
	});

	function repairNode(node: DamagedNode) {
		gameCommand({ RepairNode: { entity: node.id } });
		repairingNodes = new Map([...repairingNodes, [node.id, 'standard']]);
	}

	function emergencyRepair(node: DamagedNode) {
		gameCommand({ EmergencyRepair: { entity: node.id } });
		repairingNodes = new Map([...repairingNodes, [node.id, 'emergency']]);
	}

	function repairAll() {
		for (const node of sortedNodes) {
			if (!repairingNodes.has(node.id)) {
				gameCommand({ RepairNode: { entity: node.id } });
			}
		}
		repairingNodes = new Map(sortedNodes.map((n) => [n.id, 'standard']));
	}

	let sortedNodes = $derived([...damagedNodes].sort((a, b) => a.health - b.health));
	let criticalCount = $derived(sortedNodes.filter((n) => n.health < 0.3).length);
	let totalRepairCost = $derived(sortedNodes.reduce((s, n) => s + n.repair_cost, 0));
	let totalEmergencyCost = $derived(sortedNodes.reduce((s, n) => s + n.emergency_cost, 0));

	function severityClass(health: number): string {
		if (health < 0.3) return 'critical';
		if (health < 0.6) return 'damaged';
		return 'minor';
	}

	function severityLabel(health: number): string {
		if (health < 0.3) return 'Critical';
		if (health < 0.6) return 'Damaged';
		return 'Minor';
	}
</script>

<div class="panel">
	<div class="section">
		<h3>Repair Summary</h3>
		<div class="stat-row">
			<span class="muted">Damaged infrastructure</span>
			<span class="mono" class:warn={sortedNodes.length > 0}>{sortedNodes.length}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Critical (below 30%)</span>
			<span class="mono" class:critical={criticalCount > 0}>{criticalCount}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Total repair cost</span>
			<span class="mono">{formatMoney(totalRepairCost)}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Emergency repair cost</span>
			<span class="mono red">{formatMoney(totalEmergencyCost)}</span>
		</div>
	</div>

	{#if sortedNodes.length > 0}
		<div class="section">
			<div class="section-hdr">
				<h3>Quick Actions</h3>
				<button
					class="action-btn"
					onclick={repairAll}
					use:tooltip={'Queue standard repairs for all damaged infrastructure\nRepairs happen over time at regular cost'}
				>
					Repair All
				</button>
			</div>
		</div>

		<div class="section">
			<h3>Damaged Nodes ({sortedNodes.length})</h3>
			{#each sortedNodes as node}
				{@const repairing = repairingNodes.get(node.id)}
				<div class="node-row">
					<div class="node-info">
						<div class="node-header">
							<span class="node-type">{node.node_type}</span>
							<span class="badge {severityClass(node.health)}"
								>{severityLabel(node.health)}</span
							>
						</div>
						<div class="health-bar-container">
							<div
								class="health-bar {severityClass(node.health)}"
								style="width: {node.health * 100}%"
							></div>
						</div>
						<div class="node-stats">
							<span>
								<span class="muted">HP</span>
								<span class="mono {severityClass(node.health)}"
									>{(node.health * 100).toFixed(0)}%</span
								>
							</span>
							<span>
								<span class="muted">Repair</span>
								<span class="mono">{formatMoney(node.repair_cost)}</span>
							</span>
							<span>
								<span class="muted">Emergency</span>
								<span class="mono red">{formatMoney(node.emergency_cost)}</span>
							</span>
						</div>
					</div>
					<div class="node-actions">
						{#if repairing}
							<span class="badge repairing">
								{repairing === 'emergency' ? 'Rush' : 'Repairing'}
							</span>
						{:else}
							<button
								class="repair-btn"
								onclick={() => repairNode(node)}
								use:tooltip={() =>
									`Standard repair for ${node.node_type}\nCost: ${formatMoney(node.repair_cost)}\nRepairs over several ticks`}
							>
								Repair
							</button>
							<button
								class="emergency-btn"
								onclick={() => emergencyRepair(node)}
								use:tooltip={() =>
									`Emergency repair for ${node.node_type}\nCost: ${formatMoney(node.emergency_cost)} (higher)\nInstant repair`}
							>
								Rush
							</button>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	{:else}
		<div class="section">
			<div class="empty">All infrastructure is in good condition.</div>
		</div>
	{/if}
</div>

<style>
	.panel {
		color: var(--text-secondary);
		font-family: var(--font-sans);
		font-size: 13px;
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

	.stat-row {
		display: flex;
		justify-content: space-between;
		padding: 3px 0;
	}

	.muted {
		color: var(--text-muted);
	}

	.mono {
		font-family: var(--font-mono);
	}

	.red {
		color: var(--red);
	}

	.warn {
		color: var(--red);
	}

	.section-hdr {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 8px;
	}

	.action-btn {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--blue);
		padding: 4px 10px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 11px;
		font-family: var(--font-mono);
	}

	.action-btn:hover {
		background: var(--bg-hover);
	}

	.node-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px;
		border-radius: var(--radius-sm);
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
	}

	.node-row:hover {
		background: var(--bg-surface);
	}

	.node-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
		flex: 1;
		min-width: 0;
	}

	.node-header {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.node-type {
		font-weight: 600;
		color: var(--text-primary);
	}

	.node-stats {
		display: flex;
		gap: 12px;
		font-size: 11px;
	}

	.health-bar-container {
		width: 100%;
		height: 4px;
		background: rgba(55, 65, 81, 0.3);
		border-radius: 2px;
		overflow: hidden;
	}

	.health-bar {
		height: 100%;
		border-radius: 2px;
		transition: width 0.3s ease;
	}

	.health-bar.critical {
		background: var(--red);
	}

	.health-bar.damaged {
		background: var(--amber, #f59e0b);
	}

	.health-bar.minor {
		background: var(--green);
	}

	.badge {
		font-size: 10px;
		padding: 2px 6px;
		border-radius: var(--radius-sm);
		font-weight: 600;
	}

	.badge.critical {
		background: var(--red-bg);
		color: var(--red);
	}

	.badge.damaged {
		background: var(--amber-bg, rgba(245, 158, 11, 0.1));
		color: var(--amber, #f59e0b);
	}

	.badge.minor {
		background: var(--green-bg);
		color: var(--green);
	}

	.badge.repairing {
		background: rgba(59, 130, 246, 0.1);
		color: var(--blue);
	}

	.critical {
		color: var(--red);
	}

	.node-actions {
		display: flex;
		gap: 4px;
		margin-left: 8px;
	}

	.repair-btn {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--blue);
		padding: 4px 10px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 11px;
		font-weight: 600;
	}

	.repair-btn:hover {
		border-color: var(--blue);
		background: rgba(59, 130, 246, 0.1);
	}

	.emergency-btn {
		background: var(--red-bg);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: var(--red);
		padding: 4px 10px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 11px;
		font-weight: 600;
	}

	.emergency-btn:hover {
		background: rgba(239, 68, 68, 0.15);
	}

	.empty {
		color: var(--text-dim);
		text-align: center;
		padding: 16px;
		font-size: 12px;
	}
</style>
