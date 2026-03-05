# GlobalTelco: Systems & Features Audit

This document provides a comprehensive breakdown of the GlobalTelco codebase, categorized by architectural layers, systems, and features.

---

## 1. Core Shared Layer (`crates/gt-common`)
**Role:** The "Source of Truth" for all data structures and communication protocols used by both the Rust simulation and the Svelte frontend.

| System / Feature | Key Responsibilities | Dependencies | Status | Integration |
| :--- | :--- | :--- | :--- | :--- |
| **Command System** | Enum-based command definitions for all player actions (Build, Trade, Vote, etc.). | `serde` | **COMPLETE** | Fully integrated in `GameWorld::process_command`. 44+ commands verified. |
| **Event System** | Real-time event log for UI feedback and deterministic state sync. | `serde` | **COMPLETE** | Events like `ConstructionCompleted`, `RevenueEarned`, `GlobalNotification` pushed to queue. |
| **Protocol Layer** | Message structures for WebSocket (Multiplayer) and WASM (Singleplayer) communication. | `msgpack`, `serde_json` | **COMPLETE** | `ClientMessage` and `ServerMessage` enums handle all network traffic. |
| **Types Module** | Domain types: `Money`, `Tick`, `EntityId`, `NodeType`, `EdgeType`, `TerrainType`. | None | **COMPLETE** | All 60+ node/edge types defined with era, cost, and capacity attributes. |

---

## 2. Simulation Engine (`crates/gt-simulation`, `gt-world`, `gt-infrastructure`)
**Role:** High-performance ECS (Entity Component System) that handles the game logic, world generation, and infrastructure simulation.

### 2.1. Infrastructure & Routing
| System / Feature | Key Responsibilities | Requirements | Status | Integration |
| :--- | :--- | :--- | :--- | :--- |
| **Network Graph** | Dijkstra-based routing for traffic across fiber, copper, and wireless links. | `gt-infrastructure` | **COMPLETE** | Used by `utilization.rs` every tick for traffic flow. |
| **Road Network** | A* pathfinding along world-generated roads for realistic cable routing. | `gt-infrastructure` | **COMPLETE** | Logic moved to `gt-infrastructure`. `road_fiber_route_cost` exposed to frontend. |
| **Traffic Utilization** | Simulates GB/s flows, bottlenecks, and dropped packets based on demand. | `NetworkGraph` | **COMPLETE** | `push_traffic_on_path` now tracks hop counts for revenue attribution. |
| **FTTH/DropCables** | Last-mile connections from NAPs to individual building entities. | `BuildingFootprints` | **COMPLETE** | `calculate_building_revenue` handles direct connections vs. auto-coverage. |
| **Subsea Cables** | Global backbone construction requiring **Cable Ships** as a resource. | `cable_ships` state | **COMPLETE** | `cmd_purchase_cable_ship` and construction blocking logic implemented. |

### 2.2. Economy & Finance
| System / Feature | Key Responsibilities | Requirements | Status | Integration |
| :--- | :--- | :--- | :--- | :--- |
| **Revenue System** | Multi-party revenue sharing (Co-owners, Alliances, Transit Settlements). | `TrafficMatrix` | **COMPLETE** | Overhauled to support proportional splitting based on ownership share. |
| **Cost System** | Maintenance, workforce salaries, and insurance premiums (scaled by risk). | `HealthComponent` | **COMPLETE** | Maintenance costs now split proportionally among co-owners. |
| **Stock Market** | IPOs, share trading, dividends, and shareholder satisfaction. | `Corporation` | **COMPLETE** | Auto-IPO, Dividends, Buy/Sell commands, and Price Momentum logic fully active. |
| **Spectrum Market** | Regional frequency auctions and band assignments for wireless nodes. | `gt-world` Politics | **COMPLETE** | `cmd_bid_spectrum` and license management fully functional. |
| **Grants** | Government contracts for underserved regions with tax breaks. | `gt-world` | **COMPLETE** | Deterministic generation based on satisfaction < 30%. |

### 2.3. World & Environment
| System / Feature | Key Responsibilities | Requirements | Status | Integration |
| :--- | :--- | :--- | :--- | :--- |
| **Procgen World** | Voronoi-based region/city generation with biomes, rivers, and elevation. | `gt-world` | **COMPLETE** | `generate_world` creates a consistent deterministic map. |
| **Real Earth Mode** | Loads real-world data (countries, cities) from ESRI/OSM datasets. | `earth.json` | **COMPLETE** | `real_earth.rs` parser fully integrated with map renderer. |
| **Weather/Disaster** | Dynamic storms, earthquakes, and fiber cuts affecting infrastructure health. | `TerrainType` | **COMPLETE** | Disasters trigger `InfrastructureDamaged` events and impact insurance premiums. |
| **Population Sim** | City growth, demand scaling, and migration based on infra availability. | `gt-population` | **COMPLETE** | `population.rs` system updates city stats every 100 ticks. |

### 2.4. Satellite System
| System / Feature | Key Responsibilities | Requirements | Status | Integration |
| :--- | :--- | :--- | :--- | :--- |
| **Orbital Sim** | LEO/MEO/GEO orbit propagation, station keeping, and fuel consumption. | `PositionComponent` | **COMPLETE** | `orbital.rs` updates satellite positions every tick. |
| **Satellite Network** | Dynamic ISL (Inter-Satellite Link) and downlink edges rebuilt each tick. | `NetworkGraph` | **COMPLETE** | Memory leak fixed. Dynamic edges correctly bridge ground and space segments. |
| **Debris System** | Kessler syndrome simulation; debris collision risks in specific orbital shells. | `gt-simulation` | **COMPLETE** | `debris.rs` calculates collision probabilities and triggers cascade events. |

### 2.5. AI & Diplomacy
| System / Feature | Key Responsibilities | Requirements | Status | Integration |
| :--- | :--- | :--- | :--- | :--- |
| **AI Strategy** | Archetypes (Aggressive, Defensive, Tech, Budget) drive behavior. | `AiState` | **COMPLETE** | `select_strategy` switches modes dynamically based on cash/debt. |
| **Diplomacy** | Auctions, Mergers, Espionage, and Lobbying. | `GameWorld` | **COMPLETE** | Deterministic bidding and merger logic fully functional. |
| **Contracts** | Peering and Transit negotiation based on network utility. | `NetworkGraph` | **COMPLETE** | AI evaluates incoming deals based on price vs. revenue. |

---

## 3. Runtime Bridges (`crates/gt-wasm`, `gt-tauri`, `gt-server`)
**Role:** Exposes the simulation logic to different platforms (Web, Desktop, Server).

| System / Feature | Key Responsibilities | Platform | Status | Integration |
| :--- | :--- | :--- | :--- | :--- |
| **WASM Bridge** | High-speed bindings for browser-based single-player simulation. | Browser | **COMPLETE** | `queries.rs` exposes 40+ methods including `get_all_infrastructure`. |
| **Tauri Bridge** | IPC bindings for native Desktop performance and filesystem access. | Windows/Mac/Linux | **COMPLETE** | `gt-tauri` crate mirrors WASM API for native execution. |
| **Game Server** | Authoritative Axum/Tokio server for MMO multiplayer persistence. | Linux (Cloud) | **COMPLETE** | WebSocket handler manages rooms, ticks, and state broadcasting. |
| **Typed Arrays** | Zero-copy memory sharing for high-performance Deck.gl map rendering. | WASM | **COMPLETE** | `get_infra_nodes_typed` enables rendering millions of entities. |

---

## 4. Frontend & Visualization (`web/src`)
**Role:** The 2D/2.5D interface where players interact with the empire builder.

| System / Feature | Key Responsibilities | Requirements | Status | Integration |
| :--- | :--- | :--- | :--- | :--- |
| **Map Renderer** | Deck.gl-based visualization of millions of entities (nodes, edges, buildings). | `gpu`, `wasm` | **COMPLETE** | Layers for Infra, Buildings, Terrain, Overlay, Weather fully implemented. |
| **Game Loop** | Double-buffered state updates, worker-offloaded simulation, and interpolation. | `WebWorker` | **COMPLETE** | `GameLoop.ts` handles tick synchronization and `map-dirty` signals. |
| **Overlays** | Heatmaps for Traffic, Congestion, Demand, Disaster Risk, and Spectrum. | `latestTickResult` | **COMPLETE** | All overlays now support Worker mode via `simWorker.ts` data sync. |
| **UI Components** | Svelte 5 panels for Finance, Research, Social, and Build Menus. | `gameState` stores | **COMPLETE** | `commandRouter.ts` connects UI actions to WASM/Server commands. |
| **Audio Manager** | Procedural era-specific music and spatial SFX for construction/disasters. | `WebAudio` | **COMPLETE** | Context-aware audio engine integrated in `GameLoop`. |

---

## 5. Deployment & Persistence
**Role:** Hosting, database management, and CI/CD.

| System / Feature | Key Responsibilities | Requirements | Status | Integration |
| :--- | :--- | :--- | :--- | :--- |
| **Oracle Postgres** | Managed production database for accounts and persistent world state. | OCI Private VCN | **ACTIVE** | Migration completed. Server connecting via private IP `10.0.2.121`. |
| **R2 Storage** | Object storage for large binary world snapshots and cloud saves. | Cloudflare R2 | **ACTIVE** | `r2.rs` client configured with proper bucket permissions. |
| **Event Pruning** | Automatic database maintenance to keep storage within 512MB limits. | `tick.rs` | **ACTIVE** | `prune_old_events` runs every snapshot interval. |
| **gt TUI Tool** | Go-based CLI for cross-compilation, deployment, and status monitoring. | Go, SSH | **COMPLETE** | Deploy pipeline verifies environment variables and restarts services. |

---

## Current Integration State: **PRODUCTION READY**
The system is now fully end-to-end integrated. The simulation runs deterministically in a Web Worker, pushes state to the main thread via a synchronized bridge, and the map renderer uses this cached data to provide zero-latency visual feedback for all player actions. All critical "TODO" items from the initial audit have been resolved.
