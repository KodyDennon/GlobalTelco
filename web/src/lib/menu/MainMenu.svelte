<script lang="ts">
	import { tr } from '$lib/i18n/index';
	import { isAuthenticated } from '$lib/stores/multiplayerState';

	let { onNewGame, onLoadGame, onSettings, onMultiplayer, onCredits, onProfile }: { onNewGame: () => void; onLoadGame: () => void; onSettings: () => void; onMultiplayer: () => void; onCredits: () => void; onProfile: () => void } = $props();

	const version = typeof __APP_VERSION__ !== 'undefined' ? __APP_VERSION__ : '1.9.1';
</script>

<div class="main-menu" role="navigation" aria-label={$tr('menu.title')}>
	<div class="menu-content">
		<h1 class="title">{$tr('menu.title')}</h1>
		<p class="subtitle">{$tr('menu.subtitle')}</p>

		<div class="menu-buttons" role="group" aria-label="Main menu options">
			<button class="menu-btn primary" onclick={onNewGame} aria-label="Start a new game">{$tr('menu.new_game')}</button>
			<button class="menu-btn" onclick={onLoadGame} aria-label="Load a saved game">{$tr('menu.load_game')}</button>
			<button class="menu-btn" onclick={onMultiplayer} aria-label="Join or host multiplayer">{$tr('menu.multiplayer')}</button>
			<button class="menu-btn" onclick={onSettings} aria-label="Open settings">{$tr('menu.settings')}</button>
			{#if $isAuthenticated}
				<button class="menu-btn" onclick={onProfile} aria-label="View profile">Profile</button>
			{/if}
			<button class="menu-btn" onclick={onCredits} aria-label="View credits">{$tr('menu.credits')}</button>
		</div>
	</div>

	<div class="menu-footer" role="contentinfo">
		<span>{$tr('menu.version', { version })}</span>
		<a class="github-link" href="https://github.com/KodyDennon/GlobalTelco" target="_blank" rel="noopener noreferrer" aria-label="View source on GitHub">GitHub</a>
	</div>
</div>

<style>
	.main-menu {
		width: 100vw;
		height: 100vh;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		background: linear-gradient(135deg, #0a0e17 0%, #111827 50%, #0a0e17 100%);
		font-family: system-ui, sans-serif;
		color: #f3f4f6;
	}

	.menu-content {
		text-align: center;
	}

	.title {
		font-size: 48px;
		font-weight: 800;
		letter-spacing: -1px;
		margin: 0;
		background: linear-gradient(90deg, #10b981, #3b82f6);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.subtitle {
		color: #6b7280;
		font-size: 16px;
		margin: 8px 0 48px;
	}

	.menu-buttons {
		display: flex;
		flex-direction: column;
		gap: 12px;
		width: 280px;
		margin: 0 auto;
	}

	.menu-btn {
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		color: #d1d5db;
		padding: 14px 24px;
		font-size: 16px;
		font-family: system-ui, sans-serif;
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.menu-btn:hover:not(:disabled) {
		background: rgba(55, 65, 81, 0.8);
		color: #f3f4f6;
		border-color: rgba(75, 85, 101, 0.5);
	}

	.menu-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.menu-btn.primary {
		background: rgba(16, 185, 129, 0.2);
		border-color: rgba(16, 185, 129, 0.3);
		color: #10b981;
	}

	.menu-btn.primary:hover {
		background: rgba(16, 185, 129, 0.3);
		color: #34d399;
	}

	.menu-footer {
		position: absolute;
		bottom: 24px;
		color: #4b5563;
		font-size: 12px;
		display: flex;
		gap: 12px;
		align-items: center;
	}

	.github-link {
		color: #4b5563;
		text-decoration: none;
		transition: color 0.2s;
	}

	.github-link:hover {
		color: #9ca3af;
	}
</style>
