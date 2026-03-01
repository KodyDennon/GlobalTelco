<script lang="ts">
	import { playerCorp, formatMoney, regions, worldInfo } from '$lib/stores/gameState';
	import { gameCommand } from '$lib/game/commandRouter';
	import { tooltip } from '$lib/ui/tooltip';
	import * as bridge from '$lib/wasm/bridge';
	import type { Region } from '$lib/wasm/types';

	type PricingTier = 'Budget' | 'Standard' | 'Premium';

	interface TierInfo {
		label: PricingTier;
		revenueMultiplier: number;
		customerMultiplier: number;
		color: string;
	}

	const TIERS: TierInfo[] = [
		{ label: 'Budget', revenueMultiplier: 0.6, customerMultiplier: 1.4, color: 'var(--blue)' },
		{ label: 'Standard', revenueMultiplier: 1.0, customerMultiplier: 1.0, color: 'var(--text-primary)' },
		{ label: 'Premium', revenueMultiplier: 1.8, customerMultiplier: 0.6, color: 'var(--amber, #f59e0b)' }
	];

	// Track pricing per region
	let regionPricing: Map<number, { tier: PricingTier; pricePerUnit: number }> = $state(new Map());

	// Load initial pricing from bridge
	$effect(() => {
		const corp = $playerCorp;
		const _tick = $worldInfo.tick;
		if (!corp) return;
		const raw = bridge.getRegionPricing(corp.id);
		if (raw.length > 0) {
			regionPricing = new Map(raw.map((rp) => [rp.region_id, { tier: rp.tier as PricingTier, pricePerUnit: rp.price_per_unit }]));
		}
	});

	// Regions the player operates in (all regions for now; in full impl, filtered by owned infra)
	let operatingRegions = $derived($regions);

	function getRegionPricing(regionId: number): { tier: PricingTier; pricePerUnit: number } {
		return regionPricing.get(regionId) ?? { tier: 'Standard', pricePerUnit: 10 };
	}

	function getTierInfo(tier: PricingTier): TierInfo {
		return TIERS.find((t) => t.label === tier) ?? TIERS[1];
	}

	function setTier(region: Region, tier: PricingTier) {
		const current = getRegionPricing(region.id);
		const basePricePerUnit = 10;
		const tierInfo = getTierInfo(tier);
		const pricePerUnit = Math.round(basePricePerUnit * tierInfo.revenueMultiplier * 100) / 100;

		gameCommand({
			SetRegionPricing: {
				region: region.id,
				tier,
				price_per_unit: pricePerUnit
			}
		});

		regionPricing = new Map([...regionPricing, [region.id, { tier, pricePerUnit }]]);
	}

	function setCustomPrice(region: Region, price: number) {
		const current = getRegionPricing(region.id);
		gameCommand({
			SetRegionPricing: {
				region: region.id,
				tier: current.tier,
				price_per_unit: price
			}
		});

		regionPricing = new Map([...regionPricing, [region.id, { ...current, pricePerUnit: price }]]);
	}

	function estimatedRevenue(region: Region): number {
		const pricing = getRegionPricing(region.id);
		const tierInfo = getTierInfo(pricing.tier);
		return region.population * 0.001 * tierInfo.customerMultiplier * pricing.pricePerUnit;
	}

	let totalEstimatedRevenue = $derived(
		operatingRegions.reduce((s, r) => s + estimatedRevenue(r), 0)
	);

	let budgetCount = $derived(operatingRegions.filter((r) => getRegionPricing(r.id).tier === 'Budget').length);
	let standardCount = $derived(operatingRegions.filter((r) => getRegionPricing(r.id).tier === 'Standard').length);
	let premiumCount = $derived(operatingRegions.filter((r) => getRegionPricing(r.id).tier === 'Premium').length);
</script>

<div class="panel">
	<div class="section">
		<h3>Pricing Overview</h3>
		<div class="stat-row">
			<span class="muted">Operating regions</span>
			<span class="mono">{operatingRegions.length}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Budget regions</span>
			<span class="mono blue">{budgetCount}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Standard regions</span>
			<span class="mono">{standardCount}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Premium regions</span>
			<span class="mono amber">{premiumCount}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Est. total revenue</span>
			<span class="mono green">{formatMoney(totalEstimatedRevenue)}/tick</span>
		</div>
	</div>

	<div class="section">
		<h3>Tier Multipliers</h3>
		<div class="tier-grid">
			{#each TIERS as tier}
				<div class="tier-card">
					<span class="tier-label" style="color: {tier.color}">{tier.label}</span>
					<div class="tier-stats">
						<span class="muted">Revenue</span>
						<span class="mono" class:green={tier.revenueMultiplier > 1} class:red={tier.revenueMultiplier < 1}>
							{tier.revenueMultiplier.toFixed(1)}x
						</span>
					</div>
					<div class="tier-stats">
						<span class="muted">Customers</span>
						<span class="mono" class:green={tier.customerMultiplier > 1} class:red={tier.customerMultiplier < 1}>
							{tier.customerMultiplier.toFixed(1)}x
						</span>
					</div>
				</div>
			{/each}
		</div>
	</div>

	<div class="section">
		<h3>Regional Pricing ({operatingRegions.length})</h3>
		{#each operatingRegions as region}
			{@const pricing = getRegionPricing(region.id)}
			{@const tierInfo = getTierInfo(pricing.tier)}
			<div class="region-row">
				<div class="region-info">
					<div class="region-header">
						<span class="region-name">{region.name}</span>
						<span class="badge" style="color: {tierInfo.color}; background: {tierInfo.color}15; border: 1px solid {tierInfo.color}30">
							{pricing.tier}
						</span>
					</div>
					<div class="region-stats">
						<span>
							<span class="muted">Pop</span>
							<span class="mono">{(region.population / 1000).toFixed(0)}k</span>
						</span>
						<span>
							<span class="muted">Price</span>
							<span class="mono">{formatMoney(pricing.pricePerUnit)}/unit</span>
						</span>
						<span>
							<span class="muted">Est. Rev</span>
							<span class="mono green">{formatMoney(estimatedRevenue(region))}/tick</span>
						</span>
					</div>
				</div>
				<div class="region-actions">
					<select
						class="tier-select"
						value={pricing.tier}
						onchange={(e) => setTier(region, (e.target as HTMLSelectElement).value as PricingTier)}
					>
						{#each TIERS as tier}
							<option value={tier.label}>{tier.label}</option>
						{/each}
					</select>
				</div>
			</div>
		{/each}
		{#if operatingRegions.length === 0}
			<div class="empty">No operating regions. Build infrastructure to start serving customers.</div>
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

	.blue {
		color: var(--blue);
	}

	.amber {
		color: var(--amber, #f59e0b);
	}

	.tier-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 8px;
	}

	.tier-card {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		padding: 8px;
		text-align: center;
	}

	.tier-label {
		font-weight: 700;
		font-size: 12px;
		display: block;
		margin-bottom: 4px;
	}

	.tier-stats {
		display: flex;
		justify-content: space-between;
		font-size: 11px;
		padding: 1px 0;
	}

	.region-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px;
		border-radius: var(--radius-sm);
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
	}

	.region-row:hover {
		background: var(--bg-surface);
	}

	.region-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
		flex: 1;
		min-width: 0;
	}

	.region-header {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.region-name {
		font-weight: 600;
		color: var(--text-primary);
	}

	.region-stats {
		display: flex;
		gap: 12px;
		font-size: 11px;
	}

	.badge {
		font-size: 10px;
		padding: 2px 6px;
		border-radius: var(--radius-sm);
		font-weight: 600;
	}

	.region-actions {
		display: flex;
		gap: 4px;
		margin-left: 8px;
	}

	.tier-select {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--text-primary);
		padding: 4px 8px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 11px;
		font-family: var(--font-mono);
	}

	.tier-select:hover {
		border-color: var(--blue);
	}

	.tier-select:focus {
		outline: none;
		border-color: var(--blue);
	}

	.empty {
		color: var(--text-dim);
		text-align: center;
		padding: 16px;
		font-size: 12px;
	}
</style>
