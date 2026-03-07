# GlobalTelco Performance Plan

Full-stack performance optimization: simulation, rendering, build pipeline, multiplayer, and future tech.

**Target:** 10,000+ entities at <50ms tick, 60fps rendering, <3s initial load, <100ms multiplayer RTT.

---

## Current State (Problems)

### Build Pipeline
- **No `[profile.release]`** in workspace Cargo.toml. Using Rust defaults (no LTO, 16 codegen-units, debug symbols included).
- **wasm-pack builds may be debug.** `vercel-install.sh` and CLAUDE.md commands don't pass `--release`.
- **No wasm-opt post-processing.** No SIMD flags. No custom allocator.
- **WASM module size** is larger than necessary.

### Simulation (Rust/WASM)
- **All 36 systems run every tick unconditionally** — no dirty-bit tracking, no system skipping.
- **O(N^2) satellite ISL linking** in `satellite_network.rs` — nested loop with Haversine for all satellite pairs.
- **Clear-then-rebuild every tick** for coverage (`cell_coverage` HashMap) and satellite edges.
- **60+ `serde_json::json!()` calls** in `gt-bridge/src/queries.rs` with `format!("{:?}", enum)` for Debug formatting.
- **Only 4 of 24+ queries use typed arrays** — the rest serialize to JSON strings every call.
- **No buffer reuse** — typed array Vecs are allocated fresh every query call.
- **`Reflect::set` in hot path** — typed array return objects use 7-8 `Reflect::set` calls per query.
- **Default `dlmalloc` allocator** — bloated for WASM.

### Frontend/Rendering
- **Sim runs on main thread** — `bridge.tick()` is synchronous in `GameLoop.ts:129`. At 8x speed a 50ms tick leaves 75ms for everything else.
- **deck.gl layers fully rebuilt every render** — `MapRenderer.renderLayers()` creates all layers from scratch on each call. 12+ overlay types, no diffing.
- **Dashboard panel queries WASM every tick** — `$effect` in `DashboardPanel.svelte` fires on every `$worldInfo.tick`, calling `getDebtInstruments()` + `getInfrastructureList()` (full JSON round-trips).
- **D3 charts do full SVG nuke-and-rebuild** — `svg.selectAll('*').remove()` then recreate everything. Every 10 ticks.
- **deck.gl doesn't support OffscreenCanvas or WebGPU** — locked to WebGL on main thread.
- **No adaptive quality** — same rendering fidelity regardless of device capability.

### Multiplayer
- **WebSocket only** — TCP head-of-line blocking for all messages. State deltas and commands share one channel.
- **No unreliable transport** for state updates that can tolerate loss.

---

## Architecture Target

```
Main Thread          Sim Worker              Render Worker (future)
┌──────────────┐    ┌────────────────────┐  ┌──────────────────────┐
│ Svelte UI    │    │ WASM + SIMD        │  │ WebGPU Renderer      │
│ Panels       │◄──►│ 36 ECS systems     │  │ OffscreenCanvas      │
│ Input        │    │ Rayon par_iter     │  │ Custom map layers    │
│ Audio        │    │ talc allocator     │  │ 60fps independent    │
│ Scheduler API│    │                    │  │                      │
└──────────────┘    └────────┬───────────┘  └──────────┬───────────┘
                             │ Transferable              │ GPU dispatch
                             │ ArrayBuffers              │
                             ▼                           ▼
                    ┌──────────────────────────────────────────────┐
                    │              WebGPU (GPU)                     │
                    │  Compute: coverage, demand, routing, weather │
                    │  Render: map layers, infra, overlays         │
                    └──────────────────────────────────────────────┘
```

---

## Phase 0: Build Pipeline Fixes (Day 1)

Immediate wins. Zero code changes to game logic.

### 0.1 Add release profile to workspace Cargo.toml

Add after the `[workspace.dependencies]` section:

```toml
[profile.release]
opt-level = 's'
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

`opt-level = 's'` balances size and speed for WASM delivery. `lto = "fat"` enables cross-crate optimization across all 11 crates. `codegen-units = 1` maximizes optimization at the cost of build time. `panic = "abort"` removes unwinding machinery (~20KB). `strip = "symbols"` removes debug symbols.

### 0.2 Use --release in wasm-pack

Update `vercel-install.sh` build command:
```bash
wasm-pack build crates/gt-wasm --target web --release --out-dir ../../web/src/lib/wasm/pkg
```

Update CLAUDE.md build commands to match.

### 0.3 Enable SIMD

Update `vercel-install.sh` to set RUSTFLAGS before build:
```bash
export RUSTFLAGS="-C target-feature=+simd128"
wasm-pack build crates/gt-wasm --target web --release --out-dir ../../web/src/lib/wasm/pkg
```

All browsers support 128-bit WASM SIMD (Chrome, Firefox, Safari, Edge). The Rust compiler auto-vectorizes eligible loops. No code changes needed for baseline gains.

### 0.4 Switch to talc allocator

In `crates/gt-wasm/Cargo.toml`:
```toml
[dependencies]
talc = "4"
```

In `crates/gt-wasm/src/lib.rs`:
```rust
#[global_allocator]
static ALLOCATOR: talc::TalckWasm = unsafe { talc::TalckWasm::new_global() };
```

Smaller and faster than dlmalloc for WASM workloads.

### 0.5 Verify WASM caching headers

Ensure Vercel/Cloudflare serves `.wasm` files with:
- `Content-Type: application/wasm` (required for streaming compilation)
- `Cache-Control: public, max-age=31536000, immutable` (long cache)
- Content hash in filename for cache busting

V8 caches compiled native code after the second load — near-instant startup on third+ visits.

**Expected impact:** 30-50% smaller WASM binary. 10-30% faster execution. Zero code changes.

---

## Phase 1: Simulation Hot Path (Week 1)

### 1.1 Replace Debug enum formatting with integer discriminants

In `crates/gt-bridge/src/queries.rs`, replace all ~40 instances of:
```rust
format!("{:?}", some_enum)
```
with:
```rust
some_enum as u32  // or a simple Display impl
```

Each `format!("{:?}", ...)` allocates a temporary String. Integer discriminants are zero-cost.

Add a lookup table on the JS side to convert discriminant integers back to display strings.

### 1.2 Convert high-frequency JSON queries to typed arrays

Identify queries called every tick or on map updates and convert them from `serde_json::json!()` to typed array encoding:

| Query | Current | Frequency | Convert? |
|-------|---------|-----------|----------|
| `get_world_info` | JSON | Every tick | Yes — small struct, pack into single Float64Array |
| `get_corporation_data` | JSON | Every tick | Yes — financials already numeric |
| `get_notifications` | JSON | Every tick | No — string-heavy, keep JSON |
| `get_infrastructure_list` | JSON | Panel open | Yes — numeric data with typed arrays |
| `get_cell_coverage` | JSON | Map update | Yes — cell indices + bandwidth values |
| `get_regions` | JSON | Every 5 ticks | Partial — pack numeric fields |
| `get_cities` | JSON | Every 5 ticks | Partial — pack numeric fields |

For each converted query:
1. Define a parallel-arrays struct in `gt-bridge` (like `InfraArrays`)
2. Add a `build_*_arrays()` function
3. Add a typed array export in `gt-wasm/src/typed_arrays.rs`
4. Update the JS bridge to read typed arrays instead of parsing JSON

### 1.3 Pre-allocate reusable render buffers

In `WasmBridge`, add persistent buffers that are cleared and refilled each call instead of allocating new Vecs:

```rust
#[wasm_bindgen]
pub struct WasmBridge {
    world: GameWorld,
    node_buffer: InfraArrays,   // reused across calls
    edge_buffer: EdgeArrays,    // reused across calls
    corp_buffer: CorpArrays,    // reused across calls
}
```

Use `Vec::clear()` + `Vec::extend()` instead of creating new Vecs. Eliminates per-query allocation churn.

### 1.4 Replace Reflect::set with flat Array returns

The satellite arrays query (`get_satellite_arrays`) already uses the faster `js_sys::Array` pattern. Convert the node/edge/corp queries to match:

```rust
// Before: 8x Reflect::set calls (slow)
let obj = js_sys::Object::new();
js_sys::Reflect::set(&obj, &"ids".into(), &Uint32Array::from(...).into());
// ...

// After: single Array push (fast)
let result = js_sys::Array::new();
result.push(&JsValue::from(count as u32));
result.push(&Uint32Array::from(&arrays.ids[..]).into());
// ...
```

Fewer WASM-JS boundary crossings per query.

### 1.5 Fix O(N^2) satellite ISL linking

In `crates/gt-simulation/src/systems/satellite_network.rs`, replace the nested loop (lines 102-135) with a spatial hash grid:

1. Partition satellites into grid cells based on orbital position
2. For each satellite, only check neighbors in adjacent grid cells
3. Complexity drops from O(N^2) to O(N) average case

With 1000 satellites this eliminates ~999,000 unnecessary Haversine computations per tick.

### 1.6 Add dirty-bit system skipping

Add a `SystemDirtyFlags` bitfield to `GameWorld`:

```rust
pub struct SystemDirtyFlags {
    bits: u64, // 36 systems fit in 64 bits
}
```

Each command and event sets the relevant system dirty bits. At tick time, skip systems whose bits are clean. Reset all bits after each tick.

Priority systems to gate:
- `construction` — skip if no active construction
- `weather` — skip if no active weather events
- `disaster` — skip if no active disasters
- `debris` — skip if no debris events
- `servicing` — skip if no active servicing missions
- `auction` — skip if no active auctions
- `legal` — skip if no active lawsuits

Conservative estimate: 30-50% of systems can be skipped on an average tick.

### 1.7 Incremental coverage updates

Replace the clear-then-rebuild pattern in `coverage.rs`:

1. Track which nodes changed (built, destroyed, upgraded) via dirty set
2. Only recalculate coverage for cells affected by changed nodes
3. Use a spatial index (grid-based) to find affected cells in O(1) per changed node

Same approach for `satellite_network.rs` — only rebuild ISL links for satellites that moved significantly since last tick.

**Expected impact:** 2-5x faster ticks for mature worlds with 5,000+ entities.

---

## Phase 2: Web Worker Architecture (Week 2)

Move the WASM simulation off the main thread entirely.

### 2.1 Create sim worker

New file: `web/src/lib/workers/simWorker.ts`

```typescript
// simWorker.ts — runs WASM simulation in a dedicated Web Worker
import init, { WasmBridge } from '../wasm/pkg/gt_wasm';

let bridge: WasmBridge | null = null;

self.onmessage = async (e: MessageEvent) => {
    switch (e.data.type) {
        case 'init':
            await init();
            bridge = WasmBridge.new_game(e.data.config);
            self.postMessage({ type: 'ready' });
            break;

        case 'tick':
            bridge!.tick();
            // Collect typed arrays and transfer (zero-copy)
            const nodes = bridge!.get_infra_nodes_typed();
            const edges = bridge!.get_infra_edges_typed();
            const corps = bridge!.get_corporations_typed();
            const info = bridge!.get_world_info();
            self.postMessage(
                { type: 'tick-result', nodes, edges, corps, info },
                [/* transferable ArrayBuffers */]
            );
            break;

        case 'command':
            const result = bridge!.process_command(e.data.json);
            self.postMessage({ type: 'command-result', result, seq: e.data.seq });
            break;

        case 'query':
            const data = bridge![e.data.method](...(e.data.args || []));
            self.postMessage({ type: 'query-result', data, id: e.data.id });
            break;
    }
};
```

### 2.2 Create worker bridge

New file: `web/src/lib/wasm/workerBridge.ts`

Replaces direct WASM calls with postMessage to the sim worker. API surface stays identical to the current `bridge.ts` — all callers are unaware of the change.

For queries needed synchronously (rare), use `SharedArrayBuffer` + `Atomics.wait()` as a sync primitive. For everything else, use async postMessage.

### 2.3 Double-buffered typed arrays

Allocate two sets of typed array buffers. While the renderer reads Buffer A, the sim worker writes to Buffer B. On tick completion, swap. Transfer via `postMessage` with Transferable objects (zero-copy — ownership moves, no copying).

```typescript
// Main thread receives tick result
worker.onmessage = (e) => {
    if (e.data.type === 'tick-result') {
        // These ArrayBuffers were transferred (zero-copy)
        currentRenderData = e.data;
        requestRender();
    }
};
```

### 2.4 Update GameLoop.ts

Replace the synchronous `bridge.tick()` call with:

```typescript
// Before (blocking main thread):
bridge.tick();
updateStores();

// After (non-blocking):
simWorker.postMessage({ type: 'tick' });
// Render continues at 60fps using last received data
// Store updates happen when worker responds
```

The game loop becomes purely a render/UI loop. Tick timing moves to the worker.

### 2.5 Fallback for no-Worker environments

Keep the current synchronous path as a fallback. Feature-detect Worker support and SharedArrayBuffer availability. If unavailable (rare in 2026), fall back to main-thread WASM.

**Expected impact:** Main thread freed entirely for rendering and UI. Sim ticks never cause frame drops. Eliminates the 8x-speed jank problem.

---

## Phase 3: WebGPU Renderer (Weeks 3-5)

Replace deck.gl (WebGL) with a custom WebGPU renderer. This unlocks OffscreenCanvas, compute shaders, and the full worker architecture.

### 3.1 Renderer architecture

New directory: `web/src/lib/renderer/`

```
renderer/
├── GPURenderer.ts          # Main orchestrator (replaces MapRenderer)
├── GPUContext.ts            # WebGPU device/adapter/context setup
├── pipelines/
│   ├── landPipeline.ts      # Land polygons
│   ├── borderPipeline.ts    # Political borders
│   ├── infraPipeline.ts     # Infrastructure nodes + edges
│   ├── cityPipeline.ts      # City markers + labels
│   ├── overlayPipeline.ts   # Coverage, demand, traffic overlays
│   ├── satellitePipeline.ts # Satellite positions + trails
│   ├── weatherPipeline.ts   # Weather effects
│   └── textPipeline.ts      # Labels (SDF text rendering)
├── compute/
│   ├── coverageCompute.ts   # GPU coverage calculation
│   ├── demandCompute.ts     # GPU demand modeling
│   ├── routingCompute.ts    # GPU parallel routing
│   └── utilCompute.ts       # GPU utilization aggregation
├── shaders/
│   ├── land.wgsl
│   ├── infra.wgsl
│   ├── overlay.wgsl
│   ├── text.wgsl
│   ├── coverage.compute.wgsl
│   ├── demand.compute.wgsl
│   └── routing.compute.wgsl
└── camera/
    ├── Camera2D.ts          # Pan/zoom/rotation
    └── projections.ts       # Mercator projection math
```

### 3.2 MapLibre integration

Keep MapLibre GL JS as the base map (satellite tiles, vector data). Use `deck.gl/mapbox`-style overlay integration but with WebGPU:

- MapLibre renders the base map to its own canvas
- WebGPU renderer draws game layers to a separate canvas overlaid on top
- Camera sync: pipe MapLibre's view state (center, zoom, bearing, pitch) to the WebGPU camera

This allows incremental migration — start with infrastructure layers, then overlays, then terrain.

### 3.3 Render pipeline per layer type

Each pipeline is a self-contained WebGPU render pipeline with:
- Vertex buffer layout matched to typed array format from WASM
- Instance rendering for nodes (one draw call for all nodes of the same type)
- Line rendering with GPU-side width/color from edge typed arrays
- Overlay rendering via fullscreen quad + compute shader output texture

### 3.4 Incremental layer updates

Unlike deck.gl's full rebuild, the WebGPU renderer updates GPU buffers incrementally:

```typescript
// Only upload changed data, not entire arrays
device.queue.writeBuffer(nodeBuffer, changedOffset, changedData);
```

Track dirty ranges from sim worker delta messages. Only re-upload the changed portion of each buffer.

### 3.5 OffscreenCanvas in render worker

Once deck.gl is replaced:

```typescript
// Main thread
const canvas = document.getElementById('map-canvas');
const offscreen = canvas.transferControlToOffscreen();
renderWorker.postMessage({ type: 'init', canvas: offscreen }, [offscreen]);

// Render worker
const device = await navigator.gpu.requestAdapter()...;
const context = canvas.getContext('webgpu');
// All rendering happens here — main thread only handles UI
```

### 3.6 WebGPU fallback

Feature-detect WebGPU. If unavailable, fall back to a WebGL2 path (simplified deck.gl-like rendering). Target: WebGPU for 70%+ of users (Chrome, Firefox, Safari, Edge all support it as of late 2025). WebGL2 fallback for the rest.

```typescript
const gpu = navigator.gpu;
if (gpu) {
    const adapter = await gpu.requestAdapter();
    if (adapter) {
        return new GPURenderer(adapter);
    }
}
return new WebGLFallbackRenderer(); // deck.gl-based
```

**Expected impact:** 60fps rendering fully independent of sim. Incremental buffer updates instead of full layer rebuild. Foundation for GPU compute shaders.

---

## Phase 4: GPU Compute Shaders (Weeks 5-7)

Offload data-parallel ECS systems to WebGPU compute.

### 4.1 Identify GPU-eligible systems

| System | Parallelizable? | Data Pattern | GPU Candidate? |
|--------|----------------|--------------|----------------|
| coverage | Yes — per-cell independent | Grid cells x nodes | **Yes** |
| demand | Yes — per-cell independent | Grid cells x population | **Yes** |
| utilization | Yes — per-entity independent | All nodes + edges | **Yes** |
| orbital | Yes — per-satellite independent | All satellites | **Yes** |
| routing | Partially — parallel BFS | Network graph | **Yes (batch)** |
| weather | Yes — cellular automata | Grid cells | **Yes** |
| satellite_network | Yes — spatial neighbor search | All satellites | **Yes** |
| revenue | Partially — per-cell then aggregate | Grid cells x corps | **Yes** |
| cost | Yes — per-entity independent | All entities | **Yes** |
| construction | No — sequential state machine | Active constructions | No |
| ai | No — complex branching | Per-corporation | No |
| contract | No — state machine | Active contracts | No |
| finance | No — sequential aggregation | Per-corporation | No |
| all others | No — branchy logic or small data | Various | No |

10 of 36 systems are GPU candidates. These are also the most computationally expensive ones.

### 4.2 GPU compute architecture

```
Sim Worker (CPU/WASM):
  1. Run CPU-only systems (construction, ai, contract, finance, legal, etc.)
  2. Upload dirty entity data to GPU storage buffers
  3. Dispatch GPU compute shaders for parallel systems
  4. Read back results (coverage maps, utilization values, positions)
  5. Continue with remaining CPU systems that depend on GPU results

GPU Compute:
  - Storage buffers: node positions, edge endpoints, cell grid, coverage map
  - Compute shaders: one per GPU-eligible system
  - Dispatch: workgroup_size(64) for most, workgroup_size(8,8) for grid-based
```

### 4.3 Coverage compute shader (example)

```wgsl
// coverage.compute.wgsl
struct Node {
    lon: f32, lat: f32,
    coverage_radius: f32,
    bandwidth: f32,
    owner: u32,
    active: u32,
};

@group(0) @binding(0) var<storage, read> nodes: array<Node>;
@group(0) @binding(1) var<storage, read> cells: array<vec2<f32>>; // cell centers
@group(0) @binding(2) var<storage, read_write> coverage: array<f32>; // output

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let cell_idx = id.x;
    if (cell_idx >= arrayLength(&cells)) { return; }

    let cell_pos = cells[cell_idx];
    var total_coverage: f32 = 0.0;

    for (var i: u32 = 0u; i < arrayLength(&nodes); i++) {
        if (nodes[i].active == 0u) { continue; }
        let dist = haversine(cell_pos, vec2(nodes[i].lon, nodes[i].lat));
        if (dist < nodes[i].coverage_radius) {
            total_coverage += nodes[i].bandwidth * (1.0 - dist / nodes[i].coverage_radius);
        }
    }

    coverage[cell_idx] = total_coverage;
}
```

With 10,000 cells and 5,000 nodes, this dispatches 10,000 workgroups of 64 threads each — trivial for any modern GPU. What takes 20ms on CPU takes <1ms on GPU.

### 4.4 Hybrid tick orchestration

Update `systems/mod.rs` to split the tick into CPU and GPU phases:

```
Phase 1 (CPU): construction, maintenance
Phase 2 (GPU): orbital, satellite_network, coverage, demand, utilization, weather
Phase 3 (CPU, depends on GPU results): routing, revenue, cost, finance
Phase 4 (CPU): contract, ai, regulation, research, patent, market, auction, ...
Phase 5 (GPU): weather visualization data, overlay textures
```

GPU phases dispatch compute shaders and immediately continue with independent CPU work. GPU results are read back only when needed by dependent systems.

### 4.5 rust-gpu exploration (optional)

Evaluate writing compute shaders in Rust via rust-gpu instead of WGSL:
- Pro: same language as ECS engine, shared type definitions
- Pro: can potentially share coverage logic between CPU and GPU paths
- Con: requires nightly Rust, SPIR-V -> naga -> WGSL transpilation pipeline
- Con: limited Rust subset (no std, no heap, no dynamic dispatch)

Prototype one system (coverage) with rust-gpu. If the DX is good, migrate others.

**Expected impact:** 10-100x speedup for GPU-eligible systems. Tick time dominated by CPU-only systems (which are the lightweight ones). 50ms target easily achievable at 50,000+ entities.

---

## Phase 5: Frontend Optimizations (Week 3, parallel with Phase 3)

### 5.1 Throttle panel queries

Replace every-tick `$effect` in panels with tick-modulo gating:

```typescript
// Before: fires every tick
$effect(() => {
    const _tick = $worldInfo.tick;
    debts = bridge.getDebtInstruments(corpId);
});

// After: fires every 5th tick (200ms at normal speed)
$effect(() => {
    const tick = $worldInfo.tick;
    if (tick % 5 !== 0) return;
    debts = bridge.getDebtInstruments(corpId);
});
```

Apply to: DashboardPanel, InfraPanel, WorkforcePanel, ContractPanel, ResearchPanel. Only finance-critical displays (cash, revenue) update every tick.

### 5.2 D3 chart incremental updates

Replace full SVG nuke-and-rebuild with D3 transitions:

```typescript
// Before:
svg.selectAll('*').remove();
// ... rebuild everything

// After:
const line = svg.selectAll('.line').data([data]);
line.enter().append('path').attr('class', 'line')
    .merge(line)
    .transition().duration(200)
    .attr('d', lineGenerator);
// Only update what changed
```

For FinanceChart, PopulationChart, NetworkDiagram, MarketShareChart.

### 5.3 Consolidate setIntervals in MapView

Replace 3 separate `setInterval` calls with one:

```typescript
const interval = setInterval(() => {
    renderer?.updateInfrastructure();
    renderer?.updateCities();
    if (frameCount % 4 === 0) { // every 4th interval (2s)
        renderer?.updateGhostBuildOptions();
    }
}, 500);
```

### 5.4 Memoize derived panel values

Cache computed values (nodeTypeDistribution, profitMargin, etc.) and only recalculate when the input data reference changes, not on every tick:

```typescript
let lastInfraRef: any = null;
let cachedDistribution: Map<string, number> | null = null;

$effect(() => {
    if (infraNodes !== lastInfraRef) {
        lastInfraRef = infraNodes;
        cachedDistribution = computeDistribution(infraNodes);
    }
});
```

### 5.5 Scheduler API for task prioritization

```typescript
// UI input: highest priority (never blocked by sim)
scheduler.postTask(() => handleClick(e), { priority: 'user-blocking' });

// Sim tick: important but can yield
scheduler.postTask(() => simWorker.postMessage({ type: 'tick' }), { priority: 'user-visible' });

// Auto-save: background
scheduler.postTask(() => saveGame(), { priority: 'background' });
```

### 5.6 Compute Pressure API for adaptive quality

```typescript
const observer = new PressureObserver((records) => {
    const state = records[0].state; // 'nominal' | 'fair' | 'serious' | 'critical'
    switch (state) {
        case 'critical':
            disableWeatherEffects();
            reduceOverlayResolution();
            skipNonCriticalSystems();
            break;
        case 'serious':
            reduceParticleCount();
            lowerMapDetail();
            break;
        // 'nominal' and 'fair': full quality
    }
});
observer.observe('cpu', { sampleInterval: 2000 });
```

Chrome stable. Graceful degradation on unsupported browsers (feature-detect, no-op).

**Expected impact:** Panels 5x less work. Charts smoother. Adaptive quality on low-end devices.

---

## Phase 6: WASM Threads (Week 6, after Worker architecture)

Use Rayon's `par_iter` inside the WASM sim worker for per-system entity parallelism.

### 6.1 Toolchain setup

Pin to the most stable nightly that supports `wasm32-unknown-unknown` atomics. Chrome has supported SharedArrayBuffer since 2018 — this is mature on the browser side.

```bash
# rust-toolchain.toml
[toolchain]
channel = "nightly-2025-11-15"  # or latest stable nightly with atomics
targets = ["wasm32-unknown-unknown"]
```

Build flags:
```bash
RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals,+simd128"
```

### 6.2 Add wasm-bindgen-rayon

In `crates/gt-wasm/Cargo.toml`:
```toml
[dependencies]
wasm-bindgen-rayon = "1"
rayon = "1"
```

In `crates/gt-wasm/src/lib.rs`:
```rust
pub use wasm_bindgen_rayon::init_thread_pool;
```

JS side:
```typescript
await initThreadPool(navigator.hardwareConcurrency);
```

### 6.3 Parallelize entity iteration in hot systems

In systems that process entities independently:

```rust
// Before:
for (&eid, node) in &world.infra_nodes {
    let pos = world.positions.get(&eid);
    process_node(node, pos);
}

// After:
use rayon::prelude::*;
let results: Vec<_> = world.infra_nodes.par_iter()
    .map(|(&eid, node)| {
        let pos = world.positions.get(&eid);
        process_node(node, pos)
    })
    .collect();
```

Apply to: utilization, coverage (if not on GPU), orbital, revenue, cost.

**Important:** Collect results in deterministic order (sort by entity ID after parallel processing) to maintain multiplayer sync.

### 6.4 COOP/COEP headers

Required for SharedArrayBuffer. Add to Cloudflare/Vercel config:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

Ensure all external resources (map tiles, fonts, icons) are served with `Cross-Origin-Resource-Policy: cross-origin` or use CORS.

### 6.5 Fallback

Feature-detect SharedArrayBuffer:
```typescript
if (typeof SharedArrayBuffer !== 'undefined') {
    await initThreadPool(navigator.hardwareConcurrency);
} else {
    // Single-threaded fallback — no rayon, same code paths
}
```

**Expected impact:** 2-4x faster per-system entity processing. Scales with core count. Combined with GPU compute, the heaviest systems run in parallel on both CPU threads and GPU.

---

## Phase 7: Multiplayer Transport (Week 7-8)

### 7.1 WebTransport with WebSocket fallback

Create a transport abstraction layer:

```typescript
interface GameTransport {
    connect(url: string): Promise<void>;
    sendReliable(data: ArrayBuffer): void;      // commands, acks
    sendUnreliable(data: ArrayBuffer): void;     // state deltas, ticks
    onReliable(handler: (data: ArrayBuffer) => void): void;
    onUnreliable(handler: (data: ArrayBuffer) => void): void;
    close(): void;
}

class WebTransportTransport implements GameTransport { ... }
class WebSocketTransport implements GameTransport { ... }
```

Feature-detect and prefer WebTransport:
```typescript
function createTransport(url: string): GameTransport {
    if ('WebTransport' in window) {
        return new WebTransportTransport(url);
    }
    return new WebSocketTransport(url);
}
```

### 7.2 Message channel separation

With WebTransport's multiplexed streams:

| Channel | Transport | Content |
|---------|-----------|---------|
| Stream 0 (reliable) | Bidirectional stream | Commands + CommandAck |
| Stream 1 (reliable) | Bidirectional stream | Chat messages |
| Datagrams (unreliable) | UDP-like datagrams | TickUpdate, CommandBroadcast (DeltaOps) |
| Stream 2 (reliable) | Unidirectional (server→client) | Full snapshots (every 30 ticks) |

State deltas via unreliable datagrams means a lost packet never blocks subsequent updates. The next delta overwrites stale data anyway.

### 7.3 Server-side WebTransport

Add WebTransport endpoint to `gt-server` alongside existing WebSocket:

```rust
// Cargo.toml
[dependencies]
wtransport = "0.5"
```

Run both listeners. Clients negotiate transport on connect.

### 7.4 Binary protocol optimization

Switch from JSON debug protocol to MessagePack-only for all wire messages. The `@msgpack/msgpack` dependency already exists. Remove JSON serialization from the multiplayer hot path entirely.

**Expected impact:** 23% lower latency. No head-of-line blocking for state updates. Independent channels for different message types.

---

## Phase 8: Advanced Optimizations (Weeks 8-10)

### 8.1 Explicit SIMD intrinsics for hot loops

After Phase 0's auto-vectorization, identify remaining hot loops via profiling and add explicit SIMD:

```rust
use core::arch::wasm32::*;

// Batch distance calculation: 2 coordinate pairs at once with f64x2
fn batch_haversine_simd(positions: &[f64], target: (f64, f64)) -> Vec<f64> {
    // ... SIMD implementation
}
```

Use the `wide` crate for portable SIMD that compiles to wasm_simd128:
```toml
[dependencies]
wide = "0.7"
```

Priority targets: Haversine distance (coverage, satellite), bandwidth aggregation (utilization), financial sums (revenue/cost).

### 8.2 ECS storage optimization

Consider switching hot-path component storage from `HashMap<EntityId, T>` to contiguous `Vec<T>` with entity-index mapping:

```rust
// Current: HashMap iteration (cache-unfriendly)
for (&eid, node) in &world.infra_nodes { ... }

// Optimized: Vec iteration (cache-friendly, SIMD-friendly)
for (idx, node) in world.infra_nodes_vec.iter().enumerate() { ... }
```

Contiguous memory layout enables:
- CPU cache prefetching
- SIMD vectorization over packed arrays
- GPU buffer upload without transformation

This is a significant refactor. Only pursue if profiling shows HashMap iteration as a bottleneck.

### 8.3 Lazy query materialization

Instead of serializing query results eagerly, use a pull-based model where the frontend requests specific fields:

```typescript
// Before: serialize entire corporation data every tick
const corp = bridge.getCorporationData(corpId); // 50+ fields serialized

// After: request only what's displayed
const cash = bridge.getCorpCash(corpId);      // single f64
const revenue = bridge.getCorpRevenue(corpId); // single f64
```

Reduces serialization cost to only the fields actively displayed on screen.

### 8.4 Service Worker WASM caching

Enhance `web/src/service-worker.ts` to pre-cache the WASM binary and serve it with streaming-compatible responses:

```typescript
// Cache WASM binary on install
self.addEventListener('install', (e) => {
    e.waitUntil(
        caches.open('wasm-v1').then(cache =>
            cache.add('/wasm/gt_wasm_bg.wasm')
        )
    );
});
```

Combined with V8's code caching, third-load startup approaches native app speed.

---

## Phase 9: Future Tech (2026-2027, exploratory)

### 9.1 WebNN for ML-powered AI corporations

When WebNN ships in stable Chrome (expected 2026):
- Train small neural nets offline for AI corporation decision-making
- Deploy as ONNX models, run inference via WebNN
- Hardware-accelerated on GPU/NPU
- Single-player only (ML breaks deterministic multiplayer sync)
- Enables AI opponents that learn and adapt, not just follow archetypes

### 9.2 WASM Component Model for mod support

When WASI 1.0 ships in browsers (expected late 2026/early 2027):
- Modularize ECS systems as separate WASM components
- Hot-load custom AI logic, new building types, economy mods
- Community mods as `.wasm` components with sandboxed capabilities
- WIT interfaces define the contract between mod and engine

### 9.3 rust-gpu for unified CPU/GPU code

If rust-gpu matures:
- Write ECS systems once in Rust
- Compile to WASM for CPU path, SPIR-V -> WGSL for GPU path
- Same deterministic logic, different execution targets
- Eliminates the need to maintain separate WGSL compute shaders

---

## Dependency Graph

```
Phase 0 (Build)
    │
    ├──► Phase 1 (Sim Hot Path) ──► Phase 6 (WASM Threads)
    │                                    │
    ├──► Phase 2 (Web Worker) ──────────►│
    │         │                          │
    │         ├──► Phase 3 (WebGPU Renderer) ──► Phase 4 (GPU Compute)
    │         │
    │         └──► Phase 7 (WebTransport)
    │
    ├──► Phase 5 (Frontend) [parallel with Phase 3]
    │
    └──► Phase 8 (Advanced) [after profiling Phases 1-6]
              │
              └──► Phase 9 (Future Tech) [exploratory]
```

## Timeline Summary

| Phase | What | Duration | Depends On |
|-------|------|----------|------------|
| **0** | Build pipeline fixes | 1 day | Nothing |
| **1** | Sim hot path optimization | 1 week | Phase 0 |
| **2** | Web Worker architecture | 1 week | Phase 0 |
| **3** | WebGPU renderer (replace deck.gl) | 3 weeks | Phase 2 |
| **4** | GPU compute shaders | 2 weeks | Phase 3 |
| **5** | Frontend optimizations | 1 week | Phase 0 (parallel with 3) |
| **6** | WASM threads (rayon) | 1 week | Phase 2 |
| **7** | WebTransport multiplayer | 2 weeks | Phase 2 |
| **8** | Advanced optimizations | 2 weeks | Phases 1-6 (profiling-driven) |
| **9** | Future tech exploration | Ongoing | Various |

## Browser Compatibility

| Feature | Chrome | Firefox | Safari | Edge |
|---------|--------|---------|--------|------|
| WASM SIMD | Yes | Yes | Yes | Yes |
| Web Workers | Yes | Yes | Yes | Yes |
| Transferable Objects | Yes | Yes | Yes | Yes |
| SharedArrayBuffer | Yes | Yes | Yes | Yes |
| WebGPU | Yes | 141+ | 26+ | Yes |
| WebGPU Compute | Yes | 141+ | 26+ | Yes |
| OffscreenCanvas | Yes | Yes | 16.4+ | Yes |
| WebTransport | Yes | 138+ | Flag only | Yes |
| Compute Pressure API | Yes | No | No | Yes |
| Scheduler API | Yes | Yes | No | Yes |
| WebNN | 146 OT | No | No | Planned |

**Minimum viable:** Chrome (all features). **Broad support:** Chrome + Firefox + Edge (all except WebNN). **Full support:** All browsers for core features; Safari trails on WebTransport and newer APIs.

## Success Metrics

| Metric | Current (estimated) | Phase 0 | Phase 1-2 | Phase 3-4 | Phase 6-8 |
|--------|-------------------|---------|-----------|-----------|-----------|
| WASM binary size | ~8MB | ~4MB | ~4MB | ~4MB | ~4MB |
| Tick time (10k entities) | ~50-100ms | ~40-80ms | ~15-30ms | ~5-10ms | ~3-5ms |
| Main thread tick blocking | 50-100ms | 40-80ms | 0ms | 0ms | 0ms |
| Map render (fps) | ~20 effective | ~20 | ~20 | 60 | 60 |
| Initial load | ~4-5s | ~3s | ~3s | ~3s | ~2s |
| Multiplayer RTT | ~100ms | ~100ms | ~100ms | ~100ms | ~65ms |
| Entity capacity | ~10,000 | ~15,000 | ~30,000 | ~100,000+ | ~100,000+ |
