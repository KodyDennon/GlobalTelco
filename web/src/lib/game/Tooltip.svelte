<script lang="ts">
	import { tooltipData } from '$lib/stores/uiState';

	// Smart positioning: avoid tooltip going off-screen
	let adjustedX = $derived($tooltipData ? Math.min($tooltipData.x + 12, window.innerWidth - 270) : 0);
	let adjustedY = $derived($tooltipData ? Math.min($tooltipData.y + 12, window.innerHeight - 80) : 0);
</script>

{#if $tooltipData}
	<div
		class="tooltip"
		style="left: {adjustedX}px; top: {adjustedY}px;"
	>
		{$tooltipData.content}
	</div>
{/if}

<style>
	.tooltip {
		position: fixed;
		background: rgba(17, 24, 39, 0.97);
		border: 1px solid rgba(55, 65, 81, 0.6);
		border-radius: 6px;
		padding: 8px 12px;
		font-size: 12px;
		font-family: var(--font-sans);
		color: #d1d5db;
		z-index: 100;
		pointer-events: none;
		white-space: pre-line;
		max-width: 280px;
		line-height: 1.5;
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.5);
		animation: tooltip-fade 0.15s ease-out;
	}

	@keyframes tooltip-fade {
		from { opacity: 0; transform: translateY(2px); }
		to { opacity: 1; transform: translateY(0); }
	}
</style>
