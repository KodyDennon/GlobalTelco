<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import LoadingSkeleton from '$lib/components/LoadingSkeleton.svelte';
	import Badge from '$lib/components/Badge.svelte';
	import { toast } from '$lib/components/Toast.svelte';
	import { fetchHealth } from '$lib/api/health.js';
	import { fetchAuditLog } from '$lib/api/audit.js';
	import { broadcast } from '$lib/api/settings.js';
	import { startPolling, stopPolling } from '$lib/stores/polling.js';
	import type { ServerHealth, AuditEntry } from '$lib/api/types.js';

	let health = $state<ServerHealth | null>(null);
	let recentAudit = $state<AuditEntry[]>([]);
	let loading = $state(true);

	// Quick actions
	let broadcastMsg = $state('');
	let broadcastSending = $state(false);

	async function loadData() {
		try {
			const [h, a] = await Promise.all([
				fetchHealth(),
				fetchAuditLog(10)
			]);
			health = h;
			recentAudit = a.audit_log;
			loading = false;
		} catch (e) {
			if (loading) loading = false;
		}
	}

	async function sendBroadcast() {
		if (!broadcastMsg.trim()) return;
		broadcastSending = true;
		try {
			await broadcast(broadcastMsg);
			toast('Broadcast sent', 'success');
			broadcastMsg = '';
		} catch (e) {
			toast('Failed to send broadcast', 'error');
		} finally {
			broadcastSending = false;
		}
	}

	function formatUptime(secs: number): string {
		const d = Math.floor(secs / 86400);
		const h = Math.floor((secs % 86400) / 3600);
		const m = Math.floor((secs % 3600) / 60);
		if (d > 0) return `${d}d ${h}h`;
		if (h > 0) return `${h}h ${m}m`;
		return `${m}m`;
	}

	function timeAgo(iso: string): string {
		const diff = (Date.now() - new Date(iso).getTime()) / 1000;
		if (diff < 60) return `${Math.floor(diff)}s ago`;
		if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
		if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
		return `${Math.floor(diff / 86400)}d ago`;
	}

	onMount(() => {
		startPolling('overview', loadData, 10000);
	});

	onDestroy(() => {
		stopPolling('overview');
	});
</script>

<div class="page">
	<h1 class="page-title">Overview</h1>

	{#if loading}
		<LoadingSkeleton rows={4} height={40} />
	{:else if health}
		<div class="stats-row">
			<StatCard label="Version" value={health.version} color="var(--text-primary)" />
			<StatCard label="Uptime" value={formatUptime(health.uptime_secs)} color="var(--green)" />
			<StatCard label="Worlds" value={health.active_worlds} color="var(--blue)" />
			<StatCard label="Players" value={health.connected_players} color="var(--purple)" />
			<StatCard label="Accounts" value={health.registered_accounts} color="var(--amber)" />
			<StatCard label="Database" value={health.has_database ? 'Connected' : 'None'} color={health.has_database ? 'var(--green)' : 'var(--text-dim)'} />
		</div>

		<!-- Quick Actions -->
		<div class="section">
			<h2 class="section-title">Quick Actions</h2>
			<div class="quick-actions">
				<a href="/worlds" class="action-btn">Create World</a>
				<div class="broadcast-inline">
					<input
						type="text"
						class="broadcast-input"
						bind:value={broadcastMsg}
						placeholder="Broadcast message to all players..."
						onkeydown={(e) => e.key === 'Enter' && sendBroadcast()}
					/>
					<button class="action-btn send" onclick={sendBroadcast} disabled={broadcastSending || !broadcastMsg.trim()}>
						{broadcastSending ? 'Sending...' : 'Send'}
					</button>
				</div>
			</div>
		</div>

		<!-- Recent Activity -->
		<div class="section">
			<h2 class="section-title">Recent Activity</h2>
			{#if recentAudit.length === 0}
				<p class="empty-text">No recent activity</p>
			{:else}
				<div class="activity-feed">
					{#each recentAudit as entry}
						<div class="activity-item">
							<div class="activity-info">
								<span class="activity-actor">{entry.actor}</span>
								<Badge color="blue">{entry.action}</Badge>
								{#if entry.target}
									<span class="activity-target">{entry.target}</span>
								{/if}
							</div>
							<span class="activity-time">{timeAgo(entry.created_at)}</span>
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<!-- Active Worlds -->
		{#if health.worlds.length > 0}
			<div class="section">
				<h2 class="section-title">Active Worlds</h2>
				<div class="worlds-grid">
					{#each health.worlds as world}
						<a href="/worlds/{world.id}" class="world-card">
							<div class="world-name">{world.name}</div>
							<div class="world-meta">
								<span>{world.player_count}/{world.max_players} players</span>
								<span>Tick {world.tick}</span>
								<Badge color={world.speed === 'Paused' ? 'amber' : 'green'}>{world.speed}</Badge>
							</div>
						</a>
					{/each}
				</div>
			</div>
		{/if}
	{/if}
</div>

<style>
	.page {
		max-width: 1200px;
	}
	.page-title {
		font-size: 20px;
		font-weight: 700;
		margin-bottom: 20px;
	}
	.stats-row {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
		gap: 12px;
		margin-bottom: 24px;
	}
	.section {
		margin-bottom: 24px;
	}
	.section-title {
		font-size: 14px;
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		margin-bottom: 12px;
	}
	.quick-actions {
		display: flex;
		gap: 12px;
		align-items: center;
		flex-wrap: wrap;
	}
	.action-btn {
		padding: 8px 16px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		color: var(--text-secondary);
		font-size: 13px;
		font-weight: 500;
		cursor: pointer;
		text-decoration: none;
	}
	.action-btn:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}
	.action-btn.send {
		background: var(--blue);
		color: white;
		border: none;
	}
	.action-btn:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.broadcast-inline {
		display: flex;
		gap: 8px;
		flex: 1;
		min-width: 200px;
	}
	.broadcast-input {
		flex: 1;
		padding: 8px 12px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		color: var(--text-primary);
		font-size: 13px;
		font-family: inherit;
	}
	.activity-feed {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.activity-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px 12px;
		background: var(--bg-panel);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
	}
	.activity-info {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.activity-actor {
		font-size: 13px;
		font-weight: 500;
		color: var(--text-primary);
		font-family: var(--font-mono);
	}
	.activity-target {
		font-size: 12px;
		color: var(--text-dim);
		font-family: var(--font-mono);
	}
	.activity-time {
		font-size: 11px;
		color: var(--text-faint);
		font-family: var(--font-mono);
	}
	.empty-text {
		font-size: 13px;
		color: var(--text-dim);
		font-style: italic;
	}
	.worlds-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 10px;
	}
	.world-card {
		padding: 14px;
		background: var(--bg-panel);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		text-decoration: none;
		color: inherit;
		transition: border-color 0.15s;
	}
	.world-card:hover {
		border-color: var(--blue);
	}
	.world-name {
		font-size: 14px;
		font-weight: 600;
		margin-bottom: 6px;
	}
	.world-meta {
		display: flex;
		gap: 10px;
		align-items: center;
		font-size: 12px;
		color: var(--text-dim);
	}
	@media (max-width: 768px) {
		.broadcast-inline {
			min-width: 0;
			flex-basis: 100%;
		}
		.activity-item {
			flex-wrap: wrap;
			gap: 4px;
		}
		.activity-info {
			flex-wrap: wrap;
		}
	}
</style>
