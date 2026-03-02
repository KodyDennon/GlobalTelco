<script lang="ts">
	import { playerCorp, formatMoney, worldInfo } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import type { InfraNode, InfrastructureList } from '$lib/wasm/types';
	import { tooltip } from '$lib/ui/tooltip';

	type MaintenancePriority = 'Critical' | 'Standard' | 'Low' | 'Deferred';

	interface PriorityInfo {
		label: MaintenancePriority;
		costMultiplier: number;
		color: string;
		description: string;
	}

	const PRIORITIES: PriorityInfo[] = [
		{ label: 'Critical', costMultiplier: 1.5, color: 'var(--red)', description: 'Immediate repair, 1.5x cost' },
		{ label: 'Standard', costMultiplier: 1.0, color: 'var(--blue)', description: 'Normal schedule, 1.0x cost' },
		{ label: 'Low', costMultiplier: 0.7, color: 'var(--text-muted)', description: 'Delayed repair, 0.7x cost' },
		{ label: 'Deferred', costMultiplier: 0.0, color: 'var(--text-dim)', description: 'No maintenance, 0.0x cost' }
	];

	let infra: InfrastructureList = $state({ nodes: [], edges: [] });

	// Track priority and auto-repair per node — initialized from bridge data
	let nodePriorities: Map<number, MaintenancePriority> = $state(new Map());
	let nodeAutoRepair: Map<number, boolean> = $state(new Map());

	$effect(() => {
		const corp = $playerCorp;
		const tick = $worldInfo.tick;
		if (tick % 5 !== 0) return;
		if (!corp) return;
		infra = bridge.getInfrastructureList(corp.id);
		// Initialize priority/auto-repair from node data (fields added to infra list)
		for (const node of infra.nodes) {
			if ((node as any).maintenance_priority && !nodePriorities.has(node.id)) {
				nodePriorities.set(node.id, (node as any).maintenance_priority as MaintenancePriority);
			}
			if ((node as any).auto_repair !== undefined && !nodeAutoRepair.has(node.id)) {
				nodeAutoRepair.set(node.id, (node as any).auto_repair);
			}
		}
	});

	function getPriority(nodeId: number): MaintenancePriority {
		return nodePriorities.get(nodeId) ?? 'Standard';
	}

	function getAutoRepair(nodeId: number): boolean {
		return nodeAutoRepair.get(nodeId) ?? false;
	}

	function getPriorityInfo(priority: MaintenancePriority): PriorityInfo {
		return PRIORITIES.find((p) => p.label === priority) ?? PRIORITIES[1];
	}

	function setPriority(node: InfraNode, priority: MaintenancePriority) {
		gameCommand({
			SetMaintenancePriority: {
				entity: node.id,
				priority,
				auto_repair: getAutoRepair(node.id)
			}
		});
		nodePriorities = new Map([...nodePriorities, [node.id, priority]]);
	}

	function toggleAutoRepair(node: InfraNode) {
		const newValue = !getAutoRepair(node.id);
		gameCommand({
			SetMaintenancePriority: {
				entity: node.id,
				priority: getPriority(node.id),
				auto_repair: newValue
			}
		});
		nodeAutoRepair = new Map([...nodeAutoRepair, [node.id, newValue]]);
	}

	function setAllPriority(priority: MaintenancePriority) {
		for (const node of operationalNodes) {
			gameCommand({
				SetMaintenancePriority: {
					entity: node.id,
					priority,
					auto_repair: getAutoRepair(node.id)
				}
			});
		}
		nodePriorities = new Map(operationalNodes.map((n) => [n.id, priority]));
	}

	function maintenanceCost(node: InfraNode): number {
		const priority = getPriority(node.id);
		const info = getPriorityInfo(priority);
		return node.maintenance_cost * info.costMultiplier;
	}

	let operationalNodes = $derived(infra.nodes.filter((n) => !n.under_construction));

	// Sort: critical health first, then by priority
	let sortedNodes = $derived(
		[...operationalNodes].sort((a, b) => {
			const priorityOrder: Record<MaintenancePriority, number> = { Critical: 0, Standard: 1, Low: 2, Deferred: 3 };
			const aPriority = priorityOrder[getPriority(a.id)] ?? 1;
			const bPriority = priorityOrder[getPriority(b.id)] ?? 1;
			if (aPriority !== bPriority) return aPriority - bPriority;
			return a.health - b.health;
		})
	);

	let criticalCount = $derived(operationalNodes.filter((n) => getPriority(n.id) === 'Critical').length);
	let deferredCount = $derived(operationalNodes.filter((n) => getPriority(n.id) === 'Deferred').length);
	let autoRepairCount = $derived(operationalNodes.filter((n) => getAutoRepair(n.id)).length);
	let totalMaintenanceCost = $derived(operationalNodes.reduce((s, n) => s + maintenanceCost(n), 0));
	let damagedCount = $derived(operationalNodes.filter((n) => n.health < 1.0).length);

	// Nodes currently in the maintenance queue (health < 1.0 and not deferred)
	let maintenanceQueue = $derived(
		operationalNodes
			.filter((n) => n.health < 1.0 && getPriority(n.id) !== 'Deferred')
			.sort((a, b) => a.health - b.health)
	);

	function healthClass(health: number): string {
		if (health < 0.3) return 'critical';
		if (health < 0.6) return 'damaged';
		if (health < 1.0) return 'minor';
		return 'healthy';
	}
</script>

<div class="panel">
	<div class="section">
		<h3>Maintenance Summary</h3>
		<div class="stat-row">
			<span class="muted">Total infrastructure</span>
			<span class="mono">{operationalNodes.length}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Damaged nodes</span>
			<span class="mono" class:warn={damagedCount > 0}>{damagedCount}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Critical priority</span>
			<span class="mono red">{criticalCount}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Deferred (no maintenance)</span>
			<span class="mono">{deferredCount}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Auto-repair enabled</span>
			<span class="mono blue">{autoRepairCount}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Total maintenance cost</span>
			<span class="mono red">{formatMoney(totalMaintenanceCost)}/tick</span>
		</div>
	</div>

	<div class="section">
		<h3>Cost Multipliers</h3>
		<div class="multiplier-grid">
			{#each PRIORITIES as priority}
				<div class="multiplier-card">
					<span class="priority-label" style="color: {priority.color}">{priority.label}</span>
					<span class="mono" style="color: {priority.color}">{priority.costMultiplier.toFixed(1)}x</span>
				</div>
			{/each}
		</div>
	</div>

	<div class="section">
		<div class="section-hdr">
			<h3>Quick Actions</h3>
			<div class="btn-group">
				<button
					class="action-btn"
					onclick={() => setAllPriority('Critical')}
					use:tooltip={'Set all nodes to Critical priority\n1.5x maintenance cost, immediate repairs'}
				>
					All Critical
				</button>
				<button
					class="action-btn"
					onclick={() => setAllPriority('Standard')}
					use:tooltip={'Set all nodes to Standard priority\n1.0x maintenance cost, normal schedule'}
				>
					All Standard
				</button>
				<button
					class="action-btn danger"
					onclick={() => setAllPriority('Deferred')}
					use:tooltip={'Defer all maintenance\nNo cost, but infrastructure will degrade'}
				>
					Defer All
				</button>
			</div>
		</div>
	</div>

	{#if maintenanceQueue.length > 0}
		<div class="section">
			<h3>Repair Queue ({maintenanceQueue.length})</h3>
			{#each maintenanceQueue as node}
				{@const priority = getPriority(node.id)}
				{@const priorityInfo = getPriorityInfo(priority)}
				<div class="queue-row">
					<span class="node-type">{node.node_type}</span>
					<span class="badge {healthClass(node.health)}">{(node.health * 100).toFixed(0)}%</span>
					<span class="badge" style="color: {priorityInfo.color}">{priority}</span>
				</div>
			{/each}
		</div>
	{/if}

	<div class="section">
		<h3>Infrastructure ({sortedNodes.length})</h3>
		{#each sortedNodes as node}
			{@const priority = getPriority(node.id)}
			{@const priorityInfo = getPriorityInfo(priority)}
			{@const autoRepair = getAutoRepair(node.id)}
			<div class="node-row">
				<div class="node-info">
					<div class="node-header">
						<span class="node-type">{node.node_type}</span>
						<span class="badge {healthClass(node.health)}">{(node.health * 100).toFixed(0)}%</span>
					</div>
					<div class="health-bar-container">
						<div
							class="health-bar {healthClass(node.health)}"
							style="width: {node.health * 100}%"
						></div>
					</div>
					<div class="node-stats">
						<span>
							<span class="muted">Cost</span>
							<span class="mono">{formatMoney(maintenanceCost(node))}/tick</span>
						</span>
						<span>
							<span class="muted">Priority</span>
							<span class="mono" style="color: {priorityInfo.color}">{priority}</span>
						</span>
					</div>
				</div>
				<div class="node-actions">
					<select
						class="priority-select"
						value={priority}
						onchange={(e) => setPriority(node, (e.target as HTMLSelectElement).value as MaintenancePriority)}
					>
						{#each PRIORITIES as p}
							<option value={p.label}>{p.label} ({p.costMultiplier}x)</option>
						{/each}
					</select>
					<button
						class="toggle-btn"
						class:active={autoRepair}
						onclick={() => toggleAutoRepair(node)}
						use:tooltip={() =>
							autoRepair
								? `Disable auto-repair for ${node.node_type}\nDamage will not be automatically repaired`
								: `Enable auto-repair for ${node.node_type}\nAutomatically repairs when damaged`}
					>
						{autoRepair ? 'Auto' : 'Manual'}
					</button>
				</div>
			</div>
		{/each}
		{#if operationalNodes.length === 0}
			<div class="empty">No operational infrastructure to maintain.</div>
		{/if}
	</div>
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

	.blue {
		color: var(--blue);
	}

	.warn {
		color: var(--red);
	}

	.multiplier-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 6px;
	}

	.multiplier-card {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		padding: 6px;
		text-align: center;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.priority-label {
		font-weight: 700;
		font-size: 11px;
	}

	.section-hdr {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 8px;
	}

	.btn-group {
		display: flex;
		gap: 4px;
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

	.action-btn.danger {
		color: var(--red);
	}

	.queue-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px 0;
		font-size: 12px;
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
		background: var(--blue);
	}

	.health-bar.healthy {
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
		background: rgba(59, 130, 246, 0.1);
		color: var(--blue);
	}

	.badge.healthy {
		background: var(--green-bg);
		color: var(--green);
	}

	.node-actions {
		display: flex;
		gap: 4px;
		margin-left: 8px;
		align-items: center;
	}

	.priority-select {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--text-primary);
		padding: 4px 6px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 11px;
		font-family: var(--font-mono);
	}

	.priority-select:hover {
		border-color: var(--blue);
	}

	.priority-select:focus {
		outline: none;
		border-color: var(--blue);
	}

	.toggle-btn {
		background: var(--red-bg);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: var(--red);
		padding: 4px 10px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 11px;
		font-weight: 600;
	}

	.toggle-btn.active {
		background: var(--green-bg);
		border: 1px solid var(--green-border);
		color: var(--green);
	}

	.empty {
		color: var(--text-dim);
		text-align: center;
		padding: 16px;
		font-size: 12px;
	}
</style>
