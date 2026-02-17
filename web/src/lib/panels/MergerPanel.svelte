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
	.panel { padding: 16px; color: #e5e7eb; }
	h2 { font-size: 16px; margin: 0 0 12px; color: #60a5fa; }
	h3 { font-size: 13px; color: #9ca3af; margin: 12px 0 8px; text-transform: uppercase; letter-spacing: 0.05em; }
	section { margin-bottom: 16px; }
	.form { display: flex; flex-direction: column; gap: 8px; }
	.form select, .form input { background: #1f2937; border: 1px solid #374151; color: #e5e7eb; padding: 8px; border-radius: 4px; font-size: 13px; }
	.form button { background: #7c3aed; color: white; border: none; padding: 8px 16px; border-radius: 4px; cursor: pointer; font-size: 13px; }
	.form button:disabled { opacity: 0.5; cursor: not-allowed; }
	.form button:hover:not(:disabled) { background: #6d28d9; }
	.proposal-card { background: rgba(31, 41, 55, 0.8); border: 1px solid #374151; border-radius: 6px; padding: 10px 12px; margin-bottom: 6px; display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
	.corp-name { font-weight: 600; font-size: 13px; }
	.offer { font-family: monospace; color: #fbbf24; font-size: 13px; }
	.status { font-size: 12px; padding: 2px 6px; border-radius: 3px; background: #374151; margin-left: auto; }
	.actions { display: flex; gap: 6px; margin-left: auto; }
	.accept { background: #065f46; color: #34d399; border: none; padding: 4px 10px; border-radius: 3px; cursor: pointer; font-size: 12px; }
	.reject { background: #7f1d1d; color: #fca5a5; border: none; padding: 4px 10px; border-radius: 3px; cursor: pointer; font-size: 12px; }
	.accept:hover { background: #047857; }
	.reject:hover { background: #991b1b; }
</style>
