# GlobalTelco Comprehensive Audit Report

**Date:** 2026-02-21
**Version:** v0.1.0 (Early Development)
**Audited by:** 3 specialized agents reviewing all 36 game screenshots + full codebase analysis

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Critical Issues](#critical-issues)
3. [Major Issues](#major-issues)
4. [Minor Issues](#minor-issues)
5. [Polish Issues](#polish-issues)
6. [Verified Correct](#verified-correct)
7. [Designed vs Implemented](#designed-vs-implemented)

---

## Executive Summary

| Category | Critical | Major | Minor | Polish |
|----------|----------|-------|-------|--------|
| UI/UX & Visual Design | 4 | 8 | 9 | 10 |
| Gameplay & Simulation | 3 | 9 | 7 | -- |
| Server & Multiplayer | 5 | 9 | 9 | 5 |
| **Total** | **12** | **26** | **25** | **15** |

**Overall Assessment:** The game has a remarkably complete architectural foundation -- 15 real ECS systems, working WebSocket multiplayer, AI corporations, WASM bridge, and a full Svelte frontend. However, several critical issues prevent a viable player experience:

1. **The map is visually broken** -- chaotic polygon rendering makes the core gameplay surface unreadable
2. **Production secrets are committed to version control** -- total security compromise
3. **Multiplayer state sync is fundamentally broken** -- clients run local sim instead of using server state
4. **Player starting conditions are severely imbalanced** -- zero infrastructure vs AI's 3+ nodes

---

## Critical Issues

### C-01: PRODUCTION SECRETS COMMITTED TO VERSION CONTROL
**Category:** Security
**File:** `.env` (lines 8, 20, 23)

The `.env` file contains real production secrets in the repository:
- JWT signing secret (anyone can forge authentication tokens)
- Live Neon PostgreSQL connection string with full read/write credentials
- Admin API key (`"globaltelco"` -- a dictionary word)

**Impact:** Total compromise. An attacker with repo access can forge JWTs, access/destroy the database, and control the server.

**Fix:** (1) Immediately rotate all three secrets. (2) Add `.env` to `.gitignore`. (3) Remove from git history with `git filter-repo` or BFG. (4) Use deployment platform env vars exclusively.

---

### C-02: MAP RENDERING -- REGION POLYGONS PRODUCE CHAOTIC, NON-GEOGRAPHIC SHAPES
**Category:** UI/UX
**File:** `web/src/lib/game/MapRenderer.ts` (lines 309-336)

**Evidence:** Screenshots 01-08, 20-30. The procedural map renders as jagged, overlapping, brightly-colored triangular shards rather than coherent political regions. Shapes look like broken stained glass rather than a political map. Large bright triangles extend far beyond land boundaries into ocean.

**Root Cause:** `buildProcgenPolygons()` relies on `region.boundary_polygon` which produces highly irregular concave polygons. Three.js `ShapeGeometry` does not correctly triangulate complex concave polygons. The wrap-around triple-render (`-360, 0, 360` offsets) compounds the problem.

**Impact:** The game's core visual experience is fundamentally broken. No player can orient themselves, identify regions, or make strategic decisions. This is the most severe visual issue.

---

### C-03: OVERLAYS ARE VISUALLY INDISTINGUISHABLE FROM THE BASE MAP
**Category:** UI/UX
**File:** `web/src/lib/game/MapRenderer.ts` (lines 1103-1427)

**Evidence:** Screenshots 01 vs 03-08 are virtually identical. Overlay opacity values (0.08-0.15) produce no visible difference against the chaotic base map. No legends, color scales, or visual keys.

**Impact:** Six of seven overlay modes are non-functional from a user perspective. Players cannot use them for strategic decision-making.

---

### C-04: CORS ALLOWS ANY ORIGIN IN PRODUCTION
**Category:** Security
**File:** `crates/gt-server/src/main.rs` (lines 90-93)

```rust
let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);
```

Ships to production on Fly.io with no environment check. Any website can make authenticated requests to the server API.

**Fix:** Restrict to actual frontend origin(s). Keep `Any` behind `#[cfg(debug_assertions)]`.

---

### C-05: ADMIN API USES WEAK, LEAKED KEY WITH NO TIMING-SAFE COMPARISON
**Category:** Security
**File:** `crates/gt-server/src/routes.rs` (lines 484-501)

Hardcoded default `"globaltelco-dev-admin-key"`, production key is `"globaltelco"` (12 chars), string comparison with `==` enables timing attacks, no rate limiting or IP restriction.

**Fix:** Use `constant_time_eq`, generate 32+ byte random key, remove hardcoded default, add rate limiting.

---

### C-06: NO WEBSOCKET AUTHENTICATION ON UPGRADE
**Category:** Security
**File:** `crates/gt-server/src/routes.rs` (lines 466-468)

WebSocket upgrade accepts any connection without authentication. No connection limit per IP. Unauthenticated connections sit in the main loop consuming resources.

**Impact:** Trivial DoS -- thousands of idle WebSocket connections exhaust the 1 GB Fly.io VM.

**Fix:** Add auth timeout (close if no `Auth` within N seconds), per-IP limits, consider JWT query param for pre-auth.

---

### C-07: CHAT MESSAGES HAVE NO LENGTH LIMIT -- DoS VECTOR
**Category:** Security
**File:** `crates/gt-server/src/ws.rs` (lines 479-495)

Server broadcasts chat messages with no length validation. A client can send multi-megabyte messages broadcast to all players via `broadcast::channel`.

**Fix:** Add `if message.len() > 500 { return error; }`. Filter control characters and null bytes.

---

### C-08: PLAYER STARTS WITH ZERO INFRASTRUCTURE WHILE AI HAS 3+ NODES
**Category:** Gameplay
**File:** `crates/gt-simulation/src/world.rs` (lines 386-472)

AI corporations start with 3+ infrastructure nodes generating revenue from tick 1. The player starts with $5M cash, zero nodes, and zero revenue but immediately bleeds workforce salary costs.

**Impact:** Severe early-game imbalance. First 50+ ticks feel punishing. Player hemorrhages money with no income.

---

### C-09: ERA RESTRICTIONS NOT ENFORCED
**Category:** Gameplay
**Files:** WASM bridge `get_buildable_nodes`, `cmd_build_node`

All 8 node types are available in all eras, completely breaking the era progression design. A player starting in the Telegraph era (~1850s) can build 5G towers and fiber optic nodes.

---

### C-10: HUD TOP BAR IS OVERCROWDED AND UNREADABLE
**Category:** UI/UX
**File:** `web/src/lib/game/HUD.svelte`

**Evidence:** Screenshots 22, 28, 29. ~30+ interactive elements crammed into a single 48px-tall bar: company name, cash, profit/tick, Node/Edge buttons, edge type dropdown, 5 speed buttons, 11 panel icons, 7 overlay letters, multiplayer status, tick count, credit rating, infra count.

**Impact:** Below ~1400px width, elements overlap or wrap, becoming unusable.

---

### C-11: `u64::is_multiple_of` IS NIGHTLY-ONLY API
**Category:** Gameplay/Build
**Files:** 9 usages across multiple system files

Will fail to compile on stable Rust toolchain.

**Fix:** Replace with `tick % N == 0`.

---

### C-12: RESEARCH PANEL DOES NOT RENDER
**Category:** UI/UX
**File:** `web/src/lib/panels/ResearchPanel.svelte`

**Evidence:** Screenshot 12 shows the map with no panel visible. A core gameplay system (technology research tree) is either not displaying or invisible.

---

## Major Issues

### UI/UX

**M-UI-01: Overlay Toggle Buttons Use Single Cryptic Letters**
Letters "T", "O", "D", "C", "!", "~", "F" have no labels. The "~" for congestion is particularly opaque. Players cannot discover what each overlay does.
*File:* `web/src/lib/game/HUD.svelte` (lines 104-111)

**M-UI-02: Panel Icon Buttons Have No Visible Labels**
11 panel buttons render as tiny 16px icons without text. Players must hover each to find the right panel. Violates the "Bloomberg Terminal" design goal.
*File:* `web/src/lib/game/HUD.svelte` (lines 67-101)

**M-UI-03: Chat Panel Overlaps Events Feed**
Both positioned at bottom-right with only 8px difference. When both visible, they overlap and are unreadable.
*Files:* `web/src/lib/game/Chat.svelte` (line 69), `web/src/lib/game/NotificationFeed.svelte` (line 135)

**M-UI-04: Close Buttons Use Plain "x" Text**
Lowercase "x" with no background, no border, minimal styling. Far below 44px touch target. Easy to miss.
*Files:* `DashboardPanel.svelte`, `InfraPanel.svelte`, `ResearchPanel.svelte`, `InfoPanel.svelte`

**M-UI-05: No Loading/Progress Indicator for World Generation**
Single `<p>Loading</p>` text line. No progress bar, spinner, or estimated time.
*File:* `web/src/lib/game/GameView.svelte` (lines 73-77)

**M-UI-06: Financial Dashboard Uses Hardcoded English Strings**
"Budgets & Policies", "Maintenance Budget", "Expansion Priority", etc. bypass the i18n `$tr()` system.
*File:* `web/src/lib/panels/DashboardPanel.svelte` (lines 136-168)

**M-UI-07: Map Minimum Zoom Too Restrictive (1.3x)**
Players cannot see the full world map. Essential for "grand strategy" overview.
*File:* `web/src/lib/game/MapRenderer.ts` (line 1008)

**M-UI-08: No Confirmation Dialog for Destructive Actions**
Decommission button immediately destroys infrastructure (recovers only 20% cost) with no confirmation. Adjacent to upgrade button, both 24x24px.
*Files:* `web/src/lib/game/InfoPanel.svelte` (line 66), `web/src/lib/panels/InfraPanel.svelte` (line 155)

### Server & Multiplayer

**M-SRV-01: Tick Loop Deadlock Risk -- ABBA Lock Pattern**
`tick.rs` acquires world mutex then players rwlock. Reconnection path in `ws.rs` acquires players read lock then world mutex. Classic ABBA deadlock.
*Files:* `crates/gt-server/src/tick.rs` (lines 36-80), `crates/gt-server/src/ws.rs` (lines 287-306)

**M-SRV-02: Snapshot Does Not Include Full Game State**
`RequestSnapshot` returns only `tick` and `config`. No entities, corporations, infrastructure, finances, or regions. Late-joining players see nothing.
*File:* `crates/gt-server/src/ws.rs` (lines 445-466)

**M-SRV-03: Client Runs Local Sim in Multiplayer -- State Diverges**
`GameLoop.ts` calls `bridge.tick()` locally regardless of mode. In multiplayer, server is authoritative but client runs its own simulation creating two diverging states.
*File:* `web/src/lib/game/GameLoop.ts`

**M-SRV-04: Broadcast Channel Silently Drops Messages**
256-capacity `broadcast::channel`. When subscriber falls behind, `Lagged` error silently terminates the forwarder. Player permanently stops receiving updates.
*Files:* `crates/gt-server/src/state.rs` (line 54), `crates/gt-server/src/ws.rs` (lines 326-333)

**M-SRV-05: No Cloud Save Size Limit**
`save_data` field (`Vec<u8>`) has no size limit. A single client can fill the database or exhaust server memory.
*File:* `crates/gt-server/src/ws.rs` (lines 497-542)

**M-SRV-06: Audit Log Grows Unboundedly in Memory**
Every command appended to in-memory `Vec<AuditEntry>` that is never truncated. Eventual OOM on 1 GB VM.
*File:* `crates/gt-server/src/state.rs` (lines 260-273)

**M-SRV-07: In-Memory Account Cache Never Evicted**
Guest accounts accumulate forever with no eviction, TTL, or size limit.
*File:* `crates/gt-server/src/state.rs` (line 101)

**M-SRV-08: Server Auto-Stop on Fly.io Kills Game Worlds**
`min_machines_running = 0` and `auto_stop_machines = 'stop'` means the VM stops when idle, destroying all in-memory worlds.
*File:* `fly.toml` (lines 14-16)

**M-SRV-09: World Tick Loop Leaks When World Is Deleted**
No cancellation mechanism. Deleted worlds' tick loops keep running forever.
*Files:* `crates/gt-server/src/state.rs` (lines 287-289), `crates/gt-server/src/tick.rs`

### Gameplay & Simulation

**M-GP-01: No In-Game Date/Year Display**
Players see "Tick: 0" with no temporal context. No conversion to in-game dates despite era progression design.

**M-GP-02: Difficulty Config Parameters Not Connected to Systems**
`disaster_frequency`, `market_volatility`, `construction_time_multiplier` defined but never read by any system.

**M-GP-03: Event Serialization Uses Debug Format**
Frontend receives `ConstructionCompleted { entity: 42, tick: 5 }` strings instead of structured JSON. Fragile parsing.

**M-GP-04: No Subsidiary Management UI**
Backend supports subsidiaries but no frontend panel exists.

**M-GP-05: Population Growth Rates Likely Too Fast**
Birth/death rates applied every tick without time scaling. 1.2%/tick = 438%/year if tick=day.

**M-GP-06: Missing Major Gameplay Systems**
Spectrum auctions, stock market, antitrust -- all designed but not implemented.

**M-GP-07: Missing Tiered Management**
Flat workforce component instead of individual employees/teams/departments as designed (Dwarf Fortress style scaling).

**M-GP-08: Edge Construction Has No Build Time**
Nodes have construction delays but edges are instant, inconsistent with design.

**M-GP-09: Credit Rating Drops Immediately on First Tick**
Initial BBB may drop to BB on first tick due to salary costs exceeding zero revenue.

---

## Minor Issues

### UI/UX

| ID | Issue | File |
|----|-------|------|
| m-UI-01 | Settings screen checkbox uses default browser accent color, breaks dark theme | `web/src/lib/menu/Settings.svelte` |
| m-UI-02 | Load Game screen is bare -- no illustration, explanation, or import option | `web/src/lib/menu/LoadGame.svelte` |
| m-UI-03 | Credits screen is minimal -- no animations, scrolling, or links | `web/src/lib/menu/Credits.svelte` |
| m-UI-04 | Event feed toggle uses ASCII "v"/"^" instead of proper chevrons | `web/src/lib/game/NotificationFeed.svelte` (line 114) |
| m-UI-05 | Chart components use fixed 360x140px dimensions, cause scaling issues | `web/src/lib/charts/FinanceChart.svelte` (lines 8-9) |
| m-UI-06 | Auctions, M&A, Intel, Achievements panels lack standard close button/header | Multiple panel files |
| m-UI-07 | Multiplayer lobby shows no world preview before connecting | `web/src/lib/menu/WorldBrowser.svelte` |
| m-UI-08 | Pan boundaries too tight (+/-130, +/-110), can't access polar regions | `web/src/lib/game/MapRenderer.ts` (lines 992-993) |
| m-UI-09 | New Game form lacks validation feedback (empty name, AI corp range, seed range) | `web/src/lib/menu/NewGame.svelte` |

### Server & Multiplayer

| ID | Issue | File |
|----|-------|------|
| m-SRV-01 | WebSocket registration path skips username/password validation | `crates/gt-server/src/ws.rs` (lines 719-779) |
| m-SRV-02 | Token refresh creates new UUID on parse failure instead of erroring | `crates/gt-server/src/ws.rs` (line 826) |
| m-SRV-03 | No server-initiated ping/pong heartbeat -- silent disconnects undetected | `crates/gt-server/src/ws.rs` |
| m-SRV-04 | World list endpoint requires no authentication | `crates/gt-server/src/routes.rs` (lines 230-233) |
| m-SRV-05 | REST save upload sends empty data (binary field missing from struct) | `crates/gt-server/src/routes.rs` (line 327) |
| m-SRV-06 | `batch_insert_events` executes one INSERT per event (not actually batched) | `crates/gt-server/src/db.rs` (lines 228-246) |
| m-SRV-07 | No graceful shutdown -- SIGTERM kills without saving worlds or notifying players | `crates/gt-server/src/main.rs` (line 107) |
| m-SRV-08 | SP and MP save systems disconnected -- no import/export between them | `web/src/lib/wasm/SaveManager.ts` |
| m-SRV-09 | Any authenticated user can create unlimited worlds, exhausting resources | `crates/gt-server/src/routes.rs` (lines 248-273) |

### Gameplay & Simulation

| ID | Issue | File |
|----|-------|------|
| m-GP-01 | Disaster risk is identical (0.1) for all regions regardless of terrain | `crates/gt-simulation/src/world.rs` (line 219) |
| m-GP-02 | Coverage system O(N*M) complexity -- needs spatial index for large maps | `crates/gt-simulation/src/systems/coverage.rs` (line 187) |
| m-GP-03 | AI builds infrastructure in neighbor cells rather than city cells | `crates/gt-ai/src/strategy.rs` |
| m-GP-04 | No political events (elections, coups, trade wars) | Not implemented |
| m-GP-05 | No advanced regulations (net neutrality, privacy, environmental) | Only basic tax/strictness/zoning exists |
| m-GP-06 | Revenue history not tracked -- no data for financial dashboard trend chart | `crates/gt-simulation/src/systems/revenue.rs` |
| m-GP-07 | Potential integer overflow in population system if pop exceeds i64::MAX | `crates/gt-simulation/src/systems/population.rs` (line 74) |

---

## Polish Issues

| ID | Issue | Details |
|----|-------|---------|
| P-01 | Font usage inconsistency | Monospace not applied to all financial numbers. `InfoPanel.svelte` uses hardcoded font-family instead of CSS variable |
| P-02 | Region colors are oversaturated | `POLITICAL_COLORS` uses bright primaries. Vic3/Risk use muted pastels |
| P-03 | No city labels or infrastructure icons at default zoom | Labels have `minZoom: 1.5`, population `minZoom: 2.5`, overwhelmed by polygon rendering |
| P-04 | Terrain overlay uses square tiles instead of hex grid | `PlaneGeometry` creates screen-door pattern, design calls for hex parcels |
| P-05 | No mini-map or navigation aid | Essential for multi-layer zoom grand strategy game |
| P-06 | No keyboard shortcut indicators on in-game buttons | Shortcuts listed in Settings but not shown on actual buttons |
| P-07 | Inconsistent panel header patterns | Dashboard/Infra use `.panel-header`, Auctions/M&A/Intel/Achievements use `<h2>` |
| P-08 | PerfMonitor component exists but not wired up | `PerfMonitor.svelte` exists, F3 shortcut listed, but not in `GameView.svelte` |
| P-09 | No empty state illustrations | "No active contracts", "No saved games" etc. are plain text with no CTAs |
| P-10 | Tick loop busy-waits when paused | Wakes every 1000ms, acquires mutex, checks speed, continues. Should use `Notify` |
| P-11 | Duplicate broadcast subscriber on rejoin | Repeated join/leave/join leaks forwarder tasks |
| P-12 | Database connection pool uses defaults | `PgPool::connect` without explicit pool size, timeouts, or tuning |
| P-13 | Service worker may cache stale WASM modules | Cache-first strategy without content hashing |
| P-14 | No schema validation on game commands via WebSocket | `TakeLoan` with negative amount, `HireEmployee` with empty role accepted |
| P-15 | Covert ops and lobbying RNG has poor distribution quality | Multiplicative hash produces clustered values |

---

## Verified Correct

These systems were audited and found to be properly implemented:

### Simulation Engine
- All 15 core ECS systems have real logic, not stubs
- Deterministic sort-before-process pattern correctly used throughout
- Event queue properly implemented with push/drain lifecycle
- NetworkGraph Dijkstra routing with dirty-node invalidation is correct
- Traffic-based revenue model with OD-matrix routing is properly implemented
- Disaster cascading failures correctly mark network dirty for rerouting
- Insolvency detection with bailout/bankruptcy path is complete
- Achievement and victory condition tracking is implemented
- Coverage system backhaul validation realistically prevents disconnected nodes from providing coverage

### AI System
- 4 archetypes with dynamic strategy selection and execution is sophisticated and complete
- AI proxy on disconnect correctly activates DefensiveConsolidator, notifies players, handles reconnection

### Security (Partial)
- Password hashing uses Argon2 (current best practice)
- JWT implementation is sound (HS256, proper claims, expiration validation)
- All SQL queries use parameterized statements (no SQL injection)
- Anti-cheat command validation verifies corp ownership
- Rate limiting exists for commands (10/sec) and chat (5/10sec)

### Frontend Architecture
- Dark theme CSS variable system is well-structured with semantic naming
- Accessibility infrastructure present (colorblind modes, reduced motion, ARIA attributes)
- Tutorial system complete (10 steps, keyboard nav, progress bar, skip, localStorage persistence)
- i18n architecture in place with `$tr()` calls
- Lazy panel loading with dynamic imports
- D3.js chart integration is clean
- WebSocket reconnection uses exponential backoff (up to 30s, max 10 retries)

### Protocol
- MessagePack binary + JSON debug fallback correctly implemented
- UUID serde module ensures roundtrip as strings

### Deployment
- Dockerfile uses multi-stage builds
- Docker-compose correctly configures PostgreSQL with health check
- Vercel build pipeline chains WASM compilation with frontend bundling

### Code Quality
- Zero instances of TODO, FIXME, HACK, STUB, or unimplemented markers found (complies with project rules)

---

## Designed vs Implemented

Based on design documents in `Docs/` compared to actual implementation:

| Feature | Design Status | Implementation |
|---------|--------------|----------------|
| 15 ECS tick systems | Designed | **Implemented** (all have real logic) |
| Procedural world generation | Designed | **Implemented** (but rendering broken) |
| Real Earth mode | Designed | **Implemented** (GeoJSON + OSM) |
| AI corporations (4 archetypes) | Designed | **Implemented** |
| WebSocket multiplayer | Designed | **Partially working** (connection works, state sync broken) |
| MessagePack protocol | Designed | **Implemented** |
| JWT authentication | Designed | **Implemented** |
| Guest login | Designed | **Implemented** |
| AI proxy on disconnect | Designed | **Implemented** |
| Cloud saves | Designed | **Partially implemented** (REST upload broken) |
| Chat system | Designed | **Implemented** (no persistence, no moderation) |
| Admin panel | Designed | **Implemented** |
| Era progression | Designed | **NOT enforced** (all tech available always) |
| Tiered management | Designed | **NOT implemented** (flat workforce) |
| Spectrum auctions | Designed | **NOT implemented** |
| Stock market | Designed | **NOT implemented** |
| Antitrust system | Designed | **NOT implemented** |
| Political events | Designed | **NOT implemented** |
| Advanced regulations | Designed | **NOT implemented** (basic only) |
| Subsidiary management UI | Designed | **NOT implemented** (backend only) |
| Alliance/cooperative ownership | Designed | **Commands defined** (not integrated) |
| Full state sync on join | Designed | **BROKEN** (snapshot returns only config) |
| Client-server tick sync | Designed | **BROKEN** (client runs local sim) |
| 250 concurrent players | Designed | **Not tested** (8-player default) |
| Graceful shutdown | Designed | **NOT implemented** |
| Performance monitoring | Designed | **Component exists** (not wired up) |
| Mini-map | Designed | **NOT implemented** |
| Hex-based terrain rendering | Designed | **Square tiles used instead** |

---

## Recommended Priority Order

### Phase 1: Security (Immediate)
1. Rotate all secrets, remove `.env` from git history (C-01)
2. Restrict CORS to actual frontend origins (C-04)
3. Fix admin API authentication (C-05)
4. Add WebSocket connection auth timeout + per-IP limits (C-06)
5. Add chat message length limit (C-07)

### Phase 2: Core Visual Experience
6. Fix map polygon rendering -- proper triangulation for concave polygons (C-02)
7. Increase overlay opacity 3-5x, add legends (C-03)
8. Fix research panel rendering (C-12)
9. Redesign HUD -- split into rows or add overflow menu (C-10)

### Phase 3: Multiplayer Fundamentals
10. Fix snapshot to include full game state (M-SRV-02)
11. Disable local sim tick in multiplayer mode (M-SRV-03)
12. Fix deadlock risk in tick loop (M-SRV-01)
13. Handle broadcast `Lagged` error gracefully (M-SRV-04)
14. Set `min_machines_running = 1` on Fly.io (M-SRV-08)
15. Add graceful shutdown (m-SRV-07)

### Phase 4: Gameplay Balance
16. Give player starting infrastructure or tutorial-guided first build (C-08)
17. Enforce era restrictions on buildable node types (C-09)
18. Replace `is_multiple_of` with stable Rust alternative (C-11)
19. Add in-game date display (M-GP-01)
20. Connect difficulty config to systems (M-GP-02)

### Phase 5: Feature Completion & Polish
21. Implement missing major systems (spectrum auctions, stock market, tiered management)
22. Add confirmation dialogs for destructive actions
23. Improve panel consistency and empty states
24. Add mini-map
25. Performance optimization (coverage spatial index, batch DB inserts)
