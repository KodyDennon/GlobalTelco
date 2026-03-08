import SimWorker from '$lib/workers/simWorker?worker';
import { setLatestTickResult, setCommandProxy } from './bridge';
import type { InfraNodesTyped, InfraEdgesTyped } from './types';

export interface TickResult {
    infraNodes: InfraNodesTyped;
    infraEdges: InfraEdgesTyped;
    tick: number;
    info: any;
    playerCorp: any;
    notifications: any[];
}

let worker: Worker | null = null;
let commandId = 0;
const commandResolvers = new Map<number, (res: any) => void>();
let tickResultHandler: ((res: TickResult) => void) | null = null;

export function isSupported(): boolean {
    return typeof Worker !== 'undefined';
}

export async function init(): Promise<void> {
    worker = new SimWorker();
    
    worker.onmessage = (e) => {
        const { type, nodes, edges, tick, id, result, info, playerCorp, notifications } = e.data;
        
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
                cell_indices: nodes[8],
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
                info,
                playerCorp,
                notifications
            };
            
            if (tickResultHandler) {
                tickResultHandler(res);
            } else {
                setLatestTickResult(res as any);
            }
        }
        else if (type === 'commandResult' || type === 'queryResult') {
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

export function requestTick(bounds?: [number, number, number, number]) {
    worker?.postMessage({ type: 'tick', bounds });
}

export async function sendCommand(json: string): Promise<any> {
    if (!worker) return null;
    return new Promise<any>((resolve) => {
        const id = ++commandId;
        commandResolvers.set(id, resolve);
        worker?.postMessage({ type: 'command', id, cmd: json });
    });
}

export async function loadGame(json: string): Promise<void> {
    if (!worker) return;
    return new Promise<void>((resolve) => {
        const handler = (e: MessageEvent) => {
            if (e.data.type === 'loadComplete') {
                worker?.removeEventListener('message', handler);
                resolve();
            }
        };
        worker?.addEventListener('message', handler);
        worker?.postMessage({ type: 'load', data: json });
    });
}

export async function newGame(config?: any): Promise<void> {
    if (!worker) return;
    return new Promise<void>((resolve) => {
        const handler = (e: MessageEvent) => {
            if (e.data.type === 'newGameComplete') {
                worker?.removeEventListener('message', handler);
                resolve();
            }
        };
        worker?.addEventListener('message', handler);
        worker?.postMessage({ type: 'newGame', config });
    });
}

export async function query(name: string, ...args: any[]): Promise<any> {
    if (!worker) return null;
    return new Promise<any>((resolve) => {
        const id = ++commandId;
        commandResolvers.set(id, resolve);
        worker?.postMessage({ type: 'query', id, name, args });
    });
}

export function terminate() {
    worker?.terminate();
    worker = null;
}
