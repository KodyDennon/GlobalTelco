import * as bridge from '$lib/wasm/bridge';
import {
	initialized,
	worldInfo,
	playerCorp,
	regions,
	cities,
	notifications,
	allCorporations
} from '$lib/stores/gameState';

let running = false;
let animFrameId: number | null = null;
let lastTickTime = 0;
let tickAccumulator = 0;
let currentSpeed = 1; // ticks per second

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
		playerCorp.set(bridge.getCorporationData(info.player_corp_id));
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
}

export function start() {
	if (running) return;
	running = true;
	lastTickTime = performance.now();
	tickAccumulator = 0;
	animFrameId = requestAnimationFrame(loop);
}

export function stop() {
	running = false;
	if (animFrameId !== null) {
		cancelAnimationFrame(animFrameId);
		animFrameId = null;
	}
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

export function isRunning(): boolean {
	return running;
}
