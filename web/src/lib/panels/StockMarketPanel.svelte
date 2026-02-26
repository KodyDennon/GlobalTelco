<script lang="ts">
	import { playerCorp, formatMoney } from '$lib/stores/gameState';

	// Stock market data (will be populated from WASM bridge in future)
	// For now, derive from corporation financial data
	let isPublic = $state(false);
	let totalShares = $state(1000);
	let sharePrice = $state(0);
	let dividendsPerShare = $state(0);
	let satisfaction = $state(0.5);

	interface BoardVoteEntry {
		proposal: string;
		votesFor: number;
		votesAgainst: number;
		deadlineTick: number;
	}

	let boardVotes: BoardVoteEntry[] = $state([]);

	let marketCap = $derived(sharePrice * totalShares);
	let netProfit = $derived(($playerCorp?.revenue_per_tick ?? 0) - ($playerCorp?.cost_per_tick ?? 0));

	// Derive stock market estimates from financial data
	$effect(() => {
		const corp = $playerCorp;
		if (!corp) return;

		const nodeCount = corp.infrastructure_count;
		const cash = corp.cash;
		const profit = corp.revenue_per_tick - corp.cost_per_tick;

		// Auto-IPO conditions (mirrors Rust logic)
		if (nodeCount >= 50 && cash >= 1_000_000) {
			isPublic = true;
		}

		if (isPublic) {
			// Mirror the Rust share price formula
			const basePrice = nodeCount * 10 + 50; // reputation ~ 50
			const cashComponent = Math.floor(cash / totalShares / 10);
			const profitComponent = Math.floor((profit * 10) / totalShares);
			sharePrice = Math.max(1, basePrice + cashComponent + profitComponent);

			// Dividend calculation (mirrors Rust)
			if (profit > 0 && cash > corp.cost_per_tick * 20) {
				dividendsPerShare = Math.floor(profit / 10 / totalShares);
			} else {
				dividendsPerShare = 0;
			}
		}
	});

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
			<div class="stat-row">
				<span class="muted">Net Profit</span>
				<span class="mono" class:green={netProfit > 0} class:red={netProfit < 0}>
					{formatMoney(netProfit)}/tick
				</span>
			</div>
		</div>

		<div class="section">
			<h3>Share Price</h3>
			<div class="price-bar-container">
				<div class="price-bar" style="width: {priceBarWidth}%"></div>
				<span class="price-bar-label mono">{formatMoney(sharePrice)}</span>
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

	.price-bar-container {
		position: relative;
		height: 24px;
		background: var(--bg-surface);
		border-radius: var(--radius-sm);
		border: 1px solid var(--border);
		overflow: hidden;
	}

	.price-bar {
		height: 100%;
		background: linear-gradient(90deg, rgba(34, 197, 94, 0.3), rgba(34, 197, 94, 0.6));
		border-radius: var(--radius-sm);
		transition: width 0.3s ease;
	}

	.price-bar-label {
		position: absolute;
		top: 50%;
		left: 8px;
		transform: translateY(-50%);
		font-size: 11px;
		color: var(--text-primary);
		font-weight: 600;
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
