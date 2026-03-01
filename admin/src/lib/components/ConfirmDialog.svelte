<script lang="ts" module>
	let resolvePromise: ((value: boolean) => void) | null = null;
	let dialogState = $state<{
		open: boolean;
		title: string;
		message: string;
		confirmLabel: string;
		cancelLabel: string;
		variant: 'danger' | 'warning' | 'info';
	}>({
		open: false,
		title: '',
		message: '',
		confirmLabel: 'Confirm',
		cancelLabel: 'Cancel',
		variant: 'info'
	});

	export function confirm(
		title: string,
		message: string,
		opts?: { confirmLabel?: string; cancelLabel?: string; variant?: 'danger' | 'warning' | 'info' }
	): Promise<boolean> {
		dialogState = {
			open: true,
			title,
			message,
			confirmLabel: opts?.confirmLabel ?? 'Confirm',
			cancelLabel: opts?.cancelLabel ?? 'Cancel',
			variant: opts?.variant ?? 'info'
		};
		return new Promise((resolve) => {
			resolvePromise = resolve;
		});
	}

	function handleConfirm() {
		dialogState.open = false;
		resolvePromise?.(true);
		resolvePromise = null;
	}

	function handleCancel() {
		dialogState.open = false;
		resolvePromise?.(false);
		resolvePromise = null;
	}
</script>

<script lang="ts">
	const variantColors: Record<string, string> = {
		danger: 'var(--red)',
		warning: 'var(--amber)',
		info: 'var(--blue)'
	};

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') handleCancel();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if dialogState.open}
	<div class="overlay" onclick={handleCancel} role="presentation">
		<div class="dialog" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
			<h3 class="dialog-title">{dialogState.title}</h3>
			<p class="dialog-message">{dialogState.message}</p>
			<div class="dialog-actions">
				<button class="btn btn-cancel" onclick={handleCancel}>{dialogState.cancelLabel}</button>
				<button
					class="btn btn-confirm"
					style="background: {variantColors[dialogState.variant]}"
					onclick={handleConfirm}
				>
					{dialogState.confirmLabel}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 9998;
	}
	.dialog {
		background: var(--bg-panel);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		padding: 24px;
		max-width: 440px;
		width: 90%;
	}
	.dialog-title {
		font-size: 16px;
		font-weight: 600;
		margin-bottom: 8px;
	}
	.dialog-message {
		font-size: 13px;
		color: var(--text-secondary);
		margin-bottom: 20px;
		line-height: 1.5;
	}
	.dialog-actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
	.btn {
		padding: 6px 16px;
		border-radius: var(--radius-md);
		font-size: 13px;
		font-weight: 500;
		cursor: pointer;
		border: 1px solid var(--border);
		transition: opacity 0.15s;
	}
	.btn:hover {
		opacity: 0.85;
	}
	.btn-cancel {
		background: var(--bg-surface);
		color: var(--text-secondary);
	}
	.btn-confirm {
		color: white;
		border: none;
	}
</style>
