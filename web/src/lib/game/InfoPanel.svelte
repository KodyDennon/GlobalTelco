<script lang="ts">
	import { selectedEntityId, selectedEntityType } from '$lib/stores/uiState';
	import { cities, formatMoney, formatPopulation } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import type { InfraNode } from '$lib/wasm/types';

	let entityData: any = $state(null);
	let isPlayerOwned = $state(false);

	$effect(() => {
		const id = $selectedEntityId;
		const type = $selectedEntityType;
		if (id === null) {
			entityData = null;
			isPlayerOwned = false;
			return;
		}

		if (type === 'city') {
			const city = $cities.find((c) => c.id === id);
			entityData = city ? { ...city, entityType: 'city' } : null;
			isPlayerOwned = false;
		} else if (type === 'node') {
			const corpId = bridge.getPlayerCorpId();
			const infra = bridge.getInfrastructureList(corpId);
			const playerNode = infra.nodes.find((n: InfraNode) => n.id === id);
			if (playerNode) {
				entityData = { ...playerNode, entityType: 'node' };
				isPlayerOwned = true;
			} else {
				const allInfra = bridge.getAllInfrastructure();
				const anyNode = allInfra.nodes.find((n: any) => n.id === id);
				entityData = anyNode ? { ...anyNode, entityType: 'node' } : null;
				isPlayerOwned = false;
			}
		} else if (type === 'edge') {
			const allInfra = bridge.getAllInfrastructure();
			const edge = allInfra.edges.find((e: any) => e.id === id);
			if (edge) {
				const corpId = bridge.getPlayerCorpId();
				const playerInfra = bridge.getInfrastructureList(corpId);
				const isOwned = playerInfra.edges.some((e: any) => e.id === id);
				entityData = { ...edge, entityType: 'edge' };
				isPlayerOwned = isOwned;
			} else {
				entityData = null;
				isPlayerOwned = false;
			}
		} else {
			entityData = null;
			isPlayerOwned = false;
		}
	});

	function close() {
		selectedEntityId.set(null);
		selectedEntityType.set(null);
	}

	function repairNode() {
		if (!entityData) return;
		bridge.processCommand({ RepairNode: { entity: entityData.id } });
	}

	function emergencyRepair() {
		if (!entityData) return;
		bridge.processCommand({ EmergencyRepair: { entity: entityData.id } });
	}

	function upgradeNode() {
		if (!entityData) return;
		bridge.processCommand({ UpgradeNode: { entity: entityData.id } });
	}

	function decommissionNode() {
		if (!entityData) return;
		bridge.processCommand({ DecommissionNode: { entity: entityData.id } });
		close();
	}

	function toggleInsurance() {
		if (!entityData) return;
		// Toggle based on current state (we don't have insured field yet, so just purchase)
		bridge.processCommand({ PurchaseInsurance: { node: entityData.id } });
	}
</script>

{#if entityData}
	<div class="info-panel">
		<div class="panel-header">
			<span class="panel-title">
				{entityData.entityType === 'city' ? entityData.name : entityData.entityType === 'edge' ? entityData.edge_type : entityData.node_type}
			</span>
			{#if !isPlayerOwned && entityData.entityType === 'node' && entityData.owner_name}
				<span class="owner-tag">{entityData.owner_name}</span>
			{/if}
			<button class="close-btn" onclick={close}>x</button>
		</div>

		<div class="panel-body">
			{#if entityData.entityType === 'edge'}
				<div class="stat">
					<span class="label">Type</span>
					<span class="value">{entityData.edge_type}</span>
				</div>
				<div class="stat">
					<span class="label">Bandwidth</span>
					<span class="value">{entityData.bandwidth?.toFixed(0) ?? '—'}</span>
				</div>
				<div class="stat">
					<span class="label">Load</span>
					<span class="value">{entityData.current_load?.toFixed(0) ?? '—'}</span>
				</div>
				<div class="stat">
					<span class="label">Latency</span>
					<span class="value">{entityData.latency_ms?.toFixed(1) ?? '—'} ms</span>
				</div>
				<div class="stat">
					<span class="label">Length</span>
					<span class="value">{entityData.length_km?.toFixed(1) ?? '—'} km</span>
				</div>
				<div class="stat">
					<span class="label">Health</span>
					<span class="value" class:damaged={entityData.health < 0.5} class:warn-health={entityData.health < 0.8 && entityData.health >= 0.5}>
						{((entityData.health ?? 1) * 100).toFixed(0)}%
					</span>
				</div>
			{:else if entityData.entityType === 'city'}
				<div class="stat">
					<span class="label">Population</span>
					<span class="value">{formatPopulation(entityData.population)}</span>
				</div>
				<div class="stat">
					<span class="label">Development</span>
					<span class="value">{(entityData.development * 100).toFixed(0)}%</span>
				</div>
				<div class="stat">
					<span class="label">Telecom Demand</span>
					<span class="value">{entityData.telecom_demand.toFixed(0)}</span>
				</div>
				<div class="stat">
					<span class="label">Satisfaction</span>
					<span class="value sat" class:good={entityData.infrastructure_satisfaction >= 0.7} class:warn={entityData.infrastructure_satisfaction >= 0.4 && entityData.infrastructure_satisfaction < 0.7} class:bad={entityData.infrastructure_satisfaction < 0.4}>
						{(entityData.infrastructure_satisfaction * 100).toFixed(0)}%
					</span>
				</div>
				<div class="stat">
					<span class="label">Employment</span>
					<span class="value">{(entityData.employment_rate * 100).toFixed(0)}%</span>
				</div>
				<div class="stat">
					<span class="label">Growth</span>
					<span class="value" class:positive={entityData.migration_pressure > 0} class:negative-val={entityData.migration_pressure < 0}>
						{entityData.migration_pressure > 0 ? '+' : ''}{(entityData.migration_pressure * 100).toFixed(1)}
					</span>
				</div>
			{:else if entityData.entityType === 'node'}
				<div class="stat">
					<span class="label">Type</span>
					<span class="value">{entityData.node_type}</span>
				</div>
				<div class="stat">
					<span class="label">Throughput</span>
					<span class="value">{entityData.max_throughput.toFixed(0)}</span>
				</div>
				<div class="stat">
					<span class="label">Utilization</span>
					<span class="value" class:high-util={entityData.utilization > 0.8}>{(entityData.utilization * 100).toFixed(1)}%</span>
				</div>
				<div class="stat">
					<span class="label">Health</span>
					<span class="value" class:damaged={entityData.health < 0.5} class:warn-health={entityData.health < 0.8 && entityData.health >= 0.5}>
						{(entityData.health * 100).toFixed(0)}%
					</span>
				</div>
				<div class="stat">
					<span class="label">Latency</span>
					<span class="value">{entityData.latency_ms.toFixed(1)} ms</span>
				</div>
				{#if isPlayerOwned}
					<div class="stat">
						<span class="label">Maintenance</span>
						<span class="value">{formatMoney(entityData.maintenance_cost)}/tick</span>
					</div>
				{/if}
				{#if entityData.under_construction}
					<div class="status-badge construction">Under Construction</div>
				{/if}

				{#if isPlayerOwned && !entityData.under_construction}
					<div class="action-buttons">
						{#if entityData.health < 0.95}
							<button class="action-btn repair" onclick={repairNode} title="Repair to full health">Repair</button>
							<button class="action-btn emergency" onclick={emergencyRepair} title="Instant repair (3x cost)">Emergency</button>
						{/if}
						<button class="action-btn upgrade" onclick={upgradeNode} title="Upgrade throughput +50% (cost: {formatMoney(Math.floor((entityData.construction_cost ?? 0) / 2))})">Upgrade</button>
						<button class="action-btn insurance" onclick={toggleInsurance} title="Purchase disaster insurance">Insure</button>
						<button class="action-btn decommission" onclick={decommissionNode} title="Decommission (recover 20% cost)">Decom</button>
					</div>
				{/if}
			{/if}
		</div>
	</div>
{/if}

<style>
	.info-panel {
		position: absolute;
		right: 16px;
		top: 64px;
		width: 280px;
		background: rgba(17, 24, 39, 0.95);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 8px;
		z-index: 20;
		font-family: system-ui, sans-serif;
		color: #d1d5db;
	}

	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 16px;
		border-bottom: 1px solid rgba(55, 65, 81, 0.5);
	}

	.panel-title {
		font-weight: 600;
		font-size: 14px;
		color: #f3f4f6;
	}

	.close-btn {
		background: none;
		border: none;
		color: #6b7280;
		cursor: pointer;
		font-size: 16px;
		padding: 0 4px;
	}

	.panel-body {
		padding: 12px 16px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.stat {
		display: flex;
		justify-content: space-between;
		font-size: 13px;
	}

	.label {
		color: #9ca3af;
	}

	.value {
		color: #f3f4f6;
		font-family: 'SF Mono', 'Fira Code', monospace;
	}

	.status-badge {
		text-align: center;
		padding: 4px 8px;
		border-radius: 4px;
		font-size: 12px;
		font-weight: 600;
	}

	.status-badge.construction {
		background: rgba(245, 158, 11, 0.2);
		color: #f59e0b;
	}

	.owner-tag {
		font-size: 11px;
		padding: 2px 6px;
		border-radius: 3px;
		background: rgba(139, 92, 246, 0.15);
		color: #a78bfa;
		margin-left: auto;
		margin-right: 8px;
	}

	.sat.good { color: #10b981; }
	.sat.warn { color: #f59e0b; }
	.sat.bad { color: #ef4444; }

	.positive { color: #10b981; }
	.negative-val { color: #ef4444; }

	.high-util { color: #f59e0b; }
	.damaged { color: #ef4444; font-weight: 600; }
	.warn-health { color: #f59e0b; }

	.action-buttons {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
		margin-top: 8px;
		padding-top: 8px;
		border-top: 1px solid rgba(55, 65, 81, 0.5);
	}

	.action-btn {
		flex: 1;
		min-width: 60px;
		padding: 6px 8px;
		border-radius: 4px;
		border: 1px solid rgba(55, 65, 81, 0.5);
		background: rgba(31, 41, 55, 0.8);
		color: #d1d5db;
		font-size: 11px;
		font-family: system-ui, sans-serif;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.15s;
		text-align: center;
	}

	.action-btn:hover { background: rgba(55, 65, 81, 0.5); }

	.action-btn.repair { color: #10b981; border-color: rgba(16, 185, 129, 0.3); }
	.action-btn.repair:hover { background: rgba(16, 185, 129, 0.15); }

	.action-btn.emergency { color: #f59e0b; border-color: rgba(245, 158, 11, 0.3); }
	.action-btn.emergency:hover { background: rgba(245, 158, 11, 0.15); }

	.action-btn.upgrade { color: #3b82f6; border-color: rgba(59, 130, 246, 0.3); }
	.action-btn.upgrade:hover { background: rgba(59, 130, 246, 0.15); }

	.action-btn.insurance { color: #8b5cf6; border-color: rgba(139, 92, 246, 0.3); }
	.action-btn.insurance:hover { background: rgba(139, 92, 246, 0.15); }

	.action-btn.decommission { color: #ef4444; border-color: rgba(239, 68, 68, 0.3); }
	.action-btn.decommission:hover { background: rgba(239, 68, 68, 0.15); }
</style>
