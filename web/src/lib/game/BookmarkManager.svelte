<script lang="ts">
	const STORAGE_KEY = 'globaltelco-bookmarks';

	interface Bookmark {
		name: string;
		lon: number;
		lat: number;
		zoom: number;
	}

	let bookmarks: Bookmark[] = $state(loadBookmarks());
	let newName = $state('');
	let showAdd = $state(false);

	function loadBookmarks(): Bookmark[] {
		try {
			const raw = localStorage.getItem(STORAGE_KEY);
			return raw ? JSON.parse(raw) : [];
		} catch {
			return [];
		}
	}

	function saveBookmarks() {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(bookmarks));
	}

	function addBookmark() {
		const name = newName.trim();
		if (!name) return;

		// Get current camera from map (stored globally by MapRenderer)
		const cam = (window as any).__gtCamera ?? { lon: 0, lat: 30, zoom: 3 };
		bookmarks = [...bookmarks, { name, lon: cam.lon, lat: cam.lat, zoom: cam.zoom }];
		saveBookmarks();
		newName = '';
		showAdd = false;
	}

	function flyTo(bm: Bookmark) {
		window.dispatchEvent(new CustomEvent('map-fly-to', {
			detail: { lon: bm.lon, lat: bm.lat, zoom: bm.zoom }
		}));
	}

	function remove(index: number) {
		bookmarks = bookmarks.filter((_, i) => i !== index);
		saveBookmarks();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') addBookmark();
		if (e.key === 'Escape') { showAdd = false; newName = ''; }
	}
</script>

<div class="bm-panel">
	<div class="bm-header">
		<h3>BOOKMARKS</h3>
		<button class="add-btn" onclick={() => (showAdd = !showAdd)}>
			{showAdd ? 'Cancel' : '+ Add'}
		</button>
	</div>

	{#if showAdd}
		<div class="add-row">
			<input
				type="text"
				bind:value={newName}
				placeholder="Bookmark name..."
				onkeydown={handleKeydown}
				class="bm-input"
			/>
			<button class="save-btn" onclick={addBookmark} disabled={!newName.trim()}>Save</button>
		</div>
	{/if}

	{#if bookmarks.length === 0}
		<div class="bm-empty">No bookmarks yet. Save a camera position to quickly jump back later.</div>
	{:else}
		<div class="bm-list">
			{#each bookmarks as bm, i}
				<div class="bm-item">
					<button class="bm-name" onclick={() => flyTo(bm)} title="Fly to {bm.name}">
						{bm.name}
					</button>
					<span class="bm-coords">{bm.lat.toFixed(1)}, {bm.lon.toFixed(1)} z{bm.zoom.toFixed(0)}</span>
					<button class="bm-del" onclick={() => remove(i)} title="Delete bookmark">&times;</button>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.bm-panel {
		padding: 12px 16px;
		font-family: var(--font-sans);
		font-size: 13px;
		color: var(--text-secondary);
	}

	.bm-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 8px;
	}

	h3 {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-dim);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin: 0;
	}

	.add-btn {
		background: rgba(59, 130, 246, 0.15);
		border: 1px solid rgba(59, 130, 246, 0.3);
		color: var(--blue);
		padding: 2px 10px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 10px;
		font-weight: 600;
	}

	.add-btn:hover {
		background: rgba(59, 130, 246, 0.25);
	}

	.add-row {
		display: flex;
		gap: 6px;
		margin-bottom: 8px;
	}

	.bm-input {
		flex: 1;
		background: rgba(30, 41, 59, 0.8);
		border: 1px solid rgba(255, 255, 255, 0.1);
		border-radius: var(--radius-sm);
		color: var(--text-primary);
		padding: 4px 8px;
		font-size: 12px;
		outline: none;
	}

	.bm-input:focus {
		border-color: rgba(59, 130, 246, 0.5);
	}

	.save-btn {
		background: rgba(16, 185, 129, 0.2);
		border: 1px solid rgba(16, 185, 129, 0.4);
		color: var(--green);
		padding: 4px 10px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 11px;
		font-weight: 600;
	}

	.save-btn:disabled {
		opacity: 0.4;
		cursor: default;
	}

	.bm-empty {
		color: var(--text-dim);
		font-size: 11px;
		text-align: center;
		padding: 16px 0;
	}

	.bm-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.bm-item {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 4px 0;
		border-bottom: 1px solid rgba(55, 65, 81, 0.15);
	}

	.bm-name {
		flex: 1;
		background: none;
		border: none;
		color: var(--text-primary);
		font-size: 12px;
		font-weight: 500;
		cursor: pointer;
		text-align: left;
		padding: 2px 4px;
		border-radius: var(--radius-sm);
	}

	.bm-name:hover {
		background: rgba(59, 130, 246, 0.1);
		color: var(--blue);
	}

	.bm-coords {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--text-dim);
		white-space: nowrap;
	}

	.bm-del {
		background: none;
		border: none;
		color: var(--text-dim);
		cursor: pointer;
		font-size: 16px;
		line-height: 1;
		padding: 0 4px;
	}

	.bm-del:hover {
		color: var(--red);
	}
</style>
