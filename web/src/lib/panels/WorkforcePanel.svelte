<script lang="ts">
	import { playerCorp, formatMoney, worldInfo } from '$lib/stores/gameState';
	import { showConfirm, companyTier, companyTierLabel } from '$lib/stores/uiState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import { tr } from '$lib/i18n/index';
	import { tooltip } from '$lib/ui/tooltip';

	function hire() {
		const corp = $playerCorp;
		if (!corp) return;
		gameCommand({ HireEmployee: { corporation: corp.id, role: 'technician' } });
	}

	function fire() {
		const corp = $playerCorp;
		if (!corp) return;
		if ((corp.employee_count ?? 0) <= 1) return;
		const corpId = corp.id;
		showConfirm('Fire an employee? This will reduce your workforce by 1.', () => {
			gameCommand({ FireEmployee: { entity: corpId } });
		});
	}

	// Bulk hiring for medium/large tiers
	let bulkHireCount = $state(5);
	function bulkHire() {
		const corp = $playerCorp;
		if (!corp) return;
		const count = bulkHireCount;
		const corpId = corp.id;
		showConfirm(`Hire ${count} employees? This will increase salary costs.`, () => {
			for (let i = 0; i < count; i++) {
				gameCommand({ HireEmployee: { corporation: corpId, role: 'technician' } });
			}
		});
	}

	function bulkFire() {
		const corp = $playerCorp;
		if (!corp) return;
		const count = Math.min(bulkHireCount, (corp.employee_count ?? 0) - 1);
		if (count <= 0) return;
		const corpId = corp.id;
		showConfirm(`Fire ${count} employees? This will reduce your workforce.`, () => {
			for (let i = 0; i < count; i++) {
				gameCommand({ FireEmployee: { entity: corpId } });
			}
		});
	}

	// Large-tier policy states
	let hiringPolicy = $state('normal');
	let salaryBand = $state('market');
	let headcountTarget = $state(50);

	function setHiringPolicy(val: string) {
		hiringPolicy = val;
		gameCommand({ SetPolicy: { corporation: $playerCorp?.id ?? 0, policy: 'hiring_policy', value: val } });
	}

	function setSalaryBand(val: string) {
		salaryBand = val;
		gameCommand({ SetPolicy: { corporation: $playerCorp?.id ?? 0, policy: 'salary_band', value: val } });
	}

	function setHeadcountTarget(val: number) {
		headcountTarget = val;
		gameCommand({ SetPolicy: { corporation: $playerCorp?.id ?? 0, policy: 'headcount_target', value: String(val) } });
	}

	let moralePercent = $derived(Math.round(($playerCorp?.morale ?? 0) * 100));
	let moraleClass = $derived(moralePercent >= 70 ? 'green' : moralePercent >= 40 ? 'amber' : 'red');
	let maintenanceSpeed = $derived(Math.min(100, ($playerCorp?.employee_count ?? 0) * 5));
	let researchSpeed = $derived(Math.min(100, ($playerCorp?.employee_count ?? 0) * 4));
	let constructionSpeed = $derived(Math.min(100, ($playerCorp?.employee_count ?? 0) * 3));

	// Infrastructure count: prefer corp-specific count, fall back to world-level count
	let infraCount = $derived($playerCorp?.infrastructure_count ?? $worldInfo.infra_node_count ?? 0);

	// Staffing ratio for medium/large
	let staffPerNode = $derived(
		infraCount > 0
			? ($playerCorp?.employee_count ?? 0) / infraCount
			: 0
	);
	let staffingStatus = $derived(
		staffPerNode >= 3 ? 'overstaffed' : staffPerNode >= 1.5 ? 'optimal' : staffPerNode >= 0.5 ? 'understaffed' : 'critical'
	);

	// Cost per employee estimate (based on total cost / employees)
	let costPerEmployee = $derived(
		($playerCorp?.employee_count ?? 0) > 0
			? ($playerCorp?.cost_per_tick ?? 0) * 0.4 / ($playerCorp?.employee_count ?? 1) // ~40% of costs are salary
			: 0
	);

	// Revenue per employee
	let revenuePerEmployee = $derived(
		($playerCorp?.employee_count ?? 0) > 0
			? ($playerCorp?.revenue_per_tick ?? 0) / ($playerCorp?.employee_count ?? 1)
			: 0
	);

	// Team breakdown (simulated departments based on headcount)
	let teamBreakdown = $derived(() => {
		const total = $playerCorp?.employee_count ?? 0;
		if (total === 0) return [];
		// Approximate team distribution
		const ops = Math.max(1, Math.floor(total * 0.4));
		const eng = Math.max(1, Math.floor(total * 0.25));
		const sales = Math.max(0, Math.floor(total * 0.2));
		const admin = Math.max(0, total - ops - eng - sales);
		return [
			{ name: 'Operations', count: ops, color: 'green' },
			{ name: 'Engineering', count: eng, color: 'blue' },
			{ name: 'Sales', count: sales, color: 'amber' },
			{ name: 'Administration', count: admin, color: 'muted' },
		].filter(t => t.count > 0);
	});
</script>

<div class="panel" role="region" aria-label={$tr('panels.workforce')}>
	<!-- Tier indicator -->
	<div class="tier-badge-row">
		<span class="tier-badge tier-{$companyTier}">{$companyTierLabel}</span>
		<span class="tier-detail mono">{$playerCorp?.employee_count ?? 0} employees</span>
	</div>

	<!-- OVERVIEW — all tiers -->
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
			{#if $companyTier !== 'small'}
				<div class="stat">
					<span class="label">Staff/Node Ratio</span>
					<span class="value mono" class:green={staffingStatus === 'optimal' || staffingStatus === 'overstaffed'} class:amber={staffingStatus === 'understaffed'} class:red={staffingStatus === 'critical'}>
						{staffPerNode.toFixed(1)}x
					</span>
				</div>
				<div class="stat">
					<span class="label">Staffing</span>
					<span class="value" class:green={staffingStatus === 'optimal'} class:amber={staffingStatus === 'understaffed' || staffingStatus === 'overstaffed'} class:red={staffingStatus === 'critical'}>
						{staffingStatus === 'optimal' ? 'Optimal' : staffingStatus === 'overstaffed' ? 'Overstaffed' : staffingStatus === 'understaffed' ? 'Understaffed' : 'Critical'}
					</span>
				</div>
			{/if}
		</div>
	</div>

	{#if ($playerCorp?.employee_count ?? 0) === 0}
		<div class="section">
			<p class="empty-hint">No employees yet — hire your first team to boost operations!</p>
		</div>
	{/if}

	<!-- SMALL TIER: Individual hire/fire with per-person stats -->
	{#if $companyTier === 'small'}
		<div class="section">
			<h3>{$tr('panels.staffing')}</h3>
			<div class="action-row">
				<button class="action-btn hire" onclick={hire} use:tooltip={() => `Hire a technician\nCurrent staff: ${$playerCorp?.employee_count ?? 0}\nMore employees = faster maintenance, research, and construction`}>{$tr('panels.hire')}</button>
				<button class="action-btn fire" onclick={fire} disabled={($playerCorp?.employee_count ?? 0) <= 1} use:tooltip={() => `Fire an employee to reduce salary costs\nCurrent staff: ${$playerCorp?.employee_count ?? 0}${($playerCorp?.employee_count ?? 0) <= 1 ? '\nCannot fire last employee' : ''}`}>{$tr('panels.fire')}</button>
			</div>
			<p class="hint">{$tr('panels.staffing_hint')}</p>
		</div>

		{#if ($playerCorp?.employee_count ?? 0) > 0}
			<div class="section">
				<h3>Per-Employee Stats</h3>
				<div class="stat-grid">
					<div class="stat">
						<span class="label">Est. Salary</span>
						<span class="value mono red">{formatMoney(costPerEmployee)}/tick</span>
					</div>
					<div class="stat">
						<span class="label">Revenue/Head</span>
						<span class="value mono green">{formatMoney(revenuePerEmployee)}/tick</span>
					</div>
				</div>
			</div>
		{/if}
	{/if}

	<!-- MEDIUM TIER: Team management, bulk hiring, morale by team -->
	{#if $companyTier === 'medium'}
		<div class="section">
			<h3>Team Management</h3>
			<div class="team-grid">
				{#each teamBreakdown() as team}
					<div class="team-card">
						<div class="team-header">
							<span class="team-name">{team.name}</span>
							<span class="team-count mono {team.color}">{team.count}</span>
						</div>
						<div class="team-bar-bg">
							<div class="team-bar-fill {team.color}-bar" style="width: {(team.count / ($playerCorp?.employee_count ?? 1)) * 100}%"></div>
						</div>
					</div>
				{/each}
			</div>
		</div>

		<div class="section">
			<h3>Bulk Staffing</h3>
			<div class="bulk-row">
				<label class="bulk-label">
					Hire/Fire Count:
					<input type="range" min={1} max={20} step={1} bind:value={bulkHireCount} />
					<span class="mono">{bulkHireCount}</span>
				</label>
			</div>
			<div class="action-row">
				<button class="action-btn hire" onclick={bulkHire} use:tooltip={() => `Hire ${bulkHireCount} employees at once\nWill increase salary costs by ~${formatMoney(costPerEmployee * bulkHireCount)}/tick`}>+ Hire {bulkHireCount}</button>
				<button class="action-btn fire" onclick={bulkFire} disabled={($playerCorp?.employee_count ?? 0) <= 1} use:tooltip={() => `Fire ${Math.min(bulkHireCount, ($playerCorp?.employee_count ?? 0) - 1)} employees\nWill reduce salary costs`}>- Fire {Math.min(bulkHireCount, ($playerCorp?.employee_count ?? 0) - 1)}</button>
			</div>
			<div class="cost-preview">
				<span class="muted">Salary cost/employee:</span>
				<span class="mono red">{formatMoney(costPerEmployee)}/tick</span>
			</div>
		</div>

		<div class="section">
			<h3>Productivity by Team</h3>
			<div class="productivity-grid">
				<div class="prod-item">
					<span class="prod-label">Operations Efficiency</span>
					<span class="mono" class:green={moralePercent >= 70} class:amber={moralePercent >= 40 && moralePercent < 70} class:red={moralePercent < 40}>{Math.min(100, moralePercent + 20)}%</span>
				</div>
				<div class="prod-item">
					<span class="prod-label">Engineering Output</span>
					<span class="mono" class:green={moralePercent >= 60} class:amber={moralePercent >= 30 && moralePercent < 60} class:red={moralePercent < 30}>{Math.min(100, moralePercent + 10)}%</span>
				</div>
				<div class="prod-item">
					<span class="prod-label">Revenue/Employee</span>
					<span class="mono green">{formatMoney(revenuePerEmployee)}</span>
				</div>
			</div>
		</div>
	{/if}

	<!-- LARGE TIER: Department overview, policy settings, headcount targets -->
	{#if $companyTier === 'large'}
		<div class="section">
			<h3>Department Overview</h3>
			<div class="dept-overview">
				{#each teamBreakdown() as team}
					<div class="dept-row">
						<span class="dept-name">{team.name}</span>
						<div class="dept-bar-bg">
							<div class="dept-bar-fill {team.color}-bar" style="width: {(team.count / ($playerCorp?.employee_count ?? 1)) * 100}%"></div>
						</div>
						<span class="dept-count mono">{team.count}</span>
						<span class="dept-pct muted">{(team.count / ($playerCorp?.employee_count ?? 1) * 100).toFixed(0)}%</span>
					</div>
				{/each}
			</div>
		</div>

		<div class="section">
			<h3>Workforce Policies</h3>
			<div class="policy-grid">
				<div class="policy-card">
					<span class="policy-card-label">Hiring Policy</span>
					<select class="policy-select" bind:value={hiringPolicy}
						onchange={(e) => setHiringPolicy((e.target as HTMLSelectElement).value)}>
						<option value="normal">Normal Growth</option>
						<option value="freeze">Hiring Freeze</option>
						<option value="aggressive">Aggressive Hiring</option>
						<option value="targeted">Targeted Only</option>
					</select>
					<span class="policy-hint muted">
						{#if hiringPolicy === 'freeze'}No new hires until lifted
						{:else if hiringPolicy === 'aggressive'}Rapid expansion, +50% hiring rate
						{:else if hiringPolicy === 'targeted'}Only critical roles filled
						{:else}Standard hiring as needed
						{/if}
					</span>
				</div>
				<div class="policy-card">
					<span class="policy-card-label">Salary Band</span>
					<select class="policy-select" bind:value={salaryBand}
						onchange={(e) => setSalaryBand((e.target as HTMLSelectElement).value)}>
						<option value="below_market">Below Market (-15%)</option>
						<option value="market">Market Rate</option>
						<option value="above_market">Above Market (+15%)</option>
						<option value="premium">Premium (+30%)</option>
					</select>
					<span class="policy-hint muted">
						{#if salaryBand === 'below_market'}Lower costs, higher turnover risk
						{:else if salaryBand === 'above_market'}Higher retention, +15% salary cost
						{:else if salaryBand === 'premium'}Best retention and morale, +30% cost
						{:else}Competitive rates, standard retention
						{/if}
					</span>
				</div>
			</div>
		</div>

		<div class="section">
			<h3>Headcount Target</h3>
			<div class="headcount-row">
				<span class="policy-label">Target</span>
				<input type="range" min={10} max={500} step={5} bind:value={headcountTarget}
					oninput={(e) => {
						const val = Number((e.target as HTMLInputElement).value);
						setHeadcountTarget(val);
					}} />
				<span class="policy-val mono">{headcountTarget}</span>
			</div>
			<div class="headcount-status">
				<span class="muted">Current: {$playerCorp?.employee_count ?? 0}</span>
				<span class="mono" class:green={($playerCorp?.employee_count ?? 0) >= headcountTarget} class:amber={($playerCorp?.employee_count ?? 0) < headcountTarget}>
					{($playerCorp?.employee_count ?? 0) >= headcountTarget ? 'Target met' : `${headcountTarget - ($playerCorp?.employee_count ?? 0)} to hire`}
				</span>
			</div>
		</div>

		<div class="section">
			<h3>Workforce Economics</h3>
			<div class="stat-grid">
				<div class="stat">
					<span class="label">Total Salary Cost</span>
					<span class="value mono red">{formatMoney(costPerEmployee * ($playerCorp?.employee_count ?? 0))}/tick</span>
				</div>
				<div class="stat">
					<span class="label">Cost/Employee</span>
					<span class="value mono">{formatMoney(costPerEmployee)}/tick</span>
				</div>
				<div class="stat">
					<span class="label">Revenue/Employee</span>
					<span class="value mono green">{formatMoney(revenuePerEmployee)}/tick</span>
				</div>
				<div class="stat">
					<span class="label">Staff ROI</span>
					<span class="value mono" class:green={revenuePerEmployee > costPerEmployee} class:red={revenuePerEmployee <= costPerEmployee}>
						{costPerEmployee > 0 ? ((revenuePerEmployee / costPerEmployee) * 100).toFixed(0) : '---'}%
					</span>
				</div>
			</div>
		</div>
	{/if}

	<!-- WORKFORCE IMPACT — all tiers -->
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

	<!-- MORALE FACTORS — all tiers -->
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
				<span>{$tr('panels.workload')} ({infraCount} infra)</span>
				<span class={($playerCorp?.employee_count ?? 0) > infraCount * 2 ? 'green' : 'amber'}>
					{($playerCorp?.employee_count ?? 0) > infraCount * 2 ? '+10%' : '-5%'}
				</span>
			</div>
			{#if $companyTier === 'large'}
				<div class="factor">
					<span>Salary Band ({salaryBand.replace('_', ' ')})</span>
					<span class={salaryBand === 'premium' ? 'green' : salaryBand === 'above_market' ? 'green' : salaryBand === 'below_market' ? 'red' : 'amber'}>
						{salaryBand === 'premium' ? '+15%' : salaryBand === 'above_market' ? '+8%' : salaryBand === 'below_market' ? '-10%' : '+0%'}
					</span>
				</div>
			{/if}
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
	.blue { color: var(--blue); }
	.muted { color: var(--text-muted); font-size: 11px; }

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

	/* ── Small Tier: Staffing ────────────────────────── */

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

	/* ── Medium Tier: Team Management ────────────────── */

	.team-grid {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.team-card {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 6px 8px;
		border-radius: var(--radius-sm);
		background: rgba(17, 24, 39, 0.3);
	}

	.team-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.team-name {
		font-size: 12px;
		color: var(--text-primary);
	}

	.team-count {
		font-size: 13px;
	}

	.team-bar-bg {
		height: 4px;
		background: var(--bg-surface);
		border-radius: 2px;
		overflow: hidden;
	}

	.team-bar-fill {
		height: 100%;
		border-radius: 2px;
		transition: width 0.3s;
	}

	.green-bar { background: var(--green); }
	.blue-bar { background: var(--blue); }
	.amber-bar { background: var(--amber); }
	.muted-bar { background: var(--text-muted); }

	.bulk-row {
		margin-bottom: 8px;
	}

	.bulk-label {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 12px;
		color: var(--text-muted);
	}

	.bulk-label input[type='range'] {
		flex: 1;
		accent-color: var(--blue);
	}

	.cost-preview {
		display: flex;
		justify-content: space-between;
		padding: 4px 0;
		margin-top: 4px;
	}

	.productivity-grid {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.prod-item {
		display: flex;
		justify-content: space-between;
		padding: 4px 0;
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
	}

	.prod-label {
		font-size: 12px;
		color: var(--text-muted);
	}

	/* ── Large Tier: Department Overview ──────────────── */

	.dept-overview {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.dept-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.dept-name {
		font-size: 12px;
		color: var(--text-primary);
		min-width: 90px;
	}

	.dept-bar-bg {
		flex: 1;
		height: 6px;
		background: var(--bg-surface);
		border-radius: 3px;
		overflow: hidden;
	}

	.dept-bar-fill {
		height: 100%;
		border-radius: 3px;
		transition: width 0.3s;
	}

	.dept-count {
		font-size: 12px;
		color: var(--text-primary);
		min-width: 24px;
		text-align: right;
	}

	.dept-pct {
		min-width: 30px;
		text-align: right;
	}

	/* ── Large Tier: Policies ─────────────────────────── */

	.policy-grid {
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

	.policy-card-label {
		font-size: 11px;
		font-weight: 700;
		color: var(--text-primary);
		text-transform: uppercase;
		letter-spacing: 0.4px;
	}

	.policy-select {
		width: 100%;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 5px 6px;
		border-radius: var(--radius-sm);
		font-size: 12px;
		font-family: var(--font-sans);
	}

	.policy-hint {
		font-size: 10px;
		line-height: 1.3;
	}

	.policy-label {
		font-size: 12px;
		color: var(--text-muted);
		min-width: 60px;
	}

	.policy-val {
		font-size: 12px;
		color: var(--text-secondary);
		min-width: 30px;
		text-align: right;
	}

	.headcount-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px 0;
	}

	.headcount-row input[type='range'] {
		flex: 1;
		accent-color: var(--blue);
	}

	.headcount-status {
		display: flex;
		justify-content: space-between;
		padding: 4px 0;
		margin-top: 4px;
	}

	/* ── Impact Bars ─────────────────────────────────── */

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

	/* ── Morale Factors ──────────────────────────────── */

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
