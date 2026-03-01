<script lang="ts">
	import { playerList } from '$lib/stores/multiplayerState';
	import { tooltip } from '$lib/ui/tooltip';

	let collapsed = $state(false);

	let connectedCount = $derived($playerList.filter(p => p.status === 'Connected').length);
</script>

<div class="player-list-overlay" class:collapsed>
	<button
		class="player-header"
		type="button"
		onclick={() => (collapsed = !collapsed)}
		aria-expanded={!collapsed}
		aria-label={collapsed ? 'Expand player list' : 'Collapse player list'}
		use:tooltip={() => collapsed ? 'Expand player list' : 'Collapse player list'}
	>
		<span>Players ({connectedCount})</span>
		<span class="toggle" aria-hidden="true">{collapsed ? '+' : '-'}</span>
	</button>

	{#if !collapsed}
		<div class="player-entries" role="list">
			{#each $playerList as player}
				<div class="player-entry" role="listitem">
					<span
						class="status-dot"
						class:connected={player.status === 'Connected'}
						class:ai-proxy={player.status === 'AiProxy'}
						class:disconnected={player.status === 'Disconnected'}
					></span>
					<span class="player-name">{player.username}</span>
					{#if player.status === 'AiProxy'}
						<span class="proxy-label">AI</span>
					{/if}
				</div>
			{/each}
			{#if $playerList.length === 0}
				<div class="empty">No players</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.player-list-overlay {
		position: fixed;
		bottom: 16px;
		right: 16px;
		width: 220px;
		background: rgba(17, 24, 39, 0.95);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 8px;
		font-family: system-ui, sans-serif;
		z-index: 100;
		display: flex;
		flex-direction: column;
	}

	.player-list-overlay.collapsed {
		width: auto;
	}

	.player-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		width: 100%;
		padding: 8px 12px;
		background: transparent;
		border: none;
		color: #d1d5db;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		border-bottom: 1px solid rgba(55, 65, 81, 0.3);
		font-family: inherit;
	}

	.toggle {
		color: #6b7280;
		font-size: 16px;
	}

	.player-entries {
		max-height: 240px;
		overflow-y: auto;
		padding: 6px 8px;
	}

	.player-entry {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px 6px;
		border-radius: 4px;
	}

	.player-entry:hover {
		background: rgba(31, 41, 55, 0.5);
	}

	.status-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
		background: #4b5563;
	}

	.status-dot.connected {
		background: #10b981;
		box-shadow: 0 0 4px #10b981;
	}

	.status-dot.ai-proxy {
		background: #f59e0b;
		box-shadow: 0 0 4px #f59e0b;
	}

	.status-dot.disconnected {
		background: #4b5563;
	}

	.player-name {
		font-size: 12px;
		color: #d1d5db;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		flex: 1;
	}

	.proxy-label {
		font-size: 9px;
		font-weight: 700;
		letter-spacing: 0.05em;
		color: #f59e0b;
		background: rgba(245, 158, 11, 0.15);
		border: 1px solid rgba(245, 158, 11, 0.3);
		padding: 1px 5px;
		border-radius: 3px;
	}

	.empty {
		color: #6b7280;
		font-size: 12px;
		text-align: center;
		padding: 12px;
	}
</style>
