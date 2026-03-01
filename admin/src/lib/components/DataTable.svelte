<script lang="ts">
	import SearchInput from './SearchInput.svelte';
	import EmptyState from './EmptyState.svelte';
	import LoadingSkeleton from './LoadingSkeleton.svelte';

	interface Column {
		key: string;
		label: string;
		sortable?: boolean;
		format?: (value: unknown, row: Record<string, unknown>) => string;
		align?: 'left' | 'right' | 'center';
		width?: string;
	}

	interface Props {
		columns: Column[];
		data: Record<string, unknown>[];
		loading?: boolean;
		searchable?: boolean;
		searchPlaceholder?: string;
		paginated?: boolean;
		pageSize?: number;
		emptyMessage?: string;
		onrowclick?: (row: Record<string, unknown>) => void;
		actions?: import('svelte').Snippet<[Record<string, unknown>]>;
	}
	let {
		columns,
		data,
		loading = false,
		searchable = false,
		searchPlaceholder = 'Search...',
		paginated = false,
		pageSize = 20,
		emptyMessage = 'No data',
		onrowclick,
		actions
	}: Props = $props();

	let search = $state('');
	let sortKey = $state('');
	let sortDir = $state<'asc' | 'desc'>('asc');
	let page = $state(0);

	const filtered = $derived.by(() => {
		let result = data;
		if (search) {
			const q = search.toLowerCase();
			result = result.filter((row) =>
				columns.some((col) => {
					const val = row[col.key];
					return val != null && String(val).toLowerCase().includes(q);
				})
			);
		}
		if (sortKey) {
			result = [...result].sort((a, b) => {
				const av = a[sortKey] ?? '';
				const bv = b[sortKey] ?? '';
				const cmp = String(av).localeCompare(String(bv), undefined, { numeric: true });
				return sortDir === 'asc' ? cmp : -cmp;
			});
		}
		return result;
	});

	const totalPages = $derived(paginated ? Math.max(1, Math.ceil(filtered.length / pageSize)) : 1);
	const paged = $derived(
		paginated ? filtered.slice(page * pageSize, (page + 1) * pageSize) : filtered
	);

	function toggleSort(key: string) {
		if (sortKey === key) {
			sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		} else {
			sortKey = key;
			sortDir = 'asc';
		}
	}

	function cellValue(row: Record<string, unknown>, col: Column): string {
		const val = row[col.key];
		if (col.format) return col.format(val, row);
		if (val == null) return '--';
		return String(val);
	}
</script>

<div class="datatable">
	{#if searchable}
		<div class="datatable-search">
			<SearchInput placeholder={searchPlaceholder} onchange={(v) => { search = v; page = 0; }} />
			<span class="datatable-count">{filtered.length} result{filtered.length !== 1 ? 's' : ''}</span>
		</div>
	{/if}

	{#if loading}
		<LoadingSkeleton rows={5} height={20} />
	{:else if paged.length === 0}
		<EmptyState message={emptyMessage} />
	{:else}
		<div class="datatable-scroll">
			<table>
				<thead>
					<tr>
						{#each columns as col}
							<th
								class:sortable={col.sortable}
								style:text-align={col.align ?? 'left'}
								style:width={col.width}
								onclick={() => col.sortable && toggleSort(col.key)}
							>
								{col.label}
								{#if col.sortable && sortKey === col.key}
									<span class="sort-arrow">{sortDir === 'asc' ? '\u25B2' : '\u25BC'}</span>
								{/if}
							</th>
						{/each}
						{#if actions}
							<th style="width: 100px; text-align: right">Actions</th>
						{/if}
					</tr>
				</thead>
				<tbody>
					{#each paged as row}
						<tr
							class:clickable={!!onrowclick}
							onclick={() => onrowclick?.(row)}
						>
							{#each columns as col}
								<td style:text-align={col.align ?? 'left'}>{cellValue(row, col)}</td>
							{/each}
							{#if actions}
								<td style="text-align: right" onclick={(e) => e.stopPropagation()}>
									{@render actions(row)}
								</td>
							{/if}
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}

	{#if paginated && totalPages > 1}
		<div class="datatable-pagination">
			<button disabled={page === 0} onclick={() => (page = 0)}>First</button>
			<button disabled={page === 0} onclick={() => page--}>Prev</button>
			<span class="page-info">Page {page + 1} of {totalPages}</span>
			<button disabled={page >= totalPages - 1} onclick={() => page++}>Next</button>
			<button disabled={page >= totalPages - 1} onclick={() => (page = totalPages - 1)}>Last</button>
		</div>
	{/if}
</div>

<style>
	.datatable {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.datatable-search {
		display: flex;
		align-items: center;
		gap: 12px;
	}
	.datatable-count {
		font-size: 12px;
		color: var(--text-dim);
		white-space: nowrap;
	}
	.datatable-scroll {
		overflow-x: auto;
	}
	table {
		width: 100%;
		border-collapse: collapse;
	}
	th {
		padding: 8px 12px;
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--text-muted);
		border-bottom: 1px solid var(--border);
		white-space: nowrap;
		user-select: none;
	}
	th.sortable {
		cursor: pointer;
	}
	th.sortable:hover {
		color: var(--text-primary);
	}
	.sort-arrow {
		font-size: 9px;
		margin-left: 4px;
	}
	td {
		padding: 8px 12px;
		font-size: 13px;
		color: var(--text-secondary);
		border-bottom: 1px solid var(--border);
		white-space: nowrap;
	}
	tr.clickable {
		cursor: pointer;
	}
	tr.clickable:hover td {
		background: var(--bg-surface);
	}
	.datatable-pagination {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		padding: 8px 0;
	}
	.datatable-pagination button {
		padding: 4px 10px;
		font-size: 12px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		color: var(--text-secondary);
		cursor: pointer;
	}
	.datatable-pagination button:disabled {
		opacity: 0.4;
		cursor: default;
	}
	.datatable-pagination button:not(:disabled):hover {
		background: var(--bg-hover);
	}
	.page-info {
		font-size: 12px;
		color: var(--text-dim);
		padding: 0 8px;
	}
</style>
