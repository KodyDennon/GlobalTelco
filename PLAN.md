# GlobalTelco Full Audit Fix Implementation Plan

## Context

The AUDIT.md identified 12 Critical, 26 Major, 25 Minor, and 15 Polish issues across UI/UX, gameplay/simulation, and server/multiplayer. This plan addresses ALL issues found plus implements missing designed features (core subset). The audit was generated from 36 screenshots of the live deployed game and deep codebase analysis by 3 specialized agents.

Key user decisions:
- Map: Cell-based fill + earcut polygon borders
- HUD: Two-row layout
- Player start: Tutorial-guided first build
- Features: Core subset (spectrum auctions, era restrictions, tiered management, subsidiary UI)
- MP sync: Pure thin client (no WASM tick in MP)
- Overlays: Contour/isoline style heatmaps
- Era UX: Research tree gates (all visible, must research in order)
- Mini-map: Bottom-right corner
- Panels: Floating modal style
- Management: Team-based (construction crew, maintenance crew, sales team, etc.)
- Auctions: Real-time competitive bidding

---

## Phase 1: Security Hardening

### 1.1 Remove secrets from repo
- Add `.env` to `.gitignore`
- Create `.env.example` with placeholder values
- **Files:** `.gitignore`, `.env.example`

### 1.2 Fix CORS
- Environment-based CORS: restrict to `https://global-telco.vercel.app` in production, `Any` only in debug
- **File:** `crates/gt-server/src/main.rs` (lines 89-93)

### 1.3 Fix admin authentication
- Remove hardcoded default key, fail if `ADMIN_KEY` not set in production
- Use `subtle::ConstantTimeEq` for timing-safe comparison
- Add `Cargo.toml` dep on `subtle` crate
- **File:** `crates/gt-server/src/routes.rs` (lines 484-501)

### 1.4 WebSocket auth timeout + per-IP limits
- Add 10-second auth timeout: if no `Auth` message, close socket
- Track connections per IP with `Arc<RwLock<HashMap<IpAddr, usize>>>`, limit to 10 per IP
- **File:** `crates/gt-server/src/ws.rs`, `crates/gt-server/src/routes.rs`

### 1.5 Chat message validation
- Add `if message.len() > 500` rejection server-side
- Filter control characters
- **File:** `crates/gt-server/src/ws.rs` (lines 479-495)

### 1.6 Cloud save size limit
- Add `if save_data.len() > 50_000_000` rejection (50MB per CLAUDE.md spec)
- **File:** `crates/gt-server/src/ws.rs` (lines 497-542)

### 1.7 Command parameter validation
- Validate `TakeLoan` amount > 0, `HireEmployee` role non-empty, `ProposeContract` terms length < 10KB
- **File:** `crates/gt-server/src/ws.rs` (command handler section)

---

## Phase 2: Map Rendering Overhaul

### 2.1 Cell-based region fill
- Add `earcut` npm dependency to `web/package.json`
- Replace `buildProcgenPolygons()` with cell-based rendering:
  - Each grid cell assigned to its region
  - Render cells as small colored hexagons/circles grouped by region
  - Use muted pastel `POLITICAL_COLORS` (replace bright primaries)
- **File:** `web/src/lib/game/MapRenderer.ts` (lines 309-336)

### 2.2 Earcut polygon borders
- Use earcut.js for region boundary outlines only (not fill)
- Render as `THREE.LineLoop` or `THREE.Line` with region border color
- Remove triple-render wrap-around hack (handle Mercator wrapping properly)
- **File:** `web/src/lib/game/MapRenderer.ts`

### 2.3 Contour/isoline overlay system
- Replace current overlay renderers with isoline contour approach:
  - Generate a value grid from simulation data (demand, coverage, risk, etc.)
  - Use marching squares or D3 contour to generate isoline paths
  - Render smooth gradient fills between contour levels
  - Add overlay legend component showing color scale + values
- Increase base opacity to 0.4-0.6 range (currently 0.08-0.15)
- Each overlay gets a unique color gradient (blue-purple for demand, green-yellow for coverage, yellow-red for disaster risk, etc.)
- **Files:** `web/src/lib/game/MapRenderer.ts` (lines 1103-1427), new `web/src/lib/game/OverlayLegend.svelte`

### 2.4 Fix zoom and pan limits
- Change min zoom from 1.3 to 0.5 (allow full world view)
- Widen pan bounds from +/-130,110 to +/-180,170
- **File:** `web/src/lib/game/MapRenderer.ts` (lines 992, 1008)

### 2.5 City labels and infrastructure icons
- Lower `minZoom` thresholds: city names at 0.8, population at 1.5
- Add infrastructure node icons that scale with zoom
- Ensure labels render on top of cell fill (z-index ordering)
- **File:** `web/src/lib/game/MapRenderer.ts` (label rendering section)

### 2.6 Mini-map
- New component: `web/src/lib/game/MiniMap.svelte`
- Bottom-right corner, 200x150px, shows full world with colored regions
- Viewport indicator rectangle, click-to-navigate
- Positioned above events feed
- **Files:** new `web/src/lib/game/MiniMap.svelte`, `web/src/lib/game/GameView.svelte`

### 2.7 Terrain overlay: hex grid instead of squares
- Replace `PlaneGeometry` tiles with `CircleGeometry(6)` hexagons for terrain overlay
- Match the hex-based parcel system from the design
- **File:** `web/src/lib/game/MapRenderer.ts` (lines 1137-1190)

---

## Phase 3: HUD & Panel Overhaul

### 3.1 Two-row HUD
- **Row 1 (top):** Corp name | Cash | Profit/tick | Divider | Speed controls | Divider | Tick/Date | Credit rating | Infra count | MP status
- **Row 2 (bottom):** Build buttons | Divider | Panel buttons WITH text labels | Divider | Overlay buttons WITH short labels
- Increase HUD height from 48px to 80px (40px per row)
- Panel buttons: show icon + short label (e.g., icon + "Finance", "Infra", "Research")
- Overlay buttons: replace single letters with short labels ("Terrain", "Own", "Demand", "Cover", "Risk", "Traffic", "Flow")
- **File:** `web/src/lib/game/HUD.svelte` (complete rewrite of template + styles)

### 3.2 Floating modal panels
- Create reusable `FloatingPanel.svelte` wrapper component:
  - Draggable title bar
  - Proper close button (icon, 44px touch target, visible background)
  - Resize handle
  - Remembers position per panel type
  - Dark card styling with panel border
- Convert ALL panels to use `FloatingPanel` wrapper
- **Files:** new `web/src/lib/ui/FloatingPanel.svelte`, all panel files in `web/src/lib/panels/`

### 3.3 Standardize panel headers
- All panels get consistent header: title + close button + optional tabs
- Remove inconsistent `<h2>` headers from Auctions, M&A, Intel, Achievements panels
- **Files:** `AuctionPanel.svelte`, `MergerPanel.svelte`, `IntelPanel.svelte`, `AchievementPanel.svelte`

### 3.4 Confirmation dialog component
- New `ConfirmDialog.svelte`: modal with message, confirm/cancel buttons
- Wire up to all destructive actions: decommission node, decommission edge, take loan, fire team
- **Files:** new `web/src/lib/ui/ConfirmDialog.svelte`, `InfoPanel.svelte`, `InfraPanel.svelte`

### 3.5 Loading screen with progress
- Replace bare `<p>Loading</p>` with:
  - Animated spinner/globe
  - Step indicators: "Generating terrain...", "Placing cities...", "Creating corporations..."
  - Progress bar
- **File:** `web/src/lib/game/GameView.svelte` (lines 73-77)

### 3.6 Empty state improvements
- Add illustrations/icons and CTAs for all empty states:
  - "No infrastructure" -> show build tutorial prompt
  - "No contracts" -> "Explore regions to find contract opportunities"
  - "No auctions" -> "Spectrum auctions begin periodically"
  - "No saved games" -> "Start a new game to create your first save"
- **Files:** all panel files with empty states

### 3.7 Chat/Events positioning fix
- Chat: bottom-left corner
- Events feed: bottom-right corner (above mini-map)
- No overlap possible
- **Files:** `web/src/lib/game/Chat.svelte` (line 69), `web/src/lib/game/NotificationFeed.svelte` (line 135)

### 3.8 Wire up PerfMonitor
- Import and render `PerfMonitor.svelte` in `GameView.svelte` when `showPerfMonitor` store is true
- F3 shortcut already works in GameLoop.ts
- **File:** `web/src/lib/game/GameView.svelte`

### 3.9 Fix Research panel rendering
- Debug why Research panel doesn't appear (likely data issue or conditional rendering bug)
- Ensure research tree displays with era-gated items
- **File:** `web/src/lib/panels/ResearchPanel.svelte`

---

## Phase 4: Multiplayer Fixes

### 4.1 Full state snapshot
- Implement `serialize_full_state()` on `GameWorld` that returns all entities, corporations, financials, infrastructure, regions, cities
- Use existing `save_game_binary()` or create a JSON summary suitable for client hydration
- **File:** `crates/gt-server/src/ws.rs` (lines 445-466), `crates/gt-simulation/src/world.rs`

### 4.2 Pure thin client mode
- In `GameLoop.ts`, check `$isMultiplayer` store
- If multiplayer: do NOT call `bridge.tick()`, only call `updateStores()` from server messages
- Create `applyServerState(delta)` function in bridge that updates stores from server tick data
- **Files:** `web/src/lib/game/GameLoop.ts`, `web/src/lib/multiplayer/WebSocketClient.ts`

### 4.3 Fix deadlock (ABBA lock pattern)
- Establish strict lock ordering: always `world.world` before `world.players`
- Refactor reconnection path (ws.rs lines 287-306) to acquire locks in correct order
- **File:** `crates/gt-server/src/ws.rs` (lines 287-313)

### 4.4 Handle broadcast Lagged error
- In forwarder loop, match on `Err(broadcast::error::RecvError::Lagged(n))` explicitly
- Log warning, continue receiving (don't break)
- If lag > threshold, request fresh snapshot for that player
- **File:** `crates/gt-server/src/ws.rs` (lines 326-333)

### 4.5 Fly.io config fix
- Set `min_machines_running = 1`
- Set `auto_stop_machines = 'off'`
- **File:** `fly.toml`

### 4.6 Graceful shutdown
- Add `tokio::signal::ctrl_c()` shutdown handler
- On shutdown: save all world snapshots to DB, notify connected players, close WebSockets cleanly
- **File:** `crates/gt-server/src/main.rs` (line 107)

### 4.7 World tick loop cancellation
- Add `CancellationToken` to `WorldInstance`
- Check token in tick loop, break if cancelled
- On world deletion, trigger cancellation
- **Files:** `crates/gt-server/src/state.rs`, `crates/gt-server/src/tick.rs`

### 4.8 Memory management
- Cap audit log at 10,000 entries with ring buffer
- Add LRU eviction for account cache (max 1000 entries, evict guest accounts first)
- **File:** `crates/gt-server/src/state.rs`

### 4.9 Server-initiated heartbeat
- Send WebSocket ping every 30 seconds
- If no pong within 10 seconds, close connection and activate AI proxy
- **File:** `crates/gt-server/src/ws.rs`

### 4.10 Batch DB inserts
- Rewrite `batch_insert_events` to use single INSERT with multiple VALUES
- **File:** `crates/gt-server/src/db.rs` (lines 228-246)

### 4.11 DB connection pool config
- Use `PgPoolOptions::new().max_connections(10).min_connections(2).acquire_timeout(Duration::from_secs(5))`
- **File:** `crates/gt-server/src/db.rs` (lines 20-23)

### 4.12 WebSocket registration validation
- Share validation logic between REST and WS registration paths
- Extract to shared `validate_registration()` function
- **File:** `crates/gt-server/src/ws.rs` (lines 719-779), `crates/gt-server/src/routes.rs`

### 4.13 Fix REST save upload
- Add binary save data field to `SaveUploadRequest` struct (base64 encoded)
- **File:** `crates/gt-server/src/routes.rs` (line 327)

### 4.14 World creation limits
- Max 3 worlds per authenticated user, 0 for guests
- **File:** `crates/gt-server/src/routes.rs` (lines 248-273)

---

## Phase 5: Gameplay & Simulation Fixes

### 5.1 Replace `is_multiple_of` with stable alternative
- Replace all 9 instances of `tick.is_multiple_of(N)` with `tick % N == 0`
- **Files:** 8 files in `crates/gt-simulation/src/systems/` + `crates/gt-world/src/cities.rs`

### 5.2 Era restrictions via research tree gates
- Add `required_era` and `required_research` fields to node type definitions
- In `cmd_build_node` WASM bridge and server command handler: validate node type is unlocked
- In Research panel: show all tech, grayed out with "Requires: [prerequisite tech]" for locked ones
- Define research tree: each era unlocks new research topics, each topic unlocks specific node/edge types
- **Files:** `crates/gt-common/src/types.rs` (node type definitions), `crates/gt-wasm/src/lib.rs`, `crates/gt-simulation/src/systems/research.rs`

### 5.3 Tutorial-guided first build
- Enhance existing tutorial system (10 steps) with interactive steps:
  - Step 1-2: Welcome + overview (existing)
  - Step 3: "Click a city to see its details" (highlight a nearby city)
  - Step 4: "Click + Node to build your first tower" (highlight build button)
  - Step 5: "Select a cell near the city" (highlight valid cells)
  - Step 6: "Choose node type" (show available types for current era)
  - Step 7: "Build an edge to connect to the city" (guide edge building)
  - Step 8: "You're now earning revenue!" (show financial impact)
  - Step 9-10: Overview of panels and overlays
- **Files:** `web/src/lib/game/Tutorial.svelte`, `web/src/stores/tutorialState.ts`

### 5.4 In-game date display
- Add tick-to-date conversion: define ticks-per-day based on era (e.g., 1 tick = 1 week in Internet era)
- Calculate in-game year/month from starting era date + tick count
- Display in HUD as "Jan 1995" style date alongside tick number
- **Files:** `crates/gt-common/src/types.rs` (date conversion), `web/src/lib/game/HUD.svelte`

### 5.5 Connect difficulty config to systems
- Wire `disaster_frequency` to disaster system probability checks
- Wire `market_volatility` to market system price fluctuations
- Wire `construction_time_multiplier` to construction system build times
- **Files:** `crates/gt-simulation/src/systems/disaster.rs`, `market.rs`, `construction.rs`

### 5.6 Fix event serialization
- Replace `format!("{:?}", event)` with proper JSON serialization via serde
- Define `EventData` enum with structured variants, serialize to JSON
- Frontend parses structured events instead of Debug strings
- **Files:** `crates/gt-simulation/src/events.rs`, `web/src/lib/game/NotificationFeed.svelte`

### 5.7 Population growth rate fix
- Scale birth/death rates by ticks-per-year from era config
- Ensure growth is ~1-2% per in-game year, not per tick
- **File:** `crates/gt-simulation/src/systems/population.rs`

### 5.8 Edge construction time
- Add `construction_ticks` field to edges (like nodes have)
- Edges start as "under construction" and complete after N ticks
- **File:** `crates/gt-simulation/src/systems/construction.rs`

### 5.9 Credit rating grace period
- Don't recalculate credit rating for first 10 ticks (allow player to build before being judged)
- Or: base initial rating on starting capital rather than revenue ratio
- **File:** `crates/gt-simulation/src/systems/finance.rs`

### 5.10 Disaster risk variation
- Calculate per-region disaster risk based on terrain composition:
  - Coastal: +0.15 (floods, storms)
  - Mountainous: +0.1 (landslides, earthquakes)
  - Desert: +0.05 (sandstorms)
  - Urban: +0.05 (infrastructure failures)
  - Tundra/Frozen: +0.1 (ice storms)
- **File:** `crates/gt-simulation/src/world.rs` (line 219)

### 5.11 Revenue history tracking
- Add `revenue_history: Vec<(u64, Money)>` to corporation or financial component
- Record revenue/cost each tick (or every N ticks) for chart data
- **Files:** `crates/gt-simulation/src/systems/revenue.rs`, `crates/gt-common/src/types.rs`

### 5.12 Coverage spatial index
- Add grid-based spatial hash for cell lookups
- Replace O(N*M) cell scan with O(N*k) where k = cells in range
- **File:** `crates/gt-simulation/src/systems/coverage.rs`

---

## Phase 6: New Gameplay Systems (Core Subset)

### 6.1 Team-based workforce management
- Replace flat `Workforce { employee_count, skill_level, morale, salary_per_tick }` with:
  ```
  Team { team_type, size, skill_level, morale, salary_per_tick, assignment }
  TeamType: Construction, Maintenance, Sales, Engineering, Legal
  ```
- Corps hire/fire teams, not individuals
- Construction teams speed up building, Maintenance teams reduce failure rate, Sales teams increase contract wins
- New Workforce panel shows team list with hire/fire/reassign controls
- **Files:** `crates/gt-common/src/types.rs`, `crates/gt-simulation/src/systems/cost.rs`, `crates/gt-economy/src/lib.rs`, `web/src/lib/panels/WorkforcePanel.svelte`

### 6.2 Spectrum auction system
- Real-time competitive auctions:
  - Server creates auction events periodically (every ~50 ticks)
  - Each auction: spectrum band + region + starting price + duration (10 ticks)
  - Players/AI submit increasing bids during the window
  - Winner gets exclusive spectrum license for that band+region
  - Spectrum licenses required for wireless node types
- New components in simulation: `SpectrumLicense`, `AuctionEvent`, `Bid`
- New ECS system: `auction_system` (runs in tick order after market)
- AI bidding strategy based on archetype (Aggressive bids high, Budget bids low)
- **Files:** `crates/gt-common/src/types.rs`, new `crates/gt-simulation/src/systems/auction.rs`, `crates/gt-ai/src/strategy.rs`, `web/src/lib/panels/AuctionPanel.svelte`

### 6.3 Subsidiary management UI
- Frontend panel to view/manage subsidiaries (backend already supports them)
- Create subsidiary, transfer assets, set subsidiary budget, view subsidiary P&L
- **Files:** new `web/src/lib/panels/SubsidiaryPanel.svelte`, `web/src/lib/game/HUD.svelte`

### 6.4 Research tree with era gates
- Define full tech tree with prerequisites:
  - Telegraph era: Telegraph poles, Manual switching
  - Telephone era: Copper lines, Telephone exchange, Operator switching
  - Early Digital: Coaxial cable, Digital switching, Early fiber
  - Internet era: Fiber optic, DSL, Dial-up, Web hosting
  - Modern: 4G/5G towers, FTTH, Data centers, CDN
  - Near Future: 6G, Satellite mesh, Quantum networking
- Each tech has: cost, research_ticks, era_requirement, prerequisite_techs, unlocks (node types, edge types, abilities)
- Research panel shows tree visualization (D3.js force-directed or hierarchical)
- **Files:** `crates/gt-common/src/types.rs`, `crates/gt-simulation/src/systems/research.rs`, `web/src/lib/panels/ResearchPanel.svelte`

---

## Phase 7: UI/UX Polish

### 7.1 i18n fixes
- Replace all hardcoded English strings with `$tr()` calls
- **Files:** `DashboardPanel.svelte` (lines 136-168), any other files with hardcoded strings

### 7.2 Font consistency
- Apply `.mono` class / `var(--font-mono)` to all financial numbers across all panels
- Remove hardcoded `font-family` in `InfoPanel.svelte`
- **Files:** all panel files

### 7.3 Settings screen polish
- Style checkbox/slider controls to match dark theme (custom checkbox component)
- **File:** `web/src/lib/menu/Settings.svelte`

### 7.4 Load Game screen
- Add illustration, explanation of save locations, import button
- **File:** `web/src/lib/menu/LoadGame.svelte`

### 7.5 Event feed chevrons
- Replace ASCII "v"/"^" with proper SVG chevron icons
- **File:** `web/src/lib/game/NotificationFeed.svelte`

### 7.6 Chart responsive sizing
- Replace fixed 360x140 with responsive sizing using `ResizeObserver`
- **File:** `web/src/lib/charts/FinanceChart.svelte`

### 7.7 New Game form validation
- Add validation feedback: corp name min 2 chars, AI corps 0-8 range indicator, seed format hint
- **File:** `web/src/lib/menu/NewGame.svelte`

### 7.8 Keyboard shortcut hints
- Show shortcut keys on buttons: speed "1x (1)", build "Node (B)", etc.
- **File:** `web/src/lib/game/HUD.svelte`, `web/src/lib/game/SpeedControls.svelte`

### 7.9 Service worker WASM cache fix
- Use content-hashed filenames for WASM or stale-while-revalidate strategy
- **File:** `web/src/service-worker.ts`

### 7.10 Tick loop efficiency when paused
- Use `tokio::sync::Notify` to wake tick loop only on speed change
- **File:** `crates/gt-server/src/tick.rs`

### 7.11 Broadcast subscriber leak fix
- Track forwarder task handle, cancel old one before creating new on rejoin
- **File:** `crates/gt-server/src/ws.rs`

### 7.12 Better RNG for covert ops and lobbying
- Replace multiplicative hash with xorshift or SplitMix64 deterministic RNG
- **Files:** `crates/gt-simulation/src/systems/covert_ops.rs`, `lobbying.rs`

---

## Verification Plan

After each phase:

1. **Phase 1 (Security):** `cargo build --features postgres` compiles. Manual test: verify CORS rejects non-allowed origins. Verify admin endpoint rejects without valid key.

2. **Phase 2 (Map):** `wasm-pack build` + `bun run dev`. Visual verification: map shows coherent colored regions with borders. Overlays show visible contour gradients with legends. Zoom all the way out shows full world.

3. **Phase 3 (HUD/Panels):** Visual verification in browser. Two-row HUD readable at 1280px width. All panels open as floating modals with consistent headers/close buttons. Confirm dialog appears on decommission.

4. **Phase 4 (Multiplayer):** Deploy to Fly.io test instance. Connect two browser tabs as different guests. Join same world. Verify both see same game state. Verify late-joiner gets full snapshot. Verify disconnect triggers AI proxy.

5. **Phase 5 (Gameplay):** `cargo test` passes. Start game in each era, verify only era-appropriate tech available. Tutorial guides through first build. Date displays in HUD. Difficulty settings affect gameplay.

6. **Phase 6 (New Systems):** Spectrum auctions appear periodically. Teams can be hired/fired. Research tree shows tech prerequisites. Subsidiary panel displays.

7. **Phase 7 (Polish):** Full visual audit pass. All financial numbers in monospace. No hardcoded English. Charts resize properly. Keyboard shortcuts shown on buttons.

**Full end-to-end test:** Start new game -> tutorial guides first build -> earn revenue -> research tech -> win spectrum auction -> hire teams -> expand to new region -> open each panel and overlay -> save/load game -> join multiplayer -> verify sync.
