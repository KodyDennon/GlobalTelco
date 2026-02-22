# GlobalTelco: Master Development Charter

**Purpose:** Establish a single, high-impact charter that governs all development, ensures full implementation, enforces unified engine usage, and provides repository and coding standards for the entire project. This document serves as the authoritative guideline for any AI or human developer leading full-stack development.

---

## 1. Repository Architecture
- **Monorepo Structure:** All code lives in a single repository with clearly separated crates and directories:
  - `crates/gt-common/` — Shared types, traits, serialization
  - `crates/gt-simulation/` — Core ECS simulation engine, tick orchestrator, all systems
  - `crates/gt-world/` — World generation, geography, terrain, real earth data
  - `crates/gt-economy/` — Corporations, finance, markets, contracts, research
  - `crates/gt-infrastructure/` — Network graph, nodes, edges, routing
  - `crates/gt-population/` — Demographics, migration, employment, demand
  - `crates/gt-ai/` — AI corporation controllers, archetypes, strategy
  - `crates/gt-wasm/` — WASM bindings (wasm-bindgen bridge to JS frontend)
  - `crates/gt-server/` — Multiplayer server binary (WebSocket, auth, persistence)
  - `web/` — Svelte frontend (UI, deck.gl map, D3.js charts)
  - `desktop/` — Tauri desktop app wrapper
  - `data/` — Open data sources (OSM, World Bank, UN)
  - `Docs/` — Design documents and specifications
- **Branch Strategy:**
  - `main` — Production-ready fully integrated builds
  - `dev` — Active development branch
  - Feature branches for individual modules, merged via PRs
- **Commit Standards:** All commits must be atomic, with descriptive messages and tests included.

---

## 2. Unified Engine Rules
- **Single Simulation Engine:** All simulation logic runs in Rust through one deterministic ECS engine. The same crate (`gt-simulation`) compiles to WASM for browser single-player and native binary for multiplayer servers.
- **Crate Interactions:** All crates communicate through defined public APIs; no direct cross-crate internal access.
- **Deterministic Simulation:** All AI, disasters, routing, and economic calculations must be deterministic and testable. Same inputs = same outputs. Critical for multiplayer sync.
- **Event Queue:** Centralized event system handles infrastructure updates, disasters, player actions, AI decisions.

---

## 3. ECS Tick Order (20 Systems)

All systems run in this deterministic order every tick. Order is critical for correctness and multiplayer sync.

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

**Planned additional systems:** alliance, legal, grants, fog_of_war, pricing, maintenance_scheduling

---

## 4. Coding Standards & Conventions
- **Implementation Required:** No stub or placeholder code — all features must be fully coded, integrated, and tested.
- **Language & Framework:** Rust for all simulation logic. TypeScript/Svelte for frontend UI. deck.gl for map rendering. D3.js for data visualization.
- **Rust Standards:** `cargo clippy` clean, `cargo fmt` formatted, all public APIs documented with `///` doc comments.
- **TypeScript Standards:** Strict mode, proper typing, no `any` types.
- **Testing:** Target ~120-150 tests total. Happy path + edge cases per system (`cargo test`); integration tests for cross-crate interactions; frontend tests via Bun. Coverage goal: every system has at least 2 happy-path tests and 2 edge-case tests.
- **Naming Conventions:** Rust: snake_case for functions/variables, PascalCase for types. TypeScript: camelCase for functions/variables, PascalCase for types/components.

---

## 5. Full-Stack Responsibilities
- **Simulation Engine (Rust):** Deterministic ECS, world generation, economy, AI, infrastructure, population modeling.
- **Frontend (Svelte/deck.gl/D3.js):** 2D political map rendering, interactive dashboards, management panels, data visualization.
- **WASM Bridge:** TypeScript ↔ Rust interop via wasm-bindgen. Commands (player actions) and Queries (state reads).
- **Multiplayer Server (Rust):** Authoritative simulation, WebSocket communication, world persistence, AI proxy.
- **AI Agents:** Deterministic decision-making for infrastructure, economics, alliances, and expansion. 4 archetype personalities.
- **Persistence:** Binary ECS serialization (serde + bincode + zstd). Browser saves in IndexedDB. Cloud saves in PostgreSQL.

---

## 6. Rules for Complete Integration
- **No Partial Systems:** Every feature must be integrated end-to-end before merging to main.
- **Cross-Crate Contracts:** Crates expose public APIs; integration tests ensure contracts are respected.
- **Version Control:** Every change must include updated documentation, unit tests, and integration tests.
- **Continuous Build:** `cargo build`, `cargo test`, `cargo clippy`, and `bun run build` must all succeed on main; broken builds are blocked.
- **Tauri Desktop Compatibility:** All frontend changes must maintain compatibility with the Tauri desktop app throughout development. Test `cargo tauri dev` periodically. No browser-only APIs without Tauri fallbacks.

---

## 7. Developer / AI Agent Obligations
- Follow this charter strictly; all development must adhere to repository architecture and engine rules.
- Ensure all crates and frontend components are fully implemented, integrated, and tested.
- Maintain deterministic simulation for consistency across WASM and native builds.
- Document and enforce API contracts for crate interactions.
- Continuously validate persistence, multiplayer consistency, and AI agent decision-making.

---

**Outcome:** This Master Development Charter is the binding rulebook for any AI or human leading full-stack development of the GlobalTelco project, ensuring complete implementation, deterministic simulation, full integration, and consistent standards across all systems.
