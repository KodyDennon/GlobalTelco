<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import Badge from '$lib/components/Badge.svelte';
	import { toast } from '$lib/components/Toast.svelte';
	import { confirm } from '$lib/components/ConfirmDialog.svelte';
	import { fetchConnections } from '$lib/api/multiplayer.js';
	import { fetchWorlds, fetchWorldVotes, transferWorld, assignPlayer, setWorldSpeed } from '$lib/api/worlds.js';
	import { kickPlayer } from '$lib/api/players.js';
	import { startPolling, stopPolling } from '$lib/stores/polling.js';
	import type { ConnectionInfo } from '$lib/api/types.js';

	let activeTab = $state<'connections' | 'votes' | 'ownership' | 'assign'>('connections');
	let connections = $state<ConnectionInfo[]>([]);
	let worlds = $state<Record<string, unknown>[]>([]);
	let loading = $state(true);

	// Votes
	let votesWorldId = $state('');
	let votesData = $state<{ votes: Record<string, string>; current_speed: string; creator_id: string | null } | null>(null);

	// Transfer
	let transferWorldId = $state('');
	let transferNewOwner = $state('');

	// Assign
	let assignWorldId = $state('');
	let assignPlayerId = $state('');
	let assignCorpId = $state(0);

	const connColumns = [
		{ key: 'username', label: 'Username', sortable: true },
		{ key: 'world_name', label: 'World', sortable: true, format: (v: unknown) => v || 'Lobby' },
		{ key: 'is_guest', label: 'Type', format: (v: unknown) => v ? 'Guest' : 'Registered' },
		{ key: 'is_spectator', label: 'Spectator', format: (v: unknown) => v ? 'Yes' : 'No' },
		{ key: 'connected_at', label: 'Connected', format: (v: unknown) => {
			if (!v) return '--';
			const secs = Math.floor((Date.now() / 1000) - Number(v));
			if (secs < 60) return `${secs}s`;
			if (secs < 3600) return `${Math.floor(secs / 60)}m`;
			return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
		}},
	];

	async function loadData() {
		try {
			const [c, w] = await Promise.all([fetchConnections(), fetchWorlds()]);
			connections = c.connections;
			worlds = w.worlds as unknown as Record<string, unknown>[];
			loading = false;
		} catch { if (loading) loading = false; }
	}

	async function handleDisconnect(id: string, username: string) {
		const ok = await confirm('Disconnect', `Disconnect "${username}"?`, { variant: 'warning' });
		if (!ok) return;
		try { await kickPlayer(id); toast(`${username} disconnected`, 'success'); await loadData(); }
		catch { toast('Failed', 'error'); }
	}

	async function loadVotes() {
		if (!votesWorldId) return;
		try { votesData = await fetchWorldVotes(votesWorldId); }
		catch { toast('Failed to load votes', 'error'); }
	}

	async function handleOverrideSpeed(speed: string) {
		if (!votesWorldId) return;
		try { await setWorldSpeed(votesWorldId, speed); toast(`Speed overridden to ${speed}`, 'success'); await loadVotes(); }
		catch { toast('Failed', 'error'); }
	}

	async function handleTransfer() {
		if (!transferWorldId || !transferNewOwner) { toast('Select world and new owner', 'warning'); return; }
		const ok = await confirm('Transfer Ownership', `Transfer world ownership to ${transferNewOwner}?`, { variant: 'warning' });
		if (!ok) return;
		try { await transferWorld(transferWorldId, transferNewOwner); toast('Ownership transferred', 'success'); }
		catch { toast('Failed', 'error'); }
	}

	async function handleAssign() {
		if (!assignWorldId || !assignPlayerId) { toast('Select world and player', 'warning'); return; }
		try { await assignPlayer(assignWorldId, assignPlayerId, assignCorpId); toast('Player assigned', 'success'); }
		catch { toast('Failed', 'error'); }
	}

	onMount(() => startPolling('multiplayer', loadData, 5000));
	onDestroy(() => stopPolling('multiplayer'));
</script>

<div class="page">
	<h1 class="page-title">Multiplayer</h1>

	<div class="tabs">
		{#each (['connections', 'votes', 'ownership', 'assign'] as const) as tab}
			<button class="tab" class:active={activeTab === tab} onclick={() => (activeTab = tab)}>
				{tab === 'connections' ? `Connections (${connections.length})` : tab === 'votes' ? 'Speed Votes' : tab === 'ownership' ? 'World Ownership' : 'Force Assign'}
			</button>
		{/each}
	</div>

	{#if activeTab === 'connections'}
		<DataTable columns={connColumns} data={connections as unknown as Record<string, unknown>[]} {loading} searchable emptyMessage="No active connections">
			{#snippet actions(row)}
				<button class="btn-sm btn-danger" onclick={() => handleDisconnect(row.id as string, row.username as string)}>Disconnect</button>
			{/snippet}
		</DataTable>

	{:else if activeTab === 'votes'}
		<div class="control-form">
			<div class="form-row">
				<label>World</label>
				<select bind:value={votesWorldId} onchange={loadVotes}>
					<option value="">Select world...</option>
					{#each worlds as w}
						<option value={w.id as string}>{w.name}</option>
					{/each}
				</select>
			</div>
			{#if votesData}
				<p class="meta">Current: <strong>{votesData.current_speed}</strong> | Creator: {votesData.creator_id?.slice(0, 8) ?? 'None'}</p>
				{#if Object.keys(votesData.votes).length > 0}
					<div class="vote-list">
						{#each Object.entries(votesData.votes) as [pid, speed]}
							<div class="vote-item">
								<span class="vote-pid">{pid.slice(0, 8)}...</span>
								<Badge color="blue">{speed}</Badge>
							</div>
						{/each}
					</div>
				{:else}
					<p class="empty-text">No active votes</p>
				{/if}
				<div class="form-row">
					<label>Admin Override</label>
					<div class="speed-btns">
						{#each ['Paused', 'Normal', 'Fast', 'VeryFast', 'Ultra'] as s}
							<button class="btn-sm" class:active={votesData.current_speed === s} onclick={() => handleOverrideSpeed(s)}>{s}</button>
						{/each}
					</div>
				</div>
			{/if}
		</div>

	{:else if activeTab === 'ownership'}
		<div class="control-form">
			<div class="form-row">
				<label>World</label>
				<select bind:value={transferWorldId}>
					<option value="">Select world...</option>
					{#each worlds as w}
						<option value={w.id as string}>{w.name}</option>
					{/each}
				</select>
			</div>
			<div class="form-row">
				<label>New Owner ID</label>
				<input type="text" bind:value={transferNewOwner} placeholder="Player UUID" />
			</div>
			<button class="btn-primary" onclick={handleTransfer}>Transfer</button>
		</div>

	{:else if activeTab === 'assign'}
		<div class="control-form">
			<div class="form-row">
				<label>World</label>
				<select bind:value={assignWorldId}>
					<option value="">Select world...</option>
					{#each worlds as w}
						<option value={w.id as string}>{w.name}</option>
					{/each}
				</select>
			</div>
			<div class="form-row">
				<label>Player ID</label>
				<input type="text" bind:value={assignPlayerId} placeholder="Player UUID" />
			</div>
			<div class="form-row">
				<label>Corp ID</label>
				<input type="number" bind:value={assignCorpId} placeholder="Corporation entity ID" />
			</div>
			<button class="btn-primary" onclick={handleAssign}>Assign</button>
		</div>
	{/if}
</div>

<style>
	.page { max-width: 1200px; }
	.page-title { font-size: 20px; font-weight: 700; margin-bottom: 16px; }
	.tabs { display: flex; gap: 2px; margin-bottom: 16px; border-bottom: 1px solid var(--border); }
	.tab { padding: 8px 16px; background: none; border: none; border-bottom: 2px solid transparent; color: var(--text-dim); font-size: 13px; font-weight: 500; cursor: pointer; }
	.tab:hover { color: var(--text-primary); }
	.tab.active { color: var(--blue); border-bottom-color: var(--blue); }
	.control-form { background: var(--bg-panel); border: 1px solid var(--border); border-radius: var(--radius-lg); padding: 16px; display: flex; flex-direction: column; gap: 12px; max-width: 500px; }
	.form-row { display: flex; flex-direction: column; gap: 4px; }
	.form-row label { font-size: 11px; color: var(--text-dim); font-weight: 600; text-transform: uppercase; }
	.form-row select, .form-row input { padding: 6px 10px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-primary); font-size: 13px; font-family: inherit; }
	.btn-primary { padding: 6px 16px; background: var(--blue); color: white; border: none; border-radius: var(--radius-md); font-size: 13px; cursor: pointer; align-self: flex-start; }
	.btn-sm { padding: 3px 10px; font-size: 11px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-muted); cursor: pointer; }
	.btn-sm:hover { background: var(--bg-hover); }
	.btn-sm.active { background: var(--blue); color: white; border-color: var(--blue); }
	.btn-sm.btn-danger { color: var(--red); }
	.meta { font-size: 13px; color: var(--text-dim); }
	.meta strong { color: var(--text-primary); }
	.empty-text { font-size: 13px; color: var(--text-dim); font-style: italic; }
	.vote-list { display: flex; flex-direction: column; gap: 4px; }
	.vote-item { display: flex; align-items: center; gap: 8px; padding: 4px 8px; background: var(--bg-surface); border-radius: var(--radius-sm); }
	.vote-pid { font-size: 12px; font-family: var(--font-mono); color: var(--text-muted); }
	.speed-btns { display: flex; gap: 4px; }
</style>
