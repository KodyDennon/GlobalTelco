# Infrastructure Integration & Rendering Audit Report

## Summary
The audit of the `GlobalTelco` codebase revealed several critical issues in the integration between the simulation (WASM/Rust) and the map renderer (Deck.gl/Svelte). These issues explain why players can place infrastructure that generates revenue but does not appear on the map.

## Critical Issues

### 1. Asynchronous Command Race Condition
**Location:** `web/src/lib/game/commandRouter.ts`
**Issue:** The UI dispatches the `map-dirty` event immediately after calling `bridge.processCommand`, but does not `await` its completion.
**Impact:** `MapView.svelte` triggers a re-render before the simulation has finished adding the new node/edge to its state. The renderer sees the "old" state where the node doesn't exist yet.

### 2. Delayed Corporation Registration in UI
**Location:** `web/src/lib/game/GameLoop.ts`
**Issue:** The `allCorporations` store, which the map renderer uses to iterate over infrastructure, is only updated every 5 ticks (`info.tick % 5 === 0`).
**Impact:** When a new game starts, the player's corporation may not be in the `allCorporations` list for several seconds. Since the renderer only draws infrastructure for "known" corporations, the player's initial placement is invisible.

### 3. Inefficient & Fragmented Data Fetching
**Location:** `web/src/lib/game/map/layers/infraLayer.ts`
**Issue:** The renderer fetches infrastructure by looping through corporations and calling `getInfrastructureList(corpId)` for each.
**Impact:** This is highly inefficient (O(N) bridge calls) and creates a dependency on the `corps` list being perfectly synchronized with the simulation's entity list. If a corporation is missing from the list, its entire infrastructure network vanishes from the map.

### 4. Visibility Logic Dependency on Player ID
**Location:** `web/src/lib/game/map/layers/infraLayer.ts`
**Issue:** The renderer uses `bridge.getPlayerCorpId()` to determine if a node/edge belongs to the player.
**Impact:** If `playerCorpId` is not correctly set in the bridge (which can happen if `setPlayerCorpId` isn't called early enough), the renderer may treat the player's infrastructure as "competitor" infra, applying high transparency or LOD filters that hide it.

## Proposed Fixes

### Phase 1: Immediate Rendering Fixes
1.  **Await Commands:** Update `commandRouter.ts` to `await bridge.processCommand` before dispatching `map-dirty`.
2.  **Unified Fetching:** Refactor `infraLayer.ts` to use `bridge.getAllInfrastructure()` (which returns all nodes and edges in one call) instead of the per-corporation loop.
3.  **Force Store Update:** Ensure `GameLoop.ts` performs a full store update immediately after game initialization and after successful build commands.

### Phase 2: Performance & Reliability
1.  **Direct ID Mapping:** Use the `owner` field in the node/edge data directly to look up colors, rather than relying on the loop index of the corporations list.
2.  **LOD Cleanup:** Verify `minTierForZoom` logic to ensure new construction is always visible at the player's current zoom level.
