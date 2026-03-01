<script lang="ts">
	import { playerCorp, formatMoney, worldInfo } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import type { InfraNode, InfrastructureList } from '$lib/wasm/types';
	import { tooltip } from '$lib/ui/tooltip';

	let infra: InfrastructureList = $state({ nodes: [], edges: [] });

	// Track which nodes are insured — initialized from bridge data
	let insuredNodes: Set<number> = $state(new Set());
	let initialized = false;

	$effect(() => {
		const corp = $playerCorp;
		const _tick = $worldInfo.tick;
		if (!corp) return;
		infra = bridge.getInfrastructureList(corp.id);
		// Initialize insured set from node.insured field on first load
		if (!initialized && infra.nodes.length > 0) {
			insuredNodes = new Set(infra.nodes.filter((n) => (n as any).insured).map((n) => n.id));
			initialized = true;
		}
	});

	function premiumCost(node: InfraNode): number {
		return node.construction_cost * 0.02;
	}

	function toggleInsurance(node: InfraNode) {
		if (insuredNodes.has(node.id)) {
			gameCommand({ CancelInsurance: { node: node.id } });
			insuredNodes = new Set([...insuredNodes].filter((id) => id !== node.id));
		} else {
			gameCommand({ PurchaseInsurance: { node: node.id } });
			insuredNodes = new Set([...insuredNodes, node.id]);
		}
	}

	function insureAll() {
		for (const node of operationalNodes) {
			if (!insuredNodes.has(node.id)) {
				gameCommand({ PurchaseInsurance: { node: node.id } });
			}
		}
		insuredNodes = new Set(operationalNodes.map((n) => n.id));
	}

	function cancelAll() {
		for (const node of operationalNodes) {
			if (insuredNodes.has(node.id)) {
				gameCommand({ CancelInsurance: { node: node.id } });
			}
		}
		insuredNodes = new Set();
	}

	let operationalNodes = $derived(infra.nodes.filter((n) => !n.under_construction));
	let insuredCount = $derived(operationalNodes.filter((n) => insuredNodes.has(n.id)).length);
	let uninsuredCount = $derived(operationalNodes.length - insuredCount);
	let totalPremium = $derived(
		operationalNodes.filter((n) => insuredNodes.has(n.id)).reduce((s, n) => s + premiumCost(n), 0)
	);
	let totalPotentialPremium = $derived(operationalNodes.reduce((s, n) => s + premiumCost(n), 0));
</script>

<div class="panel">
	<div class="section">
		<h3>Summary</h3>
		<div class="stat-row">
			<span class="muted">Insured nodes</span>
			<span class="mono green">{insuredCount}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Uninsured nodes</span>
			<span class="mono" class:warn={uninsuredCount > 0}>{uninsuredCount}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Total premium</span>
			<span class="mono red">{formatMoney(totalPremium)}/tick</span>
		</div>
		<div class="stat-row">
			<span class="muted">Full coverage cost</span>
			<span class="mono">{formatMoney(totalPotentialPremium)}/tick</span>
		</div>
	</div>

	<div class="section">
		<div class="section-hdr">
			<h3>Quick Actions</h3>
			<div class="btn-group">
				<button
					class="action-btn"
					onclick={insureAll}
					disabled={insuredCount === operationalNodes.length}
					use:tooltip={'Insure all operational infrastructure\nPays 2% of construction cost per tick'}
				>
					Insure All
				</button>
				<button
					class="action-btn danger"
					onclick={cancelAll}
					disabled={insuredCount === 0}
					use:tooltip={'Cancel all insurance policies'}
				>
					Cancel All
				</button>
			</div>
		</div>
	</div>

	<div class="section">
		<h3>Infrastructure ({operationalNodes.length})</h3>
		{#each operationalNodes as node}
			<div class="node-row">
				<div class="node-info">
					<span class="node-type">{node.node_type}</span>
					<div class="node-stats">
						<span>
							<span class="muted">HP</span>
							<span class="mono" class:warn={node.health < 0.5}
								>{(node.health * 100).toFixed(0)}%</span
							>
						</span>
						<span>
							<span class="muted">Premium</span>
							<span class="mono">{formatMoney(premiumCost(node))}/tick</span>
						</span>
					</div>
				</div>
				<div class="node-actions">
					<button
						class="toggle-btn"
						class:insured={insuredNodes.has(node.id)}
						onclick={() => toggleInsurance(node)}
						use:tooltip={() =>
							insuredNodes.has(node.id)
								? `Cancel insurance on ${node.node_type}\nSaves ${formatMoney(premiumCost(node))}/tick`
								: `Insure ${node.node_type}\nCosts ${formatMoney(premiumCost(node))}/tick (2% of ${formatMoney(node.construction_cost)})`}
					>
						{insuredNodes.has(node.id) ? 'Insured' : 'Uninsured'}
					</button>
				</div>
			</div>
		{/each}
		{#if operationalNodes.length === 0}
			<div class="empty">No operational infrastructure to insure.</div>
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

	.green {
		color: var(--green);
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

	.action-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.action-btn.danger {
		color: var(--red);
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

	.node-actions {
		display: flex;
		gap: 4px;
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

	.toggle-btn.insured {
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
