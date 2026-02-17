# GlobalTelco: Technical Architecture

Comprehensive technical architecture for the GlobalTelco web-based infrastructure empire builder.

---

## 1. Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     BROWSER / TAURI                          │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │   Svelte UI  │  │   Three.js   │  │     D3.js        │  │
│  │  (panels,    │  │  (2D map     │  │  (charts, data   │  │
│  │   menus,     │  │   rendering) │  │   visualization) │  │
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
- `LandParcel` — ownable land units
- `TechResearch` — ongoing research projects
- `DebtInstrument` — loans, bonds

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

### 2b. Systems (Processing Order Per Tick)

Each economic tick, systems run in deterministic order:

1. `construction_system` — advance construction timers, complete builds
2. `maintenance_system` — check workforce vs maintenance needs, degrade unmaintained infra
3. `population_system` — update populations, migration, employment based on infrastructure
4. `demand_system` — calculate regional demand based on population and economy
5. `routing_system` — recalculate network routes if topology changed
6. `utilization_system` — calculate infrastructure utilization from routed demand
7. `revenue_system` — calculate per-corp revenue from served demand
8. `cost_system` — calculate maintenance, salary, interest costs
9. `finance_system` — update corporate finances (income, balance sheet, credit rating)
10. `contract_system` — process contract terms, renewals, breaches
11. `ai_system` — AI corporations make decisions (build, hire, contract, research)
12. `disaster_system` — roll for disasters, apply damage
13. `regulation_system` — process regulatory changes, political events
14. `research_system` — advance tech research progress
15. `market_system` — dynamic AI spawning, mergers, bankruptcies

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
├── gt-wasm/            # WASM bindings for browser
│   ├── src/
│   │   ├── lib.rs          # wasm-bindgen entry point
│   │   ├── bridge.rs       # JS ↔ Rust data bridge
│   │   ├── commands.rs     # Player action commands
│   │   └── queries.rs      # UI data queries
│
└── gt-server/          # Multiplayer server
    ├── src/
    │   ├── main.rs         # Server entry point
    │   ├── websocket.rs    # WebSocket handling
    │   ├── auth.rs         # Authentication
    │   ├── persistence.rs  # PostgreSQL save/load
    │   └── world_manager.rs # Multiple world management
```

---

## 3. Frontend Architecture (Svelte + Three.js + D3.js)

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
│   │   ├── MapRenderer.svelte      # Three.js map canvas wrapper
│   │   ├── HUD.svelte              # Heads-up display overlay
│   │   ├── SpeedControls.svelte    # Pause/play/speed buttons
│   │   ├── AdvisorPanel.svelte     # AI advisor suggestions
│   │   └── NotificationFeed.svelte # Event notifications
│   ├── panels/
│   │   ├── DashboardPanel.svelte   # Corporate financial dashboard
│   │   ├── InfraPanel.svelte       # Infrastructure management
│   │   ├── WorkforcePanel.svelte   # Employee/team management
│   │   ├── ResearchPanel.svelte    # Tech tree and R&D
│   │   ├── ContractPanel.svelte    # Contract negotiation
│   │   ├── RegionPanel.svelte      # Regional overview
│   │   └── BuildMenu.svelte        # Infrastructure build menu
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

### 3b. Three.js Map Renderer

The map is rendered using Three.js in orthographic 2D mode with a perspective camera for zoom transitions.

**Layers (rendered bottom to top):**
1. **Ocean base** — dark blue plane
2. **Land masses** — terrain-colored polygons (green/brown/white) from GeoJSON
3. **Political borders** — line geometry for country/region borders
4. **City dots** — scaled circles for population centers
5. **Infrastructure** — icon sprites for nodes, line geometry for edges
6. **Ownership overlay** — semi-transparent company-colored regions
7. **Selection highlight** — glow effect on hovered/selected entities
8. **Labels** — text sprites for city/region names at appropriate zoom levels

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
- `build_node(type, location, corp_id)` → place infrastructure
- `build_edge(type, node_a, node_b, corp_id)` → connect nodes
- `hire_employee(role, region, corp_id)` → hire workforce
- `set_policy(policy_type, params, corp_id)` → set management policy
- `take_loan(amount, corp_id)` → financial action
- `propose_contract(terms, target_corp)` → business deal
- `set_research(tech_id, corp_id)` → start researching
- `set_speed(multiplier)` → game speed
- `toggle_pause()` → pause/resume
- `save_game(slot)` / `load_game(slot)` → persistence

**Queries (sim → UI):**
- `get_visible_entities(viewport)` → entities in current map view
- `get_corporation_data(corp_id)` → financial summary
- `get_region_data(region_id)` → regional economy
- `get_infrastructure_list(corp_id)` → owned assets
- `get_workforce(corp_id)` → employee roster
- `get_contracts(corp_id)` → active contracts
- `get_research_state(corp_id)` → tech tree progress
- `get_notifications()` → recent events
- `get_advisor_suggestion()` → AI advisor recommendation

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

**Server authority:** The server runs the simulation. Clients send commands, server validates and executes, broadcasts state updates to all clients.

**Client rendering:** Clients receive state snapshots and deltas. They render the map and UI locally. No simulation runs on client in multiplayer mode.

**Single-player:** The WASM module IS the server. Same simulation code, running in the browser. Commands go directly to WASM, no network needed.

### 4b. WebSocket Protocol

Messages are serialized with MessagePack (compact binary) or JSON (debug mode).

**Client → Server:**
```
{ type: "command", action: "build_node", params: { ... }, seq: 123 }
{ type: "command", action: "set_speed", params: { multiplier: 4 }, seq: 124 }
```

**Server → Client:**
```
{ type: "tick", tick_number: 1234, delta: { ... } }  // State changes this tick
{ type: "ack", seq: 123, success: true }              // Command acknowledged
{ type: "event", event: { type: "disaster", ... } }   // Game event notification
{ type: "snapshot", state: { ... } }                   // Full state (on connect)
```

### 4c. AI Proxy (Offline Management)

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

---

## 6. Open Data Pipeline

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

## 7. Performance Targets

- **Simulation tick:** < 50ms for world with 10,000+ entities
- **Map rendering:** 60fps at all zoom levels with 100,000+ visible entities
- **WASM module size:** < 5MB (gzipped) for browser loading
- **Initial page load:** < 3 seconds on broadband
- **WebSocket latency:** < 100ms round-trip for player commands
- **Save file size:** < 50MB for a mature game world
- **Memory usage:** < 500MB in browser for large worlds

---

## 8. SVG Asset Pipeline

All visual assets (icons, symbols, indicators) use inline SVG with a unified pipeline from source files through Svelte UI and Three.js map rendering.

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

### Three.js Map Integration

For rendering infrastructure icons on the 2D political map, SVGs are rasterized to canvas textures via `SpriteFactory`:

```typescript
import { createIconSprite, preloadInfrastructureIcons, clearTextureCache } from '$lib/game/SpriteFactory';

// Single sprite
const tower = await createIconSprite('cell-tower', {
  size: 128,             // Rasterization resolution (px)
  color: '#00ff88',      // Player's company color
  worldSize: 2,          // Size in Three.js world units
});
scene.add(tower);

// Preload all infrastructure icons at game start
const textures = await preloadInfrastructureIcons('#ffffff', 64);

// Clear cache on theme/color change
clearTextureCache();
```

**Pipeline:** SVG string → `currentColor` replacement → Blob URL → Image → Canvas → `THREE.CanvasTexture` → `THREE.SpriteMaterial` → `THREE.Sprite`

**Caching:** Textures are cached by `name:size:color:padding` key. Same combo returns the same texture instance. Call `clearTextureCache()` when player changes company color or UI theme.

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
