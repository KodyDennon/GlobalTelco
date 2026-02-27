<script lang="ts">
	import { playerCorp, formatMoney } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import type { ResearchInfo } from '$lib/wasm/types';
	import { tooltip } from '$lib/ui/tooltip';

	let allTechs: ResearchInfo[] = $state([]);
	let activeCategory = $state('all');
	let rdBudget = $state(100000);

	const CATEGORIES = [
		{ key: 'all', label: 'All' },
		{ key: 'OpticalNetworks', label: 'Optical' },
		{ key: 'Wireless5G', label: 'Wireless' },
		{ key: 'Satellite', label: 'Satellite' },
		{ key: 'DataCenter', label: 'Data Center' },
		{ key: 'NetworkResilience', label: 'Resilience' },
		{ key: 'OperationalEfficiency', label: 'Operations' }
	];

	$effect(() => {
		const corp = $playerCorp;
		if (corp) {
			allTechs = bridge.getResearchState();
		}
	});

	let filteredTechs = $derived(
		activeCategory === 'all' ? allTechs : allTechs.filter((t) => t.category === activeCategory)
	);

	let playerCorpId = $derived($playerCorp?.id ?? 0);

	let activeResearch = $derived(
		allTechs.find((t) => t.researcher === playerCorpId && !t.completed)
	);

	let completedByPlayer = $derived(
		allTechs.filter((t) => t.completed && t.researcher === playerCorpId)
	);

	function canResearch(tech: ResearchInfo): boolean {
		if (tech.completed) return false;
		if (tech.researcher !== null) return false;
		if (activeResearch) return false;
		// Check prerequisites — player must have completed or licensed each prereq
		for (const prereq of tech.prerequisites) {
			const prereqTech = allTechs.find((t) => t.name === prereq);
			if (!prereqTech || !prereqTech.completed) return false;
			// If completed by another corp, player needs a license (backend validates access)
			// Allow the button to show — backend will reject if no license exists
		}
		return true;
	}

	function startResearch(techName: string) {
		gameCommand({ StartResearch: { corporation: playerCorpId, tech: techName } });
	}

	function cancelResearch() {
		gameCommand({ CancelResearch: { corporation: playerCorpId } });
	}

	function setBudget() {
		gameCommand({
			SetBudget: { corporation: playerCorpId, category: 'research', amount: rdBudget }
		});
	}

</script>

<div class="panel">
	<!-- R&D Budget -->
	<div class="section">
		<h3>R&D Budget</h3>
		<div class="budget-row">
			<input
				type="range"
				min="0"
				max="5000000"
				step="50000"
				bind:value={rdBudget}
				oninput={setBudget}
			/>
			<span class="mono green">{formatMoney(rdBudget)}/tick</span>
		</div>
	</div>

	<!-- Active Research -->
	{#if activeResearch}
		<div class="section">
			<h3>Active Research</h3>
			<div class="active-research">
				<div class="ar-name">{activeResearch.name}</div>
				<div class="ar-cat">{activeResearch.category_name}</div>
				<div class="progress-bar">
					<div class="progress-fill" style="width: {activeResearch.progress_pct * 100}%"></div>
				</div>
				<div class="ar-stats">
					<span>{(activeResearch.progress_pct * 100).toFixed(1)}%</span>
					<span>{formatMoney(activeResearch.total_cost - activeResearch.progress)} remaining</span>
				</div>
				<button class="btn cancel-research" onclick={cancelResearch} use:tooltip={'Cancel current research\nProgress will be lost — R&D budget returns to general funds'}>Cancel Research</button>
			</div>
		</div>
	{/if}

	<!-- Category Tabs -->
	<div class="tabs">
		{#each CATEGORIES as cat}
			<button
				class="tab"
				class:active={activeCategory === cat.key}
				onclick={() => (activeCategory = cat.key)}
			>
				{cat.label}
			</button>
		{/each}
	</div>

	<!-- Tech List -->
	<div class="section tech-list">
		{#each filteredTechs as tech}
			{@const researchable = canResearch(tech)}
			<div class="tech-row" class:completed={tech.completed} class:active={tech.researcher === playerCorpId && !tech.completed}>
				<div class="tech-info">
					<div class="tech-header">
						<span class="tech-name">{tech.name}</span>
						{#if tech.completed}
							<span class="badge done">Done</span>
						{:else if tech.researcher !== null}
							<span class="badge researching">
								{tech.researcher === playerCorpId ? 'You' : tech.researcher_name}
							</span>
						{/if}
					</div>
					<div class="tech-desc">{tech.description}</div>
					<div class="tech-stats">
						{#if tech.throughput_bonus > 0}
							<span class="stat">+{(tech.throughput_bonus * 100).toFixed(0)}% throughput</span>
						{/if}
						{#if tech.cost_reduction > 0}
							<span class="stat">-{(tech.cost_reduction * 100).toFixed(0)}% costs</span>
						{/if}
						{#if tech.reliability_bonus > 0}
							<span class="stat">+{(tech.reliability_bonus * 100).toFixed(0)}% reliability</span>
						{/if}
						<span class="stat cost">{formatMoney(tech.total_cost)}</span>
					</div>
					{#if tech.prerequisites.length > 0}
						<div class="prereqs">
							Requires: {tech.prerequisites.join(', ')}
						</div>
					{/if}
					{#if tech.independent_tier && tech.independent_tier !== 'None'}
						<div class="independent-info">
							{#if tech.independent_tier === 'Premium'}
								<span class="badge premium-badge">Premium Independent</span>
								<span class="muted">+10% stats, can patent</span>
							{:else}
								<span class="badge standard-badge">Standard Independent</span>
								<span class="muted">normal stats, no patent</span>
							{/if}
						</div>
					{/if}
					{#if tech.completed && tech.patent_status !== 'None'}
						<div class="patent-info">
							Patent: {tech.patent_status}
							{#if tech.patent_owner !== null && tech.patent_owner !== playerCorpId}
								<span class="muted">by {tech.patent_owner_name ?? 'Unknown'}</span>
							{/if}
							{#if tech.license_price > 0}
								| License: {formatMoney(tech.license_price)}/tick
							{/if}
						</div>
					{/if}
				</div>
				<div class="tech-actions">
					{#if researchable}
						<button class="btn research" onclick={() => startResearch(tech.name)} use:tooltip={() => `Start researching ${tech.name}\nCost: ${formatMoney(tech.total_cost)}\n${tech.throughput_bonus > 0 ? `+${(tech.throughput_bonus * 100).toFixed(0)}% throughput\n` : ''}${tech.cost_reduction > 0 ? `-${(tech.cost_reduction * 100).toFixed(0)}% costs\n` : ''}${tech.reliability_bonus > 0 ? `+${(tech.reliability_bonus * 100).toFixed(0)}% reliability` : ''}`}>
							Research
						</button>
					{/if}
				</div>
			</div>
		{/each}
	</div>

	<!-- Completed Count -->
	{#if completedByPlayer.length > 0}
		<div class="section">
			<h3>Your Completed ({completedByPlayer.length})</h3>
			<div class="completed-list">
				{#each completedByPlayer as tech}
					<span class="completed-tag">{tech.name}</span>
				{/each}
			</div>
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

	.mono { font-family: var(--font-mono); }
	.green { color: var(--green); }

	.budget-row {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.budget-row input[type='range'] {
		flex: 1;
		accent-color: var(--blue);
	}

	.active-research {
		background: rgba(59, 130, 246, 0.08);
		border: 1px solid rgba(59, 130, 246, 0.2);
		border-radius: var(--radius-md);
		padding: 10px;
	}

	.ar-name { font-weight: 600; color: var(--text-primary); }
	.ar-cat { font-size: 11px; color: var(--text-dim); margin-bottom: 6px; }

	.progress-bar {
		height: 4px;
		background: var(--bg-surface);
		border-radius: 2px;
		overflow: hidden;
		margin-bottom: 4px;
	}

	.progress-fill {
		height: 100%;
		background: var(--blue);
		border-radius: 2px;
		transition: width 0.3s;
	}

	.ar-stats {
		display: flex;
		justify-content: space-between;
		font-size: 11px;
		color: var(--text-dim);
		font-family: var(--font-mono);
	}

	.tabs {
		display: flex;
		gap: 2px;
		padding: 8px 16px;
		border-bottom: 1px solid var(--border);
		overflow-x: auto;
	}

	.tab {
		background: transparent;
		border: none;
		color: var(--text-dim);
		padding: 4px 8px;
		font-size: 11px;
		font-family: var(--font-sans);
		cursor: pointer;
		border-radius: 3px;
		white-space: nowrap;
	}

	.tab:hover { color: var(--text-primary); background: var(--bg-surface); }
	.tab.active { color: var(--blue); background: rgba(59, 130, 246, 0.1); }

	.tech-list {
		padding: 8px 16px;
		max-height: 400px;
		overflow-y: auto;
	}

	.tech-row {
		padding: 8px;
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 8px;
	}

	.tech-row.completed { opacity: 0.6; }
	.tech-row.active { background: rgba(59, 130, 246, 0.05); border-radius: var(--radius-sm); }

	.tech-info { flex: 1; min-width: 0; }

	.tech-header {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-bottom: 2px;
	}

	.tech-name { font-weight: 600; color: var(--text-primary); font-size: 13px; }

	.badge {
		font-size: 9px;
		padding: 1px 5px;
		border-radius: 3px;
		font-weight: 600;
	}

	.badge.done { background: var(--green-bg); color: var(--green); }
	.badge.researching { background: rgba(59, 130, 246, 0.15); color: var(--blue); }

	.tech-desc {
		font-size: 11px;
		color: var(--text-dim);
		margin-bottom: 4px;
	}

	.tech-stats {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		font-size: 10px;
	}

	.stat {
		background: var(--bg-surface);
		padding: 1px 5px;
		border-radius: 2px;
		color: var(--green);
	}

	.stat.cost { color: var(--amber); }

	.prereqs {
		font-size: 10px;
		color: var(--text-dim);
		margin-top: 3px;
		font-style: italic;
	}

	.muted { color: var(--text-muted); font-size: 10px; }

	.independent-info {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-top: 3px;
		font-size: 10px;
	}

	.premium-badge {
		background: rgba(245, 158, 11, 0.15);
		color: var(--amber);
		border: 1px solid rgba(245, 158, 11, 0.3);
	}

	.standard-badge {
		background: rgba(59, 130, 246, 0.1);
		color: var(--blue);
		border: 1px solid rgba(59, 130, 246, 0.2);
	}

	.patent-info {
		font-size: 10px;
		color: #8b5cf6;
		margin-top: 3px;
	}

	.tech-actions {
		flex-shrink: 0;
	}

	.btn.research {
		background: rgba(59, 130, 246, 0.15);
		border: 1px solid rgba(59, 130, 246, 0.3);
		color: var(--blue);
		padding: 4px 10px;
		font-size: 11px;
		border-radius: var(--radius-sm);
		cursor: pointer;
	}

	.btn.research:hover {
		background: rgba(59, 130, 246, 0.25);
	}

	.btn.cancel-research {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: var(--red);
		padding: 4px 10px;
		font-size: 11px;
		border-radius: var(--radius-sm);
		cursor: pointer;
		margin-top: 6px;
		width: 100%;
	}

	.btn.cancel-research:hover {
		background: rgba(239, 68, 68, 0.2);
	}

	.completed-list {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
	}

	.completed-tag {
		font-size: 10px;
		background: var(--green-bg);
		color: var(--green);
		padding: 2px 6px;
		border-radius: 3px;
	}
</style>
