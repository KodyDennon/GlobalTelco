<script lang="ts">
	interface Props {
		title: string;
		gradient: Array<{ color: string; label: string }>;
		visible: boolean;
	}

	let { title, gradient, visible }: Props = $props();
</script>

{#if visible && gradient.length > 0}
	<div class="overlay-legend" role="img" aria-label="Overlay legend: {title}">
		<div class="legend-title">{title}</div>
		<div class="legend-body">
			<div class="gradient-bar">
				{#each gradient as stop}
					<div
						class="gradient-segment"
						style="background: {stop.color}; flex: 1;"
					></div>
				{/each}
			</div>
			<div class="label-column">
				{#each gradient as stop, i}
					<div
						class="label-row"
						style="top: {(i / Math.max(gradient.length - 1, 1)) * 100}%;"
					>
						<span class="label-tick">--</span>
						<span class="label-text">{stop.label}</span>
					</div>
				{/each}
			</div>
		</div>
	</div>
{/if}

<style>
	.overlay-legend {
		position: absolute;
		top: 92px;
		left: 16px;
		z-index: 12;
		background: rgba(17, 24, 39, 0.9);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		padding: 8px 12px;
		min-width: 80px;
		pointer-events: none;
	}

	.legend-title {
		font-family: var(--font-sans, sans-serif);
		font-size: 10px;
		font-weight: 600;
		color: #6b7280;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin-bottom: 6px;
	}

	.legend-body {
		display: flex;
		gap: 6px;
		align-items: stretch;
	}

	.gradient-bar {
		width: 20px;
		height: 120px;
		display: flex;
		flex-direction: column;
		border-radius: 2px;
		overflow: hidden;
		flex-shrink: 0;
	}

	.gradient-segment {
		min-height: 1px;
	}

	.label-column {
		position: relative;
		width: 60px;
		height: 120px;
	}

	.label-row {
		position: absolute;
		left: 0;
		transform: translateY(-50%);
		display: flex;
		align-items: center;
		gap: 2px;
		white-space: nowrap;
	}

	.label-tick {
		font-family: var(--font-mono, monospace);
		font-size: 10px;
		color: #4b5563;
		user-select: none;
	}

	.label-text {
		font-family: var(--font-mono, monospace);
		font-size: 10px;
		color: #9ca3af;
	}
</style>
