# GlobalTelco: Technical Architecture

Comprehensive technical architecture for the GlobalTelco web-based infrastructure empire builder.

---

## 1. Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     BROWSER / TAURI                          │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │   Svelte UI  │  │   deck.gl    │  │     D3.js        │  │
│  │  (panels,    │  │  (2D map,    │  │  (charts, data   │  │
│  │   menus,     │  │  free place) │  │   visualization) │  │
│  │   HUD)       │  │              │  │                  │  │
│  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘  │
│         │                 │                    │             │
│  ┌──────┴─────────────────┴────────────────────┴─────────┐  │
│  │              WASM Bridge (gt-wasm)                     │  │
│  │  Rust ECS Simulation running in WebAssembly            │  │
│  │  (Single-player: full sim in browser)                  │  │
│  │  (Multiplayer: thin client, server has authority)      │  │
│  └───────────────────────┬───────────────────────────────┘  │
│                          │ WebSocket                        │
└──────────────────────────┼──────────────────────────────────┘
                           │
              ┌────────────┴────────────────┐
              │    MULTIPLAYER SERVER        │
              │    (Rust native binary)      │
              │                             │
              │  Same ECS simulation code   │
              │  + WebSocket server         │
              │  + World persistence        │
              │  + Auth validation          │
              └────────────┬────────────────┘
                           │
              ┌────────────┴────────────────┐
              │      PostgreSQL              │
              │  (world state, accounts,     │
              │   cloud saves)               │
              └─────────────────────────────┘
```

---

## 2. Rust Simulation Engine

### 2a. Entity Component System (ECS)

All game state is managed through an ECS architecture. Entities are IDs, components are data, systems process entities with specific component combinations.

**Core Entity Types:**
- `InfrastructureNode` — towers, data centers, IXPs, etc.
- `InfrastructureEdge` — fiber, microwave, subsea cables, etc.
- `Corporation` — player and AI companies
- `Subsidiary` — regional sub-companies
- `Employee` / `Team` — workforce entities
- `Region` — geographic/political regions
- `City` — population centers
- `Contract` — business agreements
- `LandParcel` — backend spatial cells (terrain, coverage queries; invisible to player)
- `TechResearch` — ongoing research projects
- `DebtInstrument` — loans, bonds
- `Patent` — owned intellectual property (tech patents, licenses)
- `LicenseAgreement` — active license between patent holder and licensee
- `Alliance` — formal multi-corp alliance entity
- `Lawsuit` — active legal dispute between corporations
- `GovernmentGrant` — government-funded infrastructure project

**NodeType Enum (era-specific, flat enum — ~33 variants):**
- **Telegraph:** `TelegraphOffice`, `TelegraphRelay`
- **Telephone:** `TelephoneExchange`, `OperatorSwitch`, `LongDistanceRelay`
- **Early Digital:** `DigitalSwitch`, `MicrowaveTower`, `CoaxHub`
- **Internet:** `DSLTerminal`, `FiberPOP`, `WebHostingCenter`, `DialUpGateway`
- **Modern:** `CellTower4G`, `CellTower5G`, `DataCenter`, `FTTHNode`, `CDNEdge`, `ExchangePoint`, `BackboneRouter`
- **Near Future:** `Cell6G`, `SatelliteGround`, `QuantumRelay`, `EdgeAINode`, `SubmarineLanding`

**Core Component Types:**
- `Position { longitude, latitude }` — geographic location
- `Ownership { corp_id, subsidiary_id }` — who owns this entity
- `Financial { revenue, costs, value }` — financial data
- `Capacity { current, max, utilization }` — throughput capacity
- `Health { current, max, degradation_rate }` — infrastructure condition
- `Construction { remaining_ticks, total_ticks }` — under construction
- `Population { count, growth_rate, employment_rate, migration_pressure }` — city populations
- `Demand { current, growth_rate, satisfaction_ratio }` — regional demand
- `Workforce { skill_level, experience, salary, assigned_region }` — employee data
- `AIStrategy { archetype, current_mode, weights }` — AI decision state
- `MaintenancePolicy { target_uptime, budget, priority }` — management policies
- `Patent { tech_id, holder_corp, filed_tick, expires_tick, licensed_to }` — patent ownership and licensing
- `LicenseAgreement { patent_id, licensee_corp, royalty_rate, expires_tick }` — active license terms
- `Alliance { alliance_id, name, trust_score, revenue_share_pct }` — alliance membership details
- `AllianceMember { alliance_id, corp_id, joined_tick, vote_weight }` — per-corp alliance participation
- `Lawsuit { plaintiff_corp, defendant_corp, type, damages_claimed, filed_tick }` — active legal dispute
- `GovernmentGrant { region_id, requirements, reward, deadline_tick, progress }` — government-funded project
- `IntelLevel { target_corp, observer_corp, level, last_updated_tick }` — espionage intelligence gathered
- `MaintenancePriority { entity_id, priority_tier, auto_repair }` — per-asset maintenance priority
- `PriceTier { region_id, corp_id, tier_name, price_per_unit }` — regional pricing strategy

### 2b. Systems (Processing Order Per Tick)

Each economic tick, systems run in deterministic order:

1. `construction_system` — advance construction timers, complete builds
2. `maintenance_system` — check workforce vs maintenance needs, degrade unmaintained infra
3. `population_system` — update populations, migration, employment based on infrastructure
4. `coverage_system` — calculate network coverage per region, signal strength, dead zones
5. `demand_system` — calculate regional demand based on population and economy
6. `routing_system` — recalculate network routes if topology changed
7. `utilization_system` — calculate infrastructure utilization from routed demand
8. `revenue_system` — calculate per-corp revenue from served demand
9. `cost_system` — calculate maintenance, salary, interest costs
10. `finance_system` — update corporate finances (income, balance sheet, credit rating)
11. `contract_system` — process contract terms, renewals, breaches
12. `ai_system` — AI corporations make decisions (build, hire, contract, research)
13. `disaster_system` — roll for disasters, apply damage
14. `regulation_system` — process regulatory changes, political events
15. `research_system` — advance tech research progress
16. `market_system` — dynamic AI spawning, mergers, bankruptcies
17. `auction_system` — process spectrum and infrastructure auction bids, resolve winners
18. `covert_ops_system` — execute espionage actions, intel gathering, sabotage resolution
19. `lobbying_system` — process lobbying investments, political influence, regulation nudges
20. `achievement_system` — check achievement conditions, unlock milestones, track stats
21. `patent_system` — license revenue collection, patent expiration, enforcement
22. `alliance_system` — trust scoring, revenue sharing, dissolution checks
23. `legal_system` — lawsuit resolution, damage calculation, settlement processing
24. `grants_system` — government grant generation, progress tracking, completion payouts

### 2c. Crate Structure

```
crates/
├── gt-common/          # Shared types, traits, serialization
│   ├── src/
│   │   ├── types.rs        # Core type definitions
│   │   ├── events.rs       # Event types
│   │   ├── config.rs       # World/game configuration
│   │   └── serialization.rs # Save/load serialization
│
├── gt-simulation/      # Core ECS engine
│   ├── src/
│   │   ├── world.rs        # ECS world container
│   │   ├── systems/        # All ECS systems
│   │   │   ├── patent.rs       # License revenue collection, patent expiration, enforcement
│   │   │   ├── alliance.rs     # Trust scoring, revenue sharing, dissolution
│   │   │   ├── legal.rs        # Lawsuit resolution, damage calculation
│   │   │   └── grants.rs       # Government grant generation and completion
│   │   ├── components/     # All ECS components
│   │   ├── events.rs       # Event queue
│   │   └── tick.rs         # Tick processing orchestrator
│
├── gt-world/           # World generation and geography
│   ├── src/
│   │   ├── earth.rs        # Real Earth data loading (OSM, World Bank)
│   │   ├── procgen.rs      # Procedural world generation
│   │   ├── regions.rs      # Region/country management
│   │   ├── cities.rs       # City placement and properties
│   │   └── terrain.rs      # Terrain classification
│
├── gt-economy/         # Economic simulation
│   ├── src/
│   │   ├── corporation.rs  # Corporation management
│   │   ├── subsidiary.rs   # Subsidiary system
│   │   ├── finance.rs      # Balance sheet, income, debt
│   │   ├── market.rs       # Market dynamics, pricing
│   │   ├── contracts.rs    # Contract system
│   │   └── research.rs     # Tech tree and R&D
│
├── gt-infrastructure/  # Network graph and infrastructure
│   ├── src/
│   │   ├── graph.rs        # Network topology
│   │   ├── routing.rs      # Dijkstra routing
│   │   ├── nodes.rs        # Infrastructure node types
│   │   ├── edges.rs        # Infrastructure edge types
│   │   └── construction.rs # Construction and maintenance
│
├── gt-population/      # Population modeling
│   ├── src/
│   │   ├── demographics.rs # Birth/death/migration
│   │   ├── employment.rs   # Job market simulation
│   │   ├── migration.rs    # Population movement
│   │   └── demand.rs       # Demand calculation from population
│
├── gt-ai/              # AI corporation logic
│   ├── src/
│   │   ├── controller.rs   # AI decision-making framework
│   │   ├── archetypes.rs   # 4 personality archetypes
│   │   ├── strategy.rs     # Strategy selection (expand/consolidate/compete/survive)
│   │   └── actions.rs      # AI action execution
│
├── gt-bridge/          # Shared bridge query trait
│   ├── src/
│   │   └── lib.rs          # BridgeQuery trait, InfraArrays, EdgeArrays structs
│
├── gt-wasm/            # WASM bindings for browser
│   ├── src/
│   │   └── lib.rs          # wasm-bindgen entry, JSON queries, typed array exports, BridgeQuery impl
│
├── gt-tauri/           # Tauri native bridge for desktop
│   ├── src/
│   │   └── lib.rs          # TauriBridge struct, BridgeQuery impl, cmd_new_game factory
│
└── gt-server/          # Multiplayer server
    ├── src/
    │   ├── main.rs         # Server entry point
    │   ├── ws.rs           # WebSocket handling, rate limiting, spatial validation, speed votes
    │   ├── routes.rs       # HTTP API (worlds, auth, admin ban/unban)
    │   ├── state.rs        # WorldInstance, AppState, player/vote/ban tracking
    │   ├── tick.rs         # Tick processing, snapshot intervals
    │   ├── auth.rs         # Authentication
    │   └── persistence.rs  # PostgreSQL save/load

desktop/src-tauri/      # Tauri desktop app (separate Cargo project)
    ├── src/
    │   └── main.rs         # SimState, 16 Tauri commands (filesystem + native sim)
    ├── tauri.conf.json     # Tauri v2 config
    └── Cargo.toml          # Depends on gt-tauri, gt-bridge, gt-common
```

---

## 3. Frontend Architecture (Svelte + deck.gl + D3.js)

### 3a. Svelte App Structure

```
web/src/
├── App.svelte              # Root component, route handling
├── lib/
│   ├── wasm/
│   │   ├── bridge.ts       # TypeScript bindings to Rust WASM
│   │   ├── commands.ts     # Send player actions to sim
│   │   └── queries.ts      # Query game state from sim
│   ├── game/
│   │   ├── GameView.svelte         # Main game screen container
│   │   ├── MapRenderer.ts           # deck.gl map renderer (free placement)
│   │   ├── HUD.svelte              # Heads-up display overlay
│   │   ├── SpeedControls.svelte    # Pause/play/speed buttons
│   │   ├── AdvisorPanel.svelte     # AI advisor suggestions
│   │   └── NotificationFeed.svelte # Event notifications
│   ├── panels/                             # 6 tabbed panel groups
│   │   ├── finance/
│   │   │   ├── DashboardPanel.svelte       # Corporate financial dashboard
│   │   │   ├── PricingPanel.svelte         # Regional pricing strategy
│   │   │   └── InsurancePanel.svelte       # Infrastructure insurance
│   │   ├── operations/
│   │   │   ├── InfraPanel.svelte           # Infrastructure management
│   │   │   ├── MaintenancePanel.svelte     # Maintenance priorities
│   │   │   ├── RepairPanel.svelte          # Repair queue and scheduling
│   │   │   ├── WorkforcePanel.svelte       # Employee/team management
│   │   │   └── BuildMenu.svelte            # Infrastructure build menu
│   │   ├── diplomacy/
│   │   │   ├── AlliancePanel.svelte        # Alliance management and proposals
│   │   │   ├── LegalPanel.svelte           # Lawsuits and legal actions
│   │   │   ├── IntelPanel.svelte           # Espionage and intelligence
│   │   │   └── CoOwnershipPanel.svelte     # Shared infrastructure voting
│   │   ├── research/
│   │   │   ├── TechTreePanel.svelte        # Tech tree and R&D
│   │   │   └── PatentPanel.svelte          # Patent filing and licensing
│   │   ├── market/
│   │   │   ├── ContractPanel.svelte        # Contract negotiation
│   │   │   ├── AuctionPanel.svelte         # Spectrum and asset auctions
│   │   │   ├── MergerPanel.svelte          # Mergers and acquisitions
│   │   │   ├── GrantPanel.svelte           # Government grants
│   │   │   └── SubsidiaryPanel.svelte      # Subsidiary management
│   │   └── info/
│   │       ├── RegionPanel.svelte          # Regional overview
│   │       ├── AdvisorPanel.svelte         # AI advisor suggestions
│   │       └── AchievementPanel.svelte     # Achievements and milestones
│   ├── menu/
│   │   ├── MainMenu.svelte         # Title screen
│   │   ├── NewGame.svelte          # New game setup
│   │   ├── LoadGame.svelte         # Save slot selection
│   │   ├── WorldBrowser.svelte     # Multiplayer world browser
│   │   └── Settings.svelte         # Game settings
│   ├── charts/
│   │   ├── FinanceChart.svelte     # D3 revenue/expense chart
│   │   ├── PopulationChart.svelte  # D3 population graph
│   │   ├── NetworkDiagram.svelte   # D3 network topology view
│   │   └── MarketShare.svelte      # D3 market share pie/bar
│   └── ui/
│       ├── Button.svelte           # Reusable button
│       ├── Panel.svelte            # Dark panel container
│       ├── Table.svelte            # Data table
│       ├── Tooltip.svelte          # Hover tooltip
│       ├── Modal.svelte            # Modal dialog
│       └── Slider.svelte           # Range slider
├── stores/
│   ├── game.ts             # Game state store (reads from WASM)
│   ├── ui.ts               # UI state (active panel, selected entity)
│   └── settings.ts         # User preferences
└── static/
    ├── icons/              # Infrastructure and UI icons
    ├── fonts/              # Typography
    └── data/               # Map data, country borders (GeoJSON)
```

### 3b. deck.gl Map Renderer

The map is rendered using deck.gl with WebGL layers. Infrastructure is placed freely at exact (lon, lat) coordinates — no grid snapping.

**Layers (rendered by deck.gl):**
1. **Land layer** — `ScatterplotLayer` rendering grid cells as dark base map (filtered to exclude ocean)
2. **Region borders** — `PathLayer` for region boundary outlines
3. **Infrastructure edges** — `ArcLayer` for connections between nodes (colored by edge type)
4. **Infrastructure nodes** — `ScatterplotLayer` for nodes (colored by corporation, sized by tier)
5. **Cities** — `ScatterplotLayer` with additive blending for city glow effect
6. **Pathfinding preview** — `PathLayer`/`ArcLayer` for edge building preview

**Build system:** Click anywhere on land → `map-clicked` event with exact (lon, lat) → BuildMenu shows options → `BuildNode { node_type, lon, lat }` command sent to WASM.

**Zoom levels control visibility:**
- Level 1 (World): continents, major countries, backbone routes
- Level 2 (Country): regions, major cities, national routes
- Level 3 (Region): all cities, regional infrastructure, local routes
- Level 4 (City): individual assets, fiber routes, tower placements

### 3c. WASM Bridge

The bridge between Svelte/JS and Rust/WASM uses `wasm-bindgen` and follows this pattern:

```
                    ┌─────────────┐
                    │   Svelte    │
                    │   Component │
                    └──────┬──────┘
                           │ calls
                    ┌──────┴──────┐
                    │  bridge.ts  │  TypeScript wrapper
                    └──────┬──────┘
                           │ wasm-bindgen
                    ┌──────┴──────┐
                    │  gt-wasm    │  Rust WASM module
                    │  (queries & │
                    │   commands) │
                    └──────┬──────┘
                           │ direct access
                    ┌──────┴──────┐
                    │  ECS World  │  Full simulation state
                    └─────────────┘
```

**Commands (player actions → sim):**
- `build_node(type, lon, lat)` → place infrastructure at exact coordinates
- `build_edge(type, node_a, node_b, corp_id)` → connect nodes
- `hire_employee(role, region, corp_id)` → hire workforce
- `set_policy(policy_type, params, corp_id)` → set management policy
- `take_loan(amount, corp_id)` → financial action
- `propose_contract(terms, target_corp)` → business deal
- `set_research(tech_id, corp_id)` → start researching
- `set_speed(multiplier)` → game speed
- `toggle_pause()` → pause/resume
- `save_game(slot)` / `load_game(slot)` → persistence
- `file_patent(tech_id, corp_id)` → file a patent on researched tech
- `request_license(patent_id, corp_id)` → request license from patent holder
- `set_license_price(patent_id, price)` → set royalty rate for a patent
- `revoke_license(patent_id, licensee_corp)` → revoke a license agreement
- `start_independent_research(tech_id, corp_id)` → research around existing patents
- `propose_alliance(target_corp, terms)` → propose a new alliance
- `accept_alliance(alliance_id, corp_id)` → accept an alliance proposal
- `dissolve_alliance(alliance_id, corp_id)` → leave or dissolve an alliance
- `alliance_vote(alliance_id, proposal_id, vote)` → vote on alliance decisions
- `file_lawsuit(target_corp, type, damages)` → initiate legal action
- `settle_lawsuit(lawsuit_id, terms)` → propose settlement
- `defend_lawsuit(lawsuit_id, strategy)` → choose defense strategy
- `bid_for_grant(grant_id, corp_id, proposal)` → bid on government grant
- `complete_grant(grant_id, corp_id)` → submit grant completion
- `set_region_pricing(region_id, corp_id, tiers)` → set per-region pricing tiers
- `set_maintenance_priority(entity_id, priority_tier)` → set asset maintenance priority

**Queries (sim → UI):**
- `get_visible_entities(viewport)` → entities in current map view (fog of war filtered)
- `get_corporation_data(corp_id)` → financial summary
- `get_region_data(region_id)` → regional economy
- `get_infrastructure_list(corp_id)` → owned assets
- `get_workforce(corp_id)` → employee roster
- `get_contracts(corp_id)` → active contracts
- `get_research_state(corp_id)` → tech tree progress (freely explorable, not era-gated)
- `get_notifications()` → recent events (priority levels: Critical/Important/Info + category filters)
- `get_advisor_suggestion()` → AI advisor recommendation
- `get_patent_data(corp_id)` → owned patents, active licenses, license revenue
- `get_alliance_data(corp_id)` → alliance membership, trust scores, revenue share
- `get_intel_data(corp_id)` → fog of war intel levels per region/competitor
- `get_lawsuit_data(corp_id)` → active lawsuits (filed and received)
- `get_grant_data(region_id)` → available government grants and progress
- `get_pricing_data(corp_id)` → per-region pricing tiers and revenue impact

---

## 4. Multiplayer Architecture

### 4a. Client-Server Model

```
                    ┌──────────────┐
        ┌──────────►│  Browser A   │ (thin client)
        │           └──────────────┘
        │
┌───────┴───────┐   ┌──────────────┐
│  Game Server  │◄──►│  Browser B   │ (thin client)
│  (Rust native)│   └──────────────┘
│               │
│  Authoritative│   ┌──────────────┐
│  simulation   │◄──►│  Tauri App C │ (thin client)
└───────┬───────┘   └──────────────┘
        │
        ▼
┌───────────────┐
│  PostgreSQL   │
└───────────────┘
```

**Server authority:** The server runs the simulation. Clients send commands, server validates and executes, broadcasts state deltas to all clients. Per-player event filtering enforced server-side based on espionage intel level. Infrastructure is always visible to all players.

**Client rendering:** Clients receive event-driven delta broadcasts (CommandBroadcast) and periodic TickUpdates. In multiplayer, the client's WASM module receives incremental state updates via `applyBatch()` (DeltaOps), not full snapshots. Full snapshots sent every 30 ticks as safety net. Pure thin client — no WASM tick execution in MP.

**Optimistic UI:** Builder sees instant ghost entity (translucent). Server confirms/rejects via CommandAck. Other players see confirmed entity immediately via CommandBroadcast + applyBatch. Sub-200ms latency for all players.

**Single-player:** The WASM module IS the server. Same simulation code, running in the browser. Commands go directly to WASM, no network needed.

**Desktop (Tauri):** WASM runs in Tauri's webview for simulation (same as browser). Tauri IPC provides native filesystem access for saves. Native sim commands exist in desktop/src-tauri for future async adoption.

### 4b. WebSocket Protocol

Messages are serialized with MessagePack (compact binary) or JSON (debug mode).

**Client → Server:**
```
GameCommand { seq: 123, command: BuildNode { CellTower, 34.05, -118.25 } }
GameCommand { seq: 124, command: SetSpeed("Fast") }  // Speed vote in MP
```

**Server → Client:**
```
CommandAck { success: true, seq: 123, entity_id: 1337, effective_tick: 500, error: null }
CommandBroadcast { tick: 500, player_id, corp_id, ops: [NodeCreated { ... }] }
SpeedVoteUpdate { votes: [...], deadline: 530, current_speed: "Normal" }
TickUpdate { tick: 501, world_info, events: [...] }  // Per-tick state (filtered per player)
FullSnapshot { tick: 530, state: "..." }              // Every 30 ticks (safety net)
```

**DeltaOp types (CommandBroadcast payload):**
```
NodeCreated { entity_id, owner, node_type, network_level, lon, lat, under_construction }
EdgeCreated { entity_id, owner, edge_type, from_node, to_node }
NodeUpgraded { entity_id, node_type }
NodeRemoved { entity_id }
EdgeRemoved { entity_id }
ConstructionCompleted { entity_id }
```

**Anti-cheat:**
- Per-type rate limiting: Build 3/sec, Financial 2/sec, Research 1/5sec, Espionage 1/30sec
- Server-side spatial validation: finite coords, world bounds (-180/180, -90/90)
- Sequence number dedup: reject if seq <= last_seq per player
- Speed control: world creator has override power, others need majority vote (30s window)

**Admin:** `POST /api/admin/ban`, `POST /api/admin/unban` endpoints. Ban check on JoinWorld.

### 4c. Fog of War (Server-Side Filtering)

In multiplayer mode, the server filters all state deltas and snapshots before sending to each client. Each client only receives:
- Full visibility of their own corporation's infrastructure and financials
- Geography data (terrain, regions, cities, borders, population) — always visible to all
- Competitor data filtered by intel level (None → Basic → Full), gathered through espionage
- Intel decays over 50 ticks unless refreshed
- Alliance members automatically share Basic intel on covered regions

The `get_visible_entities()` query respects fog of war in both SP and MP modes.

### 4d. AI Proxy (Offline Management)

When a player disconnects from a multiplayer world:
1. Server marks their corporation as "AI-managed"
2. AI proxy activates using the player's saved policies
3. AI proxy does NOT make strategic changes — only executes existing policies:
   - Maintains infrastructure (if budget allows)
   - Processes contract renewals (renew existing, don't create new)
   - Pays debts on schedule
   - Responds to disasters with repair crews (if available)
4. When player reconnects: AI proxy deactivates, player resumes control
5. Player gets a summary of what happened while away

### 4e. Research & Era Philosophy

**Research is freely explorable:** The tech tree is gated by prerequisites only, NOT by era. Players can research any technology they meet the prereqs for, regardless of the current world era. Tech functions as a primary economic commodity — it can be patented, licensed, leased, or open-sourced.

**World era is a cosmetic milestone:** The world era advances when ALL corporations in the world have completed at least one technology from that era. It is a collective achievement indicator with no gameplay effects — it does not gate content or restrict actions.

**Patent hard block:** Patented technology cannot be used by non-holders without a license. Attempting to build patent-protected infrastructure without a license is rejected by the command validator.

**Independent research workaround:** Corporations can bypass patents by independently researching at 150% cost (gains access, cannot patent) or 200% cost (gains improved version with +10% bonus, CAN patent the improvement).

---

## 5. Data Architecture

### 5a. PostgreSQL Schema (Key Tables)

```sql
-- Worlds
worlds (id, name, config, created_at, tick_count, state_snapshot)

-- Accounts
accounts (id, email, password_hash, display_name, created_at)

-- Player-World association
player_worlds (account_id, world_id, corporation_id, last_seen, policies_json)

-- Cloud saves (single-player)
cloud_saves (account_id, slot_name, save_data, created_at, updated_at)

-- World state is stored as a serialized blob per-world
-- The Rust ECS serializes to/from this blob
-- Individual queries go through the game server, not direct DB
```

### 5b. Save Format

Single-player saves serialize the entire ECS world to a binary format:
- Header: version, timestamp, world config
- Entity table: all entity IDs and their component bitmasks
- Component arrays: contiguous arrays per component type
- Compression: zstd compression on the binary blob

Cloud saves use the same format, stored as a blob in PostgreSQL.

### 5c. Save Migration

Save files include a version header. When loading a save with an older version:
- The loader applies sequential migration steps (v1→v2→v3→...) to upgrade the save format
- New components added in later versions get default values
- Removed components are stripped during migration
- If migration fails, the user is offered the option to delete or attempt recovery

---

## 6. Hosting Architecture

### Current (Development)
```
Players ──► Fly.io (Rust game server binary)
                │
                ▼
           PostgreSQL (world state, accounts, cloud saves)

Frontend ──► Vercel (Svelte app CDN)
```

### Target (Production)
```
Players ──► Cloudflare Workers (auth, matchmaking, APIs, CDN)
                │
                ▼
           Hetzner (Rust game server binary × 1-5 instances)
                │
                ▼
           PostgreSQL (world state, accounts, cloud saves)

Frontend ──► Cloudflare (static assets CDN)
```

- **Dev:** Fly.io for game server + Vercel for frontend
- **Prod:** Hetzner Dedicated AX42 (Ryzen 7 7700, 64GB RAM) for 3-5 sim instances + Cloudflare Workers for service layer
- **No AWS, no Azure, no Oracle**

---

## 7. Open Data Pipeline

### 6a. Earth Map Data

**Geography (OpenStreetMap):**
- Country borders → GeoJSON polygons
- State/province borders → GeoJSON polygons
- Major cities → point coordinates + population metadata
- Terrain classification from elevation data (SRTM)

**Economics (World Bank / UN):**
- GDP per capita by country
- Population by country/city
- Internet penetration rates
- Urbanization rates
- Political stability indices

**Pre-processing pipeline:**
1. Download raw data from open sources
2. Process into game-ready format (simplified GeoJSON, normalized economics)
3. Store in `data/` directory as static assets
4. Loaded at game start when "Real Earth" mode is selected

### 6b. Procedural World Generation

When "Procedural World" is selected:
1. Generate continental landmasses (noise-based)
2. Place mountain ranges, rivers, coastlines
3. Generate countries with capitals
4. Distribute population based on terrain suitability
5. Initialize economic indicators from terrain and population
6. All deterministic from a single world seed

---

## 8. Performance Targets

- **Simulation tick:** < 50ms for world with 10,000+ entities
- **Map rendering:** 60fps at all zoom levels with 100,000+ visible entities
- **WASM module size:** < 5MB (gzipped) for browser loading
- **Initial page load:** < 3 seconds on broadband
- **WebSocket latency:** < 100ms round-trip for player commands
- **Save file size:** < 50MB for a mature game world
- **Memory usage:** < 500MB in browser for large worlds

---

## 7A. Hosting Architecture

**Current (dev):**
```
Players ──► Vercel (Svelte frontend CDN)
                │
                ▼
           Fly.io (Rust game server)
                │
                ▼
           PostgreSQL (world state, accounts, cloud saves)
```

**Production target:**
```
Players ──► Cloudflare Workers (auth, matchmaking, APIs, CDN)
                │
                ▼
           Hetzner (Rust game server binary × 1-5 instances)
                │
                ▼
           PostgreSQL (world state, accounts, cloud saves)
```

- **Dev:** Fly.io (game server) + Vercel (frontend CDN) + PostgreSQL
- **Prod:** Hetzner Dedicated AX42 (Ryzen 7 7700, 64GB RAM, ~€57/month, runs 3-5 sim instances)
- **Service layer:** Cloudflare Workers (free tier: 100k req/day, paid $5/month: 10M req)
- **Frontend CDN:** Vercel (Svelte app) + Cloudflare (static assets)

---

## 9. SVG Asset Pipeline

All visual assets (icons, symbols, indicators) use inline SVG with a unified pipeline from source files through Svelte UI and deck.gl map rendering.

### Directory Structure

```
web/src/lib/assets/icons/
├── infrastructure/          # Node type icons (7)
│   ├── central-office.svg
│   ├── exchange-point.svg
│   ├── cell-tower.svg
│   ├── data-center.svg
│   ├── satellite-ground.svg
│   ├── submarine-landing.svg
│   └── wireless-relay.svg
├── edges/                   # Edge type icons (5)
│   ├── fiber-optic.svg
│   ├── copper.svg
│   ├── microwave.svg
│   ├── satellite.svg
│   └── submarine.svg
├── ui/                      # UI icons (12)
│   ├── pause.svg
│   ├── play.svg
│   ├── fast-forward.svg
│   ├── ultra-speed.svg
│   ├── save.svg
│   ├── money.svg
│   ├── research.svg
│   ├── workforce.svg
│   ├── contract.svg
│   ├── settings.svg
│   ├── warning.svg
│   └── dashboard.svg
└── index.ts                 # Icon registry (typed exports)
```

### SVG Conventions

- **ViewBox:** All icons use `viewBox="0 0 64 64"` for consistent detail resolution
- **Color:** All icons use `currentColor` or `fill="currentColor"` for runtime recoloring
- **Detail levels:** Icons use opacity layers (0.15-0.9) and `fill="#fff"` cutouts for depth
- **No external dependencies:** No fonts, gradients defs, or external references
- **Naming:** Kebab-case matching Rust enum variants (e.g., `cell-tower` maps to `CellTower`)

### Svelte UI Integration

Icons are imported as raw strings via Vite's `?raw` suffix and rendered inline:

```svelte
<script>
  import Icon from '$lib/components/Icon.svelte';
</script>

<Icon name="cell-tower" size={32} color="#00d4ff" />
<Icon name="warning" size={16} color="#ffaa00" title="Network overloaded" />
```

**Icon.svelte props:**
| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `name` | `IconName` | required | Icon key from registry |
| `size` | `number` | `24` | Rendered size in pixels |
| `color` | `string` | `'currentColor'` | Fill/stroke color |
| `class` | `string` | `''` | Additional CSS classes |
| `title` | `string` | `undefined` | Accessible label (adds `role="img"`) |

### deck.gl Map Integration

The map renderer uses deck.gl layers. Infrastructure nodes are rendered as `ScatterplotLayer` points colored by corporation. Edges use `ArcLayer` connecting source/target positions.

**Free Placement:** Players click anywhere on the map → deck.gl provides exact (lon, lat) coordinates via `info.coordinate` → `BuildNode { node_type, lon, lat }` command → nearest grid cell looked up for terrain/region data → node placed at exact clicked position.

**Icon Assets:** SVG infrastructure icons exist in `web/src/lib/assets/icons/infrastructure/` for future icon layer integration (currently rendered as colored dots via ScatterplotLayer).

### Adding New Icons

1. Create SVG file in the appropriate subdirectory (`infrastructure/`, `edges/`, or `ui/`)
2. Use `viewBox="0 0 64 64"` and `fill="currentColor"`
3. Add the `?raw` import to `web/src/lib/assets/icons/index.ts`
4. Add the key to the `icons` object and the appropriate category array
5. The `IconName` type updates automatically via `keyof typeof icons`

### Player Branding

Infrastructure icons on the map are rendered in the player's company color. The color is passed to `SpriteFactory` at rasterization time, replacing `currentColor` with the hex value. This means:
- No separate sprite sheets per player/AI corporation
- Color changes only require cache invalidation, not new assets
- AI corporations each get their own color, applied at the same pipeline stage
