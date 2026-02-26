<script lang="ts">
	import { playerCorp, formatMoney } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import type { ResearchInfo } from '$lib/wasm/types';
	import { tooltip } from '$lib/ui/tooltip';

	const LICENSE_TYPES = [
		{ value: 'Permanent', label: 'Permanent', description: 'One-time payment, unlimited use' },
		{ value: 'Royalty', label: 'Royalty', description: 'Percentage of revenue per tick' },
		{ value: 'PerUnit', label: 'Per Unit', description: 'Cost per infrastructure using the tech' },
		{ value: 'Lease', label: 'Lease', description: 'Time-limited access, recurring payment' }
	];

	let researchState: ResearchInfo[] = $state([]);

	$effect(() => {
		const corp = $playerCorp;
		if (corp) {
			researchState = bridge.getResearchState();
		}
	});

	// Owned patents: completed research where player is patent owner
	let ownedPatents = $derived(
		researchState.filter(
			(r) => r.patent_owner === ($playerCorp?.id ?? -1) && r.patent_status !== 'None'
		)
	);

	// Patentable: completed by player, no patent filed yet
	let patentable = $derived(
		researchState.filter(
			(r) => r.completed && r.researcher === ($playerCorp?.id ?? -1) && r.patent_status === 'None'
		)
	);

	// Licensable: patents owned by others with a license price set
	let licensable = $derived(
		researchState.filter(
			(r) =>
				r.patent_status !== 'None' &&
				r.patent_owner !== null &&
				r.patent_owner !== ($playerCorp?.id ?? -1) &&
				r.license_price > 0
		)
	);

	// Independent research candidates: patented by others, not yet completed by player
	let independentResearchable = $derived(
		researchState.filter(
			(r) =>
				r.patent_status !== 'None' &&
				r.patent_owner !== null &&
				r.patent_owner !== ($playerCorp?.id ?? -1) &&
				!r.completed
		)
	);

	// License pricing form state
	let pricingPatentId = $state<number | null>(null);
	let licensePrice = $state(50000);
	let licenseType = $state('Royalty');

	let totalLicenseRevenue = $derived(
		ownedPatents.reduce((s, p) => s + p.license_price, 0)
	);

	function filePatent(techId: number) {
		gameCommand({ FilePatent: { tech_id: techId } });
		researchState = bridge.getResearchState();
	}

	function requestLicense(patentId: number) {
		gameCommand({ RequestLicense: { patent_id: patentId } });
	}

	function setLicensePrice(patentId: number) {
		gameCommand({
			SetLicensePrice: {
				patent_id: patentId,
				price: licensePrice,
				license_type: licenseType
			}
		});
		pricingPatentId = null;
		licensePrice = 50000;
		licenseType = 'Royalty';
		researchState = bridge.getResearchState();
	}

	function revokeLicense(licenseId: number) {
		gameCommand({ RevokeLicense: { license_id: licenseId } });
		researchState = bridge.getResearchState();
	}

	function startIndependentResearch(techId: number) {
		gameCommand({ StartIndependentResearch: { tech_id: techId } });
	}
</script>

<div class="panel">
	<div class="section">
		<h3>Summary</h3>
		<div class="stat-row">
			<span class="muted">Owned patents</span>
			<span class="mono green">{ownedPatents.length}</span>
		</div>
		<div class="stat-row">
			<span class="muted">License revenue</span>
			<span class="mono green">{formatMoney(totalLicenseRevenue)}/tick</span>
		</div>
		<div class="stat-row">
			<span class="muted">Patentable techs</span>
			<span class="mono">{patentable.length}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Available licenses</span>
			<span class="mono">{licensable.length}</span>
		</div>
	</div>

	{#if patentable.length > 0}
		<div class="section">
			<h3>File Patents ({patentable.length})</h3>
			{#each patentable as tech}
				<div class="tech-row">
					<div class="tech-info">
						<div class="tech-name">{tech.name}</div>
						<div class="tech-meta">
							<span class="muted">{tech.category_name}</span>
						</div>
					</div>
					<button
						class="action-btn file"
						onclick={() => filePatent(tech.id)}
						use:tooltip={'File a patent on this technology\nEarn license revenue from competitors'}
					>
						File Patent
					</button>
				</div>
			{/each}
		</div>
	{/if}

	<div class="section">
		<h3>Owned Patents ({ownedPatents.length})</h3>
		{#each ownedPatents as patent}
			<div class="patent-card">
				<div class="patent-info">
					<div class="patent-name">{patent.name}</div>
					<div class="patent-meta">
						<span class="muted">{patent.category_name}</span>
						<span class="badge patent-badge">{patent.patent_status}</span>
					</div>
					<div class="patent-stats">
						{#if patent.license_price > 0}
							<span>
								<span class="muted">License</span>
								<span class="mono green">{formatMoney(patent.license_price)}/tick</span>
							</span>
						{:else}
							<span class="muted">No license pricing set</span>
						{/if}
					</div>
				</div>
				<div class="patent-actions">
					{#if pricingPatentId === patent.id}
						<div class="pricing-form">
							<label class="form-field">
								<span class="field-label">
									Price
									<span class="field-value mono">{formatMoney(licensePrice)}</span>
								</span>
								<input type="range" min={1000} max={10000000} step={1000} bind:value={licensePrice} />
							</label>
							<label class="form-field">
								<span class="field-label">Type</span>
								<select bind:value={licenseType}>
									{#each LICENSE_TYPES as lt}
										<option value={lt.value}>{lt.label}</option>
									{/each}
								</select>
							</label>
							<div class="pricing-type-desc muted">
								{LICENSE_TYPES.find((t) => t.value === licenseType)?.description ?? ''}
							</div>
							<div class="pricing-actions">
								<button class="confirm-btn" onclick={() => setLicensePrice(patent.id)}>Set Price</button>
								<button class="cancel-btn" onclick={() => (pricingPatentId = null)}>Cancel</button>
							</div>
						</div>
					{:else}
						<button
							class="action-btn"
							onclick={() => (pricingPatentId = patent.id)}
							use:tooltip={'Set or update license pricing for this patent'}
						>
							Set Price
						</button>
						{#if patent.license_price > 0}
							<button
								class="action-btn danger"
								onclick={() => revokeLicense(patent.id)}
								use:tooltip={'Revoke all active licenses for this patent'}
							>
								Revoke
							</button>
						{/if}
					{/if}
				</div>
			</div>
		{/each}
		{#if ownedPatents.length === 0}
			<div class="empty">No patents owned. Complete research and file patents to earn license revenue.</div>
		{/if}
	</div>

	{#if licensable.length > 0}
		<div class="section">
			<h3>Available Licenses ({licensable.length})</h3>
			{#each licensable as tech}
				<div class="tech-row">
					<div class="tech-info">
						<div class="tech-name">{tech.name}</div>
						<div class="tech-meta">
							<span class="muted">{tech.category_name}</span>
							<span class="muted">Owner: {tech.patent_owner_name ?? 'Unknown'}</span>
						</div>
						<div class="tech-stats">
							<span class="mono">{formatMoney(tech.license_price)}/tick</span>
						</div>
					</div>
					<button
						class="action-btn"
						onclick={() => requestLicense(tech.id)}
						use:tooltip={() => `Request a license for ${tech.name}\nCosts ${formatMoney(tech.license_price)}/tick`}
					>
						Request
					</button>
				</div>
			{/each}
		</div>
	{/if}

	{#if independentResearchable.length > 0}
		<div class="section">
			<h3>Independent Research ({independentResearchable.length})</h3>
			<div class="section-note muted">
				Research patented technologies independently at 2x cost to bypass licensing.
			</div>
			{#each independentResearchable as tech}
				<div class="tech-row">
					<div class="tech-info">
						<div class="tech-name">{tech.name}</div>
						<div class="tech-meta">
							<span class="muted">{tech.category_name}</span>
							<span class="muted">Patent: {tech.patent_owner_name ?? 'Unknown'}</span>
						</div>
						<div class="tech-stats">
							<span>
								<span class="muted">Cost</span>
								<span class="mono red">{formatMoney(tech.total_cost * 2)}</span>
							</span>
							<span class="cost-note muted">(2x multiplier)</span>
						</div>
					</div>
					<button
						class="action-btn"
						onclick={() => startIndependentResearch(tech.id)}
						use:tooltip={() => `Research ${tech.name} independently\nCosts ${formatMoney(tech.total_cost * 2)} (2x normal cost)`}
					>
						Research
					</button>
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

	.tech-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px;
		border-radius: var(--radius-sm);
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
	}

	.tech-row:hover {
		background: var(--bg-surface);
	}

	.tech-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.tech-name {
		font-weight: 600;
		color: var(--text-primary);
	}

	.tech-meta {
		font-size: 11px;
		display: flex;
		gap: 8px;
	}

	.tech-stats {
		font-size: 11px;
		display: flex;
		gap: 8px;
		align-items: center;
	}

	.cost-note {
		font-size: 10px;
	}

	.section-note {
		font-size: 11px;
		margin-bottom: 8px;
	}

	.action-btn {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--blue);
		padding: 4px 10px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 11px;
		font-family: var(--font-mono);
		white-space: nowrap;
	}

	.action-btn:hover {
		background: var(--bg-hover);
	}

	.action-btn.file {
		color: var(--green);
		border-color: var(--green-border);
		background: var(--green-bg);
	}

	.action-btn.danger {
		color: var(--red);
	}

	.patent-card {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		padding: 8px;
		border-radius: var(--radius-sm);
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
	}

	.patent-card:hover {
		background: var(--bg-surface);
	}

	.patent-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.patent-name {
		font-weight: 600;
		color: var(--text-primary);
	}

	.patent-meta {
		font-size: 11px;
		display: flex;
		gap: 8px;
		align-items: center;
	}

	.patent-stats {
		font-size: 11px;
	}

	.patent-actions {
		display: flex;
		flex-direction: column;
		gap: 4px;
		align-items: flex-end;
	}

	.badge {
		font-size: 10px;
		padding: 1px 6px;
		border-radius: var(--radius-sm);
		font-weight: 600;
	}

	.patent-badge {
		background: rgba(139, 92, 246, 0.1);
		color: #8b5cf6;
		border: 1px solid rgba(139, 92, 246, 0.2);
	}

	.pricing-form {
		display: flex;
		flex-direction: column;
		gap: 6px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		padding: 8px;
		min-width: 180px;
	}

	.pricing-form select,
	.pricing-form input {
		background: rgba(17, 24, 39, 0.8);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 4px 6px;
		border-radius: var(--radius-sm);
		font-size: 11px;
		font-family: var(--font-mono);
	}

	.pricing-type-desc {
		font-size: 10px;
		font-style: italic;
	}

	.pricing-actions {
		display: flex;
		gap: 4px;
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

	.confirm-btn {
		background: var(--green-bg);
		border: 1px solid var(--green-border);
		color: var(--green);
		padding: 4px 10px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 11px;
	}

	.cancel-btn {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--text-muted);
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
