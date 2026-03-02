/**
 * Sim Worker — runs WASM simulation in a dedicated Web Worker.
 * Offloads the entire tick() + query cycle off the main thread.
 *
 * Protocol:
 *   Main → Worker: { type: 'init', config?: string }
 *                   { type: 'tick' }
 *                   { type: 'command', json: string, seq: number }
 *                   { type: 'query', method: string, args?: any[], id: number }
 *                   { type: 'load', saveData: string }
 *   Worker → Main: { type: 'ready' }
 *                   { type: 'tick-result', nodes, edges, corps, info, playerCorp, notifications, tick }
 *                   { type: 'command-result', result: string, seq: number }
 *                   { type: 'query-result', data: any, id: number }
 *                   { type: 'error', message: string }
 */

// Dynamic import to allow WASM to load inside the worker
let wasmModule: any = null;
let bridge: any = null;

self.onmessage = async (e: MessageEvent) => {
	const msg = e.data;

	try {
		switch (msg.type) {
			case 'init': {
				// Import and initialize WASM inside the worker
				wasmModule = await import('../wasm/pkg/gt_wasm');
				await wasmModule.default();

				if (msg.config) {
					bridge = wasmModule.WasmBridge.new_game(msg.config);
				} else {
					bridge = new wasmModule.WasmBridge();
				}

				self.postMessage({ type: 'ready' });
				break;
			}

			case 'tick': {
				if (!bridge) break;

				bridge.tick();

				// Collect typed arrays for hot-path rendering
				const nodesArr = bridge.get_infra_nodes_typed();
				const edgesArr = bridge.get_infra_edges_typed();
				const corpsArr = bridge.get_corporations_typed();
				const infoJson = bridge.get_world_info();

				// Parse the flat arrays into structured objects
				const nodes = nodesArr && nodesArr.length >= 8 ? {
					count: nodesArr[0] as number,
					ids: nodesArr[1] as Uint32Array,
					owners: nodesArr[2] as Uint32Array,
					positions: nodesArr[3] as Float64Array,
					stats: nodesArr[4] as Float64Array,
					node_types: nodesArr[5] as Uint32Array,
					network_levels: nodesArr[6] as Uint32Array,
					construction_flags: nodesArr[7] as Uint8Array,
				} : null;

				const edges = edgesArr && edgesArr.length >= 6 ? {
					count: edgesArr[0] as number,
					ids: edgesArr[1] as Uint32Array,
					owners: edgesArr[2] as Uint32Array,
					endpoints: edgesArr[3] as Float64Array,
					stats: edgesArr[4] as Float64Array,
					edge_types: edgesArr[5] as Uint32Array,
				} : null;

				const corps = corpsArr && corpsArr.length >= 5 ? {
					count: corpsArr[0] as number,
					ids: corpsArr[1] as Uint32Array,
					financials: corpsArr[2] as Float64Array,
					name_offsets: corpsArr[3] as Uint32Array,
					names_packed: corpsArr[4] as Uint8Array,
				} : null;

				const tick = bridge.current_tick();

				// Collect player corp data and notifications for main thread
				let playerCorpJson: string | null = null;
				let notificationsJson: string | null = null;
				try {
					const info = JSON.parse(infoJson);
					if (info.player_corp_id > 0) {
						playerCorpJson = bridge.get_corporation_data(BigInt(info.player_corp_id));
					}
				} catch { /* ignore */ }
				try {
					notificationsJson = bridge.get_notifications();
				} catch { /* ignore */ }

				self.postMessage({
					type: 'tick-result',
					nodes,
					edges,
					corps,
					info: infoJson,
					playerCorp: playerCorpJson,
					notifications: notificationsJson,
					tick,
				});
				break;
			}

			case 'command': {
				if (!bridge) break;
				const result = bridge.process_command(msg.json);
				self.postMessage({
					type: 'command-result',
					result,
					seq: msg.seq,
				});
				break;
			}

			case 'query': {
				if (!bridge) break;
				const method = msg.method;
				const args = msg.args || [];

				let data: any;
				if (typeof bridge[method] === 'function') {
					data = bridge[method](...args);
				} else {
					data = null;
				}

				self.postMessage({
					type: 'query-result',
					data,
					id: msg.id,
				});
				break;
			}

			case 'load': {
				if (!wasmModule) break;
				bridge = wasmModule.WasmBridge.load_game(msg.saveData);
				self.postMessage({ type: 'ready' });
				break;
			}

			default:
				break;
		}
	} catch (err: any) {
		self.postMessage({
			type: 'error',
			message: err?.message || String(err),
		});
	}
};
