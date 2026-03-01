# Offline Single-Player Mode — Implementation Plan

## Context

GlobalTelco supports fully offline single-player from day 1. The same Rust ECS simulation engine that runs on multiplayer servers compiles to WebAssembly and runs entirely in the browser (or Tauri desktop app). No network connection required for single-player.

This plan covers: world generation, AI corporations, save/load, speed controls, and the single-player game flow.

---

## Architecture

```
┌──────────────────────────────────────────────────┐
│              BROWSER / TAURI                       │
│                                                    │
│  ┌────────────┐  ┌──────────────┐                 │
│  │  Svelte UI │  │  deck.gl     │                 │
│  │  (menus,   │  │  (2D map)    │                 │
│  │   panels)  │  │              │                 │
│  └─────┬──────┘  └──────┬───────┘                 │
│        │                │                          │
│  ┌─────┴────────────────┴──────────────────────┐  │
│  │         WASM Bridge (gt-wasm)                │  │
│  │                                              │  │
│  │  ┌────────────────────────────────────────┐  │  │
│  │  │     Rust ECS Simulation (gt-simulation) │  │  │
│  │  │  - World generation                    │  │  │
│  │  │  - AI corporations (gt-ai)             │  │  │
│  │  │  - Economy, infra, population          │  │  │
│  │  │  - All 36 systems per tick             │  │  │
│  │  └────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────┘  │
│                                                    │
│  Local Storage / IndexedDB (save files)            │
└──────────────────────────────────────────────────┘
```

**Key principle:** The WASM module IS the server. Same simulation code as multiplayer servers, just compiled to WASM instead of native. Player commands go directly to WASM — no network latency.

---

## Phase 1: Core ECS Simulation (gt-simulation + gt-common)

### 1a. Core Types (gt-common)

**Directory:** `crates/gt-common/src/types/` (modular: mod.rs, terrain.rs, network.rs, node.rs, edge.rs, spectrum.rs, satellite.rs, config.rs)

- `EntityId` — unique entity identifier (u64)
- `Tick` — simulation tick counter (u64)
- `WorldConfig` — starting era, difficulty, AI count, disaster severity, world seed, speed settings
- `TerrainType` — enum: Urban, Suburban, Rural, Mountainous, Desert, Coastal, OceanShallow, OceanDeep, Tundra, Frozen
- `NodeType` — enum: ~33 types across eras (e.g., TelegraphOffice, TelephoneExchange, AccessTower, FiberHub, DataCenter, IXP, SubseaStation, SatelliteStation, 5GSmallCell, EdgeComputeNode, etc.)
- `EdgeType` — enum: ~15 types across eras (e.g., TelegraphWire, CopperLoop, FiberLocal, FiberRegional, FiberNational, Microwave, Subsea, Satellite, 5GmmWave, MeshWireless, etc.)
- `CreditRating` — enum: AAA through D
- `AIArchetype` — enum: AggressiveExpander, DefensiveConsolidator, TechInnovator, BudgetOperator
- `AIStrategy` — enum: Expand, Consolidate, Compete, Survive

### 1b. ECS World Container (gt-simulation)

**Directory:** `crates/gt-simulation/src/world/` (modular: mod.rs, generation.rs, queries.rs, serialization.rs, commands_*.rs, utils.rs, tests.rs)

- `GameWorld` struct — owns all component storage (SoA arrays), entity allocator, event queue, tick counter
- `GameWorld::new(config: WorldConfig) -> Self` — creates empty world from config
- `GameWorld::tick(&mut self, dt: f64)` — runs all 20 systems in deterministic order
- `GameWorld::process_command(&mut self, cmd: Command) -> Result<(), CommandError>` — validates and executes player actions
- Entity CRUD: `spawn_entity()`, `despawn_entity()`, `get_component()`, `set_component()`

### 1c. Component Storage

**File:** `crates/gt-simulation/src/components/`

One file per component group:
- `position.rs` — `Position { lon: f64, lat: f64 }`
- `ownership.rs` — `Ownership { corp_id: EntityId, subsidiary_id: Option<EntityId> }`
- `financial.rs` — `Financial { revenue: f64, costs: f64, value: f64 }`
- `capacity.rs` — `Capacity { current: f64, max: f64, utilization: f64 }`
- `health.rs` — `Health { current: f64, max: f64, degradation_rate: f64 }`
- `construction.rs` — `Construction { remaining_ticks: u32, total_ticks: u32 }`
- `population.rs` — `PopulationData { count: u64, growth_rate: f64, employment_rate: f64, migration_pressure: f64 }`
- `demand.rs` — `Demand { current: f64, growth_rate: f64, satisfaction: f64 }`
- `workforce.rs` — `Workforce { skill_level: f32, experience: f32, salary: f64, region: EntityId }`
- `ai_state.rs` — `AIState { archetype: AIArchetype, strategy: AIStrategy, weights: ArchetypeWeights }`
- `policy.rs` — `MaintenancePolicy { target_uptime: f64, budget: f64, priority: u8 }`
- `corporation.rs` — `CorporationData { name: String, cash: f64, total_debt: f64, credit_rating: CreditRating, is_ai: bool, archetype_index: i32 }`

### 1d. Event System

**File:** `crates/gt-simulation/src/events.rs`

- `GameEvent` — enum of all possible events (InfraBuilt, DisasterStruck, CorporationBankrupt, ContractSigned, TechResearched, etc.)
- `EventQueue` — append-only queue, drained each tick, processed by systems
- Events are the primary way systems communicate side effects

### 1e. Systems (Deterministic Tick Order)

**File:** `crates/gt-simulation/src/systems/`

Each system is a function: `fn system_name(world: &mut GameWorld)`

36 systems in deterministic tick order:
1. `construction_system` — advance construction timers, complete builds
2. `orbital_system` — update satellite positions via Keplerian mechanics
3. `satellite_network_system` — rebuild dynamic ISL + ground station links
4. `maintenance_system` — check workforce vs maintenance needs, degrade unmaintained infra
5. `population_system` — update populations, migration, employment based on infrastructure
6. `coverage_system` — calculate network coverage per region, signal strength, dead zones
7. `demand_system` — calculate regional demand based on population and economy
8. `routing_system` — recalculate network routes if topology changed (dirty-flag optimization)
9. `utilization_system` — calculate infrastructure utilization from routed demand
10. `spectrum_system` — manage spectrum allocation, frequency assignments, interference
11. `ftth_system` — fiber-to-the-home chain validation, active NAP marking
12. `manufacturing_system` — satellite + terminal factory production
13. `launch_system` — rocket launches, reliability rolls, orbit insertion
14. `terminal_distribution_system` — terminal warehouse → city adoption
15. `satellite_revenue_system` — retail subscriber + wholesale bandwidth revenue
16. `revenue_system` — calculate per-corp revenue from served demand
17. `cost_system` — calculate maintenance, salary, interest costs
18. `finance_system` — update corporate finances (income, balance sheet, credit rating)
19. `contract_system` — process contract terms, renewals, breaches
20. `ai_system` — AI corporations make decisions (build, hire, contract, research)
21. `weather_system` — regional weather patterns, storms, disaster amplification
22. `disaster_system` — roll for disasters, apply damage
23. `debris_system` — orbital debris tracking, Kessler cascade threshold
24. `servicing_system` — satellite refuel + repair missions
25. `regulation_system` — process regulatory changes, political events
26. `research_system` — advance tech research progress
27. `patent_system` — license revenue collection, patent expiration, enforcement
28. `market_system` — dynamic AI spawning, mergers, bankruptcies
29. `auction_system` — process spectrum and infrastructure auction bids, resolve winners
30. `covert_ops_system` — execute espionage actions, intel gathering, sabotage resolution
31. `lobbying_system` — process lobbying investments, political influence, regulation nudges
32. `alliance_system` — trust scoring, revenue sharing, dissolution checks
33. `legal_system` — lawsuit resolution, damage calculation, settlement processing
34. `grants_system` — government grant generation, progress tracking, completion payouts
35. `achievement_system` — check achievement conditions, unlock milestones, track stats
36. `stock_market_system` — stock price simulation, IPO, share trading, market events

---

## Phase 2: World Generation (gt-world)

### 2a. Procedural World

**File:** `crates/gt-world/src/procgen.rs`

- Generate continental landmasses using 3D sphere-based fractal noise (avoids lon/lat seam artifacts)
- Place mountain ranges, rivers, coastlines from elevation data
- Generate countries with capitals (K-means clustering on land cells)
- Distribute population based on terrain suitability
- Initialize economic indicators from terrain and population
- All deterministic from a single `u64` world seed

### 2b. Real Earth Data

**File:** `crates/gt-world/src/earth.rs`

- Load pre-processed GeoJSON for country/region borders (from `data/` directory)
- Load pre-processed city locations with population data
- Load economic indicators (GDP, internet penetration, urbanization)
- All data stored as static assets, loaded at game start when "Real Earth" selected

### 2c. Invisible Grid Cells (Spatial Index)

**File:** `crates/gt-world/src/procgen.rs`

- Geodesic grid: icosahedral subdivision → cells on unit sphere
- Spatial hash for O(1) lookups by coordinate (`find_nearest_cell(lon, lat)`)
- Each grid cell: terrain type, disaster risk, cost modifiers — used for spatial queries only (not player-visible)
- Infrastructure placement is free at exact (lon, lat) coordinates; grid cells provide terrain/cost lookups behind the scenes
- AI nodes use jittered positions near cell centers for organic-looking placement

### 2d. Region & Economy Seeding

**File:** `crates/gt-world/src/regions.rs`

- K-means region clustering (land-aware: initial centers placed on land cells)
- Seed GDP, population, demand, connectivity per region
- Create region entities with economic components

---

## Phase 3: AI Corporation System (gt-ai)

### 3a. Archetype Definitions

**File:** `crates/gt-ai/src/archetypes.rs`

```rust
pub struct ArchetypeWeights {
    pub expansion: f32,      // 0-1, prioritize building new infra
    pub consolidation: f32,  // 0-1, improve existing network
    pub tech_investment: f32, // 0-1, R&D priority
    pub aggression: f32,     // 0-1, competitive vs cooperative
    pub risk_tolerance: f32, // 0-1, willingness to take debt
    pub financial_prudence: f32, // 0-1, cash reserve preference
}
```

4 built-in archetypes:
1. **Aggressive Expander** — expansion=0.9, consolidation=0.2, aggression=0.8, risk=0.8, prudence=0.2
2. **Defensive Consolidator** — expansion=0.3, consolidation=0.9, aggression=0.2, risk=0.2, prudence=0.9
3. **Tech Innovator** — expansion=0.5, consolidation=0.5, tech=0.9, aggression=0.4, risk=0.5
4. **Budget Operator** — expansion=0.4, consolidation=0.6, tech=0.2, aggression=0.1, risk=0.1, prudence=1.0

Company name pools per archetype for varied AI corp names.

### 3b. AI Decision System

**File:** `crates/gt-ai/src/controller.rs`

The `ai_system` runs once per tick for each AI corporation:

1. **Evaluate state** — cash, debt, owned assets, credit rating, competitors
2. **Select strategy** — based on archetype weights + current financial health:
   - Cash < 0 and debt high → Survive
   - Has expandable regions and cash available → Expand (weighted by archetype)
   - Network quality low → Consolidate (weighted by archetype)
   - Competitors nearby and aggressive → Compete (weighted by archetype)
3. **Execute actions** — based on selected strategy:

### 3c. AI Actions

**File:** `crates/gt-ai/src/actions.rs`

- `build_node(world, corp_id, archetype)` — score locations by terrain suitability, regional demand, proximity to existing network, cost. Choose node type based on demand and network gaps. Place at jittered position near target cell center.
- `build_edge(world, corp_id, archetype)` — find pairs of owned nodes that could benefit from a connection, choose edge type based on distance and terrain.
- `manage_finances(world, corp_id, archetype)` — take loans if cash low and credit good, pay down debt if cash high.
- `propose_contract(world, corp_id, archetype)` — identify nearby corps, propose peering/transit contracts based on mutual benefit.

### 3d. AI Proxy (Multiplayer Offline)

**File:** `crates/gt-ai/src/proxy.rs`

When a player disconnects in multiplayer:
- AI proxy activates using the player's saved policies
- Only executes existing policies (maintenance, contract renewals, debt payments)
- No strategic changes (no new builds, no new contracts)
- Player gets a summary of actions taken while away upon reconnecting

---

## Phase 4: Save/Load System

### 4a. Serialization

**File:** `crates/gt-common/src/serialization.rs`

- Serialize entire `GameWorld` to binary format using `serde` + `bincode`
- Header: save version, timestamp, world config, tick count
- Entity table: all entity IDs and their component bitmasks
- Component arrays: contiguous arrays per component type
- Compression: zstd compression on the binary blob
- Target: < 50MB for a mature game world

### 4b. Save/Load API (gt-wasm)

**File:** `crates/gt-wasm/src/queries.rs` (save/load commands dispatched via `lib.rs`)

WASM bridge functions:
- `save_game(slot_name: &str) -> Vec<u8>` — serialize world, return compressed bytes
- `load_game(data: &[u8]) -> Result<(), LoadError>` — deserialize bytes, restore world
- `get_save_slots() -> Vec<SaveSlotInfo>` — list available saves (from JS-side storage)

### 4c. Browser Storage

On the JS/Svelte side:
- Save files stored in IndexedDB (via `idb` library)
- Multiple save slots supported
- Auto-save every N ticks (configurable, default 50)
- Save slot metadata: name, timestamp, tick count, difficulty, corporation name

### 4d. Cloud Saves

For logged-in users:
- Same binary format uploaded to server API
- Stored as blob in PostgreSQL
- Sync on login: compare local and cloud timestamps, offer merge/override

---

## Phase 5: Speed Controls

### 5a. Simulation Speed

**In `GameWorld`:**
- `paused: bool` — when true, skip tick processing
- `speed_multiplier: f64` — 1.0 (1x), 2.0 (2x), 4.0 (4x), 8.0 (8x)
- Speed multiplier applied to delta time before tick accumulation
- Pause/speed controlled via WASM bridge commands: `toggle_pause()`, `set_speed(multiplier)`

### 5b. Frontend Speed Controls

**File:** `web/src/lib/game/SpeedControls.svelte`

- Buttons: Pause (⏸), Play (▶), 2x (▶▶), 4x (▶▶▶), 8x (⏩)
- Current speed display
- Quick save / quick load buttons
- Keyboard shortcuts: Space=pause, 1/2/3/4=speed, F5=save, F9=load

---

## Phase 6: Single-Player Game Flow

### 6a. New Game Flow

1. Player opens game in browser (or Tauri app)
2. Main menu: New Game, Load Game, Multiplayer, Settings, Quit
3. New Game settings:
   - Corporation name (text input)
   - World type: Real Earth / Procedural
   - Starting era (dropdown)
   - Difficulty (Easy/Normal/Hard/Custom)
   - AI Corporation count (slider 0-10)
   - AI aggressiveness (slider)
   - Disaster severity (slider 1-10)
   - World seed (text input, 0 = random)
4. Click "Start Game"
5. WASM module initializes: creates `GameWorld`, generates world, creates player corp with 1 starter node appropriate to starting era (e.g., Telegraph Office for Telegraph era, Cell Tower for Modern era), spawns AI corps
6. deck.gl map renders the world, Svelte UI shows HUD panels
7. Game begins paused so player can orient. Auto-pause triggers on critical events (bankruptcy warning, hostile takeover offer, major disaster).

### 6b. Initialization Sequence (in WASM)

```
new_game(config: WorldConfig, player_name: String) -> GameWorld:
    1. world = GameWorld::new(config)
    2. generate_terrain(world, config)        // gt-world
    3. generate_grid_cells(world, config)       // gt-world (invisible spatial index)
    4. generate_regions(world, config)         // gt-world
    5. seed_economics(world, config)           // gt-economy
    6. player_corp = create_corporation(world, player_name, false)  // player
    7. place_starter_node(world, player_corp, config.starting_era)  // 1 starter node appropriate to era
    8. for i in 0..config.ai_count:
         archetype = ARCHETYPES[i % 4]
         name = archetype.random_name(config.seed + i)
         create_corporation(world, name, true, archetype)  // AI corp
    9. world.paused = true                     // game starts paused
   10. return world
```

### 6c. Load Game Flow

1. Main menu → Load Game
2. List save slots from IndexedDB (name, timestamp, corp name, tick count)
3. Player selects a slot
4. WASM loads binary data, deserializes world
5. Game resumes from saved state

### 6d. Auto-Save

- Every 50 economic ticks, auto-save to "AutoSave" slot
- Rotating auto-saves: AutoSave1, AutoSave2, AutoSave3 (keeps last 3)
- Auto-save runs asynchronously (doesn't block simulation)

### 6e. Tutorial Integration

- Tutorial references the player's starter node as the first interactive element
- Step 1: "Welcome — you've just founded your telecom company. Your first [era-appropriate node] is operational."
- Tutorial guides player through examining the starter node, then building a second node and connecting them with an edge
- Tutorial is skippable at any point

### 6f. Research System (Single-Player)

- Research tree is freely explorable: techs gated by prerequisites only, NOT by era
- Players can research any tech they meet the prereqs for regardless of current era
- Tech functions as an economic commodity: patents, licenses, and open-sourcing create strategic depth
- AI corps research based on archetype priorities, not era restrictions

### 6g. Sandbox Mode

- Sandbox available as a game mode in single-player (selectable in New Game menu)
- Sandbox features: unlimited funds, no bankruptcy, instant construction, adjustable AI behavior
- All simulation systems still run — sandbox shields from failure, not from gameplay
- Useful for learning game mechanics, testing strategies, and creative play

### 6h. Panel Architecture (6 Tabbed Groups)

- Management panels organized into 6 tabbed groups for clean navigation:
  - **Finance** — Dashboard, pricing strategy, insurance
  - **Operations** — Infrastructure, maintenance, repair queue, workforce, build menu
  - **Diplomacy** — Alliances, legal actions, intel/espionage, co-ownership
  - **Research** — Tech tree, patents/licensing
  - **Market** — Contracts, auctions, mergers & acquisitions, government grants, subsidiaries
  - **Info** — Region overview, advisor, achievements

### 6i. Build Menu

- Build menu categorized by tier (Local → Regional → National → Continental → Global Backbone)
- Node types grouped within tiers for easy discovery
- Edge types shown contextually based on selected source/target node tiers
- Cost, construction time, and era availability shown per item

---

## Verification

1. **World gen:** New game → world generates with terrain, grid cells, regions, economics
2. **AI corps:** AI corporations spawn, make decisions, build infrastructure, manage finances over time
3. **Speed controls:** Pause/resume/2x/4x/8x all work correctly
4. **Save/Load:** Save a game → close browser → reopen → load → state matches exactly
5. **Auto-save:** Play for a while → close browser → reopen → auto-save slot available
6. **Determinism:** Same seed + same actions = same world state (critical for future multiplayer)
