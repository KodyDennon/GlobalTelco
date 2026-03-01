<script lang="ts">
	import { page } from '$app/state';
	import { clearAdmin } from '../stores/auth.js';
	import { preferences } from '../stores/preferences.js';
	import { pollingEnabled, lastRefresh } from '../stores/polling.js';

	let collapsed = $derived($preferences.sidebarCollapsed);

	function toggleCollapse() {
		$preferences = { ...$preferences, sidebarCollapsed: !$preferences.sidebarCollapsed };
	}

	interface NavItem {
		label: string;
		href: string;
		icon: string;
	}

	const navItems: NavItem[] = [
		{ label: 'Overview', href: '/overview', icon: '\u{1F4CA}' },
		{ label: 'Worlds', href: '/worlds', icon: '\u{1F30D}' },
		{ label: 'Players', href: '/players', icon: '\u{1F465}' },
		{ label: 'Multiplayer', href: '/multiplayer', icon: '\u{1F517}' },
		{ label: 'Monitoring', href: '/monitoring', icon: '\u{1F4C8}' },
		{ label: 'Audit Log', href: '/audit', icon: '\u{1F4CB}' },
		{ label: 'Settings', href: '/settings', icon: '\u{2699}' }
	];

	function isActive(href: string): boolean {
		return page.url.pathname.startsWith(href);
	}

	const staleSeconds = $derived.by(() => {
		if (!$lastRefresh) return null;
		return Math.floor((Date.now() - $lastRefresh.getTime()) / 1000);
	});
</script>

<aside class="sidebar" class:collapsed>
	<div class="sidebar-header">
		{#if !collapsed}
			<span class="brand">GT Admin</span>
		{/if}
		<button class="collapse-btn" onclick={toggleCollapse} title={collapsed ? 'Expand' : 'Collapse'}>
			{collapsed ? '\u{276F}' : '\u{276E}'}
		</button>
	</div>

	<nav class="sidebar-nav">
		{#each navItems as item}
			<a
				href={item.href}
				class="nav-item"
				class:active={isActive(item.href)}
				title={collapsed ? item.label : undefined}
			>
				<span class="nav-icon">{item.icon}</span>
				{#if !collapsed}
					<span class="nav-label">{item.label}</span>
				{/if}
			</a>
		{/each}
	</nav>

	<div class="sidebar-footer">
		{#if !collapsed}
			<div class="refresh-status">
				<button
					class="refresh-toggle"
					class:active={$pollingEnabled}
					onclick={() => ($pollingEnabled = !$pollingEnabled)}
					title={$pollingEnabled ? 'Pause auto-refresh' : 'Resume auto-refresh'}
				>
					{$pollingEnabled ? '\u{25CF}' : '\u{25CB}'} Auto
				</button>
				{#if staleSeconds !== null}
					<span class="stale-indicator">{staleSeconds}s ago</span>
				{/if}
			</div>
		{/if}
		<button class="logout-btn" onclick={clearAdmin} title="Logout">
			{collapsed ? '\u{2190}' : 'Logout'}
		</button>
	</div>
</aside>

<style>
	.sidebar {
		width: var(--sidebar-width);
		height: 100vh;
		background: var(--bg-panel);
		border-right: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		transition: width 0.2s;
		flex-shrink: 0;
		overflow: hidden;
	}
	.sidebar.collapsed {
		width: var(--sidebar-collapsed);
	}
	.sidebar-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px;
		border-bottom: 1px solid var(--border);
		min-height: 48px;
	}
	.brand {
		font-size: 15px;
		font-weight: 700;
		color: var(--blue);
		white-space: nowrap;
	}
	.collapse-btn {
		background: none;
		border: none;
		color: var(--text-dim);
		cursor: pointer;
		font-size: 14px;
		padding: 4px;
	}
	.collapse-btn:hover {
		color: var(--text-primary);
	}
	.sidebar-nav {
		flex: 1;
		padding: 8px;
		display: flex;
		flex-direction: column;
		gap: 2px;
		overflow-y: auto;
	}
	.nav-item {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px 10px;
		border-radius: var(--radius-md);
		color: var(--text-muted);
		text-decoration: none;
		font-size: 13px;
		font-weight: 500;
		transition: all 0.15s;
		white-space: nowrap;
	}
	.nav-item:hover {
		background: var(--bg-surface);
		color: var(--text-primary);
	}
	.nav-item.active {
		background: var(--bg-surface);
		color: var(--blue-light, var(--blue));
	}
	.nav-icon {
		font-size: 16px;
		width: 20px;
		text-align: center;
		flex-shrink: 0;
	}
	.sidebar-footer {
		padding: 12px;
		border-top: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.refresh-status {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.refresh-toggle {
		font-size: 11px;
		background: none;
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		color: var(--text-dim);
		padding: 2px 8px;
		cursor: pointer;
	}
	.refresh-toggle.active {
		color: var(--green);
		border-color: var(--green-border);
	}
	.stale-indicator {
		font-size: 10px;
		color: var(--text-faint);
		font-family: var(--font-mono);
	}
	.logout-btn {
		padding: 6px 10px;
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		color: var(--text-muted);
		font-size: 12px;
		cursor: pointer;
		text-align: center;
	}
	.logout-btn:hover {
		background: var(--red-bg);
		color: var(--red);
		border-color: var(--red);
	}
</style>
