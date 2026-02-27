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

	// Structured contract term fields (replacing hardcoded string)
	let proposeBandwidth = $state(1000);
	let proposePrice = $state(5000);
	let proposeDuration = $state(100);

	// Validation
	let bandwidthValid = $derived(proposeBandwidth >= 100 && proposeBandwidth <= 100000);
	let priceValid = $derived(proposePrice >= 100 && proposePrice <= 10000000);
	let durationValid = $derived(proposeDuration >= 10 && proposeDuration <= 1000);
	let formValid = $derived(proposeTarget > 0 && bandwidthValid && priceValid && durationValid);
	let pricePerUnit = $derived(proposeBandwidth > 0 ? proposePrice / proposeBandwidth : 0);

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
		const terms = `bandwidth:${proposeBandwidth},price:${proposePrice},duration:${proposeDuration}`;
		gameCommand({
			ProposeContract: { from: corp.id, to: proposeTarget, terms }
		});
		showProposeForm = false;
		proposeTarget = 0;
		proposeBandwidth = 1000;
		proposePrice = 5000;
		proposeDuration = 100;
		if (corp) contracts = bridge.getContracts(corp.id);
	}

	let activeContracts = $derived(contracts.filter((c) => c.status === 'Active'));
	let proposedContracts = $derived(contracts.filter((c) => c.status === 'Proposed'));
	let revenueContracts = $derived(activeContracts.filter((c) => c.from === ($playerCorp?.id ?? 0)));
	let expenseContracts = $derived(activeContracts.filter((c) => c.to === ($playerCorp?.id ?? 0)));
	let contractRevenue = $derived(revenueContracts.reduce((s, c) => s + c.price_per_tick, 0));
	let contractCost = $derived(expenseContracts.reduce((s, c) => s + c.price_per_tick, 0));
	let aiCorps = $derived($allCorporations.filter((c) => !c.is_player));
</script>

<div class="panel" aria-label="Contracts panel">
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
				<select bind:value={proposeTarget} aria-label="Target corporation for contract">
					<option value={0}>Select corporation...</option>
					{#each aiCorps as corp}
						<option value={corp.id}>{corp.name}</option>
					{/each}
				</select>

				<label class="form-field">
					<span class="field-label">
						Bandwidth
						<span class="field-value mono">{proposeBandwidth.toLocaleString()}</span>
					</span>
					<input type="range" min={100} max={100000} step={100} bind:value={proposeBandwidth} />
					{#if !bandwidthValid}
						<span class="field-error">100 - 100,000</span>
					{/if}
				</label>

				<label class="form-field">
					<span class="field-label">
						Price per tick
						<span class="field-value mono">{formatMoney(proposePrice)}</span>
					</span>
					<input type="range" min={100} max={10000000} step={100} bind:value={proposePrice} />
					{#if !priceValid}
						<span class="field-error">$100 - $10M</span>
					{/if}
				</label>

				<label class="form-field">
					<span class="field-label">
						Duration (ticks)
						<span class="field-value mono">{proposeDuration}</span>
					</span>
					<input type="range" min={10} max={1000} step={10} bind:value={proposeDuration} />
					{#if !durationValid}
						<span class="field-error">10 - 1,000 ticks</span>
					{/if}
				</label>

				<div class="contract-preview">
					<span class="preview-label">Preview</span>
					<div class="preview-row">
						<span class="muted">Bandwidth:</span>
						<span class="mono">{proposeBandwidth.toLocaleString()}</span>
					</div>
					<div class="preview-row">
						<span class="muted">Price/tick:</span>
						<span class="mono green">{formatMoney(proposePrice)}</span>
					</div>
					<div class="preview-row">
						<span class="muted">Duration:</span>
						<span class="mono">{proposeDuration} ticks</span>
					</div>
					<div class="preview-row">
						<span class="muted">Total value:</span>
						<span class="mono green">{formatMoney(proposePrice * proposeDuration)}</span>
					</div>
					<div class="preview-row">
						<span class="muted">Price/unit:</span>
						<span class="mono">{formatMoney(pricePerUnit)}/bw</span>
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
							<span class="arrow">→</span>
							<span>{contract.to_name}</span>
						</div>
						<div class="contract-terms">
							<span class="mono">{formatMoney(contract.price_per_tick)}/tick</span>
							<span class="muted">{$tr('panels.cap')}: {contract.capacity.toFixed(0)}</span>
						</div>
					</div>
					<div class="contract-actions">
						<button class="accept-btn" onclick={() => acceptContract(contract.id)} use:tooltip={() => `Accept contract from ${contract.from_name}\n${formatMoney(contract.price_per_tick)}/tick for ${contract.capacity.toFixed(0)} bandwidth`}>{$tr('panels.accept')}</button>
						<button class="reject-btn" onclick={() => rejectContract(contract.id)} use:tooltip={'Reject this proposal — no penalty'}>{$tr('panels.reject')}</button>
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
					<div class="contract-type">{contract.contract_type}</div>
					<div class="contract-parties">
						<span>{contract.from_name}</span>
						<span class="arrow">→</span>
						<span>{contract.to_name}</span>
					</div>
					<div class="contract-terms">
						<span class="mono">{formatMoney(contract.price_per_tick)}/tick</span>
						<span class="muted">{$tr('panels.ends_tick', { tick: contract.end_tick })}</span>
					</div>
				</div>
				<div class="contract-badge">
					{#if contract.from === ($playerCorp?.id ?? 0)}
						<span class="badge income">{$tr('panels.income')}</span>
					{:else}
						<span class="badge expense">{$tr('panels.expense')}</span>
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
		gap: 6px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		padding: 10px;
	}

	.propose-form select,
	.propose-form input {
		background: rgba(17, 24, 39, 0.8);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 6px 8px;
		border-radius: var(--radius-sm);
		font-size: 12px;
		font-family: var(--font-mono);
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

	.confirm-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
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

	.field-error {
		font-size: 10px;
		color: var(--red);
	}

	.contract-preview {
		background: rgba(17, 24, 39, 0.6);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		padding: 8px 10px;
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
</style>
