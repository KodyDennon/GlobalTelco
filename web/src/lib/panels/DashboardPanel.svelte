<script lang="ts">
	import { playerCorp, formatMoney, allCorporations, worldInfo } from '$lib/stores/gameState';
	import { showConfirm } from '$lib/stores/uiState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import type { DebtInfo } from '$lib/wasm/types';
	import FinanceChart from '$lib/charts/FinanceChart.svelte';
	import MarketShareChart from '$lib/charts/MarketShareChart.svelte';
	import { tr } from '$lib/i18n/index';
	import { tooltip } from '$lib/ui/tooltip';

	let debts: DebtInfo[] = $state([]);
	let showLoanDialog = $state(false);
	let loanAmount = $state(1_000_000);

	// Refresh debts when corp changes or each tick (so new loans/repayments show immediately)
	$effect(() => {
		const _tick = $worldInfo.tick;
		const corp = $playerCorp;
		if (corp) {
			debts = bridge.getDebtInstruments(corp.id);
		}
	});

	function takeLoan() {
		const corp = $playerCorp;
		if (!corp) return;
		const corpId = corp.id;
		const amount = loanAmount;
		showConfirm(`Take a loan of ${formatMoney(amount)}? Interest rates depend on your credit rating.`, () => {
			gameCommand({ TakeLoan: { corporation: corpId, amount } });
			showLoanDialog = false;
			debts = bridge.getDebtInstruments(corpId);
		});
	}

	function repayLoan(debtId: number) {
		const corp = $playerCorp;
		if (!corp) return;
		const debt = debts.find((d) => d.id === debtId);
		if (!debt) return;
		gameCommand({ RepayLoan: { loan: debtId, amount: debt.principal } });
		debts = bridge.getDebtInstruments(corp.id);
	}

	let totalDebt = $derived(debts.reduce((s, d) => s + d.principal, 0));
	let totalPayments = $derived(debts.reduce((s, d) => s + d.payment_per_tick, 0));

	// Budget & Policy state — tracks values sent via commands.
	// TODO: Replace with bridge query (e.g. bridge.getPolicies(corpId)) when backend exposes policy read API
	let maintenanceBudget = $state(500_000);
	let expansionPriority = $state('balanced');
	let pricingStrategy = $state('market');

	function setMaintenanceBudget(val: number) {
		maintenanceBudget = val;
		gameCommand({ SetBudget: { corporation: $playerCorp?.id ?? 0, category: 'maintenance', amount: val } });
	}

	function setExpansionPriority(val: string) {
		expansionPriority = val;
		gameCommand({ SetPolicy: { corporation: $playerCorp?.id ?? 0, policy: 'expansion_priority', value: val } });
	}

	function setPricingStrategy(val: string) {
		pricingStrategy = val;
		gameCommand({ SetPolicy: { corporation: $playerCorp?.id ?? 0, policy: 'pricing_strategy', value: val } });
	}
</script>

<div class="panel" aria-label={$tr('panels.dashboard')}>
	<div class="section">
		<h3>{$tr('panels.income_statement')}</h3>
		<div class="stat-grid">
			<div class="stat">
				<span class="label">{$tr('panels.revenue')}</span>
				<span class="value green">{formatMoney($playerCorp?.revenue_per_tick ?? 0)}/tick</span>
			</div>
			<div class="stat">
				<span class="label">{$tr('panels.costs')}</span>
				<span class="value red">{formatMoney($playerCorp?.cost_per_tick ?? 0)}/tick</span>
			</div>
			<div class="stat">
				<span class="label">{$tr('panels.net_income')}</span>
				<span class="value" class:green={($playerCorp?.profit_per_tick ?? 0) >= 0} class:red={($playerCorp?.profit_per_tick ?? 0) < 0}>
					{formatMoney($playerCorp?.profit_per_tick ?? 0)}/tick
				</span>
			</div>
		</div>
	</div>

	<div class="section">
		<h3>{$tr('panels.balance_sheet')}</h3>
		<div class="stat-grid">
			<div class="stat">
				<span class="label">{$tr('panels.cash')}</span>
				<span class="value mono">{formatMoney($playerCorp?.cash ?? 0)}</span>
			</div>
			<div class="stat">
				<span class="label">{$tr('panels.total_debt')}</span>
				<span class="value mono red">{formatMoney(totalDebt)}</span>
			</div>
			<div class="stat">
				<span class="label">{$tr('panels.debt_payments')}</span>
				<span class="value mono">{formatMoney(totalPayments)}/tick</span>
			</div>
			<div class="stat">
				<span class="label">{$tr('panels.credit_rating')}</span>
				<span class="value amber">{$playerCorp?.credit_rating ?? '---'}</span>
			</div>
			<div class="stat">
				<span class="label">{$tr('panels.employees')}</span>
				<span class="value mono">{$playerCorp?.employee_count ?? 0}</span>
			</div>
			<div class="stat">
				<span class="label">{$tr('panels.morale')}</span>
				<span class="value mono">{(($playerCorp?.morale ?? 0) * 100).toFixed(0)}%</span>
			</div>
		</div>
	</div>

	<div class="section">
		<h3>{$tr('panels.revenue_trend')}</h3>
		<FinanceChart />
	</div>

	<div class="section">
		<div class="section-header">
			<h3>{$tr('panels.loans', { count: debts.length })}</h3>
			<button class="action-btn" onclick={() => (showLoanDialog = !showLoanDialog)} use:tooltip={() => `Take a loan to fund expansion\nYour credit rating: ${$playerCorp?.credit_rating ?? '---'}\nBetter ratings = lower interest`}>+ Take Loan</button>
		</div>

		{#if showLoanDialog}
			<div class="loan-dialog">
				<label>
					Amount:
					<input type="range" min={100000} max={50000000} step={100000} bind:value={loanAmount} />
					<span class="mono">{formatMoney(loanAmount)}</span>
				</label>
				<button class="confirm-btn" onclick={takeLoan} use:tooltip={() => `Confirm loan of ${formatMoney(loanAmount)}\nInterest rate depends on credit rating (${$playerCorp?.credit_rating ?? '---'})`}>{$tr('panels.confirm_loan')}</button>
			</div>
		{/if}

		{#each debts.filter((d) => !d.is_paid_off) as debt}
			<div class="debt-row">
				<div class="debt-info">
					<span class="mono">{formatMoney(debt.principal)}</span>
					<span class="muted">{(debt.interest_rate * 100).toFixed(1)}% rate | {debt.remaining_ticks} ticks left</span>
				</div>
				<button class="small-btn" onclick={() => repayLoan(debt.id)} use:tooltip={() => `Repay full principal: ${formatMoney(debt.principal)}\n${debt.remaining_ticks} ticks remaining at ${(debt.interest_rate * 100).toFixed(1)}% rate`}>{$tr('panels.repay')}</button>
			</div>
		{/each}
	</div>

	<div class="section">
		<h3>Budgets & Policies</h3>
		<div class="policy-row">
			<span class="policy-label">Maintenance Budget</span>
			<input type="range" min={0} max={5000000} step={50000} bind:value={maintenanceBudget}
				oninput={(e) => {
					const val = Number((e.target as HTMLInputElement).value);
					setMaintenanceBudget(val);
				}} />
			<span class="policy-val mono">{formatMoney(maintenanceBudget)}</span>
		</div>
		<div class="policy-row">
			<span class="policy-label">Expansion Priority</span>
			<select class="policy-select" bind:value={expansionPriority}
				onchange={(e) => {
					setExpansionPriority((e.target as HTMLSelectElement).value);
				}}>
				<option value="balanced">Balanced</option>
				<option value="aggressive">Aggressive</option>
				<option value="conservative">Conservative</option>
			</select>
		</div>
		<div class="policy-row">
			<span class="policy-label">Pricing Strategy</span>
			<select class="policy-select" bind:value={pricingStrategy}
				onchange={(e) => {
					setPricingStrategy((e.target as HTMLSelectElement).value);
				}}>
				<option value="market">Market Rate</option>
				<option value="undercut">Undercut (-10%)</option>
				<option value="premium">Premium (+15%)</option>
			</select>
		</div>
	</div>

	<div class="section">
		<h3>{$tr('panels.market_share')}</h3>
		<MarketShareChart />
	</div>

	<div class="section">
		<h3>{$tr('panels.competitors')}</h3>
		{#each $allCorporations.filter((c) => !c.is_player) as corp}
			<div class="competitor-row">
				<span class="name">{corp.name}</span>
				<span class="mono">{formatMoney(corp.cash)}</span>
				<span class="rating-badge">{corp.credit_rating}</span>
			</div>
		{/each}
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

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 8px;
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

	.label {
		color: var(--text-muted);
	}

	.value {
		color: var(--text-primary);
	}

	.mono {
		font-family: var(--font-mono);
	}

	.green {
		color: var(--green);
	}

	.red {
		color: var(--red);
	}

	.amber {
		color: var(--amber);
		font-weight: 600;
	}

	.muted {
		color: var(--text-muted);
		font-size: 11px;
	}

	.action-btn {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--blue);
		padding: 4px 10px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 12px;
		font-family: var(--font-mono);
	}

	.action-btn:hover {
		background: var(--bg-hover);
	}

	.loan-dialog {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		padding: 12px;
		margin-bottom: 8px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.loan-dialog label {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--text-muted);
		font-size: 12px;
	}

	.loan-dialog input[type='range'] {
		flex: 1;
		accent-color: var(--blue);
	}

	.confirm-btn {
		background: var(--green-bg);
		border: 1px solid var(--green-border);
		color: var(--green);
		padding: 6px 12px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 12px;
	}

	.debt-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 6px 0;
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
	}

	.debt-info {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.small-btn {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--text-muted);
		padding: 3px 8px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 11px;
	}

	.small-btn:hover {
		color: var(--green);
		border-color: var(--green-border);
	}

	.competitor-row {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 6px 0;
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
	}

	.name {
		flex: 1;
		color: var(--text-primary);
	}

	.rating-badge {
		background: var(--amber-bg);
		color: var(--amber);
		padding: 2px 6px;
		border-radius: var(--radius-sm);
		font-size: 11px;
		font-weight: 600;
	}

	.policy-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px 0;
	}

	.policy-label {
		font-size: 12px;
		color: var(--text-muted);
		min-width: 120px;
	}

	.policy-val {
		font-size: 12px;
		color: var(--text-secondary);
		min-width: 50px;
		text-align: right;
	}

	.policy-row input[type='range'] {
		flex: 1;
		accent-color: var(--blue);
	}

	.policy-select {
		flex: 1;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 4px 6px;
		border-radius: var(--radius-sm);
		font-size: 12px;
		font-family: var(--font-sans);
	}
</style>
