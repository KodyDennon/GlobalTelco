<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import Badge from '$lib/components/Badge.svelte';
	import CopyButton from '$lib/components/CopyButton.svelte';
	import { toast } from '$lib/components/Toast.svelte';
	import { confirm } from '$lib/components/ConfirmDialog.svelte';
	import { fetchPlayers, kickPlayer, fetchAccounts, fetchBans, banPlayer, unbanPlayer } from '$lib/api/players.js';
	import { startPolling, stopPolling } from '$lib/stores/polling.js';
	import type { PlayerInfo, AccountInfo, Ban } from '$lib/api/types.js';

	let activeTab = $state<'connected' | 'accounts' | 'bans'>('connected');
	let players = $state<PlayerInfo[]>([]);
	let accounts = $state<AccountInfo[]>([]);
	let accountsTotal = $state(0);
	let accountsPage = $state(0);
	let accountsSearch = $state('');
	let bans = $state<Ban[]>([]);
	let loading = $state(true);

	// Ban form
	let showBanForm = $state(false);
	let banAccountId = $state('');
	let banReason = $state('');
	let banWorldId = $state('');
	let banExpiresAt = $state('');

	const playerColumns = [
		{ key: 'username', label: 'Username', sortable: true },
		{ key: 'world_id', label: 'World', sortable: true, format: (v: unknown) => v ? String(v).slice(0, 8) + '...' : 'Lobby' },
		{ key: 'corp_id', label: 'Corp', align: 'right' as const },
		{ key: 'is_guest', label: 'Status', format: (v: unknown, row: Record<string, unknown>) => {
			const parts: string[] = [];
			if (row.is_admin) parts.push('Admin');
			if (v) parts.push('Guest'); else parts.push('Registered');
			return parts.join(', ');
		}},
	];

	const accountColumns = [
		{ key: 'username', label: 'Username', sortable: true },
		{ key: 'email', label: 'Email', sortable: true, format: (v: unknown) => v || '--' },
		{ key: 'auth_provider', label: 'Provider', sortable: true, format: (v: unknown) => v || 'local' },
		{ key: 'created_at', label: 'Created', sortable: true, format: (v: unknown) => v ? new Date(v as string).toLocaleDateString() : '--' },
		{ key: 'last_login', label: 'Last Login', sortable: true, format: (v: unknown) => v ? new Date(v as string).toLocaleDateString() : 'Never' },
	];

	const banColumns = [
		{ key: 'username', label: 'Username', sortable: true },
		{ key: 'reason', label: 'Reason', sortable: true },
		{ key: 'world_id', label: 'Scope', format: (v: unknown) => v ? 'World' : 'Global' },
		{ key: 'banned_at', label: 'Banned', sortable: true, format: (v: unknown) => new Date(v as string).toLocaleDateString() },
		{ key: 'expires_at', label: 'Expires', sortable: true, format: (v: unknown) => v ? new Date(v as string).toLocaleDateString() : 'Never' },
	];

	async function loadPlayers() {
		try {
			const r = await fetchPlayers();
			players = r.players;
			loading = false;
		} catch { if (loading) loading = false; }
	}

	async function loadAccounts() {
		try {
			const r = await fetchAccounts(accountsSearch, accountsPage, 50);
			accounts = r.accounts;
			accountsTotal = r.total;
		} catch { /* ignore */ }
	}

	async function loadBans() {
		try { bans = await fetchBans(); } catch { /* ignore */ }
	}

	async function handleKick(id: string, username: string) {
		const ok = await confirm('Kick Player', `Kick "${username}"?`, { variant: 'warning' });
		if (!ok) return;
		try { await kickPlayer(id); toast(`${username} kicked`, 'success'); await loadPlayers(); }
		catch { toast('Kick failed', 'error'); }
	}

	async function handleQuickBan(id: string, username: string) {
		const reason = prompt(`Ban reason for ${username}:`);
		if (!reason) return;
		try { await banPlayer(id, reason); toast(`${username} banned`, 'success'); await Promise.all([loadPlayers(), loadBans()]); }
		catch { toast('Ban failed', 'error'); }
	}

	async function handleCreateBan() {
		if (!banAccountId.trim() || !banReason.trim()) { toast('Account ID and reason required', 'warning'); return; }
		try {
			await banPlayer(banAccountId, banReason, banWorldId || undefined, banExpiresAt || undefined);
			toast('Ban created', 'success');
			showBanForm = false;
			banAccountId = ''; banReason = ''; banWorldId = ''; banExpiresAt = '';
			await loadBans();
		} catch (e) { toast(`Ban failed: ${e}`, 'error'); }
	}

	async function handleUnban(accountId: string, worldId: string | null) {
		const ok = await confirm('Unban', 'Remove this ban?', { variant: 'info' });
		if (!ok) return;
		try { await unbanPlayer(accountId, worldId ?? undefined); toast('Ban removed', 'success'); await loadBans(); }
		catch { toast('Unban failed', 'error'); }
	}

	onMount(() => {
		startPolling('players', loadPlayers, 5000);
		loadAccounts();
		loadBans();
	});
	onDestroy(() => stopPolling('players'));

	$effect(() => {
		if (activeTab === 'accounts') loadAccounts();
		if (activeTab === 'bans') loadBans();
	});
</script>

<div class="page">
	<h1 class="page-title">Players</h1>

	<div class="tabs">
		{#each (['connected', 'accounts', 'bans'] as const) as tab}
			<button class="tab" class:active={activeTab === tab} onclick={() => (activeTab = tab)}>
				{tab === 'connected' ? `Connected (${players.length})` : tab === 'accounts' ? `All Accounts (${accountsTotal})` : `Bans (${bans.length})`}
			</button>
		{/each}
	</div>

	{#if activeTab === 'connected'}
		<DataTable columns={playerColumns} data={players as unknown as Record<string, unknown>[]} {loading} searchable searchPlaceholder="Search players..." emptyMessage="No players connected">
			{#snippet actions(row)}
				<div class="row-actions">
					<button class="btn-sm" onclick={() => handleKick(row.id as string, row.username as string)}>Kick</button>
					<button class="btn-sm btn-danger" onclick={() => handleQuickBan(row.id as string, row.username as string)}>Ban</button>
				</div>
			{/snippet}
		</DataTable>

	{:else if activeTab === 'accounts'}
		<div class="account-search">
			<input type="text" class="search-input" bind:value={accountsSearch} placeholder="Search accounts..." oninput={() => { accountsPage = 0; loadAccounts(); }} />
		</div>
		<DataTable columns={accountColumns} data={accounts as unknown as Record<string, unknown>[]} paginated pageSize={50} emptyMessage="No accounts found">
			{#snippet actions(row)}
				<div class="row-actions">
					<CopyButton text={row.id as string} label="ID" />
					<button class="btn-sm btn-danger" onclick={() => handleQuickBan(row.id as string, row.username as string)}>Ban</button>
				</div>
			{/snippet}
		</DataTable>

	{:else if activeTab === 'bans'}
		<div class="ban-header">
			<button class="btn-primary" onclick={() => (showBanForm = !showBanForm)}>
				{showBanForm ? 'Cancel' : '+ Create Ban'}
			</button>
		</div>

		{#if showBanForm}
			<div class="ban-form">
				<div class="form-grid">
					<div class="form-field"><label>Account ID</label><input type="text" bind:value={banAccountId} placeholder="UUID" /></div>
					<div class="form-field"><label>Reason</label><input type="text" bind:value={banReason} placeholder="Ban reason" /></div>
					<div class="form-field"><label>World ID (optional)</label><input type="text" bind:value={banWorldId} placeholder="Leave empty for global" /></div>
					<div class="form-field"><label>Expires (optional)</label><input type="datetime-local" bind:value={banExpiresAt} /></div>
				</div>
				<button class="btn-primary" onclick={handleCreateBan}>Create Ban</button>
			</div>
		{/if}

		<DataTable columns={banColumns} data={bans as unknown as Record<string, unknown>[]} searchable emptyMessage="No active bans">
			{#snippet actions(row)}
				<button class="btn-sm" onclick={() => handleUnban(row.account_id as string, row.world_id as string | null)}>Unban</button>
			{/snippet}
		</DataTable>
	{/if}
</div>

<style>
	.page { max-width: 1200px; }
	.page-title { font-size: 20px; font-weight: 700; margin-bottom: 16px; }
	.tabs { display: flex; gap: 2px; margin-bottom: 16px; border-bottom: 1px solid var(--border); }
	.tab { padding: 8px 16px; background: none; border: none; border-bottom: 2px solid transparent; color: var(--text-dim); font-size: 13px; font-weight: 500; cursor: pointer; }
	.tab:hover { color: var(--text-primary); }
	.tab.active { color: var(--blue); border-bottom-color: var(--blue); }
	.row-actions { display: flex; gap: 6px; }
	.btn-sm { padding: 3px 10px; font-size: 11px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-muted); cursor: pointer; }
	.btn-sm:hover { background: var(--bg-hover); }
	.btn-sm.btn-danger { color: var(--red); }
	.btn-primary { padding: 6px 16px; background: var(--blue); color: white; border: none; border-radius: var(--radius-md); font-size: 13px; cursor: pointer; }
	.ban-header { margin-bottom: 12px; }
	.ban-form { background: var(--bg-panel); border: 1px solid var(--border); border-radius: var(--radius-lg); padding: 16px; margin-bottom: 16px; display: flex; flex-direction: column; gap: 12px; }
	.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
	.form-field { display: flex; flex-direction: column; gap: 3px; }
	.form-field label { font-size: 11px; color: var(--text-dim); font-weight: 600; }
	.form-field input { padding: 6px 10px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-primary); font-size: 13px; font-family: inherit; }
	.account-search { margin-bottom: 12px; }
	.search-input { width: 300px; padding: 6px 12px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-md); color: var(--text-primary); font-size: 13px; font-family: inherit; }
</style>
