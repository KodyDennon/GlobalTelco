<script lang="ts">
	import { playerCorp, formatMoney, worldInfo } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';

	let isPublic = $state(false);
	let totalShares = $state(1000);
	let sharePrice = $state(0);
	let dividendsPerShare = $state(0);
	let satisfaction = $state(0.5);
	let ownedShares = $state(0);

	let tradeAmount = $state(10);

	interface BoardVoteEntry {
		proposal: string;
		votesFor: number;
		votesAgainst: number;
		deadlineTick: number;
	}

	let boardVotes: BoardVoteEntry[] = $state([]);

	let marketCap = $derived(sharePrice * totalShares);
	let netProfit = $derived(($playerCorp?.revenue_per_tick ?? 0) - ($playerCorp?.cost_per_tick ?? 0));

	// Load stock market data from bridge (every 5th tick — Phase 5 optimization)
	$effect(() => {
		const corp = $playerCorp;
		const tick = $worldInfo.tick;
		if (tick % 5 !== 0) return;
		if (!corp) return;
		const sm = bridge.getStockMarket(corp.id);
		isPublic = sm.public;
		totalShares = sm.total_shares || 1000;
		sharePrice = sm.share_price;
		dividendsPerShare = sm.dividends_per_share;
		satisfaction = sm.shareholder_satisfaction;
		ownedShares = sm.shareholders?.[corp.id] || 0;
		boardVotes = (sm.board_votes ?? []).map((v) => ({
			proposal: v.proposal,
			votesFor: v.votes_for,
			votesAgainst: v.votes_against,
			deadlineTick: v.deadline_tick,
		}));
	});

	function handleBuy() {
		if (!$playerCorp) return;
		gameCommand({
			BuyShares: {
				corporation: $playerCorp.id,
				count: tradeAmount
			}
		});
	}

	function handleSell() {
		if (!$playerCorp) return;
		gameCommand({
			SellShares: {
				corporation: $playerCorp.id,
				count: tradeAmount
			}
		});
	}

	let satisfactionLabel = $derived(
		satisfaction >= 0.8
			? 'Excellent'
			: satisfaction >= 0.6
				? 'Good'
				: satisfaction >= 0.4
					? 'Neutral'
					: satisfaction >= 0.2
						? 'Dissatisfied'
						: 'Critical'
	);

	let satisfactionColor = $derived(
		satisfaction >= 0.6 ? 'green' : satisfaction >= 0.4 ? 'amber' : 'red'
	);

	// Share price bar visualization (0 to max reasonable price)
	let priceBarWidth = $derived(Math.min(100, Math.max(2, (sharePrice / 5000) * 100)));
</script>

<div class="panel">
	<div class="section">
		<h3>IPO Status</h3>
		<div class="stat-row">
			<span class="muted">Status</span>
			{#if isPublic}
				<span class="badge public-badge">PUBLIC</span>
			{:else}
				<span class="badge private-badge">PRIVATE</span>
			{/if}
		</div>
		{#if !isPublic}
			<div class="ipo-requirements">
				<div class="requirement-label muted">IPO Requirements</div>
				<div class="stat-row">
					<span class="muted">Infrastructure</span>
					<span class="mono" class:green={($playerCorp?.infrastructure_count ?? 0) >= 50} class:red={($playerCorp?.infrastructure_count ?? 0) < 50}>
						{$playerCorp?.infrastructure_count ?? 0} / 50 nodes
					</span>
				</div>
				<div class="stat-row">
					<span class="muted">Cash reserves</span>
					<span class="mono" class:green={($playerCorp?.cash ?? 0) >= 1_000_000} class:red={($playerCorp?.cash ?? 0) < 1_000_000}>
						{formatMoney($playerCorp?.cash ?? 0)} / $1.0M
					</span>
				</div>
			</div>
		{/if}
	</div>

	{#if isPublic}
		<div class="section">
			<h3>Market Summary</h3>
			<div class="stat-row">
				<span class="muted">Share Price</span>
				<span class="mono green">{formatMoney(sharePrice)}</span>
			</div>
			<div class="stat-row">
				<span class="muted">Your Stake</span>
				<span class="mono">{ownedShares.toLocaleString()} shares ({((ownedShares / totalShares) * 100).toFixed(1)}%)</span>
			</div>
			<div class="stat-row">
				<span class="muted">Total Shares</span>
				<span class="mono">{totalShares.toLocaleString()}</span>
			</div>
			<div class="stat-row">
				<span class="muted">Market Cap</span>
				<span class="mono green">{formatMoney(marketCap)}</span>
			</div>
			<div class="stat-row">
				<span class="muted">Dividends/Share</span>
				<span class="mono" class:green={dividendsPerShare > 0} class:muted={dividendsPerShare === 0}>
					{dividendsPerShare > 0 ? formatMoney(dividendsPerShare) + '/tick' : 'None'}
				</span>
			</div>
		</div>

		<div class="section">
			<h3>Trade Shares</h3>
			<div class="trade-controls">
				<div class="amount-input">
					<span class="muted">Amount</span>
					<input type="number" bind:value={tradeAmount} min="1" max="1000" />
				</div>
				<div class="actions">
					<button class="buy-btn" onclick={handleBuy} disabled={($playerCorp?.cash ?? 0) < sharePrice * tradeAmount}>
						BUY ({formatMoney(sharePrice * tradeAmount)})
					</button>
					<button class="sell-btn" onclick={handleSell} disabled={ownedShares < tradeAmount}>
						SELL ({formatMoney(sharePrice * tradeAmount)})
					</button>
				</div>
			</div>
		</div>

		<div class="section">
			<h3>Shareholder Satisfaction</h3>
			<div class="satisfaction-row">
				<div class="satisfaction-bar-container">
					<div class="satisfaction-bar {satisfactionColor}" style="width: {satisfaction * 100}%"></div>
				</div>
				<span class="satisfaction-label {satisfactionColor}">{satisfactionLabel}</span>
			</div>
			<div class="stat-row">
				<span class="muted">Score</span>
				<span class="mono">{(satisfaction * 100).toFixed(0)}%</span>
			</div>
		</div>

		<div class="section">
			<h3>Board Votes ({boardVotes.length})</h3>
			{#if boardVotes.length > 0}
				{#each boardVotes as vote}
					<div class="vote-card">
						<div class="vote-proposal">{vote.proposal}</div>
						<div class="vote-stats">
							<span class="mono green">For: {vote.votesFor}</span>
							<span class="mono red">Against: {vote.votesAgainst}</span>
						</div>
					</div>
				{/each}
			{:else}
				<div class="empty">No pending board votes.</div>
			{/if}
		</div>
	{:else}
		<div class="section">
			<div class="empty">
				Corporation is privately held. Meet IPO requirements to go public and access the stock market.
			</div>
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

	.green {
		color: var(--green);
	}

	.red {
		color: var(--red);
	}

	.amber {
		color: var(--amber);
	}

	.badge {
		font-size: 10px;
		padding: 1px 8px;
		border-radius: var(--radius-sm);
		font-weight: 700;
		letter-spacing: 0.5px;
	}

	.public-badge {
		background: rgba(34, 197, 94, 0.1);
		color: var(--green);
		border: 1px solid rgba(34, 197, 94, 0.2);
	}

	.private-badge {
		background: rgba(156, 163, 175, 0.1);
		color: var(--text-muted);
		border: 1px solid rgba(156, 163, 175, 0.2);
	}

	.ipo-requirements {
		margin-top: 8px;
		padding: 8px;
		background: var(--bg-surface);
		border-radius: var(--radius-sm);
		border: 1px solid var(--border);
	}

	.requirement-label {
		font-size: 11px;
		font-weight: 600;
		margin-bottom: 4px;
	}

	.trade-controls {
		display: flex;
		flex-direction: column;
		gap: 12px;
		margin-top: 4px;
	}

	.amount-input {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.amount-input input {
		width: 60px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--text-primary);
		padding: 4px 8px;
		border-radius: 4px;
		font-family: var(--font-mono);
		text-align: right;
	}

	.actions {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
	}

	button {
		padding: 8px;
		font-size: 11px;
		font-weight: 700;
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.2s;
		border: 1px solid transparent;
	}

	button:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.buy-btn {
		background: rgba(34, 197, 94, 0.1);
		color: var(--green);
		border-color: rgba(34, 197, 94, 0.2);
	}

	.buy-btn:hover:not(:disabled) {
		background: var(--green);
		color: white;
	}

	.sell-btn {
		background: rgba(239, 68, 68, 0.1);
		color: var(--red);
		border-color: rgba(239, 68, 68, 0.2);
	}

	.sell-btn:hover:not(:disabled) {
		background: var(--red);
		color: white;
	}

	.satisfaction-row {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 6px;
	}

	.satisfaction-bar-container {
		flex: 1;
		height: 8px;
		background: var(--bg-surface);
		border-radius: 4px;
		overflow: hidden;
		border: 1px solid var(--border);
	}

	.satisfaction-bar {
		height: 100%;
		border-radius: 4px;
		transition: width 0.3s ease;
	}

	.satisfaction-bar.green {
		background: var(--green);
	}

	.satisfaction-bar.amber {
		background: var(--amber);
	}

	.satisfaction-bar.red {
		background: var(--red);
	}

	.satisfaction-label {
		font-size: 11px;
		font-weight: 600;
		white-space: nowrap;
	}

	.vote-card {
		padding: 8px;
		border-radius: var(--radius-sm);
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
	}

	.vote-card:hover {
		background: var(--bg-surface);
	}

	.vote-proposal {
		font-weight: 600;
		color: var(--text-primary);
		margin-bottom: 4px;
	}

	.vote-stats {
		display: flex;
		gap: 12px;
		font-size: 11px;
	}

	.empty {
		color: var(--text-dim);
		text-align: center;
		padding: 16px;
		font-size: 12px;
	}
</style>
