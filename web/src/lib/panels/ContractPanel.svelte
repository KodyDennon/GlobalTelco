<script lang="ts">
	import { playerCorp, formatMoney, allCorporations } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import type { ContractInfo } from '$lib/wasm/types';
	import { tr } from '$lib/i18n/index';
	import { tooltip } from '$lib/ui/tooltip';

	let contracts: ContractInfo[] = $state([]);
	let showProposeForm = $state(false);
	let proposeTarget = $state(0);

	// Structured contract term fields
	let contractType = $state<'Transit' | 'Peering' | 'SLA'>('Transit');
	let proposeBandwidth = $state(1000);
	let proposePrice = $state(5000);
	let proposeDuration = $state(100);

	// Direct input mode tracking (toggled per field)
	let bandwidthInputMode = $state(false);
	let priceInputMode = $state(false);
	let durationInputMode = $state(false);

	// Validation ranges
	const BANDWIDTH_MIN = 100;
	const BANDWIDTH_MAX = 100000;
	const PRICE_MIN = 100;
	const PRICE_MAX = 10000000;
	const DURATION_MIN = 10;
	const DURATION_MAX = 1000;

	// Validation
	let bandwidthValid = $derived(proposeBandwidth >= BANDWIDTH_MIN && proposeBandwidth <= BANDWIDTH_MAX);
	let priceValid = $derived(proposePrice >= PRICE_MIN && proposePrice <= PRICE_MAX);
	let durationValid = $derived(proposeDuration >= DURATION_MIN && proposeDuration <= DURATION_MAX);
	let formValid = $derived(proposeTarget > 0 && bandwidthValid && priceValid && durationValid);

	// Computed contract metrics
	let pricePerUnit = $derived(proposeBandwidth > 0 ? proposePrice / proposeBandwidth : 0);
	let totalValue = $derived(proposePrice * proposeDuration);
	let estimatedPenalty = $derived(Math.max(Math.floor(totalValue / 10), 1000));
	let slaTier = $derived(
		proposeBandwidth > 5000 ? { label: '99.5%', tier: 'high' } :
		proposeBandwidth > 1000 ? { label: '99.0%', tier: 'mid' } :
		{ label: '98.0%', tier: 'low' }
	);

	$effect(() => {
		const corp = $playerCorp;
		if (corp) {
			contracts = bridge.getContracts(corp.id);
		}
	});

	function acceptContract(id: number) {
		gameCommand({ AcceptContract: { contract: id } });
		if ($playerCorp) contracts = bridge.getContracts($playerCorp.id);
	}

	function rejectContract(id: number) {
		gameCommand({ RejectContract: { contract: id } });
		if ($playerCorp) contracts = bridge.getContracts($playerCorp.id);
	}

	function proposeContract() {
		const corp = $playerCorp;
		if (!corp || !formValid) return;
		const terms = `type:${contractType},bandwidth:${proposeBandwidth},price:${proposePrice},duration:${proposeDuration}`;
		gameCommand({
			ProposeContract: { from: corp.id, to: proposeTarget, terms }
		});
		showProposeForm = false;
		resetForm();
		if (corp) contracts = bridge.getContracts(corp.id);
	}

	function resetForm() {
		proposeTarget = 0;
		contractType = 'Transit';
		proposeBandwidth = 1000;
		proposePrice = 5000;
		proposeDuration = 100;
		bandwidthInputMode = false;
		priceInputMode = false;
		durationInputMode = false;
	}

	function clampBandwidth() {
		proposeBandwidth = Math.max(BANDWIDTH_MIN, Math.min(BANDWIDTH_MAX, Math.round(proposeBandwidth)));
	}

	function clampPrice() {
		proposePrice = Math.max(PRICE_MIN, Math.min(PRICE_MAX, Math.round(proposePrice)));
	}

	function clampDuration() {
		proposeDuration = Math.max(DURATION_MIN, Math.min(DURATION_MAX, Math.round(proposeDuration)));
	}

	let activeContracts = $derived(contracts.filter((c) => c.status === 'Active'));
	let proposedContracts = $derived(contracts.filter((c) => c.status === 'Proposed'));
	let revenueContracts = $derived(activeContracts.filter((c) => c.from === ($playerCorp?.id ?? 0)));
	let expenseContracts = $derived(activeContracts.filter((c) => c.to === ($playerCorp?.id ?? 0)));
	let contractRevenue = $derived(revenueContracts.reduce((s, c) => s + c.price_per_tick, 0));
	let contractCost = $derived(expenseContracts.reduce((s, c) => s + c.price_per_tick, 0));
	let aiCorps = $derived($allCorporations.filter((c) => !c.is_player));

	// Interconnection metrics
	let peeringCount = $derived(activeContracts.filter(c => c.contract_type === 'Peering').length);
	let transitCount = $derived(activeContracts.filter(c => c.contract_type === 'Transit' || c.contract_type === 'SLA').length);
	let transitRevenueTotal = $derived(activeContracts.reduce((s, c) => s + (c.transit_revenue ?? 0), 0));
	let transitCostTotal = $derived(activeContracts.reduce((s, c) => s + (c.transit_cost ?? 0), 0));
</script>

<div class="panel" aria-label="Contracts panel">
	<div class="section">
		<h3>Interconnection Status</h3>
		<div class="stat-row">
			<span class="muted">Peering Agreements</span>
			<span class="mono">{peeringCount}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Transit Agreements</span>
			<span class="mono">{transitCount}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Transit Revenue</span>
			<span class="mono green">{formatMoney(transitRevenueTotal)}/tick</span>
		</div>
		<div class="stat-row">
			<span class="muted">Transit Cost</span>
			<span class="mono red">{formatMoney(transitCostTotal)}/tick</span>
		</div>
	</div>

	<div class="section">
		<h3>{$tr('panels.summary')}</h3>
		<div class="stat-row">
			<span class="muted">{$tr('panels.active')}</span>
			<span class="mono">{activeContracts.length}</span>
		</div>
		<div class="stat-row">
			<span class="muted">{$tr('panels.revenue_from_contracts')}</span>
			<span class="mono green">{formatMoney(contractRevenue)}/tick</span>
		</div>
		<div class="stat-row">
			<span class="muted">{$tr('panels.contract_expenses')}</span>
			<span class="mono red">{formatMoney(contractCost)}/tick</span>
		</div>
	</div>

	<div class="section">
		<div class="section-hdr">
			<h3>Propose Contract</h3>
			<button class="action-btn" onclick={() => (showProposeForm = !showProposeForm)} aria-expanded={showProposeForm} aria-label={showProposeForm ? 'Cancel contract proposal' : 'Propose a new contract'} use:tooltip={'Propose a bandwidth contract with another corporation\nContracts provide recurring revenue'}>
				{showProposeForm ? 'Cancel' : '+ Propose'}
			</button>
		</div>
		{#if showProposeForm}
			<div class="propose-form">
				<div class="form-row">
					<label class="form-field">
						<span class="field-label">Target Corporation</span>
						<select bind:value={proposeTarget} aria-label="Target corporation for contract">
							<option value={0}>Select corporation...</option>
							{#each aiCorps as corp}
								<option value={corp.id}>{corp.name}</option>
							{/each}
						</select>
					</label>

					<label class="form-field">
						<span class="field-label">Contract Type</span>
						<select bind:value={contractType} aria-label="Contract type">
							<option value="Transit">Transit</option>
							<option value="Peering">Peering</option>
							<option value="SLA">SLA</option>
						</select>
					</label>
				</div>

				<label class="form-field">
					<span class="field-label">
						Bandwidth (Mbps)
						{#if bandwidthInputMode}
							<input
								type="number"
								class="inline-input"
								bind:value={proposeBandwidth}
								onblur={clampBandwidth}
								min={BANDWIDTH_MIN}
								max={BANDWIDTH_MAX}
								step={100}
								aria-label="Bandwidth exact value"
							/>
						{:else}
							<button class="field-value mono clickable" onclick={() => (bandwidthInputMode = true)} aria-label="Click to enter exact bandwidth value">{proposeBandwidth.toLocaleString()}</button>
						{/if}
					</span>
					<input type="range" min={BANDWIDTH_MIN} max={BANDWIDTH_MAX} step={100} bind:value={proposeBandwidth} aria-label="Bandwidth slider" />
					<div class="field-range">
						<span>{BANDWIDTH_MIN.toLocaleString()}</span>
						<span>{BANDWIDTH_MAX.toLocaleString()}</span>
					</div>
					{#if !bandwidthValid}
						<span class="field-error">{BANDWIDTH_MIN.toLocaleString()} - {BANDWIDTH_MAX.toLocaleString()}</span>
					{/if}
				</label>

				<label class="form-field">
					<span class="field-label">
						Price per tick
						{#if priceInputMode}
							<input
								type="number"
								class="inline-input"
								bind:value={proposePrice}
								onblur={clampPrice}
								min={PRICE_MIN}
								max={PRICE_MAX}
								step={100}
								aria-label="Price exact value"
							/>
						{:else}
							<button class="field-value mono clickable" onclick={() => (priceInputMode = true)} aria-label="Click to enter exact price value">{formatMoney(proposePrice)}</button>
						{/if}
					</span>
					<input type="range" min={PRICE_MIN} max={PRICE_MAX} step={100} bind:value={proposePrice} aria-label="Price per tick slider" />
					<div class="field-range">
						<span>{formatMoney(PRICE_MIN)}</span>
						<span>{formatMoney(PRICE_MAX)}</span>
					</div>
					{#if !priceValid}
						<span class="field-error">{formatMoney(PRICE_MIN)} - {formatMoney(PRICE_MAX)}</span>
					{/if}
				</label>

				<label class="form-field">
					<span class="field-label">
						Duration (ticks)
						{#if durationInputMode}
							<input
								type="number"
								class="inline-input"
								bind:value={proposeDuration}
								onblur={clampDuration}
								min={DURATION_MIN}
								max={DURATION_MAX}
								step={10}
								aria-label="Duration exact value"
							/>
						{:else}
							<button class="field-value mono clickable" onclick={() => (durationInputMode = true)} aria-label="Click to enter exact duration value">{proposeDuration}</button>
						{/if}
					</span>
					<input type="range" min={DURATION_MIN} max={DURATION_MAX} step={10} bind:value={proposeDuration} aria-label="Duration slider" />
					<div class="field-range">
						<span>{DURATION_MIN}</span>
						<span>{DURATION_MAX}</span>
					</div>
					{#if !durationValid}
						<span class="field-error">{DURATION_MIN} - {DURATION_MAX.toLocaleString()} ticks</span>
					{/if}
				</label>

				<div class="contract-preview">
					<span class="preview-label">Contract Preview</span>
					<div class="preview-row">
						<span class="muted">Type:</span>
						<span class="mono">{contractType}</span>
					</div>
					<div class="preview-row">
						<span class="muted">Bandwidth:</span>
						<span class="mono">{proposeBandwidth.toLocaleString()} Mbps</span>
					</div>
					<div class="preview-row">
						<span class="muted">Price/tick:</span>
						<span class="mono green">{formatMoney(proposePrice)}</span>
					</div>
					<div class="preview-row">
						<span class="muted">Duration:</span>
						<span class="mono">{proposeDuration} ticks</span>
					</div>
					<div class="preview-divider"></div>
					<div class="preview-row">
						<span class="muted">Total value:</span>
						<span class="mono green">{formatMoney(totalValue)}</span>
					</div>
					<div class="preview-row">
						<span class="muted">Price/unit:</span>
						<span class="mono">{formatMoney(pricePerUnit)}/Mbps</span>
					</div>
					<div class="preview-row">
						<span class="muted">Breach penalty:</span>
						<span class="mono amber">{formatMoney(estimatedPenalty)}</span>
					</div>
					<div class="preview-row">
						<span class="muted">SLA target:</span>
						<span class="mono sla-{slaTier.tier}">{slaTier.label} uptime</span>
					</div>
				</div>

				<button class="confirm-btn" onclick={proposeContract} disabled={!formValid} use:tooltip={'Send contract proposal\nThe target corporation will accept or reject based on their strategy'}>Send Proposal</button>
			</div>
		{/if}
	</div>

	{#if proposedContracts.length > 0}
		<div class="section">
			<h3>{$tr('panels.pending_proposals', { count: proposedContracts.length })}</h3>
			{#each proposedContracts as contract}
				<div class="contract-card proposal">
					<div class="contract-info">
						<div class="contract-type">{contract.contract_type}</div>
						<div class="contract-parties">
							<span>{contract.from_name}</span>
							<span class="arrow">&rarr;</span>
							<span>{contract.to_name}</span>
						</div>
						<div class="contract-terms">
							<span class="mono">{formatMoney(contract.price_per_tick)}/tick</span>
							<span class="muted">{$tr('panels.cap')}: {contract.capacity.toFixed(0)}</span>
						</div>
					</div>
					<div class="contract-actions">
						<button class="accept-btn" onclick={() => acceptContract(contract.id)} use:tooltip={() => `Accept contract from ${contract.from_name}\n${formatMoney(contract.price_per_tick)}/tick for ${contract.capacity.toFixed(0)} bandwidth`}>{$tr('panels.accept')}</button>
						<button class="reject-btn" onclick={() => rejectContract(contract.id)} use:tooltip={'Reject this proposal \u2014 no penalty'}>{$tr('panels.reject')}</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}

	<div class="section">
		<h3>{$tr('panels.active_contracts', { count: activeContracts.length })}</h3>
		{#each activeContracts as contract}
			<div class="contract-card">
				<div class="contract-info">
					<div class="contract-header">
						<span class="contract-type type-{contract.contract_type.toLowerCase()}">{contract.contract_type}</span>
						{#if contract.from === ($playerCorp?.id ?? 0)}
							<span class="badge income">{$tr('panels.income')}</span>
						{:else}
							<span class="badge expense">{$tr('panels.expense')}</span>
						{/if}
					</div>
					<div class="contract-parties">
						<span>{contract.from_name}</span>
						<span class="arrow">&rarr;</span>
						<span>{contract.to_name}</span>
					</div>
					<div class="contract-terms">
						<span class="mono">{formatMoney(contract.price_per_tick)}/tick</span>
						<span class="muted">{$tr('panels.ends_tick', { tick: contract.end_tick })}</span>
					</div>
					{#if contract.traffic_current > 0}
						<div class="contract-traffic">
							<span class="muted">Traffic:</span>
							<span class="mono">{contract.traffic_current.toFixed(0)}</span>
							<div class="traffic-bar">
								<div class="traffic-fill" style="width: {Math.min(contract.traffic_capacity_pct, 100)}%"></div>
							</div>
							<span class="mono muted">{contract.traffic_capacity_pct.toFixed(0)}%</span>
						</div>
					{/if}
					{#if contract.transit_revenue > 0}
						<div class="contract-terms">
							<span class="muted">Transit earned:</span>
							<span class="mono green">{formatMoney(contract.transit_revenue)}/tick</span>
						</div>
					{/if}
					{#if contract.transit_cost > 0}
						<div class="contract-terms">
							<span class="muted">Transit cost:</span>
							<span class="mono red">{formatMoney(contract.transit_cost)}/tick</span>
						</div>
					{/if}
					{#if contract.sla_status}
						<div class="contract-sla">
							<span class="sla-badge sla-{contract.sla_status}">{contract.sla_current_performance.toFixed(1)}%</span>
							<span class="muted">target {contract.sla_target.toFixed(1)}%</span>
						</div>
					{/if}
				</div>
			</div>
		{/each}
		{#if activeContracts.length === 0}
			<div class="empty">{$tr('panels.no_active_contracts')}</div>
		{/if}
	</div>
</div>

<style>
	.panel {
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

	.stat-row {
		display: flex;
		justify-content: space-between;
		padding: 3px 0;
	}

	.muted {
		color: var(--text-muted);
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
		color: var(--amber, #f59e0b);
	}

	.contract-card {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px;
		border-radius: var(--radius-sm);
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
	}

	.contract-card.proposal {
		background: rgba(59, 130, 246, 0.05);
		border: 1px solid rgba(59, 130, 246, 0.2);
		border-radius: var(--radius-md);
		margin-bottom: 6px;
	}

	.contract-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.contract-type {
		font-weight: 600;
		color: var(--text-primary);
	}

	.contract-parties {
		font-size: 12px;
		display: flex;
		gap: 6px;
		align-items: center;
	}

	.arrow {
		color: var(--text-dim);
	}

	.contract-terms {
		font-size: 11px;
		display: flex;
		gap: 12px;
	}

	.contract-sla {
		font-size: 11px;
		display: flex;
		gap: 8px;
		align-items: center;
		margin-top: 2px;
	}

	.sla-badge {
		font-family: var(--font-mono);
		font-size: 10px;
		padding: 1px 6px;
		border-radius: var(--radius-sm);
		font-weight: 600;
	}

	.sla-badge.sla-ok {
		background: var(--green-bg);
		color: var(--green);
	}

	.sla-badge.sla-at_risk {
		background: rgba(245, 158, 11, 0.1);
		color: var(--amber, #f59e0b);
	}

	.sla-badge.sla-breach {
		background: var(--red-bg);
		color: var(--red);
	}

	.contract-actions {
		display: flex;
		gap: 4px;
	}

	.accept-btn {
		background: var(--green-bg);
		border: 1px solid var(--green-border);
		color: var(--green);
		padding: 4px 10px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 11px;
	}

	.reject-btn {
		background: var(--red-bg);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: var(--red);
		padding: 4px 10px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 11px;
	}

	.badge {
		font-size: 10px;
		padding: 2px 8px;
		border-radius: var(--radius-sm);
		font-weight: 600;
	}

	.badge.income {
		background: var(--green-bg);
		color: var(--green);
	}

	.badge.expense {
		background: var(--red-bg);
		color: var(--red);
	}

	.empty {
		color: var(--text-dim);
		text-align: center;
		padding: 16px;
		font-size: 12px;
	}

	.section-hdr {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 8px;
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

	.propose-form {
		display: flex;
		flex-direction: column;
		gap: 10px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		padding: 12px;
	}

	.form-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 10px;
	}

	.propose-form select,
	.propose-form input[type="range"] {
		background: rgba(17, 24, 39, 0.8);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 6px 8px;
		border-radius: var(--radius-sm);
		font-size: 12px;
		font-family: var(--font-mono);
		width: 100%;
		box-sizing: border-box;
	}

	.propose-form input[type="range"] {
		padding: 0;
		height: 6px;
		-webkit-appearance: none;
		appearance: none;
		border: none;
		border-radius: 3px;
		cursor: pointer;
	}

	.propose-form input[type="range"]::-webkit-slider-thumb {
		-webkit-appearance: none;
		width: 14px;
		height: 14px;
		border-radius: 50%;
		background: var(--blue, #3b82f6);
		border: 2px solid rgba(17, 24, 39, 0.8);
		cursor: pointer;
	}

	.confirm-btn {
		background: var(--green-bg);
		border: 1px solid var(--green-border);
		color: var(--green);
		padding: 8px 12px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 12px;
		font-weight: 600;
		margin-top: 4px;
	}

	.confirm-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.confirm-btn:not(:disabled):hover {
		background: rgba(34, 197, 94, 0.15);
	}

	.form-field {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.field-label {
		display: flex;
		justify-content: space-between;
		align-items: center;
		font-size: 11px;
		color: var(--text-muted);
	}

	.field-value {
		color: var(--text-primary);
		font-size: 12px;
	}

	.clickable {
		background: none;
		border: 1px solid transparent;
		padding: 1px 4px;
		cursor: pointer;
		border-radius: var(--radius-sm);
		transition: border-color 0.15s;
	}

	.clickable:hover {
		border-color: var(--border);
		background: rgba(17, 24, 39, 0.5);
	}

	.inline-input {
		width: 90px;
		padding: 2px 4px;
		font-size: 12px;
		font-family: var(--font-mono);
		background: rgba(17, 24, 39, 0.8);
		border: 1px solid var(--blue, #3b82f6);
		color: var(--text-primary);
		border-radius: var(--radius-sm);
		text-align: right;
	}

	.inline-input:focus {
		outline: none;
		border-color: var(--blue, #3b82f6);
		box-shadow: 0 0 0 1px var(--blue, #3b82f6);
	}

	.field-range {
		display: flex;
		justify-content: space-between;
		font-size: 9px;
		color: var(--text-dim);
		font-family: var(--font-mono);
		margin-top: -2px;
	}

	.field-error {
		font-size: 10px;
		color: var(--red);
	}

	.contract-preview {
		background: rgba(17, 24, 39, 0.6);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		padding: 10px 12px;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.preview-label {
		font-size: 10px;
		font-weight: 600;
		color: var(--text-dim);
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin-bottom: 2px;
	}

	.preview-row {
		display: flex;
		justify-content: space-between;
		font-size: 11px;
	}

	.preview-divider {
		border-top: 1px solid rgba(55, 65, 81, 0.3);
		margin: 4px 0;
	}

	.sla-high {
		color: var(--green);
	}

	.sla-mid {
		color: var(--blue, #3b82f6);
	}

	.sla-low {
		color: var(--text-muted);
	}

	.contract-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.type-peering {
		color: var(--green);
	}

	.type-transit {
		color: var(--blue, #3b82f6);
	}

	.type-sla {
		color: var(--amber, #f59e0b);
	}

	.contract-traffic {
		display: flex;
		gap: 6px;
		align-items: center;
		font-size: 11px;
		margin-top: 2px;
	}

	.traffic-bar {
		flex: 1;
		height: 4px;
		background: rgba(55, 65, 81, 0.4);
		border-radius: 2px;
		overflow: hidden;
		min-width: 40px;
	}

	.traffic-fill {
		height: 100%;
		background: var(--blue, #3b82f6);
		border-radius: 2px;
		transition: width 0.3s ease;
	}
</style>
