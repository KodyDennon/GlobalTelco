<script lang="ts">
	import { tr } from '$lib/i18n/index';
	import { worldInfo } from '$lib/stores/gameState';
	import { simTickTime } from '$lib/game/GameLoop';

	let fps = $state(0);
	let frameTime = $state(0);
	let memory = $state('');
	let drawCalls = $state(0);
	let triangles = $state(0);
	let frames: number[] = [];
	let lastTime = performance.now();
	let rafId: number;

	let { rendererInfo }: { rendererInfo?: { calls: number; triangles: number } } = $props();

	function tick() {
		const now = performance.now();
		const delta = now - lastTime;
		lastTime = now;

		frames.push(delta);
		if (frames.length > 60) frames.shift();

		const avg = frames.reduce((a, b) => a + b, 0) / frames.length;
		fps = Math.round(1000 / avg);
		frameTime = Math.round(avg * 10) / 10;

		if ((performance as any).memory) {
			const mb = (performance as any).memory.usedJSHeapSize / 1048576;
			memory = `${Math.round(mb)}MB`;
		}

		if (rendererInfo) {
			drawCalls = rendererInfo.calls;
			triangles = rendererInfo.triangles;
		}

		rafId = requestAnimationFrame(tick);
	}

	import { onMount, onDestroy } from 'svelte';
	onMount(() => { rafId = requestAnimationFrame(tick); });
	onDestroy(() => { cancelAnimationFrame(rafId); });
</script>

<div class="perf-monitor" role="status" aria-label={$tr('perf.title')}>
	<div class="row"><span class="label">{$tr('perf.fps')}</span><span class="value">{fps}</span></div>
	<div class="row"><span class="label">{$tr('perf.frame_time')}</span><span class="value">{frameTime}ms</span></div>
	<div class="row"><span class="label">{$tr('perf.sim_tick')}</span><span class="value">{$simTickTime}ms</span></div>
	<div class="row"><span class="label">{$tr('perf.tick_rate')}</span><span class="value">{$worldInfo.tick}</span></div>
	<div class="row"><span class="label">{$tr('perf.entities')}</span><span class="value">{$worldInfo.entity_count ?? '—'}</span></div>
	{#if drawCalls > 0}
		<div class="row"><span class="label">{$tr('perf.draw_calls')}</span><span class="value">{drawCalls}</span></div>
		<div class="row"><span class="label">{$tr('perf.triangles')}</span><span class="value">{triangles}</span></div>
	{/if}
	{#if memory}
		<div class="row"><span class="label">{$tr('perf.memory')}</span><span class="value">{memory}</span></div>
	{/if}
</div>

<style>
	.perf-monitor {
		position: absolute;
		top: 56px;
		right: 8px;
		background: rgba(0, 0, 0, 0.75);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		padding: 8px 12px;
		font-family: var(--font-mono);
		font-size: 11px;
		color: #9ca3af;
		z-index: 50;
		min-width: 140px;
		pointer-events: none;
	}

	.row {
		display: flex;
		justify-content: space-between;
		gap: 12px;
		line-height: 1.6;
	}

	.label {
		color: #6b7280;
	}

	.value {
		color: #10b981;
		font-weight: 600;
	}
</style>
