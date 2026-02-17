<script lang="ts">
	import { regions, cities, formatMoney, formatPopulation, allCorporations } from '$lib/stores/gameState';
	import { activePanel } from '$lib/stores/uiState';
	import * as bridge from '$lib/wasm/bridge';

	function close() {
		activePanel.set('none');
	}

	let totalPop = $derived($cities.reduce((s, c) => s + c.population, 0));
	let totalDemand = $derived($cities.reduce((s, c) => s + c.telecom_demand, 0));
	let avgSatisfaction = $derived(
		$cities.length > 0
			? $cities.reduce((s, c) => s + c.infrastructure_satisfaction, 0) / $cities.length
			: 0
	);
</script>

<div class="panel">
	<div class="panel-header">
		<span class="title">Regions & Markets</span>
		<button class="close" onclick={close}>x</button>
	</div>

	<div class="section">
		<h3>Global Overview</h3>
		<div class="stat-grid">
			<div class="stat">
				<span class="label">Total Population</span>
				<span class="value mono">{formatPopulation(totalPop)}</span>
			</div>
			<div class="stat">
				<span class="label">Telecom Demand</span>
				<span class="value mono">{totalDemand.toFixed(0)}</span>
			</div>
			<div class="stat">
				<span class="label">Avg Satisfaction</span>
				<span class="value mono" class:warn={avgSatisfaction < 0.5}>{(avgSatisfaction * 100).toFixed(0)}%</span>
			</div>
			<div class="stat">
				<span class="label">Regions</span>
				<span class="value mono">{$regions.length}</span>
			</div>
		</div>
	</div>

	{#each $regions as region}
		<div class="section region-card">
			<div class="region-header">
				<span class="region-name">{region.name}</span>
				<span class="region-pop">{formatPopulation(region.population)}</span>
			</div>
			<div class="region-stats">
				<span><span class="dim">GDP</span> <span class="mono">{formatMoney(region.gdp)}</span></span>
				<span><span class="dim">Dev</span> <span class="mono">{(region.development * 100).toFixed(0)}%</span></span>
				<span><span class="dim">Tax</span> <span class="mono">{(region.tax_rate * 100).toFixed(0)}%</span></span>
				<span><span class="dim">Risk</span> <span class="mono" class:warn={region.disaster_risk > 0.5}>{(region.disaster_risk * 100).toFixed(0)}%</span></span>
			</div>

			{#each $cities.filter((c) => c.region_id === region.id) as city}
				<div class="city-row">
					<span class="city-name">{city.name}</span>
					<span class="mono">{formatPopulation(city.population)}</span>
					<span class="mono" class:warn={city.infrastructure_satisfaction < 0.3}>{(city.infrastructure_satisfaction * 100).toFixed(0)}% sat</span>
				</div>
			{/each}
		</div>
	{/each}
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

	.stat-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 6px;
	}

	.stat {
		display: flex;
		justify-content: space-between;
		padding: 3px 0;
	}

	.label {
		color: var(--text-muted);
	}

	.value {
		color: var(--text-primary);
	}

	.mono {
		font-family: var(--font-mono);
	}

	.dim {
		color: var(--text-dim);
		font-size: 11px;
	}

	.warn {
		color: var(--red);
	}

	.region-card {
		padding: 10px 16px;
	}

	.region-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 6px;
	}

	.region-name {
		font-weight: 600;
		color: var(--text-primary);
	}

	.region-pop {
		font-family: var(--font-mono);
		color: var(--blue);
		font-size: 12px;
	}

	.region-stats {
		display: flex;
		gap: 12px;
		font-size: 11px;
		margin-bottom: 6px;
	}

	.city-row {
		display: flex;
		gap: 12px;
		padding: 3px 0 3px 12px;
		font-size: 12px;
		border-left: 2px solid var(--border);
	}

	.city-name {
		flex: 1;
		color: var(--text-secondary);
	}
</style>
