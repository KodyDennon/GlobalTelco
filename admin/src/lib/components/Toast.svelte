<script lang="ts" module>
	export type ToastType = 'success' | 'error' | 'warning' | 'info';

	interface ToastItem {
		id: number;
		type: ToastType;
		message: string;
	}

	let toasts = $state<ToastItem[]>([]);
	let nextId = 0;

	export function toast(message: string, type: ToastType = 'info') {
		const id = nextId++;
		toasts.push({ id, type, message });
		setTimeout(() => dismissToast(id), 5000);
	}

	export function dismissToast(id: number) {
		toasts = toasts.filter((t) => t.id !== id);
	}
</script>

<script lang="ts">
	const typeColors: Record<ToastType, string> = {
		success: 'var(--green)',
		error: 'var(--red)',
		warning: 'var(--amber)',
		info: 'var(--blue)'
	};

	const typeIcons: Record<ToastType, string> = {
		success: '\u2713',
		error: '\u2717',
		warning: '\u26A0',
		info: '\u2139'
	};
</script>

{#if toasts.length > 0}
	<div class="toast-container">
		{#each toasts as t (t.id)}
			<div class="toast" style="border-left-color: {typeColors[t.type]}">
				<span class="toast-icon" style="color: {typeColors[t.type]}">{typeIcons[t.type]}</span>
				<span class="toast-msg">{t.message}</span>
				<button class="toast-close" onclick={() => dismissToast(t.id)}>&times;</button>
			</div>
		{/each}
	</div>
{/if}

<style>
	.toast-container {
		position: fixed;
		bottom: 16px;
		right: 16px;
		z-index: 9999;
		display: flex;
		flex-direction: column;
		gap: 8px;
		max-width: 400px;
	}
	.toast {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 14px;
		background: var(--bg-panel);
		border: 1px solid var(--border);
		border-left: 3px solid;
		border-radius: var(--radius-md);
		font-size: 13px;
		color: var(--text-primary);
		animation: slideIn 0.2s ease-out;
	}
	.toast-icon {
		font-size: 16px;
		flex-shrink: 0;
	}
	.toast-msg {
		flex: 1;
	}
	.toast-close {
		background: none;
		border: none;
		color: var(--text-dim);
		cursor: pointer;
		font-size: 18px;
		line-height: 1;
		padding: 0 2px;
	}
	.toast-close:hover {
		color: var(--text-primary);
	}
	@keyframes slideIn {
		from {
			transform: translateX(100%);
			opacity: 0;
		}
		to {
			transform: translateX(0);
			opacity: 1;
		}
	}
</style>
