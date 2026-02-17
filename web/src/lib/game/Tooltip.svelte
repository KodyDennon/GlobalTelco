<script lang="ts">
	import { tooltipData } from '$lib/stores/uiState';
</script>

{#if $tooltipData}
	<div
		class="tooltip"
		style="left: {$tooltipData.x + 12}px; top: {$tooltipData.y + 12}px;"
	>
		{$tooltipData.content}
	</div>
{/if}

<style>
	.tooltip {
		position: fixed;
		background: rgba(17, 24, 39, 0.95);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		padding: 6px 10px;
		font-size: 11px;
		font-family: var(--font-sans);
		color: var(--text-secondary);
		z-index: 100;
		pointer-events: none;
		white-space: pre-line;
		max-width: 250px;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
	}
</style>
