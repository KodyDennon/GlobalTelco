<script lang="ts">
	interface Props {
		rows?: number;
		height?: number;
	}
	let { rows = 3, height = 16 }: Props = $props();
</script>

<div class="skeleton-wrap">
	{#each Array(rows) as _, i}
		<div
			class="skeleton-line"
			style="height: {height}px; width: {80 + Math.sin(i * 1.5) * 20}%"
		></div>
	{/each}
</div>

<style>
	.skeleton-wrap {
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 12px 0;
	}
	.skeleton-line {
		background: linear-gradient(
			90deg,
			var(--bg-surface) 25%,
			var(--bg-hover) 50%,
			var(--bg-surface) 75%
		);
		background-size: 200% 100%;
		border-radius: var(--radius-sm);
		animation: shimmer 1.5s infinite;
	}
	@keyframes shimmer {
		0% {
			background-position: 200% 0;
		}
		100% {
			background-position: -200% 0;
		}
	}
</style>
