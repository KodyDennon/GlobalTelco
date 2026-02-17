<script lang="ts">
	import { tr } from '$lib/i18n/index';
	import MapView from './MapView.svelte';
	import HUD from './HUD.svelte';
	import InfoPanel from './InfoPanel.svelte';
	import BuildMenu from './BuildMenu.svelte';
	import NotificationFeed from './NotificationFeed.svelte';
	import Tooltip from './Tooltip.svelte';
	import Tutorial from './Tutorial.svelte';
	import Chat from './Chat.svelte';
	import { initialized } from '$lib/stores/gameState';
	import { activePanel } from '$lib/stores/uiState';
	import { tutorialCompleted, startTutorial } from '$lib/stores/tutorialState';
	import { isMultiplayer } from '$lib/stores/multiplayerState';

	// Lazy-load panels only when needed
	const panelComponents: Record<string, () => Promise<any>> = {
		dashboard: () => import('$lib/panels/DashboardPanel.svelte'),
		infrastructure: () => import('$lib/panels/InfraPanel.svelte'),
		contracts: () => import('$lib/panels/ContractPanel.svelte'),
		region: () => import('$lib/panels/RegionPanel.svelte'),
		research: () => import('$lib/panels/ResearchPanel.svelte'),
		workforce: () => import('$lib/panels/WorkforcePanel.svelte'),
		advisor: () => import('$lib/panels/AdvisorPanel.svelte'),
		auctions: () => import('$lib/panels/AuctionPanel.svelte'),
		mergers: () => import('$lib/panels/MergerPanel.svelte'),
		intel: () => import('$lib/panels/IntelPanel.svelte'),
		achievements: () => import('$lib/panels/AchievementPanel.svelte')
	};

	let PanelComponent: any = $state(null);

	$effect(() => {
		const panel = $activePanel;
		if (panel !== 'none' && panelComponents[panel]) {
			panelComponents[panel]().then((mod) => {
				PanelComponent = mod.default;
			});
		} else {
			PanelComponent = null;
		}
	});

	// Auto-start tutorial on first game
	$effect(() => {
		if ($initialized && !$tutorialCompleted) {
			startTutorial();
		}
	});
</script>

{#if $initialized}
	<div class="game-view">
		<MapView />
		<HUD />
		<InfoPanel />
		<BuildMenu />
		<NotificationFeed />
		<Tooltip />
		<Tutorial />
		{#if $isMultiplayer}
			<Chat />
		{/if}
		{#if PanelComponent}
			<div class="side-panel">
				<PanelComponent />
			</div>
		{/if}
	</div>
{:else}
	<div class="loading">
		<p>{$tr('game.loading')}</p>
	</div>
{/if}

<style>
	.game-view {
		width: 100vw;
		height: 100vh;
		position: relative;
		overflow: hidden;
		background: #0a0e17;
	}

	.loading {
		width: 100vw;
		height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #0a0e17;
		color: #9ca3af;
		font-family: system-ui, sans-serif;
		font-size: 16px;
	}

	.side-panel {
		position: absolute;
		left: 0;
		top: 48px;
		bottom: 0;
		width: 400px;
		background: rgba(17, 24, 39, 0.97);
		border-right: 1px solid var(--border);
		z-index: 15;
		overflow-y: auto;
	}
</style>
