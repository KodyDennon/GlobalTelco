<script lang="ts">
	import type { WorldListEntry } from '$lib/multiplayer/lobbyApi';

	let {
		worlds,
		onJoin,
		joiningWorldId
	}: {
		worlds: WorldListEntry[];
		onJoin: (id: string) => void;
		joiningWorldId: string | null;
	} = $props();

	let searchQuery = $state('');
	let sortBy = $state<'players' | 'newest' | 'era' | 'name'>('players');
	let eraFilter = $state('All');
	let hasSpaceOnly = $state(false);

	const ERA_OPTIONS = ['All', 'Telegraph', 'Telephone', 'EarlyDigital', 'Internet', 'Modern', 'NearFuture'];

	let filteredWorlds = $derived(() => {
		let result = worlds;

		// Search filter
		if (searchQuery.trim()) {
			const q = searchQuery.toLowerCase();
			result = result.filter(
				(w) =>
					w.name.toLowerCase().includes(q) ||
					(w.template_name && w.template_name.toLowerCase().includes(q))
			);
		}

		// Era filter
		if (eraFilter !== 'All') {
			result = result.filter((w) => w.era === eraFilter);
		}

		// Has space filter
		if (hasSpaceOnly) {
			result = result.filter((w) => w.player_count < w.max_players);
		}

		// Sort
		switch (sortBy) {
			case 'players':
				result = [...result].sort((a, b) => b.player_count - a.player_count);
				break;
			case 'newest':
				result = [...result].sort((a, b) => b.tick - a.tick);
				break;
			case 'era':
				result = [...result].sort((a, b) => a.era.localeCompare(b.era));
				break;
			case 'name':
				result = [...result].sort((a, b) => a.name.localeCompare(b.name));
				break;
		}

		return result;
	});

	function copyInviteCode(code: string) {
		navigator.clipboard.writeText(code);
	}
</script>

<div class="world-list-container">
	<div class="filters">
		<input
			type="text"
			class="search-input"
			placeholder="Search worlds..."
			bind:value={searchQuery}
		/>
		<select class="sort-select" bind:value={sortBy}>
			<option value="players">Most Players</option>
			<option value="newest">Newest</option>
			<option value="era">Era</option>
			<option value="name">Name</option>
		</select>
		<label class="has-space-check">
			<input type="checkbox" bind:checked={hasSpaceOnly} />
			Has Space
		</label>
	</div>

	<div class="era-pills">
		{#each ERA_OPTIONS as era}
			<button
				class="era-pill"
				class:active={eraFilter === era}
				onclick={() => (eraFilter = era)}
			>
				{era}
			</button>
		{/each}
	</div>

	<div class="worlds">
		{#if filteredWorlds().length === 0}
			<div class="empty">No worlds match your filters</div>
		{:else}
			{#each filteredWorlds() as world}
				<div class="world-card">
					<div class="world-info">
						<div class="world-header">
							<h3>{world.name}</h3>
							{#if world.template_name}
								<span class="template-badge">{world.template_name}</span>
							{/if}
						</div>
						<div class="world-details">
							<span class="era-badge">{world.era}</span>
							<div class="player-bar-wrap">
								<div class="player-bar">
									<div
										class="player-fill"
										style="width: {(world.player_count / world.max_players) * 100}%"
									></div>
								</div>
								<span class="player-text">{world.player_count}/{world.max_players}</span>
							</div>
							<span class="detail-sep">&middot;</span>
							<span>Tick {world.tick}</span>
							<span class="detail-sep">&middot;</span>
							<span>{world.speed}</span>
							<span class="detail-sep">&middot;</span>
							<span>{world.map_size}</span>
						</div>
						{#if world.invite_code}
							<div class="invite-row">
								<span class="invite-label">Invite:</span>
								<code class="invite-code">{world.invite_code}</code>
								<button class="copy-btn" onclick={() => copyInviteCode(world.invite_code!)}>Copy</button>
							</div>
						{/if}
					</div>
					<button
						class="btn-join"
						onclick={() => onJoin(world.id)}
						disabled={world.player_count >= world.max_players || joiningWorldId === world.id}
					>
						{#if joiningWorldId === world.id}
							Joining...
						{:else if world.player_count >= world.max_players}
							Full
						{:else}
							Join
						{/if}
					</button>
				</div>
			{/each}
		{/if}
	</div>
</div>

<style>
	.world-list-container {
		display: flex;
		flex-direction: column;
		gap: 12px;
		flex: 1;
		min-height: 0;
	}

	.filters {
		display: flex;
		gap: 8px;
		align-items: center;
	}

	.search-input {
		flex: 1;
		padding: 8px 12px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		color: #f3f4f6;
		font-size: 13px;
		font-family: system-ui, sans-serif;
	}

	.search-input:focus {
		outline: none;
		border-color: #10b981;
	}

	.sort-select {
		padding: 8px 12px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		color: #d1d5db;
		font-size: 13px;
		font-family: system-ui, sans-serif;
	}

	.has-space-check {
		display: flex;
		align-items: center;
		gap: 4px;
		font-size: 13px;
		color: #9ca3af;
		white-space: nowrap;
		cursor: pointer;
	}

	.era-pills {
		display: flex;
		gap: 6px;
		flex-wrap: wrap;
	}

	.era-pill {
		padding: 4px 10px;
		background: rgba(31, 41, 55, 0.6);
		border: 1px solid rgba(55, 65, 81, 0.3);
		border-radius: 12px;
		color: #9ca3af;
		font-size: 12px;
		cursor: pointer;
		font-family: system-ui, sans-serif;
	}

	.era-pill:hover {
		border-color: rgba(55, 65, 81, 0.6);
		color: #d1d5db;
	}

	.era-pill.active {
		background: rgba(59, 130, 246, 0.2);
		border-color: rgba(59, 130, 246, 0.4);
		color: #60a5fa;
	}

	.worlds {
		flex: 1;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.world-card {
		display: flex;
		align-items: center;
		justify-content: space-between;
		background: rgba(31, 41, 55, 0.6);
		border: 1px solid rgba(55, 65, 81, 0.3);
		border-radius: 8px;
		padding: 14px 16px;
	}

	.world-info {
		flex: 1;
		min-width: 0;
	}

	.world-header {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 4px;
	}

	.world-header h3 {
		margin: 0;
		font-size: 15px;
		font-weight: 600;
	}

	.template-badge {
		font-size: 10px;
		padding: 2px 6px;
		background: rgba(99, 102, 241, 0.2);
		border: 1px solid rgba(99, 102, 241, 0.3);
		border-radius: 4px;
		color: #818cf8;
	}

	.world-details {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 12px;
		color: #9ca3af;
	}

	.era-badge {
		font-size: 10px;
		padding: 1px 6px;
		background: rgba(16, 185, 129, 0.15);
		border: 1px solid rgba(16, 185, 129, 0.25);
		border-radius: 4px;
		color: #34d399;
	}

	.player-bar-wrap {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.player-bar {
		width: 40px;
		height: 5px;
		background: rgba(55, 65, 81, 0.6);
		border-radius: 3px;
		overflow: hidden;
	}

	.player-fill {
		height: 100%;
		background: #10b981;
		border-radius: 3px;
	}

	.player-text {
		font-size: 11px;
		color: #6b7280;
	}

	.detail-sep {
		color: #4b5563;
	}

	.invite-row {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-top: 4px;
		font-size: 11px;
	}

	.invite-label {
		color: #6b7280;
	}

	.invite-code {
		background: rgba(17, 24, 39, 0.6);
		padding: 1px 6px;
		border-radius: 3px;
		font-size: 11px;
		color: #d1d5db;
	}

	.copy-btn {
		background: none;
		border: none;
		color: #60a5fa;
		font-size: 11px;
		cursor: pointer;
		padding: 0;
		font-family: system-ui, sans-serif;
	}

	.copy-btn:hover {
		color: #93bbfd;
	}

	.btn-join {
		background: rgba(59, 130, 246, 0.2);
		border: 1px solid rgba(59, 130, 246, 0.3);
		color: #60a5fa;
		padding: 8px 20px;
		border-radius: 6px;
		cursor: pointer;
		font-family: system-ui, sans-serif;
		white-space: nowrap;
		margin-left: 16px;
		font-size: 13px;
	}

	.btn-join:hover:not(:disabled) {
		background: rgba(59, 130, 246, 0.3);
	}

	.btn-join:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.empty {
		text-align: center;
		color: #6b7280;
		padding: 40px;
	}
</style>
