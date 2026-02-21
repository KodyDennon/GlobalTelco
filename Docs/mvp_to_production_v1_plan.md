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
- [x] Core type definitions: EntityId, Tick, WorldConfig, TerrainType, NodeType (~33 types across eras), EdgeType (~15 types across eras), CreditRating, AIArchetype, AIStrategy
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
- [x] 20 core systems fully implemented in deterministic tick order:
  1. construction → 2. maintenance → 3. population → 4. coverage → 5. demand → 6. routing → 7. utilization → 8. revenue → 9. cost → 10. finance → 11. contract → 12. ai → 13. disaster → 14. regulation → 15. research → 16. market → 17. auction → 18. covert_ops → 19. lobbying → 20. achievement
- [ ] Planned additional systems: alliance, legal, grants, fog_of_war, pricing, maintenance_scheduling

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
- [x] Subsidiary system (child corporations with parent reference)
- [x] Contract system (peering, transit, SLA — terms, capacity, penalties, renewal)

### gt-infrastructure — Network Graph
- [x] Infrastructure node entities (~33 types across eras, flat expansion — e.g., telegraph office, telephone exchange, cell tower, fiber hub, data center, IXP, subsea station, satellite station, 5G small cell, edge compute node, etc.)
- [x] Infrastructure edge entities (~15 types across eras, flat expansion — e.g., telegraph wire, copper loop, fiber local/regional/national, microwave, subsea, satellite, 5G mmWave, mesh wireless, etc.)
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
- [x] AI proxy for offline multiplayer (policy-only execution)

### System Implementation (gt-simulation)
- [x] Fully implement all 20 systems (not stubs):
  - construction, maintenance, population, coverage, demand, routing, utilization, revenue, cost, finance, contract, ai, disaster, regulation, research, market, auction, covert_ops, lobbying, achievement

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

### Management Panels (6 Tabbed Groups)
- [x] Panel architecture uses 6 tabbed groups for organized access:
  - **Finance** — Dashboard, pricing strategy, insurance
  - **Operations** — Infrastructure, maintenance, repair queue, workforce, build menu
  - **Diplomacy** — Alliances, legal actions, intel/espionage, co-ownership
  - **Research** — Tech tree, patents/licensing
  - **Market** — Contracts, auctions, mergers & acquisitions, government grants, subsidiaries
  - **Info** — Region overview, advisor, achievements
- [x] `DashboardPanel.svelte` — Financial overview: cash, revenue, expenses, net income, debt, credit rating (charts over time using D3)
- [x] `InfraPanel.svelte` — Owned infrastructure list: status, revenue contribution, maintenance cost, upgrade options
- [x] `WorkforcePanel.svelte` — Employee/team management (hire/fire, morale, workforce impact bars)
- [x] `ContractPanel.svelte` — Active contracts, pending proposals, propose new contracts
- [x] `RegionPanel.svelte` — Regional overview: demand, population, competitor presence, market share
- [x] `BuildMenu.svelte` — Context menu for build placement (node types categorized by tier)

### Financial Actions
- [x] Take loan (choose amount, see interest rate based on credit rating)
- [x] Repay debt (select instrument to pay down)
- [x] Income statement breakdown (revenue by source, expenses by category)
- [x] Balance sheet view

### D3.js Charts
- [x] `FinanceChart.svelte` — Revenue/expense line chart over time
- [x] `PopulationChart.svelte` — Population bar chart (top 10 cities by population, embedded in RegionPanel)
- [x] `NetworkDiagram.svelte` — D3.js force-directed network topology visualization (embedded in InfraPanel)
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
- [x] WASM serialize/deserialize (bincode + zstd compression)
- [x] IndexedDB storage on JS side (idb library)
- [x] Save slot management: create, list, delete, rename
- [x] Save metadata: name, timestamp, tick count, difficulty, corporation name
- [x] Auto-save every 50 ticks (rotating 3 slots: AutoSave1/2/3)
- [x] Quick save (F5) / quick load (F9) keyboard shortcuts
- [x] Save file version header for forward-compatibility

### Load Game UI
- [x] `LoadGame.svelte` — Save slot list with metadata, load/delete buttons
- [x] Confirmation dialog for overwriting saves
- [x] Loading screen while world deserializes

### Verification
- [x] Save → close browser → reopen → load → exact same state
- [x] Auto-save slots rotate correctly
- [x] Delete save works
- [x] Save version check prevents loading incompatible saves

---

## Phase 6: Disaster System & World Events

*Goal: The world is dynamic and dangerous. Disasters create strategic challenges.*

### Disaster Generation
- [x] Disaster probability rolls per tick, per region (based on terrain disaster risk profile)
- [x] Configurable severity from WorldConfig (1-10 slider)
- [x] Disaster types: earthquake, hurricane, flooding, landslide, cyber attack, political unrest, regulatory change, equipment failure
- [x] Each type: weighted probability, severity range, primary terrain targets

### Disaster Effects
- [x] Damage infrastructure in affected area (degrade health based on severity)
- [x] Severe damage: health < 0.2 → capacity reduced to 10%
- [x] Political events: temporary regulatory changes (via regulation system)
- [x] Population displacement: disasters with severity > 0.3 displace 5% population

### Repair System
- [x] Degraded infrastructure can be repaired (RepairNode command — 20% base cost, restores 50% health)
- [x] Emergency repair option (EmergencyRepair command — 60% base cost, instant full restore)
- [x] Insurance system: per-node premium (2% of construction cost), 60% payout on disaster damage, PurchaseInsurance/CancelInsurance commands

### Notification & Visualization
- [x] `NotificationFeed.svelte` — Event feed with category colors (disaster, infra, finance, contract, research, market)
- [x] Notification urgency via event categories
- [x] Expandable feed (3 recent → 20 expanded)

### Verification
- [x] High disaster severity → frequent disasters → infrastructure damage → repair costs → financial pressure
- [x] Notifications appear with correct info and category colors

---

## Phase 7: Technology & Research

*Goal: R&D unlocks competitive advantages. Patent system creates economic depth.*

### Tech Tree
- [x] Define 6 categories: Optical Networks, Wireless/5G, Satellite, Data Center, Network Resilience, Operational Efficiency
- [x] 6 techs per category (36 total), each with: name, description, cost, prerequisites, unlock effects (throughput/cost/reliability bonuses)
- [x] Tech definitions as code-defined data (Rust structs via `generate_tech_tree()`, no external assets)
- [x] Freely-explorable tree: techs are gated by prerequisites only, NOT by era. Players can research any tech they meet the prereqs for regardless of current era.
- [x] Tech as economic commodity: completed research has market value, can be licensed, traded, or open-sourced for strategic advantage.

### Research System
- [x] R&D budget allocation per corporation (per-tick spending)
- [x] Research progress accumulates based on budget
- [x] One active research per corporation
- [x] Research completion fires event, applies bonuses (throughput, cost reduction, reliability)

### Patent & Licensing
- [x] Completed research becomes a patent owned by researching corp
- [x] Patent options: PatentStatus enum (None, Patented, OpenSourced, Proprietary)
- [x] License price field, licensed_to tracking
- [x] Patent owner and license info exposed via WASM bridge
- [x] Hard block enforcement: patented tech cannot be used by non-holders without a license. Attempting to build patent-protected infrastructure without a license is rejected.
- [x] Independent research workaround: corps can independently research a patented tech at 150% cost (standard), or 200% cost (rush), bypassing the patent holder entirely.
- [x] License types: Exclusive (one licensee, premium price), Non-Exclusive (multiple licensees, standard price), Open Source (free to all, reputation bonus to holder).

### Research UI
- [x] `ResearchPanel.svelte` — Tech tree display with category tabs (All + 6 categories)
- [x] Show: available techs, prerequisites, progress, cost, bonuses (throughput/cost/reliability)
- [x] R&D budget slider (0-5M)
- [x] Active research progress bar, completed tech count

### Verification
- [x] Set R&D budget → select tech → progress advances → tech unlocks → bonuses applied
- [x] AI corps research techs based on strategy
- [x] Patent status tracked per technology

---

## Phase 8: Complete UI & Polish

*Goal: Every piece of information the player needs is accessible through polished UI.*

### Map Overlays
- [x] Overlay toggle system (T/O/D/C buttons in HUD)
- [x] Terrain type coloring (enhanced terrain colors overlay)
- [x] Ownership coloring (expanded corp territory circles)
- [x] Regional demand heatmap (population-density gradient blue→red)
- [x] Network coverage (green radius around operational nodes)
- [x] Disaster risk heatmap (region-level, green→yellow→red by disaster_risk)
- [x] Congestion heatmap (per-node, green→red by utilization, radius scales with congestion)

### Tooltip System
- [x] `Tooltip.svelte` — Tooltip component wired to uiState store
- [x] Tooltip follows mouse position with dark theme styling

### News & Advisor
- [x] `NotificationFeed.svelte` — categorized event feed (disaster, infra, finance, contract, research, market)
- [x] Notification category colors (red, blue, green, purple, cyan, amber)
- [x] `AdvisorPanel.svelte` — AI advisor suggests actions based on game state analysis
- [x] Advisor suggestions: negative cash, low reserves, operating at loss, no infrastructure, damaged nodes, unmet demand, no active research, poor credit rating
- [x] Priority levels: critical (red), warning (amber), info (blue)

### Settings
- [x] `Settings.svelte` — expanded game settings panel
- [x] Graphics settings (map quality: low/medium/high)
- [x] Audio settings (music volume, SFX volume)
- [x] Gameplay settings (auto-save interval: 25/50/100/200/disabled, notification toggle)
- [x] Keyboard shortcut reference (Space, F5, F9, B, E, 1-4, Esc)

### Visual Polish
- [x] Consistent dark theme across all panels (navy/charcoal base, Bloomberg Terminal feel)
- [x] Color palette: green=profit, red=loss, blue=neutral, amber=warning
- [x] Corporation brand colors on map (8 distinct corp colors)
- [x] Hover states on all interactive elements
- [x] A11y: 0 warnings (all clickable elements properly accessible)

### Verification
- [x] All overlays toggle and show accurate data
- [x] Advisor gives relevant, context-aware suggestions
- [x] Settings save and persist (localStorage)
- [x] `svelte-check` — 0 errors, 0 warnings (467 files)
- [x] `cargo clippy -- -D warnings` — clean
- [x] `cargo test` — target ~120-150 tests (happy path + edge cases per system, integration cross-crate, frontend via Bun)
- [x] `bun run build` — production build succeeds

---

## Phase 9: Audio & Content Polish

*Goal: The game feels like a real game, not a tech demo.*

### Audio
- [x] AudioManager singleton using Web Audio API (synthesized oscillator tones — no audio file dependencies)
- [x] Event-driven sound effects: build, complete, cash, alarm, crash, discovery, achievement, click, open, close, error
- [x] Event-to-sound mapping for all game events (ConstructionStarted, DisasterStruck, ResearchCompleted, etc.)
- [x] Multi-note chords for achievement/completion sounds (major third harmonics)
- [x] Dynamic intensity control (adjusts music layer volume based on game state)
- [x] Volume controls: music and SFX volume sliders wired to AudioManager via settings stores
- [x] Mute/unmute toggle, proper dispose on game exit
- [x] AudioManager auto-initializes on game start, subscribes to settings store changes

### Full Audio Expansion
- [ ] Ambient background music tracks per era (synthesized or licensed, layered loops)
- [ ] Era-specific sound palettes (telegraph clicks for Telegraph era, digital tones for Modern, etc.)
- [ ] UI interaction sounds: panel open/close, tab switch, button hover, slider drag
- [ ] Environmental audio: city hum at city zoom, ocean ambience at coastal zoom, wind at mountain zoom
- [ ] Disaster-specific audio cues: earthquake rumble, storm winds, cyber attack glitch sounds
- [ ] Victory/achievement fanfare with escalating intensity based on achievement tier
- [ ] Audio ducking: lower music volume during important notifications and events
- [ ] Spatial audio hints: directional cues for off-screen events (disaster in another region)

### Visual Content
- [x] Distinct node shapes per type: CentralOffice=square, CellTower=triangle, DataCenter=pentagon, ExchangePoint=hexagon, SatelliteGround=star, SubmarineLanding=diamond, WirelessRelay=circle
- [x] Edge visualization: 7 distinct styles (FiberLocal=dashed green, FiberRegional=solid blue, FiberNational=solid indigo, Copper=brown, Microwave=dashed cyan, Satellite=dashed yellow, Submarine=dashed blue thick)
- [x] Night-earth city glow effect (warm orange glow sprites proportional to city population)
- [x] Company badge system (first letter of corp name displayed near owned nodes, visible at zoom >3x)

### Tutorial
- [x] 10-step interactive tutorial: Welcome → Camera Controls → Dashboard → Build Node → Build Edge → Revenue → Panels → Speed Controls → Save → Ready
- [x] Tutorial overlay with step counter, progress bar, next/back/skip buttons
- [x] Skippable at any time (Skip Tutorial button)
- [x] Auto-triggers on first new game (persisted to localStorage)
- [x] Reset Tutorial button in Settings panel
- [x] Positioned cards (center, top-right, bottom-left) based on context

### Verification
- [x] Audio plays on game events with correct volume (synthesized tones via Web Audio API)
- [x] Distinct node shapes and differentiated edges visible on map
- [x] City glow effect visible at world zoom level
- [x] Tutorial completes all 10 steps
- [x] Settings volume sliders control audio
- [x] `bun run build` succeeds with all Phase 9 additions

---

## Phase 10: Advanced Gameplay

*Goal: Competitive depth that makes the game strategically interesting beyond build-and-earn.*

### Bankruptcy & Auctions
- [x] Detect insolvency (negative cash + CreditRating::D + debt > 90x cost, checked in finance system)
- [x] Player gets InsolvencyWarning event; AI auto-decides bailout vs bankruptcy based on archetype
- [x] Bailout: 20% of debt as emergency loan at 15% interest rate
- [x] Bankruptcy: all assets move to sealed-bid auction, corp zeroed out
- [x] Auction system: 50-tick open period, AI bids based on asset value × archetype willingness multiplier
- [x] Auction resolution: highest bidder wins, assets transfer, payment deducted
- [x] AuctionPanel.svelte: active auctions list, bid input, asset/bid counts

### Hostile Takeover & Mergers
- [x] ProposeAcquisition command: offer price to acquire target corporation
- [x] AI evaluates offers vs book value × archetype premium (1.2x-2.0x by archetype)
- [x] Successful acquisition: `transfer_corporation_assets()` moves all nodes, edges, contracts, debt, workforce
- [x] AI proposes mergers between compatible AI corps (both Defensive or Tech)
- [x] MergerPanel.svelte: incoming/outgoing proposals, valuations, accept/reject UI

### Sabotage & Espionage
- [x] Espionage missions: reveal competitor stats, cost $200K+, 10-tick duration
- [x] Sabotage missions: degrade node health by 30%, cost $400K+, 15-tick duration
- [x] Success/detection probability based on security levels (base 60-40%, ±10% per security level)
- [x] Counter-espionage: UpgradeSecurity command ($100K per level)
- [x] Detection penalties: caught → reputation loss, covert_ops system processes missions
- [x] AI espionage behavior: Aggressive Expander uses frequently, others rarely

### Lobbying & Political Influence
- [x] LobbyPolicy enum: ReduceTax, RelaxZoning, FastTrackPermits, IncreasedCompetitorBurden, SubsidyRequest
- [x] Diminishing returns: `influence_gain = budget / (1 + total_prior_spend / 1M)`
- [x] Scandal chance: 5% per $500K spent, causes reputation loss + campaign cancellation
- [x] Policy effects: tax reduction, zoning relaxation, construction speed bonus, competitor tax increase, subsidies
- [x] AI lobbying per archetype: BudgetOperator→ReduceTax, AggressiveExpander→CompetitorBurden, etc.
- [x] IntelPanel.svelte: espionage/sabotage/security controls + lobbying campaigns with progress bars

### Cooperative Infrastructure
- [x] ProposeCoOwnership command: offer share percentage to another corp
- [x] RespondCoOwnership: accept/reject, creates multi-owner node (owners Vec with share percentages)
- [x] ProposeBuyout: offer to buy out co-owner's share
- [x] VoteUpgrade: majority approval for node upgrades
- [x] Revenue/cost split proportionally by ownership share in revenue/cost systems

### Alliance System
- [ ] Form alliances between corps (player-initiated or AI-proposed)
- [ ] Trust scoring based on contract history, shared infrastructure, trade volume
- [ ] Revenue sharing within alliances (configurable percentage)
- [ ] Alliance dissolution checks: trust drops below threshold → alliance breaks
- [ ] Alliance-exclusive benefits: shared routing, bulk discount on co-builds

### Legal System
- [ ] Lawsuit filing: patent infringement, contract breach, anti-competitive behavior
- [ ] Lawsuit resolution over N ticks (damage calculation, settlement offers)
- [ ] Legal costs scale with lawsuit complexity and jurisdiction
- [ ] Court outcomes: damages awarded, injunctions, forced licensing
- [ ] AI corps file lawsuits based on archetype (Aggressive Expander litigates frequently)

### Government Grants System
- [ ] Government grants generated per region (underserved area incentives)
- [ ] Grant requirements: build coverage in target area within deadline
- [ ] Grant rewards: cash payout, tax breaks, exclusive land access
- [ ] Progress tracking toward grant completion
- [ ] AI corps compete for grants based on strategy mode

### Fog of War System
- [ ] Competitor infrastructure hidden by default (only see own assets + public info)
- [ ] Intel levels: None → Basic (node count) → Detailed (capacity, revenue) → Full (financials, strategy)
- [ ] Intel gathered through espionage missions, market reports, alliance sharing
- [ ] Intel decays over time (stale data)
- [ ] Map overlay for fog of war visualization (dimmed regions with unknown competitor data)

### Pricing System
- [ ] Per-region pricing tiers (economy, standard, premium)
- [ ] Price-per-unit setting affects demand capture and revenue per customer
- [ ] Undercut pricing strategy: lower prices to steal market share
- [ ] Premium pricing: higher margins but slower growth
- [ ] AI corps adjust pricing dynamically based on archetype and competition

### Maintenance Scheduling System
- [ ] Per-asset maintenance priority tiers (Critical, High, Normal, Low, Deferred)
- [ ] Auto-repair toggle per asset
- [ ] Scheduled maintenance windows (reduced capacity during maintenance vs emergency downtime)
- [ ] Preventive maintenance reduces disaster damage
- [ ] Maintenance budget allocation with diminishing returns

### Achievements & Win Conditions
- [x] 20 achievements tracked: FirstNode, FirstProfit, TenNodes, HundredNodes, MillionRevenue, BillionRevenue, AAARating, DebtFree, GlobalBackbone, OceanCable, FirstContract, AllRegions, FirstMerger, MonopolyRegion, SurviveBankruptcy, ResearchComplete, etc.
- [x] Victory conditions: Domination (>75% regions), Tech (all research), Wealth ($10B net worth), Infrastructure (200+ nodes)
- [x] Weighted total score: 0.3×domination + 0.2×tech + 0.25×wealth + 0.25×infrastructure
- [x] Achievement system checks every 30 ticks for performance
- [x] AchievementPanel.svelte: victory progress bars (4 scores), achievement grid with unlock status
- [x] VictoryAchieved event emitted when total score reaches 1.0

### Sandbox Mode
- [ ] Sandbox as a selectable game mode alongside Standard (in New Game menu)
- [ ] Sandbox features: unlimited funds, no bankruptcy, instant construction, adjustable AI behavior
- [ ] All systems still run (disasters, AI, market) but player is shielded from failure
- [ ] Sandbox mode flag stored in WorldConfig, checked by finance/bankruptcy systems
- [ ] Available in both single-player and multiplayer (host option)

### Verification
- [x] `cargo build --release` succeeds with all Phase 10 additions
- [x] `cargo test` — target ~120-150 tests (happy path + edge cases per system, integration cross-crate)
- [x] All 4 existing systems integrated into tick order (auction, covert_ops, lobbying, achievement)
- [ ] 6 new planned systems to integrate: alliance, legal, grants, fog_of_war, pricing, maintenance_scheduling
- [x] ~25 new events, ~15 new commands, 6 new component files, 4 new systems
- [x] 4 new frontend panels (AuctionPanel, MergerPanel, IntelPanel, AchievementPanel)
- [x] HUD updated with panel buttons: Auc, M&A, Int, Ach

---

## Phase 11: Multiplayer Server

*Goal: Multiple players connect to a persistent world and play together.*

### gt-server — Multiplayer Binary
- [x] Rust Axum WebSocket server — full WebSocket handler in ws.rs with MessagePack + JSON support
- [x] Same `gt-simulation` crate, compiled natively (not WASM) — gt-server depends on gt-simulation
- [x] World management: create, load, tick, save worlds — WorldInstance with Mutex<GameWorld>, tick loop, world CRUD
- [x] WebSocket protocol: MessagePack binary serialization — rmp-serde with serialize_msgpack/deserialize_msgpack
- [x] Client → Server: commands (build, hire, set policy, chat, saves) — 11 ClientMessage variants
- [x] Server → Client: tick deltas, acks, events, full snapshots — 13 ServerMessage variants including ProxySummary, SaveList, SaveData
- [x] Server-authoritative validation on all commands — corp ownership check, world membership verification
- [x] Rate limiting on player actions — sliding window: 10 commands/sec, 5 chat/10sec

### Authentication & Accounts
- [x] JWT auth with argon2 password hashing — auth.rs with generate_access_token, generate_refresh_token, verify_password
- [x] REST endpoints: POST /api/auth/register, POST /api/auth/login — in routes.rs with validation
- [x] WebSocket auth: Login, Register, TokenRefresh, Guest — 4 AuthRequest variants handled in ws.rs
- [x] Database-backed accounts with in-memory fallback — state.rs checks db first, falls back to HashMap

### World Persistence
- [x] PostgreSQL schema — 001_initial_schema.sql: accounts, game_worlds, player_sessions, cloud_saves, world_snapshots, event_log, leaderboard
- [x] Feature-gated sqlx integration — `postgres` Cargo feature, db.rs with full CRUD for all 7 tables
- [x] Binary save format — save_game_binary() with version byte + bincode + zstd compression in world.rs
- [x] Periodic snapshot saving — every 100 ticks in tick.rs, async background save to PostgreSQL
- [x] Cloud save REST API — POST/GET/DELETE /api/saves/{slot} with max 50MB validation
- [x] Cloud save WebSocket API — UploadSave, RequestSaves, DownloadSave, DeleteSave client messages

### AI Proxy
- [x] Player disconnect → AI proxy activates — DefensiveConsolidator with proxy_mode=true, inserted into ai_states
- [x] Player reconnect → AI proxy deactivates, summary sent — ProxySummary ServerMessage with ticks_elapsed and actions
- [x] Database session tracking — set_player_disconnected/set_player_connected with is_ai_proxy flag

### Multiplayer UI
- [x] `WorldBrowser.svelte` — server address input, auth tabs (guest/login/register), world list with join buttons
- [x] `WebSocketClient.ts` — MessagePack WebSocket client with exponential backoff reconnection
- [x] `multiplayerState.ts` — Svelte stores for connection state, chat, players, proxy summary
- [x] `Chat.svelte` — collapsible in-game chat overlay, 100 message scrollback
- [x] HUD multiplayer indicators — connection status badge (Online/Reconnecting/Offline) + player count

### Deployment
- [x] Dockerfile — multi-stage build: rust:1.83 builder → debian:bookworm-slim runtime, postgres feature enabled
- [x] docker-compose.yml — gt-server + PostgreSQL 16 Alpine with health checks
- [x] CI workflow — desktop-release.yml with matrix builds (macOS ARM/Intel, Linux, Windows)
- **Hosting:** Currently Fly.io + Vercel. Production target: Hetzner + Cloudflare Workers.

### Verification
- [x] `cargo build --release` succeeds — all crates compile clean
- [x] `cargo test` passes — target ~120-150 tests across all crates
- [x] `bun run build` succeeds — frontend compiles with multiplayer components
- [x] WebSocket protocol roundtrip tests pass — MessagePack serialization verified

---

## Phase 12: Desktop App & Distribution

*Goal: The game is downloadable and distributable.*

### Tauri Desktop App
- [x] Tauri v2 project in `desktop/src-tauri` — configured with frontendDist, devUrl, window size 1440x900
- [x] Plugins: tauri-plugin-shell, tauri-plugin-dialog, tauri-plugin-fs, tauri-plugin-process
- [x] Native save commands — save_game_native, load_game_native, list_saves, get_saves_dir via #[tauri::command]
- [x] Save files stored as `.gtco` in platform app data directory
- [x] `DesktopSaveManager.ts` — Tauri IPC wrapper with isTauri() environment detection
- [x] tauri.conf.json — fs plugin scope, window config, build commands

### Web Distribution
- [x] PWA manifest — manifest.json with app name, theme color, display: standalone
- [x] Service worker — cache-first for WASM/JS/CSS/images, network-first for API/pages, offline fallback
- [x] Layout integration — service worker registration, manifest link, theme-color meta tag

### CI/CD
- [x] desktop-release.yml — GitHub Actions matrix: macOS ARM/Intel, Linux, Windows via tauri-action
- [x] web-deploy job — WASM build + bun build in release workflow
- [x] Existing ci.yml — check, test, clippy, fmt, wasm, frontend jobs

### Platform Integration (future)
- [ ] Steam integration (Steamworks via Tauri plugin — auth, achievements, cloud saves)
- [ ] itch.io distribution

### Verification
- [x] `cargo build --release` succeeds
- [x] `bun run build` succeeds
- [x] Frontend builds with all new components

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
