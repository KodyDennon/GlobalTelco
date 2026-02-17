<script lang="ts">
	import { playerCorp, formatMoney, regions, cities } from '$lib/stores/gameState';
	import { activePanel } from '$lib/stores/uiState';
	import * as bridge from '$lib/wasm/bridge';

	interface Suggestion {
		priority: 'critical' | 'warning' | 'info';
		title: string;
		detail: string;
	}

	let suggestions: Suggestion[] = $state([]);

	$effect(() => {
		const corp = $playerCorp;
		if (!corp) return;

		const s: Suggestion[] = [];

		// Financial health
		if (corp.cash < 0) {
			s.push({ priority: 'critical', title: 'Negative Cash', detail: 'Take a loan or reduce expenses immediately.' });
		} else if (corp.cash < 1_000_000) {
			s.push({ priority: 'warning', title: 'Low Cash Reserves', detail: 'Consider reducing spending or taking a small loan.' });
		}

		if (corp.profit_per_tick < 0) {
			s.push({ priority: 'warning', title: 'Operating at a Loss', detail: `Losing ${formatMoney(Math.abs(corp.profit_per_tick))}/tick. Build revenue-generating infrastructure.` });
		}

		// Infrastructure
		if (corp.infrastructure_count === 0) {
			s.push({ priority: 'critical', title: 'No Infrastructure', detail: 'Build your first cell tower to start generating revenue.' });
		} else if (corp.infrastructure_count < 5) {
			s.push({ priority: 'info', title: 'Expand Network', detail: 'Build more nodes and connect them to increase coverage and revenue.' });
		}

		// Check damaged nodes
		const damaged = bridge.getDamagedNodes(corp.id);
		if (damaged.length > 0) {
			s.push({ priority: 'warning', title: `${damaged.length} Damaged Node(s)`, detail: 'Repair damaged infrastructure to restore capacity.' });
		}

		// Check unmet demand
		const regs = $regions;
		const highDemandRegions = regs.filter((r) => r.population > 100000);
		if (highDemandRegions.length > 0 && corp.infrastructure_count < highDemandRegions.length * 2) {
			s.push({ priority: 'info', title: 'Unmet Market Demand', detail: 'High-population regions need more infrastructure coverage.' });
		}

		// Research
		const research = bridge.getResearchState();
		const activeResearch = research.find((r) => r.researcher === corp.id && !r.completed);
		if (!activeResearch) {
			s.push({ priority: 'info', title: 'No Active Research', detail: 'Start researching to gain competitive advantages.' });
		}

		// Credit rating
		if (corp.credit_rating === 'CCC' || corp.credit_rating === 'D') {
			s.push({ priority: 'critical', title: 'Poor Credit Rating', detail: 'Reduce debt and increase revenue to improve your rating.' });
		}

		suggestions = s;
	});

	function close() {
		activePanel.set('none');
	}

	function priorityColor(p: string): string {
		switch (p) {
			case 'critical': return 'var(--red)';
			case 'warning': return 'var(--amber)';
			default: return 'var(--blue)';
		}
	}
</script>

<div class="panel">
	<div class="panel-header">
		<span class="title">Advisor</span>
		<button class="close" onclick={close}>x</button>
	</div>

	{#if suggestions.length === 0}
		<div class="section">
			<div class="all-good">Everything looks good! Keep expanding your network.</div>
		</div>
	{:else}
		<div class="section">
			<h3>Suggestions ({suggestions.length})</h3>
			{#each suggestions as sug}
				<div class="suggestion">
					<div class="sug-header">
						<span class="dot" style="background: {priorityColor(sug.priority)}"></span>
						<span class="sug-title">{sug.title}</span>
						<span class="sug-priority" style="color: {priorityColor(sug.priority)}">{sug.priority}</span>
					</div>
					<div class="sug-detail">{sug.detail}</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.panel {
		color: var(--text-secondary);
		font-family: var(--font-sans);
		font-size: 13px;
	}

	.panel-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 12px 16px;
		border-bottom: 1px solid var(--border);
		position: sticky;
		top: 0;
		background: var(--bg-panel);
		z-index: 1;
	}

	.title { font-weight: 700; font-size: 14px; color: var(--text-primary); }
	.close { background: none; border: none; color: var(--text-dim); cursor: pointer; font-size: 16px; }

	.section { padding: 12px 16px; }

	h3 {
		font-size: 12px;
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin-bottom: 8px;
	}

	.all-good {
		color: var(--green);
		text-align: center;
		padding: 24px 0;
		font-size: 14px;
	}

	.suggestion {
		padding: 10px;
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
	}

	.sug-header {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 4px;
	}

	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.sug-title {
		font-weight: 600;
		color: var(--text-primary);
		flex: 1;
	}

	.sug-priority {
		font-size: 10px;
		font-weight: 600;
		text-transform: uppercase;
	}

	.sug-detail {
		font-size: 12px;
		color: var(--text-muted);
		padding-left: 16px;
	}
</style>
