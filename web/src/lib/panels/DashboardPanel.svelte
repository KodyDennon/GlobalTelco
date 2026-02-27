<script lang="ts">
	import { playerCorp, formatMoney, allCorporations, worldInfo } from '$lib/stores/gameState';
	import { showConfirm, companyTier, companyTierLabel } from '$lib/stores/uiState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import type { DebtInfo, InfraNode } from '$lib/wasm/types';
	import FinanceChart from '$lib/charts/FinanceChart.svelte';
	import MarketShareChart from '$lib/charts/MarketShareChart.svelte';
	import { tr } from '$lib/i18n/index';
	import { tooltip } from '$lib/ui/tooltip';

	let debts: DebtInfo[] = $state([]);
	let showLoanDialog = $state(false);
	let loanAmount = $state(1_000_000);

	// Infrastructure list for small-tier per-node view
	let infraNodes: InfraNode[] = $state([]);

	// Refresh debts and infra when corp changes or each tick
	$effect(() => {
		const _tick = $worldInfo.tick;
		const corp = $playerCorp;
		if (corp) {
			debts = bridge.getDebtInstruments(corp.id);
			// Fetch infra list for small/medium tier detail views
			if ($companyTier === 'small' || $companyTier === 'medium') {
				const infra = bridge.getInfrastructureList(corp.id);
				infraNodes = infra.nodes;
			}
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

	// Budget & Policy state
	let maintenanceBudget = $state(500_000);
	let expansionPriority = $state('balanced');
	let pricingStrategy = $state('market');

	// Large-tier policy states
	let hiringPolicy = $state('normal');
	let researchFocus = $state('balanced');

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

	function setHiringPolicy(val: string) {
		hiringPolicy = val;
		gameCommand({ SetPolicy: { corporation: $playerCorp?.id ?? 0, policy: 'hiring_policy', value: val } });
	}

	function setResearchFocus(val: string) {
		researchFocus = val;
		gameCommand({ SetPolicy: { corporation: $playerCorp?.id ?? 0, policy: 'research_focus', value: val } });
	}

	// Derived stats for medium/large tiers
	let revenuePerEmployee = $derived(
		($playerCorp?.employee_count ?? 0) > 0
			? ($playerCorp?.revenue_per_tick ?? 0) / ($playerCorp?.employee_count ?? 1)
			: 0
	);
	let costPerNode = $derived(
		($playerCorp?.infrastructure_count ?? 0) > 0
			? ($playerCorp?.cost_per_tick ?? 0) / ($playerCorp?.infrastructure_count ?? 1)
			: 0
	);
	let profitMargin = $derived(
		($playerCorp?.revenue_per_tick ?? 0) > 0
			? (($playerCorp?.profit_per_tick ?? 0) / ($playerCorp?.revenue_per_tick ?? 1)) * 100
			: 0
	);

	// Per-node revenue estimate for small tier
	let revenuePerNode = $derived(
		($playerCorp?.infrastructure_count ?? 0) > 0
			? ($playerCorp?.revenue_per_tick ?? 0) / ($playerCorp?.infrastructure_count ?? 1)
			: 0
	);

	// Node type distribution for medium/large views
	let nodeTypeDistribution = $derived(() => {
		const counts: Record<string, number> = {};
		for (const node of infraNodes) {
			counts[node.node_type] = (counts[node.node_type] ?? 0) + 1;
		}
		return Object.entries(counts).sort((a, b) => b[1] - a[1]);
	});
</script>

<div class="panel" aria-label={$tr('panels.dashboard')}>
	<!-- Tier badge -->
	<div class="tier-badge-row">
		<span class="tier-badge tier-{$companyTier}">{$companyTierLabel}</span>
		<span class="tier-detail mono">{$playerCorp?.infrastructure_count ?? 0} nodes</span>
	</div>

	<!-- INCOME STATEMENT — all tiers -->
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
			{#if $companyTier !== 'small'}
				<div class="stat">
					<span class="label">{$tr('panels.profit_margin')}</span>
					<span class="value mono" class:green={profitMargin >= 0} class:red={profitMargin < 0}>
						{profitMargin.toFixed(1)}%
					</span>
				</div>
			{/if}
		</div>
	</div>

	<!-- BALANCE SHEET — all tiers, extra detail for medium+ -->
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
			{#if $companyTier !== 'small'}
				<div class="stat">
					<span class="label">{$tr('panels.revenue_per_employee')}</span>
					<span class="value mono green">{formatMoney(revenuePerEmployee)}</span>
				</div>
				<div class="stat">
					<span class="label">{$tr('panels.cost_per_node')}</span>
					<span class="value mono red">{formatMoney(costPerNode)}</span>
				</div>
			{/if}
		</div>
	</div>

	<!-- SMALL TIER: Individual node list with per-node revenue -->
	{#if $companyTier === 'small'}
		<div class="section">
			<h3>{$tr('panels.your_infrastructure')}</h3>
			{#if infraNodes.length === 0}
				<p class="empty-hint">{$tr('panels.no_infra_hint')}</p>
			{:else}
				<div class="node-list">
					{#each infraNodes as node}
						<div class="node-row">
							<div class="node-info">
								<span class="node-type">{node.node_type.replace(/([A-Z])/g, ' $1').trim()}</span>
								<span class="node-meta muted">
									{node.under_construction ? 'Building...' : `${(node.utilization * 100).toFixed(0)}% util`}
								</span>
							</div>
							<div class="node-stats">
								<span class="mono green" use:tooltip={() => `Estimated revenue share per node: ${formatMoney(revenuePerNode)}/tick`}>
									~{formatMoney(revenuePerNode)}/tick
								</span>
								<span class="health-bar">
									<span class="health-fill" class:green-bar={node.health >= 0.7} class:amber-bar={node.health >= 0.4 && node.health < 0.7} class:red-bar={node.health < 0.4} style="width: {node.health * 100}%"></span>
								</span>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</div>

		<div class="section">
			<h3>{$tr('panels.quick_actions')}</h3>
			<div class="quick-links">
				<span class="quick-link" use:tooltip={() => 'Open Workforce panel to hire or fire employees'}>Manage Employees ({$playerCorp?.employee_count ?? 0})</span>
				<span class="quick-link" use:tooltip={() => 'Open Infrastructure panel for build options'}>Build Infrastructure</span>
			</div>
		</div>
	{/if}

	<!-- MEDIUM TIER: Team management summary, regional budget, department stats -->
	{#if $companyTier === 'medium'}
		<div class="section">
			<h3>{$tr('panels.department_summary')}</h3>
			<div class="dept-grid">
				<div class="dept-card">
					<span class="dept-name">Operations</span>
					<span class="dept-stat mono">{$playerCorp?.infrastructure_count ?? 0} nodes</span>
					<span class="dept-detail muted">Avg health: {infraNodes.length > 0 ? (infraNodes.reduce((s, n) => s + n.health, 0) / infraNodes.length * 100).toFixed(0) : 0}%</span>
				</div>
				<div class="dept-card">
					<span class="dept-name">Workforce</span>
					<span class="dept-stat mono">{$playerCorp?.employee_count ?? 0} staff</span>
					<span class="dept-detail muted">Morale: {(($playerCorp?.morale ?? 0) * 100).toFixed(0)}%</span>
				</div>
				<div class="dept-card">
					<span class="dept-name">Finance</span>
					<span class="dept-stat mono" class:green={($playerCorp?.profit_per_tick ?? 0) >= 0} class:red={($playerCorp?.profit_per_tick ?? 0) < 0}>{formatMoney($playerCorp?.profit_per_tick ?? 0)}/tick</span>
					<span class="dept-detail muted">Rating: {$playerCorp?.credit_rating ?? '---'}</span>
				</div>
				<div class="dept-card">
					<span class="dept-name">Debt Mgmt</span>
					<span class="dept-stat mono red">{formatMoney(totalDebt)}</span>
					<span class="dept-detail muted">{debts.filter(d => !d.is_paid_off).length} active loans</span>
				</div>
			</div>
		</div>

		<div class="section">
			<h3>{$tr('panels.asset_distribution')}</h3>
			<div class="dist-list">
				{#each nodeTypeDistribution() as [type, count]}
					<div class="dist-row">
						<span class="dist-label">{type.replace(/([A-Z])/g, ' $1').trim()}</span>
						<div class="dist-bar-bg">
							<div class="dist-bar-fill" style="width: {(count / ($playerCorp?.infrastructure_count ?? 1)) * 100}%"></div>
						</div>
						<span class="dist-count mono">{count}</span>
					</div>
				{/each}
			</div>
		</div>

		<div class="section">
			<h3>{$tr('panels.regional_budget')}</h3>
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
	{/if}

	<!-- LARGE TIER: Policy overview, department performance, AI execution, quarterly -->
	{#if $companyTier === 'large'}
		<div class="section">
			<h3>{$tr('panels.corporate_policy')}</h3>
			<div class="policy-cards">
				<div class="policy-card">
					<span class="policy-card-title">Expansion</span>
					<select class="policy-select-lg" bind:value={expansionPriority}
						onchange={(e) => setExpansionPriority((e.target as HTMLSelectElement).value)}>
						<option value="balanced">Balanced</option>
						<option value="aggressive">Aggressive</option>
						<option value="conservative">Conservative</option>
					</select>
					<span class="policy-desc muted">Controls new node placement rate and risk appetite</span>
				</div>
				<div class="policy-card">
					<span class="policy-card-title">Pricing</span>
					<select class="policy-select-lg" bind:value={pricingStrategy}
						onchange={(e) => setPricingStrategy((e.target as HTMLSelectElement).value)}>
						<option value="market">Market Rate</option>
						<option value="undercut">Undercut (-10%)</option>
						<option value="premium">Premium (+15%)</option>
					</select>
					<span class="policy-desc muted">Sets default service pricing across all regions</span>
				</div>
				<div class="policy-card">
					<span class="policy-card-title">Hiring</span>
					<select class="policy-select-lg" bind:value={hiringPolicy}
						onchange={(e) => setHiringPolicy((e.target as HTMLSelectElement).value)}>
						<option value="normal">Normal</option>
						<option value="freeze">Hiring Freeze</option>
						<option value="aggressive">Aggressive Hiring</option>
						<option value="targeted">Targeted Only</option>
					</select>
					<span class="policy-desc muted">Controls workforce growth rate and hiring automation</span>
				</div>
				<div class="policy-card">
					<span class="policy-card-title">Research</span>
					<select class="policy-select-lg" bind:value={researchFocus}
						onchange={(e) => setResearchFocus((e.target as HTMLSelectElement).value)}>
						<option value="balanced">Balanced</option>
						<option value="cost_reduction">Cost Reduction</option>
						<option value="throughput">Throughput</option>
						<option value="reliability">Reliability</option>
					</select>
					<span class="policy-desc muted">Prioritizes research direction for AI-managed labs</span>
				</div>
			</div>
		</div>

		<div class="section">
			<h3>{$tr('panels.dept_performance')}</h3>
			<div class="dept-perf-grid">
				<div class="dept-perf-card">
					<div class="dept-perf-header">
						<span class="dept-perf-name">Operations</span>
						<span class="dept-perf-badge" class:green={($playerCorp?.infrastructure_count ?? 0) > 0}>Active</span>
					</div>
					<div class="dept-perf-stats">
						<div class="dept-perf-stat">
							<span class="muted">Nodes</span>
							<span class="mono">{$playerCorp?.infrastructure_count ?? 0}</span>
						</div>
						<div class="dept-perf-stat">
							<span class="muted">Avg Health</span>
							<span class="mono">{infraNodes.length > 0 ? (infraNodes.reduce((s, n) => s + n.health, 0) / infraNodes.length * 100).toFixed(0) : 0}%</span>
						</div>
						<div class="dept-perf-stat">
							<span class="muted">Cost/Node</span>
							<span class="mono red">{formatMoney(costPerNode)}</span>
						</div>
					</div>
				</div>
				<div class="dept-perf-card">
					<div class="dept-perf-header">
						<span class="dept-perf-name">Human Resources</span>
						<span class="dept-perf-badge" class:green={($playerCorp?.morale ?? 0) >= 0.7} class:amber={($playerCorp?.morale ?? 0) >= 0.4 && ($playerCorp?.morale ?? 0) < 0.7} class:red={($playerCorp?.morale ?? 0) < 0.4}>
							{(($playerCorp?.morale ?? 0) * 100).toFixed(0)}% Morale
						</span>
					</div>
					<div class="dept-perf-stats">
						<div class="dept-perf-stat">
							<span class="muted">Headcount</span>
							<span class="mono">{$playerCorp?.employee_count ?? 0}</span>
						</div>
						<div class="dept-perf-stat">
							<span class="muted">Rev/Employee</span>
							<span class="mono green">{formatMoney(revenuePerEmployee)}</span>
						</div>
						<div class="dept-perf-stat">
							<span class="muted">Policy</span>
							<span class="mono">{hiringPolicy === 'freeze' ? 'Frozen' : hiringPolicy === 'aggressive' ? 'Hiring' : 'Normal'}</span>
						</div>
					</div>
				</div>
				<div class="dept-perf-card">
					<div class="dept-perf-header">
						<span class="dept-perf-name">Treasury</span>
						<span class="dept-perf-badge" class:green={($playerCorp?.profit_per_tick ?? 0) > 0} class:red={($playerCorp?.profit_per_tick ?? 0) <= 0}>
							{($playerCorp?.profit_per_tick ?? 0) > 0 ? 'Profitable' : 'Loss'}
						</span>
					</div>
					<div class="dept-perf-stats">
						<div class="dept-perf-stat">
							<span class="muted">Cash</span>
							<span class="mono">{formatMoney($playerCorp?.cash ?? 0)}</span>
						</div>
						<div class="dept-perf-stat">
							<span class="muted">Debt</span>
							<span class="mono red">{formatMoney(totalDebt)}</span>
						</div>
						<div class="dept-perf-stat">
							<span class="muted">Margin</span>
							<span class="mono" class:green={profitMargin >= 0} class:red={profitMargin < 0}>{profitMargin.toFixed(1)}%</span>
						</div>
					</div>
				</div>
			</div>
		</div>

		<div class="section">
			<h3>{$tr('panels.maintenance_budget')}</h3>
			<div class="policy-row">
				<span class="policy-label">{$tr('panels.monthly_budget')}</span>
				<input type="range" min={0} max={5000000} step={50000} bind:value={maintenanceBudget}
					oninput={(e) => {
						const val = Number((e.target as HTMLInputElement).value);
						setMaintenanceBudget(val);
					}} />
				<span class="policy-val mono">{formatMoney(maintenanceBudget)}</span>
			</div>
		</div>

		<div class="section">
			<h3>{$tr('panels.ai_execution')}</h3>
			<div class="ai-summary">
				<div class="ai-row">
					<span class="ai-label">Auto-maintenance</span>
					<span class="ai-status green">Active</span>
					<span class="ai-detail muted">{infraNodes.filter(n => n.health < 0.9).length} nodes queued for repair</span>
				</div>
				<div class="ai-row">
					<span class="ai-label">Hiring Automation</span>
					<span class="ai-status" class:green={hiringPolicy !== 'freeze'} class:amber={hiringPolicy === 'freeze'}>{hiringPolicy === 'freeze' ? 'Paused' : 'Active'}</span>
					<span class="ai-detail muted">{hiringPolicy === 'freeze' ? 'No new hires' : 'Filling open positions'}</span>
				</div>
				<div class="ai-row">
					<span class="ai-label">Research Direction</span>
					<span class="ai-status blue">{researchFocus === 'balanced' ? 'Balanced' : researchFocus === 'cost_reduction' ? 'Cost Focus' : researchFocus === 'throughput' ? 'Throughput' : 'Reliability'}</span>
					<span class="ai-detail muted">Prioritizing {researchFocus.replace('_', ' ')} research</span>
				</div>
			</div>
		</div>

		<div class="section">
			<h3>{$tr('panels.quarterly_report')}</h3>
			<div class="quarterly-grid">
				<div class="quarterly-stat">
					<span class="quarterly-label">Revenue Growth</span>
					<span class="quarterly-value mono green">+{formatMoney($playerCorp?.revenue_per_tick ?? 0)}</span>
				</div>
				<div class="quarterly-stat">
					<span class="quarterly-label">Network Expansion</span>
					<span class="quarterly-value mono">{$playerCorp?.infrastructure_count ?? 0} total nodes</span>
				</div>
				<div class="quarterly-stat">
					<span class="quarterly-label">Workforce</span>
					<span class="quarterly-value mono">{$playerCorp?.employee_count ?? 0} employees</span>
				</div>
				<div class="quarterly-stat">
					<span class="quarterly-label">Debt Ratio</span>
					<span class="quarterly-value mono" class:green={totalDebt < ($playerCorp?.cash ?? 0)} class:red={totalDebt >= ($playerCorp?.cash ?? 0)}>
						{($playerCorp?.cash ?? 0) > 0 ? (totalDebt / ($playerCorp?.cash ?? 1) * 100).toFixed(0) : '---'}%
					</span>
				</div>
			</div>
		</div>
	{/if}

	<!-- REVENUE TREND — all tiers -->
	<div class="section">
		<h3>{$tr('panels.revenue_trend')}</h3>
		<FinanceChart />
	</div>

	<!-- LOANS — all tiers -->
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

	<!-- BUDGETS & POLICIES — small tier only (medium/large have their own sections) -->
	{#if $companyTier === 'small'}
		<div class="section">
			<h3>{$tr('panels.budgets_policies')}</h3>
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
	{/if}

	<!-- MARKET SHARE — all tiers -->
	<div class="section">
		<h3>{$tr('panels.market_share')}</h3>
		<MarketShareChart />
	</div>

	<!-- COMPETITORS — all tiers -->
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

	.blue {
		color: var(--blue);
	}

	.muted {
		color: var(--text-muted);
		font-size: 11px;
	}

	/* ── Tier Badge ──────────────────────────────────── */

	.tier-badge-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 16px;
		border-bottom: 1px solid var(--border);
		background: rgba(17, 24, 39, 0.5);
	}

	.tier-badge {
		display: inline-block;
		padding: 2px 8px;
		border-radius: var(--radius-sm);
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.tier-badge.tier-small {
		background: rgba(59, 130, 246, 0.15);
		color: var(--blue);
		border: 1px solid rgba(59, 130, 246, 0.3);
	}

	.tier-badge.tier-medium {
		background: rgba(245, 158, 11, 0.15);
		color: var(--amber);
		border: 1px solid rgba(245, 158, 11, 0.3);
	}

	.tier-badge.tier-large {
		background: rgba(16, 185, 129, 0.15);
		color: var(--green);
		border: 1px solid rgba(16, 185, 129, 0.3);
	}

	.tier-detail {
		font-size: 11px;
		color: var(--text-muted);
	}

	/* ── Small Tier: Node List ───────────────────────── */

	.node-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.node-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 6px 8px;
		border-radius: var(--radius-sm);
		background: rgba(17, 24, 39, 0.3);
	}

	.node-row:hover {
		background: var(--bg-hover);
	}

	.node-info {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.node-type {
		font-size: 12px;
		color: var(--text-primary);
	}

	.node-meta {
		font-size: 10px;
	}

	.node-stats {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 3px;
	}

	.node-stats .mono {
		font-size: 11px;
	}

	.health-bar {
		width: 40px;
		height: 4px;
		background: var(--bg-surface);
		border-radius: 2px;
		overflow: hidden;
	}

	.health-fill {
		height: 100%;
		border-radius: 2px;
		transition: width 0.3s;
	}

	.green-bar { background: var(--green); }
	.amber-bar { background: var(--amber); }
	.red-bar { background: var(--red); }

	.quick-links {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.quick-link {
		font-size: 12px;
		color: var(--blue);
		cursor: default;
		padding: 4px 0;
	}

	.empty-hint {
		color: var(--text-dim);
		font-size: 12px;
		text-align: center;
		margin: 0;
		padding: 8px 0;
	}

	/* ── Medium Tier: Department Cards ───────────────── */

	.dept-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
	}

	.dept-card {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 8px;
		border-radius: var(--radius-sm);
		background: rgba(17, 24, 39, 0.4);
		border: 1px solid rgba(55, 65, 81, 0.3);
	}

	.dept-name {
		font-size: 11px;
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.3px;
	}

	.dept-stat {
		font-size: 14px;
		color: var(--text-primary);
	}

	.dept-detail {
		font-size: 10px;
	}

	/* ── Medium Tier: Asset Distribution ─────────────── */

	.dist-list {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.dist-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.dist-label {
		font-size: 12px;
		color: var(--text-muted);
		min-width: 100px;
	}

	.dist-bar-bg {
		flex: 1;
		height: 6px;
		background: var(--bg-surface);
		border-radius: 3px;
		overflow: hidden;
	}

	.dist-bar-fill {
		height: 100%;
		background: var(--blue);
		border-radius: 3px;
		transition: width 0.3s;
	}

	.dist-count {
		font-size: 12px;
		color: var(--text-primary);
		min-width: 24px;
		text-align: right;
	}

	/* ── Large Tier: Policy Cards ────────────────────── */

	.policy-cards {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
	}

	.policy-card {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 10px;
		border-radius: var(--radius-sm);
		background: rgba(17, 24, 39, 0.4);
		border: 1px solid rgba(55, 65, 81, 0.3);
	}

	.policy-card-title {
		font-size: 11px;
		font-weight: 700;
		color: var(--text-primary);
		text-transform: uppercase;
		letter-spacing: 0.4px;
	}

	.policy-select-lg {
		width: 100%;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 5px 6px;
		border-radius: var(--radius-sm);
		font-size: 12px;
		font-family: var(--font-sans);
	}

	.policy-desc {
		font-size: 10px;
		line-height: 1.3;
	}

	/* ── Large Tier: Department Performance ──────────── */

	.dept-perf-grid {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.dept-perf-card {
		padding: 10px;
		border-radius: var(--radius-sm);
		background: rgba(17, 24, 39, 0.4);
		border: 1px solid rgba(55, 65, 81, 0.3);
	}

	.dept-perf-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 8px;
	}

	.dept-perf-name {
		font-size: 12px;
		font-weight: 600;
		color: var(--text-primary);
	}

	.dept-perf-badge {
		font-size: 10px;
		padding: 2px 6px;
		border-radius: var(--radius-sm);
		background: var(--bg-surface);
		color: var(--text-muted);
	}

	.dept-perf-badge.green {
		background: rgba(16, 185, 129, 0.15);
		color: var(--green);
	}

	.dept-perf-badge.amber {
		background: rgba(245, 158, 11, 0.15);
		color: var(--amber);
	}

	.dept-perf-badge.red {
		background: rgba(239, 68, 68, 0.15);
		color: var(--red);
	}

	.dept-perf-stats {
		display: grid;
		grid-template-columns: 1fr 1fr 1fr;
		gap: 4px;
	}

	.dept-perf-stat {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}

	.dept-perf-stat .muted {
		font-size: 10px;
	}

	.dept-perf-stat .mono {
		font-size: 12px;
		color: var(--text-primary);
	}

	/* ── Large Tier: AI Execution Summary ────────────── */

	.ai-summary {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.ai-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px 0;
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
	}

	.ai-label {
		font-size: 12px;
		color: var(--text-muted);
		min-width: 120px;
	}

	.ai-status {
		font-size: 11px;
		font-weight: 600;
		min-width: 50px;
	}

	.ai-detail {
		flex: 1;
		text-align: right;
	}

	/* ── Large Tier: Quarterly Report ────────────────── */

	.quarterly-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
	}

	.quarterly-stat {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: 8px;
		border-radius: var(--radius-sm);
		background: rgba(17, 24, 39, 0.3);
	}

	.quarterly-label {
		font-size: 10px;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.3px;
	}

	.quarterly-value {
		font-size: 13px;
		color: var(--text-primary);
	}

	/* ── Shared Styles ───────────────────────────────── */

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
