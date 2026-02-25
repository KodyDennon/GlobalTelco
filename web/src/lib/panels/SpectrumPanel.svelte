<script lang="ts">
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import type { SpectrumLicense, SpectrumAuction, AvailableSpectrum, Region } from '$lib/wasm/types';
	import { worldInfo, playerCorp, regions as regionStore, formatMoney } from '$lib/stores/gameState';
	import { tooltip } from '$lib/ui/tooltip';

	let licenses: SpectrumLicense[] = $state([]);
	let auctions: SpectrumAuction[] = $state([]);
	let available: AvailableSpectrum[] = $state([]);
	let bidAmounts: Record<number, number> = $state({});

	// UI state
	let selectedRegion = $state(0);
	let selectedBand = $state('');
	let newBidAmount = $state(0);
	let activeTab: 'holdings' | 'auctions' | 'available' = $state('holdings');

	let playerId = $derived($playerCorp?.id ?? 0);
	let playerCash = $derived($playerCorp?.cash ?? 0);
	let regions: Region[] = $derived($regionStore);

	// Reactive: refresh data when tick changes
	$effect(() => {
		const _tick = $worldInfo.tick;
		refresh();
	});

	// Refresh available bands when region changes
	$effect(() => {
		if (selectedRegion > 0) {
			available = bridge.getAvailableSpectrum(selectedRegion);
		} else {
			available = [];
		}
	});

	function refresh() {
		licenses = bridge.getSpectrumLicenses();
		auctions = bridge.getSpectrumAuctions();
		if (selectedRegion > 0) {
			available = bridge.getAvailableSpectrum(selectedRegion);
		}
	}

	function placeBid(auctionId: number) {
		const amount = bidAmounts[auctionId] || 0;
		if (amount <= 0) return;
		const auction = auctions.find(a => a.id === auctionId);
		if (!auction) return;
		gameCommand({ BidSpectrum: { band: auction.band, region: auction.region_id, bid: amount } });
		bidAmounts[auctionId] = 0;
		refresh();
	}

	function startNewAuction() {
		if (!selectedBand || selectedRegion <= 0 || newBidAmount <= 0) return;
		gameCommand({ BidSpectrum: { band: selectedBand, region: selectedRegion, bid: newBidAmount } });
		selectedBand = '';
		newBidAmount = 0;
		refresh();
	}

	let myLicenses = $derived(licenses.filter(l => l.owner === playerId));
	let otherLicenses = $derived(licenses.filter(l => l.owner !== playerId));
	let currentTick = $derived($worldInfo.tick);

	function bandColor(category: string): string {
		switch (category) {
			case 'low': return '#10b981';
			case 'mid': return '#3b82f6';
			case 'high': return '#8b5cf6';
			default: return '#94a3b8';
		}
	}
</script>

<div class="panel" role="region" aria-label="Spectrum Management">
	<!-- Tab bar -->
	<div class="tab-bar">
		<button
			class="tab" class:active={activeTab === 'holdings'}
			onclick={() => activeTab = 'holdings'}
		>
			Holdings ({myLicenses.length})
		</button>
		<button
			class="tab" class:active={activeTab === 'auctions'}
			onclick={() => activeTab = 'auctions'}
		>
			Auctions ({auctions.length})
		</button>
		<button
			class="tab" class:active={activeTab === 'available'}
			onclick={() => activeTab = 'available'}
		>
			Available
		</button>
	</div>

	<!-- Holdings Tab -->
	{#if activeTab === 'holdings'}
		<div class="section">
			<h3 class="section-title">Your Spectrum Licenses</h3>
			{#if myLicenses.length === 0}
				<p class="empty">No spectrum licenses held. Bid on available frequencies to expand wireless coverage.</p>
			{:else}
				<div class="license-grid">
					{#each myLicenses as license}
						<div class="license-card">
							<div class="license-header">
								<span class="band-badge" style="color: {bandColor(license.band_category)}; border-color: {bandColor(license.band_category)}30">
									{license.band_name}
								</span>
								<span class="region-tag">{license.region_name}</span>
							</div>
							<div class="license-details">
								<div class="detail-row">
									<span class="label">Bandwidth</span>
									<span class="value">{license.bandwidth_mhz.toFixed(0)} MHz</span>
								</div>
								<div class="detail-row">
									<span class="label">Coverage</span>
									<span class="value">{license.coverage_radius_km.toFixed(1)} km</span>
								</div>
								<div class="detail-row">
									<span class="label">Expires</span>
									<span class="value" class:expiring={license.end_tick - currentTick < 20}>
										Tick {license.end_tick}
										({license.end_tick - currentTick} left)
									</span>
								</div>
								<div class="detail-row">
									<span class="label">Cost/tick</span>
									<span class="value cost">{formatMoney(license.cost_per_tick)}</span>
								</div>
							</div>
						</div>
					{/each}
				</div>
			{/if}

			{#if otherLicenses.length > 0}
				<h3 class="section-title" style="margin-top: 16px">Competitor Licenses</h3>
				<div class="competitor-table">
					<div class="table-header">
						<span>Band</span>
						<span>Region</span>
						<span>Owner</span>
						<span>Expires</span>
					</div>
					{#each otherLicenses as license}
						<div class="table-row">
							<span class="band-inline" style="color: {bandColor(license.band_category)}">{license.band_name}</span>
							<span>{license.region_name}</span>
							<span>{license.owner_name}</span>
							<span class="mono">T{license.end_tick}</span>
						</div>
					{/each}
				</div>
			{/if}
		</div>

	<!-- Auctions Tab -->
	{:else if activeTab === 'auctions'}
		<div class="section">
			<h3 class="section-title">Active Spectrum Auctions</h3>
			{#if auctions.length === 0}
				<p class="empty">No active spectrum auctions. Start one from the Available tab.</p>
			{:else}
				{#each auctions as auction}
					<div class="auction-card">
						<div class="auction-header">
							<span class="band-badge" style="color: {bandColor(auction.band_category)}; border-color: {bandColor(auction.band_category)}30">
								{auction.band_name}
							</span>
							<span class="region-tag">{auction.region_name}</span>
							<span class="time-left" class:urgent={auction.ticks_remaining <= 3}>
								{auction.ticks_remaining} tick{auction.ticks_remaining !== 1 ? 's' : ''} left
							</span>
						</div>
						<div class="auction-details">
							<div class="detail-row">
								<span class="label">Bandwidth</span>
								<span class="value">{auction.bandwidth_mhz.toFixed(0)} MHz</span>
							</div>
							<div class="detail-row">
								<span class="label">Coverage</span>
								<span class="value">{auction.coverage_radius_km.toFixed(1)} km</span>
							</div>
							<div class="detail-row">
								<span class="label">Current Bid</span>
								<span class="value bid-amount">{formatMoney(auction.current_bid)}</span>
							</div>
							<div class="detail-row">
								<span class="label">Leading</span>
								<span class="value" class:is-player={auction.highest_bidder === playerId}>
									{auction.bidder_name}
								</span>
							</div>
						</div>
						<div class="bid-section">
							<input
								type="number"
								bind:value={bidAmounts[auction.id]}
								placeholder="Your bid..."
								aria-label="Bid amount"
								min={auction.current_bid + 1}
							/>
							<button
								class="bid-btn"
								onclick={() => placeBid(auction.id)}
								disabled={!bidAmounts[auction.id] || (bidAmounts[auction.id] ?? 0) <= auction.current_bid || (bidAmounts[auction.id] ?? 0) > playerCash}
								use:tooltip={() => `Place bid of ${formatMoney(bidAmounts[auction.id] || 0)}\nMust exceed ${formatMoney(auction.current_bid)}`}
							>
								Bid
							</button>
						</div>
					</div>
				{/each}
			{/if}
		</div>

	<!-- Available Tab -->
	{:else if activeTab === 'available'}
		<div class="section">
			<h3 class="section-title">Start New Spectrum Auction</h3>

			<div class="form-group">
				<label for="region-select">Region</label>
				<select id="region-select" bind:value={selectedRegion}>
					<option value={0}>Select region...</option>
					{#each regions as region}
						<option value={region.id}>{region.name}</option>
					{/each}
				</select>
			</div>

			{#if selectedRegion > 0}
				<div class="band-reference">
					<h4 class="reference-title">Frequency Band Reference</h4>
					<div class="band-grid">
						<div class="band-grid-header">
							<span>Band</span>
							<span>Category</span>
							<span>Coverage</span>
							<span>Bandwidth</span>
							<span>Min Bid</span>
							<span>Status</span>
						</div>
						{#each available as band}
							<div
								class="band-grid-row"
								class:selected={selectedBand === band.band}
								onclick={() => { selectedBand = band.band; newBidAmount = band.min_bid; }}
								role="button"
								tabindex="0"
								onkeydown={(e) => { if (e.key === 'Enter') { selectedBand = band.band; newBidAmount = band.min_bid; } }}
							>
								<span class="band-name" style="color: {bandColor(band.band_category)}">{band.band_name}</span>
								<span class="category-tag" style="color: {bandColor(band.band_category)}">{band.band_category}</span>
								<span class="mono">{band.coverage_radius_km.toFixed(1)} km</span>
								<span class="mono">{band.max_bandwidth_mhz.toFixed(0)} MHz</span>
								<span class="mono">{formatMoney(band.min_bid)}</span>
								<span class="status-available">Available</span>
							</div>
						{/each}
						{#if available.length === 0}
							<p class="empty" style="grid-column: 1 / -1; padding: 8px;">All bands are licensed or in auction in this region.</p>
						{/if}
					</div>
				</div>

				{#if selectedBand}
					<div class="new-bid-section">
						<div class="selected-info">
							Bidding on <strong>{selectedBand}</strong> in <strong>{regions.find(r => r.id === selectedRegion)?.name ?? 'Unknown'}</strong>
						</div>
						<div class="bid-form">
							<input
								type="number"
								bind:value={newBidAmount}
								placeholder="Bid amount..."
								aria-label="New auction bid amount"
								min="1"
							/>
							<button
								class="bid-btn primary"
								onclick={startNewAuction}
								disabled={newBidAmount <= 0 || newBidAmount > playerCash}
								use:tooltip={() => `Start auction with bid of ${formatMoney(newBidAmount)}\nCash: ${formatMoney(playerCash)}`}
							>
								Start Auction
							</button>
						</div>
						{#if newBidAmount > playerCash}
							<p class="error-text">Insufficient funds ({formatMoney(playerCash)} available)</p>
						{/if}
					</div>
				{/if}
			{/if}
		</div>
	{/if}
</div>

<style>
	.panel {
		padding: 0;
		color: var(--text-secondary, #e2e8f0);
		font-family: var(--font-sans, 'Inter', sans-serif);
	}

	.tab-bar {
		display: flex;
		border-bottom: 1px solid var(--border, rgba(55, 65, 81, 0.5));
		padding: 0 12px;
	}

	.tab {
		background: none;
		border: none;
		color: var(--text-muted, #6b7280);
		font-size: 12px;
		font-family: var(--font-mono, 'JetBrains Mono', monospace);
		font-weight: 600;
		padding: 10px 16px;
		cursor: pointer;
		border-bottom: 2px solid transparent;
		transition: all 0.15s;
	}

	.tab:hover {
		color: var(--text-secondary, #e2e8f0);
	}

	.tab.active {
		color: var(--green, #10b981);
		border-bottom-color: var(--green, #10b981);
	}

	.section {
		padding: 12px 16px;
	}

	.section-title {
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--text-muted, #6b7280);
		margin: 0 0 10px 0;
	}

	.empty {
		color: var(--text-dim, #4b5563);
		font-size: 13px;
		padding: 12px 0;
	}

	/* License cards */
	.license-grid {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.license-card {
		background: rgba(30, 41, 59, 0.8);
		border: 1px solid var(--border, rgba(55, 65, 81, 0.5));
		border-radius: 6px;
		padding: 10px 12px;
	}

	.license-header {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 8px;
	}

	.band-badge {
		font-size: 11px;
		font-weight: 700;
		font-family: var(--font-mono, 'JetBrains Mono', monospace);
		padding: 2px 8px;
		border: 1px solid;
		border-radius: 4px;
	}

	.region-tag {
		font-size: 11px;
		color: var(--text-muted, #6b7280);
	}

	.license-details {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 4px;
	}

	.detail-row {
		display: flex;
		justify-content: space-between;
		font-size: 12px;
		padding: 2px 0;
	}

	.label {
		color: var(--text-muted, #6b7280);
	}

	.value {
		font-family: var(--font-mono, 'JetBrains Mono', monospace);
		color: var(--text-secondary, #e2e8f0);
	}

	.value.cost {
		color: var(--red, #ef4444);
	}

	.value.expiring {
		color: var(--amber, #f59e0b);
		font-weight: 600;
	}

	.value.is-player {
		color: var(--green, #10b981);
		font-weight: 600;
	}

	/* Competitor table */
	.competitor-table {
		font-size: 11px;
		border: 1px solid var(--border, rgba(55, 65, 81, 0.5));
		border-radius: 6px;
		overflow: hidden;
	}

	.table-header {
		display: grid;
		grid-template-columns: 1fr 1fr 1fr 0.7fr;
		gap: 8px;
		padding: 6px 10px;
		background: rgba(30, 41, 59, 0.6);
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--text-muted, #6b7280);
		font-size: 10px;
	}

	.table-row {
		display: grid;
		grid-template-columns: 1fr 1fr 1fr 0.7fr;
		gap: 8px;
		padding: 5px 10px;
		border-top: 1px solid rgba(55, 65, 81, 0.3);
	}

	.band-inline {
		font-family: var(--font-mono, 'JetBrains Mono', monospace);
		font-weight: 600;
	}

	.mono {
		font-family: var(--font-mono, 'JetBrains Mono', monospace);
	}

	/* Auction cards */
	.auction-card {
		background: rgba(30, 41, 59, 0.8);
		border: 1px solid var(--border, rgba(55, 65, 81, 0.5));
		border-radius: 6px;
		padding: 10px 12px;
		margin-bottom: 8px;
	}

	.auction-header {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 8px;
	}

	.time-left {
		margin-left: auto;
		font-size: 11px;
		font-family: var(--font-mono, 'JetBrains Mono', monospace);
		color: var(--text-muted, #6b7280);
	}

	.time-left.urgent {
		color: var(--red, #ef4444);
		font-weight: 700;
	}

	.auction-details {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 4px;
		margin-bottom: 8px;
	}

	.bid-amount {
		color: var(--amber, #f59e0b);
		font-weight: 600;
	}

	.bid-section {
		display: flex;
		gap: 8px;
	}

	.bid-section input, .bid-form input {
		flex: 1;
		background: rgba(17, 24, 39, 0.8);
		border: 1px solid var(--border, rgba(55, 65, 81, 0.5));
		color: var(--text-secondary, #e2e8f0);
		padding: 6px 10px;
		border-radius: 4px;
		font-size: 13px;
		font-family: var(--font-mono, 'JetBrains Mono', monospace);
	}

	.bid-section input:focus, .bid-form input:focus {
		outline: none;
		border-color: var(--green, #10b981);
	}

	.bid-btn {
		background: rgba(59, 130, 246, 0.2);
		border: 1px solid rgba(59, 130, 246, 0.4);
		color: #60a5fa;
		font-size: 12px;
		font-weight: 600;
		font-family: var(--font-mono, 'JetBrains Mono', monospace);
		padding: 6px 14px;
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.15s;
		white-space: nowrap;
	}

	.bid-btn:hover:not(:disabled) {
		background: rgba(59, 130, 246, 0.35);
	}

	.bid-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.bid-btn.primary {
		background: rgba(16, 185, 129, 0.2);
		border-color: rgba(16, 185, 129, 0.4);
		color: #10b981;
	}

	.bid-btn.primary:hover:not(:disabled) {
		background: rgba(16, 185, 129, 0.35);
	}

	/* Available tab */
	.form-group {
		margin-bottom: 12px;
	}

	.form-group label {
		display: block;
		font-size: 11px;
		font-weight: 600;
		color: var(--text-muted, #6b7280);
		margin-bottom: 4px;
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}

	.form-group select {
		width: 100%;
		background: rgba(17, 24, 39, 0.8);
		border: 1px solid var(--border, rgba(55, 65, 81, 0.5));
		color: var(--text-secondary, #e2e8f0);
		padding: 7px 10px;
		border-radius: 4px;
		font-size: 13px;
		font-family: var(--font-sans, 'Inter', sans-serif);
	}

	.form-group select:focus {
		outline: none;
		border-color: var(--green, #10b981);
	}

	.band-reference {
		margin-top: 8px;
	}

	.reference-title {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-muted, #6b7280);
		margin: 0 0 6px 0;
	}

	.band-grid {
		border: 1px solid var(--border, rgba(55, 65, 81, 0.5));
		border-radius: 6px;
		overflow: hidden;
	}

	.band-grid-header {
		display: grid;
		grid-template-columns: 1.2fr 0.7fr 0.8fr 0.8fr 1fr 0.7fr;
		gap: 6px;
		padding: 6px 10px;
		background: rgba(30, 41, 59, 0.6);
		font-size: 10px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--text-muted, #6b7280);
	}

	.band-grid-row {
		display: grid;
		grid-template-columns: 1.2fr 0.7fr 0.8fr 0.8fr 1fr 0.7fr;
		gap: 6px;
		padding: 6px 10px;
		font-size: 11px;
		border-top: 1px solid rgba(55, 65, 81, 0.3);
		cursor: pointer;
		transition: background 0.12s;
	}

	.band-grid-row:hover {
		background: rgba(55, 65, 81, 0.3);
	}

	.band-grid-row.selected {
		background: rgba(16, 185, 129, 0.1);
		border-left: 2px solid var(--green, #10b981);
	}

	.band-name {
		font-family: var(--font-mono, 'JetBrains Mono', monospace);
		font-weight: 600;
	}

	.category-tag {
		font-size: 10px;
		font-weight: 600;
		text-transform: uppercase;
	}

	.status-available {
		color: var(--green, #10b981);
		font-size: 10px;
		font-weight: 600;
	}

	.new-bid-section {
		margin-top: 12px;
		padding: 10px 12px;
		background: rgba(30, 41, 59, 0.8);
		border: 1px solid var(--border, rgba(55, 65, 81, 0.5));
		border-radius: 6px;
	}

	.selected-info {
		font-size: 12px;
		margin-bottom: 8px;
		color: var(--text-muted, #6b7280);
	}

	.selected-info strong {
		color: var(--text-secondary, #e2e8f0);
	}

	.bid-form {
		display: flex;
		gap: 8px;
	}

	.error-text {
		color: var(--red, #ef4444);
		font-size: 11px;
		margin-top: 4px;
	}
</style>
