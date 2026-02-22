import * as bridge from '$lib/wasm/bridge';
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
import { saveToSlot, loadFromSlot, getNextAutoSaveSlot, QUICK_SAVE_SLOT } from '$lib/saves/SaveManager';
import { get } from 'svelte/store';
import { audioManager } from '$lib/audio/AudioManager';
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

// Performance profiling stores
export const simTickTime = writable<number>(0);

// Auto-pause state
export const autoPauseReason = writable<string | null>(null);

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

	if (currentSpeed > 0) {
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

	updateStores();
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

function checkCriticalEvent(event: string): string | null {
	// Disasters with severity > 0.3
	if (event.includes('Disaster') && event.includes('severity')) {
		const match = event.match(/severity:\s*([\d.]+)/);
		if (match && parseFloat(match[1]) > 0.3) {
			return 'Major disaster detected!';
		}
	}
	// Insolvency warning
	if (event.includes('Bankruptcy') || event.includes('Insolvency')) {
		return 'Financial crisis — insolvency warning!';
	}
	// Hostile acquisition
	if (event.includes('HostileAcquisition') || event.includes('hostile_acquisition')) {
		return 'Hostile acquisition attempt!';
	}
	// Espionage detected
	if (event.includes('EspionageDetected') || event.includes('espionage_detected')) {
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
			{ tick, event: 'AutoSaveFailed' },
			...n
		].slice(0, 50));
	}
}

export function start() {
	if (running) return;
	running = true;
	lastTickTime = performance.now();
	tickAccumulator = 0;

	// Register bridge error handler — pause game on WASM failure
	bridge.setErrorHandler((error, context) => {
		console.error(`WASM error in ${context}: ${error}`);
		if (context === 'tick') {
			setSpeed(0); // Pause on tick failures
			notifications.update((n) => [
				{ tick: 0, event: `WASMError: ${error}` },
				...n
			].slice(0, 50));
		}
	});

	// Register command notification handler — shows errors immediately when commands fail
	bridge.setCommandNotificationHandler((notifs) => {
		notifications.update((n) => [...notifs, ...n].slice(0, 50));
		for (const notif of notifs) {
			audioManager.playEventSound(notif.event);
		}
	});

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
}

export function setSpeed(speed: number) {
	currentSpeed = speed;
	// Clear auto-pause reason when resuming
	if (speed > 0) {
		autoPauseReason.set(null);
	}
	if (speed === 0) {
		bridge.processCommand({ SetSpeed: 'Paused' });
	} else {
		const speedMap: Record<number, string> = {
			1: 'Normal',
			2: 'Fast',
			4: 'VeryFast',
			8: 'Ultra'
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

export async function initGame(config?: object) {
	await bridge.initWasm();
	bridge.newGame(config as any);
	await audioManager.init();
	// Start paused so player can orient
	setSpeed(0);
	initialized.set(true);
	updateStores();
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
