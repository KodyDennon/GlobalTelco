<script lang="ts">
	import Sparkline from './Sparkline.svelte';

	interface Props {
		label: string;
		value: string | number;
		unit?: string;
		sparklineData?: number[];
		color?: string;
	}
	let { label, value, unit = '', sparklineData, color = 'var(--blue)' }: Props = $props();
</script>

<div class="stat-card">
	<div class="stat-label">{label}</div>
	<div class="stat-value" style="color: {color}">
		{typeof value === 'number' ? value.toLocaleString() : value}
		{#if unit}<span class="stat-unit">{unit}</span>{/if}
	</div>
	{#if sparklineData && sparklineData.length > 1}
		<div class="stat-sparkline">
			<Sparkline data={sparklineData} {color} width={100} height={24} />
		</div>
	{/if}
</div>

<style>
	.stat-card {
		background: var(--bg-panel);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		padding: 14px 18px;
		display: flex;
		flex-direction: column;
		gap: 4px;
		min-width: 140px;
	}
	.stat-label {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--text-muted);
	}
	.stat-value {
		font-size: 24px;
		font-weight: 700;
		font-family: var(--font-mono);
		line-height: 1.2;
	}
	.stat-unit {
		font-size: 13px;
		font-weight: 400;
		color: var(--text-dim);
	}
	.stat-sparkline {
		margin-top: 4px;
	}
</style>
