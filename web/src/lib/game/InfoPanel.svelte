<script lang="ts">
	import { selectedEntityId, selectedEntityType, activePanel } from '$lib/stores/uiState';
	import { cities, regions, formatMoney, formatPopulation } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import type { InfraNode } from '$lib/wasm/types';

	let entityData: any = $state(null);

	$effect(() => {
		const id = $selectedEntityId;
		const type = $selectedEntityType;
		if (id === null) {
			entityData = null;
			return;
		}

		if (type === 'city') {
			const city = $cities.find((c) => c.id === id);
			entityData = city ? { ...city, entityType: 'city' } : null;
		} else if (type === 'node') {
			const corpId = bridge.getPlayerCorpId();
			const infra = bridge.getInfrastructureList(corpId);
			const node = infra.nodes.find((n: InfraNode) => n.id === id);
			entityData = node ? { ...node, entityType: 'node' } : null;
		} else {
			entityData = null;
		}
	});

	function close() {
		selectedEntityId.set(null);
		selectedEntityType.set(null);
	}
</script>

{#if entityData}
	<div class="info-panel">
		<div class="panel-header">
			<span class="panel-title">
				{entityData.entityType === 'city' ? entityData.name : entityData.node_type}
			</span>
			<button class="close-btn" onclick={close}>x</button>
		</div>

		<div class="panel-body">
			{#if entityData.entityType === 'city'}
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
					<span class="value">{(entityData.infrastructure_satisfaction * 100).toFixed(0)}%</span>
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
					<span class="value">{(entityData.utilization * 100).toFixed(1)}%</span>
				</div>
				<div class="stat">
					<span class="label">Health</span>
					<span class="value">{(entityData.health * 100).toFixed(0)}%</span>
				</div>
				<div class="stat">
					<span class="label">Latency</span>
					<span class="value">{entityData.latency_ms.toFixed(1)} ms</span>
				</div>
				<div class="stat">
					<span class="label">Maintenance</span>
					<span class="value">{formatMoney(entityData.maintenance_cost)}/tick</span>
				</div>
				{#if entityData.under_construction}
					<div class="status-badge construction">Under Construction</div>
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
</style>
