<script lang="ts">
	import { playerCorp, allCorporations } from '$lib/stores/gameState';
	import { gameCommand } from '$lib/game/commandRouter';
	import { tooltip } from '$lib/ui/tooltip';

	// ── Local UI state for alliances (no bridge query yet) ──────────────────────
	interface Alliance {
		id: number;
		name: string;
		members: string[];
		trust: number;
		revenue_share: number;
		status: 'Active' | 'Proposed';
		proposer_id: number;
	}

	let alliances: Alliance[] = $state([]);
	let showProposeForm = $state(false);

	// Propose form fields
	let proposeTarget = $state(0);
	let allianceName = $state('');
	let revenueShare = $state(10);

	// Validation
	let nameValid = $derived(allianceName.trim().length >= 2 && allianceName.trim().length <= 50);
	let shareValid = $derived(revenueShare >= 5 && revenueShare <= 30);
	let formValid = $derived(proposeTarget > 0 && nameValid && shareValid);

	let aiCorps = $derived($allCorporations.filter((c) => !c.is_player));
	let activeAlliances = $derived(alliances.filter((a) => a.status === 'Active'));
	let proposedAlliances = $derived(alliances.filter((a) => a.status === 'Proposed'));

	function proposeAlliance() {
		const corp = $playerCorp;
		if (!corp || !formValid) return;
		gameCommand({
			ProposeAlliance: {
				target_corp: proposeTarget,
				name: allianceName.trim(),
				revenue_share: revenueShare / 100
			}
		});
		showProposeForm = false;
		proposeTarget = 0;
		allianceName = '';
		revenueShare = 10;
	}

	function acceptAlliance(id: number) {
		gameCommand({ AcceptAlliance: { alliance_id: id } });
		alliances = alliances.map((a) =>
			a.id === id ? { ...a, status: 'Active' as const } : a
		);
	}

	function dissolveAlliance(id: number) {
		gameCommand({ DissolveAlliance: { alliance_id: id } });
		alliances = alliances.filter((a) => a.id !== id);
	}
</script>

<div class="panel">
	<div class="section">
		<h3>Summary</h3>
		<div class="stat-row">
			<span class="muted">Active alliances</span>
			<span class="mono">{activeAlliances.length}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Pending proposals</span>
			<span class="mono" class:warn={proposedAlliances.length > 0}>{proposedAlliances.length}</span>
		</div>
	</div>

	<div class="section">
		<div class="section-hdr">
			<h3>Propose Alliance</h3>
			<button
				class="action-btn"
				onclick={() => (showProposeForm = !showProposeForm)}
				use:tooltip={'Propose a strategic alliance with another corporation\nShare revenue and coordinate infrastructure'}
			>
				{showProposeForm ? 'Cancel' : '+ Propose'}
			</button>
		</div>
		{#if showProposeForm}
			<div class="propose-form">
				<select bind:value={proposeTarget}>
					<option value={0}>Select corporation...</option>
					{#each aiCorps as corp}
						<option value={corp.id}>{corp.name}</option>
					{/each}
				</select>

				<label class="form-field">
					<span class="field-label">Alliance Name</span>
					<input
						type="text"
						bind:value={allianceName}
						placeholder="e.g. Pacific Network Coalition"
						maxlength={50}
					/>
					{#if allianceName.length > 0 && !nameValid}
						<span class="field-error">2-50 characters required</span>
					{/if}
				</label>

				<label class="form-field">
					<span class="field-label">
						Revenue Share
						<span class="field-value mono">{revenueShare}%</span>
					</span>
					<input type="range" min={5} max={30} step={1} bind:value={revenueShare} />
					{#if !shareValid}
						<span class="field-error">5% - 30%</span>
					{/if}
				</label>

				<div class="preview">
					<span class="preview-label">Preview</span>
					<div class="preview-row">
						<span class="muted">Partner:</span>
						<span class="mono">{aiCorps.find((c) => c.id === proposeTarget)?.name ?? 'None'}</span>
					</div>
					<div class="preview-row">
						<span class="muted">Revenue share:</span>
						<span class="mono">{revenueShare}%</span>
					</div>
				</div>

				<button
					class="confirm-btn"
					onclick={proposeAlliance}
					disabled={!formValid}
					use:tooltip={'Send alliance proposal\nThe target corporation will accept or reject based on their strategy'}
				>
					Send Proposal
				</button>
			</div>
		{/if}
	</div>

	{#if proposedAlliances.length > 0}
		<div class="section">
			<h3>Pending Proposals ({proposedAlliances.length})</h3>
			{#each proposedAlliances as alliance}
				<div class="alliance-card proposal">
					<div class="alliance-info">
						<div class="alliance-name">{alliance.name}</div>
						<div class="alliance-members">
							{#each alliance.members as member}
								<span class="member-tag">{member}</span>
							{/each}
						</div>
						<div class="alliance-terms">
							<span class="mono">{(alliance.revenue_share * 100).toFixed(0)}% share</span>
						</div>
					</div>
					<div class="alliance-actions">
						{#if alliance.proposer_id !== ($playerCorp?.id ?? 0)}
							<button
								class="accept-btn"
								onclick={() => acceptAlliance(alliance.id)}
								use:tooltip={'Accept this alliance proposal'}
							>
								Accept
							</button>
						{/if}
						<button
							class="reject-btn"
							onclick={() => dissolveAlliance(alliance.id)}
							use:tooltip={'Reject or withdraw this proposal'}
						>
							Withdraw
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}

	<div class="section">
		<h3>Active Alliances ({activeAlliances.length})</h3>
		{#each activeAlliances as alliance}
			<div class="alliance-card">
				<div class="alliance-info">
					<div class="alliance-name">{alliance.name}</div>
					<div class="alliance-members">
						{#each alliance.members as member}
							<span class="member-tag">{member}</span>
						{/each}
					</div>
					<div class="alliance-terms">
						<span>
							<span class="muted">Trust</span>
							<span class="mono" class:green={alliance.trust >= 0.7} class:warn={alliance.trust < 0.3}>
								{(alliance.trust * 100).toFixed(0)}%
							</span>
						</span>
						<span>
							<span class="muted">Share</span>
							<span class="mono">{(alliance.revenue_share * 100).toFixed(0)}%</span>
						</span>
					</div>
				</div>
				<div class="alliance-actions">
					<button
						class="action-btn danger"
						onclick={() => dissolveAlliance(alliance.id)}
						use:tooltip={'Dissolve this alliance\nMay damage trust with former partner'}
					>
						Dissolve
					</button>
				</div>
			</div>
		{/each}
		{#if activeAlliances.length === 0}
			<div class="empty">No alliances yet. Propose one to share revenue and coordinate strategy.</div>
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

	.warn {
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

	.action-btn.danger {
		color: var(--red);
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

	.alliance-card {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px;
		border-radius: var(--radius-sm);
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
	}

	.alliance-card.proposal {
		background: rgba(59, 130, 246, 0.05);
		border: 1px solid rgba(59, 130, 246, 0.2);
		border-radius: var(--radius-md);
		margin-bottom: 6px;
	}

	.alliance-card:hover {
		background: var(--bg-surface);
	}

	.alliance-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.alliance-name {
		font-weight: 600;
		color: var(--text-primary);
	}

	.alliance-members {
		display: flex;
		gap: 4px;
		flex-wrap: wrap;
	}

	.member-tag {
		font-size: 10px;
		padding: 1px 6px;
		border-radius: var(--radius-sm);
		background: rgba(59, 130, 246, 0.1);
		color: var(--blue);
		border: 1px solid rgba(59, 130, 246, 0.2);
	}

	.alliance-terms {
		font-size: 11px;
		display: flex;
		gap: 12px;
	}

	.alliance-actions {
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

	.empty {
		color: var(--text-dim);
		text-align: center;
		padding: 16px;
		font-size: 12px;
	}
</style>
