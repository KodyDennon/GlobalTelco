<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import Badge from '$lib/components/Badge.svelte';
	import CopyButton from '$lib/components/CopyButton.svelte';
	import LoadingSkeleton from '$lib/components/LoadingSkeleton.svelte';
	import { toast } from '$lib/components/Toast.svelte';
	import { confirm } from '$lib/components/ConfirmDialog.svelte';
	import { fetchServerConfig, broadcast, fetchResetQueue, resolveReset } from '$lib/api/settings.js';
	import { fetchAuditLog } from '$lib/api/audit.js';
	import { startPolling, stopPolling } from '$lib/stores/polling.js';
	import { preferences } from '$lib/stores/preferences.js';
	import type { ServerConfig, ResetRequest, AuditEntry } from '$lib/api/types.js';

	let activeTab = $state<'config' | 'broadcast' | 'resets' | 'preferences'>('config');
	let config = $state<ServerConfig | null>(null);
	let resets = $state<ResetRequest[]>([]);
	let loading = $state(true);

	// Broadcast
	let broadcastMessage = $state('');
	let broadcastWorldId = $state('');
	let broadcastHistory = $state<AuditEntry[]>([]);
	let sendingBroadcast = $state(false);

	// Preferences
	let refreshInterval = $state($preferences.refreshInterval);
	let autoRefresh = $state($preferences.autoRefresh);
	let sidebarCollapsed = $state($preferences.sidebarCollapsed);

	// Reset queue
	let generatedPasswords = $state<Record<string, string>>({});

	const resetColumns = [
		{ key: 'username', label: 'Username', sortable: true },
		{ key: 'status', label: 'Status', sortable: true },
		{ key: 'created_at', label: 'Requested', sortable: true, format: (v: unknown) => v ? new Date(v as string).toLocaleString() : '--' },
	];

	const envVarDescriptions: Record<string, string> = {
		ADMIN_KEY: 'Admin dashboard authentication key',
		DATABASE_URL: 'PostgreSQL connection string',
		JWT_SECRET: 'JSON Web Token signing secret',
		GITHUB_CLIENT_ID: 'GitHub OAuth application ID',
		GITHUB_CLIENT_SECRET: 'GitHub OAuth application secret',
		TILE_DIR: 'Map tile data directory path',
		R2_ACCOUNT_ID: 'Cloudflare R2 storage account',
		R2_ACCESS_KEY_ID: 'Cloudflare R2 access key',
		R2_SECRET_ACCESS_KEY: 'Cloudflare R2 secret key',
		R2_BUCKET_NAME: 'Cloudflare R2 bucket name',
		CORS_ORIGINS: 'Allowed CORS origins (comma-separated)',
		PORT: 'Server listen port',
	};

	async function loadConfig() {
		try {
			config = await fetchServerConfig();
			loading = false;
		} catch { if (loading) loading = false; }
	}

	async function loadResets() {
		try {
			resets = await fetchResetQueue();
		} catch { /* ignore */ }
	}

	async function loadBroadcastHistory() {
		try {
			const r = await fetchAuditLog(20, 0, undefined);
			broadcastHistory = r.audit_log.filter(e => e.action === 'broadcast');
		} catch { /* ignore */ }
	}

	async function handleBroadcast() {
		if (!broadcastMessage.trim()) { toast('Message cannot be empty', 'warning'); return; }
		const ok = await confirm('Send Broadcast', `Send message to ${broadcastWorldId ? 'selected world' : 'all players'}?\n\n"${broadcastMessage}"`, { variant: 'warning' });
		if (!ok) return;
		sendingBroadcast = true;
		try {
			await broadcast(broadcastMessage, broadcastWorldId || undefined);
			toast('Broadcast sent', 'success');
			broadcastMessage = '';
			await loadBroadcastHistory();
		} catch { toast('Broadcast failed', 'error'); }
		sendingBroadcast = false;
	}

	async function handleResolveReset(accountId: string, username: string) {
		const ok = await confirm('Generate Temp Password', `Generate a temporary password for "${username}"? The user will need to change it on next login.`, { variant: 'warning' });
		if (!ok) return;
		try {
			const r = await resolveReset(accountId);
			generatedPasswords = { ...generatedPasswords, [accountId]: r.temp_password };
			toast(`Temp password generated for ${username}`, 'success');
			await loadResets();
		} catch { toast('Failed to resolve reset', 'error'); }
	}

	function savePreferences() {
		$preferences = {
			refreshInterval,
			autoRefresh,
			sidebarCollapsed,
		};
		toast('Preferences saved', 'success');
	}

	onMount(() => {
		loadConfig();
		startPolling('settings', loadConfig, 30000);
	});
	onDestroy(() => stopPolling('settings'));

	$effect(() => {
		if (activeTab === 'resets') loadResets();
		if (activeTab === 'broadcast') loadBroadcastHistory();
	});
</script>

<div class="page">
	<h1 class="page-title">Settings</h1>

	<div class="tabs">
		{#each (['config', 'broadcast', 'resets', 'preferences'] as const) as tab}
			<button class="tab" class:active={activeTab === tab} onclick={() => (activeTab = tab)}>
				{tab === 'config' ? 'Server Config' : tab === 'broadcast' ? 'Broadcast' : tab === 'resets' ? 'Reset Queue' : 'Preferences'}
			</button>
		{/each}
	</div>

	{#if activeTab === 'config'}
		{#if loading}
			<LoadingSkeleton rows={8} height={28} />
		{:else if config}
			<div class="config-section">
				<h2 class="section-title">Environment Variables</h2>
				<div class="env-grid">
					{#each Object.entries(config.env_vars) as [key, isSet]}
						<div class="env-row">
							<div class="env-info">
								<span class="env-key">{key}</span>
								<span class="env-desc">{envVarDescriptions[key] ?? ''}</span>
							</div>
							<Badge color={isSet ? 'green' : 'gray'}>{isSet ? 'Set' : 'Not Set'}</Badge>
						</div>
					{/each}
				</div>
			</div>

			<div class="config-section">
				<h2 class="section-title">Database</h2>
				<div class="db-grid">
					<div class="db-item">
						<span class="db-label">Connection</span>
						<Badge color={config.database.connected ? 'green' : 'red'}>{config.database.connected ? 'Connected' : 'Disconnected'}</Badge>
					</div>
					<div class="db-item">
						<span class="db-label">Pool Size</span>
						<span class="db-value">{config.database.pool_size}</span>
					</div>
				</div>
			</div>

			<div class="config-section">
				<h2 class="section-title">Features</h2>
				<div class="feature-grid">
					<div class="feature-item">
						<span class="feature-label">PostgreSQL</span>
						<Badge color={config.features.postgres ? 'green' : 'gray'}>{config.features.postgres ? 'Enabled' : 'Disabled'}</Badge>
					</div>
					<div class="feature-item">
						<span class="feature-label">Cloudflare R2</span>
						<Badge color={config.features.r2 ? 'green' : 'gray'}>{config.features.r2 ? 'Enabled' : 'Disabled'}</Badge>
					</div>
				</div>
			</div>
		{:else}
			<p class="empty-text">Failed to load server config</p>
		{/if}

	{:else if activeTab === 'broadcast'}
		<div class="broadcast-section">
			<div class="broadcast-form">
				<div class="form-row">
					<label for="broadcast-msg">Message</label>
					<textarea id="broadcast-msg" bind:value={broadcastMessage} placeholder="Enter message to broadcast..." rows={3} class="broadcast-input"></textarea>
				</div>
				<div class="form-row">
					<label for="broadcast-target">Target</label>
					<div class="target-row">
						<select id="broadcast-target" bind:value={broadcastWorldId} class="target-select">
							<option value="">All Players</option>
						</select>
						<button class="btn-primary" onclick={handleBroadcast} disabled={sendingBroadcast || !broadcastMessage.trim()}>
							{sendingBroadcast ? 'Sending...' : 'Send Broadcast'}
						</button>
					</div>
				</div>
			</div>

			{#if broadcastHistory.length > 0}
				<div class="history-section">
					<h3 class="section-title">Recent Broadcasts</h3>
					<div class="broadcast-history">
						{#each broadcastHistory as entry}
							<div class="history-item">
								<div class="history-meta">
									<span class="history-actor">{entry.actor}</span>
									<span class="history-time">{entry.created_at ? new Date(entry.created_at).toLocaleString() : '--'}</span>
								</div>
								<span class="history-details">{entry.details ? (typeof entry.details === 'string' ? entry.details : JSON.stringify(entry.details)) : '--'}</span>
							</div>
						{/each}
					</div>
				</div>
			{/if}
		</div>

	{:else if activeTab === 'resets'}
		<DataTable columns={resetColumns} data={resets as unknown as Record<string, unknown>[]} searchable emptyMessage="No pending reset requests">
			{#snippet actions(row)}
				<div class="reset-actions">
					{#if generatedPasswords[row.account_id as string]}
						<div class="temp-password">
							<code>{generatedPasswords[row.account_id as string]}</code>
							<CopyButton text={generatedPasswords[row.account_id as string]} label="Password" />
						</div>
					{:else}
						<button class="btn-sm btn-primary-sm" onclick={() => handleResolveReset(row.account_id as string, row.username as string)}>
							Generate Temp Password
						</button>
					{/if}
				</div>
			{/snippet}
		</DataTable>

	{:else if activeTab === 'preferences'}
		<div class="prefs-form">
			<div class="pref-group">
				<h3 class="pref-title">Auto-Refresh</h3>
				<div class="pref-row">
					<label class="pref-label" for="pref-auto">Enable Auto-Refresh</label>
					<label class="toggle">
						<input id="pref-auto" type="checkbox" bind:checked={autoRefresh} />
						<span class="toggle-track"><span class="toggle-thumb"></span></span>
					</label>
				</div>
				<div class="pref-row">
					<span class="pref-label">Refresh Interval</span>
					<div class="interval-options">
						{#each [5000, 10000, 30000, 60000] as ms}
							<button
								class="interval-btn"
								class:active={refreshInterval === ms}
								onclick={() => (refreshInterval = ms)}
							>
								{ms / 1000}s
							</button>
						{/each}
					</div>
				</div>
			</div>

			<div class="pref-group">
				<h3 class="pref-title">Layout</h3>
				<div class="pref-row">
					<label class="pref-label" for="pref-sidebar">Sidebar Collapsed</label>
					<label class="toggle">
						<input id="pref-sidebar" type="checkbox" bind:checked={sidebarCollapsed} />
						<span class="toggle-track"><span class="toggle-thumb"></span></span>
					</label>
				</div>
			</div>

			<button class="btn-primary" onclick={savePreferences}>Save Preferences</button>
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
	.empty-text { font-size: 13px; color: var(--text-dim); font-style: italic; }

	/* Config */
	.config-section { margin-bottom: 24px; }
	.section-title { font-size: 14px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.04em; margin-bottom: 12px; }
	.env-grid { display: flex; flex-direction: column; gap: 4px; }
	.env-row { display: flex; align-items: center; justify-content: space-between; padding: 8px 12px; background: var(--bg-panel); border: 1px solid var(--border); border-radius: var(--radius-md); }
	.env-info { display: flex; flex-direction: column; gap: 2px; }
	.env-key { font-size: 13px; font-family: var(--font-mono); font-weight: 600; color: var(--text-primary); }
	.env-desc { font-size: 11px; color: var(--text-dim); }
	.db-grid, .feature-grid { display: flex; gap: 16px; flex-wrap: wrap; }
	.db-item, .feature-item { display: flex; align-items: center; gap: 8px; padding: 8px 12px; background: var(--bg-panel); border: 1px solid var(--border); border-radius: var(--radius-md); }
	.db-label, .feature-label { font-size: 13px; color: var(--text-muted); }
	.db-value { font-size: 13px; font-family: var(--font-mono); font-weight: 600; color: var(--text-primary); }

	/* Broadcast */
	.broadcast-section { max-width: 600px; }
	.broadcast-form { background: var(--bg-panel); border: 1px solid var(--border); border-radius: var(--radius-lg); padding: 16px; display: flex; flex-direction: column; gap: 12px; margin-bottom: 20px; }
	.form-row { display: flex; flex-direction: column; gap: 4px; }
	.form-row label { font-size: 11px; color: var(--text-dim); font-weight: 600; text-transform: uppercase; }
	.broadcast-input { padding: 8px 10px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-primary); font-size: 13px; font-family: inherit; resize: vertical; min-height: 60px; }
	.target-row { display: flex; gap: 8px; align-items: center; }
	.target-select { flex: 1; padding: 6px 10px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-primary); font-size: 13px; font-family: inherit; }
	.btn-primary { padding: 6px 16px; background: var(--blue); color: white; border: none; border-radius: var(--radius-md); font-size: 13px; cursor: pointer; }
	.btn-primary:disabled { opacity: 0.5; cursor: default; }
	.history-section { margin-top: 16px; }
	.broadcast-history { display: flex; flex-direction: column; gap: 6px; }
	.history-item { padding: 8px 12px; background: var(--bg-panel); border: 1px solid var(--border); border-radius: var(--radius-md); }
	.history-meta { display: flex; justify-content: space-between; margin-bottom: 4px; }
	.history-actor { font-size: 12px; font-weight: 600; color: var(--text-muted); }
	.history-time { font-size: 11px; color: var(--text-dim); font-family: var(--font-mono); }
	.history-details { font-size: 13px; color: var(--text-secondary); }

	/* Resets */
	.reset-actions { display: flex; align-items: center; gap: 8px; }
	.temp-password { display: flex; align-items: center; gap: 6px; }
	.temp-password code { font-size: 12px; font-family: var(--font-mono); padding: 2px 6px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--green); }
	.btn-sm { padding: 3px 10px; font-size: 11px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-muted); cursor: pointer; }
	.btn-primary-sm { background: var(--blue); color: white; border-color: var(--blue); }

	/* Preferences */
	.prefs-form { max-width: 500px; display: flex; flex-direction: column; gap: 20px; }
	.pref-group { background: var(--bg-panel); border: 1px solid var(--border); border-radius: var(--radius-lg); padding: 16px; }
	.pref-title { font-size: 14px; font-weight: 600; margin-bottom: 12px; color: var(--text-primary); }
	.pref-row { display: flex; align-items: center; justify-content: space-between; padding: 8px 0; }
	.pref-row + .pref-row { border-top: 1px solid var(--border); }
	.pref-label { font-size: 13px; color: var(--text-secondary); }
	.toggle { position: relative; display: inline-block; cursor: pointer; }
	.toggle input { position: absolute; opacity: 0; width: 0; height: 0; }
	.toggle-track { display: block; width: 36px; height: 20px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: 10px; transition: background 0.2s; }
	.toggle input:checked + .toggle-track { background: var(--blue); border-color: var(--blue); }
	.toggle-thumb { display: block; width: 16px; height: 16px; background: white; border-radius: 50%; margin: 1px; transition: transform 0.2s; }
	.toggle input:checked + .toggle-track .toggle-thumb { transform: translateX(16px); }
	.interval-options { display: flex; gap: 4px; }
	.interval-btn { padding: 4px 12px; font-size: 12px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-muted); cursor: pointer; }
	.interval-btn:hover { background: var(--bg-hover); }
	.interval-btn.active { background: var(--blue); color: white; border-color: var(--blue); }
	@media (max-width: 768px) {
		.tabs {
			overflow-x: auto;
			flex-wrap: nowrap;
			scrollbar-width: none;
		}
		.tabs::-webkit-scrollbar { display: none; }
		.broadcast-section {
			max-width: 100%;
		}
		.target-row {
			flex-wrap: wrap;
		}
		.prefs-form {
			max-width: 100%;
		}
		.env-row {
			flex-wrap: wrap;
			gap: 4px;
		}
		.interval-options {
			flex-wrap: wrap;
		}
	}
</style>
