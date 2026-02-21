<script lang="ts">
	interface Props {
		onclose: () => void;
	}

	let { onclose }: Props = $props();

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose();
	}
</script>

<svelte:window onkeydown={handleKeyDown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onclose} onkeydown={handleKeyDown} role="presentation">
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="guide" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog" aria-label="Network Tier Guide" tabindex="-1">
		<div class="header">
			<span class="title">Network Tier Guide</span>
			<button class="close-btn" onclick={onclose}>&#x2715;</button>
		</div>
		<div class="body">
			<section class="section">
				<h4>Node Tiers</h4>
				<div class="tier-row"><span class="tn">T1</span><span class="tl">Access</span><span class="td">Cell Tower, Wireless Relay</span></div>
				<div class="tier-row"><span class="tn">T2</span><span class="tl">Aggregation</span><span class="td">Central Office, Exchange Point</span></div>
				<div class="tier-row"><span class="tn">T3</span><span class="tl">Core</span><span class="td">Data Center</span></div>
				<div class="tier-row"><span class="tn">T4</span><span class="tl">Backbone</span><span class="td">Backbone Router</span></div>
				<div class="tier-row"><span class="tn">T5</span><span class="tl">Global</span><span class="td">Satellite Ground, Submarine Landing</span></div>
			</section>
			<section class="section">
				<h4>Edge Connections &amp; Range</h4>
				<div class="edge-row">
					<span class="el">Copper</span>
					<span class="ec">Access ↔ Access, Access ↔ Agg</span>
					<span class="er">Short</span>
				</div>
				<div class="edge-row">
					<span class="el">Fiber Local</span>
					<span class="ec">Access ↔ Agg, Agg ↔ Agg</span>
					<span class="er">Medium</span>
				</div>
				<div class="edge-row">
					<span class="el">Microwave</span>
					<span class="ec">Access ↔ Agg, Agg ↔ Core</span>
					<span class="er">Long</span>
				</div>
				<div class="edge-row">
					<span class="el">Fiber Regional</span>
					<span class="ec">Agg ↔ Core, Core ↔ Core</span>
					<span class="er">Very Long</span>
				</div>
				<div class="edge-row">
					<span class="el">Fiber National</span>
					<span class="ec">Core ↔ BB, BB ↔ BB</span>
					<span class="er">Massive</span>
				</div>
				<div class="edge-row">
					<span class="el">Satellite</span>
					<span class="ec">Core ↔ Global, BB ↔ Global</span>
					<span class="er highlight">Unlimited</span>
				</div>
				<div class="edge-row">
					<span class="el">Submarine</span>
					<span class="ec">Global ↔ Global</span>
					<span class="er">Extreme</span>
				</div>
			</section>
			<section class="section">
				<h4>Distance Limits</h4>
				<p class="hint">Each edge type has a <strong>maximum range</strong> based on map size. If "Connection distance too long" appears, you need a longer-range edge type or intermediate relay nodes.</p>
				<p class="hint" style="margin-top: 6px;">
					<strong>Tip:</strong> To connect across cities, use <span class="tip-edge">Microwave</span> (longer range than Fiber Local, same tier pairs for Access ↔ Agg). For cross-region links, build <span class="tip-edge">Fiber Regional</span> between Aggregation and Core nodes.
				</p>
			</section>
			<section class="section">
				<h4>Build Order</h4>
				<p class="hint">Build bottom-up: Access nodes (Cell Towers) → connect with Fiber Local/Microwave to Aggregation (Central Offices) → Fiber Regional to Core (Data Centers) → Fiber National to Backbone → Satellite/Submarine to Global.</p>
			</section>
			<section class="section">
				<h4>Rules</h4>
				<p class="hint">Edges can only connect nodes within the tier pairs shown above. Nodes connect to adjacent tiers (1 step apart). One node can have many edges — a Central Office can link to multiple Cell Towers.</p>
			</section>
		</div>
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.4);
		z-index: 50;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.guide {
		background: rgba(17, 24, 39, 0.98);
		border: 1px solid rgba(55, 65, 81, 0.6);
		border-radius: 10px;
		width: 520px;
		max-height: 80vh;
		overflow-y: auto;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
	}

	.header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 16px;
		border-bottom: 1px solid rgba(55, 65, 81, 0.4);
	}

	.title {
		font-family: var(--font-sans, system-ui, sans-serif);
		font-size: 14px;
		font-weight: 700;
		color: #d1d5db;
	}

	.close-btn {
		background: transparent;
		border: none;
		color: #6b7280;
		font-size: 16px;
		cursor: pointer;
		padding: 4px 8px;
		border-radius: 4px;
	}

	.close-btn:hover {
		background: rgba(239, 68, 68, 0.2);
		color: #ef4444;
	}

	.body {
		padding: 16px;
	}

	.section {
		margin-bottom: 16px;
	}

	.section:last-child {
		margin-bottom: 0;
	}

	.section h4 {
		font-family: var(--font-sans, system-ui, sans-serif);
		font-size: 11px;
		font-weight: 700;
		color: #6b7280;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin: 0 0 8px;
	}

	.tier-row, .edge-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 5px 0;
		font-size: 12px;
		border-bottom: 1px solid rgba(55, 65, 81, 0.15);
	}

	.tn {
		color: #60a5fa;
		font-weight: 700;
		font-family: var(--font-mono, monospace);
		min-width: 24px;
	}

	.tl {
		color: #9ca3af;
		font-weight: 600;
		min-width: 90px;
	}

	.td {
		color: #d1d5db;
	}

	.el {
		color: #10b981;
		font-weight: 600;
		min-width: 110px;
		flex-shrink: 0;
	}

	.ec {
		color: #d1d5db;
		flex: 1;
	}

	.er {
		color: #f59e0b;
		font-weight: 600;
		font-size: 10px;
		text-transform: uppercase;
		letter-spacing: 0.3px;
		min-width: 65px;
		text-align: right;
		flex-shrink: 0;
	}

	.er.highlight {
		color: #60a5fa;
	}

	.hint {
		font-family: var(--font-sans, system-ui, sans-serif);
		font-size: 12px;
		color: #9ca3af;
		line-height: 1.6;
		margin: 0;
	}

	.hint strong {
		color: #d1d5db;
	}

	.tip-edge {
		color: #10b981;
		font-weight: 600;
	}
</style>
