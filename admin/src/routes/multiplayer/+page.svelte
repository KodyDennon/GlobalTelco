<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import Badge from '$lib/components/Badge.svelte';
	import LoadingSkeleton from '$lib/components/LoadingSkeleton.svelte';
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
	let error = $state<string | null>(null);

	// Votes
	let votesWorldId = $state('');
	let votesData = $state<{ votes: Record<string, string>; current_speed: string; creator_id: string | null } | null>(null);
	let votesLoading = $state(false);

	// Transfer
	let transferWorldId = $state('');
	let transferNewOwner = $state('');
	let transferring = $state(false);

	// Assign
	let assignWorldId = $state('');
	let assignPlayerId = $state('');
	let assignCorpId = $state(0);
	let assigning = $state(false);

	const connColumns = [
		{ key: 'username', label: 'Username', sortable: true },
		{ key: 'world_name', label: 'World', sortable: true, format: (v: unknown) => (v ? String(v) : 'Lobby') },
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
			worlds = w as unknown as Record<string, unknown>[];
			error = null;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load data';
		} finally {
			loading = false;
		}
	}

	async function handleDisconnect(id: string, username: string) {
		const ok = await confirm('Disconnect', `Disconnect "${username}"?`, { variant: 'warning' });
		if (!ok) return;
		try { await kickPlayer(id); toast(`${username} disconnected`, 'success'); await loadData(); }
		catch { toast('Failed', 'error'); }
	}

	async function loadVotes() {
		if (!votesWorldId) return;
		votesLoading = true;
		try { votesData = await fetchWorldVotes(votesWorldId); }
		catch { toast('Failed to load votes', 'error'); }
		finally { votesLoading = false; }
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
		transferring = true;
		try { await transferWorld(transferWorldId, transferNewOwner); toast('Ownership transferred', 'success'); }
		catch { toast('Failed', 'error'); }
		finally { transferring = false; }
	}

	async function handleAssign() {
		if (!assignWorldId || !assignPlayerId) { toast('Select world and player', 'warning'); return; }
		assigning = true;
		try { await assignPlayer(assignWorldId, assignPlayerId, assignCorpId); toast('Player assigned', 'success'); }
		catch { toast('Failed', 'error'); }
		finally { assigning = false; }
	}

	onMount(() => startPolling('multiplayer', loadData));
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
		{#if error && connections.length === 0}
			<div class="error-inline">
				<span>{error}</span>
				<button onclick={loadData}>Retry</button>
			</div>
		{/if}
		<DataTable columns={connColumns} data={connections as unknown as Record<string, unknown>[]} {loading} searchable emptyMessage="No active connections">
			{#snippet actions(row)}
				<button class="btn-sm btn-danger" onclick={() => handleDisconnect(row.id as string, row.username as string)}>Disconnect</button>
			{/snippet}
		</DataTable>

	{:else if activeTab === 'votes'}
		<div class="control-form">
			<div class="form-row">
				<label for="votes-world">World</label>
				<select id="votes-world" bind:value={votesWorldId} onchange={loadVotes}>
					<option value="">Select world...</option>
					{#each worlds as w}
						<option value={w.id as string}>{w.name}</option>
					{/each}
				</select>
			</div>
			{#if votesLoading}
				<LoadingSkeleton rows={2} height={20} />
			{:else if votesData}
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
					<span class="field-label">Admin Override</span>
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
				<label for="trans-world">World</label>
				<select id="trans-world" bind:value={transferWorldId}>
					<option value="">Select world...</option>
					{#each worlds as w}
						<option value={w.id as string}>{w.name}</option>
					{/each}
				</select>
			</div>
			<div class="form-row">
				<label for="trans-owner">New Owner ID</label>
				<input id="trans-owner" type="text" bind:value={transferNewOwner} placeholder="Player UUID" />
			</div>
			<button class="btn-primary" onclick={handleTransfer} disabled={transferring}>
				{transferring ? 'Transferring...' : 'Transfer'}
			</button>
		</div>

	{:else if activeTab === 'assign'}
		<div class="control-form">
			<div class="form-row">
				<label for="assign-world">World</label>
				<select id="assign-world" bind:value={assignWorldId}>
					<option value="">Select world...</option>
					{#each worlds as w}
						<option value={w.id as string}>{w.name}</option>
					{/each}
				</select>
			</div>
			<div class="form-row">
				<label for="assign-player">Player ID</label>
				<input id="assign-player" type="text" bind:value={assignPlayerId} placeholder="Player UUID" />
			</div>
			<div class="form-row">
				<label for="assign-corp">Corp ID</label>
				<input id="assign-corp" type="number" bind:value={assignCorpId} placeholder="Corporation entity ID" />
			</div>
			<button class="btn-primary" onclick={handleAssign} disabled={assigning}>
				{assigning ? 'Assigning...' : 'Assign'}
			</button>
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
	.btn-primary:disabled { opacity: 0.5; cursor: default; }
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
	.error-inline { display: flex; align-items: center; justify-content: space-between; padding: 8px 14px; background: var(--red-bg); border: 1px solid rgba(239, 68, 68, 0.3); border-radius: var(--radius-md); margin-bottom: 12px; font-size: 12px; color: var(--red-light); }
	.error-inline button { padding: 2px 10px; background: rgba(239, 68, 68, 0.2); border: 1px solid rgba(239, 68, 68, 0.3); border-radius: var(--radius-sm); color: var(--red-light); font-size: 11px; cursor: pointer; }
	@media (max-width: 768px) {
		.tabs { overflow-x: auto; flex-wrap: nowrap; scrollbar-width: none; }
		.tabs::-webkit-scrollbar { display: none; }
		.speed-btns { flex-wrap: wrap; }
		.vote-item { flex-wrap: wrap; }
		.control-form { max-width: 100%; }
	}
</style>
