<script lang="ts">
	interface Props {
		icon?: string;
		message: string;
		actionLabel?: string;
		onaction?: () => void;
	}
	let { icon = '--', message, actionLabel, onaction }: Props = $props();
</script>

<div class="empty">
	<div class="empty-icon">{icon}</div>
	<p class="empty-msg">{message}</p>
	{#if actionLabel && onaction}
		<button class="empty-btn" onclick={onaction}>{actionLabel}</button>
	{/if}
</div>

<style>
	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 40px 20px;
		gap: 8px;
	}
	.empty-icon {
		font-size: 32px;
		color: var(--text-faint);
	}
	.empty-msg {
		font-size: 13px;
		color: var(--text-dim);
		text-align: center;
	}
	.empty-btn {
		margin-top: 8px;
		padding: 6px 16px;
		background: var(--blue);
		color: white;
		border: none;
		border-radius: var(--radius-md);
		font-size: 13px;
		cursor: pointer;
	}
	.empty-btn:hover {
		opacity: 0.85;
	}
</style>
