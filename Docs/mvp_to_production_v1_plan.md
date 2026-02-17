# GlobalTelco: Development Phase Plan

Comprehensive phased implementation plan from scratch to shippable production v1. Each phase builds on the previous. Tech stack: Rust (simulation ECS → WASM + native), Svelte + Three.js + D3.js (frontend), Bun (build), Tauri (desktop), PostgreSQL + Hetzner + Cloudflare (backend).

---

## Phase 1: Rust Workspace & Core ECS

*Goal: The Rust simulation compiles, creates a world, and ticks. No rendering yet — just the engine.*

### Workspace Setup
- [x] Initialize Cargo workspace with all 9 crates (gt-common, gt-simulation, gt-world, gt-economy, gt-infrastructure, gt-population, gt-ai, gt-wasm, gt-server)
- [x] Set up workspace-level dependencies (serde, bincode, rand, noise, wasm-bindgen)
- [x] Configure `cargo test` for all crates
- [x] Set up CI (GitHub Actions: build, test, clippy, fmt)

### gt-common — Shared Types
- [x] Core type definitions: EntityId, Tick, WorldConfig, TerrainType, NodeType, EdgeType, CreditRating, AIArchetype, AIStrategy
- [x] GameEvent enum (all event types)
- [x] Command enum (all player action types)
- [x] Serialization traits (serde Serialize/Deserialize on all types)
- [x] Configuration structs: DifficultyPreset, EraConfig

### gt-simulation — ECS Engine
- [x] Entity allocator (monotonic u64 IDs, free list for recycling)
- [x] Component storage (struct-of-arrays per component type, indexed by EntityId)
- [x] GameWorld struct: owns all storage, tick counter, event queue, config
- [x] `GameWorld::tick()` — runs all systems in deterministic order
- [x] `GameWorld::process_command()` — validates and applies player commands
- [x] Event queue: append-only, drained each tick
- [x] All 15 systems fully implemented (construction, maintenance, population, demand, routing, utilization, revenue, cost, finance, contract, ai, disaster, regulation, research, market)

### gt-world — World Generation
- [x] 3D sphere-based fractal noise for terrain elevation
- [x] Icosahedral geodesic grid subdivision (configurable resolution)
- [x] Spatial hash for O(1) coordinate-to-cell lookup
- [x] Terrain classification from elevation (Urban, Suburban, Rural, Mountainous, Desert, Coastal, Ocean, Tundra, Frozen)
- [x] Land parcel entity creation with terrain, zoning, cost modifiers
- [x] K-means region clustering (land-aware seeding)
- [x] City placement from terrain and region data
- [x] Economic data seeding per region (GDP, population, demand)
- [x] Real Earth data loader (48 countries, 71 cities with real GDP/population data, embedded in `data/earth.json`)
- [x] All deterministic from world seed

### Verification
- [x] `cargo build` succeeds for all crates (0 errors, 0 warnings)
- [x] `cargo test` — 34 tests pass: world gen, determinism, ECS, network graph, protocol, real earth
- [x] Determinism test: same seed → same world state after N ticks
- [x] `cargo clippy -- -D warnings` passes clean
- [x] `cargo fmt --check` passes clean

---

## Phase 2: Economy, Infrastructure & AI Foundations

*Goal: Corporations exist, own things, earn money, and AI makes decisions. Still no rendering.*

### gt-economy — Corporate Finance
- [x] Corporation entity with CorporationData component (name, cash, debt, credit rating, is_ai)
- [x] Create/destroy corporation functions
- [x] Balance sheet tracking (assets, liabilities, equity)
- [x] Income statement per tick (revenue, COGS, operating expenses, interest, net income)
- [x] Credit rating calculation (based on debt ratio, cash flow, history)
- [x] Debt instruments (individual loans with principal, rate, maturity)
- [ ] Subsidiary system (child corporations with parent reference)
- [x] Contract system (peering, transit, SLA — terms, capacity, penalties, renewal)

### gt-infrastructure — Network Graph
- [x] Infrastructure node entities (6 types: tower, fiber hub, data center, IXP, subsea station, satellite station)
- [x] Infrastructure edge entities (6 types: fiber local/regional/national, microwave, subsea, satellite)
- [x] 5-level hierarchical network graph (Local → Regional → National → Continental → Global Backbone)
- [x] Dijkstra routing with cached shortest-path trees
- [x] Dirty-node invalidation (only recalculate affected clusters)
- [x] Node/edge attributes: capacity, latency, reliability, maintenance cost, health, construction state
- [x] Terrain multipliers on construction cost, maintenance cost, reliability

### gt-population — Demographics
- [x] City entities with population count, growth rate, employment, migration pressure
- [x] Birth/death rate modeling per tick
- [x] Migration system: population moves toward cities with better infrastructure/jobs
- [x] Employment tracking: infrastructure creates jobs, migration fills them
- [x] Demand calculation from population × economic development × infrastructure quality

### gt-ai — AI Corporations
- [x] 4 archetype definitions with weight tuning
- [x] Strategy selection (Expand/Consolidate/Compete/Survive) based on financial health + archetype
- [x] AI actions: acquire land, build node, build edge, manage finances, propose contract
- [x] Parcel scoring algorithm (terrain, demand, proximity, cost, weighted by archetype)
- [ ] AI proxy for offline multiplayer (policy-only execution)

### System Implementation (gt-simulation)
- [x] Fully implement all 15 systems (not stubs):
  - construction, maintenance, population, demand, routing, utilization, revenue, cost, finance, contract, ai, disaster, regulation, research, market

### Verification
- [x] Create a world with 1 player corp + 4 AI corps → tick 500 times → AI corps build infrastructure, earn revenue, some grow, some struggle
- [x] Revenue flows correctly: infrastructure → utilization → revenue → corporation cash
- [x] Maintenance costs deduct properly, infrastructure degrades without maintenance
- [x] AI strategy switches based on financial state (expand when rich, survive when poor)
- [x] Contracts form between AI corps

---

## Phase 3: WASM Bridge & Minimal Frontend

*Goal: The game runs in a browser. Player sees a map, can interact, and the simulation runs in WASM.*

### gt-wasm — WASM Bindings
- [x] wasm-bindgen entry point
- [x] `new_game(config_json: &str) -> *mut GameWorld` — create and initialize world
- [x] `tick(world: *mut GameWorld, dt: f64)` — advance simulation
- [x] Command functions: `build_node()`, `build_edge()`, `hire_employee()`, `set_policy()`, `take_loan()`, `set_speed()`, `toggle_pause()`
- [x] Query functions: `get_visible_entities()`, `get_corporation_data()`, `get_region_data()`, `get_infrastructure_list()`, `get_notifications()`
- [x] `save_game()` → returns compressed bytes; `load_game(data)` → restores world
- [x] Build with `wasm-pack build --target web`

### Svelte App Bootstrap
- [x] Initialize Svelte project with Bun (`bun create svelte`)
- [x] Configure TypeScript, dark theme CSS, font setup (sans-serif + monospace)
- [x] WASM loading and initialization (`web/src/lib/wasm/bridge.ts`)
- [x] TypeScript command/query wrappers (`commands.ts`, `queries.ts`)
- [x] Svelte stores: game state (from WASM queries), UI state (active panel, selection), settings
- [x] Game loop: `requestAnimationFrame` → tick WASM → query state → update stores → render

### Three.js Map Renderer
- [x] Three.js scene setup in `MapRenderer.svelte` (orthographic camera for 2D mode)
- [x] Layer 1: Ocean base (dark blue plane)
- [x] Layer 2: Land masses (terrain-colored polygons from GeoJSON or proc-gen data)
- [x] Layer 3: Political borders (line geometry for country/region borders)
- [x] Layer 4: City dots (scaled circles for population centers)
- [x] Layer 5: Infrastructure icons (sprites for nodes, line geometry for edges)
- [x] Layer 6: Ownership overlay (semi-transparent company-colored regions)
- [x] Layer 7: Selection highlight (glow on hovered/selected entities)
- [x] Layer 8: Labels (text sprites for city/region names at appropriate zoom)
- [x] Zoom level visibility control (World/Country/Region/City)
- [x] Pan and zoom with mouse/touch
- [x] Click-to-select entities (raycast to find nearest entity)

### Minimal UI Panels
- [x] `MainMenu.svelte` — New Game, Load Game, Settings, Quit
- [x] `NewGame.svelte` — Corp name, world type, era, difficulty, AI count, disaster severity, seed, Start button
- [x] `GameView.svelte` — Main game screen container (map + HUD + panels)
- [x] `HUD.svelte` — Top bar: corporation name, cash, tick counter, speed display
- [x] `SpeedControls.svelte` — Pause/Play/2x/4x/8x buttons + quick save/load
- [x] Basic panel system: click buttons to open/close side panels

### Verification
- [x] `bun run dev` → browser opens → main menu renders
- [x] New Game → world generates → map renders with terrain, borders, cities
- [x] Pan/zoom works smoothly at 60fps
- [x] Speed controls: pause/resume/2x/4x
- [x] Click a parcel → selection highlight appears
- [x] HUD shows live cash and tick counter updating
- [x] AI corps build infrastructure visible on map over time

---

## Phase 4: Player Build UX & Financial Management

*Goal: The player can build infrastructure, manage finances, and make strategic decisions.*

### Build Interaction
- [x] Build mode toggle (keyboard shortcut or button)
- [x] Click parcel in build mode → show build menu (available node types with costs and construction time)
- [x] Select node type → place on parcel (starts construction, deducts cost)
- [x] Edge creation: select source node → select target node → choose edge type → confirm
- [x] Visual feedback: ghost/preview before confirming, construction-in-progress indicator
- [x] Validation: parcel ownership, zoning compatibility, sufficient funds, no duplicates

### Management Panels
- [x] `DashboardPanel.svelte` — Financial overview: cash, revenue, expenses, net income, debt, credit rating (charts over time using D3)
- [x] `InfraPanel.svelte` — Owned infrastructure list: status, revenue contribution, maintenance cost, upgrade options
- [ ] `WorkforcePanel.svelte` — Employee/team management (deferred — workforce is auto-managed for now)
- [x] `ContractPanel.svelte` — Active contracts, pending proposals, propose new contracts
- [x] `RegionPanel.svelte` — Regional overview: demand, population, competitor presence, market share
- [x] `BuildMenu.svelte` — Context menu for build placement (node type selection)

### Financial Actions
- [x] Take loan (choose amount, see interest rate based on credit rating)
- [x] Repay debt (select instrument to pay down)
- [x] Income statement breakdown (revenue by source, expenses by category)
- [x] Balance sheet view

### D3.js Charts
- [x] `FinanceChart.svelte` — Revenue/expense line chart over time
- [ ] `PopulationChart.svelte` — Population growth graph (deferred)
- [ ] `NetworkDiagram.svelte` — Network topology visualization (deferred)
- [x] `MarketShare.svelte` — Market share pie/bar chart

### Verification
- [x] Build mode → click hex → build menu → place tower → construction timer → completes → operational → earning revenue
- [x] Select two nodes → lay fiber edge → traffic routes through → utilization visible
- [x] Finance panel → take loan → cash increases → interest accrues
- [x] D3 charts update live as game progresses

---

## Phase 5: Save/Load & Game Persistence

*Goal: Players can save, load, and never lose progress.*

### Save System
- [ ] WASM serialize/deserialize (bincode + zstd compression)
- [ ] IndexedDB storage on JS side (idb library)
- [ ] Save slot management: create, list, delete, rename
- [ ] Save metadata: name, timestamp, tick count, difficulty, corporation name
- [ ] Auto-save every 50 ticks (rotating 3 slots: AutoSave1/2/3)
- [ ] Quick save (F5) / quick load (F9) keyboard shortcuts
- [ ] Save file version header for forward-compatibility

### Load Game UI
- [ ] `LoadGame.svelte` — Save slot list with metadata, load/delete buttons
- [ ] Confirmation dialog for overwriting saves
- [ ] Loading screen while world deserializes

### Verification
- [ ] Save → close browser → reopen → load → exact same state
- [ ] Auto-save slots rotate correctly
- [ ] Delete save works
- [ ] Save version check prevents loading incompatible saves

---

## Phase 6: Disaster System & World Events

*Goal: The world is dynamic and dangerous. Disasters create strategic challenges.*

### Disaster Generation
- [ ] Disaster probability rolls per tick, per region (based on terrain disaster risk profile)
- [ ] Configurable severity from WorldConfig (1-10 slider)
- [ ] Disaster types: earthquake, hurricane, flooding, landslide, volcanic, political unrest, regulatory crackdown, cyber attack
- [ ] Each type: affected radius, severity range, duration, primary terrain targets

### Disaster Effects
- [ ] Damage infrastructure in affected area (degrade or destroy nodes/edges based on severity)
- [ ] Cascade effects: destroyed hub disconnects downstream nodes
- [ ] Political events: temporary regulatory changes (increased taxes, operating restrictions)
- [ ] Population displacement: disasters in cities reduce population temporarily

### Repair System
- [ ] Degraded infrastructure can be repaired (costs money, takes ticks)
- [ ] Destroyed infrastructure must be rebuilt from scratch
- [ ] Emergency repair option (instant, very expensive)
- [ ] Insurance system: per-asset premium, covers partial rebuild cost

### Notification & Visualization
- [ ] `NotificationFeed.svelte` — Event feed with severity icons
- [ ] Disaster overlay on map (colored highlight on affected hexes)
- [ ] Notification click → jump to disaster location on map

### Verification
- [ ] High disaster severity → frequent disasters → infrastructure damage → repair costs → financial pressure
- [ ] Insurance reduces financial impact
- [ ] Notifications appear with correct info and location links

---

## Phase 7: Technology & Research

*Goal: R&D unlocks competitive advantages. Patent system creates economic depth.*

### Tech Tree
- [ ] Define 6 categories: Optical Networks, Wireless/5G, Satellite, Data Center, Network Resilience, Operational Efficiency
- [ ] 5-8 techs per category (30-48 total), each with: name, cost, prerequisites, unlock effects
- [ ] Tech definitions as code-defined data (Rust structs, no external assets)

### Research System
- [ ] R&D budget allocation per corporation (per-tick spending)
- [ ] Research progress accumulates based on budget
- [ ] One active research per corporation
- [ ] Research completion fires event, applies bonuses

### Patent & Licensing
- [ ] Completed research becomes a patent owned by researching corp
- [ ] Patent options: patent it (costs legal fees, earns licensing revenue), open-source it (free for all, goodwill), keep proprietary (only your corp uses it)
- [ ] License negotiation: set price, other corps accept/decline
- [ ] AI corps evaluate license offers based on archetype

### Research UI
- [ ] `ResearchPanel.svelte` — Tech tree display with category tabs
- [ ] Show: available techs, prerequisites, progress, cost, effects
- [ ] R&D budget slider
- [ ] Patent management: view owned patents, set license prices, view incoming licenses

### Verification
- [ ] Set R&D budget → select tech → progress advances → tech unlocks → bonuses applied
- [ ] Patent a tech → AI corp licenses it → royalty income flows
- [ ] AI corps research techs based on archetype preference

---

## Phase 8: Complete UI & Polish

*Goal: Every piece of information the player needs is accessible through polished UI.*

### Map Overlays
- [ ] Overlay toggle system (buttons in HUD)
- [ ] Terrain type coloring
- [ ] Ownership coloring (by corporation)
- [ ] Regional demand heatmap
- [ ] Disaster risk heatmap
- [ ] Network coverage
- [ ] Congestion heatmap (green/yellow/red on edges)

### Tooltip System
- [ ] Hover hex → terrain, owner, zoning, demand
- [ ] Hover node → type, status, capacity, utilization, owner
- [ ] Hover edge → type, bandwidth, latency, utilization
- [ ] Hover corp name → cash, credit rating, node count

### News & Advisor
- [ ] `NotificationFeed.svelte` — filterable event feed (infra, financial, competitor, disaster, contract)
- [ ] Notification urgency levels (info, warning, critical)
- [ ] `AdvisorPanel.svelte` — AI advisor suggests actions and explains why
- [ ] Advisor suggestions based on game state (build here, expand there, take a loan, repair this)

### Settings
- [ ] `Settings.svelte` — game settings panel
- [ ] Graphics settings (render quality, zoom sensitivity)
- [ ] Audio settings (volume sliders)
- [ ] Gameplay settings (auto-save frequency, notification preferences)
- [ ] Keyboard shortcut reference

### Visual Polish
- [ ] Consistent dark theme across all panels (navy/charcoal base, Bloomberg Terminal feel)
- [ ] Color palette: green=profit, red=loss, blue=neutral, amber=warning
- [ ] Corporation brand colors on map
- [ ] Panel animations (slide-in/out, hover states)
- [ ] Loading indicators during world gen and save/load

### Verification
- [ ] All overlays toggle and show accurate data
- [ ] Tooltips show correct live data on hover
- [ ] Advisor gives relevant suggestions
- [ ] Settings save and persist

---

## Phase 9: Audio & Content Polish

*Goal: The game feels like a real game, not a tech demo.*

### Audio
- [ ] Ambient music: 3-5 gameplay tracks (calm strategic mood, varying intensity)
- [ ] Main menu music: 1 track
- [ ] UI sound effects: button clicks, panel open/close, notification chimes, build placement
- [ ] Disaster sounds: rumble, wind, alerts
- [ ] Construction completion sound, revenue chime
- [ ] Dynamic audio: slightly more tense during crises
- [ ] Volume controls, mute toggle

### Visual Content
- [ ] Infrastructure icon set (realistic miniatures for all 6 node types)
- [ ] Edge visualization (distinct styles for fiber, microwave, subsea, satellite)
- [ ] Map visual polish (night-earth city lights effect at world zoom, terrain detail at region zoom)
- [ ] Company logo/badge system (players choose from preset logos + custom color)

### Tutorial
- [ ] Tutorial sequence: camera controls → select a parcel → buy parcel → build tower → connect with fiber → view revenue → take loan → set R&D → pause/speed → save
- [ ] Tutorial hint system (highlights, arrows, text boxes pointing at UI elements)
- [ ] Skippable
- [ ] Only triggers on first new game

### Verification
- [ ] Audio plays correctly with volume controls
- [ ] Icons are distinct and readable at all zoom levels
- [ ] Tutorial guides a new player through all core mechanics

---

## Phase 10: Advanced Gameplay

*Goal: Competitive depth that makes the game strategically interesting beyond build-and-earn.*

### Bankruptcy & Auctions
- [ ] Detect insolvency (negative cash + maxed debt + no sellable assets)
- [ ] Player choice: government bailout (high-interest emergency loan) or declare bankruptcy (assets liquidated)
- [ ] AI auto-choose based on archetype
- [ ] Bankrupt corp's assets go to sealed-bid auction
- [ ] Government auctions for unclaimed premium parcels (periodic events)

### Hostile Takeover & Mergers
- [ ] Propose acquisition (offer price based on equity)
- [ ] Target accepts or rejects (AI evaluates offer vs book value)
- [ ] Successful acquisition: absorb all assets, debt, contracts
- [ ] Merger: two corps combine (AI-only for v1)

### Sabotage & Espionage
- [ ] Espionage: spend money to reveal competitor's stats in a region
- [ ] Sabotage: spend money to temporarily degrade competitor's infrastructure (risk of detection)
- [ ] Counter-espionage: invest in security to reduce vulnerability
- [ ] Detection: caught sabotage → lawsuit, financial penalty, reputation damage

### Lobbying & Political Influence
- [ ] Lobby for favorable regulation (reduced taxes, relaxed zoning, fast-track permits)
- [ ] Lobby against competitors (increased regulatory burden)
- [ ] Diminishing returns, potential scandal backfire
- [ ] AI lobbying behavior based on archetype

### Cooperative Infrastructure
- [ ] Multi-owner nodes and edges (shared revenue proportional to ownership stake)
- [ ] Upgrade voting (majority approval required)
- [ ] Buyout offers between co-owners

### Achievements & Win Conditions
- [ ] Achievement system (first international cable, first profitable quarter, survive 3 disasters, etc.)
- [ ] Optional SP victory conditions (dominate X% of global traffic, reach $X net worth, AAA credit rating)
- [ ] End-game summary screen with stats and timeline

### Verification
- [ ] Bankruptcy → bailout or restart flow works
- [ ] Acquisition → all assets transfer correctly
- [ ] Espionage reveals data, sabotage degrades infra
- [ ] Achievements trigger correctly

---

## Phase 11: Multiplayer Server

*Goal: Multiple players connect to a persistent world and play together.*

### gt-server — Multiplayer Binary
- [ ] Rust Axum WebSocket server
- [ ] Same `gt-simulation` crate, compiled natively (not WASM)
- [ ] World management: create, load, tick, save worlds
- [ ] WebSocket protocol: MessagePack binary serialization
- [ ] Client → Server: commands (build, hire, set policy, etc.)
- [ ] Server → Client: tick deltas, acks, events, full snapshots on connect
- [ ] Server-authoritative validation on all commands
- [ ] Rate limiting on player actions

### Authentication & Accounts
- [ ] Cloudflare Workers auth service (register, login, session tokens)
- [ ] Session token validation on WebSocket connect
- [ ] Player profile: display name, corporation history

### World Persistence
- [ ] PostgreSQL schema: worlds, accounts, player_worlds, cloud_saves
- [ ] Periodic world state save to PostgreSQL (every N ticks)
- [ ] World restore from DB on server startup
- [ ] Player corporation persists between sessions

### AI Proxy
- [ ] Player disconnect → AI proxy activates (policy-only execution)
- [ ] Player reconnect → AI proxy deactivates, summary of actions while away
- [ ] No strategic changes while proxied

### Multiplayer UI
- [ ] `WorldBrowser.svelte` — list available worlds (name, player count, age, ping)
- [ ] Connect button, create world button
- [ ] Chat system: global, regional, alliance, direct message channels
- [ ] Chat UI with channel tabs

### Deployment
- [ ] Docker container for gt-server
- [ ] Deploy to Hetzner Cloud (dev: CX22, prod: AX42)
- [ ] Cloudflare Workers for auth endpoints
- [ ] Vercel deployment for Svelte frontend

### Verification
- [ ] Two browsers connect to same server → both see same world
- [ ] Player A builds → Player B sees it
- [ ] Player disconnects → AI proxy manages corp → player reconnects → gets summary
- [ ] Server restarts → world state fully restored from DB
- [ ] Chat works across players

---

## Phase 12: Desktop App & Distribution

*Goal: The game is downloadable and distributable.*

### Tauri Desktop App
- [ ] Initialize Tauri project in `desktop/`
- [ ] Configure to load the Svelte frontend via system webview
- [ ] Bundle WASM module for offline single-player
- [ ] File system access for local saves (alternative to IndexedDB)
- [ ] Auto-update mechanism (check version on launch)
- [ ] Build for macOS, Windows, Linux

### Web Distribution
- [ ] Production Svelte build on Vercel
- [ ] CDN caching for WASM module and static assets
- [ ] Service worker for offline capability (cache WASM + assets)
- [ ] Progressive Web App (PWA) manifest for "install" option

### Platform Integration (future)
- [ ] Steam integration (Steamworks via Tauri plugin — auth, achievements, cloud saves)
- [ ] itch.io distribution

### Verification
- [ ] Tauri app launches, plays offline single-player, saves locally
- [ ] Web version loads quickly (< 3s on broadband)
- [ ] PWA installs and works offline

---

## Phase 13: Localization & Accessibility

*Goal: The game is localization-ready and accessible.*

### Localization
- [ ] Extract all user-facing strings to translation files (JSON or YAML per locale)
- [ ] Svelte i18n integration (dynamic string loading)
- [ ] Number/date formatting per locale
- [ ] Rust-side strings (entity names, event descriptions) localization-ready
- [ ] Initial language: English. Structure supports adding more.

### Accessibility
- [ ] Colorblind-friendly mode: alternative color schemes for overlays and corp colors
- [ ] UI scaling option (text/UI size slider)
- [ ] Keyboard navigation for all menus (tab, enter, escape)
- [ ] ARIA attributes on interactive elements
- [ ] Screen reader compatibility for key information

### Verification
- [ ] All strings come from translation files (no hardcoded text in components)
- [ ] Colorblind mode is visually distinct
- [ ] All menus navigable by keyboard

---

## Phase 14: Production Hardening & Launch

*Goal: Stable, performant, ready for real players.*

### Performance
- [ ] Profile simulation tick (target: < 50ms for 10,000+ entities)
- [ ] Profile Three.js rendering (target: 60fps with 100,000+ visible entities)
- [ ] Profile WASM module size (target: < 5MB gzipped)
- [ ] Memory profiling (target: < 500MB in browser for large worlds)
- [ ] Optimize network graph routing (target: < 10ms per Dijkstra query)
- [ ] WebSocket latency (target: < 100ms round-trip)

### Stability
- [ ] Graceful error handling throughout (no panics in WASM)
- [ ] Save file corruption recovery (detect, offer delete or attempt recovery)
- [ ] Network disconnection handling (reconnect prompt, local state preservation)
- [ ] Save migration system (version-based upgrades for older saves)
- [ ] Structured logging in Rust (tracing crate)

### Anti-Cheat (Multiplayer)
- [ ] Server-authoritative validation on all actions
- [ ] Rate limiting
- [ ] Sanity checks on client data
- [ ] Admin tools: kick, ban, view stats

### QA
- [ ] Full SP playthrough on each difficulty → 100+ ticks → all systems function
- [ ] Save/load cycle at various game states → state matches
- [ ] Multiplayer: 2+ players, 50+ ticks, sync verified
- [ ] Edge cases: 0 AI, 10 AI, seed 0, max seed
- [ ] Disaster stress test (severity 10)
- [ ] Financial stress test (force bankruptcy)
- [ ] Performance test: 10k+ entities, 10 AI corps, 4x speed, no frame drops

### Launch Prep
- [ ] Loading screen with tips during world gen
- [ ] Credits screen
- [ ] Version number in main menu
- [ ] Splash screen on launch

### Verification
- [ ] Game runs stable for 1000+ ticks with no crashes
- [ ] Memory stays bounded over long sessions
- [ ] Fresh install → complete new-player flow → everything works
- [ ] Hand to someone new → they can learn via tutorial

---

## Milestone Summary

| Milestone | Phases | What It Means |
|-----------|--------|---------------|
| **Engine Working** | 1-2 | Rust sim compiles, creates worlds, AI plays, economy flows (no UI) |
| **Playable in Browser** | 1-3 | WASM runs in browser, map renders, basic interaction works |
| **Core Gameplay** | 1-5 | Player can build, manage finances, save/load — it's a game |
| **Feature-Complete SP** | 1-8 | All single-player systems with full UI, overlays, advisor |
| **Content-Complete SP** | 1-10 | Audio, polish, tutorial, advanced gameplay, achievements |
| **Online-Ready** | 1-11 | Persistent multiplayer with accounts, chat, AI proxy |
| **Ship-Ready** | 1-13 | Desktop app, distribution, localization, accessibility |
| **Production v1** | 1-14 | Hardened, tested, polished, launched |
