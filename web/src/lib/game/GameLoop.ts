import * as bridge from '$lib/wasm/bridge';
import * as workerBridge from '$lib/wasm/workerBridge';
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
	viewport,
} from '$lib/stores/uiState';
import { removeGhost, speedVotes, corpId } from '$lib/stores/multiplayerState';
import type { OverlayType } from '$lib/stores/uiState';
import { autoPauseOnCritical, showPerfMonitor, autoSaveInterval } from '$lib/stores/settings';
import { writable } from 'svelte/store';

let running = false;
let animFrameId: number | null = null;
let lastTickTime = 0;
let tickAccumulator = 0;
let currentSpeed = 1; // ticks per second
let lastAutoSaveTick = 0;
// Auto-save interval read from settings store (default 50 ticks)
let mpCleanupFns: Array<() => void> = [];
let isMultiplayerMode = false; // true when game is server-driven
let tickInFlight = false; // prevents overlapping async ticks (native sim or worker)
let useWorker = false; // true when sim runs in a Web Worker
let workerTickStart = 0; // timestamp when worker tick was requested

// High-water mark: the highest tick we've applied from the server.
// Prevents stale snapshots from rolling back the displayed tick.
let mpHighWaterTick = 0;
// Guard to prevent concurrent snapshot loads from racing
let mpSnapshotLoading = false;

// Performance profiling stores
export const simTickTime = writable<number>(0);

// Compute Pressure API — adaptive quality (Phase 5.6)
export type PressureState = 'nominal' | 'fair' | 'serious' | 'critical';
export const cpuPressure = writable<PressureState>('nominal');
let pressureObserver: any = null;

function initPressureObserver() {
	if (typeof (globalThis as any).PressureObserver === 'undefined') return;
	try {
		pressureObserver = new (globalThis as any).PressureObserver((records: any[]) => {
			if (records.length > 0) {
				cpuPressure.set(records[0].state as PressureState);
			}
		});
		pressureObserver.observe('cpu', { sampleInterval: 2000 });
	} catch { /* unsupported — no-op */ }
}

function teardownPressureObserver() {
	if (pressureObserver) {
		try { pressureObserver.disconnect(); } catch { }
		pressureObserver = null;
	}
}

/** Schedule a background task using Scheduler API if available, else setTimeout. */
function scheduleBackground(fn: () => void) {
	if (typeof (globalThis as any).scheduler?.postTask === 'function') {
		(globalThis as any).scheduler.postTask(fn, { priority: 'background' });
	} else {
		setTimeout(fn, 0);
	}
}

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

		if (useWorker) {
			// Worker mode: non-blocking tick via Web Worker.
			// Only request one tick at a time (worker processes asynchronously).
			if (!tickInFlight && tickAccumulator >= interval) {
				tickAccumulator -= interval;
				tickInFlight = true;
				workerTickStart = performance.now();
				const v = get(viewport);
				workerBridge.requestTick([v.minX, v.minY, v.maxX, v.maxY]);
			}
		} else if (bridge.isNativeSim()) {
			// Native sim: async tick via Tauri IPC. Only one in flight at a time.
			if (!tickInFlight && tickAccumulator >= interval) {
				tickAccumulator -= interval;
				tickInFlight = true;
				const t0 = performance.now();
				bridge.tick().then(() => {
					const t1 = performance.now();
					simTickTime.set(Math.round((t1 - t0) * 100) / 100);
					tickInFlight = false;
				}).catch((e) => {
					console.error('[GameLoop] Native tick failed:', e);
					tickInFlight = false;
				});
			}
		} else {
			// WASM: synchronous ticks (fallback — main thread)
			let ticked = false;
			while (tickAccumulator >= interval) {
				tickAccumulator -= interval;
				const t0 = performance.now();
				bridge.tick();
				const t1 = performance.now();
				simTickTime.set(Math.round((t1 - t0) * 100) / 100);
				ticked = true;
			}
			if (ticked) {
				updateStores();
			}
		}
	}

	// In worker mode, store updates happen via the tick result handler.
	// In direct mode, we already updated stores if a tick occurred.
	// We only need a fallback for the first frame or initial state.
	// However, initGame already calls updateStores once.
	animFrameId = requestAnimationFrame(loop);
}

/** Handle tick results from the Web Worker. Called asynchronously when worker completes a tick. */
function handleWorkerTickResult(result: workerBridge.TickResult) {
	tickInFlight = false;
	const t1 = performance.now();
	simTickTime.set(Math.round((t1 - workerTickStart) * 100) / 100);

	// Sync worker's latest state with the bridge for UI consumers (prevents staleness)
	bridge.setLatestTickResult(result);

	const info = result.info ? JSON.parse(result.info) : null;

	try {
		if (info) {
			worldInfo.set(info);
		}
		// Update player corp from worker result
		if (result.playerCorp) {
			try {
				const corpData = JSON.parse(result.playerCorp);
				playerCorp.set(corpData);

				if (info) {
					// Record finance snapshot every 10 ticks
					if (info.tick % 10 === 0) {
						recordSnapshot(info.tick, corpData.revenue_per_tick, corpData.cost_per_tick, corpData.cash);
					}
				}

				// Update ambient music intensity based on game state
				if (corpData.cash !== undefined) {
					const profitRatio = corpData.profit_per_tick / Math.max(1, corpData.revenue_per_tick || 1);
					const cashHealth = Math.min(1, corpData.cash / 1_000_000);
					const intensity = Math.max(0, Math.min(1, 0.5 - profitRatio * 0.3 - cashHealth * 0.2));
					audioManager.setIntensity(intensity);
				}
			} catch { /* ignore corp parse errors */ }
		}

		// Record network snapshot every 10 ticks (same frequency as direct mode)
		if (info && info.tick % 10 === 0) {
			const trafficPromise = workerBridge.query('get_traffic_flows');
			const infraPromise = workerBridge.query('get_infrastructure_list', BigInt(info.player_corp_id));
			Promise.all([
				trafficPromise,
				infraPromise,
			]).then(([trafficJson, infraJson]) => {
				if (trafficJson && infraJson) {
					try {
						recordNetworkSnapshot(info.tick, JSON.parse(trafficJson), JSON.parse(infraJson));
					} catch { }
				}
			}).catch(() => { });
		}

		// Update less frequently (every 5th tick) via async worker queries
		const shouldUpdateFullAsync = info && (info.tick % 5 === 0 || info.tick === 0 || get(allCorporations).length === 0);
		if (shouldUpdateFullAsync) {
			workerBridge.query('get_regions').then(json => {
				if (json) { try { regions.set(JSON.parse(json)); } catch { } }
			}).catch(() => { });
			workerBridge.query('get_cities').then(json => {
				if (json) { try { cities.set(JSON.parse(json)); } catch { } }
			}).catch(() => { });
			workerBridge.query('get_all_corporations').then(json => {
				if (json) { try { allCorporations.set(JSON.parse(json)); } catch { } }
			}).catch(() => { });
		}

		// Process notifications from worker
		if (result.notifications) {
			try {
				const notifs = JSON.parse(result.notifications);
				if (Array.isArray(notifs) && notifs.length > 0) {
					notifications.update((n) => [...notifs, ...n].slice(0, 50));
					for (const notif of notifs) {
						audioManager.playEventSound(notif.event);
					}

					// Auto-pause on critical events
					if (get(autoPauseOnCritical) && currentSpeed > 0) {
						for (const notif of notifs) {
							const reason = checkCriticalEvent(notif.event);
							if (reason) {
								autoPauseReason.set(reason);
								break;
							}
						}
					}
				}
			} catch { /* ignore notification parse errors */ }
		}

		// Auto-save check (scheduled as background priority)
		if (info && info.tick > 0 && info.tick - lastAutoSaveTick >= get(autoSaveInterval)) {
			lastAutoSaveTick = info.tick;
			const saveTick = info.tick;
			scheduleBackground(() => performAutoSaveWorker(saveTick));
		}
	} catch (e) {
		console.error('[GameLoop] Error processing worker tick result:', e);
	}

	// Signal map for re-render with new data
	window.dispatchEvent(new CustomEvent('map-dirty'));
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
	const shouldUpdateFull = info.tick % 5 === 0 || info.tick === 0 || get(allCorporations).length === 0;
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

	// Auto-save check (scheduled as background priority)
	if (info.tick > 0 && info.tick - lastAutoSaveTick >= get(autoSaveInterval)) {
		lastAutoSaveTick = info.tick;
		const saveTick = info.tick;
		scheduleBackground(() => performAutoSave(saveTick));
	}

	// Signal map for re-render with new data
	window.dispatchEvent(new CustomEvent('map-dirty'));
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
		const data = await bridge.saveGame();
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

/** Auto-save via worker — queries the worker for save data. */
async function performAutoSaveWorker(tick: number) {
	try {
		const data = await workerBridge.query('save_game');
		if (!data) return;
		const corp = get(playerCorp);
		const slot = getNextAutoSaveSlot();
		await saveToSlot(slot, slot, data, tick, corp?.name ?? 'Unknown', 'Normal', 'Internet');
	} catch (e) {
		console.warn('[auto-save worker] Failed:', e);
	}
}

export function start() {
	if (running) return;
	running = true;
	lastTickTime = performance.now();
	tickAccumulator = 0;

	animFrameId = requestAnimationFrame(loop);
	setupKeyboardShortcuts();
	initPressureObserver();
}

export function stop() {
	running = false;
	if (animFrameId !== null) {
		cancelAnimationFrame(animFrameId);
		animFrameId = null;
	}
	teardownKeyboardShortcuts();
	teardownPressureObserver();
	audioManager.dispose();
	// Clean up multiplayer event listeners
	for (const fn of mpCleanupFns) fn();
	mpCleanupFns = [];
	// Terminate worker if active
	if (useWorker) {
		bridge.setCommandProxy(null);
		workerBridge.terminate();
		useWorker = false;
	}
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
	
	const canUseWorker = workerBridge.isSupported() && !bridge.isNativeSim();

	if (canUseWorker) {
		loadingStage.set(1);
		await yieldToUI();
		try {
			await workerBridge.init();
			await workerBridge.newGame(config);
			workerBridge.setTickResultHandler(handleWorkerTickResult);
			
			bridge.setCommandProxy((json) => workerBridge.sendCommand(json));
			useWorker = true;
			console.log('[GameLoop] Pure Worker mode initialized');
		} catch (e) {
			console.warn('[GameLoop] Worker init failed, falling back to main thread:', e);
			await bridge.initWasm();
			await bridge.newGame(config);
			useWorker = false;
		}
	} else {
		await bridge.initWasm();
		loadingStage.set(1);
		await yieldToUI();
		await bridge.newGame(config);
		useWorker = false;
	}

	loadingStage.set(2);
	await yieldToUI();

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

	await audioManager.init();
	const startingEra = (config as Record<string, unknown> | undefined)?.starting_era as string | undefined;
	audioManager.playMusic(configEraToAudioEra(startingEra));

	if (useWorker) {
		workerBridge.requestTick();
	} else {
		updateStores();
	}
	
	await bridge.fetchGridCells();
	loadingStage.set(3);
	await yieldToUI();

	setSpeed(0);
	showWelcome.set(true);
	initialized.set(true);
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

	await bridge.loadGame(saveData);
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

	// Ensure WASM knows which corp we are in this world
	const id = get(corpId);
	if (id !== null) {
		bridge.setPlayerCorpId(id);
	}

	updateStores();
	await bridge.fetchGridCells();

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
		const myCorpId = info.player_corp_id || get(corpId);

		for (const delta of deltas) {
			const cId = delta.corp_id as number;
			if (cId === myCorpId) {
				playerCorp.update((corp) => {
					if (!corp) return corp;
					const rev = delta.revenue ?? corp.revenue_per_tick;
					const cost = delta.cost ?? corp.cost_per_tick;
					return {
						...corp,
						cash: delta.cash ?? corp.cash,
						revenue_per_tick: rev,
						cost_per_tick: cost,
						profit_per_tick: rev - cost,
						debt: delta.debt ?? corp.debt,
						infrastructure_count: delta.node_count ?? corp.infrastructure_count,
					};
				});
			}
			allCorporations.update((corps) =>
				corps.map((c) => {
					if (c.id !== cId) return c;
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
	const handleSnapshotReload = async (e: Event) => {
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
			// loadGame is async when native sim, but we handle it in both cases
			const loadResult = bridge.loadGame(state_json);
			if (loadResult instanceof Promise) await loadResult;
			mpHighWaterTick = tick;

			// Re-apply our player corp ID as the snapshot might have a different one (or 0)
			const id = get(corpId);
			if (id !== null) {
				bridge.setPlayerCorpId(id);
			}

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

		// Apply deltas to local sim state
		try {
			const batchResult = bridge.applyBatch(ops);
			if (batchResult instanceof Promise) batchResult.catch((err: unknown) => console.error('[MP] Failed to apply batch:', err));

			// Refresh stores to pick up changes (financials, ownership, etc.)
			updateStores();
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

	// Listen for speed vote updates from the server
	const handleSpeedVote = (e: Event) => {
		const detail = (e as CustomEvent).detail;
		if (detail?.votes && Array.isArray(detail.votes)) {
			speedVotes.set(detail.votes.map((v: { username?: string; speed?: string }) => ({
				player_id: '',
				username: v.username ?? '',
				speed: v.speed ?? 'Normal',
				timestamp: Date.now()
			})));
		}
	};

	window.addEventListener('mp-corp-deltas', handleCorpDeltas);
	window.addEventListener('mp-snapshot', handleSnapshotReload);
	window.addEventListener('mp-command-broadcast', handleCommandBroadcast);
	window.addEventListener('mp-command-ack', handleCommandAck);
	window.addEventListener('mp-speed-vote', handleSpeedVote);

	mpCleanupFns.push(
		() => window.removeEventListener('mp-corp-deltas', handleCorpDeltas),
		() => window.removeEventListener('mp-snapshot', handleSnapshotReload),
		() => window.removeEventListener('mp-command-broadcast', handleCommandBroadcast),
		() => window.removeEventListener('mp-command-ack', handleCommandAck),
		() => window.removeEventListener('mp-speed-vote', handleSpeedVote),
		() => { isMultiplayerMode = false; mpHighWaterTick = 0; },
	);
}

export async function quickSave(): Promise<void> {
	if (!bridge.isInitialized()) return;
	if (useWorker) {
		const data = await workerBridge.query('save_game');
		if (!data) return;
		const info = get(worldInfo);
		const corp = get(playerCorp);
		await saveToSlot(
			QUICK_SAVE_SLOT,
			'Quick Save',
			data,
			info?.tick ?? 0,
			corp?.name ?? 'Unknown',
			'Normal',
			'Internet'
		);
		return;
	}
	const data = await bridge.saveGame();
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
	if (useWorker) {
		await workerBridge.loadGame(result.data);
		// Worker owns state — request a tick to push fresh data via handleWorkerTickResult
		workerBridge.requestTick();
	} else {
		await bridge.loadGame(result.data);
		updateStores();
	}
	lastAutoSaveTick = 0;
	return true;
}

export async function loadFromSave(data: string): Promise<void> {
	if (useWorker) {
		await workerBridge.loadGame(data);
		// Worker owns state — request a tick to push fresh data via handleWorkerTickResult
		workerBridge.requestTick();
	} else {
		await bridge.loadGame(data);
		updateStores();
	}
	lastAutoSaveTick = 0;
	initialized.set(true);
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
