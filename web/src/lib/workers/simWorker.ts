import initWasm, { WasmBridge } from '../wasm/pkg/gt_wasm';

let bridge: WasmBridge | null = null;

self.onmessage = async (e: MessageEvent) => {
    const { type, payload } = e.data;

    switch (type) {
        case 'init':
            await initWasm();
            bridge = new WasmBridge();
            self.postMessage({ type: 'initComplete' });
            break;

        case 'newGame':
            if (!bridge) return;
            try {
                const { config } = e.data;
                // new_game is a static method that re-initializes the internal world
                (bridge as any).constructor.new_game(config ? JSON.stringify(config) : undefined);
                self.postMessage({ type: 'newGameComplete' });
            } catch (err) {
                console.error('Worker newGame failed:', err);
            }
            break;

        case 'tick':
            if (!bridge) return;
            bridge.tick();
            
            const { bounds } = e.data; // [west, south, east, north]
            
            // Extract hot-path data (Typed Arrays) - Viewport aware if bounds provided
            let nodesArr, edgesArr;
            if (bounds && bounds.length === 4) {
                nodesArr = (bridge as any).get_infra_nodes_typed_viewport(bounds[0], bounds[1], bounds[2], bounds[3]);
                edgesArr = (bridge as any).get_infra_edges_typed_viewport(bounds[0], bounds[1], bounds[2], bounds[3]);
            } else {
                nodesArr = bridge.get_infra_nodes_typed();
                edgesArr = bridge.get_infra_edges_typed();
            }
            
            // Extract JSON data for UI (WorldInfo, PlayerCorp, Notifications)
            const infoStr = bridge.get_world_info();
            const info = JSON.parse(infoStr);
            
            // Parse data in worker to avoid blocking main thread
            const playerCorpStr = info.player_corp_id ? bridge.get_corporation_data(BigInt(info.player_corp_id)) : "{}";
            const playerCorp = JSON.parse(playerCorpStr);
            
            const notificationsStr = bridge.get_notifications();
            const notifications = JSON.parse(notificationsStr);

            // Helper to get buffers
            const transferables: Transferable[] = [];
            const addBuffers = (arr: any[]) => {
                arr.forEach(item => {
                    if (item && item.buffer instanceof ArrayBuffer) {
                        transferables.push(item.buffer);
                    }
                });
            };

            addBuffers(nodesArr);
            addBuffers(edgesArr);
            
            self.postMessage({
                type: 'tickResult',
                nodes: nodesArr,
                edges: edgesArr,
                tick: bridge.current_tick(),
                info,
                playerCorp,
                notifications
            }, { transfer: transferables });
            break;

        case 'command':
            if (!bridge) return;
            try {
                const { id, cmd } = e.data;
                const resultStr = bridge.process_command(cmd);
                const result = JSON.parse(resultStr);
                self.postMessage({ type: 'commandResult', id, result });
            } catch (err) {
                console.error(err);
            }
            break;

        case 'load':
            try {
                bridge = WasmBridge.load_game(e.data.data);
                self.postMessage({ type: 'loadComplete' });
            } catch (err) {
                console.error('Worker load failed:', err);
            }
            break;

        case 'query':
            if (!bridge) return;
            try {
                const { id, name, args } = e.data;
                let result;
                if (typeof (bridge as any)[name] === 'function') {
                    const rawResult = (bridge as any)[name](...args);
                    // If result is a string that looks like JSON, parse it here
                    if (typeof rawResult === 'string' && (rawResult.startsWith('{') || rawResult.startsWith('['))) {
                        try {
                            result = JSON.parse(rawResult);
                        } catch {
                            result = rawResult;
                        }
                    } else {
                        result = rawResult;
                    }
                } else {
                    console.error(`Unknown query: ${name}`);
                    result = null;
                }
                self.postMessage({ type: 'queryResult', id, result });
            } catch (err) {
                console.error(err);
            }
            break;
    }
};
