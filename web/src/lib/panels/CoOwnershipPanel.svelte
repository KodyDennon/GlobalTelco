<script lang="ts">
	import { playerCorp, formatMoney, allCorporations, worldInfo } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import type { InfraNode, InfrastructureList, CoOwnershipProposal, UpgradeVote } from '$lib/wasm/types';
	import { tooltip } from '$lib/ui/tooltip';

	let infra: InfrastructureList = $state({ nodes: [], edges: [] });
	let proposals: CoOwnershipProposal[] = $state([]);
	let upgradeVotes: UpgradeVote[] = $state([]);

	let showProposeForm = $state(false);
	let selectedNode = $state(0);
	let targetCorp = $state(0);
	let sharePercent = $state(50);

	// Load all co-ownership data from bridge (every 5th tick)
	$effect(() => {
		const corp = $playerCorp;
		const tick = $worldInfo.tick;
		if (tick % 5 !== 0) return;
		if (corp) {
			infra = bridge.getInfrastructureList(corp.id);
			proposals = bridge.getCoOwnershipProposals(corp.id);
			upgradeVotes = bridge.getPendingUpgradeVotes(corp.id);
		}
	});

	function proposeCoOwnership() {
		const corp = $playerCorp;
		if (!corp || !selectedNode || !targetCorp) return;
		gameCommand({
			ProposeCoOwnership: {
				node: selectedNode,
				target_corp: targetCorp,
				share_pct: sharePercent / 100
			}
		});
		showProposeForm = false;
		selectedNode = 0;
		targetCorp = 0;
		sharePercent = 50;
	}

	function respondProposal(proposalId: number, accept: boolean) {
		gameCommand({
			RespondCoOwnership: {
				proposal: proposalId,
				accept
			}
		});
	}

	function proposeBuyout(nodeId: number, targetCorpId: number, price: number) {
		gameCommand({ ProposeBuyout: { node: nodeId, target_corp: targetCorpId, price } });
	}

	function voteUpgrade(nodeId: number, approve: boolean) {
		gameCommand({ VoteUpgrade: { node: nodeId, approve } });
	}

	let operationalNodes = $derived(infra.nodes.filter((n) => !n.under_construction));
	let incomingProposals = $derived(proposals.filter((p) => p.direction === 'incoming'));
	let outgoingProposals = $derived(proposals.filter((p) => p.direction === 'outgoing'));
	let aiCorps = $derived($allCorporations.filter((c) => !c.is_player));
</script>

<div class="panel">
	<div class="section">
		<h3>Co-Ownership Summary</h3>
		<div class="stat-row">
			<span class="muted">Your infrastructure</span>
			<span class="mono">{operationalNodes.length}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Pending proposals</span>
			<span class="mono" class:warn={incomingProposals.length > 0}>{proposals.length}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Active votes</span>
			<span class="mono" class:warn={upgradeVotes.length > 0}>{upgradeVotes.length}</span>
		</div>
	</div>

	{#if upgradeVotes.length > 0}
		<div class="section">
			<h3 class="warn">Active Upgrade Votes</h3>
			{#each upgradeVotes as vote}
				<div class="vote-card">
					<div class="vote-info">
						<div class="vote-target">{vote.node_type} (ID: {vote.node_id})</div>
						<div class="vote-proposer">Proposed by: {vote.proposer_name}</div>
					</div>
					<div class="vote-actions">
						{#if !vote.has_voted}
							<button class="accept-btn" onclick={() => voteUpgrade(vote.node_id, true)}>APPROVE</button>
							<button class="reject-btn" onclick={() => voteUpgrade(vote.node_id, false)}>REJECT</button>
						{:else}
							<span class="badge voted">Voted</span>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	{/if}

	<div class="section">
		<div class="section-hdr">
			<h3>Propose Co-Ownership</h3>
			<button
				class="action-btn"
				onclick={() => (showProposeForm = !showProposeForm)}
				use:tooltip={'Propose joint ownership of infrastructure\nShare costs and revenue with another corporation'}
			>
				{showProposeForm ? 'Cancel' : '+ Propose'}
			</button>
		</div>
		{#if showProposeForm}
			<div class="propose-form">
				<label class="form-label">
					<span class="muted">Infrastructure</span>
					<select bind:value={selectedNode} aria-label="Select infrastructure node">
						<option value={0}>Select node...</option>
						{#each operationalNodes as node}
							<option value={node.id}>{node.node_type} (HP: {(node.health * 100).toFixed(0)}%)</option>
						{/each}
					</select>
				</label>
				<label class="form-label">
					<span class="muted">Target Corporation</span>
					<select bind:value={targetCorp} aria-label="Select target corporation">
						<option value={0}>Select corporation...</option>
						{#each aiCorps as corp}
							<option value={corp.id}>{corp.name}</option>
						{/each}
					</select>
				</label>
				<label class="form-label">
					<span class="muted">Share offered: {sharePercent}%</span>
					<input type="range" min={10} max={90} step={5} bind:value={sharePercent} />
					<div class="share-labels">
						<span>You: {100 - sharePercent}%</span>
						<span>Partner: {sharePercent}%</span>
					</div>
				</label>
				<button
					class="confirm-btn"
					onclick={proposeCoOwnership}
					disabled={!selectedNode || !targetCorp}
					use:tooltip={'Send co-ownership proposal\nThe target corporation will review and respond'}
				>
					Send Proposal
				</button>
			</div>
		{/if}
	</div>

	{#if incomingProposals.length > 0}
		<div class="section">
			<h3>Incoming Proposals ({incomingProposals.length})</h3>
			{#each incomingProposals as proposal}
				<div class="proposal-card incoming">
					<div class="proposal-info">
						<div class="proposal-type">{proposal.node_type}</div>
						<div class="proposal-parties">
							<span>{proposal.from_name}</span>
							<span class="arrow">wants</span>
							<span class="mono">{proposal.share_pct}%</span>
						</div>
					</div>
					<div class="proposal-actions">
						<button
							class="accept-btn"
							onclick={() => respondProposal(proposal.id, true)}
							use:tooltip={() =>
								`Accept co-ownership with ${proposal.from_name}\nThey get ${proposal.share_pct}% share of ${proposal.node_type}`}
						>
							Accept
						</button>
						<button
							class="reject-btn"
							onclick={() => respondProposal(proposal.id, false)}
							use:tooltip={'Reject this co-ownership proposal'}
						>
							Reject
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}

	{#if outgoingProposals.length > 0}
		<div class="section">
			<h3>Outgoing Proposals ({outgoingProposals.length})</h3>
			{#each outgoingProposals as proposal}
				<div class="proposal-card outgoing">
					<div class="proposal-info">
						<div class="proposal-type">{proposal.node_type}</div>
						<div class="proposal-parties">
							<span class="arrow">to</span>
							<span>{proposal.to_name}</span>
							<span class="mono">({proposal.share_pct}%)</span>
						</div>
					</div>
					<div class="proposal-badge">
						<span class="badge pending">Pending</span>
					</div>
				</div>
			{/each}
		</div>
	{/if}

	<div class="section">
		<h3>Your Infrastructure</h3>
		{#each operationalNodes as node}
			<div class="node-row">
				<div class="node-info">
					<span class="node-type">{node.node_type}</span>
					<div class="node-stats">
						<span>
							<span class="muted">HP</span>
							<span class="mono" class:warn={node.health < 0.5}
								>{(node.health * 100).toFixed(0)}%</span
							>
						</span>
						<span>
							<span class="muted">Value</span>
							<span class="mono">{formatMoney(node.construction_cost)}</span>
						</span>
					</div>
				</div>
				<div class="node-actions">
					<button
						class="tiny-btn"
						onclick={() => voteUpgrade(node.id, true)}
						use:tooltip={() => `Vote to upgrade ${node.node_type}\nRequires co-owner agreement`}
					>
						Upgrade
					</button>
					<button
						class="tiny-btn buyout"
						onclick={() => proposeBuyout(node.id, 0, node.construction_cost)}
						use:tooltip={() => `Propose buying out co-owners of ${node.node_type}`}
					>
						Buyout
					</button>
				</div>
			</div>
		{/each}
		{#if operationalNodes.length === 0}
			<div class="empty">No infrastructure available for co-ownership.</div>
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

	.warn {
		color: var(--amber, #f59e0b);
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

	.vote-card {
		background: rgba(245, 158, 11, 0.05);
		border: 1px solid rgba(245, 158, 11, 0.2);
		border-radius: var(--radius-md);
		padding: 10px;
		margin-bottom: 8px;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.vote-target {
		font-weight: 700;
		color: var(--text-primary);
	}

	.vote-proposer {
		font-size: 11px;
		color: var(--text-dim);
	}

	.vote-actions {
		display: flex;
		gap: 6px;
	}

	.propose-form {
		display: flex;
		flex-direction: column;
		gap: 10px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		padding: 10px;
	}

	.form-label {
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: 12px;
	}

	.propose-form select {
		background: rgba(17, 24, 39, 0.8);
		border: 1px solid var(--border);
		color: var(--text-secondary);
		padding: 6px 8px;
		border-radius: var(--radius-sm);
		font-size: 12px;
		font-family: var(--font-mono);
	}

	.propose-form input[type='range'] {
		width: 100%;
		accent-color: var(--blue);
	}

	.share-labels {
		display: flex;
		justify-content: space-between;
		font-size: 11px;
		color: var(--text-dim);
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

	.proposal-card {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px;
		border-radius: var(--radius-md);
		margin-bottom: 6px;
	}

	.proposal-card.incoming {
		background: rgba(59, 130, 246, 0.05);
		border: 1px solid rgba(59, 130, 246, 0.2);
	}

	.proposal-card.outgoing {
		background: rgba(55, 65, 81, 0.1);
		border: 1px solid rgba(55, 65, 81, 0.3);
	}

	.proposal-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.proposal-type {
		font-weight: 600;
		color: var(--text-primary);
	}

	.proposal-parties {
		font-size: 12px;
		display: flex;
		gap: 6px;
		align-items: center;
	}

	.arrow {
		color: var(--text-dim);
	}

	.proposal-actions {
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

	.badge.pending {
		background: rgba(245, 158, 11, 0.1);
		color: #f59e0b;
	}

	.badge.voted {
		background: rgba(34, 197, 94, 0.1);
		color: var(--green);
	}

	.proposal-badge {
		display: flex;
		align-items: center;
	}

	.node-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px;
		border-radius: var(--radius-sm);
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
	}

	.node-row:hover {
		background: var(--bg-surface);
	}

	.node-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.node-type {
		font-weight: 600;
		color: var(--text-primary);
	}

	.node-stats {
		display: flex;
		gap: 12px;
		font-size: 11px;
	}

	.node-actions {
		display: flex;
		gap: 4px;
	}

	.tiny-btn {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		color: var(--text-muted);
		padding: 4px 8px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 11px;
		font-weight: 600;
	}

	.tiny-btn:hover {
		color: var(--blue);
		border-color: var(--blue);
	}

	.tiny-btn.buyout:hover {
		color: var(--green);
		border-color: var(--green);
	}

	.empty {
		color: var(--text-dim);
		text-align: center;
		padding: 16px;
		font-size: 12px;
	}
</style>
