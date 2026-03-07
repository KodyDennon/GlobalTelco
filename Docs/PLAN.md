# GlobalTelco Full Audit Fix + Feature Expansion Plan

> **Note:** File paths in this plan reflect the structure at time of writing. Since then, large files have been modularized into subdirectories (e.g., `types.rs` → `types/`, `world.rs` → `world/`, `routes.rs` → `routes/`, `ws.rs` → `ws/`, `db.rs` → `db/`). See `Docs/technical_architecture.md` Section 2c for the current crate structure.

## Context

The AUDIT.md identified 12 Critical, 26 Major, 25 Minor, and 15 Polish issues across UI/UX, gameplay/simulation, and server/multiplayer. This plan addresses ALL audit issues plus implements missing designed features based on extensive design doc review and user decisions.

### Key Decisions (Original)
- Map: deck.gl rendering, free placement (no grid snap), invisible cell grid for backend spatial queries
- HUD: Two-row layout
- Player start: Tutorial-guided first build + 1 starter node
- MP sync: Pure thin client (no WASM tick in MP)
- Overlays: Contour/isoline style heatmaps
- Mini-map: Bottom-right corner
- Panels: Floating modal style
- Management: Team-based (construction crew, maintenance crew, sales team, etc.)
- Auctions: Real-time competitive bidding
- Hosting: Stay on Fly.io + Vercel (no migration to Hetzner/Cloudflare)

### Key Decisions (Design Doc Review — Round 1)
- **Era-specific node types:** Add era-specific types to Rust type system (not just gate existing 8)
- **Alliance system:** Add (creation, maintenance, dissolution, shared voting, joint infrastructure)
- **Government grants:** Add (bid for public contracts, receive grants for underserved areas). Skip SLA-based pricing.
- **Legal system:** Add (lawsuits, sabotage claims, ownership disputes, arbitration)
- **i18n/a11y:** Full implementation (translation files, locale support, colorblind mode, keyboard nav, ARIA, screen reader)
- **Parcel leasing:** Skip
- **Player start:** 1 starter node in starting region (tutorial still guides first expansion)
- **Event formatting:** Frontend formats events from structured JSON data (not server-side descriptions)
- **Tier visualization:** Full hierarchy visualization (larger icons for higher tiers, tier labels, backbone highlighted)
- **Co-ownership panel:** Full panel (view co-owners, vote on upgrades, manage buyout offers)
- **Start paused:** Game starts paused so player can orient. Auto-pause on disasters/critical events.
- **Maintenance scheduling:** Player-controlled per-node maintenance budget and priority tiers
- **Player-controlled pricing:** Set price points per region (price vs market share tradeoff)
- **Production hardening:** Full (save migration, structured logging)
- **Performance profiling:** Basic profiling only (tick timing, memory tracking)
- **Dynamic AI:** Spawning + mergers mid-game when market conditions support it
- **Tech/Patent system:** Robust — research, patent, license, lease. Not just bonuses.
- **Infrastructure icons:** SVG icon sprites (recognizable telecom imagery, readable at all zooms)
- **Testing:** Comprehensive (unit per system, integration cross-crate, frontend via Bun)
- **Interactive charts:** Zoom, hover data points, time range selection
- **AI visibility:** Intel-gated (only see AI activity if you have intel/espionage on them)
- **Zoom LOD:** Keep uniform zoom (no level-of-detail changes)
- **Connectivity feedback:** Full loop (infra → GDP growth → migration → demand increase)
- **MP lobby:** Full overhaul (persistent worlds, filter/search, world creation, leaderboard)
- **Era progression:** Auto-progression through gameplay (Telegraph → ... → Near Future)
- **Insurance panel:** Full management UI (view insured nodes, purchase/cancel)
- **Repair UX:** Damage alerts + dedicated repair panel (choose emergency vs normal repair)

### Key Decisions (Clarification Q&A — Round 2)
- **NodeType enum:** Flat enum with all ~33 variants (single enum, not nested). Existing saves break — migration needed.
- **Research tree:** NOT era-gated. Tech tree organized by era but freely explorable — a player CAN research higher-era tech early. Tech is the economic commodity (patent, license, lease). World era = collective cosmetic milestone when all corps have tech coverage at that level (no gameplay effects).
- **MP fog of war:** Full fog of war for ALL competitors (AI and players). Geography/terrain/cities visible, competitor infrastructure and activity hidden until intel obtained via espionage.
- **Patent enforcement:** Hard block + independent research workaround. 150% cost/time = base access (unlicensed). 200% cost/time = improved version that CAN be patented (competing patent for similar tech).
- **AI sophistication:** Full autonomy — AI uses ALL new systems (patents, alliances, legal, pricing, maintenance, grants, insurance). Each archetype has unique behavior per system.
- **Connectivity-GDP dampening:** All four real-world mechanisms: market saturation (demand ceiling per region), competition splits pie (GDP boost divided among providers), rising costs (land + labor costs scale with GDP), regulatory intervention (anti-monopoly at >60% market share).
- **Cost scaling:** Land + labor costs rise with regional GDP (not maintenance costs).
- **Phase execution:** Strict sequential (1→14). No parallelism.
- **Alliance routing economics:** Free routing between allies, revenue split proportional to hops (nodes/edges each ally contributes to route).
- **Legal filing UX:** Auto-suggest with player confirmation. Notification when grounds exist, player clicks to file.
- **World era effect:** Cosmetic + milestone only. Achievement unlocked. No gameplay changes tied to world era.
- **Panel architecture:** 6 tabbed panel groups: Finance (Dashboard, Pricing, Insurance) | Operations (Infra, Maintenance, Repair, Workforce) | Diplomacy (Alliance, Legal, Intel, Co-ownership) | Research (Tech Tree, Patents) | Market (Contracts, Auctions, Mergers, Grants, Subsidiary) | Info (Region, Advisor, Achievements)
- **Audio:** Full audio expansion — sounds for all major events (auction bell, disaster siren, legal gavel, alliance trumpet, era fanfare, construction chime, bankruptcy alert).
- **Desktop (Tauri):** Maintain compatibility throughout development. Test Tauri build after each phase.
- **Tech tree visualization:** D3 force-directed graph. Techs cluster by category, prerequisite lines pull connected techs together.
- **Build menu UX:** Categorized by network tier (Access, Aggregation, Core, Backbone, Global). Only researched/licensed types shown; locked types grayed with "Requires: [tech]" tooltip.
- **Event management:** Both priority levels (critical banner + sound, important in feed, info collapsed) AND category filters (Construction, Finance, Competition, Diplomacy, Disaster, Achievement).
- **Late-join balance:** No special treatment. Being late is a disadvantage — realistic. Motivates joining worlds early. Late-joiners can license tech from established players.
- **Sandbox mode:** Full sandbox as a game mode in New Game menu. Infinite money, all tech unlocked, adjustable AI (spawn/remove on demand, control behavior). Can test alliances, auctions, legal, everything.
- **Test coverage target:** Happy path + edge cases per system. ~120-150 total tests.

### Codebase State Summary
- **Rust:** 11 crates (added gt-bridge, gt-tauri), 20 ECS systems, 27 component types, 38 commands, 50+ event types — all functional
- **Frontend:** 13 panel types, 8 overlay types, deck.gl map renderer, D3 charts, typed array bridge, ghost entity system
- **Server:** Full Axum HTTP + WebSocket, JWT auth, AI proxy, admin API (with ban/unban), optional PostgreSQL, event-driven delta broadcasts, per-type rate limiting, speed vote system, per-player event filtering, sequence dedup
- **Desktop:** Tauri v2 with 16 commands (4 filesystem + 12 native sim), compiles clean
- **Tests:** 79 passing, 0 warnings across entire codebase
- **Key gaps:** No era enforcement, no patent enforcement gate, Debug format events, no tier visualization, no co-ownership/insurance/repair UI, no dynamic AI spawning, uniform pricing only

### Completed Supplemental Plans
- **INSTANT_MULTIPLAYER_PLAN.md** — ALL 10 PHASES COMPLETE. Sub-200ms multiplayer sync, anti-cheat, typed array bridge, Tauri native bridge. See that document for details.

---

## Phase 1: Security Hardening

### 1.1 Remove secrets from repo
- Add `.env` to `.gitignore`
- Create `.env.example` with placeholder values
- **Files:** `.gitignore`, `.env.example`

### 1.2 Fix CORS
- Environment-based CORS: restrict to `https://globaltelco.online` in production, `Any` only in debug
- **File:** `crates/gt-server/src/main.rs`

### 1.3 Fix admin authentication
- Remove hardcoded default key, fail if `ADMIN_KEY` not set in production
- Use `subtle::ConstantTimeEq` for timing-safe comparison
- Add `Cargo.toml` dep on `subtle` crate
- **File:** `crates/gt-server/src/routes.rs`

### 1.4 WebSocket auth timeout + per-IP limits
- Add 10-second auth timeout: if no `Auth` message, close socket
- Track connections per IP with `Arc<RwLock<HashMap<IpAddr, usize>>>`, limit to 10 per IP
- **Files:** `crates/gt-server/src/ws.rs`, `crates/gt-server/src/routes.rs`

### 1.5 Chat message validation
- Add `if message.len() > 500` rejection server-side
- Filter control characters
- **File:** `crates/gt-server/src/ws.rs`

### 1.6 Cloud save size limit
- Add `if save_data.len() > 50_000_000` rejection (50MB per CLAUDE.md spec)
- **File:** `crates/gt-server/src/ws.rs`

### 1.7 Command parameter validation
- Validate `TakeLoan` amount > 0, `HireEmployee` role non-empty, `ProposeContract` terms length < 10KB
- **File:** `crates/gt-server/src/ws.rs`

---

## Phase 2: Map Rendering Overhaul

### 2.1 Cell-based region fill [DONE — deck.gl rewrite]
- **Replaced Three.js with deck.gl** — ScatterplotLayer renders cells as base map
- Region fill via cell coloring (land layer)
- **File:** `web/src/lib/game/MapRenderer.ts` (rewritten to ~300 lines using deck.gl)

### 2.2 Region borders [DONE — deck.gl rewrite]
- PathLayer renders region boundary outlines
- No earcut needed — deck.gl handles polygon rendering natively
- **File:** `web/src/lib/game/MapRenderer.ts`

### 2.3 Contour/isoline overlay system
- Replace current overlay renderers with isoline contour approach:
  - Generate a value grid from simulation data (demand, coverage, risk, etc.)
  - Use marching squares or D3 contour to generate isoline paths
  - Render smooth gradient fills between contour levels
  - Add overlay legend component showing color scale + values
- Increase base opacity to 0.4-0.6 range (currently 0.08-0.15)
- Each overlay gets a unique color gradient (blue-purple for demand, green-yellow for coverage, yellow-red for disaster risk, etc.)
- **Files:** `web/src/lib/game/MapRenderer.ts`, new `web/src/lib/game/OverlayLegend.svelte`

### 2.4 Fix zoom and pan limits
- Change min zoom from 1.3 to 0.5 (allow full world view)
- Widen pan bounds from +/-130,110 to +/-180,170
- **File:** `web/src/lib/game/MapRenderer.ts`

### 2.5 SVG icon sprites for infrastructure
- Replace geometric shapes (triangle, pentagon, etc.) with SVG/sprite-based icons:
  - Cell tower: antenna/mast icon
  - Data center: server rack icon
  - Central office: building icon
  - Exchange point: hub/interconnect icon
  - Backbone router: router icon
  - Satellite ground station: satellite dish icon
  - Submarine landing: undersea cable icon
  - Wireless relay: small antenna icon
  - Era-specific variants (telegraph pole, telephone exchange, etc. — see Phase 6)
- Load SVGs as canvas textures, render as deck.gl icon sprites
- Icons scale with zoom, remain readable at all levels
- **Build menu categorized by network tier:**
  - Access (tier 1): towers, terminals, relays, FTTH
  - Aggregation (tier 2): offices, exchanges, hubs, CDN
  - Core (tier 3): data centers, switches, hosting
  - Backbone (tier 4): routers, long-distance relays
  - Global (tier 5): satellite ground, submarine landing
  - Only show node types the player has researched or licensed. Locked types shown grayed with "Requires: [tech name]" tooltip.
- **Files:** `web/src/lib/game/MapRenderer.ts`, `web/src/lib/game/BuildMenu.svelte` (tier-categorized layout), new SVG assets in `web/static/icons/infrastructure/`

### 2.6 Tier-based visual hierarchy
- Node icon SIZE scales by network tier:
  - Access (tier 1): 1x base size
  - Aggregation (tier 2): 1.3x
  - Core (tier 3): 1.6x
  - Backbone (tier 4): 2x
  - Global (tier 5): 2.5x
- Tier labels visible at medium+ zoom (e.g., small "T1", "T4" badge)
- Backbone routes (tier 4-5 edges) rendered as thicker, brighter lines
- Global edges (submarine, satellite) get distinctive dash pattern or glow
- Edge thickness scales: Local fiber thin → Submarine cable thick
- **Files:** `web/src/lib/game/MapRenderer.ts` (node rendering, edge rendering)

### 2.7 City labels and zoom thresholds
- Lower `minZoom` thresholds: city names at 0.8, population at 1.5
- Ensure labels render on top of cell fill (z-index ordering)
- **File:** `web/src/lib/game/MapRenderer.ts`

### 2.8 Mini-map
- New component: `web/src/lib/game/MiniMap.svelte`
- Bottom-right corner, 200x150px, shows full world with colored regions
- Viewport indicator rectangle, click-to-navigate
- Positioned above events feed
- **Files:** new `web/src/lib/game/MiniMap.svelte`, `web/src/lib/game/GameView.svelte`

### 2.9 Terrain overlay [DONE — deck.gl rewrite]
- Terrain overlay uses ScatterplotLayer with terrain-colored cells
- No hex geometry needed — deck.gl renders circles per cell
- **File:** `web/src/lib/game/MapRenderer.ts`

### 2.10 Free placement system [DONE]
- **Removed parcel-based build flow** — nodes placed at exact (lon, lat) clicked coordinates
- `BuildNode` command changed from `{ node_type, parcel }` to `{ node_type, lon, lat }`
- Invisible grid cells remain as backend spatial index (terrain, coverage, demand, AI)
- AI nodes placed with random jitter around cell centers (not grid-snapped)
- Frontend: `buildMenuParcel` → `buildMenuLocation`, `parcel-clicked` → `map-clicked`
- **Files:** `commands.rs`, `world.rs`, `gt-wasm/lib.rs`, `bridge.ts`, `BuildMenu.svelte`, `MapView.svelte`, `MapRenderer.ts`, `uiState.ts`, `HUD.svelte`, `GameLoop.ts`

---

## Phase 3: HUD & Panel System Overhaul

### 3.1 Two-row HUD
- **Row 1 (top):** Corp name | Cash | Profit/tick | Divider | Speed controls | Divider | Tick/Date (in-game date) | Credit rating | Infra count | MP status
- **Row 2 (bottom):** Build buttons | Divider | Panel buttons WITH text labels | Divider | Overlay buttons WITH short labels
- Increase HUD height from 48px to 80px (40px per row)
- Panel buttons: show icon + short label (e.g., icon + "Finance", "Infra", "Research")
- Overlay buttons: replace single letters with short labels ("Terrain", "Own", "Demand", "Cover", "Risk", "Traffic", "Flow")
- **File:** `web/src/lib/game/HUD.svelte` (complete rewrite of template + styles)

### 3.2 Floating modal panels with tabbed groups
- Create reusable `FloatingPanel.svelte` wrapper component:
  - Draggable title bar
  - Proper close button (icon, 44px touch target, visible background)
  - Resize handle
  - Remembers position per panel type
  - Dark card styling with panel border
  - **Tabbed sub-navigation** within panel groups
- Convert ALL panels to use `FloatingPanel` wrapper
- **6 panel groups** (each group = one floating panel with tabs):
  1. **Finance:** Dashboard | Pricing | Insurance
  2. **Operations:** Infrastructure | Maintenance | Repair | Workforce
  3. **Diplomacy:** Alliance | Legal | Intel | Co-ownership
  4. **Research:** Tech Tree | Patents
  5. **Market:** Contracts | Auctions | Mergers | Grants | Subsidiary
  6. **Info:** Region | Advisor | Achievements
- HUD shows 6 group buttons instead of 13+ individual panel buttons
- Clicking a group button opens the floating panel to the default (first) tab
- Tabs within the panel switch content without closing/reopening
- **Files:** new `web/src/lib/ui/FloatingPanel.svelte`, new `web/src/lib/ui/TabbedPanelGroup.svelte`, all panel files in `web/src/lib/panels/`, `web/src/stores/uiState.ts` (panel group + active tab state)

### 3.3 Standardize panel headers
- All panels get consistent header: title + close button + optional tabs
- Remove inconsistent `<h2>` headers from Auctions, M&A, Intel, Achievements panels
- **Files:** `AuctionPanel.svelte`, `MergerPanel.svelte`, `IntelPanel.svelte`, `AchievementPanel.svelte`

### 3.4 Confirmation dialog component
- New `ConfirmDialog.svelte`: modal with message, confirm/cancel buttons
- Wire up to all destructive actions: decommission node, decommission edge, take loan, fire team
- **Files:** new `web/src/lib/ui/ConfirmDialog.svelte`, `InfoPanel.svelte`, `InfraPanel.svelte`

### 3.5 Loading screen with progress
- Replace bare `<p>Loading</p>` with animated spinner/globe, step indicators, progress bar
- Steps: "Generating terrain...", "Placing cities...", "Creating corporations...", "Building AI infrastructure..."
- **File:** `web/src/lib/game/GameView.svelte`

### 3.6 Empty state improvements
- Add illustrations/icons and CTAs for all empty states:
  - "No infrastructure" → show build tutorial prompt
  - "No contracts" → "Explore regions to find contract opportunities"
  - "No auctions" → "Spectrum auctions begin periodically"
  - "No saved games" → "Start a new game to create your first save"
- **Files:** all panel files with empty states

### 3.7 Chat/Events positioning fix
- Chat: bottom-left corner
- Events feed: bottom-right corner (above mini-map)
- No overlap possible
- **Files:** `web/src/lib/game/Chat.svelte`, `web/src/lib/game/NotificationFeed.svelte`

### 3.8 Wire up PerfMonitor
- Import and render `PerfMonitor.svelte` in `GameView.svelte` when `showPerfMonitor` store is true
- F3 shortcut already works in GameLoop.ts
- **File:** `web/src/lib/game/GameView.svelte`

### 3.9 Fix Research panel rendering
- Debug why Research panel doesn't appear (likely data issue or conditional rendering bug)
- Ensure research tree displays with era-gated items
- **File:** `web/src/lib/panels/ResearchPanel.svelte`

### 3.10 Start-paused behavior
- Game starts in PAUSED state after world generation completes
- Show orientation overlay: "Welcome to [Corp Name]. Your starting region is [Region]. Press Play to begin."
- Tutorial automatically begins when player unpauses
- **Files:** `web/src/lib/game/GameLoop.ts`, `web/src/lib/game/GameView.svelte`, `crates/gt-wasm/src/lib.rs` (initial speed = Paused)

### 3.11 Auto-pause on critical events
- Auto-pause when:
  - Disaster strikes player infrastructure (severity > 0.3)
  - Corporation enters insolvency
  - Hostile acquisition proposed against player
  - Espionage detected against player
  - Era progression milestone reached
- Show notification banner with "PAUSED: [reason]" and resume button
- Can be toggled off in settings
- **Files:** `web/src/lib/game/GameLoop.ts`, `web/src/stores/settings.ts`, `web/src/lib/game/NotificationFeed.svelte`

---

## Phase 4: Multiplayer & Lobby

> **Note:** Items 4.1, 4.2, and parts of 4.9/4.12 have been substantially addressed by INSTANT_MULTIPLAYER_PLAN.md (all 10 phases complete). The multiplayer system now uses event-driven delta broadcasts (CommandBroadcast + DeltaOps), incremental WASM state updates (applyBatch), per-type rate limiting, spatial validation, speed vote system, per-player event filtering, sequence dedup, and ghost entity optimistic UI.

### 4.1 Full state snapshot [ADDRESSED by INSTANT_MULTIPLAYER_PLAN]
- Full snapshots now sent every 30 ticks as safety net (down from 5)
- Primary sync via CommandBroadcast + applyBatch (sub-200ms)
- **Files:** `crates/gt-server/src/ws.rs`, `crates/gt-simulation/src/world.rs`, `crates/gt-server/src/tick.rs`

### 4.2 Pure thin client mode [ADDRESSED by INSTANT_MULTIPLAYER_PLAN]
- Event-driven rendering (map-dirty events + 2s fallback, not 500ms polling)
- applyBatch for incremental WASM state updates from CommandBroadcast
- Ghost entity system for optimistic UI
- **Files:** `web/src/lib/game/GameLoop.ts`, `web/src/lib/multiplayer/WebSocketClient.ts`, `web/src/lib/stores/multiplayerState.ts`, `web/src/lib/game/commandRouter.ts`

### 4.3 Fix deadlock (ABBA lock pattern)
- Establish strict lock ordering: always `world.world` before `world.players`
- Refactor reconnection path to acquire locks in correct order
- **File:** `crates/gt-server/src/ws.rs`

### 4.4 Handle broadcast Lagged error
- In forwarder loop, match on `Err(broadcast::error::RecvError::Lagged(n))` explicitly
- Log warning, continue receiving (don't break)
- If lag > threshold, request fresh snapshot for that player
- **File:** `crates/gt-server/src/ws.rs`

### 4.5 Fly.io config fix
- Set `min_machines_running = 1`
- Set `auto_stop_machines = 'off'`
- **File:** `fly.toml`

### 4.6 Graceful shutdown
- Add `tokio::signal::ctrl_c()` shutdown handler
- On shutdown: save all world snapshots to DB, notify connected players, close WebSockets cleanly
- **File:** `crates/gt-server/src/main.rs`

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
- **File:** `crates/gt-server/src/db.rs`

### 4.11 DB connection pool config
- Use `PgPoolOptions::new().max_connections(10).min_connections(2).acquire_timeout(Duration::from_secs(5))`
- **File:** `crates/gt-server/src/db.rs`

### 4.12 WebSocket registration validation
- Share validation logic between REST and WS registration paths
- Extract to shared `validate_registration()` function
- **Files:** `crates/gt-server/src/ws.rs`, `crates/gt-server/src/routes.rs`

### 4.13 Fix REST save upload
- Add binary save data field to `SaveUploadRequest` struct (base64 encoded)
- **File:** `crates/gt-server/src/routes.rs`

### 4.14 World creation limits
- Max 3 worlds per authenticated user, 0 for guests
- **File:** `crates/gt-server/src/routes.rs`

### 4.15 Full lobby overhaul
- **World Browser:**
  - Persistent world list with real-time player count
  - Filter by: era, player count, world age, region focus
  - Search by world name or creator
  - Sort by: newest, most players, most active
  - World preview card: settings summary, player list, leaderboard position
- **World Creation:**
  - Authenticated users can create persistent worlds (max 3 per user)
  - Configure: era, map size, difficulty, max players, world name, description
  - Custom world rules (disaster frequency, starting capital, AI count)
- **Leaderboard:**
  - Per-world rankings: net worth, infrastructure count, market share, regions dominated
  - Global rankings across all worlds
  - Historical leaderboard snapshots
- **Files:** `web/src/lib/menu/WorldBrowser.svelte` (major rewrite), `crates/gt-server/src/routes.rs` (leaderboard endpoints), new `web/src/lib/menu/WorldCreation.svelte`, new `web/src/lib/menu/Leaderboard.svelte`

---

## Phase 5: Core Simulation Fixes

### 5.1 Replace `is_multiple_of` with stable alternative
- Replace all 9 instances of `tick.is_multiple_of(N)` with `tick % N == 0`
- **Files:** 8 files in `crates/gt-simulation/src/systems/` + `crates/gt-world/src/cities.rs`

### 5.2 Fix event serialization + priority/category system
- Replace `format!("{:?}", event)` with proper serde JSON serialization
- Define structured `EventData` variants with human-relevant fields
- Frontend receives structured JSON, formats into readable strings:
  - `ConstructionCompleted { entity, node_type, region }` → "Your Cell Tower in Pacific Northwest is complete!"
  - `DisasterStruck { region, severity, type, affected }` → "Hurricane hits Pacific Northwest — 3 nodes damaged!"
  - `BankruptcyDeclared { corp_name }` → "TelcoMax has declared bankruptcy!"
- **Event priority levels:**
  - `Critical`: disaster strikes, bankruptcy, hostile takeover, espionage detected → shows as banner + sound, auto-pause
  - `Important`: construction complete, patent filed, auction won, contract signed → shows in notification feed
  - `Info`: minor market changes, routine regulation, population updates → collapsed by default, expandable
- **Event categories** (filterable in notification feed):
  - Construction, Finance, Competition, Diplomacy, Disaster, Achievement, Legal, Research, Market
  - Player can toggle categories on/off; unread badge per category
- **Files:** `crates/gt-common/src/events.rs` (add priority + category fields), `crates/gt-wasm/src/lib.rs`, `web/src/lib/game/NotificationFeed.svelte` (priority rendering + category filters), new `web/src/lib/game/eventFormatter.ts`

### 5.3 Player starter node
- When creating player corporation, place 1 starter node (appropriate to starting era):
  - Telegraph: Telegraph office
  - Telephone: Telephone exchange
  - Early Digital: Central office
  - Internet: Central office
  - Modern: Cell tower
  - Near Future: Cell tower
- Place in highest-population city cell within starting region
- Connect with 1 edge to nearest city backbone
- Tutorial references this existing node: "You have a [node type] in [city]. Let's expand your network."
- **Files:** `crates/gt-simulation/src/world.rs` (player corp creation), `web/src/lib/game/Tutorial.svelte`

### 5.4 Tutorial-guided first expansion
- Enhance existing tutorial system with interactive steps (10 steps):
  - Step 1-2: Welcome + overview ("Welcome to [Corp Name]! You have a [starter node] in [city].")
  - Step 3: "Click your node to see its details" (highlight starter node on map)
  - Step 4: "Let's expand — click + Node to build" (highlight build button)
  - Step 5: "Select a cell near a city" (highlight valid cells, show coverage preview)
  - Step 6: "Choose node type" (show available types for current era, explain tier system)
  - Step 7: "Connect with an edge" (guide edge building, explain bandwidth)
  - Step 8: "You're now earning revenue!" (show financial impact, explain revenue sources)
  - Step 9: "Explore panels" (quick tour of dashboard, research, contracts)
  - Step 10: "Ready to compete!" (dismiss tutorial, mention advisor panel)
- **Files:** `web/src/lib/game/Tutorial.svelte`, `web/src/stores/tutorialState.ts`

### 5.5 In-game date display
- Add tick-to-date conversion in `gt-common`:
  - Telegraph era: 1 tick = 1 month (start ~1850)
  - Telephone era: 1 tick = 2 weeks (start ~1900)
  - Early Digital: 1 tick = 1 week (start ~1970)
  - Internet era: 1 tick = 1 week (start ~1990)
  - Modern: 1 tick = 1 day (start ~2010)
  - Near Future: 1 tick = 1 day (start ~2030)
- Calculate in-game year/month from starting era date + tick count
- Display in HUD as "Jan 1995" alongside tick number
- **Files:** `crates/gt-common/src/types.rs`, `web/src/lib/game/HUD.svelte`

### 5.6 Connect difficulty config to systems
- Wire `disaster_frequency` to disaster system probability checks
- Wire `market_volatility` to market system price fluctuations
- Wire `construction_time_multiplier` to construction system build times
- **Files:** `crates/gt-simulation/src/systems/disaster.rs`, `market.rs`, `construction.rs`

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
- Base initial rating on starting capital rather than revenue ratio
- **File:** `crates/gt-simulation/src/systems/finance.rs`

### 5.10 Disaster risk variation
- Calculate per-region disaster risk based on terrain composition:
  - Coastal: +0.15 (floods, storms)
  - Mountainous: +0.1 (landslides, earthquakes)
  - Desert: +0.05 (sandstorms)
  - Urban: +0.05 (infrastructure failures)
  - Tundra/Frozen: +0.1 (ice storms)
- **File:** `crates/gt-simulation/src/world.rs`

### 5.11 Revenue history tracking
- Add `revenue_history: Vec<(u64, Money)>` to corporation or financial component
- Record revenue/cost each tick (or every N ticks) for chart data
- Expose via WASM bridge for chart consumption
- **Files:** `crates/gt-simulation/src/systems/revenue.rs`, `crates/gt-common/src/types.rs`, `crates/gt-wasm/src/lib.rs`

### 5.12 Coverage spatial index
- Add grid-based spatial hash for cell lookups
- Replace O(N*M) cell scan with O(N*k) where k = cells in range
- **File:** `crates/gt-simulation/src/systems/coverage.rs`

### 5.13 Connectivity → GDP feedback loop (with real-world dampening)
- Infrastructure connectivity drives regional economic growth:
  - Calculate **connectivity score** per region: weighted sum of (node count * tier weight + edge bandwidth * utilization)
  - Connectivity score feeds into GDP growth modifier
  - Higher GDP → population migration toward connected regions (people move where jobs are)
  - Higher population → increased demand (more customers)
  - Result: positive feedback loop that grows regional economy — BUT with real-world dampening

- **Growth mechanics:**
  - GDP growth rate = base_rate * connectivity_modifier * competition_factor * saturation_factor
  - Connectivity modifier: 0-25% satisfaction → +0.5% GDP/year, 25-50% → +1.0%, 50-75% → +2.0%, 75-100% → +3.0%
  - Migration: regions above median connectivity attract 0.1-0.5% of neighboring population per year

- **Dampening mechanism 1: Market saturation**
  - Each region has a demand ceiling based on population * wealth_factor
  - When 95%+ of potential customers are served, new infrastructure doesn't grow demand further
  - Saturation factor: `min(1.0, unserved_customers / total_potential_customers)`

- **Dampening mechanism 2: Competition splits the pie**
  - GDP growth bonus from connectivity is split among ALL providers in a region
  - If 3 corps serve a region, each gets 1/3 of the GDP boost benefit for revenue calculation
  - First-mover advantage: early entrant captures larger market share before competitors arrive

- **Dampening mechanism 3: Rising costs (terrain + labor)**
  - As regional GDP grows, construction costs increase: `base_cost * terrain_modifier * (1 + gdp_growth_ratio * 0.5)`
  - Construction labor costs rise similarly: `base_construction * (1 + gdp_growth_ratio * 0.3)`
  - Effect: booming regions become more expensive to build in, creating natural cost brakes

- **Dampening mechanism 4: Regulatory intervention**
  - When one corp dominates a region (>60% market share):
    - Regulators impose price caps (max 1.5x standard pricing)
    - Force infrastructure sharing requests (co-ownership proposals can't be refused if >70% share)
    - Block further land acquisition in that region for the dominant corp
  - Anti-monopoly threshold scales with region size (smaller regions tolerate higher concentration)

- **Files:** `crates/gt-simulation/src/systems/population.rs` (migration), `crates/gt-simulation/src/systems/demand.rs` (saturation), `crates/gt-simulation/src/systems/market.rs` (competition split), `crates/gt-simulation/src/systems/regulation.rs` (anti-monopoly), `crates/gt-common/src/types.rs` (regional GDP/cost fields)

### 5.14 World era as collective milestone (cosmetic)
- **Important:** Research is NOT era-gated. Players can freely research up the tech tree into higher eras. A player in the "Telegraph era" CAN research Internet-era tech if they have the prerequisites and resources.
- **Individual corps** don't have an "era" — they have a tech portfolio. The build menu shows all node types they've researched regardless of era label.
- **World era** is a cosmetic collective milestone:
  - World era = the highest era where ALL active corps have at least 1 completed tech
  - Example: if 4 corps exist and 3 have Internet-era tech but 1 only has Telephone-era tech, world era = Telephone
  - When world era advances: `GameEvent::WorldEraAdvanced` emitted, achievement unlocked, displayed in HUD
  - No gameplay effects tied to world era — individual tech research is what gates building
- **Display:** World era shown in HUD next to in-game date. Individual corp's "highest researched era" shown in dashboard.
- **Tick-to-date:** Based on the game's starting era config (set at world creation), not the current world era. Date progresses linearly from start.
- **Files:** `crates/gt-simulation/src/systems/research.rs` (world era calculation after any tech completion), `web/src/lib/game/HUD.svelte`

---

## Phase 6: Era-Specific Infrastructure & Technology Overhaul

### 6.1 Era-specific node types
- Expand the 8 generic node types to era-specific variants. Each era introduces historically accurate infrastructure:

**Telegraph Era (~1850s):**
- `TelegraphOffice` — Access tier, wired, 2km range, $10K construction
- `TelegraphRelay` — Aggregation tier, wired, $20K

**Telephone Era (~1900s):**
- `TelephoneExchange` — Aggregation tier, wired, 5km, $50K
- `OperatorSwitch` — Core tier, wired, $100K
- `LongDistanceRelay` — Backbone tier, $200K

**Early Digital Era (~1970s):**
- `DigitalSwitch` — Core tier, wired, $500K
- `MicrowaveTower` — Backbone tier, wireless, 50km, $300K
- `CoaxHub` — Access tier, wired, 3km, $150K

**Internet Era (~1990s):**
- `DSLTerminal` — Access tier, wired, 5km, $200K
- `FiberPOP` — Aggregation tier, wired, $800K
- `WebHostingCenter` — Core tier, $2M
- `DialUpGateway` — Access tier, wired, $100K

**Modern Era (~2010s):**
- `CellTower4G` / `CellTower5G` — Access tier, wireless, 15km, $200K / $350K
- `DataCenter` — Core tier, $10M
- `FTTHNode` — Access tier, wired, 2km, $500K
- `CDNEdge` — Aggregation tier, $1M
- `ExchangePoint` — Aggregation tier, $2M
- `BackboneRouter` — Backbone tier, $3M

**Near Future Era (~2030s):**
- `Cell6G` — Access tier, wireless, 20km, $500K
- `SatelliteGround` — Global tier, 200km, $5M
- `QuantumRelay` — Backbone tier, $15M
- `EdgeAINode` — Aggregation tier, $3M
- `SubmarineLanding` — Global tier, $20M

- Keep backward compatibility: existing 8 generic types become aliases for their Modern-era equivalents
- Each node type definition includes: `era_requirement: Era`, `required_research: Option<TechId>`, standard fields
- **Files:** `crates/gt-common/src/types.rs` (major expansion of NodeType enum), `crates/gt-wasm/src/lib.rs` (get_build_options filters by corp's current era)

### 6.2 Era-specific edge types
- Similar expansion for edges:

**Telegraph:** `TelegraphWire` (low bandwidth, cheap)
**Telephone:** `CopperPair` (voice-grade), `TrunkLine` (multi-pair copper)
**Early Digital:** `CoaxialCable`, `MicrowaveLink`
**Internet:** `FiberLocal`, `FiberRegional`, `DSLLine`
**Modern:** `FiberNational`, `DarkFiber` (unlit, capacity lease), `5GBackhaul`
**Near Future:** `Submarine`, `Satellite`, `QuantumFiber`

- **Files:** `crates/gt-common/src/types.rs` (EdgeType expansion), `crates/gt-infrastructure/src/lib.rs` (tier connection rules)

### 6.3 Robust technology & patent system
- **Research tree philosophy:** Tech is the primary economic commodity. Research is organized by era but NOT gated by era — any corp can research up the tree freely if they have prerequisites and funds. This creates a tech-driven economy where patents, licenses, and leases are core revenue streams.

- **Overhaul research from bonus-only to unlock-gating:**
  - Each technology has: `id`, `name`, `era_label` (cosmetic grouping only), `category`, `cost`, `research_ticks`, `prerequisites: Vec<TechId>`, `unlocks: Vec<NodeType/EdgeType/Ability>`, `bonuses: TechBonuses`
  - Research completion REQUIRED to build era-specific node/edge types
  - Build validation: `cmd_build_node` checks corp has researched the required tech OR holds a valid license
  - Restructure existing 36 techs INTO the era framework: "Dense WDM" → Internet-era, "Quantum Key Distribution" → Near Future. Add new techs to fill gaps in earlier eras (telegraph, telephone, early digital).

- **Patent system (full enforcement with workaround):**
  - When a corp completes research FIRST (globally), they can choose: Patent, Open Source, or Keep Proprietary
  - `Patent`: Corp owns exclusive rights for N ticks (50-200 depending on tech complexity). Others CANNOT build the unlocked node types without a license. **Hard block.**
  - `OpenSource`: All corps immediately get the tech. Researcher gets reputation/achievement bonus.
  - `Proprietary`: Corp keeps bonuses. Others can still research the tech independently (see workaround below).
  - Patent enforcement gate: `cmd_build_node` → check if required tech is patented by another corp → if yes, check for valid license → if no license, reject build.

- **Independent research workaround (bypasses patents):**
  - If a tech is patented by another corp, you have three options:
    1. **License it** (pay the patent holder)
    2. **Independent research at 150% cost/time** — produces base access to the tech. You can build the node types but CANNOT patent your version. The original patent is unaffected.
    3. **Independent research at 200% cost/time** — produces an IMPROVED version (+10% bonus over original). You CAN patent your improved version as a competing patent. Creates innovation races.
  - Independent research requires no permission from the patent holder
  - Existing nodes built under a license remain functional even if the license expires (no retroactive destruction)

- **License system:**
  - Patent holder sets license price: one-time fee, per-tick royalty, or per-node-built fee
  - Other corps can: request license, negotiate price, or wait for patent expiration
  - License revenue flows to patent holder each tick (for royalty-based)
  - License types:
    - `Permanent`: one-time payment, perpetual access
    - `Royalty`: per-tick payment, revocable if payment missed
    - `PerUnit`: fee charged each time a node of that type is built
  - AI strategy for licensing: Aggressive denies competitors or sets extortionate prices, Budget offers cheap licenses for revenue, Tech Innovator patents aggressively and licenses selectively, Defensive licenses readily for defensive alliances

- **Lease system (temporary license):**
  - Corps can lease technology access instead of buying outright
  - Lease = lower upfront cost but ongoing payments (higher than royalty long-term)
  - Lease has fixed duration (negotiable, typically 50-100 ticks)
  - If lease expires: lessee can't build NEW nodes of that type (existing nodes remain and function)
  - Lease renewals possible at renegotiated rates
  - Good for late-joiners who need quick access to catch up

- **Research tree visualization (D3 force-directed graph):**
  - Techs as nodes, prerequisites as directed edges
  - Clustering by category (Optical, Wireless, Satellite, etc.) via force simulation
  - Era labels as background regions/zones (not gates)
  - Color coding: green=completed, blue=in-progress, gray=available, dark-gray=prerequisites-not-met, gold=patented-by-you, red=patented-by-competitor, purple=licensed
  - Node size proportional to tech importance (number of things it unlocks)
  - Click tech to see: description, cost, time, prerequisites, what it unlocks, patent status, license availability, independent research option
  - Patent/license/lease controls integrated into tech detail popup
  - Filter view: by category, by era, by patent status, "show only available"

- **New components:** `Patent { owner, tech_id, filed_tick, expires_tick, license_price, license_type, improved_version: bool }`, `LicenseAgreement { licensor, licensee, tech_id, license_type: Permanent|Royalty|PerUnit|Lease, amount, duration, ticks_remaining }`, expanded `TechResearch { independent: bool, improved: bool }`
- **New/modified ECS system:** `research.rs` (add patent filing, license validation, independent research tracking), new `patent.rs` system for license revenue collection, patent expiration, and enforcement
- **Files:** `crates/gt-common/src/types.rs`, `crates/gt-common/src/commands.rs` (FilePatent, RequestLicense, SetLicensePrice, RevokeLicense, StartIndependentResearch, ProposeLease, AcceptLease), `crates/gt-simulation/src/systems/research.rs` (major overhaul), new `crates/gt-simulation/src/systems/patent.rs`, `crates/gt-wasm/src/lib.rs`, `web/src/lib/panels/ResearchPanel.svelte` (major overhaul with D3 force graph)

---

## Phase 7: New Gameplay Systems

### 7.1 Team-based workforce management
- Replace flat `Workforce { employee_count, skill_level, morale, salary_per_tick }` with:
  ```
  Team { team_type, size, skill_level, morale, salary_per_tick, assignment }
  TeamType: Construction, Maintenance, Sales, Engineering, Legal, Research
  ```
- Corps hire/fire teams, not individuals
- Team effects:
  - Construction teams: speed up building (more teams = faster construction, diminishing returns)
  - Maintenance teams: reduce failure rate, increase repair speed
  - Sales teams: increase contract win probability, improve customer satisfaction
  - Engineering teams: boost research speed
  - Legal teams: required for patent filing, improve lawsuit outcomes (Phase 8)
  - Research teams: dedicated R&D — boost tech completion speed
- New Workforce panel shows team list with hire/fire/reassign controls
- **Files:** `crates/gt-common/src/types.rs`, `crates/gt-simulation/src/systems/cost.rs`, `crates/gt-economy/src/lib.rs`, `web/src/lib/panels/WorkforcePanel.svelte`

### 7.2 Spectrum auction system
- Real-time competitive auctions:
  - Server creates auction events periodically (every ~50 ticks)
  - Each auction: spectrum band + region + starting price + duration (10 ticks)
  - Players/AI submit increasing bids during the window
  - Winner gets exclusive spectrum license for that band+region
  - Spectrum licenses required for wireless node types in that region
- New components: `SpectrumLicense`, `AuctionEvent`, `Bid`
- AI bidding strategy based on archetype (Aggressive bids high, Budget bids low, Tech Innovator targets advanced bands)
- **Files:** `crates/gt-common/src/types.rs`, `crates/gt-simulation/src/systems/auction.rs` (expand existing), `crates/gt-ai/src/strategy.rs`, `web/src/lib/panels/AuctionPanel.svelte`

### 7.3 Player-controlled regional pricing
- Players set price tiers per region (or globally with per-region overrides):
  - **Budget:** Low price, high market share capture, low revenue per customer
  - **Standard:** Balanced pricing
  - **Premium:** High price, low market share, high revenue per customer
  - **Custom:** Set exact price multiplier (0.5x to 2.0x)
- Pricing affects:
  - Customer acquisition rate (lower price = faster adoption)
  - Revenue per customer (direct multiplier)
  - Customer churn (high price + low quality = churn)
  - Competition response (AI adjusts pricing in response)
- Price elasticity varies by region wealth (rich regions tolerate premium, poor regions need budget)
- New command: `SetRegionPricing { region_id, price_tier }` or `SetGlobalPricing { price_tier }`
- **Files:** `crates/gt-common/src/types.rs` (PriceTier enum, pricing fields on Corporation), `crates/gt-common/src/commands.rs`, `crates/gt-simulation/src/systems/revenue.rs` (pricing multiplier), `crates/gt-simulation/src/systems/demand.rs` (price affects adoption), new pricing section in `web/src/lib/panels/DashboardPanel.svelte` or dedicated `PricingPanel.svelte`

### 7.4 Maintenance scheduling
- Player-controlled maintenance per node:
  - **Priority tiers:** Critical (highest budget, fastest repair), Standard, Low Priority, Deferred (no maintenance spend)
  - **Maintenance budget:** Allocate maintenance budget per node or per tier of nodes
  - Per-node maintenance status: health %, last maintained tick, estimated failure risk
- Effects:
  - Critical priority: 1.5x maintenance cost, health degrades 50% slower, repairs 2x faster
  - Standard: 1x cost (current behavior)
  - Low Priority: 0.5x cost, health degrades 1.5x faster
  - Deferred: 0x cost, health degrades 3x faster (use for nodes being decommissioned)
- Maintenance teams (from 7.1) amplify effectiveness: more maintenance teams = better coverage across all nodes
- New command: `SetMaintenancePriority { node_id, priority }`, `SetMaintenanceBudget { budget_per_tick }`
- **Files:** `crates/gt-common/src/types.rs`, `crates/gt-common/src/commands.rs`, `crates/gt-simulation/src/systems/maintenance.rs`, new `web/src/lib/panels/MaintenancePanel.svelte`

### 7.5 Government grants & development contracts
- Periodically, regions generate **government grant opportunities**:
  - "Expand broadband to underserved rural area X" — build N nodes in specified cells within M ticks
  - "Upgrade backbone capacity for region Y" — achieve N Gbps throughput
  - "Disaster recovery for region Z" — rebuild damaged infrastructure within M ticks
- Grant structure: `GovernmentGrant { region_id, description, requirements, reward, deadline_tick, status }`
- Rewards: cash grant (50-500% of estimated build cost), tax reduction in region, exclusive operating license
- Corps can bid for grants (lowest bid wins, or best capability wins based on existing infrastructure)
- AI corps also compete for grants
- Grants appear in a dedicated section of the Contracts panel or new Government panel
- **Files:** `crates/gt-common/src/types.rs`, `crates/gt-common/src/commands.rs` (BidForGrant, CompleteGrant), new `crates/gt-simulation/src/systems/grants.rs`, `web/src/lib/panels/ContractPanel.svelte` (add grants section)

### 7.6 Subsidiary management UI
- Frontend panel to view/manage subsidiaries (backend already supports them):
  - Create subsidiary (specify name, region focus, initial budget)
  - Transfer assets between parent and subsidiary
  - Set subsidiary operating budget and policies
  - View subsidiary P&L, infrastructure, workforce
  - Merge subsidiary back into parent
- **Files:** new `web/src/lib/panels/SubsidiaryPanel.svelte`, `web/src/lib/game/HUD.svelte`

### 7.7 Sandbox mode
- Add "Sandbox" as a game mode option in New Game menu (alongside Normal/Custom difficulty):
  - **Infinite money:** Starting cash = $999,999,999,999. Revenue/cost still tracked for display but bankruptcy impossible.
  - **All tech unlocked:** Every research topic marked complete. All node/edge types buildable from tick 1.
  - **Adjustable AI:** Spawn/remove AI corps on demand via sandbox controls panel. Control AI archetype, strategy, aggression.
  - **No disasters** (toggle-able: can re-enable for testing)
  - **Full system access:** Alliances, auctions, legal, patents all functional — AI participates fully
  - **Speed override:** Add 16x and 32x speed options in sandbox
  - **God-mode controls:** Teleport to any region, instantly build nodes, force auction, trigger disaster, advance world era
- Sandbox controls accessible via dedicated floating panel (only in sandbox mode)
- Sandbox mode clearly indicated in HUD: "[SANDBOX]" badge
- Sandbox saves are marked and cannot be converted to normal mode
- **Files:** `crates/gt-common/src/types.rs` (GameMode enum: Normal, Custom, Sandbox), `crates/gt-simulation/src/world.rs` (sandbox overrides), `web/src/lib/menu/NewGame.svelte` (sandbox option), new `web/src/lib/panels/SandboxPanel.svelte`, `web/src/lib/game/HUD.svelte` (sandbox badge)

### 7.8 Full audio expansion
- Add sound effects for all major game events using existing `AudioManager.ts` (Web Audio API):
  - **Construction:** Build complete chime, edge connection sound
  - **Disaster:** Alarm siren (severity-scaled), damage impact sound
  - **Finance:** Cash register (revenue milestone), warning tone (low funds), bankruptcy alarm
  - **Auction:** Opening bell, bid placed tick, auction won fanfare, outbid alert
  - **Research:** Tech complete discovery sound, patent filed stamp sound
  - **Legal:** Gavel sound (lawsuit filed/resolved), settlement ding
  - **Alliance:** Trumpet/fanfare (alliance formed), dissolution warning tone
  - **Era:** Progression fanfare when world era advances
  - **Espionage:** Stealth/spy sound (mission launched), alert (detected)
  - **UI:** Panel open/close subtle click, button hover, notification pop
- Audio volume controls per category in Settings (master, UI, alerts, ambient)
- Mute toggle in HUD
- **Files:** `web/src/lib/game/AudioManager.ts` (expand with new sound types), `web/src/stores/settings.ts` (volume controls), new audio assets in `web/static/audio/`

---

## Phase 8: Advanced Gameplay Systems

### 8.1 Alliance system
- Corps can form alliances for mutual benefit:
  - **Creation:** Propose alliance to another corp (player or AI). Both must accept.
  - **Benefits:**
    - **Free routing through ally infrastructure** — no transit fees. Revenue from customers served through shared routes is split proportional to hops (nodes/edges each ally contributes). Example: traffic crosses 3 of your nodes + 2 of ally's → you get 60%, ally gets 40%.
    - Joint spectrum bidding (pool resources for auctions)
    - Mutual defense pact (ally warned of espionage/sabotage against you)
    - Shared research (optional: share completed tech with allies at 50% of license cost)
  - **Maintenance:** Alliance has a trust score. Competing in ally's regions, undercutting prices, or hostile actions reduce trust.
  - **Dissolution:** Either party can dissolve. Sudden dissolution imposes transition period (30 ticks) where shared access winds down. During transition, routing reverts to paid transit fees.
  - **Voting:** Major alliance decisions (joint infrastructure investment, accepting new members) require majority vote.
  - **Limits:** Max 3 corps per alliance. No mega-alliances.
- AI alliance behavior: DefensiveConsolidator seeks alliances readily, AggressiveExpander avoids them, TechInnovator allies for research sharing, BudgetOperator allies for cost reduction
- **Files:** `crates/gt-common/src/types.rs` (Alliance, AllianceMember, AllianceProposal), `crates/gt-common/src/commands.rs` (ProposeAlliance, AcceptAlliance, DissolveAlliance, AllianceVote), new `crates/gt-simulation/src/systems/alliance.rs`, `crates/gt-ai/src/diplomacy.rs` (expand), new `web/src/lib/panels/AlliancePanel.svelte`

### 8.2 Legal system
- Corps can take legal action:
  - **Lawsuit types:**
    - Sabotage claim: after detected espionage/sabotage, sue for damages (requires Legal team)
    - Ownership dispute: challenge co-ownership terms or hostile acquisition
    - Patent infringement: sue for unlicensed use of patented technology
    - Regulatory complaint: report competitor to regulators (triggers investigation)
  - **Process:**
    - Filing costs: base fee + Legal team requirement (must have at least 1 Legal team from Phase 7.1)
    - Resolution: takes 20-50 ticks. Outcome influenced by: evidence strength, Legal team size, jurisdiction regulatory strictness
    - Outcomes: damages awarded (cash transfer), forced licensing, asset forfeiture, regulatory fine, case dismissed
  - **UX: Auto-suggest with player confirmation:**
    - When grounds for a lawsuit are detected (espionage caught, patent infringement, sabotage), notification appears: "You have grounds for a [lawsuit type] against [corp]. File lawsuit?"
    - Player clicks to confirm or dismiss. Never auto-files.
    - Legal panel shows "Potential Claims" section with all actionable grounds
  - **Arbitration:** For co-ownership and contract disputes, mandatory arbitration (faster, cheaper, less punitive)
  - **AI behavior:** Aggressive corps file more lawsuits. Defensive corps settle quickly. AI evaluates lawsuit ROI (expected damages vs filing cost + legal team time) before filing. Full autonomy — AI initiates lawsuits proactively.
- **Files:** `crates/gt-common/src/types.rs` (Lawsuit, LawsuitType, LawsuitOutcome), `crates/gt-common/src/commands.rs` (FileLawsuit, SettleLawsuit, DefendLawsuit), new `crates/gt-simulation/src/systems/legal.rs`, `web/src/lib/panels/IntelPanel.svelte` (add legal section) or new `LegalPanel.svelte`

### 8.3 Dynamic AI spawning & mergers
- **Mid-game AI spawning:**
  - Market system evaluates conditions every 100 ticks:
    - If total market satisfaction < 60% and fewer than `max_ai_corps` exist → spawn new AI corp
    - New corp gets: random archetype, starting capital based on current era, placed in underserved region
    - Spawn rate limited: max 1 new corp per 200 ticks
  - New corp announcement: "A new competitor, [Generated Name], has entered [Region]!"

- **AI mergers:**
  - Two AI corps can merge when:
    - Both are in Consolidate or Survive strategy
    - Combined market share < 40% (anti-monopoly)
    - Adjacent or overlapping regions
  - Merger process: 50-tick negotiation period, then assets combine under dominant corp's name
  - Merged corp gets combined infrastructure, larger workforce, averaged financials
  - Event: "[Corp A] and [Corp B] have merged to form [New Name]!"

- **AI bankruptcy & dissolution:**
  - Already implemented but enhance: bankrupt AI corp's assets go to auction (existing behavior)
  - Add: remaining workforce becomes available for hire (player gets notification)
  - Add: bankrupt corp's customers redistributed to remaining providers in region

- **Files:** `crates/gt-simulation/src/systems/market.rs` (spawning logic), `crates/gt-simulation/src/systems/ai/mod.rs` (merger evaluation), `crates/gt-simulation/src/world.rs` (spawn_ai_corporation utility), `crates/gt-common/src/events.rs` (AISpawned, AIMerger events)

### 8.4 Intel-gated visibility (full fog of war — all competitors)
- **Applies to ALL competitors — both AI and other players in multiplayer.** Geography (terrain, regions, cities, borders) is always visible. Competitor INFRASTRUCTURE and activity is hidden by default.
- Intel levels per competitor:
  - **No intel (default):** See competitor corp names and aggregate market share (public filings). CANNOT see their infrastructure on map, building activity, research progress, or strategy. Map shows blank areas.
  - **Basic intel (via espionage success):** See competitor's infrastructure locations on map, basic financials (revenue estimate), active regions. Infrastructure icons appear on map for that competitor.
  - **Full intel (via sustained espionage):** See competitor's current strategy, research progress, patent portfolio, planned builds, pricing, alliance memberships, workforce size. Full dossier.
- Intel levels decay over time (50 ticks) unless maintained by ongoing espionage missions
- Intel gathered via existing `CovertOps` system (espionage missions) — requires Espionage covert op targeting specific competitor
- **Map rendering:** Infrastructure sprites only rendered for competitors where player has basic+ intel. Unknown competitors' regions show as neutral-colored (no ownership overlay for them).
- **Multiplayer implications:** In MP, players must spy on EACH OTHER to see infrastructure. Creates strategic depth — you don't know what other players are building until you invest in espionage. Alliance members automatically share basic intel (mutual defense pact benefit).
- Intel panel shows intelligence dossier per competitor corp with current intel level and decay timer
- Notifications filtered: "Our intelligence reports that TelcoMax is expanding into Region 5" (requires basic intel on TelcoMax)
- **Files:** `crates/gt-common/src/types.rs` (IntelLevel enum, intel tracking per corp pair), `crates/gt-simulation/src/systems/covert_ops.rs` (intel accumulation + decay), `crates/gt-wasm/src/lib.rs` (filter infrastructure/corporation queries by intel level), `crates/gt-server/src/ws.rs` (server-side intel filtering for MP — don't send infrastructure data player shouldn't see), `web/src/lib/game/MapRenderer.ts` (conditional rendering), `web/src/lib/panels/IntelPanel.svelte` (dossier view with intel levels)

---

## Phase 9: New Frontend Panels

### 9.1 Co-ownership panel
- Backend already supports co-ownership (ProposeCoOwnership, RespondCoOwnership, VoteUpgrade, ProposeBuyout commands)
- New panel to manage shared infrastructure:
  - **My Co-owned Assets:** list of nodes/edges with co-owners and ownership percentages
  - **Pending Proposals:** incoming co-ownership requests with accept/decline
  - **Active Votes:** upgrade proposals requiring your vote
  - **Buyout Offers:** incoming buyout offers with accept/counter/decline
  - **Revenue Split:** shows revenue distribution per co-owned asset
  - **Propose Co-ownership:** select one of your nodes, set ownership % to offer, select target corp
- **Files:** new `web/src/lib/panels/CoOwnershipPanel.svelte`, add to `uiState.ts` panel types, add button to HUD

### 9.2 Insurance management panel
- Backend already supports insurance (PurchaseInsurance/CancelInsurance commands, insured flag, premium calculation)
- New panel:
  - **Insured Assets:** list of insured nodes with premium cost per tick
  - **Uninsured Assets:** list of uninsured nodes with "Insure" button and premium quote
  - **Total Premium:** aggregate insurance cost per tick
  - **Claims History:** past disaster payouts received
  - **Bulk Actions:** "Insure All", "Cancel All", "Insure by Tier" (e.g., insure all backbone nodes)
  - Premium shown as: $X/tick (Y% of construction cost)
  - Coverage: 60% of damage cost (per existing design)
- **Files:** new `web/src/lib/panels/InsurancePanel.svelte`, `crates/gt-wasm/src/lib.rs` (add get_insurance_data query)

### 9.3 Damage alerts & repair panel
- **Damage Alert System:**
  - When disaster strikes, show prominent alert banner: "ALERT: [Disaster] in [Region] — [N] nodes damaged!"
  - Auto-pause triggers (per 3.11)
  - Damaged nodes flash/pulse on map with warning icon overlay
  - Notification feed shows each damaged node individually

- **Repair Panel:**
  - **Damaged Infrastructure:** list of all damaged nodes with current health %, damage severity, location
  - **Repair Options per node:**
    - Emergency Repair: 60% of construction cost, instant (next tick)
    - Normal Repair: 20% of construction cost, takes N ticks (based on damage severity)
    - Decommission: salvage 10% of construction cost, remove node
  - **Bulk Repair:** "Emergency Repair All", "Normal Repair All" with total cost shown
  - **Repair Queue:** show queued normal repairs with estimated completion ticks
  - **Maintenance Team Effect:** more maintenance teams = faster normal repair completion
  - Existing `RepairNode` and `EmergencyRepair` commands already implemented — wire them to UI
- **Files:** new `web/src/lib/panels/RepairPanel.svelte`, `web/src/lib/game/MapRenderer.ts` (damaged node indicators), `web/src/lib/game/NotificationFeed.svelte` (damage alert banner), `crates/gt-wasm/src/lib.rs` (get_damaged_nodes already exists — verify completeness)

### 9.4 Pricing panel
- Dedicated panel for regional pricing management (see 7.3):
  - Per-region price tier selection (Budget/Standard/Premium/Custom)
  - Market analysis per region: competitor pricing, customer willingness to pay, elasticity estimate
  - Revenue projection: "At Premium pricing, estimated revenue $X/tick but -Y% market share"
  - Global default + per-region overrides
- **Files:** new `web/src/lib/panels/PricingPanel.svelte`

---

## Phase 10: Full i18n & Accessibility

### 10.1 Translation infrastructure
- Expand existing `en.json` to cover ALL UI strings (currently partial)
- Create locale file structure: `web/src/lib/i18n/locales/{en,es,fr,de,ja,zh,pt,ko}.json`
- Integrate svelte-i18n or custom `$tr()` system with locale switching
- Replace ALL hardcoded English strings across all components with `$tr()` calls
- Add language selector to Settings page
- **Files:** `web/src/lib/i18n/`, all `.svelte` files with hardcoded strings

### 10.2 Number/date formatting per locale
- Use `Intl.NumberFormat` for all financial numbers (currency, percentages)
- Use `Intl.DateTimeFormat` for in-game dates
- Format large numbers with locale-appropriate separators (1,000,000 vs 1.000.000)
- **Files:** `web/src/stores/gameState.ts` (formatMoney, formatPopulation), `web/src/lib/game/HUD.svelte`

### 10.3 Colorblind mode
- Add colorblind-safe palette option in Settings:
  - Default palette (current)
  - Deuteranopia-safe (red-green)
  - Protanopia-safe
  - Tritanopia-safe (blue-yellow)
- Apply to: map region colors, overlay gradients, chart colors, status indicators (green/red for profit/loss)
- Use CSS custom properties for easy palette swapping
- **Files:** `web/src/lib/game/MapRenderer.ts` (color palettes), `web/src/stores/settings.ts`, `web/src/app.css` (CSS variables)

### 10.4 UI scaling
- Add UI scale slider in Settings: 80% / 90% / 100% / 110% / 120% / 150%
- Apply via CSS `transform: scale()` on game UI container or `font-size` base
- Ensure panels, HUD, and text remain usable at all scale levels
- **Files:** `web/src/stores/settings.ts`, `web/src/lib/game/GameView.svelte`, `web/src/app.css`

### 10.5 Keyboard navigation
- Full keyboard nav for all panels:
  - Tab through interactive elements
  - Enter/Space to activate buttons
  - Escape to close panels/dialogs
  - Arrow keys for list navigation within panels
- Focus indicators visible on all interactive elements (outline ring)
- Keyboard shortcuts documented in Settings/Help
- **Files:** all panel `.svelte` files, `web/src/lib/ui/FloatingPanel.svelte`

### 10.6 ARIA attributes & screen reader support
- Add appropriate ARIA roles to all components:
  - `role="dialog"` on panels, `role="toolbar"` on HUD, `role="status"` on notification feed
  - `aria-label` on all icon-only buttons
  - `aria-live="polite"` on notification feed (announces new events)
  - `aria-expanded` on collapsible sections
- Alt text for map elements (though deck.gl canvas is limited — add accessible summary panel)
- Screen reader announcement for game state changes (speed change, pause, era progression)
- **Files:** all `.svelte` files with interactive elements

---

## Phase 11: Interactive Charts & UI Polish

### 11.1 Interactive D3 charts
- Overhaul all 4 chart components:
  - **FinanceChart:** Revenue/cost/profit over time. Add: zoom (brush to select time range), hover shows exact values at tick, toggle individual lines on/off
  - **MarketShareChart:** Pie/donut with hover showing corp name + %. Add: click to focus on corp, historical market share line chart toggle
  - **NetworkDiagram:** Force-directed topology. Add: hover shows node details, click to center map on node, filter by tier
  - **PopulationChart:** Population over time by region. Add: stacked area chart, hover for region details, zoom time range
- All charts: responsive sizing via `ResizeObserver`, smooth transitions on data update, consistent color palette (matched to corp/region colors)
- **Files:** `web/src/lib/charts/FinanceChart.svelte`, `MarketShareChart.svelte`, `NetworkDiagram.svelte`, `PopulationChart.svelte`

### 11.2 Font consistency
- Apply `.mono` class / `var(--font-mono)` to all financial numbers across all panels
- Remove hardcoded `font-family` in `InfoPanel.svelte`
- **Files:** all panel files

### 11.3 Settings screen polish
- Style checkbox/slider controls to match dark theme (custom checkbox component)
- Add new settings: colorblind mode, UI scale, auto-pause toggle, language selector
- **File:** `web/src/lib/menu/Settings.svelte`

### 11.4 Load Game screen
- Add illustration, explanation of save locations (IndexedDB for browser, file system for desktop)
- Import save file button
- Save file metadata: date, era, tick, corp name, net worth
- **File:** `web/src/lib/menu/LoadGame.svelte`

### 11.5 Event feed SVG chevrons
- Replace ASCII "v"/"^" with proper SVG chevron icons
- **File:** `web/src/lib/game/NotificationFeed.svelte`

### 11.6 New Game form validation
- Add validation feedback: corp name min 2 chars, AI corps 0-8 range indicator, seed format hint
- **File:** `web/src/lib/menu/NewGame.svelte`

### 11.7 Keyboard shortcut hints
- Show shortcut keys on buttons: speed "1x (1)", build "Node (B)", etc.
- Full shortcut reference in Settings/Help panel
- **Files:** `web/src/lib/game/HUD.svelte`, `web/src/lib/game/SpeedControls.svelte`

### 11.8 Service worker WASM cache fix
- Use content-hashed filenames for WASM or stale-while-revalidate strategy
- **File:** `web/src/service-worker.ts`

---

## Phase 12: Production Hardening

### 12.1 Save migration system
- Add `save_version: u32` field to serialized game state
- On `load_game`:
  - Check save version against current version
  - If older: run migration chain (v1→v2→v3→...→current)
  - Each migration function handles one version bump (add default values for new fields, remove obsolete fields, transform changed types)
  - If version too old (>3 versions behind): show warning, attempt migration, allow user to confirm
- Migration functions in `crates/gt-simulation/src/save_migration.rs`
- **Files:** `crates/gt-simulation/src/world.rs` (save/load), new `crates/gt-simulation/src/save_migration.rs`

### 12.2 Structured logging (tracing crate)
- Replace all `println!`/`eprintln!` in server with `tracing` structured logging
- Add tracing spans for: WebSocket connections, command processing, tick execution, DB queries
- Log levels: ERROR (failures), WARN (degraded), INFO (connections, world events), DEBUG (tick details), TRACE (component updates)
- Add `tracing-subscriber` with JSON output for production, pretty console for dev
- **Files:** `crates/gt-server/Cargo.toml` (add tracing deps), all `gt-server/src/*.rs` files

### 12.3 Basic performance profiling
- Add tick timing measurement: record duration of each system in tick
- Expose via admin API: `/admin/perf` returns average tick time, per-system breakdown, entity count
- Add `PerfStats` struct: `tick_duration_ms`, `entity_count`, `broadcast_count`, `ws_connections`
- Frontend PerfMonitor shows: tick time, FPS, entity count, memory estimate
- Warn if tick time exceeds 50ms target
- **Files:** `crates/gt-simulation/src/world.rs` (system timing), `crates/gt-server/src/routes.rs` (perf endpoint), `web/src/lib/game/PerfMonitor.svelte`

### 12.4 Tick loop efficiency when paused
- Use `tokio::sync::Notify` to wake tick loop only on speed change
- Don't spin-wait when paused
- **File:** `crates/gt-server/src/tick.rs`

### 12.5 Better RNG for covert ops and lobbying
- Replace multiplicative hash with xorshift or SplitMix64 deterministic RNG
- **Files:** `crates/gt-simulation/src/systems/covert_ops.rs`, `lobbying.rs`

### 12.6 Broadcast subscriber leak fix
- Track forwarder task handle, cancel old one before creating new on rejoin
- **File:** `crates/gt-server/src/ws.rs`

### 12.7 Tauri desktop compatibility
- Test `cargo tauri build` after Phase 12 completion
- Verify: save/load works with file system (DesktopSaveManager.ts), WASM loads correctly, all panels functional
- Fix any Tauri-specific issues (CSP headers, file system permissions, window sizing)
- Ensure audio works in Tauri webview
- **Files:** `desktop/` (Tauri config), `web/src/lib/game/DesktopSaveManager.ts`

---

## Phase 13: Comprehensive Testing

### 13.1 Rust unit tests — per system
- Every system in `crates/gt-simulation/src/systems/` gets dedicated unit tests:
  - `construction`: build completes in correct ticks, edge construction time, era-gated building blocked
  - `maintenance`: health degradation rates by priority tier, repair costs, maintenance team effect
  - `population`: growth rate scaling per era, migration toward connected regions, employment
  - `coverage`: wireless coverage radius, signal attenuation, backhaul validation
  - `demand`: satisfaction calculation, pricing effect on demand
  - `routing`: Dijkstra correctness, dirty-node invalidation, alternate paths
  - `utilization`: traffic matrix, congestion, rerouting
  - `revenue`: traffic-based revenue, contract revenue, pricing multiplier
  - `cost`: team salaries, maintenance costs, insurance premiums
  - `finance`: debt payments, credit rating, insolvency trigger, grace period
  - `contract`: activation, expiry, breach penalty
  - `ai`: strategy selection per archetype, building decisions, merger evaluation
  - `disaster`: damage calculation, insurance payout, cascading failure
  - `regulation`: tax changes, zoning effects
  - `research`: tech completion, era prerequisite check, patent filing, license validation
  - `market`: economic health, interest rates, AI spawning conditions
  - `auction`: bid resolution, spectrum license
  - `covert_ops`: espionage success rate, intel accumulation, sabotage damage
  - `lobbying`: influence accumulation, scandal risk
  - `achievement`: unlock conditions
  - `alliance`: creation, trust scoring, dissolution
  - `legal`: lawsuit filing, resolution, damage calculation
  - `grants`: generation, bidding, completion
  - `patent`: license revenue, expiration, enforcement
- **Files:** `crates/gt-simulation/src/systems/*/tests.rs` or `#[cfg(test)] mod tests` blocks

### 13.2 Integration tests — cross-crate
- Expand existing `tests/phase2_integration.rs`:
  - Full game lifecycle: start → build → earn → research → patent → license → expand → win
  - Multiplayer simulation: 2+ corps interacting (contracts, auctions, competition, alliance)
  - AI behavior over 1000 ticks: verify AI builds, researches, bids, manages finance
  - Era progression: start in Telegraph, verify auto-progression through eras over time
  - Disaster recovery: damage nodes, verify repair, insurance payout, rerouting
  - Dynamic AI: verify new AI spawns when market is underserved
  - Save/load roundtrip: save state, load, verify identical state
  - Save migration: save with older version, load with current, verify migration
- **Files:** `crates/gt-simulation/tests/`

### 13.3 Frontend tests via Bun
- Component tests for critical UI:
  - HUD renders correctly with all data
  - Each panel opens/closes, displays data, handles empty states
  - Build menu shows only era-appropriate options
  - Tutorial progresses through all steps
  - Event formatter produces readable strings from all event types
  - Chart components render with sample data
  - Settings persist to localStorage
  - Save/load manager works with IndexedDB
- WASM bridge integration: verify `tick()`, `process_command()`, and all query methods
- **Files:** `web/src/__tests__/` or `web/tests/`

### 13.4 Server endpoint tests
- REST API tests:
  - Auth flow: register, login, refresh token, guest account
  - World CRUD: create, list, get, delete (with limit enforcement)
  - Save CRUD: upload, list, download, delete (with size limit)
  - Admin endpoints: require valid admin key
  - Leaderboard: returns correct rankings
- WebSocket tests:
  - Connection, authentication, join world, receive ticks
  - Command processing and validation
  - Chat rate limiting
  - Disconnect → AI proxy activation → reconnect → proxy summary
- **Files:** `crates/gt-server/tests/`

---

## Phase 14: Verification Plan

After each phase:

1. **Phase 1 (Security):** `cargo build --features postgres` compiles. Manual test: CORS rejects non-allowed origins. Admin endpoint rejects without valid key. WebSocket disconnects unauthenticated clients after 10s.

2. **Phase 2 (Map):** `wasm-pack build` + `bun run dev`. Visual: map shows coherent colored regions with borders. Overlays show visible contour gradients with legends. Zoom all the way out shows full world. Infrastructure nodes show SVG icons sized by tier. Backbone edges are visually prominent.

3. **Phase 3 (HUD/Panels):** Visual verification in browser. Two-row HUD readable at 1280px. All panels open as floating modals. Confirm dialog on destructive actions. Game starts paused. Auto-pause triggers on disaster.

4. **Phase 4 (Multiplayer):** Deploy to Fly.io test instance. Two tabs join same world. Both see same state. Late-joiner gets snapshot. Disconnect triggers AI proxy. Lobby shows world list with filters. Leaderboard populates.

5. **Phase 5 (Simulation):** `cargo test` passes. Event notifications are human-readable. Player starts with 1 node. Tutorial references starter node. In-game date displays. Connectivity affects GDP growth. Population migrates toward connected regions.

6. **Phase 6 (Era/Tech):** Start game → research tree shows all techs organized by era but freely explorable. Research telegraph tech → telegraph node types buildable. Patent a tech → competitors hard-blocked, must license or do independent research (150%/200% cost). Build menu categorized by tier. Force-directed D3 graph renders correctly.

7. **Phase 7 (Gameplay):** Teams hire/fire correctly. Spectrum auctions run every ~50 ticks. Pricing affects revenue + market share. Maintenance priority affects health degradation. Government grants appear and can be completed. Subsidiaries manageable. Sandbox mode: infinite money, all tech, adjustable AI. All new audio sounds trigger correctly.

8. **Phase 8 (Advanced):** Alliance formed — free routing with proportional revenue split works. Auto-suggested lawsuit filed after detected sabotage — resolves with damages. New AI corp spawns in underserved region. Two AI corps merge. Fog of war: competitor infrastructure hidden until intel obtained. Alliance members share basic intel automatically.

9. **Phase 9 (Panels):** Co-ownership panel shows shared assets. Insurance panel shows premiums. Repair panel appears after disaster with correct options. Pricing panel shows per-region controls.

10. **Phase 10 (i18n/a11y):** Switch language — all strings update. Numbers format per locale. Colorblind mode applies alternative palette. Tab navigation works through all panels. Screen reader announces game events.

11. **Phase 11 (Charts/Polish):** Finance chart has zoom/hover. Market share chart clickable. Network diagram shows topology. Settings page has all new options. Keyboard shortcuts shown on buttons.

12. **Phase 12 (Hardening):** Save with v1 format, load with v2 — migration runs. Server logs structured JSON. Admin perf endpoint shows tick times. Paused tick loop uses no CPU. Tauri desktop build works — save/load, all panels, audio functional.

13. **Phase 13 (Testing):** `cargo test` runs all system unit tests (~120-150 tests: happy path + edge cases per system). Integration tests cover full game lifecycle. `bun test` runs frontend component tests. Server tests cover all endpoints.

**Full End-to-End Acceptance Test:**
Start new game (Telegraph era) → game starts paused → player has 1 starter telegraph node → unpause, tutorial guides expansion → build more telegraph infrastructure → research telegraph tech (freely — no era gate) → patent it → competitors hard-blocked from building → license it out for revenue → research up into Telephone-era tech → new node types buildable → hire teams → set maintenance priorities → set regional pricing (Budget in poor regions, Premium in rich) → win spectrum auction → build wireless → earn revenue → connectivity grows GDP → land+labor costs rise (dampening) → population migrates → demand increases → market saturation kicks in → government grant appears → complete grant → form alliance (free routing, proportional revenue) → launch espionage on competitor → basic intel: see their infrastructure on map → detect counter-espionage → auto-suggest lawsuit → file lawsuit, win damages → new AI spawns in underserved region → two AI corps merge → disaster strikes → auto-pause with critical banner + siren sound → repair panel → emergency repair → check insurance payout → open all 6 tabbed panel groups → check interactive charts with zoom/hover → save game → load game (triggers migration if version changed) → join multiplayer → verify fog of war (can't see other player's infra without intel) → check leaderboard → switch language → enable colorblind mode → tab-navigate all panels → verify Tauri desktop build → test sandbox mode (infinite money, spawn/remove AI)

**Sandbox Acceptance Test:**
New game in sandbox mode → verify infinite cash → all tech unlocked → spawn AI corp on demand → test alliance/auction/legal systems → trigger disaster manually → verify 32x speed → confirm sandbox saves are marked

---

## Summary: Phase Count & Scope

| Phase | Name | Items | Scope |
|-------|------|-------|-------|
| 1 | Security Hardening | 7 | CORS, auth, validation, rate limits |
| 2 | Map Rendering | 9 | Cell fill, borders, overlays, SVG icons, tier viz, mini-map |
| 3 | HUD & Panels | 11 | Two-row HUD, tabbed panel groups (6 groups), start-paused, auto-pause |
| 4 | Multiplayer & Lobby | 15 | Fixes, snapshots, thin client, fog-of-war-aware, lobby overhaul |
| 5 | Core Simulation Fixes | 14 | Events (priority+category), starter node, tutorial, dates, connectivity+dampening, world era milestone |
| 6 | Era & Tech Overhaul | 3 | Era-specific types (flat enum ~33), freely-explorable research, patent/license/lease system, D3 force graph |
| 7 | New Gameplay Systems | 8 | Teams, auctions, pricing, maintenance, grants, subsidiaries, sandbox mode, full audio |
| 8 | Advanced Gameplay | 4 | Alliance (free routing + revenue share), legal (auto-suggest), dynamic AI, fog of war |
| 9 | New Frontend Panels | 4 | Co-ownership, insurance, repair, pricing panels (within tabbed groups) |
| 10 | i18n & Accessibility | 6 | Full translation, colorblind, UI scale, keyboard nav, ARIA |
| 11 | Charts & UI Polish | 8 | Interactive charts, font, settings, shortcuts |
| 12 | Production Hardening | 7 | Save migration, tracing, profiling, efficiency, Tauri compat |
| 13 | Comprehensive Testing | 4 | ~120-150 tests: happy path + edge cases per system |
| 14 | Verification | — | Per-phase + full E2E + sandbox acceptance tests |

**Total: 100 items across 14 phases (strict sequential execution)**
