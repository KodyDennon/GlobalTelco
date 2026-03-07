# GlobalTelco Performance Audit

**Date:** March 5, 2026
**Scope:** Single-player performance, zoom lag, rendering architecture, and data transfer.

## Executive Summary

The primary cause of single-player lag and zoom stuttering is the **heavy use of JSON serialization** for transferring infrastructure data from the Rust/WASM simulation to the JavaScript frontend.

The rendering loop currently requests the *entire* infrastructure graph (nodes and edges) as a JSON string every few frames. This triggers expensive:
1.  **Serialization** (Rust side): Allocating and formatting a massive JSON string.
2.  **Deserialization** (JS side): Parsing a massive JSON string into JS objects.
3.  **Garbage Collection** (JS side): Creating thousands of temporary objects per frame.
4.  **Layer Re-construction**: Re-creating `deck.gl` layers from scratch, potentially invalidating internal buffers.

## 1. Critical Bottlenecks

### 1.1. `getAllInfrastructure` (Hot Path JSON)
- **Location:** `web/src/lib/game/map/layers/infraLayer.ts` calling `bridge.getAllInfrastructure()`.
- **Issue:** This function is called every ~3 frames (via `renderLayers`). It invokes `query_all_infrastructure` in Rust, which iterates **all** entities, serializes them to a JSON string, passes it to JS, and `JSON.parse`s it.
- **Impact:** O(N) CPU cost on both WASM and JS threads. As entity count grows (10k+), this causes frame drops and "stop-the-world" GC pauses.
- **Severity:** **CRITICAL**

### 1.2. Lack of Spatial Indexing
- **Location:** `crates/gt-bridge/src/queries.rs` -> `query_visible_entities`.
- **Issue:** The spatial query performs a linear scan (`iter().filter(...)`) over all entities to find those in the viewport.
- **Impact:** O(N) cost for viewport culling. While cheaper than sending everything, it still degrades linearly with world size.
- **Severity:** **HIGH**

### 1.3. Reactive Layer Re-creation
- **Location:** `web/src/lib/game/map/MapRenderer.ts` -> `renderLayers()`.
- **Issue:** `deck.gl` layers are re-instantiated with new data references on every render cycle. While `deck.gl` performs prop diffing, providing fresh data arrays (from `JSON.parse`) prevents shallow comparison optimizations and forces buffer updates.
- **Severity:** **MEDIUM**

## 2. Architectural Findings

### 2.1. Rendering Pipeline
- **Current:** Hybrid `MapLibre` (base) + `deck.gl` (overlay).
- **WIP:** `GPURenderer.ts` (WebGPU) exists but appears to be a work-in-progress or experimental path.
- **Recommendation:** Stick to `deck.gl` for now but optimize the data feed. `deck.gl` is capable of handling millions of points *if* fed correctly (typed arrays, binary attributes).

### 2.2. Unused Optimizations
- **Typed Arrays:** `crates/gt-wasm/src/typed_arrays.rs` and `web/src/lib/wasm/bridge.ts` already expose `getInfraNodesTyped()` and `getInfraEdgesTyped()`.
- **Status:** These return efficient `Float64Array`/`Uint32Array` views into WASM memory (zero-copy or low-copy).
- **Missed Opportunity:** The frontend is currently ignoring these efficiently exposed arrays in favor of the slow JSON path.

## 3. Recommended Fixes

### Phase 1: The "Low Hanging Fruit" (Immediate Performance)
1.  **Switch to Typed Arrays:** Refactor `infraLayer.ts` to consume `getInfraNodesTyped()` and `getInfraEdgesTyped()` instead of `getAllInfrastructure()`. This eliminates JSON parsing overhead entirely for the heaviest assets.
2.  **Memoize Data Access:** In `MapRenderer.ts`, only fetch new data when the simulation tick changes or an explicit dirty flag is set, rather than on every animation frame.

### Phase 2: Spatial Optimization
1.  **Rust Spatial Index:** Implement an R-Tree (via `rstar` crate) or a simple Grid Map in `gt-simulation` to speed up `get_visible_entities`.
2.  **Viewport Culling:** Only sync entities within the current viewport + a buffer zone to the frontend.

### Phase 3: Binary Protocol (Long Term)
1.  **Replace JSON:** For complex queries that can't use flat arrays, switch from JSON to `MessagePack` (already a dependency) or `FlatBuffers`.
2.  **Worker Offloading:** Move the WASM simulation to a Web Worker (already partially supported via `workerBridge.ts`) to keep the main thread free for rendering.

## 4. Questions for Performance Overhaul Plan

1.  **Typed Arrays State:** Is there a reason `getInfraNodesTyped` was not adopted? (e.g., missing fields, bugs?)
2.  **Browser Support:** Do we need to support browsers without `SharedArrayBuffer`? (Affects threading strategy).
3.  **Visual Fidelity:** Switching to typed arrays might make complex styling (like per-company coloring logic) slightly harder to read in code. Is code maintainability or raw performance the priority here?
