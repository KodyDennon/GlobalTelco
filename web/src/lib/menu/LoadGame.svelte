<script lang="ts">
	import { listSaves, loadFromSlot, deleteSave, type SaveMetadata } from '$lib/saves/SaveManager';

	let { onLoad, onBack }: { onLoad: (data: string) => void; onBack: () => void } = $props();

	let saves: SaveMetadata[] = $state([]);
	let loading = $state(true);
	let loadingSlot: string | null = $state(null);
	let deleteConfirm: string | null = $state(null);
	let error: string | null = $state(null);

	$effect(() => {
		refreshSaves();
	});

	async function refreshSaves() {
		loading = true;
		error = null;
		try {
			saves = await listSaves();
		} catch (e) {
			error = 'Failed to load saves';
		}
		loading = false;
	}

	async function handleLoad(slot: string) {
		loadingSlot = slot;
		error = null;
		try {
			const result = await loadFromSlot(slot);
			if (result) {
				onLoad(result.data);
			} else {
				error = 'Save not found';
			}
		} catch (e: any) {
			error = e.message || 'Failed to load save';
		}
		loadingSlot = null;
	}

	async function handleDelete(slot: string) {
		if (deleteConfirm !== slot) {
			deleteConfirm = slot;
			return;
		}
		await deleteSave(slot);
		deleteConfirm = null;
		await refreshSaves();
	}

	function formatDate(timestamp: number): string {
		const d = new Date(timestamp);
		return d.toLocaleDateString(undefined, {
			month: 'short',
			day: 'numeric',
			year: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function formatTick(tick: number): string {
		if (tick >= 1000) return `${(tick / 1000).toFixed(1)}K`;
		return `${tick}`;
	}
</script>

<div class="load-game">
	<div class="container">
		<h2>Load Game</h2>

		{#if error}
			<div class="error">{error}</div>
		{/if}

		{#if loading}
			<div class="empty">Loading saves...</div>
		{:else if saves.length === 0}
			<div class="empty">No saved games found</div>
		{:else}
			<div class="save-list">
				{#each saves as save}
					<div class="save-slot" class:auto={save.slot.startsWith('AutoSave')} class:quick={save.slot === 'QuickSave'}>
						<div class="save-info">
							<div class="save-header">
								<span class="save-name">{save.name}</span>
								<span class="save-slot-label">{save.slot}</span>
							</div>
							<div class="save-details">
								<span>{save.corpName}</span>
								<span class="sep">|</span>
								<span>{save.era}</span>
								<span class="sep">|</span>
								<span>Tick {formatTick(save.tick)}</span>
								<span class="sep">|</span>
								<span>{save.difficulty}</span>
							</div>
							<div class="save-date">{formatDate(save.timestamp)}</div>
						</div>
						<div class="save-actions">
							<button
								class="btn load"
								onclick={() => handleLoad(save.slot)}
								disabled={loadingSlot !== null}
							>
								{loadingSlot === save.slot ? 'Loading...' : 'Load'}
							</button>
							<button
								class="btn delete"
								onclick={() => handleDelete(save.slot)}
							>
								{deleteConfirm === save.slot ? 'Confirm?' : 'Del'}
							</button>
						</div>
					</div>
				{/each}
			</div>
		{/if}

		<button class="btn back" onclick={onBack}>Back</button>
	</div>
</div>

<style>
	.load-game {
		width: 100vw;
		height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #0a0e17;
		font-family: system-ui, sans-serif;
		color: #f3f4f6;
	}

	.container {
		width: 500px;
		max-height: 80vh;
		background: rgba(17, 24, 39, 0.95);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 12px;
		padding: 32px;
		display: flex;
		flex-direction: column;
	}

	h2 {
		margin: 0 0 20px;
		font-size: 24px;
		font-weight: 700;
	}

	.error {
		background: rgba(239, 68, 68, 0.15);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: #f87171;
		padding: 8px 12px;
		border-radius: 6px;
		font-size: 13px;
		margin-bottom: 12px;
	}

	.empty {
		color: #6b7280;
		text-align: center;
		padding: 40px 0;
		font-size: 14px;
	}

	.save-list {
		flex: 1;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin-bottom: 16px;
	}

	.save-slot {
		display: flex;
		justify-content: space-between;
		align-items: center;
		background: rgba(31, 41, 55, 0.6);
		border: 1px solid rgba(55, 65, 81, 0.4);
		border-radius: 8px;
		padding: 12px 16px;
		transition: border-color 0.15s;
	}

	.save-slot:hover {
		border-color: rgba(75, 85, 101, 0.6);
	}

	.save-slot.auto {
		border-left: 3px solid rgba(245, 158, 11, 0.5);
	}

	.save-slot.quick {
		border-left: 3px solid rgba(59, 130, 246, 0.5);
	}

	.save-info {
		flex: 1;
		min-width: 0;
	}

	.save-header {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 4px;
	}

	.save-name {
		font-weight: 600;
		font-size: 14px;
		color: #f3f4f6;
	}

	.save-slot-label {
		font-size: 10px;
		padding: 1px 6px;
		background: rgba(55, 65, 81, 0.5);
		border-radius: 3px;
		color: #9ca3af;
	}

	.save-details {
		font-size: 12px;
		color: #9ca3af;
		display: flex;
		gap: 4px;
	}

	.sep {
		color: #4b5563;
	}

	.save-date {
		font-size: 11px;
		color: #6b7280;
		margin-top: 2px;
	}

	.save-actions {
		display: flex;
		gap: 6px;
		margin-left: 12px;
	}

	.btn {
		padding: 6px 14px;
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		font-size: 13px;
		font-family: system-ui, sans-serif;
		cursor: pointer;
		transition: all 0.15s;
	}

	.btn.load {
		background: rgba(16, 185, 129, 0.15);
		border-color: rgba(16, 185, 129, 0.3);
		color: #10b981;
	}

	.btn.load:hover:not(:disabled) {
		background: rgba(16, 185, 129, 0.25);
	}

	.btn.load:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn.delete {
		background: transparent;
		color: #6b7280;
	}

	.btn.delete:hover {
		color: #ef4444;
		border-color: rgba(239, 68, 68, 0.3);
	}

	.btn.back {
		width: 100%;
		background: rgba(31, 41, 55, 0.8);
		color: #d1d5db;
		padding: 10px;
	}

	.btn.back:hover {
		background: rgba(55, 65, 81, 0.8);
	}
</style>
