# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**GlobalTelco** — a 2D infrastructure empire builder. Mix of city builder, tycoon/business sim, and grand strategy. Players build and operate telecom infrastructure on a political map (Victoria 3 / Risk style), growing from a local ISP to a global telecom empire. Web-based with offline single-player and async persistent multiplayer.

## Technology Stack

- **Simulation engine:** Rust (ECS architecture) — compiles to WASM for browser + native binary for servers
- **Frontend:** Svelte (UI framework) + Three.js (2D map rendering) + D3.js (charts/data viz)
- **Build/runtime:** Bun (bundler, package manager, test runner) + wasm-pack (Rust → WASM)
- **Desktop app:** Tauri (Rust-based wrapper, uses system webview)
- **Multiplayer servers:** Rust native binary (same sim code as WASM, compiled natively)
- **API/WebSocket:** Rust (Axum) or Bun for lightweight services
- **Database:** PostgreSQL (world state, accounts, cloud saves)
- **Hosting:** Hetzner (game servers), Cloudflare Workers (auth, APIs), Vercel (frontend CDN)
- **Real-time:** WebSocket for multiplayer state sync
- **Deterministic:** Same inputs = same outputs. Critical for multiplayer sync.

## Project Structure

```
globaltelco/
├── crates/                    # Rust workspace
│   ├── gt-common/             # Shared types, traits, serialization
│   ├── gt-simulation/         # Core ECS engine, tick orchestrator
│   ├── gt-world/              # World generation, geography, terrain
│   ├── gt-economy/            # Corporations, finance, markets, contracts, research
│   ├── gt-infrastructure/     # Network graph, nodes, edges, routing
│   ├── gt-population/         # Demographics, migration, employment, demand
│   ├── gt-ai/                 # AI corporation controllers, archetypes, strategy
│   ├── gt-wasm/               # WASM bindings (wasm-bindgen bridge to JS)
│   └── gt-server/             # Multiplayer server binary (WebSocket, auth, persistence)
├── web/                       # Svelte frontend
│   ├── src/
│   │   ├── lib/
│   │   │   ├── wasm/          # TypeScript WASM bridge (commands + queries)
│   │   │   ├── game/          # Game screen (map renderer, HUD, speed controls, advisor)
│   │   │   ├── panels/        # Management panels (dashboard, infra, workforce, research, contracts)
│   │   │   ├── menu/          # Main menu, new game, load game, world browser, settings
│   │   │   ├── charts/        # D3.js visualizations (finance, population, network, market share)
│   │   │   └── ui/            # Reusable components (button, panel, table, tooltip, modal, slider)
│   │   └── stores/            # Svelte stores (game state, UI state, settings)
│   ├── static/                # Icons, fonts, map data (GeoJSON)
│   └── package.json
├── desktop/                   # Tauri desktop app wrapper
├── data/                      # Open data sources (OSM, World Bank, UN)
├── Docs/                      # Design specification documents
└── Cargo.toml                 # Rust workspace root
```

## Build Commands

```bash
# --- Rust simulation engine ---

# Build all crates (debug)
cargo build

# Build all crates (release)
cargo build --release

# Run tests
cargo test

# Build WASM module for browser
wasm-pack build crates/gt-wasm --target web --out-dir ../../web/src/lib/wasm/pkg

# Run multiplayer server
cargo run --bin gt-server

# --- Frontend ---

# Install dependencies
cd web && bun install

# Dev server (hot reload)
cd web && bun run dev

# Production build
cd web && bun run build

# Run frontend tests
cd web && bun test

# --- Desktop app ---

# Dev mode
cd desktop && cargo tauri dev

# Production build
cd desktop && cargo tauri build

# --- Full build pipeline ---

# Build WASM, then build frontend
wasm-pack build crates/gt-wasm --target web --out-dir ../../web/src/lib/wasm/pkg && cd web && bun run build
```

## Crate Dependency Graph

```
gt-server → gt-simulation, gt-common
gt-wasm → gt-simulation, gt-common
gt-simulation → gt-world, gt-economy, gt-infrastructure, gt-population, gt-ai, gt-common
gt-world → gt-common
gt-economy → gt-common
gt-infrastructure → gt-common
gt-population → gt-common
gt-ai → gt-common, gt-economy, gt-infrastructure
```

## ECS Architecture

All game state is managed through an Entity Component System. Entities are IDs, components are data, systems process entities with matching component sets.

**Core entity types:** InfrastructureNode, InfrastructureEdge, Corporation, Subsidiary, Employee/Team, Region, City, Contract, LandParcel, TechResearch, DebtInstrument

**Systems run in deterministic order each tick:**
1. construction → 2. maintenance → 3. population → 4. demand → 5. routing → 6. utilization → 7. revenue → 8. cost → 9. finance → 10. contract → 11. ai → 12. disaster → 13. regulation → 14. research → 15. market

## Key Architecture Concepts

**Political Map (2D):** Multi-layer zoom: World → Country → Region → City. Three.js in orthographic 2D mode. Layers: ocean, land, borders, cities, infrastructure, ownership overlay, selection, labels.

**Hex-based Land Parcels:** Terrain classification, zoning, ownership, regulatory strictness, disaster risk, cost modifiers. Terrain types (urban, suburban, rural, mountainous, desert, coastal, ocean shallow/deep, tundra, frozen) apply multipliers.

**Hierarchical Network Graph (5 levels):** Local → Regional → National → Continental → Global Backbone. Event-driven dirty-node invalidation, cluster-based routing, cached shortest-path trees. Aggregate bandwidth — no packet-level sim.

**Modular Industry Abstraction:** Sim core is industry-agnostic (Node, Edge, Resource, Dependency, Throughput, Risk, Ownership, Jurisdiction). Telecom is first module; energy, water, transport are future modules.

**Cooperative Ownership:** Infrastructure assets can be jointly owned with shared revenue and upgrade voting.

**Tiered Management:** Small company = hands-on. Medium = teams and budgets. Large = policies, departments, regional managers, AI execution. Dwarf Fortress style scaling.

**Era Progression:** Telegraph (~1850s) → Telephone (~1900s) → Early Digital (~1970s) → Internet (~1990s) → Modern (~2010s) → Near Future (~2030s). Player picks starting era.

## WASM Bridge Pattern

```
Svelte Component → bridge.ts (TypeScript) → gt-wasm (wasm-bindgen) → ECS World

Commands (player → sim): build_node, build_edge, hire_employee, set_policy, take_loan, propose_contract, set_research, set_speed, toggle_pause, save_game, load_game

Queries (sim → UI): get_visible_entities, get_corporation_data, get_region_data, get_infrastructure_list, get_workforce, get_contracts, get_research_state, get_notifications, get_advisor_suggestion
```

## AI Corporation System

- 4 archetypes: Aggressive Expander, Defensive Consolidator, Tech Innovator, Budget Operator
- 4 strategy modes selected dynamically: Expand, Consolidate, Compete, Survive
- AI actions: land acquisition, node/edge building, finance management, contract proposals
- Dynamic market: AI companies spawn, grow, merge, go bankrupt naturally

## Multiplayer Architecture

- **Single-player:** WASM module IS the server. Full sim runs in browser. No network needed.
- **Multiplayer:** Server-authoritative. Clients send commands via WebSocket, server validates and broadcasts state deltas.
- **AI proxy:** When player disconnects, AI manages their corp using saved policies. No strategic changes — maintenance only.
- **Persistent worlds:** 24/7 servers, multiple worlds with different settings/eras.
- **Protocol:** MessagePack (binary) or JSON (debug). Commands, ticks, acks, events, snapshots.

## Save System

- **Single-player:** Binary ECS world serialization + zstd compression. Local save slots.
- **Cloud saves:** Same format, stored as blob in PostgreSQL.
- **Auto-save:** Periodic (every N ticks).
- **Multiple save slots** for single-player.

## Visual Design

- **Map:** Satellite-inspired dark base, terrain-colored land, political borders overlaid. Night-earth vibes with city lights.
- **UI panels:** Solid dark panels, Bloomberg Terminal aesthetic. Navy/charcoal base, green=profit, red=loss, blue=neutral, amber=warning.
- **Typography:** Clean sans-serif. Monospace for financial numbers.
- **Player branding:** Company color + logo appears on map.
- **Infrastructure icons:** Realistic miniatures, readable at all zoom levels.

## Design Documents

All located in `Docs/`:

| File | Purpose |
|------|---------|
| `game_design_decisions.md` | **Definitive reference** — all gameplay, technical, and design decisions (19 sections) |
| `technical_architecture.md` | Full technical architecture — ECS, crate structure, frontend, WASM bridge, multiplayer, data schema |
| `global_telecom_infrastructure_mmo_project_design_document.md` | Original master design doc (world structure, infrastructure, economics, competition, disasters) |
| `telecom_mmo_master_dev_charter.md` | Development rules — coding standards, integration requirements |
| `telecom_mmo_infrastructure_simulation.md` | Infrastructure module spec |
| `telecom_mmo_economic_corporate_simulation.md` | Economy module spec |
| `telecom_mmo_multiplayer_governance_simulation.md` | Multiplayer module spec |
| `telecom_mmo_ai_dev_docs.md` | AI development specs |
| `offline_singleplayer_implementation_plan.md` | Single-player implementation plan |
| `mvp_to_production_v1_plan.md` | Development phase plan |

## Development Rules (Mandatory)

- **No stubs or placeholders** — all features must be fully coded, integrated, and tested
- **Single deterministic simulation engine** — all calculations deterministic and testable
- **Crate isolation via APIs** — crates communicate through defined public APIs only; no direct cross-crate internal access
- **Centralized event queue** — all state mutations flow through the event system
- **End-to-end integration before merge** — no partial systems merged to main
- **Branch strategy:** `main` (production-ready), `dev` (active development), feature branches merged via PRs
- **Atomic commits** with descriptive messages and tests included

## Hosting Architecture

```
Players ──► Cloudflare Workers (auth, matchmaking, APIs, CDN)
                │
                ▼
           Hetzner (Rust game server binary × 1-5 instances)
                │
                ▼
           PostgreSQL (world state, accounts, cloud saves)
```

- **Dev:** Hetzner Cloud CX22 (2 vCPU, 4GB RAM, ~€3.49/month)
- **Prod:** Hetzner Dedicated AX42 (Ryzen 7 7700, 64GB RAM, ~€57/month, runs 3-5 sim instances)
- **Service layer:** Cloudflare Workers (free tier: 100k req/day, paid $5/month: 10M req)
- **Frontend CDN:** Vercel (Svelte app) + Cloudflare (static assets)
- **No AWS, no Azure, no Oracle, no Unreal Engine.**

## Performance Targets

- Simulation tick: < 50ms for 10,000+ entities
- Map rendering: 60fps at all zoom levels
- WASM module: < 5MB gzipped
- Initial page load: < 3 seconds
- WebSocket latency: < 100ms round-trip
- Save file: < 50MB for mature world
- Memory: < 500MB in browser
