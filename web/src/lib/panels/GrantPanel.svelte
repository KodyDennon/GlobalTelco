<script lang="ts">
	import { playerCorp, formatMoney, worldInfo } from '$lib/stores/gameState';
	import { gameCommand } from '$lib/game/commandRouter';
	import { tooltip } from '$lib/ui/tooltip';
	import * as bridge from '$lib/wasm/bridge';
	import type { GrantInfo } from '$lib/wasm/types';

	let grants: GrantInfo[] = $state([]);

	// Sync grants with bridge (every 5th tick)
	$effect(() => {
		const corp = $playerCorp;
		const tick = $worldInfo.tick;
		if (tick % 5 !== 0) return;
		if (corp) {
			grants = bridge.getGrants(corp.id);
		}
	});

	function bidForGrant(id: number) {
		gameCommand({ BidForGrant: { grant_id: id } });
	}

	function completeGrant(id: number) {
		gameCommand({ CompleteGrant: { grant_id: id } });
	}

	let availableGrants = $derived(grants.filter(g => g.status === 'available'));
	let activeGrants = $derived(grants.filter(g => g.status === 'active' && g.is_holder));
	let completedGrants = $derived(grants.filter(g => g.status === 'completed' && g.is_holder));

	let totalRewardPending = $derived(activeGrants.reduce((s, g) => s + g.reward, 0));
	let totalRewardEarned = $derived(completedGrants.reduce((s, g) => s + g.reward, 0));

	function progressPct(grant: GrantInfo): number {
		if (grant.required_coverage <= 0) return 100;
		return Math.min(100, (grant.current_coverage / grant.required_coverage) * 100);
	}
</script>

<div class="panel">
	<div class="section">
		<h3>Grant Summary</h3>
		<div class="stat-row">
			<span class="muted">Available grants</span>
			<span class="mono">{availableGrants.length}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Active grants</span>
			<span class="mono blue">{activeGrants.length}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Pending rewards</span>
			<span class="mono green">{formatMoney(totalRewardPending)}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Total earned</span>
			<span class="mono green">{formatMoney(totalRewardEarned)}</span>
		</div>
	</div>

	{#if activeGrants.length > 0}
		<div class="section">
			<h3>Active Grants ({activeGrants.length})</h3>
			{#each activeGrants as grant}
				<div class="grant-row">
					<div class="grant-info">
						<div class="grant-header">
							<span class="grant-name">{grant.region_name}</span>
							<span class="badge active">Active</span>
						</div>
						<div class="progress-bar-container">
							<div class="progress-bar" style="width: {progressPct(grant)}%"></div>
						</div>
						<div class="grant-stats">
							<span>
								<span class="muted">Coverage</span>
								<span class="mono">{(grant.current_coverage * 100).toFixed(0)}% / {(grant.required_coverage * 100).toFixed(0)}%</span>
							</span>
							<span>
								<span class="muted">Reward</span>
								<span class="mono green">{formatMoney(grant.reward)}</span>
							</span>
							<span>
								<span class="muted">Ends in</span>
								<span class="mono">{grant.ticks_remaining} ticks</span>
							</span>
						</div>
					</div>
					<div class="grant-actions">
						<button
							class="complete-btn"
							onclick={() => completeGrant(grant.id)}
							disabled={progressPct(grant) < 100}
							use:tooltip={() =>
								progressPct(grant) >= 100
									? `Submit grant completion for ${grant.region_name}\nCollect ${formatMoney(grant.reward)} reward`
									: `Coverage target not yet met\nNeed ${(grant.required_coverage * 100).toFixed(0)}% coverage`}
						>
							Complete
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}

	<div class="section">
		<h3>Available Grants ({availableGrants.length})</h3>
		{#each availableGrants as grant}
			<div class="grant-row">
				<div class="grant-info">
					<div class="grant-header">
						<span class="grant-name">{grant.region_name}</span>
					</div>
					<div class="grant-stats">
						<span>
							<span class="muted">Required</span>
							<span class="mono">{(grant.required_coverage * 100).toFixed(0)}% coverage</span>
						</span>
						<span>
							<span class="muted">Reward</span>
							<span class="mono green">{formatMoney(grant.reward)}</span>
						</span>
						<span>
							<span class="muted">Duration</span>
							<span class="mono">{grant.ticks_remaining} ticks</span>
						</span>
					</div>
				</div>
				<div class="grant-actions">
					<button
						class="bid-btn"
						onclick={() => bidForGrant(grant.id)}
						use:tooltip={() =>
							`Bid for government grant in ${grant.region_name}\nBuild ${(grant.required_coverage * 100).toFixed(0)}% coverage to earn ${formatMoney(grant.reward)}`}
					>
						Bid
					</button>
				</div>
			</div>
		{/each}
		{#if availableGrants.length === 0}
			<div class="empty">No government grants currently available.</div>
		{/if}
	</div>

	{#if completedGrants.length > 0}
		<div class="section">
			<h3>Completed ({completedGrants.length})</h3>
			{#each completedGrants as grant}
				<div class="grant-row completed">
					<div class="grant-info">
						<div class="grant-header">
							<span class="grant-name">{grant.region_name}</span>
							<span class="badge done">Completed</span>
						</div>
						<div class="grant-stats">
							<span>
								<span class="muted">Reward</span>
								<span class="mono green">{formatMoney(grant.reward)}</span>
							</span>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.panel { color: var(--text-secondary); font-family: var(--font-sans); font-size: 13px; }
	.section { padding: 12px 16px; border-bottom: 1px solid var(--border); }
	h3 { font-size: 12px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 8px; }
	.stat-row { display: flex; justify-content: space-between; padding: 3px 0; }
	.muted { color: var(--text-muted); }
	.mono { font-family: var(--font-mono); }
	.green { color: var(--green); }
	.blue { color: var(--blue); }

	.grant-row { display: flex; justify-content: space-between; align-items: center; padding: 8px; border-radius: var(--radius-sm); border-bottom: 1px solid rgba(55, 65, 81, 0.2); }
	.grant-row:hover { background: var(--bg-surface); }
	.grant-row.completed { opacity: 0.7; }
	.grant-info { display: flex; flex-direction: column; gap: 4px; flex: 1; min-width: 0; }
	.grant-header { display: flex; align-items: center; gap: 8px; }
	.grant-name { font-weight: 600; color: var(--text-primary); }
	.grant-stats { display: flex; gap: 12px; font-size: 11px; }
	.progress-bar-container { width: 100%; height: 4px; background: rgba(55, 65, 81, 0.3); border-radius: 2px; overflow: hidden; }
	.progress-bar { height: 100%; border-radius: 2px; background: var(--blue); transition: width 0.3s ease; }
	.badge { font-size: 10px; padding: 2px 6px; border-radius: var(--radius-sm); font-weight: 600; }
	.badge.active { background: rgba(59, 130, 246, 0.1); color: var(--blue); }
	.badge.done { background: var(--green-bg); color: var(--green); }
	.grant-actions { display: flex; gap: 4px; margin-left: 8px; }
	.bid-btn { background: var(--bg-surface); border: 1px solid var(--border); color: var(--blue); padding: 4px 10px; border-radius: var(--radius-sm); cursor: pointer; font-size: 11px; font-weight: 600; }
	.bid-btn:hover { border-color: var(--blue); background: rgba(59, 130, 246, 0.1); }
	.complete-btn { background: var(--green-bg); border: 1px solid var(--green-border); color: var(--green); padding: 4px 10px; border-radius: var(--radius-sm); cursor: pointer; font-size: 11px; font-weight: 600; }
	.complete-btn:hover { background: rgba(16, 185, 129, 0.15); }
	.complete-btn:disabled { opacity: 0.4; cursor: not-allowed; }
	.empty { color: var(--text-dim); text-align: center; padding: 16px; font-size: 12px; }
</style>
