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

let running = false;
let animFrameId: number | null = null;
let lastTickTime = 0;
let tickAccumulator = 0;
let currentSpeed = 1; // ticks per second
let lastAutoSaveTick = 0;
const AUTO_SAVE_INTERVAL = 50; // ticks between auto-saves
let gameConfig: object | undefined;

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
			bridge.tick();
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
	}

	// Auto-save check
	if (info.tick > 0 && info.tick - lastAutoSaveTick >= AUTO_SAVE_INTERVAL) {
		lastAutoSaveTick = info.tick;
		performAutoSave(info.tick);
	}
}

async function performAutoSave(tick: number) {
	try {
		const data = bridge.saveGame();
		const corp = get(playerCorp);
		const slot = getNextAutoSaveSlot();
		await saveToSlot(slot, slot, data, tick, corp?.name ?? 'Unknown', 'Normal', 'Internet');
	} catch {
		// Silent auto-save failure — don't disrupt gameplay
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
}

export function setSpeed(speed: number) {
	currentSpeed = speed;
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
	}
}

export function isRunning(): boolean {
	return running;
}
