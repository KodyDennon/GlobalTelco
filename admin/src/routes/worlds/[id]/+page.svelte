<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { page } from '$app/state';
	import DataTable from '$lib/components/DataTable.svelte';
	import Badge from '$lib/components/Badge.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import LoadingSkeleton from '$lib/components/LoadingSkeleton.svelte';
	import { toast } from '$lib/components/Toast.svelte';
	import { confirm } from '$lib/components/ConfirmDialog.svelte';
	import { debugWorld, fetchWorldChat, fetchWorldVotes, setWorldSpeed } from '$lib/api/worlds.js';
	import { kickPlayer, banPlayer } from '$lib/api/players.js';
	import { startPolling, stopPolling } from '$lib/stores/polling.js';
	import type { WorldDebug, ChatMessage, SpeedVotes } from '$lib/api/types.js';

	const worldId = $derived(page.params.id!);
	let world = $state<WorldDebug | null>(null);
	let chat = $state<ChatMessage[]>([]);
	let votes = $state<SpeedVotes | null>(null);
	let loading = $state(true);
	let activeTab = $state<'corps' | 'players' | 'entities' | 'config' | 'chat' | 'votes'>('corps');

	const corpColumns = [
		{ key: 'name', label: 'Name', sortable: true },
		{ key: 'cash', label: 'Cash', sortable: true, align: 'right' as const, format: (v: unknown) => v != null ? `$${Number(v).toLocaleString()}` : '--' },
		{ key: 'revenue', label: 'Revenue', sortable: true, align: 'right' as const, format: (v: unknown) => v != null ? `$${Number(v).toLocaleString()}` : '--' },
		{ key: 'cost', label: 'Costs', sortable: true, align: 'right' as const, format: (v: unknown) => v != null ? `$${Number(v).toLocaleString()}` : '--' },
		{ key: 'debt', label: 'Debt', sortable: true, align: 'right' as const, format: (v: unknown) => v != null ? `$${Number(v).toLocaleString()}` : '--' },
		{ key: 'nodes', label: 'Nodes', sortable: true, align: 'right' as const },
	];

	async function loadData() {
		try {
			const d = await debugWorld(worldId);
			world = d;
			loading = false;
		} catch {
			if (loading) loading = false;
		}
	}

	async function loadChat() {
		try { const r = await fetchWorldChat(worldId, 100); chat = r.messages; } catch { /* ignore */ }
	}
	async function loadVotes() {
		try { votes = await fetchWorldVotes(worldId); } catch { /* ignore */ }
	}

	async function handleKick(playerId: string, username: string) {
		const ok = await confirm('Kick Player', `Kick "${username}" from this world?`, { variant: 'warning' });
		if (!ok) return;
		try { await kickPlayer(playerId); toast(`${username} kicked`, 'success'); await loadData(); }
		catch { toast('Failed to kick', 'error'); }
	}

	async function handleBan(playerId: string, username: string) {
		const reason = prompt(`Ban reason for ${username}:`);
		if (!reason) return;
		try { await banPlayer(playerId, reason, worldId); toast(`${username} banned`, 'success'); await loadData(); }
		catch { toast('Failed to ban', 'error'); }
	}

	async function handleSpeed(speed: string) {
		try { await setWorldSpeed(worldId, speed); toast(`Speed set to ${speed}`, 'success'); await loadData(); }
		catch { toast('Failed', 'error'); }
	}

	onMount(() => {
		startPolling('world-detail', loadData, 5000);
	});
	onDestroy(() => { stopPolling('world-detail'); });

	$effect(() => {
		if (activeTab === 'chat') loadChat();
		if (activeTab === 'votes') loadVotes();
	});
</script>

<div class="page">
	{#if loading}
		<LoadingSkeleton rows={6} height={24} />
	{:else if world}
		<a href="/worlds" class="back-link">Back to Worlds</a>

		<div class="world-header">
			<h1 class="page-title">{world.world_name}</h1>
			<div class="header-meta">
				<Badge color={world.speed === 'Paused' ? 'amber' : 'green'}>{world.speed}</Badge>
				<span class="meta-text">Tick {world.tick.toLocaleString()}</span>
				<select class="speed-select" value={world.speed} onchange={(e) => handleSpeed((e.target as HTMLSelectElement).value)}>
					{#each ['Paused', 'Normal', 'Fast', 'VeryFast', 'Ultra'] as s}
						<option value={s}>{s}</option>
					{/each}
				</select>
			</div>
		</div>

		<div class="stats-row">
			<StatCard label="Corporations" value={world.entity_counts.corporations ?? 0} color="var(--blue)" />
			<StatCard label="Infra Nodes" value={world.entity_counts.infra_nodes ?? 0} color="var(--green)" />
			<StatCard label="Infra Edges" value={world.entity_counts.infra_edges ?? 0} color="var(--purple)" />
			<StatCard label="Regions" value={world.entity_counts.regions ?? 0} color="var(--amber)" />
			<StatCard label="Cities" value={world.entity_counts.cities ?? 0} color="var(--text-primary)" />
		</div>

		<!-- Tabs -->
		<div class="tabs">
			{#each (['corps', 'players', 'entities', 'config', 'chat', 'votes'] as const) as tab}
				<button class="tab" class:active={activeTab === tab} onclick={() => (activeTab = tab)}>
					{tab === 'corps' ? 'Corporations' : tab === 'votes' ? 'Speed Votes' : tab.charAt(0).toUpperCase() + tab.slice(1)}
				</button>
			{/each}
		</div>

		{#if activeTab === 'corps'}
			<DataTable columns={corpColumns} data={world.corporations as unknown as Record<string, unknown>[]} searchable emptyMessage="No corporations" />

		{:else if activeTab === 'players'}
			{#if world.connected_players.length === 0}
				<p class="empty-text">No players connected</p>
			{:else}
				{#each world.connected_players as p}
					<div class="player-row">
						<span class="player-name">{p.username}</span>
						<Badge color={p.is_guest ? 'gray' : 'blue'}>{p.is_guest ? 'Guest' : 'Registered'}</Badge>
						<span class="player-corp">Corp: {p.corp_id ?? '--'}</span>
						<div class="player-actions">
							<button class="btn-sm" onclick={() => handleKick(p.id, p.username)}>Kick</button>
							<button class="btn-sm btn-danger" onclick={() => handleBan(p.id, p.username)}>Ban</button>
						</div>
					</div>
				{/each}
			{/if}

		{:else if activeTab === 'entities'}
			<div class="entity-grid">
				{#each Object.entries(world.entity_counts) as [key, count]}
					<div class="entity-item">
						<span class="entity-key">{key.replace(/_/g, ' ')}</span>
						<span class="entity-val">{Number(count).toLocaleString()}</span>
					</div>
				{/each}
			</div>

		{:else if activeTab === 'config'}
			<pre class="config-display">{JSON.stringify({ world_id: world.world_id, tick_rate_ms: world.tick_rate_ms, broadcast_subscribers: world.broadcast_subscribers }, null, 2)}</pre>

		{:else if activeTab === 'chat'}
			{#if chat.length === 0}
				<p class="empty-text">No chat messages</p>
			{:else}
				<div class="chat-log">
					{#each chat as msg}
						<div class="chat-msg">
							<span class="chat-user">{msg.username}</span>
							<span class="chat-text">{msg.message}</span>
							<span class="chat-time">{new Date(msg.created_at).toLocaleTimeString()}</span>
						</div>
					{/each}
				</div>
			{/if}

		{:else if activeTab === 'votes'}
			{#if votes}
				<p class="meta-text">Current speed: <strong>{votes.current_speed}</strong> | Creator: {votes.creator_id ?? 'None'}</p>
				{#if Object.keys(votes.votes).length === 0}
					<p class="empty-text">No active votes</p>
				{:else}
					{#each Object.entries(votes.votes) as [pid, speed]}
						<div class="vote-row">
							<span class="vote-player">{pid}</span>
							<Badge color="blue">{speed}</Badge>
						</div>
					{/each}
				{/if}
			{:else}
				<LoadingSkeleton rows={3} />
			{/if}
		{/if}
	{:else}
		<p class="empty-text">World not found</p>
	{/if}
</div>

<style>
	.page { max-width: 1200px; }
	.back-link { font-size: 12px; color: var(--text-dim); text-decoration: none; margin-bottom: 8px; display: inline-block; }
	.back-link:hover { color: var(--blue); }
	.page-title { font-size: 20px; font-weight: 700; }
	.world-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
	.header-meta { display: flex; align-items: center; gap: 10px; }
	.meta-text { font-size: 13px; color: var(--text-dim); font-family: var(--font-mono); }
	.speed-select { padding: 3px 8px; font-size: 12px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-primary); }
	.stats-row { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 10px; margin-bottom: 20px; }
	.tabs { display: flex; gap: 2px; margin-bottom: 16px; border-bottom: 1px solid var(--border); }
	.tab { padding: 8px 16px; background: none; border: none; border-bottom: 2px solid transparent; color: var(--text-dim); font-size: 13px; font-weight: 500; cursor: pointer; }
	.tab:hover { color: var(--text-primary); }
	.tab.active { color: var(--blue); border-bottom-color: var(--blue); }
	.empty-text { font-size: 13px; color: var(--text-dim); font-style: italic; padding: 16px 0; }
	.player-row { display: flex; align-items: center; gap: 10px; padding: 8px 12px; background: var(--bg-panel); border: 1px solid var(--border); border-radius: var(--radius-md); margin-bottom: 4px; }
	.player-name { font-size: 13px; font-weight: 500; flex: 1; }
	.player-corp { font-size: 11px; color: var(--text-dim); font-family: var(--font-mono); }
	.player-actions { display: flex; gap: 6px; }
	.btn-sm { padding: 3px 10px; font-size: 11px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-muted); cursor: pointer; }
	.btn-sm:hover { background: var(--bg-hover); }
	.btn-sm.btn-danger { color: var(--red); }
	.entity-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 8px; }
	.entity-item { display: flex; justify-content: space-between; padding: 8px 12px; background: var(--bg-panel); border: 1px solid var(--border); border-radius: var(--radius-md); }
	.entity-key { font-size: 13px; color: var(--text-muted); text-transform: capitalize; }
	.entity-val { font-size: 13px; font-family: var(--font-mono); font-weight: 600; }
	.config-display { background: var(--bg-panel); border: 1px solid var(--border); border-radius: var(--radius-md); padding: 16px; font-size: 12px; font-family: var(--font-mono); color: var(--text-secondary); overflow-x: auto; white-space: pre-wrap; }
	.chat-log { display: flex; flex-direction: column; gap: 4px; max-height: 500px; overflow-y: auto; }
	.chat-msg { display: flex; gap: 8px; padding: 4px 8px; font-size: 13px; }
	.chat-user { font-weight: 600; color: var(--blue); min-width: 100px; }
	.chat-text { flex: 1; color: var(--text-secondary); }
	.chat-time { font-size: 11px; color: var(--text-faint); font-family: var(--font-mono); }
	.vote-row { display: flex; align-items: center; gap: 10px; padding: 6px 12px; background: var(--bg-panel); border: 1px solid var(--border); border-radius: var(--radius-md); margin-bottom: 4px; }
	.vote-player { font-size: 12px; font-family: var(--font-mono); color: var(--text-muted); flex: 1; }
</style>
