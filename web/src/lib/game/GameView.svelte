<script lang="ts">
	import { tr } from "$lib/i18n/index";
	import LoadingScreen from "./LoadingScreen.svelte";
	import MapView from "./MapView.svelte";
	import HUD from "./HUD.svelte";
	import InfoPanel from "./InfoPanel.svelte";
	import BuildMenu from "./BuildMenu.svelte";
	import NotificationFeed from "./NotificationFeed.svelte";
	import Tooltip from "./Tooltip.svelte";
	import Tutorial from "./Tutorial.svelte";
	import Chat from "./Chat.svelte";
	import MiniMap from "./MiniMap.svelte";
	import OverlayLegend from "./OverlayLegend.svelte";
	import PerfMonitor from "./PerfMonitor.svelte";
	import FloatingPanel from "$lib/ui/FloatingPanel.svelte";
	import ConfirmDialog from "$lib/ui/ConfirmDialog.svelte";
	import ComingSoon from "$lib/ui/ComingSoon.svelte";
	import { initialized, playerCorp, regions } from "$lib/stores/gameState";
	import {
		activePanelGroup,
		activeGroupTab,
		activeOverlay,
		PANEL_GROUP_TABS,
		PANEL_GROUP_NAMES,
		openPanelGroup,
		closePanelGroup,
	} from "$lib/stores/uiState";
	import type { PanelGroupType } from "$lib/stores/uiState";
	import {
		tutorialCompleted,
		startTutorial,
	} from "$lib/stores/tutorialState";
	import { isMultiplayer } from "$lib/stores/multiplayerState";
	import { showPerfMonitor } from "$lib/stores/settings";
	import { loadingStage, showWelcome, setSpeed } from "./GameLoop";

	// Lazy-load panels only when needed
	const panelComponents: Record<string, () => Promise<any>> = {
		dashboard: () => import("$lib/panels/DashboardPanel.svelte"),
		infrastructure: () => import("$lib/panels/InfraPanel.svelte"),
		contracts: () => import("$lib/panels/ContractPanel.svelte"),
		region: () => import("$lib/panels/RegionPanel.svelte"),
		research: () => import("$lib/panels/ResearchPanel.svelte"),
		workforce: () => import("$lib/panels/WorkforcePanel.svelte"),
		advisor: () => import("$lib/panels/AdvisorPanel.svelte"),
		auctions: () => import("$lib/panels/AuctionPanel.svelte"),
		mergers: () => import("$lib/panels/MergerPanel.svelte"),
		intel: () => import("$lib/panels/IntelPanel.svelte"),
		achievements: () => import("$lib/panels/AchievementPanel.svelte"),
	};

	let PanelComponent: any = $state(null);

	// Load panel component when tab changes
	$effect(() => {
		const group = $activePanelGroup;
		const tab = $activeGroupTab;
		if (group === 'none' || !tab) {
			PanelComponent = null;
			return;
		}
		const tabs = PANEL_GROUP_TABS[group];
		const tabDef = tabs?.find(t => t.key === tab);
		if (tabDef?.component && panelComponents[tabDef.component]) {
			panelComponents[tabDef.component]().then((mod) => {
				PanelComponent = mod.default;
			});
		} else {
			PanelComponent = null;
		}
	});

	// Current tab definition (for coming-soon check)
	let currentTabDef = $derived.by(() => {
		const group = $activePanelGroup;
		const tab = $activeGroupTab;
		if (group === 'none') return null;
		const tabs = PANEL_GROUP_TABS[group];
		return tabs?.find(t => t.key === tab) ?? null;
	});

	// Overlay legend config per overlay type
	const OVERLAY_LEGENDS: Record<string, { title: string; gradient: Array<{ color: string; label: string }> }> = {
		demand: {
			title: 'Demand',
			gradient: [
				{ color: '#3b82f6', label: 'Low' },
				{ color: '#8b5cf6', label: 'Med' },
				{ color: '#ef4444', label: 'High' },
			]
		},
		coverage: {
			title: 'Coverage',
			gradient: [
				{ color: '#ef4444', label: '0%' },
				{ color: '#eab308', label: '50%' },
				{ color: '#22c55e', label: '100%' },
			]
		},
		disaster: {
			title: 'Disaster Risk',
			gradient: [
				{ color: '#22c55e', label: 'Low' },
				{ color: '#eab308', label: 'Med' },
				{ color: '#ef4444', label: 'High' },
			]
		},
		congestion: {
			title: 'Congestion',
			gradient: [
				{ color: '#22c55e', label: 'Free' },
				{ color: '#f59e0b', label: 'Busy' },
				{ color: '#ef4444', label: 'Full' },
			]
		},
		traffic: {
			title: 'Traffic Flow',
			gradient: [
				{ color: '#3b82f6', label: 'Low' },
				{ color: '#22d3ee', label: 'Med' },
				{ color: '#ffffff', label: 'High' },
			]
		},
	};

	let legendConfig = $derived(OVERLAY_LEGENDS[$activeOverlay] ?? null);

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
		<MiniMap />
		<ConfirmDialog />
		{#if $showWelcome}
			<div class="welcome-overlay">
				<div class="welcome-card">
					<h2 class="welcome-title">Welcome to {$playerCorp?.name ?? 'your corporation'}!</h2>
					{#if $regions.length > 0}
						<p class="welcome-text">Your starting region is <strong>{$regions[0]?.name}</strong>. Build infrastructure, expand your network, and grow into a global telecom empire.</p>
					{:else}
						<p class="welcome-text">Build infrastructure, expand your network, and grow into a global telecom empire.</p>
					{/if}
					<button class="welcome-btn" onclick={() => setSpeed(1)}>Start Playing</button>
					<p class="welcome-hint">or press Spacebar</p>
				</div>
			</div>
		{/if}
		{#if $isMultiplayer}
			<Chat />
		{/if}
		{#if $showPerfMonitor}
			<PerfMonitor />
		{/if}
		<OverlayLegend
			title={legendConfig?.title ?? ''}
			gradient={legendConfig?.gradient ?? []}
			visible={legendConfig !== null}
		/>
		{#if $activePanelGroup !== 'none'}
			{@const group = $activePanelGroup as PanelGroupType}
			{@const tabs = PANEL_GROUP_TABS[group]}
			<FloatingPanel
				title={PANEL_GROUP_NAMES[group]}
				groupId={group}
				{tabs}
				activeTab={$activeGroupTab}
				onclose={closePanelGroup}
				ontabchange={(tab) => openPanelGroup(group, tab)}
			>
				{#if currentTabDef?.comingSoon}
					<ComingSoon
						feature={currentTabDef.comingSoon.feature}
						phase={currentTabDef.comingSoon.phase}
						description={currentTabDef.comingSoon.description}
					/>
				{:else if PanelComponent}
					<PanelComponent />
				{/if}
			</FloatingPanel>
		{/if}
	</div>
{:else}
	<LoadingScreen stage={$loadingStage} />
{/if}

<style>
	.game-view {
		width: 100vw;
		height: 100vh;
		position: relative;
		overflow: hidden;
		background: #06101f;
	}

	.welcome-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 90;
	}

	.welcome-card {
		background: rgba(17, 24, 39, 0.98);
		border: 1px solid rgba(55, 65, 81, 0.6);
		border-radius: 12px;
		padding: 32px 40px;
		max-width: 440px;
		text-align: center;
		box-shadow: 0 12px 48px rgba(0, 0, 0, 0.5);
	}

	.welcome-title {
		font-size: 22px;
		font-weight: 700;
		background: linear-gradient(90deg, #10b981, #3b82f6);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
		margin: 0 0 12px;
	}

	.welcome-text {
		font-size: 14px;
		color: #9ca3af;
		line-height: 1.6;
		margin: 0 0 24px;
	}

	.welcome-text strong {
		color: #f3f4f6;
	}

	.welcome-btn {
		background: linear-gradient(135deg, #10b981, #3b82f6);
		color: white;
		border: none;
		padding: 12px 32px;
		border-radius: 8px;
		font-size: 15px;
		font-weight: 600;
		cursor: pointer;
		transition: opacity 0.15s;
	}

	.welcome-btn:hover {
		opacity: 0.9;
	}

	.welcome-hint {
		font-size: 12px;
		color: #6b7280;
		margin: 12px 0 0;
	}
</style>
