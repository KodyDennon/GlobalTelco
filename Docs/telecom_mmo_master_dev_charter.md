# Telecom MMO: Master Development Charter

**Purpose:** Establish a single, high-impact charter that governs all development, ensures full implementation, enforces unified engine usage, and provides repository and coding standards for the entire project. This document serves as the authoritative guideline for any AI or human developer leading full-stack development.

---

## 1. Repository Architecture
- **Monorepo Structure:** All code lives in a single repository with clearly separated modules:
  - `/engine` — Core unified game engine, simulation logic, event handling
  - `/infrastructure` — Nodes, edges, routing, disaster simulation
  - `/economy` — Corporate and regional simulation logic, AI company behavior
  - `/multiplayer` — Server authoritative logic, player management, alliances, conflicts
  - `/frontend` — UI/UX, 2.5D and global 3D views, dashboards, player interaction
  - `/tools` — Build scripts, deployment scripts, automated testing
  - `/docs` — Design docs, API references, coding conventions
- **Branch Strategy:**
  - `main` — Production-ready fully integrated builds
  - `dev` — Active development branch
  - Feature branches for individual modules, merged via PRs
- **Commit Standards:** All commits must be atomic, with descriptive messages and tests included.

---

## 2. Unified Engine Rules
- **Single Simulation Engine:** All simulation logic runs on one deterministic engine to ensure consistency across modules.
- **Module Interactions:** All modules (Infrastructure, Economy, Multiplayer) communicate through defined APIs; no direct cross-module manipulation.
- **Deterministic Simulation:** All AI, disasters, routing, and economic calculations must be deterministic and testable.
- **Event Queue:** Centralized event system handles network updates, disasters, player actions, AI decisions.

---

## 3. Coding Standards & Conventions
- **Implementation Required:** No stub or placeholder code — all features must be fully coded, integrated, and tested.
- **Language & Framework:** Unreal Engine preferred, with Blueprints for UI, C++ for core engine logic.
- **Documentation:** All functions, classes, and modules must include full documentation and usage examples.
- **Testing:** Unit tests for every module; integration tests for cross-module interactions.
- **Naming Conventions:** Consistent, descriptive names; follow engine and company conventions.

---

## 4. Full-Stack Responsibilities
- **Backend:** Authoritative server, persistent world state, event handling, multi-player arbitration.
- **Frontend:** 2.5D/3D visualization, interactive dashboards, negotiation, arbitration interfaces.
- **AI Agents:** Deterministic decision-making for infrastructure, economics, alliances, and expansion.
- **Persistence:** Full serialization/deserialization, save/load across servers, tick-based updates.

---

## 5. Rules for Complete Integration
- **No Partial Systems:** Every feature must be integrated end-to-end before merging to main.
- **Cross-Module Contracts:** Modules expose APIs; integration tests ensure contracts are respected.
- **Version Control:** Every change must include updated documentation, unit tests, and integration tests.
- **Continuous Build:** Build must succeed on main; broken builds are blocked.

---

## 6. Developer / AI Agent Obligations
- Follow this charter strictly; all development must adhere to repository architecture and engine rules.
- Ensure all modules are fully implemented, integrated, and tested.
- Maintain deterministic simulation for consistency.
- Document and enforce API contracts for module interactions.
- Continuously validate persistence, multi-player consistency, and AI agent decision-making.

---

**Outcome:** This Master Development Charter is the binding rulebook for any AI or human leading full-stack development of the Telecom MMO project, ensuring complete implementation, deterministic simulation, full integration, and consistent standards across all systems.

