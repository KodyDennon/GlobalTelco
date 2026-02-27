import * as bridge from '$lib/wasm/bridge';
import { eventType, eventData } from '$lib/wasm/types';
import type { GameEvent } from '$lib/wasm/types';
import {
	initialized,
	worldInfo,
	playerCorp,
	regions,
	cities,
	notifications,
	allCorporations,
	recordSnapshot
} from '$lib/stores/gameState';
import { recordNetworkSnapshot } from '$lib/stores/networkHistory';
import { saveToSlot, loadFromSlot, getNextAutoSaveSlot, QUICK_SAVE_SLOT } from '$lib/saves/SaveManager';
import { get } from 'svelte/store';
import { audioManager } from '$lib/audio/AudioManager';
import type { EraName } from '$lib/audio/AudioManager';
import {
	buildMode,
	activePanelGroup,
	buildEdgeSource,
	buildMenuLocation,
	selectedEntityId,
	selectedEntityType,
	activeOverlay,
	openPanelGroup,
	closePanelGroup,
} from '$lib/stores/uiState';
import { removeGhost } from '$lib/stores/multiplayerState';
import type { OverlayType } from '$lib/stores/uiState';
import { autoPauseOnCritical, showPerfMonitor } from '$lib/stores/settings';
import { writable } from 'svelte/store';

let running = false;
let animFrameId: number | null = null;
let lastTickTime = 0;
let tickAccumulator = 0;
let currentSpeed = 1; // ticks per second
let lastAutoSaveTick = 0;
const AUTO_SAVE_INTERVAL = 50; // ticks between auto-saves
let mpCleanupFns: Array<() => void> = [];
let isMultiplayerMode = false; // true when game is server-driven

// High-water mark: the highest tick we've applied from the server.
// Prevents stale snapshots from rolling back the displayed tick.
let mpHighWaterTick = 0;
// Guard to prevent concurrent snapshot loads from racing
let mpSnapshotLoading = false;

// Performance profiling stores
export const simTickTime = writable<number>(0);

// Loading stage for LoadingScreen (0-3)
export const loadingStage = writable<number>(0);

// Map asset loading complete (terrain bitmap, icon atlas, pathfinder)
export const mapReady = writable<boolean>(false);

// Auto-pause state
export const autoPauseReason = writable<string | null>(null);

// Welcome overlay shown on first game load
export const showWelcome = writable<boolean>(false);

/** Map a starting_era config string to an EraName for audio. */
function configEraToAudioEra(era?: string): EraName {
	switch (era?.toLowerCase()) {
		case 'telegraph': return 'telegraph';
		case 'telephone': return 'telephone';
		case 'earlydigital': case 'early_digital': return 'early_digital';
		case 'internet': return 'internet';
		case 'modern': return 'modern';
		case 'nearfuture': case 'near_future': return 'near_future';
		default: return 'internet';
	}
}

function getTickInterval(): number {
	switch (currentSpeed) {
		case 0:
			return Infinity; // paused
		case 1:
			return 1000;
		case 2:
			return 500;
		case 4:
			return 250;
		case 8:
			return 125;
		default:
			return 1000;
	}
}

function loop(timestamp: number) {
	if (!running) return;

	const delta = timestamp - lastTickTime;
	lastTickTime = timestamp;

	// In multiplayer, the server drives all simulation ticks.
	// The client never ticks locally — it only renders server state.
	if (!isMultiplayerMode && currentSpeed > 0) {
		tickAccumulator += delta;
		const interval = getTickInterval();

		while (tickAccumulator >= interval) {
			tickAccumulator -= interval;
			const t0 = performance.now();
			bridge.tick();
			const t1 = performance.now();
			simTickTime.set(Math.round((t1 - t0) * 100) / 100);
		}
	}

	if (!isMultiplayerMode) {
		updateStores();
	}
	animFrameId = requestAnimationFrame(loop);
}

function updateStores() {
	const info = bridge.getWorldInfo();
	worldInfo.set(info);

	if (info.player_corp_id > 0) {
		const corpData = bridge.getCorporationData(info.player_corp_id);
		playerCorp.set(corpData);

		// Record finance snapshot every 10 ticks
		if (info.tick % 10 === 0) {
			recordSnapshot(info.tick, corpData.revenue_per_tick, corpData.cost_per_tick, corpData.cash);

			// Record network snapshot for the Network Dashboard
			const trafficData = bridge.getTrafficFlows();
			const infraData = bridge.getInfrastructureList(info.player_corp_id);
			recordNetworkSnapshot(info.tick, trafficData, infraData);
		}

		// Update ambient music intensity based on game state
		if (corpData.cash !== undefined) {
			const profitRatio = corpData.profit_per_tick / Math.max(1, corpData.revenue_per_tick || 1);
			const cashHealth = Math.min(1, corpData.cash / 1_000_000);
			const intensity = Math.max(0, Math.min(1, 0.5 - profitRatio * 0.3 - cashHealth * 0.2));
			audioManager.setIntensity(intensity);
		}
	}

	// Update less frequently (every 10 frames roughly)
	const shouldUpdateFull = info.tick % 5 === 0;
	if (shouldUpdateFull) {
		regions.set(bridge.getRegions());
		cities.set(bridge.getCities());
		allCorporations.set(bridge.getAllCorporations());
	}

	const notifs = bridge.getNotifications();
	if (notifs.length > 0) {
		notifications.update((n) => [...notifs, ...n].slice(0, 50));
		for (const notif of notifs) {
			audioManager.playEventSound(notif.event);
		}

		// Auto-pause on critical events
		if (get(autoPauseOnCritical) && currentSpeed > 0) {
			for (const notif of notifs) {
				const reason = checkCriticalEvent(notif.event);
				if (reason) {
					setSpeed(0);
					autoPauseReason.set(reason);
					break;
				}
			}
		}
	}

	// Auto-save check
	if (info.tick > 0 && info.tick - lastAutoSaveTick >= AUTO_SAVE_INTERVAL) {
		lastAutoSaveTick = info.tick;
		performAutoSave(info.tick);
	}
}

function checkCriticalEvent(event: GameEvent): string | null {
	const type = eventType(event);
	const data = eventData(event);

	// Disasters with severity > 0.3
	if (type === 'DisasterStruck') {
		const severity = data.severity as number;
		if (severity > 0.3) {
			return `Major disaster: ${data.disaster_type ?? 'Unknown'}!`;
		}
	}
	// Insolvency / Bankruptcy
	if (type === 'InsolvencyWarning' || type === 'BankruptcyDeclared' || type === 'Bankruptcy') {
		return 'Financial crisis — insolvency warning!';
	}
	// Hostile acquisition
	if (type === 'AcquisitionProposed') {
		return 'Acquisition attempt!';
	}
	// Espionage detected
	if (type === 'EspionageDetected' || type === 'SabotageDetected') {
		return 'Espionage operation detected!';
	}
	return null;
}

async function performAutoSave(tick: number) {
	try {
		const data = bridge.saveGame();
		const corp = get(playerCorp);
		const slot = getNextAutoSaveSlot();
		await saveToSlot(slot, slot, data, tick, corp?.name ?? 'Unknown', 'Normal', 'Internet');
	} catch (e) {
		console.warn('[auto-save] Failed:', e);
		notifications.update((n) => [
			{ tick, event: { GlobalNotification: { message: 'Auto-save failed', level: 'warning' } } },
			...n
		].slice(0, 50));
	}
}

export function start() {
	if (running) return;
	running = true;
	lastTickTime = performance.now();
	tickAccumulator = 0;

	animFrameId = requestAnimationFrame(loop);
	setupKeyboardShortcuts();
}

export function stop() {
	running = false;
	if (animFrameId !== null) {
		cancelAnimationFrame(animFrameId);
		animFrameId = null;
	}
	teardownKeyboardShortcuts();
	audioManager.dispose();
	// Clean up multiplayer event listeners
	for (const fn of mpCleanupFns) fn();
	mpCleanupFns = [];
}

export function setSpeed(speed: number) {
	currentSpeed = speed;
	// Clear auto-pause reason and welcome overlay when resuming
	if (speed > 0) {
		autoPauseReason.set(null);
		showWelcome.set(false);
	}
	if (speed === 0) {
		bridge.processCommand({ SetSpeed: 'Paused' });
	} else {
		const speedMap: Record<number, string> = {
			1: 'Normal',
			2: 'Fast',
			4: 'VeryFast',
			8: 'Ultra',
			32: 'Ludicrous'
		};
		bridge.processCommand({ SetSpeed: speedMap[speed] || 'Normal' });
	}
}

export function togglePause() {
	if (currentSpeed === 0) {
		setSpeed(1);
	} else {
		setSpeed(0);
	}
}

/** Yield to the browser so the loading screen can repaint between stages. */
function yieldToUI(): Promise<void> {
	return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

export async function initGame(config?: Partial<import('$lib/wasm/types').WorldConfig>) {
	loadingStage.set(0);
	mapReady.set(false);
	await bridge.initWasm();
	loadingStage.set(1);

	// Yield so the loading screen renders "Generating World..."
	await yieldToUI();

	// Register handlers before any commands can be issued
	bridge.setErrorHandler((error, context) => {
		console.error(`WASM error in ${context}: ${error}`);
		if (context === 'tick') {
			setSpeed(0);
			notifications.update((n) => [
				{ tick: 0, event: { GlobalNotification: { message: `WASM Error: ${error}`, level: 'error' } } },
				...n
			].slice(0, 50));
		}
	});
	bridge.setCommandNotificationHandler((notifs) => {
		notifications.update((n) => [...notifs, ...n].slice(0, 50));
		for (const notif of notifs) {
			audioManager.playEventSound(notif.event);
		}
	});

	bridge.newGame(config);
	loadingStage.set(2);

	// Yield so the loading screen renders "Initializing Audio..."
	await yieldToUI();

	await audioManager.init();

	// Start era-appropriate ambient music
	const startingEra = (config as Record<string, unknown> | undefined)?.starting_era as string | undefined;
	audioManager.playMusic(configEraToAudioEra(startingEra));

	loadingStage.set(3);

	// Yield so the loading screen renders "Preparing Map..."
	await yieldToUI();

	// Start paused so player can orient
	setSpeed(0);
	showWelcome.set(true);
	initialized.set(true);
	updateStores();
}

export async function initMultiplayer(saveData: string) {
	isMultiplayerMode = true;
	mapReady.set(false);
	loadingStage.set(0);
	await bridge.initWasm();
	loadingStage.set(1);
	await yieldToUI();

	bridge.setErrorHandler((error, context) => {
		console.error(`WASM error in ${context}: ${error}`);
	});
	bridge.setCommandNotificationHandler((notifs) => {
		notifications.update((n) => [...notifs, ...n].slice(0, 50));
		for (const notif of notifs) {
			audioManager.playEventSound(notif.event);
		}
	});

	bridge.loadGame(saveData);
	loadingStage.set(2);
	await yieldToUI();
	await audioManager.init();

	// Start era music (default to internet era for multiplayer)
	audioManager.playMusic('internet');

	loadingStage.set(3);
	await yieldToUI();
	// Server drives ticks in multiplayer — no local tick advancement
	currentSpeed = 0;
	mpHighWaterTick = 0;
	mpSnapshotLoading = false;
	initialized.set(true);
	updateStores();

	// Listen for corp delta updates from WebSocket TickUpdate messages.
	// These arrive every server tick and carry lightweight financial deltas.
	// The tick value is authoritative — we only advance forward, never backward.
	const handleCorpDeltas = (e: Event) => {
		const { deltas, tick } = (e as CustomEvent).detail;
		if (!Array.isArray(deltas)) return;

		// Monotonic tick guard: never go backward
		if (tick < mpHighWaterTick) return;
		mpHighWaterTick = tick;

		// Tick is already set by WebSocketClient's TickUpdate handler.
		// We only apply corp financial deltas here — no redundant worldInfo update.
		const info = bridge.getWorldInfo();

		for (const delta of deltas) {
			const corpId = delta.corp_id as number;
			if (corpId === info.player_corp_id) {
				playerCorp.update((corp) => {
					if (!corp) return corp;
					return {
						...corp,
						cash: delta.cash ?? corp.cash,
						revenue_per_tick: delta.revenue ?? corp.revenue_per_tick,
						cost_per_tick: delta.cost ?? corp.cost_per_tick,
						debt: delta.debt ?? corp.debt,
						infrastructure_count: delta.node_count ?? corp.infrastructure_count,
					};
				});
			}
			allCorporations.update((corps) =>
				corps.map((c) => {
					if (c.id !== corpId) return c;
					return {
						...c,
						cash: delta.cash ?? c.cash,
						revenue: delta.revenue ?? c.revenue,
						cost: delta.cost ?? c.cost,
					};
				})
			);
		}
	};

	// Listen for full snapshot reloads (pushed by server every 5 ticks).
	// Snapshots fully replace WASM state. Guard against:
	//   1. Stale snapshots (tick < high-water mark)
	//   2. Concurrent loads (skip if another snapshot is being applied)
	const handleSnapshotReload = (e: Event) => {
		const { state_json, tick } = (e as CustomEvent).detail;

		// Skip stale snapshots — a newer tick update already arrived
		if (tick < mpHighWaterTick) {
			console.log(`[MP] Skipping stale snapshot tick=${tick} (current=${mpHighWaterTick})`);
			return;
		}

		// Prevent concurrent loadGame calls from racing
		if (mpSnapshotLoading) {
			console.log(`[MP] Skipping snapshot tick=${tick} (another load in progress)`);
			return;
		}

		try {
			mpSnapshotLoading = true;
			bridge.loadGame(state_json);
			mpHighWaterTick = tick;

			// Refresh stores from WASM, but force the tick to our high-water mark
			// to prevent any drift from the WASM internal tick counter.
			const info = bridge.getWorldInfo();
			worldInfo.set({ ...info, tick: mpHighWaterTick });

			if (info.player_corp_id > 0) {
				const corpData = bridge.getCorporationData(info.player_corp_id);
				playerCorp.set(corpData);
			}

			// Full entity refresh (regions, cities, corps) every snapshot
			regions.set(bridge.getRegions());
			cities.set(bridge.getCities());
			allCorporations.set(bridge.getAllCorporations());

			console.log(`[MP] Synced snapshot at tick ${tick}`);
		} catch (err) {
			console.error('[MP] Failed to reload snapshot:', err);
		} finally {
			mpSnapshotLoading = false;
		}
	};

	// Listen for command broadcasts (instant delta ops from other players' actions)
	const handleCommandBroadcast = (e: Event) => {
		const { tick, ops } = (e as CustomEvent).detail;
		if (!Array.isArray(ops) || ops.length === 0) return;

		// Apply deltas to local WASM state
		try {
			bridge.applyBatch(ops);
		} catch (err) {
			console.error('[MP] Failed to apply command broadcast:', err);
		}

		// Update tick if it's newer
		if (tick > mpHighWaterTick) {
			mpHighWaterTick = tick;
			worldInfo.update((info) => {
				if (!info) return info;
				return { ...info, tick };
			});
		}

		// Signal map to re-render infrastructure
		window.dispatchEvent(new CustomEvent('map-dirty'));
	};

	// Listen for command acks (confirmation of our own commands)
	const handleCommandAck = (e: Event) => {
		const { success, error, seq, entity_id, effective_tick } = (e as CustomEvent).detail;
		// Remove ghost entity for this seq (confirmed or rejected)
		if (seq != null) {
			removeGhost(seq);
		}
		if (!success && error) {
			notifications.update((n) => [
				{ tick: effective_tick || 0, event: { GlobalNotification: { message: error, level: 'warning' } } },
				...n
			].slice(0, 50));
			// Exit placement mode on build failure (e.g., insufficient funds)
			if (get(buildMode)) {
				buildMode.set(null);
				buildMenuLocation.set(null);
				buildEdgeSource.set(null);
			}
		}
		// On successful build, refresh corp data to update HUD counters immediately
		if (success && entity_id != null) {
			const info = bridge.getWorldInfo();
			if (info.player_corp_id > 0) {
				const corpData = bridge.getCorporationData(info.player_corp_id);
				playerCorp.set(corpData);
			}
			window.dispatchEvent(new CustomEvent('map-dirty'));
		}
	};

	window.addEventListener('mp-corp-deltas', handleCorpDeltas);
	window.addEventListener('mp-snapshot', handleSnapshotReload);
	window.addEventListener('mp-command-broadcast', handleCommandBroadcast);
	window.addEventListener('mp-command-ack', handleCommandAck);

	mpCleanupFns.push(
		() => window.removeEventListener('mp-corp-deltas', handleCorpDeltas),
		() => window.removeEventListener('mp-snapshot', handleSnapshotReload),
		() => window.removeEventListener('mp-command-broadcast', handleCommandBroadcast),
		() => window.removeEventListener('mp-command-ack', handleCommandAck),
		() => { isMultiplayerMode = false; mpHighWaterTick = 0; },
	);
}

export async function quickSave(): Promise<void> {
	if (!bridge.isInitialized()) return;
	const data = bridge.saveGame();
	const info = bridge.getWorldInfo();
	const corp = get(playerCorp);
	await saveToSlot(
		QUICK_SAVE_SLOT,
		'Quick Save',
		data,
		info.tick,
		corp?.name ?? 'Unknown',
		'Normal',
		'Internet'
	);
}

export async function quickLoad(): Promise<boolean> {
	const result = await loadFromSlot(QUICK_SAVE_SLOT);
	if (!result) return false;
	bridge.loadGame(result.data);
	lastAutoSaveTick = 0;
	updateStores();
	return true;
}

export async function loadFromSave(data: string): Promise<void> {
	bridge.loadGame(data);
	lastAutoSaveTick = 0;
	initialized.set(true);
	updateStores();
}

function setupKeyboardShortcuts() {
	window.addEventListener('keydown', handleKeyDown);
}

function teardownKeyboardShortcuts() {
	window.removeEventListener('keydown', handleKeyDown);
}

function handleKeyDown(e: KeyboardEvent) {
	if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement) return;

	switch (e.key) {
		case 'F5':
			e.preventDefault();
			quickSave();
			break;
		case 'F9':
			e.preventDefault();
			quickLoad();
			break;
		case ' ':
			e.preventDefault();
			togglePause();
			break;
		case 'b':
		case 'B':
			e.preventDefault();
			buildMode.update((m) => {
				if (m === 'node') {
					buildMenuLocation.set(null);
					return null;
				}
				buildEdgeSource.set(null);
				return 'node';
			});
			break;
		case 'e':
		case 'E':
			e.preventDefault();
			buildMode.update((m) => {
				if (m === 'edge') {
					buildEdgeSource.set(null);
					return null;
				}
				buildMenuLocation.set(null);
				return 'edge';
			});
			break;
		case '1':
			e.preventDefault();
			setSpeed(1);
			break;
		case '2':
			e.preventDefault();
			setSpeed(2);
			break;
		case '3':
			e.preventDefault();
			setSpeed(4);
			break;
		case '4':
			e.preventDefault();
			setSpeed(8);
			break;
		case 'Escape':
			e.preventDefault();
			// Close panel, cancel build mode, or deselect entity
			if (get(buildMode)) {
				buildMode.set(null);
				buildMenuLocation.set(null);
				buildEdgeSource.set(null);
			} else if (get(selectedEntityId) !== null) {
				selectedEntityId.set(null);
				selectedEntityType.set(null);
			} else if (get(activePanelGroup) !== 'none') {
				closePanelGroup();
			}
			break;
		case 'F3':
			e.preventDefault();
			showPerfMonitor.update((v) => !v);
			break;
		// Panel group shortcuts
		case 'd':
		case 'D':
			e.preventDefault();
			if (get(activePanelGroup) === 'finance') closePanelGroup();
			else openPanelGroup('finance');
			break;
		case 'i':
		case 'I':
			e.preventDefault();
			if (get(activePanelGroup) === 'operations') closePanelGroup();
			else openPanelGroup('operations');
			break;
		case 'r':
		case 'R':
			e.preventDefault();
			if (get(activePanelGroup) === 'research') closePanelGroup();
			else openPanelGroup('research');
			break;
		case 'c':
		case 'C':
			e.preventDefault();
			if (get(activePanelGroup) === 'market') closePanelGroup();
			else openPanelGroup('market');
			break;
		case 'w':
		case 'W':
			e.preventDefault();
			if (get(activePanelGroup) === 'operations') closePanelGroup();
			else openPanelGroup('operations', 'workforce');
			break;
		// Overlay shortcuts
		case 't':
		case 'T':
			e.preventDefault();
			activeOverlay.update((o) => o === 'terrain' ? 'none' : 'terrain');
			break;
		case 'o':
		case 'O':
			e.preventDefault();
			activeOverlay.update((o) => o === 'ownership' ? 'none' : 'ownership');
			break;
	}
}

export function isRunning(): boolean {
	return running;
}
