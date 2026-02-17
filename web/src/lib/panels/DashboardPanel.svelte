<script lang="ts">
	import { playerCorp, formatMoney, allCorporations } from '$lib/stores/gameState';
	import { activePanel } from '$lib/stores/uiState';
	import * as bridge from '$lib/wasm/bridge';
	import type { DebtInfo } from '$lib/wasm/types';
	import FinanceChart from '$lib/charts/FinanceChart.svelte';

	let debts: DebtInfo[] = $state([]);
	let showLoanDialog = $state(false);
	let loanAmount = $state(1_000_000);

	$effect(() => {
		const corp = $playerCorp;
		if (corp) {
			debts = bridge.getDebtInstruments(corp.id);
		}
	});

	function takeLoan() {
		const corp = $playerCorp;
		if (!corp) return;
		bridge.processCommand({ TakeLoan: { corporation: corp.id, amount: loanAmount } });
		showLoanDialog = false;
		debts = bridge.getDebtInstruments(corp.id);
	}

	function repayLoan(debtId: number) {
		const corp = $playerCorp;
		if (!corp) return;
		const debt = debts.find((d) => d.id === debtId);
		if (!debt) return;
		bridge.processCommand({ RepayLoan: { loan: debtId, amount: debt.principal } });
		debts = bridge.getDebtInstruments(corp.id);
	}

	function close() {
		activePanel.set('none');
	}

	let totalDebt = $derived(debts.reduce((s, d) => s + d.principal, 0));
	let totalPayments = $derived(debts.reduce((s, d) => s + d.payment_per_tick, 0));
</script>

<div class="panel">
	<div class="panel-header">
		<span class="title">Financial Dashboard</span>
		<button class="close" onclick={close}>x</button>
	</div>

	<div class="section">
		<h3>Income Statement</h3>
		<div class="stat-grid">
			<div class="stat">
				<span class="label">Revenue</span>
				<span class="value green">{formatMoney($playerCorp?.revenue_per_tick ?? 0)}/tick</span>
			</div>
			<div class="stat">
				<span class="label">Costs</span>
				<span class="value red">{formatMoney($playerCorp?.cost_per_tick ?? 0)}/tick</span>
			</div>
			<div class="stat">
				<span class="label">Net Income</span>
				<span class="value" class:green={($playerCorp?.profit_per_tick ?? 0) >= 0} class:red={($playerCorp?.profit_per_tick ?? 0) < 0}>
					{formatMoney($playerCorp?.profit_per_tick ?? 0)}/tick
				</span>
			</div>
		</div>
	</div>

	<div class="section">
		<h3>Balance Sheet</h3>
		<div class="stat-grid">
			<div class="stat">
				<span class="label">Cash</span>
				<span class="value mono">{formatMoney($playerCorp?.cash ?? 0)}</span>
			</div>
			<div class="stat">
				<span class="label">Total Debt</span>
				<span class="value mono red">{formatMoney(totalDebt)}</span>
			</div>
			<div class="stat">
				<span class="label">Debt Payments</span>
				<span class="value mono">{formatMoney(totalPayments)}/tick</span>
			</div>
			<div class="stat">
				<span class="label">Credit Rating</span>
				<span class="value amber">{$playerCorp?.credit_rating ?? '---'}</span>
			</div>
			<div class="stat">
				<span class="label">Employees</span>
				<span class="value mono">{$playerCorp?.employee_count ?? 0}</span>
			</div>
			<div class="stat">
				<span class="label">Morale</span>
				<span class="value mono">{(($playerCorp?.morale ?? 0) * 100).toFixed(0)}%</span>
			</div>
		</div>
	</div>

	<div class="section">
		<h3>Revenue Trend</h3>
		<FinanceChart />
	</div>

	<div class="section">
		<div class="section-header">
			<h3>Loans ({debts.length})</h3>
			<button class="action-btn" onclick={() => (showLoanDialog = !showLoanDialog)}>+ Take Loan</button>
		</div>

		{#if showLoanDialog}
			<div class="loan-dialog">
				<label>
					Amount:
					<input type="range" min={100000} max={50000000} step={100000} bind:value={loanAmount} />
					<span class="mono">{formatMoney(loanAmount)}</span>
				</label>
				<button class="confirm-btn" onclick={takeLoan}>Confirm Loan</button>
			</div>
		{/if}

		{#each debts.filter((d) => !d.is_paid_off) as debt}
			<div class="debt-row">
				<div class="debt-info">
					<span class="mono">{formatMoney(debt.principal)}</span>
					<span class="muted">{(debt.interest_rate * 100).toFixed(1)}% rate | {debt.remaining_ticks} ticks left</span>
				</div>
				<button class="small-btn" onclick={() => repayLoan(debt.id)}>Repay</button>
			</div>
		{/each}
	</div>

	<div class="section">
		<h3>Competitors</h3>
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

	.title {
		font-weight: 700;
		font-size: 14px;
		color: var(--text-primary);
	}

	.close {
		background: none;
		border: none;
		color: var(--text-dim);
		cursor: pointer;
		font-size: 16px;
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
</style>
