<script lang="ts">
	import { playerCorp, formatMoney } from '$lib/stores/gameState';
	import { showConfirm } from '$lib/stores/uiState';
	import * as bridge from '$lib/wasm/bridge';
	import { tr } from '$lib/i18n/index';

	function hire() {
		const corp = $playerCorp;
		if (!corp) return;
		bridge.processCommand({ HireEmployee: { corporation: corp.id, role: 'technician' } });
	}

	function fire() {
		const corp = $playerCorp;
		if (!corp) return;
		if ((corp.employee_count ?? 0) <= 1) return;
		const corpId = corp.id;
		showConfirm('Fire an employee? This will reduce your workforce by 1.', () => {
			bridge.processCommand({ FireEmployee: { entity: corpId } });
		});
	}

	let moralePercent = $derived(Math.round(($playerCorp?.morale ?? 0) * 100));
	let moraleClass = $derived(moralePercent >= 70 ? 'green' : moralePercent >= 40 ? 'amber' : 'red');
	let maintenanceSpeed = $derived(Math.min(100, ($playerCorp?.employee_count ?? 0) * 5));
	let researchSpeed = $derived(Math.min(100, ($playerCorp?.employee_count ?? 0) * 4));
	let constructionSpeed = $derived(Math.min(100, ($playerCorp?.employee_count ?? 0) * 3));
</script>

<div class="panel" role="region" aria-label={$tr('panels.workforce')}>
	<div class="section">
		<h3>{$tr('panels.overview')}</h3>
		<div class="stat-grid">
			<div class="stat">
				<span class="label">{$tr('panels.employees')}</span>
				<span class="value mono">{$playerCorp?.employee_count ?? 0}</span>
			</div>
			<div class="stat">
				<span class="label">{$tr('panels.morale')}</span>
				<span class="value {moraleClass}">{moralePercent}%</span>
			</div>
		</div>
	</div>

	{#if ($playerCorp?.employee_count ?? 0) === 0}
		<div class="section">
			<p class="empty-hint">No employees yet — hire your first team to boost operations!</p>
		</div>
	{/if}

	<div class="section">
		<h3>{$tr('panels.staffing')}</h3>
		<div class="action-row">
			<button class="action-btn hire" onclick={hire}>{$tr('panels.hire')}</button>
			<button class="action-btn fire" onclick={fire} disabled={($playerCorp?.employee_count ?? 0) <= 1}>{$tr('panels.fire')}</button>
		</div>
		<p class="hint">{$tr('panels.staffing_hint')}</p>
	</div>

	<div class="section">
		<h3>{$tr('panels.workforce_impact')}</h3>
		<div class="impact-grid">
			<div class="impact-item">
				<span class="impact-label">{$tr('panels.maintenance_speed')}</span>
				<div class="bar-bg" role="progressbar" aria-valuenow={maintenanceSpeed} aria-valuemin={0} aria-valuemax={100}>
					<div class="bar-fill green-bar" style="width: {maintenanceSpeed}%"></div>
				</div>
			</div>
			<div class="impact-item">
				<span class="impact-label">{$tr('panels.research_speed')}</span>
				<div class="bar-bg" role="progressbar" aria-valuenow={researchSpeed} aria-valuemin={0} aria-valuemax={100}>
					<div class="bar-fill blue-bar" style="width: {researchSpeed}%"></div>
				</div>
			</div>
			<div class="impact-item">
				<span class="impact-label">{$tr('panels.construction_speed')}</span>
				<div class="bar-bg" role="progressbar" aria-valuenow={constructionSpeed} aria-valuemin={0} aria-valuemax={100}>
					<div class="bar-fill amber-bar" style="width: {constructionSpeed}%"></div>
				</div>
			</div>
		</div>
	</div>

	<div class="section">
		<h3>{$tr('panels.morale_factors')}</h3>
		<div class="factor-list">
			<div class="factor">
				<span>{$tr('panels.base_morale')}</span>
				<span class="green">+50%</span>
			</div>
			<div class="factor">
				<span>{$tr('panels.company_profitability')}</span>
				<span class={($playerCorp?.profit_per_tick ?? 0) >= 0 ? 'green' : 'red'}>
					{($playerCorp?.profit_per_tick ?? 0) >= 0 ? '+10%' : '-15%'}
				</span>
			</div>
			<div class="factor">
				<span>{$tr('panels.workload')}</span>
				<span class={($playerCorp?.employee_count ?? 0) > ($playerCorp?.infrastructure_count ?? 0) * 2 ? 'green' : 'amber'}>
					{($playerCorp?.employee_count ?? 0) > ($playerCorp?.infrastructure_count ?? 0) * 2 ? '+10%' : '-5%'}
				</span>
			</div>
		</div>
	</div>
</div>

<style>
	.panel {
		padding: 0;
		color: var(--text-secondary);
		font-family: var(--font-sans);
		font-size: 13px;
	}

	.section {
		padding: 12px 16px;
		border-bottom: 1px solid var(--border);
	}

	h3 {
		font-size: 12px;
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin-bottom: 8px;
	}

	.stat-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 6px;
	}

	.stat {
		display: flex;
		justify-content: space-between;
		padding: 4px 0;
	}

	.label { color: var(--text-muted); }
	.value { color: var(--text-primary); }
	.mono { font-family: var(--font-mono); }
	.green { color: var(--green); }
	.red { color: var(--red); }
	.amber { color: var(--amber); }

	.action-row {
		display: flex;
		gap: 8px;
		margin-bottom: 8px;
	}

	.action-btn {
		flex: 1;
		padding: 8px 12px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 13px;
		font-family: var(--font-mono);
		font-weight: 600;
	}

	.action-btn.hire {
		background: var(--green-bg);
		border: 1px solid var(--green-border);
		color: var(--green);
	}

	.action-btn.hire:hover {
		background: rgba(16, 185, 129, 0.2);
	}

	.action-btn.fire {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--red);
	}

	.action-btn.fire:hover {
		background: rgba(239, 68, 68, 0.1);
	}

	.action-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.hint {
		font-size: 11px;
		color: var(--text-dim);
		margin: 0;
	}

	.impact-grid {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.impact-item {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.impact-label {
		font-size: 12px;
		color: var(--text-muted);
	}

	.bar-bg {
		height: 6px;
		background: var(--bg-surface);
		border-radius: 3px;
		overflow: hidden;
	}

	.bar-fill {
		height: 100%;
		border-radius: 3px;
		transition: width 0.3s;
	}

	.green-bar { background: var(--green); }
	.blue-bar { background: var(--blue); }
	.amber-bar { background: var(--amber); }

	.factor-list {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.factor {
		display: flex;
		justify-content: space-between;
		padding: 4px 0;
		font-size: 12px;
		color: var(--text-muted);
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
	}

	.empty-hint {
		color: var(--text-dim);
		font-size: 13px;
		text-align: center;
		margin: 0;
	}
</style>
