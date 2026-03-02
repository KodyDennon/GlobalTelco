<script lang="ts">
	import { playerCorp, formatMoney, allCorporations, worldInfo } from '$lib/stores/gameState';
	import { gameCommand } from '$lib/game/commandRouter';
	import { tooltip } from '$lib/ui/tooltip';
	import * as bridge from '$lib/wasm/bridge';

	interface Lawsuit {
		id: number;
		plaintiff: string;
		plaintiff_id: number;
		defendant: string;
		defendant_id: number;
		lawsuit_type: string;
		damages: number;
		status: 'Active' | 'Settled' | 'Dismissed' | 'Won' | 'Lost';
		filed_tick: number;
	}

	const LAWSUIT_TYPES = [
		{ value: 'PatentInfringement', label: 'Patent Infringement' },
		{ value: 'OwnershipDispute', label: 'Ownership Dispute' },
		{ value: 'SabotageClaim', label: 'Sabotage Claim' },
		{ value: 'RegulatoryComplaint', label: 'Regulatory Complaint' }
	];

	let lawsuits: Lawsuit[] = $state([]);

	// Load lawsuits from bridge (every 5th tick — Phase 5 optimization)
	$effect(() => {
		const corp = $playerCorp;
		const tick = $worldInfo.tick;
		if (tick % 5 !== 0) return;
		if (!corp) return;
		const raw = bridge.getLawsuits(corp.id);
		lawsuits = raw.map((l) => ({
			id: l.id,
			plaintiff: l.plaintiff_name,
			plaintiff_id: l.plaintiff,
			defendant: l.defendant_name,
			defendant_id: l.defendant,
			lawsuit_type: l.lawsuit_type,
			damages: l.damages_claimed,
			status: l.status as Lawsuit['status'],
			filed_tick: l.filed_tick,
		}));
	});
	let showFileForm = $state(false);

	// File lawsuit form fields
	let targetCorp = $state(0);
	let lawsuitType = $state('PatentInfringement');
	let damagesAmount = $state(100000);

	// Validation
	let damagesValid = $derived(damagesAmount >= 10000 && damagesAmount <= 100000000);
	let formValid = $derived(targetCorp > 0 && lawsuitType.length > 0 && damagesValid);

	let aiCorps = $derived($allCorporations.filter((c) => !c.is_player));
	let activeLawsuits = $derived(lawsuits.filter((l) => l.status === 'Active'));
	let resolvedLawsuits = $derived(lawsuits.filter((l) => l.status !== 'Active'));

	// Summary stats
	let totalDamagesClaimed = $derived(
		activeLawsuits
			.filter((l) => l.plaintiff_id === ($playerCorp?.id ?? 0))
			.reduce((s, l) => s + l.damages, 0)
	);
	let totalDamagesFaced = $derived(
		activeLawsuits
			.filter((l) => l.defendant_id === ($playerCorp?.id ?? 0))
			.reduce((s, l) => s + l.damages, 0)
	);

	function fileLawsuit() {
		const corp = $playerCorp;
		if (!corp || !formValid) return;
		gameCommand({
			FileLawsuit: {
				defendant: targetCorp,
				lawsuit_type: lawsuitType,
				damages: damagesAmount
			}
		});
		showFileForm = false;
		targetCorp = 0;
		lawsuitType = 'PatentInfringement';
		damagesAmount = 100000;
	}

	function settleLawsuit(id: number) {
		gameCommand({ SettleLawsuit: { lawsuit_id: id } });
		lawsuits = lawsuits.map((l) =>
			l.id === id ? { ...l, status: 'Settled' as const } : l
		);
	}

	function defendLawsuit(id: number) {
		gameCommand({ DefendLawsuit: { lawsuit_id: id } });
	}

	function lawsuitTypeLabel(type: string): string {
		return LAWSUIT_TYPES.find((t) => t.value === type)?.label ?? type;
	}

	function statusClass(status: string): string {
		if (status === 'Active') return 'status-active';
		if (status === 'Won') return 'status-won';
		if (status === 'Settled') return 'status-settled';
		return 'status-lost';
	}
</script>

<div class="panel">
	<div class="section">
		<h3>Summary</h3>
		<div class="stat-row">
			<span class="muted">Active cases</span>
			<span class="mono">{activeLawsuits.length}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Damages claimed</span>
			<span class="mono green">{formatMoney(totalDamagesClaimed)}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Damages faced</span>
			<span class="mono red">{formatMoney(totalDamagesFaced)}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Resolved cases</span>
			<span class="mono">{resolvedLawsuits.length}</span>
		</div>
	</div>

	<div class="section">
		<div class="section-hdr">
			<h3>File Lawsuit</h3>
			<button
				class="action-btn"
				onclick={() => (showFileForm = !showFileForm)}
				use:tooltip={'File a lawsuit against a competitor\nClaiming damages takes time and legal fees'}
			>
				{showFileForm ? 'Cancel' : '+ File'}
			</button>
		</div>
		{#if showFileForm}
			<div class="file-form">
				<select bind:value={targetCorp} aria-label="Select defendant corporation">
					<option value={0}>Select defendant...</option>
					{#each aiCorps as corp}
						<option value={corp.id}>{corp.name}</option>
					{/each}
				</select>

				<label class="form-field">
					<span class="field-label">Lawsuit Type</span>
					<select bind:value={lawsuitType} aria-label="Lawsuit type">
						{#each LAWSUIT_TYPES as lt}
							<option value={lt.value}>{lt.label}</option>
						{/each}
					</select>
				</label>

				<label class="form-field">
					<span class="field-label">
						Damages Claimed
						<span class="field-value mono">{formatMoney(damagesAmount)}</span>
					</span>
					<input type="range" min={10000} max={100000000} step={10000} bind:value={damagesAmount} />
					{#if !damagesValid}
						<span class="field-error">$10K - $100M</span>
					{/if}
				</label>

				<div class="preview">
					<span class="preview-label">Preview</span>
					<div class="preview-row">
						<span class="muted">Defendant:</span>
						<span class="mono">{aiCorps.find((c) => c.id === targetCorp)?.name ?? 'None'}</span>
					</div>
					<div class="preview-row">
						<span class="muted">Type:</span>
						<span class="mono">{lawsuitTypeLabel(lawsuitType)}</span>
					</div>
					<div class="preview-row">
						<span class="muted">Damages:</span>
						<span class="mono red">{formatMoney(damagesAmount)}</span>
					</div>
				</div>

				<button
					class="confirm-btn"
					onclick={fileLawsuit}
					disabled={!formValid}
					use:tooltip={'File the lawsuit\nLegal proceedings will begin next tick'}
				>
					File Lawsuit
				</button>
			</div>
		{/if}
	</div>

	{#if activeLawsuits.length > 0}
		<div class="section">
			<h3>Active Cases ({activeLawsuits.length})</h3>
			{#each activeLawsuits as lawsuit}
				<div class="lawsuit-card">
					<div class="lawsuit-info">
						<div class="lawsuit-type">{lawsuitTypeLabel(lawsuit.lawsuit_type)}</div>
						<div class="lawsuit-parties">
							<span>{lawsuit.plaintiff}</span>
							<span class="arrow">vs</span>
							<span>{lawsuit.defendant}</span>
						</div>
						<div class="lawsuit-terms">
							<span class="mono red">{formatMoney(lawsuit.damages)}</span>
							<span class="muted">Filed tick {lawsuit.filed_tick}</span>
						</div>
					</div>
					<div class="lawsuit-actions">
						{#if lawsuit.defendant_id === ($playerCorp?.id ?? 0)}
							<button
								class="defend-btn"
								onclick={() => defendLawsuit(lawsuit.id)}
								use:tooltip={'Contest the lawsuit in court\nRequires legal team resources'}
							>
								Defend
							</button>
							<button
								class="settle-btn"
								onclick={() => settleLawsuit(lawsuit.id)}
								use:tooltip={'Settle out of court\nPay reduced damages to end the case'}
							>
								Settle
							</button>
						{:else}
							<span class="badge plaintiff">Plaintiff</span>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	{/if}

	<div class="section">
		<h3>Resolved Cases ({resolvedLawsuits.length})</h3>
		{#each resolvedLawsuits as lawsuit}
			<div class="lawsuit-card resolved">
				<div class="lawsuit-info">
					<div class="lawsuit-type">{lawsuitTypeLabel(lawsuit.lawsuit_type)}</div>
					<div class="lawsuit-parties">
						<span>{lawsuit.plaintiff}</span>
						<span class="arrow">vs</span>
						<span>{lawsuit.defendant}</span>
					</div>
					<div class="lawsuit-terms">
						<span class="mono">{formatMoney(lawsuit.damages)}</span>
					</div>
				</div>
				<span class="badge {statusClass(lawsuit.status)}">{lawsuit.status}</span>
			</div>
		{/each}
		{#if resolvedLawsuits.length === 0 && activeLawsuits.length === 0}
			<div class="empty">No lawsuits filed. Use legal action to protect patents and dispute sabotage.</div>
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

	.file-form {
		display: flex;
		flex-direction: column;
		gap: 6px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		padding: 10px;
	}

	.file-form select,
	.file-form input {
		background: rgba(17, 24, 39, 0.8);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 6px 8px;
		border-radius: var(--radius-sm);
		font-size: 12px;
		font-family: var(--font-mono);
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

	.preview {
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

	.lawsuit-card {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px;
		border-radius: var(--radius-sm);
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
	}

	.lawsuit-card:hover {
		background: var(--bg-surface);
	}

	.lawsuit-card.resolved {
		opacity: 0.7;
	}

	.lawsuit-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.lawsuit-type {
		font-weight: 600;
		color: var(--text-primary);
	}

	.lawsuit-parties {
		font-size: 12px;
		display: flex;
		gap: 6px;
		align-items: center;
	}

	.arrow {
		color: var(--text-dim);
		font-style: italic;
		font-size: 11px;
	}

	.lawsuit-terms {
		font-size: 11px;
		display: flex;
		gap: 12px;
	}

	.lawsuit-actions {
		display: flex;
		gap: 4px;
	}

	.defend-btn {
		background: rgba(59, 130, 246, 0.1);
		border: 1px solid rgba(59, 130, 246, 0.3);
		color: var(--blue);
		padding: 4px 10px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 11px;
	}

	.settle-btn {
		background: rgba(245, 158, 11, 0.1);
		border: 1px solid rgba(245, 158, 11, 0.3);
		color: #f59e0b;
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

	.badge.plaintiff {
		background: rgba(59, 130, 246, 0.1);
		color: var(--blue);
	}

	.badge.status-active {
		background: rgba(59, 130, 246, 0.1);
		color: var(--blue);
	}

	.badge.status-won {
		background: var(--green-bg);
		color: var(--green);
	}

	.badge.status-settled {
		background: rgba(245, 158, 11, 0.1);
		color: #f59e0b;
	}

	.badge.status-lost {
		background: var(--red-bg);
		color: var(--red);
	}

	.empty {
		color: var(--text-dim);
		text-align: center;
		padding: 16px;
		font-size: 12px;
	}
</style>
