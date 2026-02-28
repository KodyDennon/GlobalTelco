# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**GlobalTelco** — a 2D infrastructure empire builder. Mix of city builder, tycoon/business sim, and grand strategy. Players build and operate telecom infrastructure on a political map (Victoria 3 / Risk style), growing from a local ISP to a global telecom empire. Web-based with offline single-player and async persistent multiplayer.

## Technology Stack

- **Simulation engine:** Rust (ECS architecture) — compiles to WASM for browser + native binary for servers
- **Frontend:** Svelte (UI framework) + deck.gl (2D map rendering) + D3.js (charts/data viz)
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
│   ├── gt-common/             # Shared types, traits, serialization, protocol
│   ├── gt-simulation/         # Core ECS engine, tick orchestrator
│   ├── gt-world/              # World generation, geography, terrain
│   ├── gt-economy/            # Corporations, finance, markets, contracts, research
│   ├── gt-infrastructure/     # Network graph, nodes, edges, routing
│   ├── gt-population/         # Demographics, migration, employment, demand
│   ├── gt-ai/                 # AI corporation controllers, archetypes, strategy
│   ├── gt-bridge/             # Shared BridgeQuery trait + typed array structs (WASM & Tauri)
│   ├── gt-wasm/               # WASM bindings (wasm-bindgen bridge to JS + typed arrays)
│   ├── gt-tauri/              # Tauri native bridge (BridgeQuery impl for desktop)
│   └── gt-server/             # Multiplayer server binary (WebSocket, auth, persistence)
├── web/                       # Svelte frontend
│   ├── src/
│   │   ├── lib/
│   │   │   ├── wasm/          # TypeScript WASM bridge (commands + queries + typed arrays)
│   │   │   ├── game/          # Game screen (map renderer, HUD, speed controls, advisor)
│   │   │   ├── multiplayer/   # WebSocket client, multiplayer state management
│   │   │   ├── panels/        # Management panels (dashboard, infra, workforce, research, contracts)
│   │   │   ├── menu/          # Main menu, new game, load game, world browser, settings
│   │   │   ├── charts/        # D3.js visualizations (finance, population, network, market share)
│   │   │   └── ui/            # Reusable components (button, panel, table, tooltip, modal, slider)
│   │   └── stores/            # Svelte stores (game state, UI state, settings)
│   ├── static/                # Icons, fonts, map data (GeoJSON)
│   └── package.json
├── desktop/                   # Tauri desktop app wrapper (separate Cargo project)
│   └── src-tauri/             # Rust backend with native sim commands + file system ops
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
gt-wasm → gt-simulation, gt-bridge, gt-common
gt-tauri → gt-simulation, gt-bridge, gt-common
gt-bridge → gt-common
gt-simulation → gt-world, gt-economy, gt-infrastructure, gt-population, gt-ai, gt-common
gt-world → gt-common
gt-economy → gt-common
gt-infrastructure → gt-common
gt-population → gt-common
gt-ai → gt-common, gt-economy, gt-infrastructure
desktop/src-tauri → gt-tauri, gt-bridge, gt-common (separate Cargo project, excluded from workspace)
```

## ECS Architecture

All game state is managed through an Entity Component System. Entities are IDs, components are data, systems process entities with matching component sets.

**Core entity types:** InfrastructureNode, InfrastructureEdge, Corporation, Subsidiary, Employee/Team, Region, City, Contract, LandParcel, TechResearch, DebtInstrument, Patent, LicenseAgreement, Alliance, Lawsuit, GovernmentGrant

**Systems run in deterministic order each tick (28 systems):**
1. construction → 2. maintenance → 3. population → 4. coverage → 5. demand → 6. routing → 7. utilization → 8. spectrum → 9. ftth → 10. revenue → 11. cost → 12. finance → 13. contract → 14. ai → 15. weather → 16. disaster → 17. regulation → 18. research → 19. patent → 20. market → 21. auction → 22. covert_ops → 23. lobbying → 24. alliance → 25. legal → 26. grants → 27. achievement → 28. stock_market

*After all 28 systems, `resolve_spectrum_auctions()` runs to finalize spectrum auction results and expire licenses.*

## Key Architecture Concepts

**Political Map (2D):** Multi-layer zoom: World → Country → Region → City. deck.gl in 2D mode. Layers: ocean, land, borders, cities, infrastructure, ownership overlay, selection, labels.

**Free Placement + Invisible Grid:** Infrastructure can be placed at any exact (lon, lat) coordinate on the map — no grid snapping. An invisible hex grid of cells remains as the backend spatial index for terrain lookup, coverage calculations, demand modeling, and AI strategic decisions. Terrain types (urban, suburban, rural, mountainous, desert, coastal, ocean shallow/deep, tundra, frozen) apply cost multipliers based on the nearest cell's terrain.

**Hierarchical Network Graph (5 levels):** Local → Regional → National → Continental → Global Backbone. Event-driven dirty-node invalidation, cluster-based routing, cached shortest-path trees. Aggregate bandwidth — no packet-level sim. Competitor infrastructure hidden by default — requires espionage intel to view.

**Modular Industry Abstraction:** Sim core is industry-agnostic (Node, Edge, Resource, Dependency, Throughput, Risk, Ownership, Jurisdiction). Telecom is first module; energy, water, transport are future modules. Research is organized by era but freely explorable — no era gates. Tech is the primary economic commodity (patent, license, lease).

**Cooperative Ownership:** Infrastructure assets can be jointly owned with shared revenue and upgrade voting.

**Tiered Management:** Small company = hands-on. Medium = teams and budgets. Large = policies, departments, regional managers, AI execution. Dwarf Fortress style scaling.

**Era Progression:** Telegraph (~1850s) → Telephone (~1900s) → Early Digital (~1970s) → Internet (~1990s) → Modern (~2010s) → Near Future (~2030s). Player picks starting era. Research tree organized by era but freely explorable — players can research higher-era tech with prerequisites. World era = cosmetic collective milestone (no gameplay effects).

## WASM Bridge Pattern

```
Single-player: Svelte Component → bridge.ts → gt-wasm (wasm-bindgen) → ECS World
Multiplayer:   Svelte Component → commandRouter.ts → WebSocketClient → Server → validates → broadcasts
Desktop:       Same as single-player (WASM in Tauri webview), Tauri IPC for native filesystem

Commands (player → sim, 61 total): build_node, build_edge, update_edge_waypoints, upgrade_node, decommission_node, repair_node, repair_edge, emergency_repair, hire_employee, fire_employee, assign_team, take_loan, repay_loan, set_budget, propose_contract, accept_contract, reject_contract, start_research, cancel_research, set_policy, create_subsidiary, purchase_insurance, cancel_insurance, declare_bankruptcy, request_bailout, accept_bailout, place_bid, propose_acquisition, respond_to_acquisition, launch_espionage, launch_sabotage, upgrade_security, start_lobbying, cancel_lobbying, propose_alliance, accept_alliance, dissolve_alliance, file_lawsuit, settle_lawsuit, defend_lawsuit, file_patent, request_license, set_license_price, revoke_license, start_independent_research, bid_for_grant, complete_grant, propose_co_ownership, respond_co_ownership, propose_buyout, vote_upgrade, bid_spectrum, assign_spectrum, unassign_spectrum, purchase_cable_ship, set_region_pricing, set_maintenance_priority, set_speed, toggle_pause, save_game, load_game

Queries (sim → UI):
  JSON queries: get_visible_entities, get_corporation_data, get_region_data, get_infrastructure_list, get_workforce, get_contracts, get_research_state, get_notifications, get_advisor_suggestion, get_patents, get_licenses, get_alliances, get_lawsuits, get_grants, get_intel_levels, get_stock_market, get_weather, get_spectrum_licenses, get_pricing, get_maintenance_priorities
  Typed array queries (hot-path): get_infra_nodes_typed, get_infra_edges_typed, get_corporations_typed

Multiplayer protocol: CommandAck (with seq, entity_id, tick), CommandBroadcast (DeltaOps), SpeedVoteUpdate, applyBatch (incremental WASM state update)
```

## AI Corporation System

- 4 archetypes: Aggressive Expander, Defensive Consolidator, Tech Innovator, Budget Operator
- 4 strategy modes selected dynamically: Expand, Consolidate, Compete, Survive
- AI actions: land acquisition, node/edge building, finance management, contract proposals
- Full autonomy — AI uses ALL game systems: patents, alliances, legal, pricing, maintenance, grants, insurance
- Dynamic market: AI companies spawn, grow, merge, go bankrupt naturally. Dynamic spawning (new AI corps when market underserved) and mergers.

## Multiplayer Architecture

- **Single-player:** WASM module IS the server. Full sim runs in browser. No network needed.
- **Multiplayer:** Server-authoritative. Clients send commands via WebSocket, server validates and broadcasts state deltas.
- **Sync model:** Event-driven delta broadcasts. CommandBroadcast carries DeltaOps (NodeCreated, EdgeCreated, etc.) for sub-200ms sync. Full snapshots every 30 ticks as safety net.
- **Optimistic UI:** Ghost entities (translucent) appear instantly for builder. Resolved on CommandAck. Other players see confirmed entities immediately via applyBatch.
- **Anti-cheat:** Per-type rate limiting (Build 3/sec, Financial 2/sec, Research 1/5sec, Espionage 1/30sec), server-side spatial validation, sequence number dedup.
- **Speed control:** World creator has override power. Others need majority vote (30s window).
- **Per-player event filtering:** Infrastructure events visible to all. Financial/internal events gated by espionage intel level.
- **Admin:** Ban/unban endpoints, real-time monitoring.
- **AI proxy:** When player disconnects, AI manages their corp using saved policies. No strategic changes — maintenance only.
- **Persistent worlds:** 24/7 servers, multiple worlds with different settings/eras.
- **Protocol:** MessagePack (binary) or JSON (debug). Commands (with seq), CommandAck (with entity_id + tick), CommandBroadcast (DeltaOps), SpeedVoteUpdate, TickUpdate, full snapshots.

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

**Currently deployed (dev):**
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

- **Dev (current):** Fly.io (game server) + Vercel (frontend CDN) + PostgreSQL
- **Prod (target):** Hetzner Dedicated AX42 (Ryzen 7 7700, 64GB RAM, ~€57/month, runs 3-5 sim instances)
- **Service layer (target):** Cloudflare Workers (free tier: 100k req/day, paid $5/month: 10M req)
- **Frontend CDN:** Vercel (Svelte app) + Cloudflare (static assets)
- **No AWS, no Azure, no Oracle, no Unreal Engine.**

## Tool Rules (Mandatory)

- **NEVER use `sed` or `awk` for file editing.** Use the Edit tool for all file modifications. No exceptions.
- **NEVER use `cat` with heredoc or `echo` redirection to write files.** Use the Write tool.
- **NEVER use `grep` or `rg` via Bash.** Use the Grep tool.
- Use Bash only for: git operations, build commands (cargo, bun, wasm-pack), and system commands that require shell execution.

## Performance Targets

- Simulation tick: < 50ms for 10,000+ entities
- Map rendering: 60fps at all zoom levels
- WASM module: < 5MB gzipped
- Initial page load: < 3 seconds
- WebSocket latency: < 100ms round-trip
- Save file: < 50MB for mature world
- Memory: < 500MB in browser

## Current Implementation Status

**Codebase counts (verified from source):**
- **28 ECS systems** (construction, maintenance, population, coverage, demand, routing, utilization, spectrum, ftth, revenue, cost, finance, contract, ai, weather, disaster, regulation, research, patent, market, auction, covert_ops, lobbying, alliance, legal, grants, achievement, stock_market)
- **38 component modules** (achievements, acquisition, ai_state, alliance, auction, building, capacity, city, construction, contract, corporation, covert_ops, debt_instrument, demand, financial, grant, health, infra_edge, infra_node, land_parcel, lawsuit, lobbying, maintenance_priority, ownership, patent, policy, population, position, pricing, region, road_graph, spectrum, stock_market, tech_research, victory, weather, workforce)
- **41 NodeType variants** across 6 eras (Telegraph through Near Future)
- **25 EdgeType variants** across 6 eras
- **61 commands** (infrastructure, workforce, finance, contracts, research, policy, subsidiary, insurance, bankruptcy/auctions, M&A, espionage/sabotage, lobbying, alliance, legal, patents/licensing, grants, co-ownership, spectrum, pricing, maintenance, game control)
- **66+ event types** with per-corporation relevance filtering for multiplayer
- **23 frontend panels** (Dashboard, Infra, Workforce, Contract, Research, Region, Auction, Merger, Achievement, Advisor, Intel, Spectrum, Insurance, Repair, Grant, Pricing, Maintenance, StockMarket, Alliance, Legal, Patent, CoOwnership, NetworkDashboard)
- **Audio system:** AudioManager + SpatialAudio (ambient music, UI sounds, environmental audio, disaster cues)

**Implemented systems (beyond original 20):**
- **Alliance system:** Trust scoring, revenue sharing, auto-dissolution below trust threshold, free routing between allies
- **Legal system:** Lawsuit filing/resolution, settlement, multiple lawsuit types (sabotage claim, ownership dispute, patent infringement, regulatory complaint)
- **Patent system:** Filing, licensing (Permanent/Royalty/PerUnit/Lease), royalty collection, independent research (150%/200% cost), patent enforcement
- **Grants system:** Government grants per region, bidding, progress tracking, payouts
- **Weather system:** Regional weather patterns (storms, ice storms, flooding, extreme heat, earthquakes, hurricanes), terrain-weighted generation, 15-40 tick duration, disaster severity amplification
- **Stock market system:** Auto-IPO at 50+ nodes, share price calculation, dividends, shareholder satisfaction, board votes
- **Spectrum system:** Carrier aggregation, interference penalties, spectrum auctions, license management, per-node band assignment
- **FTTH system:** Central Office to NAP chain validation, active NAP marking, distribution fiber topology
- **Sandbox mode:** Infinite money, skip bankruptcy, configurable via WorldConfig
- **Regional pricing:** Per-region pricing tiers (Budget/Standard/Premium), price per unit
- **Maintenance priority:** Per-node priority tiers (Critical/Standard/Low/Deferred), auto-repair toggle

**Key remaining gaps:**
- No era enforcement (all tech freely buildable regardless of era)
- Fog of war partial (espionage/sabotage missions exist but no full intel level tiers with decay)
- Management scaling UI not implemented (small/medium/large company tier detection)
- Dynamic AI spawning mid-game not implemented (AI corps only created at game start)
- Submarine cable mechanics partial (landing stations exist but no cable ship construction mechanic)
- No colorblind mode or full accessibility features
- No localization beyond English (i18n framework exists)
