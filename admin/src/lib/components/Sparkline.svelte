<script lang="ts">
	interface Props {
		data: number[];
		width?: number;
		height?: number;
		color?: string;
		showLabels?: boolean;
	}
	let { data, width = 120, height = 32, color = 'var(--blue)', showLabels = false }: Props = $props();

	let hoverIndex = $state<number | null>(null);

	const points = $derived.by(() => {
		if (data.length < 2) return '';
		const min = Math.min(...data);
		const max = Math.max(...data);
		const range = max - min || 1;
		const stepX = width / (data.length - 1);
		return data.map((v, i) => {
			const x = i * stepX;
			const y = height - ((v - min) / range) * (height - 4) - 2;
			return `${x},${y}`;
		}).join(' ');
	});

	const minVal = $derived(data.length ? Math.min(...data) : 0);
	const maxVal = $derived(data.length ? Math.max(...data) : 0);
	const hoverValue = $derived(hoverIndex !== null ? data[hoverIndex] : null);
	const hoverX = $derived.by(() => {
		if (hoverIndex === null || data.length < 2) return 0;
		return hoverIndex * (width / (data.length - 1));
	});
	const hoverY = $derived.by(() => {
		if (hoverIndex === null || data.length < 2) return 0;
		const min = Math.min(...data);
		const max = Math.max(...data);
		const range = max - min || 1;
		return height - ((data[hoverIndex!] - min) / range) * (height - 4) - 2;
	});
</script>

<div class="sparkline-wrap">
	{#if showLabels}
		<span class="label max">{maxVal.toLocaleString()}</span>
	{/if}
	<svg
		{width}
		{height}
		viewBox="0 0 {width} {height}"
		role="img"
		aria-label="Sparkline chart"
		onmousemove={(e) => {
			const rect = (e.currentTarget as SVGElement).getBoundingClientRect();
			const x = e.clientX - rect.left;
			const idx = Math.round((x / width) * (data.length - 1));
			hoverIndex = Math.max(0, Math.min(data.length - 1, idx));
		}}
		onmouseleave={() => (hoverIndex = null)}
	>
		{#if data.length >= 2}
			<polyline
				fill="none"
				stroke={color}
				stroke-width="1.5"
				stroke-linecap="round"
				stroke-linejoin="round"
				{points}
			/>
			{#if hoverIndex !== null}
				<circle cx={hoverX} cy={hoverY} r="3" fill={color} />
			{/if}
		{/if}
	</svg>
	{#if showLabels}
		<span class="label min">{minVal.toLocaleString()}</span>
	{/if}
	{#if hoverValue !== null}
		<div class="tooltip" style="left: {hoverX}px">{hoverValue.toLocaleString()}</div>
	{/if}
</div>

<style>
	.sparkline-wrap {
		position: relative;
		display: inline-flex;
		flex-direction: column;
		gap: 2px;
	}
	svg {
		display: block;
		cursor: crosshair;
	}
	.label {
		font-size: 9px;
		font-family: var(--font-mono);
		color: var(--text-dim);
	}
	.tooltip {
		position: absolute;
		top: -20px;
		transform: translateX(-50%);
		background: var(--bg-panel);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		padding: 1px 6px;
		font-size: 10px;
		font-family: var(--font-mono);
		color: var(--text-primary);
		white-space: nowrap;
		pointer-events: none;
	}
</style>
