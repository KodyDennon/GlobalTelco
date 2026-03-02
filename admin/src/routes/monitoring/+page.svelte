<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import Sparkline from '$lib/components/Sparkline.svelte';
	import LoadingSkeleton from '$lib/components/LoadingSkeleton.svelte';
	import { fetchMetrics } from '$lib/api/metrics.js';
	import { startPolling, stopPolling } from '$lib/stores/polling.js';
	import type { ServerMetrics, WorldMetrics } from '$lib/api/types.js';

	let metrics = $state<ServerMetrics | null>(null);
	let loading = $state(true);
	let expandedWorld = $state<string | null>(null);

	// History for sparklines
	let memoryHistory = $state<number[]>([]);
	let playerHistory = $state<number[]>([]);
	let wsHistory = $state<number[]>([]);

	async function loadData() {
		try {
			const m = await fetchMetrics();
			metrics = m;
			// Append to history (cap at 60)
			const memMb = m.server.memory_mb ?? Math.round(m.server.memory_estimate_bytes / 1048576);
			memoryHistory = [...memoryHistory.slice(-59), memMb];
			playerHistory = [...playerHistory.slice(-59), m.server.connected_players];
			wsHistory = [...wsHistory.slice(-59), m.server.ws_messages_per_sec ?? 0];
			loading = false;
		} catch { if (loading) loading = false; }
	}

	function formatUs(us: number): string {
		if (us > 1000000) return `${(us / 1000000).toFixed(1)}s`;
		if (us > 1000) return `${(us / 1000).toFixed(1)}ms`;
		return `${us}us`;
	}

	function systemColor(us: number): string {
		if (us > 5000) return 'var(--red)';
		if (us > 2000) return 'var(--amber)';
		if (us > 500) return 'var(--blue)';
		return 'var(--green)';
	}

	function formatUptime(secs: number): string {
		const d = Math.floor(secs / 86400);
		const h = Math.floor((secs % 86400) / 3600);
		const m = Math.floor((secs % 3600) / 60);
		if (d > 0) return `${d}d ${h}h`;
		return `${h}h ${m}m`;
	}

	onMount(() => startPolling('monitoring', loadData, 10000));
	onDestroy(() => stopPolling('monitoring'));
</script>

<div class="page">
	<h1 class="page-title">Monitoring</h1>

	{#if loading}
		<LoadingSkeleton rows={6} height={32} />
	{:else if metrics}
		<!-- Server Stats -->
		<div class="stats-row">
			<StatCard
				label="Memory"
				value={metrics.server.memory_mb ?? Math.round(metrics.server.memory_estimate_bytes / 1048576)}
				unit="MB"
				color="var(--blue)"
				sparklineData={memoryHistory}
			/>
			<StatCard
				label="Players"
				value={metrics.server.connected_players}
				color="var(--purple)"
				sparklineData={playerHistory}
			/>
			<StatCard
				label="WS Msg/s"
				value={Math.round(metrics.server.ws_messages_per_sec ?? 0)}
				color="var(--green)"
				sparklineData={wsHistory}
			/>
			<StatCard
				label="Uptime"
				value={formatUptime(metrics.server.uptime_secs)}
				color="var(--text-primary)"
			/>
		</div>

		<!-- Per-World Metrics -->
		<div class="section">
			<h2 class="section-title">World Performance</h2>
			{#if metrics.worlds.length === 0}
				<p class="empty-text">No worlds running</p>
			{:else}
				<div class="world-metrics">
					{#each metrics.worlds as world}
						<div class="world-metric-card">
							<button class="world-metric-header" onclick={() => (expandedWorld = expandedWorld === world.id ? null : world.id)}>
								<span class="wm-name">{world.name}</span>
								<div class="wm-stats">
									<span class="wm-stat"><span class="wm-label">Avg</span> {formatUs(world.avg_tick_us)}</span>
									<span class="wm-stat"><span class="wm-label">Max</span> {formatUs(world.max_tick_us ?? 0)}</span>
									<span class="wm-stat"><span class="wm-label">P99</span> {formatUs(world.p99_tick_us ?? 0)}</span>
									<span class="wm-stat"><span class="wm-label">Entities</span> {world.entity_count.toLocaleString()}</span>
								</div>
								{#if world.tick_history && world.tick_history.length > 1}
									<Sparkline data={world.tick_history} width={80} height={20} color="var(--blue)" />
								{/if}
								<span class="expand-icon">{expandedWorld === world.id ? '\u25BC' : '\u25B6'}</span>
							</button>

							{#if expandedWorld === world.id}
								<div class="system-breakdown">
									<h4>Per-System Tick Duration</h4>
									{#if world.system_times && Object.keys(world.system_times).length > 0}
										<div class="system-list">
											{#each Object.entries(world.system_times).sort((a, b) => Number(b[1]) - Number(a[1])) as [name, us]}
												<div class="system-row">
													<span class="sys-name">{name}</span>
													<div class="sys-bar-wrap">
														<div class="sys-bar" style="width: {Math.min(100, (Number(us) / Math.max(1, ...Object.values(world.system_times).map(Number))) * 100)}%; background: {systemColor(Number(us))};"></div>
													</div>
													<span class="sys-val" style="color: {systemColor(Number(us))}">{formatUs(Number(us))}</span>
												</div>
											{/each}
										</div>
									{:else}
										<p class="empty-text">No system timing data (requires server update)</p>
									{/if}
								</div>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.page { max-width: 1200px; }
	.page-title { font-size: 20px; font-weight: 700; margin-bottom: 20px; }
	.stats-row { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 12px; margin-bottom: 28px; }
	.section { margin-bottom: 24px; }
	.section-title { font-size: 14px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.04em; margin-bottom: 12px; }
	.empty-text { font-size: 13px; color: var(--text-dim); font-style: italic; }
	.world-metrics { display: flex; flex-direction: column; gap: 8px; }
	.world-metric-card { background: var(--bg-panel); border: 1px solid var(--border); border-radius: var(--radius-lg); overflow: hidden; }
	.world-metric-header { display: flex; align-items: center; gap: 16px; padding: 12px 16px; width: 100%; background: none; border: none; color: var(--text-primary); cursor: pointer; text-align: left; font-family: inherit; }
	.world-metric-header:hover { background: var(--bg-surface); }
	.wm-name { font-size: 14px; font-weight: 600; min-width: 120px; }
	.wm-stats { display: flex; gap: 16px; flex: 1; }
	.wm-stat { font-size: 12px; font-family: var(--font-mono); color: var(--text-secondary); }
	.wm-label { font-size: 10px; color: var(--text-dim); text-transform: uppercase; margin-right: 4px; }
	.expand-icon { font-size: 10px; color: var(--text-dim); }
	.system-breakdown { padding: 12px 16px; border-top: 1px solid var(--border); background: var(--bg-surface); }
	.system-breakdown h4 { font-size: 12px; font-weight: 600; color: var(--text-muted); margin-bottom: 8px; }
	.system-list { display: flex; flex-direction: column; gap: 3px; }
	.system-row { display: flex; align-items: center; gap: 8px; }
	.sys-name { font-size: 11px; font-family: var(--font-mono); color: var(--text-muted); min-width: 160px; }
	.sys-bar-wrap { flex: 1; height: 8px; background: var(--bg-panel); border-radius: 4px; overflow: hidden; }
	.sys-bar { height: 100%; border-radius: 4px; transition: width 0.3s; }
	.sys-val { font-size: 11px; font-family: var(--font-mono); min-width: 60px; text-align: right; }
	@media (max-width: 768px) {
		.world-metric-header {
			flex-wrap: wrap;
			gap: 8px;
		}
		.wm-stats {
			flex-wrap: wrap;
			gap: 8px;
		}
		.wm-name {
			min-width: auto;
			width: 100%;
		}
		.sys-name {
			min-width: 80px;
		}
	}
</style>
