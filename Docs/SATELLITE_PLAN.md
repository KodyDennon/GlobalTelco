# Satellite System Implementation Plan

> **Note:** File paths in this plan reflect the structure at time of writing. Since then, large files have been modularized into subdirectories (e.g., `types.rs` → `types/`, `world.rs` → `world/`). See `Docs/technical_architecture.md` Section 2c for the current crate structure.

## Context

GlobalTelco currently has satellite "ground stations" (SatelliteGround, SatelliteGroundStation, LEO_SatelliteGateway) that magically provide coverage — no actual satellites exist in orbit. This plan adds a full satellite vertical: orbital mechanics with moving satellites, constellation management, manufacturing supply chain (satellite + terminal factories), launch logistics (contract + own rockets), customer terminal distribution, orbital debris/Kessler syndrome, and AI satellite operators. Inspired by Starlink/HughesNet operations and SpaceX tycoon gameplay.

## Design Decisions (User-Confirmed)

| Decision | Choice |
|---|---|
| Orbital model | Hybrid Keplerian: circular orbits (fast trig) + simplified decay/fuel |
| Orbit types | LEO + MEO + GEO + HEO (4 tiers) |
| Coverage model | Cone projection + capacity limit + **real network routing** (sat must uplink to ground station) |
| Inter-satellite links | Auto-mesh in-plane (2 neighbors) + cross-plane via research |
| Ground stations | Must connect to separate POP via terrestrial fiber (not self-backhauled) |
| Manufacturing | Factory + tiers (Small Batch -> Standard -> Mass Production) |
| Launch system | Contract launches first, build own launch pads + reusability research |
| Customer terminals | Factory -> Regional Warehouse -> City adoption |
| Competition model | Complement terrestrial (rural/remote focus, not urban competitor) |
| Satellite lifecycle | Fuel consumption, decay, deorbit/service, debris accumulation |
| Debris mechanic | Kessler syndrome affects ALL operators per shell (competitive weapon via negligence) |
| Revenue model | Both retail subscribers (via terminals) + wholesale bandwidth |
| Constellation scale | Medium: 200-1000 sats for LEO, 20-50 MEO, 3-8 GEO |
| Regulation | Spectrum only (new satellite bands via existing auction system) |
| Sabotage | Ground-only (factories, warehouses, ground stations). No ASAT. |
| AI | Full satellite ops + new Satellite Pioneer archetype + AI launch providers |
| Era unlock | Research-gated, no era restriction (matches existing design rule) |
| Rendering | Orbital overlay on deck.gl (globe projection already exists at zoom < 2.5) |

## Architecture: Satellites as Moving InfraNodes

**Core insight**: A satellite in orbit IS an `InfraNode` entity with:
- A `Satellite` component (orbital parameters, fuel, status)
- A `Position` component updated each tick by the orbital system
- Dynamic `InfraEdge` connections (ISL + ground station downlinks) created/destroyed each tick

Traffic routing: **Terminal -> Nearest Overhead Sat -> ISL Laser Links -> Sat Over Ground Station -> Ground Station Node -> Fiber Edge -> POP Node -> Internet Backbone**

This means satellites participate in the real `NetworkGraph` — no magic coverage blobs.

---

## Phase S1: Foundation (Orbital Mechanics + Core Types)

### New types in `crates/gt-common/src/types.rs`
- New enums: `OrbitType { LEO, MEO, GEO, HEO }`, `SatelliteStatus { Manufacturing, AwaitingLaunch, Operational, Decaying, Deorbiting, Dead }`, `RocketType { Small, Medium, Heavy, SuperHeavy }`, `FactoryTier { SmallBatch, StandardProduction, MassProduction }`
- New NodeType variants: `GEO_Satellite`, `MEO_Satellite`, `LEO_Satellite`, `HEO_Satellite`
- Add all match arms: `display_name()`, `tier()`, `coverage_radius_km()`, `is_wireless()`, `era()`, `max_throughput()`, `construction_cost()`, etc.

### New component: `crates/gt-simulation/src/components/satellite.rs`
```
Satellite { orbit_type, altitude_km, base_altitude_km, inclination_deg, ascending_node_deg,
            mean_anomaly_deg, fuel_remaining, fuel_capacity, station_keeping_rate, status,
            constellation_id, mass_kg, max_isl_links, coverage_cone_half_angle_deg, launched_tick }
```

### New component: `crates/gt-simulation/src/components/constellation.rs`
```
Constellation { name, owner, orbit_type, target_altitude_km, target_inclination_deg,
                num_planes, sats_per_plane, satellite_ids, operational_count }
```

### New system: `crates/gt-simulation/src/systems/orbital.rs`
- Runs after `construction`, before `satellite_network` (new) in tick order
- For each operational satellite: advance `mean_anomaly_deg` based on orbital period `T = 2*PI * sqrt((R+alt)^3 / GM)`
- Calculate sub-satellite point (lon, lat) from inclination + ascending node + mean anomaly (simple trig)
- Update `Position` component
- Consume station-keeping fuel; if empty, set status=Decaying, begin altitude loss
- If altitude < minimum viable: set Dead, add to debris

### GameWorld additions in `crates/gt-simulation/src/world.rs`
- `satellites: HashMap<EntityId, Satellite>`
- `constellations: HashMap<EntityId, Constellation>`
- `orbital_shells: Vec<OrbitalShell>` (initialized with standard altitude bands: LEO-low/mid/high, MEO-low/high, GEO, HEO)
- `dynamic_satellite_edges: Vec<EntityId>` (cleared and rebuilt each tick)

### New commands
- `BuildConstellation { name, orbit_type, num_planes, sats_per_plane, altitude_km, inclination_deg }`
- `DeorbitSatellite { satellite }`
- Temporary: direct satellite spawning for testing (skip manufacturing/launch)

---

## Phase S2: Network Integration (ISL + Downlinks + Routing)

### New EdgeType variants in `crates/gt-common/src/types.rs`
- `SatelliteDownlink` (sat -> ground station, dynamic)
- `IntraplaneISL` (in-plane inter-satellite link, auto-formed)
- `CrossplaneISL` (cross-plane link, requires research)
- `GEO_SatelliteLink`, `MEO_SatelliteLink` (semi-permanent ground links)

### New system: `crates/gt-simulation/src/systems/satellite_network.rs`
- Runs after `orbital`, before `maintenance` in tick order
- Each tick: remove all edges in `dynamic_satellite_edges`, then rebuild:
  - **In-plane ISL**: connect each sat to 2 nearest in same constellation plane (within max range)
  - **Cross-plane ISL** (if researched): connect to nearest sats in adjacent planes
  - **Ground station downlink**: connect to nearest owned/allied ground station within line-of-sight
- Add edges to `NetworkGraph` via `add_edge_with_id()`, mark affected nodes dirty
- **Optimization**: only mark nodes dirty when connectivity topology changes (not every position update)

### New ground station NodeType variants
- `LEO_GroundStation` (Era 5, Backbone tier), `MEO_GroundStation` (Era 4, Backbone tier)
- Ground stations MUST connect to a separate POP (FiberPOP, ExchangePoint, DataCenter) via fiber — **not self-backhauled**

### Modify coverage system (`crates/gt-simulation/src/systems/coverage.rs`)
- Satellite nodes: use dynamically updated `Position` (not fixed `cell_index`) for coverage center
- Coverage radius from altitude: `footprint_radius = f(altitude_km, min_elevation_deg)`
- Satellite coverage gated on having a path to a ground station through the network graph (no magic)

### Routing: No code changes needed
- `NetworkGraph` already supports dynamic add/remove; Dijkstra runs on whatever edges exist

### Performance (1000 sats)
- Position calc: ~5us/tick (negligible)
- Edge rebuild: ~5000 HashMap ops = ~50us/tick
- Dirty node routing: only ~10-50 nodes per tick (connectivity changes, not position changes)

---

## Phase S3: Manufacturing + Launch

### New NodeType variants
- `SatelliteFactory` (Era 4, Core tier, 100M cost)
- `TerminalFactory` (Era 5, Aggregation tier, 20M cost)
- `SatelliteWarehouse` (Era 5, Aggregation tier, 5M cost)
- `LaunchPad` (Era 4, Backbone tier, 200M cost)

### New components
- `SatelliteFactory { tier, production_rate, production_progress, queue, owner }`
- `TerminalFactory { tier, production_rate, production_progress, produced_stored, owner }`
- `Warehouse { region_id, terminal_inventory, distribution_rate, owner }`
- `LaunchPad { owner, launch_queue, cooldown_ticks, reusable }`

### New system: `crates/gt-simulation/src/systems/manufacturing.rs`
- Runs after `ftth`, before `launch`
- Satellite factories: accumulate production, create satellite entities with status=AwaitingLaunch
- Terminal factories: accumulate production, increment `produced_stored`

### New system: `crates/gt-simulation/src/systems/launch.rs`
- Runs after `manufacturing`
- Process launch queue: check payload weight vs rocket capacity
- Reliability roll (Small=90%, Medium=93%, Heavy=95%, SuperHeavy=97%)
- Success: satellites go Operational, add to network graph
- Failure: satellites destroyed, debris added, cost lost
- Contract launches: same mechanics, flat fee, no owned pad needed
- Reusability research: -65% cost, -40% cooldown

### New commands
- `OrderSatellites { factory, constellation_id, count }`
- `ScheduleLaunch { launch_pad, rocket_type, satellites }`
- `ContractLaunch { rocket_type, satellites }`

---

## Phase S4: Terminals + Revenue

### New component: `SatelliteSubscription { city_id, corp_id, subscribers, terminals_deployed, monthly_rate }`

### New system: `crates/gt-simulation/src/systems/terminal_distribution.rs`
- Runs after `launch`
- Warehouses distribute terminals to cities in their region
- City adoption = min(terminals_available, demand, satellite_coverage)

### New system: `crates/gt-simulation/src/systems/satellite_revenue.rs`
- Runs after `terminal_distribution`, before existing `revenue`
- Retail: `revenue = subscribers * monthly_rate * quality_factor`
- Quality factor = f(satellite_capacity / users_in_beam, latency)
- Wholesale: bandwidth contracts with other operators
- Rural/remote cities: high demand, low terrestrial competition

### New commands
- `OrderTerminals { factory, count }`, `ShipTerminals { factory, warehouse, count }`
- `SetSatellitePricing { region, monthly_rate }`

### Modify demand system (`crates/gt-simulation/src/systems/demand.rs`)
- Add satellite-specific demand for underserved areas

---

## Phase S5: Debris + Servicing

### Orbital debris tracking on GameWorld
```
OrbitalShell { min_altitude_km, max_altitude_km, debris_count, collision_probability,
               kessler_threshold, cascade_active }
```
Standard shells: LEO-low (200-600km), LEO-mid (600-1200km), LEO-high (1200-2000km), MEO-low (2000-10000km), MEO-high (10000-35786km), GEO (35786km), HEO (200-40000km)

### New system: `crates/gt-simulation/src/systems/debris.rs`
- Runs after `disaster`
- Per shell: collision probability = `debris_count * base_rate`
- Roll per operational satellite; collision = satellite destroyed + 3-10 new debris fragments
- **Kessler cascade**: above threshold, debris grows exponentially, shell becomes unusable for ALL operators
- Dead satellites not actively deorbited add 1 debris each
- Natural decay at very low LEO altitudes only

### New system: `crates/gt-simulation/src/systems/servicing.rs`
- Runs after `debris`
- Process active service missions: refuel (reset fuel) or repair (restore health)
- Command: `ServiceSatellite { satellite, service_type: "Refuel"|"Repair" }`

---

## Phase S6: AI Satellite Pioneer

### New archetype in `crates/gt-common/src/types.rs`
- `AIArchetype::SatellitePioneer`

### New AI module: `crates/gt-simulation/src/systems/ai/satellite.rs`
- Prioritizes satellite research, builds factories + launch pads early
- Targets rural/underserved regions with no terrestrial coverage
- Strategy modes: Expand (launch more sats), Consolidate (fill gaps), Compete (undercut pricing)
- AI as launch provider: accepts contract launch proposals, prices based on cost + margin

### Modify existing archetypes
- TechInnovator + AggressiveExpander: opportunistic satellite building when underserved markets exist

---

## Phase S7: Frontend

### Orbital overlay layers in `web/src/lib/game/map/MapRenderer.ts`
- OrbitalPathLayer (PathLayer): orbital ground tracks as arcs, color by owner
- SatelliteIconLayer (IconLayer): satellite positions as moving dots
- CoverageFootprintLayer (PolygonLayer): semi-transparent coverage cones
- ISL_LinkLayer (LineLayer): active inter-satellite links
- DownlinkLayer (LineLayer): satellite-to-ground-station connections
- Connection visualization to cities/terminals

### New panels in `web/src/lib/panels/`
- **SatellitePanel.svelte**: constellation manager (sat count, health/fuel, coverage %)
- **LaunchPanel.svelte**: rocket catalog, launch pad status, schedule launches, history
- **TerminalPanel.svelte**: factory production, warehouse inventory, city adoption rates
- **DebrisPanel.svelte**: per-shell debris (D3 bar chart), Kessler warnings, collision probability

### New bridge queries in `crates/gt-bridge/src/lib.rs`
- `get_constellation_data(corp_id)`, `get_orbital_view()`, `get_satellite_coverage()`
- `get_launch_schedule(corp_id)`, `get_terminal_inventory(corp_id)`, `get_debris_status()`
- Hot-path typed array: `get_satellite_arrays() -> SatelliteArrays { ids, owners, positions, altitudes, orbit_types, statuses, fuel_levels }`

### Modified panels
- ResearchPanel: satellite tech tree
- DashboardPanel: satellite metrics
- BuildMenu/BuildHotbar: satellite infrastructure nodes
- Overlay selector: add "Satellite" toggle

---

## Phase S8: Polish + Integration

- Spectrum system: new satellite bands (Ku, Ka, V, Q) via existing auction system
- Covert ops: ground station/factory/warehouse sabotage targets
- Achievement system: "First Satellite", "Constellation", "Mega Constellation", "Global Coverage", "Kessler Survivor"
- Weather: severe storms damage ground stations (existing weather damage, no rain fade)
- DeltaOp: `SatelliteLaunched`, `SatelliteRemoved` for multiplayer broadcasts
- Rate limiting: satellite commands in gt-server
- Research tree expansion: ~8 new techs (GEO Design, MEO Constellation, Manufacturing I/II, Rocket Reusability, HEO Design, Cross-plane ISL, Servicing, Terminal Mass Prod, Advanced Ground Station)

---

## Updated Tick Order (28 -> 36 systems)

```
 1. construction             (existing)
 2. orbital                  *** NEW
 3. satellite_network        *** NEW
 4. maintenance              (existing)
 5. population               (existing)
 6. coverage                 (existing, MODIFIED)
 7. demand                   (existing, MODIFIED)
 8. routing                  (existing)
 9. utilization              (existing)
10. spectrum                 (existing, MODIFIED)
11. ftth                     (existing)
12. manufacturing            *** NEW
13. launch                   *** NEW
14. terminal_distribution    *** NEW
15. satellite_revenue        *** NEW
16. revenue                  (existing)
17. cost                     (existing, MODIFIED)
18. finance                  (existing)
19. contract                 (existing)
20. ai                       (existing, MODIFIED)
21. weather                  (existing)
22. disaster                 (existing)
23. debris                   *** NEW
24. servicing                *** NEW
25. regulation               (existing)
26. research                 (existing)
27. patent                   (existing)
28. market                   (existing)
29. auction                  (existing)
30. covert_ops               (existing, MODIFIED)
31. lobbying                 (existing)
32. alliance                 (existing)
33. legal                    (existing)
34. grants                   (existing)
35. achievement              (existing, MODIFIED)
36. stock_market             (existing)
    resolve_spectrum_auctions()
```

---

## Critical Files

| File | Changes |
|---|---|
| `crates/gt-common/src/types.rs` | ~8 NodeType variants, ~5 EdgeType variants, OrbitType/SatelliteStatus/RocketType/FactoryTier enums, satellite FrequencyBand variants, AIArchetype::SatellitePioneer |
| `crates/gt-common/src/commands.rs` | ~10 new Command variants |
| `crates/gt-common/src/events.rs` | ~15 new GameEvent variants |
| `crates/gt-common/src/protocol.rs` | ~2 new DeltaOp variants |
| `crates/gt-simulation/src/world.rs` | GameWorld storage fields, ~10 cmd_* handlers |
| `crates/gt-simulation/src/systems/mod.rs` | 8 new system modules, updated tick order |
| `crates/gt-simulation/src/components/` | 8 new component modules |
| `crates/gt-simulation/src/systems/coverage.rs` | Satellite orbital coverage |
| `crates/gt-simulation/src/systems/demand.rs` | Satellite demand |
| `crates/gt-simulation/src/systems/ai/` | SatellitePioneer archetype |
| `crates/gt-simulation/src/components/tech_research.rs` | ~8 new research techs |
| `crates/gt-bridge/src/lib.rs` | ~7 new query methods, SatelliteArrays struct |
| `crates/gt-wasm/src/lib.rs` | Implement new bridge queries |
| `web/src/lib/game/map/MapRenderer.ts` | Orbital overlay layers |
| `web/src/lib/panels/` | 4 new panel files |

## Verification

1. **Rust tests**: `cargo test` — new unit tests for orbital position calculation, ISL formation, launch reliability, debris accumulation, terminal distribution, satellite revenue
2. **Build check**: `cargo build` + `wasm-pack build crates/gt-wasm --target web` — zero errors/warnings
3. **Frontend**: `cd web && bun run dev` — verify orbital overlay renders, satellite panels load
4. **Integration**: Start game, build constellation, launch satellites, verify they appear on map, move each tick, form ISL links, connect to ground station, route traffic, generate subscriber revenue
5. **Performance**: With 1000 LEO sats, verify tick stays < 50ms and map renders at 60fps
