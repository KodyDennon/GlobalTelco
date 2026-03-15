<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import DataTable from '$lib/components/DataTable.svelte';
	import Badge from '$lib/components/Badge.svelte';
	import ConfigEditor from '$lib/components/ConfigEditor.svelte';
	import WorldConfigForm from '$lib/components/WorldConfigForm.svelte';
	import { toast } from '$lib/components/Toast.svelte';
	import { confirm } from '$lib/components/ConfirmDialog.svelte';
	import { fetchWorlds, createWorld, deleteWorld, purgeWorlds, setWorldSpeed, pauseWorld, fetchTemplates, createTemplate, updateTemplate, deleteTemplate, fetchServerLimits, setServerLimits } from '$lib/api/worlds.js';
	import { startPolling, stopPolling } from '$lib/stores/polling.js';
	import type { WorldConfig, WorldTemplate } from '$lib/api/types.js';
	import type { ServerLimits } from '$lib/api/worlds.js';

	let worlds = $state<Record<string, unknown>[]>([]);
	let templates = $state<WorldTemplate[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	// Server limits
	let limits = $state<ServerLimits | null>(null);
	let limitsMaxWorlds = $state(10);
	let limitsMaxPerPlayer = $state(2);
	let savingLimits = $state(false);

	// Create world form
	let showCreate = $state(false);
	let newName = $state('');
	let newMaxPlayers = $state(8);
	let newConfig = $state<WorldConfig>({});
	let creating = $state(false);

	// Templates
	let showTemplates = $state(false);
	let showTemplateForm = $state(false);
	let editingTemplate = $state<WorldTemplate | null>(null);
	let tplName = $state('');
	let tplDescription = $state('');
	let tplIcon = $state('');
	let tplMaxInstances = $state(5);
	let tplEnabled = $state(true);
	let tplSortOrder = $state(0);
	let tplConfigDefaults = $state<Record<string, unknown>>({});
	let tplConfigBounds = $state<Record<string, unknown>>({});

	const columns = [
		{ key: 'name', label: 'Name', sortable: true },
		{ key: 'player_count', label: 'Players', sortable: true, format: (v: unknown, row: Record<string, unknown>) => `${v}/${row.max_players}` },
		{ key: 'tick', label: 'Tick', sortable: true, format: (v: unknown) => Number(v).toLocaleString(), align: 'right' as const },
		{ key: 'era', label: 'Era', sortable: true },
		{ key: 'map_size', label: 'Map Size', sortable: true },
		{ key: 'speed', label: 'Speed', sortable: true },
	];

	const speeds = ['Paused', 'Normal', 'Fast', 'VeryFast', 'Ultra'];

	async function loadData() {
		try {
			const [w, t, l] = await Promise.all([
				fetchWorlds(),
				fetchTemplates().catch(() => []),
				fetchServerLimits().catch(() => null),
			]);
			worlds = w as unknown as Record<string, unknown>[];
			templates = t;
			if (l) {
				limits = l;
				limitsMaxWorlds = l.max_active_worlds;
				limitsMaxPerPlayer = l.max_worlds_per_player;
			}
			error = null;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load worlds';
		} finally {
			loading = false;
		}
	}

	async function handleCreate() {
		if (!newName.trim()) { toast('Name required', 'warning'); return; }
		creating = true;
		try {
			await createWorld(newName, newConfig, newMaxPlayers);
			toast(`World "${newName}" created`, 'success');
			newName = '';
			newConfig = {};
			showCreate = false;
			await loadData();
		} catch (e) {
			toast(`Failed: ${e}`, 'error');
		} finally {
			creating = false;
		}
	}

	async function handleDelete(worldId: string, name: string) {
		const ok = await confirm('Delete World', `Delete "${name}"? All players will be kicked.`, { variant: 'danger', confirmLabel: 'Delete' });
		if (!ok) return;
		try {
			await deleteWorld(worldId);
			toast(`World "${name}" deleted`, 'success');
			await loadData();
		} catch (e) {
			toast(`Failed: ${e}`, 'error');
		}
	}

	async function handlePurge() {
		const ok = await confirm('Purge Archived Worlds', 'Permanently delete all archived and deleted worlds from the database? This action cannot be undone.', { variant: 'danger', confirmLabel: 'Purge All' });
		if (!ok) return;
		try {
			const res = await purgeWorlds();
			toast(`Successfully purged ${res.count} world(s)`, 'success');
			await loadData();
		} catch (e) {
			toast(`Failed: ${e}`, 'error');
		}
	}

	async function handleSpeed(worldId: string, speed: string) {
		try {
			await setWorldSpeed(worldId, speed);
			toast(`Speed set to ${speed}`, 'success');
			await loadData();
		} catch (e) {
			toast(`Failed: ${e}`, 'error');
		}
	}

	async function handlePause(worldId: string) {
		try {
			const res = await pauseWorld(worldId);
			toast(res.paused ? 'World paused' : 'World resumed', 'success');
			await loadData();
		} catch (e) {
			toast(`Failed: ${e}`, 'error');
		}
	}

	async function handleSaveLimits() {
		savingLimits = true;
		try {
			await setServerLimits({ max_active_worlds: limitsMaxWorlds, max_worlds_per_player: limitsMaxPerPlayer });
			toast('Server limits updated', 'success');
			await loadData();
		} catch (e) {
			toast(`Failed: ${e}`, 'error');
		} finally {
			savingLimits = false;
		}
	}

	async function handleSaveTemplate() {
		if (!tplName.trim()) { toast('Template name required', 'warning'); return; }
		try {
			const payload = {
				name: tplName, description: tplDescription, icon: tplIcon,
				config_defaults: tplConfigDefaults, config_bounds: tplConfigBounds,
				max_instances: tplMaxInstances, enabled: tplEnabled, sort_order: tplSortOrder,
			};
			if (editingTemplate) {
				await updateTemplate(editingTemplate.id, payload);
				toast('Template updated', 'success');
			} else {
				await createTemplate(payload);
				toast('Template created', 'success');
			}
			resetTemplateForm();
			await loadData();
		} catch (e) {
			toast(`Failed: ${e}`, 'error');
		}
	}

	function editTemplate(t: WorldTemplate) {
		editingTemplate = t;
		tplName = t.name; tplDescription = t.description; tplIcon = t.icon;
		tplMaxInstances = t.max_instances; tplEnabled = t.enabled; tplSortOrder = t.sort_order;
		tplConfigDefaults = { ...t.config_defaults }; tplConfigBounds = { ...t.config_bounds };
		showTemplateForm = true;
	}

	function resetTemplateForm() {
		editingTemplate = null; tplName = ''; tplDescription = ''; tplIcon = '';
		tplMaxInstances = 5; tplEnabled = true; tplSortOrder = 0;
		tplConfigDefaults = {}; tplConfigBounds = {};
		showTemplateForm = false;
	}

	async function handleDeleteTemplate(id: string, name: string) {
		const ok = await confirm('Delete Template', `Delete "${name}"?`, { variant: 'danger' });
		if (!ok) return;
		try {
			await deleteTemplate(id);
			toast('Template deleted', 'success');
			await loadData();
		} catch (e) {
			toast(`Failed: ${e}`, 'error');
		}
	}

	onMount(() => startPolling('worlds', loadData));
	onDestroy(() => stopPolling('worlds'));
</script>

<div class="page">
	<div class="page-header">
		<h1 class="page-title">Worlds</h1>
		<div class="header-actions">
			<button class="btn-sm btn-danger" onclick={handlePurge} style="margin-right: 8px">
				Purge Archived
			</button>
			<button class="btn-primary" onclick={() => (showCreate = !showCreate)}>
				{showCreate ? 'Cancel' : '+ Create World'}
			</button>
		</div>
	</div>

	{#if error && worlds.length === 0 && !loading}
		<div class="error-inline">
			<span>{error}</span>
			<button onclick={loadData}>Retry</button>
		</div>
	{/if}

	<!-- Server Limits -->
	{#if limits}
		<div class="limits-section">
			<div class="limits-header">
				<h2 class="section-title">Server Limits</h2>
				<span class="limits-usage">{limits.active_world_count} / {limits.max_active_worlds} active worlds</span>
			</div>
			<div class="limits-bar">
				<div class="limits-fill" style="width: {Math.min(100, (limits.active_world_count / limits.max_active_worlds) * 100)}%"></div>
			</div>
			<div class="limits-grid">
				<div class="form-field">
					<label for="lim-max-w">Max Active Worlds</label>
					<input id="lim-max-w" type="number" bind:value={limitsMaxWorlds} min="1" max="100" />
				</div>
				<div class="form-field">
					<label for="lim-max-p">Max Worlds Per Player</label>
					<input id="lim-max-p" type="number" bind:value={limitsMaxPerPlayer} min="1" max="50" />
				</div>
				<div class="form-field" style="align-self: end;">
					<button class="btn-primary" onclick={handleSaveLimits} disabled={savingLimits}>
						{savingLimits ? 'Saving...' : 'Save Limits'}
					</button>
				</div>
			</div>
		</div>
	{/if}

	{#if showCreate}
		<div class="create-form">
			<div class="form-grid">
				<div class="form-field">
					<label for="new-w-name">Name</label>
					<input id="new-w-name" type="text" bind:value={newName} placeholder="World name" />
				</div>
				<div class="form-field">
					<label for="new-w-max">Max Players</label>
					<input id="new-w-max" type="number" bind:value={newMaxPlayers} min="1" max="100" />
				</div>
			</div>
			<WorldConfigForm bind:value={newConfig} onchange={(c) => (newConfig = c)} />
			<button class="btn-primary" onclick={handleCreate} disabled={creating}>
				{creating ? 'Creating...' : 'Create World'}
			</button>
		</div>
	{/if}

	<DataTable
		{columns}
		data={worlds}
		{loading}
		searchable
		searchPlaceholder="Search worlds..."
		emptyMessage="No worlds running"
		onrowclick={(row) => goto(`/worlds/${row.id}`)}
	>
		{#snippet actions(row)}
			<div class="row-actions">
				<select
					value={row.speed as string}
					onchange={(e) => handleSpeed(row.id as string, (e.target as HTMLSelectElement).value)}
				>
					{#each speeds as s}
						<option value={s}>{s}</option>
					{/each}
				</select>
				<button class="btn-sm btn-danger" onclick={() => handleDelete(row.id as string, row.name as string)}>Delete</button>
			</div>
		{/snippet}
	</DataTable>

	<!-- Templates Section -->
	<div class="section" style="margin-top: 32px">
		<div class="section-header">
			<h2 class="section-title">World Templates</h2>
			<div class="section-actions">
				<button class="btn-sm" onclick={() => (showTemplates = !showTemplates)}>{showTemplates ? 'Hide' : 'Show'}</button>
				{#if showTemplates}
					<button class="btn-sm btn-primary" onclick={() => { resetTemplateForm(); showTemplateForm = true; }}>+ Template</button>
				{/if}
			</div>
		</div>

		{#if showTemplates}
			{#if showTemplateForm}
				<div class="template-form">
					<h3>{editingTemplate ? 'Edit Template' : 'New Template'}</h3>
					<div class="form-grid">
						<div class="form-field"><label for="tpl-name">Name</label><input id="tpl-name" type="text" bind:value={tplName} /></div>
						<div class="form-field"><label for="tpl-icon">Icon</label><input id="tpl-icon" type="text" bind:value={tplIcon} placeholder="e.g. tower" /></div>
						<div class="form-field full"><label for="tpl-desc">Description</label><input id="tpl-desc" type="text" bind:value={tplDescription} /></div>
						<div class="form-field"><label for="tpl-max">Max Instances</label><input id="tpl-max" type="number" bind:value={tplMaxInstances} /></div>
						<div class="form-field"><label for="tpl-sort">Sort Order</label><input id="tpl-sort" type="number" bind:value={tplSortOrder} /></div>
						<div class="form-field"><label><input type="checkbox" bind:checked={tplEnabled} /> Enabled</label></div>
					</div>
					<ConfigEditor label="Config Defaults" value={tplConfigDefaults} onchange={(v) => (tplConfigDefaults = v)} />
					<ConfigEditor label="Config Bounds" value={tplConfigBounds} onchange={(v) => (tplConfigBounds = v)} />
					<div class="form-actions">
						<button class="btn-sm" onclick={resetTemplateForm}>Cancel</button>
						<button class="btn-sm btn-primary" onclick={handleSaveTemplate}>Save</button>
					</div>
				</div>
			{/if}
			{#each templates as t}
				<div class="template-row">
					<span class="tpl-name">{t.icon} {t.name}</span>
					<Badge color={t.enabled ? 'green' : 'gray'}>{t.enabled ? 'Active' : 'Disabled'}</Badge>
					<span class="tpl-meta">Max: {t.max_instances}</span>
					<button class="btn-sm" onclick={() => editTemplate(t)}>Edit</button>
					<button class="btn-sm btn-danger" onclick={() => handleDeleteTemplate(t.id, t.name)}>Delete</button>
				</div>
			{/each}
			{#if templates.length === 0}
				<p class="empty-text">No templates</p>
			{/if}
		{/if}
	</div>
</div>

<style>
	.page { max-width: 1200px; }
	.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
	.page-title { font-size: 20px; font-weight: 700; }
	.section { margin-bottom: 24px; }
	.section-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
	.section-title { font-size: 14px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.04em; }
	.section-actions { display: flex; gap: 8px; }

	/* Server Limits */
	.limits-section {
		background: var(--bg-panel); border: 1px solid var(--border); border-radius: var(--radius-lg);
		padding: 16px; margin-bottom: 16px;
	}
	.limits-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; }
	.limits-usage { font-size: 13px; color: var(--text-muted); font-family: monospace; }
	.limits-bar {
		height: 6px; background: var(--bg-surface); border-radius: 3px; margin-bottom: 12px; overflow: hidden;
	}
	.limits-fill {
		height: 100%; background: var(--blue); border-radius: 3px; transition: width 0.3s;
	}
	.limits-grid { display: grid; grid-template-columns: 1fr 1fr auto; gap: 10px; align-items: end; }

	.create-form, .template-form {
		background: var(--bg-panel); border: 1px solid var(--border); border-radius: var(--radius-lg);
		padding: 16px; margin-bottom: 16px; display: flex; flex-direction: column; gap: 12px;
	}
	.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
	.form-field { display: flex; flex-direction: column; gap: 3px; }
	.form-field.full { grid-column: 1 / -1; }
	.form-field label { font-size: 11px; color: var(--text-dim); font-weight: 600; }
	.form-field input {
		padding: 6px 10px; background: var(--bg-surface); border: 1px solid var(--border);
		border-radius: var(--radius-sm); color: var(--text-primary); font-size: 13px; font-family: inherit;
	}
	.form-actions { display: flex; gap: 8px; justify-content: flex-end; }
	.btn-primary {
		padding: 6px 16px; background: var(--blue); color: white; border: none;
		border-radius: var(--radius-md); font-size: 13px; font-weight: 500; cursor: pointer;
	}
	.btn-primary:hover { opacity: 0.9; }
	.btn-primary:disabled { opacity: 0.5; }
	.btn-sm {
		padding: 3px 10px; font-size: 11px; background: var(--bg-surface);
		border: 1px solid var(--border); border-radius: var(--radius-sm);
		color: var(--text-muted); cursor: pointer;
	}
	.btn-sm:hover { background: var(--bg-hover); color: var(--text-primary); }
	.btn-sm.btn-primary { background: var(--blue); color: white; border: none; }
	.btn-sm.btn-danger { color: var(--red); }
	.btn-sm.btn-danger:hover { background: var(--red-bg); }
	.row-actions { display: flex; gap: 6px; align-items: center; }
	.row-actions select {
		padding: 2px 6px; font-size: 11px; background: var(--bg-surface);
		border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-primary);
	}
	.template-row {
		display: flex; align-items: center; gap: 10px; padding: 8px 12px;
		background: var(--bg-panel); border: 1px solid var(--border); border-radius: var(--radius-md);
		margin-bottom: 4px;
	}
	.tpl-name { font-size: 13px; font-weight: 500; flex: 1; }
	.tpl-meta { font-size: 11px; color: var(--text-dim); }
	.empty-text { font-size: 13px; color: var(--text-dim); font-style: italic; }
	h3 { font-size: 14px; font-weight: 600; }
	.error-inline { display: flex; align-items: center; justify-content: space-between; padding: 8px 14px; background: var(--red-bg); border: 1px solid rgba(239, 68, 68, 0.3); border-radius: var(--radius-md); margin-bottom: 12px; font-size: 12px; color: var(--red-light); }
	.error-inline button { padding: 2px 10px; background: rgba(239, 68, 68, 0.2); border: 1px solid rgba(239, 68, 68, 0.3); border-radius: var(--radius-sm); color: var(--red-light); font-size: 11px; cursor: pointer; }
	@media (max-width: 768px) {
		.template-row { flex-wrap: wrap; }
		.page-header { flex-direction: column; align-items: flex-start; gap: 8px; }
		.limits-grid { grid-template-columns: 1fr; }
	}
</style>
