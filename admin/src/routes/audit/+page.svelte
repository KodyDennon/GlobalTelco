<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import Badge from '$lib/components/Badge.svelte';
	import { fetchAuditLog } from '$lib/api/audit.js';
	import { startPolling, stopPolling } from '$lib/stores/polling.js';
	import type { AuditEntry } from '$lib/api/types.js';

	let entries = $state<AuditEntry[]>([]);
	let total = $state(0);
	let loading = $state(true);
	let currentPage = $state(0);
	let pageSize = 50;
	let actorFilter = $state('');
	let actionFilter = $state('all');
	let dateFrom = $state('');
	let dateTo = $state('');
	let autoRefresh = $state(true);

	const columns = [
		{ key: 'created_at', label: 'Time', sortable: true, format: (v: unknown) => v ? new Date(v as string).toLocaleString() : '--', width: '180px' },
		{ key: 'actor', label: 'Actor', sortable: true },
		{ key: 'action', label: 'Action', sortable: true },
		{ key: 'target', label: 'Target', sortable: true, format: (v: unknown) => v ? String(v).slice(0, 16) + (String(v).length > 16 ? '...' : '') : '--' },
		{ key: 'details', label: 'Details', format: (v: unknown) => v ? JSON.stringify(v).slice(0, 50) : '--' },
		{ key: 'ip_address', label: 'IP', format: (v: unknown) => (v ? String(v) : '--') },
	];

	const actionTypes = ['all', 'kick_player', 'ban_player', 'unban_player', 'create_world', 'delete_world', 'set_speed', 'broadcast', 'create_template', 'update_template', 'delete_template', 'resolve_reset'];

	async function loadData() {
		try {
			const action = actionFilter !== 'all' ? actionFilter : undefined;
			const from = dateFrom || undefined;
			const to = dateTo || undefined;
			const r = await fetchAuditLog(pageSize, currentPage * pageSize, actorFilter || undefined, action, from, to);
			entries = r.audit_log;
			total = r.total;
			loading = false;
		} catch { if (loading) loading = false; }
	}

	function exportData(format: 'json' | 'csv') {
		let content: string;
		let mime: string;
		let ext: string;

		if (format === 'json') {
			content = JSON.stringify(entries, null, 2);
			mime = 'application/json';
			ext = 'json';
		} else {
			const headers = ['Time', 'Actor', 'Action', 'Target', 'Details', 'IP'];
			const rows = entries.map(e => [
				e.created_at,
				e.actor,
				e.action,
				e.target || '',
				e.details ? JSON.stringify(e.details) : '',
				e.ip_address || ''
			]);
			content = [headers.join(','), ...rows.map(r => r.map(c => `"${String(c).replace(/"/g, '""')}"`).join(','))].join('\n');
			mime = 'text/csv';
			ext = 'csv';
		}

		const blob = new Blob([content], { type: mime });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `audit_log_${new Date().toISOString().slice(0, 10)}.${ext}`;
		a.click();
		URL.revokeObjectURL(url);
	}

	onMount(() => {
		startPolling('audit', loadData, 10000);
	});
	onDestroy(() => stopPolling('audit'));

	$effect(() => {
		// Re-load when filter or page changes
		actorFilter;
		actionFilter;
		dateFrom;
		dateTo;
		currentPage;
		loadData();
	});
</script>

<div class="page">
	<div class="page-header">
		<h1 class="page-title">Audit Log</h1>
		<div class="header-actions">
			<label class="auto-toggle">
				<input type="checkbox" bind:checked={autoRefresh} onchange={() => {
					if (autoRefresh) startPolling('audit', loadData, 10000);
					else stopPolling('audit');
				}} />
				Auto-refresh
			</label>
			<button class="btn-sm" onclick={() => exportData('csv')}>Export CSV</button>
			<button class="btn-sm" onclick={() => exportData('json')}>Export JSON</button>
		</div>
	</div>

	<div class="filters">
		<div class="filter-group">
			<label for="filter-actor">Actor Filter</label>
			<input id="filter-actor" type="text" bind:value={actorFilter} placeholder="Filter by actor..." class="filter-input" />
		</div>
		<div class="filter-group">
			<label for="filter-action">Action Type</label>
			<select id="filter-action" bind:value={actionFilter} class="filter-input filter-select">
				{#each actionTypes as t}
					<option value={t}>{t === 'all' ? 'All Actions' : t.replace(/_/g, ' ')}</option>
				{/each}
			</select>
		</div>
		<div class="filter-group">
			<label for="filter-from">From</label>
			<input id="filter-from" type="date" bind:value={dateFrom} class="filter-input filter-date" />
		</div>
		<div class="filter-group">
			<label for="filter-to">To</label>
			<input id="filter-to" type="date" bind:value={dateTo} class="filter-input filter-date" />
		</div>
		<span class="total-count">{total} total entries</span>
	</div>

	<DataTable
		{columns}
		data={entries as unknown as Record<string, unknown>[]}
		{loading}
		emptyMessage="No audit log entries"
	/>

	{#if total > pageSize}
		<div class="pagination">
			<button disabled={currentPage === 0} onclick={() => currentPage--}>Prev</button>
			<span>Page {currentPage + 1} of {Math.ceil(total / pageSize)}</span>
			<button disabled={(currentPage + 1) * pageSize >= total} onclick={() => currentPage++}>Next</button>
		</div>
	{/if}
</div>

<style>
	.page { max-width: 1200px; }
	.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
	.page-title { font-size: 20px; font-weight: 700; }
	.header-actions { display: flex; gap: 8px; align-items: center; }
	.auto-toggle { display: flex; align-items: center; gap: 4px; font-size: 12px; color: var(--text-dim); cursor: pointer; }
	.btn-sm { padding: 4px 12px; font-size: 11px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-muted); cursor: pointer; }
	.btn-sm:hover { background: var(--bg-hover); color: var(--text-primary); }
	.filters { display: flex; align-items: center; gap: 16px; margin-bottom: 16px; }
	.filter-group { display: flex; flex-direction: column; gap: 3px; }
	.filter-group label { font-size: 10px; color: var(--text-dim); text-transform: uppercase; font-weight: 600; }
	.filter-input { padding: 5px 10px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-primary); font-size: 13px; width: 200px; font-family: inherit; }
	.filter-select { width: 180px; cursor: pointer; appearance: auto; }
	.filter-date { width: 150px; }
	.filter-date::-webkit-calendar-picker-indicator { filter: invert(0.7); cursor: pointer; }
	.total-count { font-size: 12px; color: var(--text-dim); margin-left: auto; }
	.pagination { display: flex; align-items: center; justify-content: center; gap: 10px; margin-top: 12px; }
	.pagination button { padding: 4px 12px; font-size: 12px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); color: var(--text-secondary); cursor: pointer; }
	.pagination button:disabled { opacity: 0.4; cursor: default; }
	.pagination span { font-size: 12px; color: var(--text-dim); }
	@media (max-width: 768px) {
		.header-actions {
			flex-wrap: wrap;
		}
		.filters {
			flex-direction: column;
			align-items: stretch;
		}
		.filter-input,
		.filter-select,
		.filter-date {
			width: 100% !important;
		}
		.total-count {
			margin-left: 0;
		}
	}
</style>
