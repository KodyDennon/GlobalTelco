import initWasm, { WasmBridge } from '../wasm/pkg/gt_wasm';

let bridge: WasmBridge | null = null;

self.onmessage = async (e: MessageEvent) => {
    const { type, payload } = e.data;

    switch (type) {
        case 'init':
            await initWasm();
            bridge = WasmBridge.new();
            if (payload?.config) {
                // bridge.new_game(JSON.stringify(payload.config));
            }
            self.postMessage({ type: 'initComplete' });
            break;

        case 'tick':
            if (!bridge) return;
            bridge.tick();
            
            const { bounds } = e.data; // [west, south, east, north]
            
            // Extract hot-path data (Typed Arrays) - Viewport aware if bounds provided
            let nodesArr, edgesArr;
            if (bounds && bounds.length === 4) {
                nodesArr = bridge.get_infra_nodes_typed_viewport(bounds[0], bounds[1], bounds[2], bounds[3]);
                edgesArr = bridge.get_infra_edges_typed_viewport(bounds[0], bounds[1], bounds[2], bounds[3]);
            } else {
                nodesArr = bridge.get_infra_nodes_typed();
                edgesArr = bridge.get_infra_edges_typed();
            }
            
            // const corpsArr = bridge.get_corporations_typed(); 

            // Extract JSON data for UI (WorldInfo, PlayerCorp, Notifications)
            const info = bridge.get_world_info();
            // Parse info to get player_corp_id for next query
            const infoObj = JSON.parse(info);
            const playerCorp = infoObj.player_corp_id ? bridge.get_corporation_data(BigInt(infoObj.player_corp_id)) : "{}";
            const notifications = bridge.get_notifications();

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
            }, transferables);
            break;

        case 'command':
            if (!bridge) return;
            try {
                // e.data has { type, id, cmd }
                const { id, cmd } = e.data;
                const result = bridge.process_command(cmd);
                self.postMessage({ type: 'commandResult', id, result });
            } catch (err) {
                console.error(err);
            }
            break;

        case 'load':
            if (!bridge) return;
            try {
                bridge.load_game(e.data.data);
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
                // Safe lookup?
                // Note: args is Array. WasmBridge methods expect spread? Or specific args?
                // Example: get_corporation_data(id). args=[id].
                // bridge.get_corporation_data(...args) works.
                
                // Need to cast bridge to any to access dynamic method
                if (typeof (bridge as any)[name] === 'function') {
                    // Convert args if necessary (e.g. string to BigInt?)
                    // JS BigInt is transferable via postMessage, so args can contain BigInt.
                    // But if they came from JSON, they are strings/numbers.
                    // Rust bindgen expects BigInt for u64.
                    
                    const castArgs = args.map((a: any) => {
                        // Heuristic: if method expects u64, and we got number/string, cast to BigInt?
                        // Hard to know method signature here.
                        // For now, rely on caller passing correct types or Rust bindgen handling it.
                        return a;
                    });
                    
                    result = (bridge as any)[name](...castArgs);
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
