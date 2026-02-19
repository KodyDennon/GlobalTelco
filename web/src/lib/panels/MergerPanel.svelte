<script lang="ts">
	import { getAcquisitionProposals, getAllCorporations, getPlayerCorpId, processCommand, getCorporationData } from '$lib/wasm/bridge';
	import type { AcquisitionInfo, CorpSummary } from '$lib/wasm/types';
	import { tr } from '$lib/i18n/index';

	let proposals: AcquisitionInfo[] = $state([]);
	let corporations: CorpSummary[] = $state([]);
	let playerId = $state(0);
	let playerCash = $state(0);
	let selectedTarget = $state(0);
	let offerAmount = $state(0);

	function refresh() {
		proposals = getAcquisitionProposals();
		corporations = getAllCorporations();
		playerId = getPlayerCorpId();
		const corp = getCorporationData(playerId);
		playerCash = corp.cash;
	}

	$effect(() => {
		refresh();
		const interval = setInterval(refresh, 2000);
		return () => clearInterval(interval);
	});

	function proposeAcquisition() {
		if (!selectedTarget || offerAmount <= 0) return;
		processCommand({ ProposeAcquisition: { target: selectedTarget, offer: offerAmount } });
		offerAmount = 0;
		selectedTarget = 0;
		refresh();
	}

	function respond(proposalId: number, accept: boolean) {
		processCommand({ RespondToAcquisition: { proposal: proposalId, accept } });
		refresh();
	}

	function formatMoney(val: number): string {
		if (Math.abs(val) >= 1_000_000) return `$${(val / 1_000_000).toFixed(1)}M`;
		if (Math.abs(val) >= 1_000) return `$${(val / 1_000).toFixed(0)}K`;
		return `$${val}`;
	}

	const aiCorps = $derived(corporations.filter((c) => !c.is_player));
	const incomingProposals = $derived(proposals.filter((p) => p.target === playerId && p.status === 'Pending'));
	const outgoingProposals = $derived(proposals.filter((p) => p.acquirer === playerId));
</script>

<div class="panel" role="region" aria-label={$tr('panels.mergers')}>
	<h2>{$tr('panels.mergers')}</h2>

	<section>
		<h3>{$tr('panels.propose_acquisition')}</h3>
		<div class="form">
			<select bind:value={selectedTarget} aria-label={$tr('panels.select_target')}>
				<option value={0}>{$tr('panels.select_target')}</option>
				{#each aiCorps as corp}
					<option value={corp.id}>{corp.name} ({formatMoney(corp.cash)})</option>
				{/each}
			</select>
			<input type="number" bind:value={offerAmount} placeholder={$tr('panels.offer_amount')} aria-label={$tr('panels.offer_amount')} min="1" />
			<button onclick={proposeAcquisition} disabled={!selectedTarget || offerAmount <= 0 || offerAmount > playerCash}>
				{$tr('panels.propose')}
			</button>
		</div>
	</section>

	{#if incomingProposals.length > 0}
		<section>
			<h3>{$tr('panels.incoming_proposals')}</h3>
			{#each incomingProposals as proposal}
				<div class="proposal-card">
					<div class="proposal-info">
						<span class="corp-name">{proposal.acquirer_name}</span>
						<span class="offer">{$tr('panels.offers')} {formatMoney(proposal.offer)}</span>
					</div>
					<div class="actions">
						<button class="accept" onclick={() => respond(proposal.id, true)}>{$tr('panels.accept')}</button>
						<button class="reject" onclick={() => respond(proposal.id, false)}>{$tr('panels.reject')}</button>
					</div>
				</div>
			{/each}
		</section>
	{/if}

	{#if outgoingProposals.length > 0}
		<section>
			<h3>{$tr('panels.your_proposals')}</h3>
			{#each outgoingProposals as proposal}
				<div class="proposal-card">
					<span class="corp-name">{proposal.target_name}</span>
					<span class="offer">{formatMoney(proposal.offer)}</span>
					<span class="status">{proposal.status}</span>
				</div>
			{/each}
		</section>
	{/if}
</div>

<style>
	.panel { padding: 16px; color: var(--text-secondary); }
	h2 { font-size: 16px; margin: 0 0 12px; color: var(--blue-light); }
	h3 { font-size: 13px; color: var(--text-muted); margin: 12px 0 8px; text-transform: uppercase; letter-spacing: 0.05em; }
	section { margin-bottom: 16px; }
	.form { display: flex; flex-direction: column; gap: 8px; }
	.form select, .form input { background: var(--bg-surface); border: 1px solid var(--border); color: var(--text-secondary); padding: 8px; border-radius: var(--radius-sm); font-size: 13px; }
	.form button { background: var(--purple); color: white; border: none; padding: 8px 16px; border-radius: var(--radius-sm); cursor: pointer; font-size: 13px; }
	.form button:disabled { opacity: 0.5; cursor: not-allowed; }
	.form button:hover:not(:disabled) { opacity: 0.85; }
	.proposal-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-md); padding: 10px 12px; margin-bottom: 6px; display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
	.corp-name { font-weight: 600; font-size: 13px; }
	.offer { font-family: var(--font-mono); color: var(--amber); font-size: 13px; }
	.status { font-size: 12px; padding: 2px 6px; border-radius: var(--radius-sm); background: var(--bg-hover); margin-left: auto; }
	.actions { display: flex; gap: 6px; margin-left: auto; }
	.accept { background: var(--green-bg); color: var(--green-light); border: none; padding: 4px 10px; border-radius: var(--radius-sm); cursor: pointer; font-size: 12px; }
	.reject { background: var(--red-bg); color: var(--red-light); border: none; padding: 4px 10px; border-radius: var(--radius-sm); cursor: pointer; font-size: 12px; }
	.accept:hover { opacity: 0.85; }
	.reject:hover { opacity: 0.85; }
</style>
