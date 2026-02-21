<script lang="ts">
	import { confirmDialog } from '$lib/stores/uiState';

	function handleConfirm() {
		const state = $confirmDialog;
		if (state.onConfirm) state.onConfirm();
		confirmDialog.set({ visible: false, message: '', onConfirm: null });
	}

	function handleCancel() {
		confirmDialog.set({ visible: false, message: '', onConfirm: null });
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Escape') handleCancel();
		if (e.key === 'Enter') handleConfirm();
	}
</script>

{#if $confirmDialog.visible}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="overlay" onkeydown={handleKeyDown}>
		<div class="dialog" role="alertdialog" aria-modal="true" aria-label="Confirm action">
			<p class="message">{$confirmDialog.message}</p>
			<div class="actions">
				<button class="btn cancel" onclick={handleCancel}>Cancel</button>
				<button class="btn confirm" onclick={handleConfirm}>Confirm</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 100;
	}

	.dialog {
		background: rgba(17, 24, 39, 0.98);
		border: 1px solid rgba(55, 65, 81, 0.6);
		border-radius: 8px;
		padding: 24px;
		max-width: 400px;
		width: 90%;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
	}

	.message {
		font-family: var(--font-sans, system-ui, sans-serif);
		font-size: 14px;
		color: #d1d5db;
		margin: 0 0 20px;
		line-height: 1.5;
	}

	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 10px;
	}

	.btn {
		padding: 8px 20px;
		border-radius: 6px;
		font-family: var(--font-sans, system-ui, sans-serif);
		font-size: 13px;
		font-weight: 500;
		cursor: pointer;
		border: 1px solid transparent;
		transition: all 0.15s;
	}

	.cancel {
		background: rgba(55, 65, 81, 0.4);
		border-color: rgba(55, 65, 81, 0.6);
		color: #9ca3af;
	}

	.cancel:hover {
		background: rgba(55, 65, 81, 0.6);
		color: #d1d5db;
	}

	.confirm {
		background: rgba(239, 68, 68, 0.2);
		border-color: rgba(239, 68, 68, 0.4);
		color: #ef4444;
	}

	.confirm:hover {
		background: rgba(239, 68, 68, 0.3);
	}
</style>
