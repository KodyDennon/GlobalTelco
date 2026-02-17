<script lang="ts">
	import { getAuctions, getPlayerCorpId, processCommand, getCorporationData } from '$lib/wasm/bridge';
	import type { AuctionInfo } from '$lib/wasm/types';
	import { tr } from '$lib/i18n/index';

	let auctions: AuctionInfo[] = $state([]);
	let playerCash = $state(0);
	let bidAmounts: Record<number, number> = $state({});

	function refresh() {
		auctions = getAuctions();
		const corpId = getPlayerCorpId();
		const corp = getCorporationData(corpId);
		playerCash = corp.cash;
	}

	$effect(() => {
		refresh();
		const interval = setInterval(refresh, 2000);
		return () => clearInterval(interval);
	});

	function placeBid(auctionId: number) {
		const amount = bidAmounts[auctionId] || 0;
		if (amount <= 0) return;
		processCommand({ PlaceBid: { auction: auctionId, amount } });
		refresh();
	}

	function formatMoney(val: number): string {
		if (Math.abs(val) >= 1_000_000) return `$${(val / 1_000_000).toFixed(1)}M`;
		if (Math.abs(val) >= 1_000) return `$${(val / 1_000).toFixed(0)}K`;
		return `$${val}`;
	}
</script>

<div class="panel" role="region" aria-label={$tr('panels.auctions')}>
	<h2>{$tr('panels.auctions')}</h2>

	{#if auctions.length === 0}
		<p class="empty">{$tr('panels.no_auctions')}</p>
	{:else}
		{#each auctions as auction}
			<div class="auction-card" class:closed={auction.status !== 'Open'}>
				<div class="auction-header">
					<span class="seller">{auction.seller_name}</span>
					<span class="status" class:open={auction.status === 'Open'}>{auction.status}</span>
				</div>
				<div class="auction-details">
					<div class="detail"><span class="label">{$tr('panels.assets')}</span><span class="value">{auction.asset_count}</span></div>
					<div class="detail"><span class="label">{$tr('panels.bids')}</span><span class="value">{auction.bid_count}</span></div>
					<div class="detail"><span class="label">{$tr('panels.highest')}</span><span class="value">{formatMoney(auction.highest_bid)}</span></div>
					<div class="detail"><span class="label">{$tr('panels.ends')}</span><span class="value">Tick {auction.end_tick}</span></div>
				</div>
				{#if auction.status === 'Open'}
					<div class="bid-section">
						<input
							type="number"
							bind:value={bidAmounts[auction.id]}
							placeholder={$tr('panels.bid_amount')}
							aria-label={$tr('panels.bid_amount')}
							min="1"
						/>
						<button
							onclick={() => placeBid(auction.id)}
							disabled={!bidAmounts[auction.id] || bidAmounts[auction.id] > playerCash}
							aria-label={$tr('panels.place_bid')}
						>
							{$tr('panels.place_bid')}
						</button>
					</div>
				{/if}
			</div>
		{/each}
	{/if}
</div>

<style>
	.panel { padding: 16px; color: #e5e7eb; }
	h2 { font-size: 16px; margin: 0 0 12px; color: #60a5fa; }
	.empty { color: #6b7280; font-size: 13px; }
	.auction-card { background: rgba(31, 41, 55, 0.8); border: 1px solid #374151; border-radius: 6px; padding: 12px; margin-bottom: 8px; }
	.auction-card.closed { opacity: 0.5; }
	.auction-header { display: flex; justify-content: space-between; margin-bottom: 8px; }
	.seller { font-weight: 600; font-size: 14px; }
	.status { font-size: 12px; padding: 2px 8px; border-radius: 4px; background: #374151; }
	.status.open { background: #065f46; color: #34d399; }
	.auction-details { display: grid; grid-template-columns: 1fr 1fr; gap: 4px; margin-bottom: 8px; }
	.detail { display: flex; justify-content: space-between; font-size: 12px; }
	.label { color: #9ca3af; }
	.value { color: #e5e7eb; font-family: monospace; }
	.bid-section { display: flex; gap: 8px; }
	.bid-section input { flex: 1; background: #1f2937; border: 1px solid #374151; color: #e5e7eb; padding: 6px 8px; border-radius: 4px; font-size: 13px; }
	.bid-section button { background: #2563eb; color: white; border: none; padding: 6px 12px; border-radius: 4px; cursor: pointer; font-size: 13px; }
	.bid-section button:disabled { opacity: 0.5; cursor: not-allowed; }
	.bid-section button:hover:not(:disabled) { background: #1d4ed8; }
</style>
