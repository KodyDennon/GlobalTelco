<script lang="ts">
	import { tr } from "$lib/i18n/index";
	import LoadingScreen from "./LoadingScreen.svelte";
	import MapView from "./MapView.svelte";
	import HUD from "./HUD.svelte";
	import InfoPanel from "./InfoPanel.svelte";
	import RadialBuildMenu from "./RadialBuildMenu.svelte";
	import BuildHotbar from "./BuildHotbar.svelte";
	import NotificationFeed from "./NotificationFeed.svelte";
	import Tooltip from "./Tooltip.svelte";
	import Tutorial from "./Tutorial.svelte";
	import Chat from "./Chat.svelte";
	import CableDrawingMode from "./CableDrawingMode.svelte";
	import MiniMap from "./MiniMap.svelte";
	import BookmarkManager from "./BookmarkManager.svelte";
	import SearchOverlay from "./SearchOverlay.svelte";
	import OverlayLegend from "./OverlayLegend.svelte";
	import PerfMonitor from "./PerfMonitor.svelte";
	import FloatingPanel from "$lib/ui/FloatingPanel.svelte";
	import ConfirmDialog from "$lib/ui/ConfirmDialog.svelte";
	import ComingSoon from "$lib/ui/ComingSoon.svelte";
	import { initialized, playerCorp, regions, notifications, worldInfo } from "$lib/stores/gameState";
	import { eventType, eventData } from "$lib/wasm/types";
	import type { ActiveDisaster, ForecastDisaster } from "./WeatherLayer";
	import { DISASTER_DISPLAY_DURATION, computeDisasterForecasts } from "./WeatherLayer";
	import * as bridge from "$lib/wasm/bridge";
	import DisasterAlert from "./DisasterAlert.svelte";
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
	import { loadingStage, showWelcome, setSpeed, mapReady } from "./GameLoop";
	import { KeyboardManager, createDefaultBindings, hotkeyOverlayVisible } from "./KeyboardManager";
	import { EventEffectManager } from "./EventEffects";
	import { ScreenshotManager } from "./ScreenshotMode";
	import { AudioManager } from "$lib/audio/AudioManager";
	import { SpatialAudioController } from "$lib/audio/SpatialAudio";
	import { onMount, onDestroy, untrack } from "svelte";

	let gameViewEl: HTMLElement = $state(null!);
	let bookmarksPanelVisible = $state(false);
	let keyboardManager: KeyboardManager | null = null;
	let effectManager: EventEffectManager | null = null;
	let screenshotManager: ScreenshotManager | null = null;
	let audioManager: AudioManager | null = null;
	let spatialAudio: SpatialAudioController | null = null;

	// ── Active disaster tracking ─────────────────────────────────────────────
	// Disasters are extracted from notifications and kept alive for DISASTER_DISPLAY_DURATION ticks.
	let activeDisasters: ActiveDisaster[] = $state([]);
	let processedDisasterIds = new Set<string>();

	// Derive active disasters from notifications: scan for DisasterStruck events
	$effect(() => {
		const notifs = $notifications;
		const currentTick = $worldInfo.tick;
		const regionList = $regions;
		let updated = false;
		let currentDisasters = untrack(() => activeDisasters);

		// Add new disasters from recent notifications
		for (const notif of notifs) {
			const type = eventType(notif.event);
			if (type !== 'DisasterStruck') continue;

			const data = eventData(notif.event);
			const id = `disaster-${notif.tick}-${data.region}`;
			if (processedDisasterIds.has(id)) continue;
			processedDisasterIds.add(id);

			// Find region center for positioning
			const region = regionList.find(r => r.id === (data.region as number));
			const lon = region?.center_lon ?? 0;
			const lat = region?.center_lat ?? 0;
			const regionName = region?.name ?? `Region ${data.region}`;

			currentDisasters = [...currentDisasters, {
				id,
				disasterType: (data.disaster_type as string) ?? 'Unknown',
				lon,
				lat,
				severity: (data.severity as number) ?? 0.3,
				startTick: notif.tick,
				regionName,
				regionId: (data.region as number) ?? 0,
				affectedCount: (data.affected_nodes as number) ?? 0,
			}];
			updated = true;
		}

		// Prune expired disasters
		const initialCount = currentDisasters.length;
		currentDisasters = currentDisasters.filter(
			d => (currentTick - d.startTick) < DISASTER_DISPLAY_DURATION
		);
		if (currentDisasters.length !== initialCount) updated = true;

		if (updated) {
			activeDisasters = currentDisasters;

			// Dispatch to MapView for weather layer + infra layer visualization
			window.dispatchEvent(new CustomEvent('active-disasters-update', {
				detail: activeDisasters,
			}));
		}
	});

	// ── Disaster forecast tracking ────────────────────────────────────────────
	// Recompute forecasts every 10 ticks (bucketed for stability, since seed includes tick).
	let disasterForecasts: ForecastDisaster[] = $state([]);
	let lastForecastBucket = -1;

	$effect(() => {
		const currentTick = $worldInfo.tick;
		const regionList = $regions;
		const bucket = Math.floor(currentTick / 10);
		if (bucket === lastForecastBucket) return;
		lastForecastBucket = bucket;
		disasterForecasts = computeDisasterForecasts(regionList, bucket);
	});

	onMount(() => {
		// Keyboard shortcuts
		keyboardManager = new KeyboardManager();
		createDefaultBindings(keyboardManager);

		// Bind screenshot (F12) — screenshots need the map container
		keyboardManager.bind('f12', () => {
			const mapContainer = gameViewEl?.querySelector('.map-container') as HTMLElement;
			if (mapContainer && screenshotManager) {
				screenshotManager.captureScreenshot(mapContainer, $playerCorp?.name);
			}
		});

		// Bind bookmark panel toggle (Shift+B — plain 'b' is node build mode)
		keyboardManager.bind('shift+b', () => {
			bookmarksPanelVisible = !bookmarksPanelVisible;
		});

		keyboardManager.attach();

		// Event effects (CSS-based visual effects for game events)
		if (gameViewEl) {
			effectManager = new EventEffectManager(gameViewEl);
		}

		// Screenshot manager
		screenshotManager = new ScreenshotManager();

		// Audio (lazy init on first interaction — browser requirement)
		audioManager = new AudioManager();
		spatialAudio = new SpatialAudioController(audioManager);

		// Init audio on first click (browsers require user gesture)
		const initAudio = () => {
			audioManager?.init();
			window.removeEventListener('click', initAudio);
			window.removeEventListener('keydown', initAudio);
		};
		window.addEventListener('click', initAudio, { once: true });
		window.addEventListener('keydown', initAudio, { once: true });

		// Listen for game events to trigger effects + audio
		const handleGameEvent = (e: Event) => {
			const detail = (e as CustomEvent).detail;
			if (detail?.event) {
				effectManager?.triggerEffect(detail.event);
				// Play SFX for specific events
				if (detail.event.type === 'EarthquakeStruck' || detail.event.type === 'Earthquake') {
					audioManager?.playSfx('earthquake');
				} else if (detail.event.type === 'StormStruck' || detail.event.type === 'Storm') {
					audioManager?.playSfx('storm');
				} else if (detail.event.type === 'ConstructionComplete') {
					audioManager?.playSfx('build');
				} else if (detail.event.type === 'ResearchComplete') {
					audioManager?.playSfx('research_complete');
				} else if (detail.event.type === 'ContractSigned') {
					audioManager?.playSfx('contract_signed');
				}
			}
		};
		window.addEventListener('game-event', handleGameEvent);

		return () => {
			window.removeEventListener('game-event', handleGameEvent);
		};
	});

	onDestroy(() => {
		keyboardManager?.dispose();
		effectManager?.dispose();
		audioManager?.dispose();
	});

	// Lazy-load panels only when needed
	const panelComponents: Record<string, () => Promise<any>> = {
		dashboard: () => import("$lib/panels/DashboardPanel.svelte"),
		infrastructure: () => import("$lib/panels/InfraPanel.svelte"),
		network: () => import("$lib/panels/NetworkDashboard.svelte"),
		contracts: () => import("$lib/panels/ContractPanel.svelte"),
		region: () => import("$lib/panels/RegionPanel.svelte"),
		research: () => import("$lib/panels/ResearchPanel.svelte"),
		workforce: () => import("$lib/panels/WorkforcePanel.svelte"),
		advisor: () => import("$lib/panels/AdvisorPanel.svelte"),
		auctions: () => import("$lib/panels/AuctionPanel.svelte"),
		mergers: () => import("$lib/panels/MergerPanel.svelte"),
		intel: () => import("$lib/panels/IntelPanel.svelte"),
		achievements: () => import("$lib/panels/AchievementPanel.svelte"),
		spectrum: () => import("$lib/panels/SpectrumPanel.svelte"),
		insurance: () => import("$lib/panels/InsurancePanel.svelte"),
		repair: () => import("$lib/panels/RepairPanel.svelte"),
		coownership: () => import("$lib/panels/CoOwnershipPanel.svelte"),
		grants: () => import("$lib/panels/GrantPanel.svelte"),
		pricing: () => import("$lib/panels/PricingPanel.svelte"),
		maintenance: () => import("$lib/panels/MaintenancePanel.svelte"),
		alliance: () => import("$lib/panels/AlliancePanel.svelte"),
		legal: () => import("$lib/panels/LegalPanel.svelte"),
		patents: () => import("$lib/panels/PatentPanel.svelte"),
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
	<div class="game-view" bind:this={gameViewEl}>
		<MapView />
		{#if !$mapReady}
			<div class="map-loading-overlay">
				<div class="map-loading-card">
					<div class="map-loading-spinner"></div>
					<span class="map-loading-text">Preparing map...</span>
				</div>
			</div>
		{/if}
		<HUD />
		<DisasterAlert disasters={activeDisasters} forecasts={disasterForecasts} />
		<CableDrawingMode />
		<InfoPanel />
		<RadialBuildMenu />
		<BuildHotbar />
		<NotificationFeed />
		<Tooltip />
		<Tutorial />
		<MiniMap />
		<!-- Bookmark button + floating panel (bottom-right, near minimap) -->
		<button
			class="bookmark-toggle-btn"
			title="Bookmarks (Shift+B)"
			onclick={() => bookmarksPanelVisible = !bookmarksPanelVisible}
			class:active={bookmarksPanelVisible}
		>
			<svg width="16" height="16" viewBox="0 0 16 16" fill="none">
				<path d="M3 2h10v13l-5-3.5L3 15V2z" stroke="currentColor" stroke-width="1.5" fill={bookmarksPanelVisible ? 'currentColor' : 'none'} />
			</svg>
		</button>
		{#if bookmarksPanelVisible}
			<div class="bookmark-floating-panel">
				<BookmarkManager />
			</div>
		{/if}
		<SearchOverlay />
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
		{#if $hotkeyOverlayVisible}
			<div class="hotkey-overlay" role="dialog">
				<div class="hotkey-card">
					<h3 class="hotkey-title">Keyboard Shortcuts</h3>
					<div class="hotkey-grid">
						<div class="hotkey-section">
							<h4>Game</h4>
							<div class="hk"><kbd>Space</kbd> Pause / Resume</div>
							<div class="hk"><kbd>Shift+1-4</kbd> Set Speed</div>
							<div class="hk"><kbd>Ctrl+S</kbd> Quick Save</div>
						</div>
						<div class="hotkey-section">
							<h4>Build</h4>
							<div class="hk"><kbd>1-9</kbd> Hotbar Slots</div>
							<div class="hk"><kbd>Right-click</kbd> Radial Menu</div>
							<div class="hk"><kbd>B</kbd> Node Build Mode</div>
							<div class="hk"><kbd>E</kbd> Edge Build Mode</div>
							<div class="hk"><kbd>Esc</kbd> Cancel</div>
						</div>
						<div class="hotkey-section">
							<h4>Overlays</h4>
							<div class="hk"><kbd>T</kbd> Terrain</div>
							<div class="hk"><kbd>O</kbd> Ownership</div>
							<div class="hk"><kbd>D</kbd> Demand</div>
							<div class="hk"><kbd>C</kbd> Coverage</div>
							<div class="hk"><kbd>R</kbd> Risk</div>
							<div class="hk"><kbd>G</kbd> Congestion</div>
							<div class="hk"><kbd>F</kbd> Traffic Flow</div>
						</div>
						<div class="hotkey-section">
							<h4>Panels</h4>
							<div class="hk"><kbd>F1-F6</kbd> Panel Groups</div>
							<div class="hk"><kbd>Tab</kbd> Next Tab</div>
							<div class="hk"><kbd>Q</kbd> Close Panel</div>
						</div>
						<div class="hotkey-section">
							<h4>Navigation</h4>
							<div class="hk"><kbd>/</kbd> Search (cities, regions)</div>
							<div class="hk"><kbd>M</kbd> Toggle Minimap</div>
							<div class="hk"><kbd>Shift+B</kbd> Bookmarks</div>
							<div class="hk"><kbd>Home</kbd> Reset View</div>
							<div class="hk"><kbd>P</kbd> Toggle 3D Pitch</div>
						</div>
					</div>
					<button class="hotkey-close" onclick={() => hotkeyOverlayVisible.set(false)}>Close</button>
				</div>
			</div>
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

	.hotkey-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.7);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 100;
	}

	.hotkey-card {
		background: rgba(17, 24, 39, 0.98);
		border: 1px solid rgba(55, 65, 81, 0.6);
		border-radius: 12px;
		padding: 24px 32px;
		max-width: 600px;
		width: 90vw;
		max-height: 80vh;
		overflow-y: auto;
	}

	.hotkey-title {
		font-size: 18px;
		font-weight: 700;
		color: #f3f4f6;
		margin: 0 0 16px;
		text-align: center;
	}

	.hotkey-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 16px;
	}

	.hotkey-section h4 {
		font-size: 13px;
		font-weight: 600;
		color: #9ca3af;
		margin: 0 0 8px;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.hk {
		font-size: 12px;
		color: #d1d5db;
		margin: 4px 0;
	}

	.hk :global(kbd) {
		display: inline-block;
		background: rgba(55, 65, 81, 0.5);
		border: 1px solid rgba(75, 85, 99, 0.5);
		border-radius: 4px;
		padding: 1px 6px;
		font-family: monospace;
		font-size: 11px;
		color: #e5e7eb;
		margin-right: 6px;
		min-width: 20px;
		text-align: center;
	}

	.hotkey-close {
		display: block;
		margin: 16px auto 0;
		background: rgba(55, 65, 81, 0.5);
		color: #d1d5db;
		border: 1px solid rgba(75, 85, 99, 0.4);
		padding: 8px 24px;
		border-radius: 6px;
		font-size: 13px;
		cursor: pointer;
	}

	.hotkey-close:hover {
		background: rgba(75, 85, 99, 0.5);
	}

	/* Map loading overlay — shown while terrain/icons are building after init */
	.map-loading-overlay {
		position: absolute;
		inset: 0;
		z-index: 80;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(6, 16, 31, 0.85);
		backdrop-filter: blur(4px);
		animation: fade-in 0.2s ease;
	}

	@keyframes fade-in {
		from { opacity: 0; }
		to { opacity: 1; }
	}

	.map-loading-card {
		display: flex;
		align-items: center;
		gap: 14px;
		background: rgba(17, 24, 39, 0.9);
		border: 1px solid rgba(55, 65, 81, 0.4);
		border-radius: 10px;
		padding: 16px 28px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
	}

	.map-loading-spinner {
		width: 20px;
		height: 20px;
		border: 2.5px solid rgba(59, 130, 246, 0.2);
		border-top: 2.5px solid #3b82f6;
		border-radius: 50%;
		animation: map-spin 0.8s linear infinite;
	}

	@keyframes map-spin {
		to { transform: rotate(360deg); }
	}

	.map-loading-text {
		font-size: 14px;
		font-weight: 500;
		color: #9ca3af;
		letter-spacing: 0.02em;
	}

	/* ── Bookmark toggle button + floating panel ─────────────────────── */

	.bookmark-toggle-btn {
		position: fixed;
		bottom: 180px;
		right: 16px;
		z-index: 50;
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(13, 18, 30, 0.9);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		color: #9ca3af;
		cursor: pointer;
		transition: all 0.15s;
	}

	.bookmark-toggle-btn:hover {
		background: rgba(30, 41, 59, 0.95);
		color: #f3f4f6;
		border-color: rgba(59, 130, 246, 0.5);
	}

	.bookmark-toggle-btn.active {
		background: rgba(59, 130, 246, 0.15);
		border-color: rgba(59, 130, 246, 0.5);
		color: #60a5fa;
	}

	.bookmark-floating-panel {
		position: fixed;
		bottom: 220px;
		right: 16px;
		z-index: 55;
		width: 280px;
		max-height: 320px;
		overflow-y: auto;
		background: rgba(13, 18, 30, 0.96);
		border: 1px solid rgba(55, 65, 81, 0.6);
		border-radius: 8px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(255, 255, 255, 0.03);
		animation: bm-fade-in 0.12s ease;
	}

	@keyframes bm-fade-in {
		from { opacity: 0; transform: translateY(4px); }
		to { opacity: 1; transform: translateY(0); }
	}
</style>
