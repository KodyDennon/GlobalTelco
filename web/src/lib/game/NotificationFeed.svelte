<script lang="ts">
	import { notifications } from '$lib/stores/gameState';
	import { tr } from '$lib/i18n/index';

	let expanded = $state(false);

	function getCategory(event: string): string {
		if (event.includes('Disaster')) return 'disaster';
		if (event.includes('Construction') || event.includes('Node') || event.includes('Edge') || event.includes('Repair')) return 'infra';
		if (event.includes('Revenue') || event.includes('Cost') || event.includes('Loan') || event.includes('Bankruptcy')) return 'finance';
		if (event.includes('Contract')) return 'contract';
		if (event.includes('Research')) return 'research';
		if (event.includes('Regulation') || event.includes('Market')) return 'market';
		return 'info';
	}

	function getCategoryColor(cat: string): string {
		switch (cat) {
			case 'disaster': return 'var(--red)';
			case 'infra': return 'var(--blue)';
			case 'finance': return 'var(--green)';
			case 'contract': return 'var(--amber)';
			case 'research': return '#8b5cf6';
			case 'market': return '#ec4899';
			default: return 'var(--text-dim)';
		}
	}

	function formatEvent(event: string): string {
		// Clean up Rust debug format
		return event
			.replace(/\{[^}]*\}/g, '')
			.replace(/([A-Z])/g, ' $1')
			.trim();
	}

	let recentNotifs = $derived($notifications.slice(0, expanded ? 20 : 3));
	let hasNotifs = $derived($notifications.length > 0);
</script>

{#if hasNotifs}
	<div class="feed" class:expanded role="log" aria-live="polite">
		<button class="feed-header" onclick={() => (expanded = !expanded)}>
			<span class="feed-title">{$tr('game.events')}</span>
			<span class="feed-count">{$notifications.length}</span>
			<span class="toggle">{expanded ? 'v' : '^'}</span>
		</button>
		<div class="feed-list">
			{#each recentNotifs as notif}
				{@const cat = getCategory(notif.event)}
				<div class="notif-row">
					<span class="dot" style="background: {getCategoryColor(cat)}"></span>
					<span class="notif-tick">T{notif.tick}</span>
					<span class="notif-text">{formatEvent(notif.event)}</span>
				</div>
			{/each}
		</div>
	</div>
{/if}

<style>
	.feed {
		position: absolute;
		bottom: 8px;
		right: 8px;
		width: 320px;
		max-height: 52px;
		overflow: hidden;
		background: rgba(17, 24, 39, 0.92);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		z-index: 12;
		transition: max-height 0.2s ease;
		font-family: var(--font-sans);
	}

	.feed.expanded {
		max-height: 400px;
		overflow-y: auto;
	}

	.feed-header {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 10px;
		cursor: pointer;
		border: none;
		border-bottom: 1px solid var(--border);
		background: transparent;
		width: 100%;
		font-size: 11px;
		color: inherit;
	}

	.feed-title {
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.5px;
	}

	.feed-count {
		background: var(--bg-surface);
		padding: 1px 6px;
		border-radius: 8px;
		font-size: 10px;
		color: var(--text-dim);
		font-family: var(--font-mono);
	}

	.toggle {
		margin-left: auto;
		color: var(--text-dim);
		font-size: 10px;
	}

	.feed-list {
		padding: 2px 0;
	}

	.notif-row {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 3px 10px;
		font-size: 11px;
		color: var(--text-muted);
		border-bottom: 1px solid rgba(55, 65, 81, 0.15);
	}

	.dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.notif-tick {
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--text-dim);
		min-width: 40px;
	}

	.notif-text {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
