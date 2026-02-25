# Instant Multiplayer + Anti-Cheat + Performance Overhaul

## Implementation Status: ALL 10 PHASES COMPLETE

All phases implemented, compiled, and verified:
- `cargo build` — 0 errors, 0 warnings
- `cargo test` — 79 tests passing
- `wasm-pack build` — success
- `bun run check` — 0 errors, 0 warnings
- `desktop/src-tauri cargo check` — compiles clean

## Overview

Transform GlobalTelco's multiplayer from a 6-second-latency polling system to sub-100ms instant sync with full anti-cheat, optimistic UI, and a high-performance typed array bridge. Every action — building, upgrading, decommissioning, repairing — is instantly visible to all connected players.

---

## Architecture Decisions (Locked In)

### Multiplayer Sync
| Decision | Choice |
|----------|--------|
| Sync model | Server-authoritative, event-driven delta broadcasts |
| Optimistic UI | Ghost for builder, immediate confirmed entity for everyone else |
| Broadcast payload | Server builds full entity snapshot after command (~300 bytes/node) |
| Snapshot frequency | Every 30 seconds as safety net (down from 5 ticks) |
| Map rendering trigger | Event-driven on broadcasts + 2-second fallback poll |
| Ghost scope | All visual commands: nodes, edges, upgrades, decommissions, repairs |
| Rollback UX | Red flash + fade out + error toast |
| Enhanced CommandAck | Returns entity_id + effective_tick + seq echo |
| WASM delta application | `applyBatch()` with typed DeltaOp enum — push directly into WASM state |

### Fog of War / Visibility
| Decision | Choice |
|----------|--------|
| Infrastructure | Always visible to all players (real-world logic — you can see competitors' towers) |
| Financials | Gated by espionage intel levels (existing 0-3 system) |
| Research/tech | Hidden until completed; gated by intel |
| Events in TickUpdate | Per-player filtering: infra events always pass, financial/internal gated by intel |

### Anti-Cheat & Security
| Decision | Choice |
|----------|--------|
| Replay protection | Full: client seq numbers, server dedup, command journal, prompt-on-reconnect |
| Rate limiting | Per-command-type: Build 3/sec, Financial 2/sec, Research 1/5sec, Espionage 1/30sec |
| Spatial validation | Full server-side: finite coords, world bounds, terrain compatibility, min distance |
| Economic validation | Server pre-validates funds, prerequisites, entity state before processing |

### Speed Control
| Decision | Choice |
|----------|--------|
| Model | World creator gets override power. Others need majority vote. |
| Creator assignment | First non-spectator player to join becomes creator |
| Vote window | 30 seconds |
| Tie-breaking | Keep current speed on tie |

### Performance Bridge
| Decision | Choice |
|----------|--------|
| WASM bridge | Typed arrays for hot-path rendering queries (zero-copy Float64Array views into WASM linear memory) |
| String fields | All typed arrays with string table (Uint8Array with offsets) |
| Tauri bridge | Separate `gt-tauri` crate with `#[tauri::command]` wrappers, native Rust — no WASM overhead |
| Shared interface | `BridgeQuery` trait implemented by both gt-wasm and gt-tauri |
| Three compilation targets | Browser (WASM + typed arrays), Desktop (Tauri native), Server (native binary) |

### Admin
| Decision | Choice |
|----------|--------|
| Admin panel | Enhanced debug endpoint, real-time monitoring, player kick/ban |
| Ban persistence | In-memory initially, PostgreSQL later |
| Admin key | Fix the `!` character quoting issue in systemd .env |

---

## Current Latency Chain (What We're Fixing)

```
Player clicks Build → WebSocket (~100ms) → Server validates + executes →
CommandAck (success only, no data) → Wait for TickUpdate (~1s) →
Wait for Snapshot (~5s) → bridge.loadGame(full JSON) → MapView polls (500ms)
= ~1-6 seconds worst case
```

## Target Latency Chain

```
Player clicks Build → Ghost appears instantly (0ms) →
WebSocket (~50-100ms) → Server validates + executes →
CommandAck (entity_id + tick) confirms ghost →
CommandBroadcast to all players (~50-100ms) →
Other players see confirmed entity instantly via applyBatch → map-dirty event → render
= <200ms for all players
```

---

## Phase 1: Enhanced Protocol Foundation [COMPLETE]

Enrich the protocol types that everything else builds on.

**Changes:**
- `protocol.rs`: Enhance `CommandAck` with `seq: Option<u64>`, `entity_id: Option<EntityId>`, `effective_tick: Option<Tick>`
- `protocol.rs`: Add `seq: Option<u64>` field to `ClientMessage::GameCommand`
- `protocol.rs`: New `ServerMessage::CommandBroadcast` variant carrying tick, player_id, corp_id, command echo, and full entity data (node/edge struct with all fields)
- `protocol.rs`: New `ServerMessage::SpeedVoteUpdate` variant
- `protocol.rs`: New `WorldDelta` and `DeltaOp` types for batch operations
- `world.rs`: Change `process_command()` return type from `()` to `CommandResult { success, error, entity_id }`
- `world.rs`: Every `cmd_*` method returns entity IDs on success, error messages on failure
- `gt-wasm/lib.rs`: Update `process_command` WASM export to surface the new return type
- `ws.rs`: Update GameCommand handler to capture `CommandResult` and build enriched `CommandAck`

**Backward compat:** All new fields are `Option` with `#[serde(default)]` + `skip_serializing_if`.

---

## Phase 2: Command Broadcast [COMPLETE]

After a successful command, broadcast entity data to all players in the world.

**Changes:**
- `ws.rs` GameCommand handler: After `process_command` succeeds, read back the created/modified entity's full data from the world state (node fields, position, health, construction status)
- `ws.rs`: Build a `CommandBroadcast` message with the full entity snapshot and send via `broadcast_tx`
- `ws.rs`: Clone the command before `process_command` consumes it (Command already derives Clone)
- `WebSocketClient.ts`: Handle `CommandBroadcast` message, dispatch `mp-command-broadcast` custom event
- `GameLoop.ts`: Listen for `mp-command-broadcast` in `initMultiplayer()`, build a `WorldDelta` from the broadcast, call `bridge.applyBatch()` to push into WASM, dispatch `map-dirty` event

**Key detail:** The broadcast goes through the existing per-player filter task. Since infrastructure is always visible, `CommandBroadcast` passes through unmodified for all players.

---

## Phase 3: Optimistic Ghost UI + Rollback [COMPLETE]

Builder sees instant ghost. Server confirms or rejects. Other players see confirmed entity immediately (not a ghost).

**Changes:**
- `multiplayerState.ts`: New stores — `ghostEntities`, `pendingCommands` (Map by seq), `nextSeq`
- `commandRouter.ts`: Major expansion — `gameCommand()` creates ghosts for visual commands before sending to server. Ghost types: node placement, edge placement, upgrade glow, decommission fade, repair pulse
- `commandRouter.ts`: New `CommandJournal` class tracking pending commands by seq
- `WebSocketClient.ts`: Handle enriched `CommandAck`, dispatch `mp-command-ack` with seq/entity_id/tick
- `GameLoop.ts`: On `mp-command-ack` success — link ghost to real entity_id, transition from translucent to solid. On failure — red flash, fade out, error notification. 5-second timeout for unacked ghosts.
- `infraLayer.ts`: New ghost rendering layers — translucent nodes/edges with pulsing animation, dashed ghost edges, red rollback flash animation
- `MapRenderer.ts`: Pass ghost entities to `createInfraLayers()`

**Key detail:** Only the builder sees ghosts. Other players receive the `CommandBroadcast` and see the entity as fully confirmed immediately via the Phase 2 `applyBatch` flow.

---

## Phase 4: WASM applyBatch + Event-Driven Map [COMPLETE]

Replace expensive full-snapshot reloads with incremental delta application. Replace 500ms polling with event-driven rendering.

**Changes:**
- `gt-common` or `gt-simulation`: Define `WorldDelta { tick, operations: Vec<DeltaOp> }` and `DeltaOp` enum (NodeCreated, EdgeCreated, NodeRemoved, NodeHealthUpdated, FinancialUpdated, ConstructionCompleted, TickAdvanced)
- `world.rs`: New `apply_delta(&mut self, delta: &WorldDelta)` method — direct HashMap mutations for each DeltaOp (insert into infra_nodes, positions, ownerships, healths, capacities, constructions, corp_infra_nodes, network graph)
- `gt-wasm/lib.rs`: New `apply_batch(batch_json: &str)` wasm-bindgen export
- `bridge.ts`: New `applyBatch(json: string)` function
- `types.ts`: TypeScript types for WorldDelta and DeltaOp
- `tick.rs`: Change `CLIENT_SNAPSHOT_INTERVAL_TICKS` from 5 to 30
- `MapView.svelte`: Replace 500ms `setInterval` with event listener for `map-dirty` + 2-second fallback poll
- `GameLoop.ts`: Dispatch `map-dirty` event after applying any delta, ghost state change, or snapshot

**Key detail:** `apply_delta` bypasses ECS systems intentionally — deltas represent already-validated server state. Full snapshots every 30 seconds overwrite everything as a safety net.

---

## Phase 5: Per-Type Rate Limiting + Spatial Validation [COMPLETE]

Replace global rate limiter with per-command-type limits. Add server-side spatial anti-cheat.

**Changes:**
- `ws.rs`: Replace `RateLimiter` with `PerTypeRateLimiter` using `CommandCategory` enum (Build 3/sec, Financial 2/sec, Research 1/5sec, Espionage 1/30sec, GameControl 2/sec, Other 5/sec)
- `ws.rs`: New `validate_build_command()` function — check finite coords, world bounds (-180/180, -90/90), terrain compatibility (no land nodes on ocean), minimum distance between nodes
- `ws.rs`: Restructure GameCommand handler order: parameter validation → per-type rate limit → corp ownership → acquire mutex → spatial validation → process command → broadcast → release mutex → return ack
- `ws.rs`: Economic pre-validation: check funds, tech prerequisites, entity state before processing

**Key detail:** Spatial validation runs inside the mutex (it reads world state). The distance check is O(n) on positions but <1ms for 10k entities.

---

## Phase 6: Speed Vote System [COMPLETE]

World creator has override power. Other players vote with 30-second window.

**Changes:**
- `state.rs`: Add `creator_id: Option<Uuid>` and `speed_votes: RwLock<SpeedVoteState>` to `WorldInstance`
- `ws.rs`: On JoinWorld — if first non-spectator player, set as creator
- `ws.rs`: Intercept `SetSpeed`/`TogglePause` commands in multiplayer: creator applies immediately, others record vote + broadcast `SpeedVoteUpdate`
- `ws.rs` or `tick.rs`: Periodic check for vote window expiry (30s), apply plurality result
- `WebSocketClient.ts`: Handle `SpeedVoteUpdate`, dispatch to store
- `multiplayerState.ts`: New `speedVoteState` store
- `SpeedControls.svelte`: In MP mode, clicking speed sends a vote. Show vote tally overlay, countdown timer, "Creator Override" badge

---

## Phase 7: Per-Player Event Filtering [COMPLETE]

Filter TickUpdate events so competitor internals don't leak.

**Changes:**
- `events.rs`: Add `related_corp(&self) -> Option<EntityId>` method on `GameEvent` (maps each variant to its corp, or None for public events)
- `ws.rs`: Extend `filter_tick_update_for_player` to filter the `events` vector (currently passed through unmodified). Rules:
  - Own events: always visible
  - Infrastructure events (NodeBuilt, EdgeBuilt, ConstructionStarted/Completed, NodeDestroyed): always visible (infra is public)
  - Financial events (Revenue, Cost, Loan, Bankruptcy): require intel >= 1
  - Internal events (Research, Staffing, Policy): require intel >= 2
  - Public events (Disaster, Regulation, Market): always visible
- Handle multi-corp events (ContractProposed): visible if player is either party or has intel on either party

---

## Phase 8: Sequence Numbers + Reconnect Protection [COMPLETE]

Full replay protection and graceful reconnection.

**Changes:**
- `state.rs` or `ws.rs`: Add `last_seq: u64` per player, persisted across reconnections in the world's player map
- `ws.rs`: GameCommand handler — reject if seq <= last_seq (duplicate/replay), update last_seq on success
- `commandRouter.ts`: `CommandJournal` class — stores pending commands by seq, marks acked, prunes old entries
- `WebSocketClient.ts`: On reconnect after re-auth and re-join — check journal for unacked commands, show prompt: "You had N pending actions. Retry them?"
- `GameLoop.ts`: On `mp-command-ack`, mark journal entry as acked

---

## Phase 9: Admin Panel Enhancements [COMPLETE]

Ban system, monitoring, fix admin key.

**Changes:**
- `state.rs`: Add `banned_players: RwLock<HashMap<Uuid, BanEntry>>` and `banned_ips: RwLock<HashMap<IpAddr, BanEntry>>` to AppState
- `ws.rs`: Check ban list during auth — reject banned players/IPs with PermissionDenied
- `routes.rs`: New endpoints — `POST /api/admin/ban`, `POST /api/admin/unban`, `GET /api/admin/bans`, `GET /api/admin/debug/{world_id}/entities`, `GET /api/admin/monitoring`
- `admin/api.ts`: New API functions for ban management and monitoring
- `admin/+page.svelte`: Ban management panel, real-time monitoring dashboard, enhanced entity browser
- Deploy script: Fix ADMIN_KEY quoting in .env file (quote the value to handle `!` in systemd)

---

## Phase 10: Typed Array Bridge + Tauri Bridge [COMPLETE]

Zero-copy rendering pipeline for browser. Native Rust bridge for desktop.

**Changes:**

### Shared Interface
- New `crates/gt-bridge/` crate with `BridgeQuery` trait defining the query API (get_infrastructure_list, get_all_infrastructure, get_visible_entities, etc.)
- Both gt-wasm and gt-tauri implement `BridgeQuery`

### WASM Typed Array Bridge (gt-wasm)
- New typed array exports alongside existing JSON ones
- For infrastructure queries: write entity data as flat arrays into WASM linear memory
  - `Float64Array` for positions: `[lon0, lat0, lon1, lat1, ...]`
  - `Float64Array` for stats: `[health0, utilization0, throughput0, ...]`
  - `Uint32Array` for IDs and enums: `[entity_id0, node_type0, owner0, ...]`
  - `Uint8Array` string table for names: packed bytes with offset/length index
- TypeScript reader creates zero-copy `Float64Array` views into `wasmMemory.buffer`
- deck.gl layers consume typed arrays directly (ScatterplotLayer, LineLayer natively support typed arrays for positions)
- Keep JSON exports for non-hot-path queries (world info, corporation data, research state)

### Tauri Native Bridge (gt-tauri)
- New `crates/gt-tauri/` crate
- `#[tauri::command]` functions wrapping gt-simulation queries
- Direct Rust struct returns (Tauri auto-serializes via serde)
- For hot-path rendering: raw byte channel pushing binary buffers to frontend
- Wire into `desktop/src-tauri/` — add gt-simulation + gt-tauri as dependencies, register commands in main.rs
- Frontend detection: `bridge.ts` checks for `window.__TAURI__` and uses Tauri invoke instead of WASM calls

### Current Tauri State
- `desktop/src-tauri/` exists with Tauri v2, has save/load file commands only
- No simulation integration — currently wraps the WASM-based web frontend
- gt-tauri will add native simulation commands alongside existing file commands

---

## Phase Dependency Graph

```
Phase 1 (Protocol Foundation)
    |
    v
Phase 2 (Command Broadcast)
    |
    v
Phase 3 (Ghost UI + Rollback)
    |
    v
Phase 4 (applyBatch + Event-Driven Map)

Phase 5 (Rate Limiting + Spatial Validation) — parallel with 3-4
Phase 6 (Speed Votes) — parallel with 3-5
Phase 7 (Event Filtering) — parallel with 3-6
Phase 8 (Seq Numbers + Reconnect) — after Phase 1, parallel with 4+
Phase 9 (Admin Panel) — independent, anytime
Phase 10 (Typed Array + Tauri Bridge) — independent, anytime after Phase 4
```

**Execution order:** 1 → 2 → 3 → [4 + 5 + 7] → [6 + 8] → [9 + 10]

---

## Files Touched Per Phase

| Phase | Rust Files | TypeScript Files |
|-------|-----------|-----------------|
| 1 | protocol.rs, world.rs, gt-wasm/lib.rs, ws.rs | — |
| 2 | ws.rs | WebSocketClient.ts, GameLoop.ts |
| 3 | — | multiplayerState.ts, commandRouter.ts, infraLayer.ts, MapRenderer.ts, GameLoop.ts |
| 4 | gt-common (delta types), world.rs, gt-wasm/lib.rs, tick.rs | bridge.ts, types.ts, MapView.svelte, GameLoop.ts |
| 5 | ws.rs | — |
| 6 | state.rs, ws.rs | WebSocketClient.ts, multiplayerState.ts, SpeedControls.svelte, GameLoop.ts |
| 7 | ws.rs, events.rs | — |
| 8 | state.rs, ws.rs | commandRouter.ts, WebSocketClient.ts, GameLoop.ts |
| 9 | state.rs, routes.rs, ws.rs | admin/api.ts, admin/+page.svelte |
| 10 | NEW gt-bridge/, gt-wasm/lib.rs, NEW gt-tauri/, desktop/src-tauri/ | bridge.ts, infraLayer.ts |

---

## Message Flow Diagrams

### Build Node (Builder's Perspective)
```
Builder clicks "Build Cell Tower at (34.05, -118.25)"
    |
    v
commandRouter creates ghost node (translucent, pulsing) ← INSTANT (0ms)
commandRouter assigns seq=42, records in journal
commandRouter sends GameCommand { seq: 42, command: BuildNode { CellTower, 34.05, -118.25 } }
    |
    v [~50-100ms network]
    |
Server receives, validates (rate limit, ownership, spatial, economic)
Server acquires world mutex, process_command → CommandResult { entity_id: 1337 }
Server reads back full entity data from world state
Server sends CommandAck { success: true, seq: 42, entity_id: 1337, tick: 500 } to builder
Server broadcasts CommandBroadcast { tick: 500, player_id, corp_id, entity_data: {...} } to ALL
Server releases mutex
    |
    v [~50-100ms network back]
    |
Builder receives CommandAck → ghost seq=42 transitions from translucent to solid
Builder receives CommandBroadcast → applyBatch pushes node into WASM → map-dirty → render
Journal marks seq=42 as acked
```

### Build Node (Other Player's Perspective)
```
Other player is looking at the map
    |
    v
Receives CommandBroadcast { entity_data: { id: 1337, type: CellTower, lon: 34.05, lat: -118.25, ... } }
    |
    v
GameLoop builds WorldDelta from broadcast
bridge.applyBatch(delta) pushes node into WASM state
Dispatch 'map-dirty' event
MapView re-renders → infraLayer queries WASM → new node appears as solid confirmed entity
    |
    = ~100-200ms from when builder clicked
```

### Failed Build (Builder's Perspective)
```
Builder clicks "Build Cell Tower" on ocean
    |
    v
Ghost appears instantly (translucent) ← INSTANT
Command sent to server
    |
    v
Server spatial validation: "Cannot build on water"
Server sends CommandAck { success: false, error: "Cannot build on water", seq: 42 }
NO CommandBroadcast sent (command failed)
    |
    v
Builder receives failed ack → ghost turns red → fades out over 300ms
Error toast: "Cannot build on water"
Journal removes seq=42
```

---

## Bandwidth Budget

| Message | Size (MessagePack) | Frequency |
|---------|-------------------|-----------|
| CommandBroadcast (single node) | ~300 bytes | Per command (max 3/sec/player for builds) |
| TickUpdate (10 corps, filtered) | ~400-600 bytes | Every tick (1/sec) |
| Full Snapshot (compressed) | ~200-400 KB | Every 30 seconds |
| CommandAck | ~50 bytes | Per command |
| SpeedVoteUpdate | ~100 bytes | On vote change |

At peak: 8 players, 3 builds/sec each = 24 broadcasts/sec * 300 bytes = **7.2 KB/sec** total broadcast bandwidth. Trivial.

---

## Pre-Requisites Before Starting

1. Commit pending tick desync fixes (monotonic guards, high-water mark in GameLoop.ts + WebSocketClient.ts, debug endpoint in routes.rs)
2. Clean git state on main branch
