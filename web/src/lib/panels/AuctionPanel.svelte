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
	.panel { padding: 16px; color: var(--text-secondary); }
	h2 { font-size: 16px; margin: 0 0 12px; color: var(--blue-light); }
	.empty { color: var(--text-dim); font-size: 13px; }
	.auction-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-md); padding: 12px; margin-bottom: 8px; }
	.auction-card.closed { opacity: 0.5; }
	.auction-header { display: flex; justify-content: space-between; margin-bottom: 8px; }
	.seller { font-weight: 600; font-size: 14px; }
	.status { font-size: 12px; padding: 2px 8px; border-radius: var(--radius-sm); background: var(--bg-hover); }
	.status.open { background: var(--green-bg); color: var(--green-light); }
	.auction-details { display: grid; grid-template-columns: 1fr 1fr; gap: 4px; margin-bottom: 8px; }
	.detail { display: flex; justify-content: space-between; font-size: 12px; }
	.label { color: var(--text-muted); }
	.value { color: var(--text-secondary); font-family: var(--font-mono); }
	.bid-section { display: flex; gap: 8px; }
	.bid-section input { flex: 1; background: var(--bg-surface); border: 1px solid var(--border); color: var(--text-secondary); padding: 6px 8px; border-radius: var(--radius-sm); font-size: 13px; }
	.bid-section button { background: var(--blue); color: white; border: none; padding: 6px 12px; border-radius: var(--radius-sm); cursor: pointer; font-size: 13px; }
	.bid-section button:disabled { opacity: 0.5; cursor: not-allowed; }
	.bid-section button:hover:not(:disabled) { opacity: 0.85; }
</style>
