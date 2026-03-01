<script lang="ts">
	import { fetchWorldHistory } from '$lib/multiplayer/lobbyApi';
	import type { WorldHistoryEntry } from '$lib/multiplayer/lobbyApi';
	import { onMount } from 'svelte';

	let { onJoin }: { onJoin: (worldId: string) => void } = $props();

	let history = $state<WorldHistoryEntry[]>([]);
	let loading = $state(true);
	let error = $state('');

	onMount(async () => {
		try {
			history = await fetchWorldHistory();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load history';
		} finally {
			loading = false;
		}
	});

	function formatDate(iso: string): string {
		const d = new Date(iso);
		const now = new Date();
		const diff = now.getTime() - d.getTime();
		const mins = Math.floor(diff / 60000);
		if (mins < 60) return `${mins}m ago`;
		const hours = Math.floor(mins / 60);
		if (hours < 24) return `${hours}h ago`;
		const days = Math.floor(hours / 24);
		if (days < 7) return `${days}d ago`;
		return d.toLocaleDateString();
	}
</script>

<div class="recent-container">
	<h3>Recent Worlds</h3>

	{#if loading}
		<div class="loading">Loading history...</div>
	{:else if error}
		<div class="error">{error}</div>
	{:else if history.length === 0}
		<div class="empty">No recent worlds. Join a world to see it here.</div>
	{:else}
		<div class="history-list">
			{#each history as entry}
				<div class="history-card">
					<div class="history-info">
						<span class="history-name">{entry.world_name}</span>
						<span class="history-time">{formatDate(entry.last_played)}</span>
					</div>
					<button class="btn-rejoin" onclick={() => onJoin(entry.world_id)}>
						Rejoin
					</button>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.recent-container {
		max-width: 600px;
	}

	h3 {
		font-size: 18px;
		margin: 0 0 12px;
	}

	.loading,
	.empty {
		text-align: center;
		color: #6b7280;
		padding: 40px;
	}

	.error {
		color: #ef4444;
		font-size: 13px;
	}

	.history-list {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.history-card {
		display: flex;
		align-items: center;
		justify-content: space-between;
		background: rgba(31, 41, 55, 0.6);
		border: 1px solid rgba(55, 65, 81, 0.3);
		border-radius: 8px;
		padding: 12px 16px;
	}

	.history-info {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.history-name {
		font-size: 14px;
		color: #d1d5db;
	}

	.history-time {
		font-size: 12px;
		color: #6b7280;
	}

	.btn-rejoin {
		background: rgba(59, 130, 246, 0.2);
		border: 1px solid rgba(59, 130, 246, 0.3);
		color: #60a5fa;
		padding: 6px 16px;
		border-radius: 6px;
		cursor: pointer;
		font-family: system-ui, sans-serif;
		font-size: 13px;
	}

	.btn-rejoin:hover {
		background: rgba(59, 130, 246, 0.3);
	}
</style>
