/**
 * Worker Bridge — proxies WASM calls to the sim worker.
 *
 * Provides the same API surface as the direct bridge.ts module,
 * but routes all calls through postMessage to a Web Worker.
 * This keeps the main thread free for rendering and UI.
 *
 * Usage:
 *   import * as workerBridge from './workerBridge';
 *   await workerBridge.init(config);
 *   workerBridge.requestTick();
 *   const info = await workerBridge.query('get_world_info');
 */

import type {
	InfraNodesTyped,
	InfraEdgesTyped,
	CorporationsTyped,
} from './types';

let worker: Worker | null = null;
let queryIdCounter = 0;
let commandSeqCounter = 0;
let ready = false;

// Pending query/command callbacks
const pendingQueries = new Map<number, { resolve: (data: any) => void; reject: (err: any) => void }>();
const pendingCommands = new Map<number, { resolve: (data: any) => void; reject: (err: any) => void }>();

// Latest tick result (double-buffered: worker writes, main thread reads)
let latestTickResult: TickResult | null = null;
let onTickResult: ((result: TickResult) => void) | null = null;

export interface TickResult {
	nodes: InfraNodesTyped | null;
	edges: InfraEdgesTyped | null;
	corps: CorporationsTyped | null;
	info: string;
	playerCorp: string | null;
	notifications: string | null;
	tick: number;
}

/**
 * Initialize the sim worker and WASM module.
 * Returns a promise that resolves when the worker is ready.
 */
export function init(config?: string): Promise<void> {
	return new Promise((resolve, reject) => {
		try {
			worker = new Worker(
				new URL('../workers/simWorker.ts', import.meta.url),
				{ type: 'module' }
			);
		} catch (err) {
			reject(err);
			return;
		}

		const onReady = (e: MessageEvent) => {
			if (e.data.type === 'ready') {
				ready = true;
				resolve();
			}
		};

		worker.addEventListener('message', onReady, { once: true });

		worker.addEventListener('message', handleWorkerMessage);

		worker.postMessage({ type: 'init', config });
	});
}

function handleWorkerMessage(e: MessageEvent) {
	const msg = e.data;

	switch (msg.type) {
		case 'tick-result': {
			latestTickResult = {
				nodes: msg.nodes,
				edges: msg.edges,
				corps: msg.corps,
				info: msg.info,
				playerCorp: msg.playerCorp ?? null,
				notifications: msg.notifications ?? null,
				tick: msg.tick,
			};
			if (onTickResult) {
				onTickResult(latestTickResult);
			}
			break;
		}

		case 'command-result': {
			const pending = pendingCommands.get(msg.seq);
			if (pending) {
				pendingCommands.delete(msg.seq);
				pending.resolve(msg.result);
			}
			break;
		}

		case 'query-result': {
			const pending = pendingQueries.get(msg.id);
			if (pending) {
				pendingQueries.delete(msg.id);
				pending.resolve(msg.data);
			}
			break;
		}

		case 'error': {
			console.error('[SimWorker]', msg.message);
			break;
		}
	}
}

/** Request the worker to run one tick. Non-blocking — result arrives via onTickResult callback. */
export function requestTick(): void {
	if (!worker || !ready) return;
	worker.postMessage({ type: 'tick' });
}

/** Set a callback for when tick results arrive. */
export function setTickResultHandler(handler: (result: TickResult) => void): void {
	onTickResult = handler;
}

/** Get the latest tick result (may be from previous tick). */
export function getLatestTickResult(): TickResult | null {
	return latestTickResult;
}

/** Send a command to the worker and wait for the result. */
export function sendCommand(commandJson: string): Promise<string> {
	return new Promise((resolve, reject) => {
		if (!worker || !ready) {
			reject(new Error('Worker not ready'));
			return;
		}
		const seq = ++commandSeqCounter;
		pendingCommands.set(seq, { resolve, reject });
		worker.postMessage({ type: 'command', json: commandJson, seq });
	});
}

/** Run a named query method on the worker bridge and wait for the result. */
export function query(method: string, ...args: any[]): Promise<any> {
	return new Promise((resolve, reject) => {
		if (!worker || !ready) {
			reject(new Error('Worker not ready'));
			return;
		}
		const id = ++queryIdCounter;
		pendingQueries.set(id, { resolve, reject });
		worker.postMessage({ type: 'query', method, args, id });
	});
}

/** Load a saved game in the worker. */
export function loadGame(saveData: string): Promise<void> {
	return new Promise((resolve, reject) => {
		if (!worker) {
			reject(new Error('Worker not initialized'));
			return;
		}

		const onReady = (e: MessageEvent) => {
			if (e.data.type === 'ready') {
				ready = true;
				resolve();
			}
		};

		worker.addEventListener('message', onReady, { once: true });
		worker.postMessage({ type: 'load', saveData });
	});
}

/** Check if the worker is initialized and ready. */
export function isReady(): boolean {
	return ready;
}

/** Check if worker bridge is available (Web Workers supported). */
export function isSupported(): boolean {
	return typeof Worker !== 'undefined';
}

/** Terminate the worker. */
export function terminate(): void {
	if (worker) {
		worker.terminate();
		worker = null;
		ready = false;
	}
}
