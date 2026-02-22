<script lang="ts">
	import * as bridge from '$lib/wasm/bridge';
	import type { AcquisitionInfo } from '$lib/wasm/types';
	import { worldInfo, playerCorp, allCorporations, formatMoney } from '$lib/stores/gameState';
	import { tr } from '$lib/i18n/index';

	let proposals: AcquisitionInfo[] = $state([]);
	let selectedTarget = $state(0);
	let offerAmount = $state(0);

	let playerId = $derived($playerCorp?.id ?? 0);
	let playerCash = $derived($playerCorp?.cash ?? 0);
	let corporations = $derived($allCorporations);

	// Reactive: refresh proposals when tick changes
	$effect(() => {
		const _tick = $worldInfo.tick;
		proposals = bridge.getAcquisitionProposals();
	});

	function refresh() {
		proposals = bridge.getAcquisitionProposals();
	}

	function proposeAcquisition() {
		if (!selectedTarget || offerAmount <= 0) return;
		bridge.processCommand({ ProposeAcquisition: { target: selectedTarget, offer: offerAmount } });
		offerAmount = 0;
		selectedTarget = 0;
		refresh();
	}

	function respond(proposalId: number, accept: boolean) {
		bridge.processCommand({ RespondToAcquisition: { proposal: proposalId, accept } });
		refresh();
	}



	const aiCorps = $derived(corporations.filter((c) => !c.is_player));
	const incomingProposals = $derived(proposals.filter((p) => p.target === playerId && p.status === 'Pending'));
	const outgoingProposals = $derived(proposals.filter((p) => p.acquirer === playerId));
</script>

<div class="panel" role="region" aria-label={$tr('panels.mergers')}>
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

	{#if incomingProposals.length === 0 && outgoingProposals.length === 0}
		<div class="empty">No acquisition proposals yet. Grow your empire to attract M&A interest, or propose an acquisition above.</div>
	{/if}
</div>

<style>
	.panel { padding: 16px; color: var(--text-secondary); }
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
	.empty { color: var(--text-dim); font-size: 13px; text-align: center; padding: 16px; }
</style>
