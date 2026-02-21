<script lang="ts">
	import { tr } from "$lib/i18n/index";
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
	import { initialized } from "$lib/stores/gameState";
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
	<div class="loading">
		<p>{$tr("game.loading")}</p>
	</div>
{/if}

<style>
	.game-view {
		width: 100vw;
		height: 100vh;
		position: relative;
		overflow: hidden;
		background: #06101f;
	}

	.loading {
		width: 100vw;
		height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #06101f;
		color: #9ca3af;
		font-family: system-ui, sans-serif;
		font-size: 16px;
	}
</style>
