import SimWorker from '$lib/workers/sim.worker?worker';
import type { TickResult } from './wasm/bridge';
import { setLatestTickResult, setCommandProxy } from './wasm/bridge';
import type { InfraNodesTyped, InfraEdgesTyped } from './wasm/types';

let worker: Worker | null = null;
let commandId = 0;
const commandResolvers = new Map<number, (res: string) => void>();
let tickResultHandler: ((res: TickResult) => void) | null = null;

export function isSupported(): boolean {
    return typeof Worker !== 'undefined';
}

export async function init(): Promise<void> {
    worker = new SimWorker();
    
    worker.onmessage = (e) => {
        const { type, nodes, edges, corps, tick, id, result, info, playerCorp, notifications } = e.data;
        
        if (type === 'initComplete') {
            console.log('[WorkerBridge] Worker initialized');
        }
        else if (type === 'tickResult') {
            const infraNodes: InfraNodesTyped = {
                count: nodes[0],
                ids: nodes[1],
                owners: nodes[2],
                positions: nodes[3],
                stats: nodes[4],
                node_types: nodes[5],
                network_levels: nodes[6],
                construction_flags: nodes[7],
            };
            
            const infraEdges: InfraEdgesTyped = {
                count: edges[0],
                ids: edges[1],
                owners: edges[2],
                endpoints: edges[3],
                stats: edges[4],
                edge_types: edges[5],
                deployment_types: edges[6],
                waypoints_data: edges[7],
                waypoint_offsets: edges[8],
                waypoint_lengths: edges[9],
            };

            const res: TickResult = { 
                infraNodes, 
                infraEdges, 
                tick,
                // These are passed as JSON strings for compatibility with GameLoop expectations
                // GameLoop expects 'info', 'playerCorp', 'notifications' in the result object
                // But TickResult interface in bridge.ts might be strictly typed arrays?
                // bridge.ts TickResult: { infraNodes?, infraEdges?, corporations?, tick? }
                // GameLoop casts result to 'any' or workerBridge.TickResult?
                // Let's check GameLoop again. It says `result: workerBridge.TickResult`.
                // So I should define TickResult here to include JSON fields.
            };
            
            // Attach JSON fields for GameLoop to parse
            (res as any).info = info; // from worker
            (res as any).playerCorp = playerCorp;
            (res as any).notifications = notifications;

            if (tickResultHandler) {
                tickResultHandler(res);
            } else {
                // If no handler (e.g. initial load), just set it
                setLatestTickResult(res);
            }
        }
        else if (type === 'commandResult') {
            const resolve = commandResolvers.get(id);
            if (resolve) {
                resolve(result);
                commandResolvers.delete(id);
            }
        }
        else if (type === 'queryResult') {
             const resolve = commandResolvers.get(id);
             if (resolve) {
                 resolve(result);
                 commandResolvers.delete(id);
             }
        }
    };

    return new Promise<void>((resolve) => {
        const handler = (e: MessageEvent) => {
            if (e.data.type === 'initComplete') {
                worker?.removeEventListener('message', handler);
                resolve();
            }
        };
        worker?.addEventListener('message', handler);
        worker?.postMessage({ type: 'init' });
    });
}

export function setTickResultHandler(handler: (res: TickResult) => void) {
    tickResultHandler = handler;
}

export function requestTick() {
    worker?.postMessage({ type: 'tick' });
}

export async function sendCommand(json: string): Promise<string> {
    if (!worker) return "";
    return new Promise<string>((resolve) => {
        const id = ++commandId;
        commandResolvers.set(id, resolve);
        worker?.postMessage({ type: 'command', id, cmd: json });
    });
}

export async function loadGame(json: string): Promise<void> {
    // We send a command to load game? Or specific message?
    // bridge.load_game is a query/command? It's a method on WasmBridge.
    // Let's assume we can use sendCommand with a special "LoadGame" command if supported,
    // or add a 'load' message type.
    // SimWorker needs to handle 'load'.
    if (!worker) return;
    worker.postMessage({ type: 'load', data: json });
}

export async function query(name: string, ...args: any[]): Promise<string> {
    if (!worker) return "";
    return new Promise<string>((resolve) => {
        const id = ++commandId;
        commandResolvers.set(id, resolve);
        worker?.postMessage({ type: 'query', id, name, args });
    });
}

export function terminate() {
    worker?.terminate();
    worker = null;
}

// Re-export TickResult to satisfy GameLoop import
export type { TickResult } from './wasm/bridge';
