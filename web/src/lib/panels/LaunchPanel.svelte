<script lang="ts">
	import { playerCorp, formatMoney } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import type { LaunchPadInfo } from '$lib/wasm/bridge';

	let launchPads: LaunchPadInfo[] = $state([]);

	$effect(() => {
		const corp = $playerCorp;
		if (corp) {
			launchPads = bridge.getLaunchSchedule(corp.id);
		}
	});

	let totalQueued = $derived(
		launchPads.reduce((s, lp) => s + lp.queue.reduce((qs, q) => qs + q.satellite_count, 0), 0)
	);

	const rocketSpecs: Record<string, { payload: number; cost: number; reliability: string }> = {
		Small: { payload: 5, cost: 15_000_000, reliability: '90%' },
		Medium: { payload: 15, cost: 40_000_000, reliability: '93%' },
		Heavy: { payload: 30, cost: 100_000_000, reliability: '95%' },
		SuperHeavy: { payload: 60, cost: 250_000_000, reliability: '97%' },
	};
</script>

<div class="panel">
	<div class="section">
		<h3>Launch Overview</h3>
		<div class="stat-row">
			<span class="muted">Launch pads</span>
			<span class="mono">{launchPads.length}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Queued launches</span>
			<span class="mono blue">{totalQueued} sats</span>
		</div>
	</div>

	<div class="section">
		<h3>Launch Pads</h3>
		{#each launchPads as lp}
			<div class="pad-card">
				<div class="pad-header">
					<span class="pad-name">Pad #{lp.launch_pad_id}</span>
					{#if lp.reusable}
						<span class="reusable-badge">Reusable</span>
					{/if}
				</div>
				<div class="pad-stats">
					{#if lp.cooldown_remaining > 0}
						<span class="cooldown"><span class="muted">Cooldown</span> <span class="mono red">{lp.cooldown_remaining} ticks</span></span>
					{:else}
						<span class="ready"><span class="mono green">Ready</span></span>
					{/if}
				</div>
				{#if lp.queue.length > 0}
					<div class="queue-list">
						{#each lp.queue as launch, i}
							<div class="queue-item">
								<span class="muted">#{i + 1}</span>
								<span class="mono">{launch.rocket_type}</span>
								<span class="mono">{launch.satellite_count} sats</span>
							</div>
						{/each}
					</div>
				{:else}
					<div class="queue-empty">No launches queued</div>
				{/if}
			</div>
		{:else}
			<div class="empty">No launch pads built. Construct a Launch Pad to begin orbital operations.</div>
		{/each}
	</div>

	<div class="section">
		<h3>Rocket Catalog</h3>
		<div class="rocket-grid">
			{#each Object.entries(rocketSpecs) as [name, spec]}
				<div class="rocket-card">
					<div class="rocket-name">{name}</div>
					<div class="rocket-stats">
						<span><span class="muted">Payload</span> <span class="mono">{spec.payload} sats</span></span>
						<span><span class="muted">Cost</span> <span class="mono">{formatMoney(spec.cost)}</span></span>
						<span><span class="muted">Success</span> <span class="mono green">{spec.reliability}</span></span>
					</div>
				</div>
			{/each}
		</div>
	</div>
</div>

<style>
	.panel { color: var(--text-secondary); font-family: var(--font-sans); font-size: 13px; }
	.section { padding: 12px 16px; border-bottom: 1px solid var(--border); }
	h3 { font-size: 12px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 8px; }
	.stat-row { display: flex; justify-content: space-between; padding: 3px 0; }
	.muted { color: var(--text-muted); }
	.mono { font-family: var(--font-mono); }
	.green { color: var(--green); }
	.blue { color: var(--blue); }
	.red { color: var(--red); }

	.pad-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 10px; margin-bottom: 8px; }
	.pad-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px; }
	.pad-name { font-weight: 600; color: var(--text-primary); }
	.reusable-badge { background: rgba(16, 185, 129, 0.1); color: var(--green); font-size: 10px; padding: 2px 6px; border-radius: 3px; }
	.pad-stats { margin-bottom: 6px; }
	.queue-list { border-top: 1px solid var(--border); padding-top: 6px; }
	.queue-item { display: flex; gap: 12px; padding: 2px 0; font-size: 11px; }
	.queue-empty { font-size: 11px; color: var(--text-dim); }

	.rocket-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; }
	.rocket-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 8px; }
	.rocket-name { font-weight: 600; color: var(--text-primary); margin-bottom: 4px; font-size: 12px; }
	.rocket-stats { display: flex; flex-direction: column; gap: 2px; font-size: 11px; }
	.empty { color: var(--text-dim); text-align: center; padding: 16px; font-size: 12px; }
</style>
