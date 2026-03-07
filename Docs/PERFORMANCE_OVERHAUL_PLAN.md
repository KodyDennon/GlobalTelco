# GlobalTelco Performance Overhaul Plan

**Goal:** Eliminate rendering lag and enable 60 FPS at 10k+ entities by moving to a binary data pipeline and offloading simulation to a Web Worker.
**Strategy:** "Modern High-Performance" (SharedArrayBuffer/Transferables + Typed Arrays).

## Phase 1: Binary Data Pipeline (The "Plumbing" Fix)
*Eliminate JSON serialization on the hot path. This is the biggest single performace win.*

### 1.1. Enhance Rust Typed Arrays (`crates/gt-wasm`)
The current `get_infra_nodes_typed` is good but missing critical visual data (colors/owners).
- **Action:** Update `crates/gt-wasm/src/typed_arrays.rs` to return:
    - **Nodes:** Add `owner_ids` (Uint32) and `node_type_ids` (Uint8).
    - **Edges:** Add `owner_ids`, `edge_type_ids`, and `deployment_type` (Uint8).
    - **Lookup Tables:** Create a small JSON query `get_static_definitions()` that returns the mapping of `ID -> Name` for Corps, NodeTypes, EdgeTypes. This is fetched *once*, not every frame.

### 1.2. Solve the "Curved Cable" Problem
Binary arrays don't handle variable-length arrays (like waypoints) easily.
- **Action:** Implement a "Packed Waypoint Buffer":
    - `waypoints_data`: Float64Array (flat list of all x,y points).
    - `edge_waypoint_offsets`: Uint32Array (index into data).
    - `edge_waypoint_lengths`: Uint8Array (count).
- **Result:** JS can reconstruct paths efficiently or `deck.gl` can consume them (with custom accessors).

### 1.3. Refactor Frontend Bridge
- **Action:** Update `web/src/lib/wasm/bridge.ts` to expose these new binary methods.
- **Action:** Create `web/src/lib/game/map/DataStore.ts`. This class will:
    - Call the typed WASM methods.
    - Cache the `Int/Float` arrays.
    - Provide helper methods like `getNode(index)` that look up data in the arrays without creating objects.

### 1.4. Refactor `infraLayer.ts` to Binary
- **Action:** Rewrite `deck.gl` accessors to read from Typed Arrays.
    - *Before:* `getColor: d => d.color` (Object access)
    - *After:* `getColor: (_, {index}) => [colors[index*3], colors[index*3+1], ...]` (Array access)
- **Benefit:** Zero object creation during render cycles.

---

## Phase 2: Web Worker Offloading (The "Smoothness" Fix)
*Move the heavy lifting off the UI thread so the map never freezes.*

### 2.1. Worker Infrastructure
- **Action:** complete `web/src/lib/workerBridge.ts`.
- **Action:** Create `web/src/lib/workers/sim.worker.ts`.
    - It will import the WASM module.
    - It will handle the simulation loop (`tick()`).
    - It will handle "Commands" (user actions) sent from Main thread.

### 2.2. Zero-Copy State Sync
- **Action:** Implement `transferState()` in the worker.
    - It calls `get_infra_nodes_typed()`.
    - It `postMessage()`s the resulting TypedArrays with `[transferables]` list.
    - This "moves" the memory to the Main thread instantly (no copy).
- **Action:** Main thread receives the arrays and swaps them into `DataStore`.

### 2.3. Input Handling
- **Action:** User clicks/commands are sent as async messages to the worker.
- **Action:** Optimistic UI updates (optional): Show "Building..." immediately while waiting for Worker confirmation.

---

## Phase 3: Spatial & Render Optimizations (The "Scale" Fix)
*Handle 100k+ entities intelligently.*

### 3.1. Spatial Indexing in Rust
- **Action:** Add `rstar` (R-Tree) crate to `gt-simulation`.
- **Action:** Index all static nodes.
- **Action:** Implement `query_viewport_packed(bbox)`: Returns packed binary data *only* for entities in view.

### 3.2. Deck.gl Attributes
- **Action:** Use `deck.gl`'s low-level `attributes` prop. Instead of passing `data={array}` and accessors, we pass GPU buffers directly.
    - `getPositions: { value: float64Array, size: 2 }`
- **Benefit:** Skips even the JS-side iteration in `deck.gl`. Data goes straight to WebGL.

### 3.3. Shader-Based Styling
- **Action:** Move logic like "If Health < 50% turn red" into a custom shader module or `deck.gl` expression.
- **Benefit:** CPU does 0 work for styling.

---

## Execution Checklist

### Step 1: Prep
- [ ] Verify `deck.gl` binary data support in current version.
- [ ] Confirm `vite` config handles Worker imports correctly.

### Step 2: Phase 1 (Binary Data)
- [ ] Rust: Update `typed_arrays.rs` with `owners`, `types`, `waypoints`.
- [ ] Rust: Add `get_static_defs()` query.
- [ ] TS: Create `DataStore` class.
- [ ] TS: Refactor `infraLayer.ts` to use `DataStore` (binary mode).

### Step 3: Phase 2 (Worker)
- [ ] TS: Create `sim.worker.ts`.
- [ ] TS: Update `workerBridge.ts` to handle full game loop.
- [ ] TS: Implement "Transferable" logic for tick updates.
- [ ] TS: Update `App.svelte` to initialize Worker instead of direct WASM.

### Step 4: Phase 3 (Cleanup)
- [ ] Rust: Add `rstar` spatial index.
- [ ] TS: Implement viewport-based fetching.
