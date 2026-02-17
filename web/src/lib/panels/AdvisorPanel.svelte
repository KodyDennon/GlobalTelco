<script lang="ts">
	import { playerCorp, formatMoney, regions, cities } from '$lib/stores/gameState';
	import { activePanel } from '$lib/stores/uiState';
	import * as bridge from '$lib/wasm/bridge';
	import { tr, t } from '$lib/i18n/index';

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
		const translate = $tr;

		// Financial health
		if (corp.cash < 0) {
			s.push({ priority: 'critical', title: translate('advisor.negative_cash_title'), detail: translate('advisor.negative_cash_detail') });
		} else if (corp.cash < 1_000_000) {
			s.push({ priority: 'warning', title: translate('advisor.low_cash_title'), detail: translate('advisor.low_cash_detail') });
		}

		if (corp.profit_per_tick < 0) {
			s.push({ priority: 'warning', title: translate('advisor.operating_loss_title'), detail: translate('advisor.operating_loss_detail', { amount: formatMoney(Math.abs(corp.profit_per_tick)) }) });
		}

		// Infrastructure
		if (corp.infrastructure_count === 0) {
			s.push({ priority: 'critical', title: translate('advisor.no_infra_title'), detail: translate('advisor.no_infra_detail') });
		} else if (corp.infrastructure_count < 5) {
			s.push({ priority: 'info', title: translate('advisor.expand_title'), detail: translate('advisor.expand_detail') });
		}

		// Check damaged nodes
		const damaged = bridge.getDamagedNodes(corp.id);
		if (damaged.length > 0) {
			s.push({ priority: 'warning', title: translate('advisor.damaged_title', { count: damaged.length }), detail: translate('advisor.damaged_detail') });
		}

		// Check unmet demand
		const regs = $regions;
		const highDemandRegions = regs.filter((r) => r.population > 100000);
		if (highDemandRegions.length > 0 && corp.infrastructure_count < highDemandRegions.length * 2) {
			s.push({ priority: 'info', title: translate('advisor.demand_title'), detail: translate('advisor.demand_detail') });
		}

		// Research
		const research = bridge.getResearchState();
		const activeResearch = research.find((r) => r.researcher === corp.id && !r.completed);
		if (!activeResearch) {
			s.push({ priority: 'info', title: translate('advisor.no_research_title'), detail: translate('advisor.no_research_detail') });
		}

		// Credit rating
		if (corp.credit_rating === 'CCC' || corp.credit_rating === 'D') {
			s.push({ priority: 'critical', title: translate('advisor.poor_credit_title'), detail: translate('advisor.poor_credit_detail') });
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

<div class="panel" role="region" aria-label={$tr('panels.advisor')}>
	<div class="panel-header">
		<span class="title">{$tr('panels.advisor')}</span>
		<button class="close" onclick={close}>x</button>
	</div>

	{#if suggestions.length === 0}
		<div class="section">
			<div class="all-good">{$tr('panels.all_good')}</div>
		</div>
	{:else}
		<div class="section">
			<h3>{$tr('panels.suggestions', { count: suggestions.length })}</h3>
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
