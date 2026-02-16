# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Global Telecom Infrastructure MMO — a systemic infrastructure civilization simulator (not a tycoon game). Players build, operate, compete, and expand telecom infrastructure in a persistent, authoritative 3D world with interlocking systems: engineering, corporate finance, geopolitics, macroeconomics, disaster modeling, and market competition.

## Technology Stack

- **Engine:** Unreal Engine 5.7 (installed at `/Users/Shared/Epic Games/UE_5.7/`)
- **Language:** C++ for all simulation logic, Blueprints for UI layout
- **Server:** Dedicated authoritative simulation server (thin client architecture)
- **Concurrency:** 250 concurrent players per world
- **Simulation tick:** 3–5 second economic ticks; routing recalculation is event-driven only

## Build Commands

```bash
# Generate Xcode project files from .uproject
"/Users/Shared/Epic Games/UE_5.7/Engine/Build/BatchFiles/Mac/GenerateProjectFiles.sh" \
  /Users/kody/NuGit/GlobalTelco/GlobalTelco.uproject -game

# Build editor target (for development)
"/Users/Shared/Epic Games/UE_5.7/Engine/Build/BatchFiles/RunUBT.sh" \
  GlobalTelcoEditor Mac Development \
  -project="/Users/kody/NuGit/GlobalTelco/GlobalTelco.uproject"

# Build game target
"/Users/Shared/Epic Games/UE_5.7/Engine/Build/BatchFiles/RunUBT.sh" \
  GlobalTelco Mac Development \
  -project="/Users/kody/NuGit/GlobalTelco/GlobalTelco.uproject"

# Build dedicated server target
"/Users/Shared/Epic Games/UE_5.7/Engine/Build/BatchFiles/RunUBT.sh" \
  GlobalTelcoServer Mac Development \
  -project="/Users/kody/NuGit/GlobalTelco/GlobalTelco.uproject"

# Open in Unreal Editor
"/Users/Shared/Epic Games/UE_5.7/Engine/Binaries/Mac/UnrealEditor.app/Contents/MacOS/UnrealEditor" \
  /Users/kody/NuGit/GlobalTelco/GlobalTelco.uproject
```

## Module Architecture

Six UE5 C++ modules with a strict dependency graph (no circular dependencies):

```
GlobalTelco.uproject
├── Source/GlobalTelco/          — Main game module (GameMode, GameState, PlayerController)
├── Source/GTCore/               — Deterministic simulation engine, event queue, tick system
├── Source/GTInfrastructure/     — Network graph, nodes, edges, routing, disasters
├── Source/GTEconomy/            — Corporations, regional economics, market dynamics
├── Source/GTMultiplayer/        — Alliances, contracts, land parcels, governance
├── Source/GTFrontend/           — UMG widgets, HUD, corporate dashboard
├── Config/                      — DefaultEngine.ini, DefaultGame.ini, etc.
├── Content/                     — UE assets (meshes, textures, blueprints)
└── Docs/                        — Design specification documents
```

**Dependency graph:**
```
GlobalTelco → GTCore, GTInfrastructure, GTEconomy, GTMultiplayer, GTFrontend, AIModule, GameplayTasks
GTInfrastructure → GTCore
GTEconomy → GTCore
GTMultiplayer → GTCore, OnlineSubsystem
GTFrontend → GTCore, GTEconomy, GTMultiplayer, GTInfrastructure, UMG, Slate
```

**Build targets:**
- `GlobalTelco.Target.cs` — Game client
- `GlobalTelcoEditor.Target.cs` — Editor (development)
- `GlobalTelcoServer.Target.cs` — Dedicated server (no GTFrontend)

## Key Classes by Module

**GTCore** — Simulation engine foundation
- `UGTSimulationSubsystem` — World subsystem driving the economic tick cycle; supports pause/resume and speed multiplier (1x, 2x, 4x)
- `UGTEventQueue` — Centralized, thread-safe event queue for all state mutations
- `FGTSimulationEvent` — Atomic event struct (type, tick, source/target entity, payload)
- `EGTSimulationEventType` — Event categories (infrastructure, economic, disaster, player, political)
- `UGTAIArchetypeRegistry` — Static registry of 4 built-in AI personality archetypes (Aggressive Expander, Defensive Consolidator, Tech Innovator, Budget Operator)
- `FGTAIArchetypeData` — Data struct defining AI personality weights (expansion, consolidation, tech, aggression, risk, prudence)

**GTInfrastructure** — Network topology and routing
- `UGTNetworkGraph` — World subsystem: 5-level hierarchical graph with Dijkstra routing
- `AGTNetworkNode` — Replicated actor base for infrastructure nodes (towers, data centers, IXPs)
- `UGTNetworkEdge` — Edge UObject connecting two nodes (fiber, microwave, subsea, satellite)
- Dirty-node invalidation triggers cluster-based route recalculation

**GTEconomy** — Corporate finance and markets
- `UGTCorporation` — Corporation UObject: balance sheet, income, debt, credit rating, shareholders; `bIsAI` + `ArchetypeIndex` fields for AI identification
- `UGTCorporationManager` — World subsystem managing corporation lifecycle: create, destroy, lookup, tick processing for all corps
- `UGTRegionalEconomy` — World subsystem: per-region population, GDP, demand, connectivity effects

**GTMultiplayer** — Social and governance systems
- `UGTAllianceManager` — World subsystem: alliances and contracts between corporations
- `UGTLandParcelSystem` — World subsystem: hex-based parcel grid, ownership, zoning, leasing
- `FGTContract` — Contract struct with pricing, capacity guarantees, breach penalties

**GTFrontend** — UI layer
- `UGTHUDWidget` — Base HUD widget (abstract, Blueprint-extensible)
- `UGTDashboardWidget` — Corporate dashboard (abstract, Blueprint-extensible)
- `UGTMainMenuWidget` — Main menu: New Game, Load Game, Quit (abstract, Blueprint-extensible)
- `UGTNewGameWidget` — New game configuration: corp name, difficulty, AI count, disasters, world seed
- `UGTSpeedControlWidget` — In-game simulation speed controls: Pause, 1x, 2x, 4x + quick save/load

**GlobalTelco** — Game framework
- `AGTGameMode` — Server-authoritative game mode for multiplayer, enforces 250-player cap
- `AGTSinglePlayerGameMode` — Offline single-player game mode: world gen, AI corps, save/load, auto-save
- `AGTGameState` — Replicated simulation tick and world time
- `AGTPlayerController` — Per-player controller with corporation ID
- `UGTGameInstance` — Game instance bridging menu to gameplay: holds pending world settings, player config, save slot
- `AGTAICorporationController` — AI controller with programmatic behavior tree (no pawn): land acquisition, node/edge building, finance, contracts
- `UGTSaveGame` — USaveGame subclass: complete world snapshot (parcels, corporations, economy, contracts)
- `UGTSaveLoadSubsystem` — GameInstance subsystem: save, load, delete, enumerate save slots, auto-save

## Design Documents

All located in `Docs/`:

| File | Purpose |
|------|---------|
| `global_telecom_infrastructure_mmo_project_design_document.md` | Master design — world structure, infrastructure model, network graph, economics, competition, disasters |
| `telecom_mmo_master_dev_charter.md` | Binding development rules — repo structure, coding standards, integration requirements |
| `telecom_mmo_ai_dev_docs.md` | AI-agentic development specs overview |
| `telecom_mmo_infrastructure_simulation.md` | Infrastructure module spec |
| `telecom_mmo_economic_corporate_simulation.md` | Economy module spec |
| `telecom_mmo_multiplayer_governance_simulation.md` | Multiplayer module spec |
| `game_design_decisions.md` | Concrete implementation decisions — MVP scope, build UX, economy settings, tech tree, multiplayer, hosting |
| `offline_singleplayer_implementation_plan.md` | Offline single-player mode implementation plan — AI corps, save/load, speed controls, UI |

## Development Charter Rules (Mandatory)

- **No stubs or placeholders** — all features must be fully coded, integrated, and tested
- **Single deterministic simulation engine** — all calculations must be deterministic and testable
- **Module isolation via APIs** — modules communicate through defined APIs only; no direct cross-module manipulation
- **Centralized event queue** — all state mutations flow through `UGTEventQueue`
- **End-to-end integration before merge** — no partial systems merged to main
- **Branch strategy:** `main` (production-ready), `dev` (active development), feature branches merged via PRs
- **Atomic commits** with descriptive messages and tests included

## Core Architecture Concepts

**Hierarchical Network Graph (5 levels):** Local -> Regional -> National -> Continental -> Global Backbone. Uses event-driven dirty-node invalidation, cluster-based recomputation, and cached shortest-path trees. Aggregate bandwidth modeling only — no packet-level simulation.

**Hex-based Land Parcels:** Each parcel has terrain classification, zoning, ownership, regulatory strictness, disaster risk profile, and cost modifiers. Terrain types (urban, mountainous, desert, coastal, ocean) apply multipliers to cost, reliability, and maintenance.

**Modular Industry Abstraction:** The simulation core is industry-agnostic, built around: Node, Edge, Resource, Dependency, Throughput, Risk, Ownership, Jurisdiction. Telecom is the first module; energy, water, and transportation are planned future modules.

**Cooperative Ownership:** Infrastructure assets can be jointly owned by multiple players with shared revenue and upgrade voting.

## Offline Single-Player Architecture

The game supports a fully offline single-player mode with AI corporation competitors:

**Game Flow:** Main Menu → New Game Settings → `UGTGameInstance` stores config → Level opens → `AGTSinglePlayerGameMode::InitGame()` generates world, creates player corp, spawns AI corps → Gameplay with auto-save.

**AI Corporation System:**
- `AGTAICorporationController` extends `AAIController` without a possessed pawn (pure decision-making agent)
- Full UE5 Behavior Tree constructed programmatically in C++ (no .uasset files)
- 4 strategy modes: Expand, Consolidate, Compete, Survive — selected dynamically based on financial health and archetype personality
- BT tasks: land acquisition, node building, edge building, finance management, contract proposals
- Revenue generation from owned nodes proportional to regional demand; maintenance costs scale with infrastructure

**AI Archetypes (4 built-in):**
1. Aggressive Expander — rapid growth, high debt tolerance, competitive pricing
2. Defensive Consolidator — cautious, debt-averse, strong networks over large networks
3. Tech Innovator — high-capacity infrastructure, R&D focused, balanced growth
4. Budget Operator — minimal spending, no debt, value infrastructure only

**Save/Load System:**
- `UGTSaveGame` extends `USaveGame` — UE5 binary serialization (FMemoryArchive)
- Saves: simulation tick, all parcels, all corporations (financial state, owned assets), regional economy, contracts/alliances
- `UGTSaveLoadSubsystem` (GameInstance subsystem): save, load, delete, enumerate slots
- Internal slot naming: `GT_<SlotName>.sav` in `Saved/SaveGames/`
- Auto-save every 50 economic ticks to "AutoSave" slot
- Save version field for forward-compatibility guards

**Speed Controls:**
- `UGTSimulationSubsystem::SetPaused()` / `SetSpeedMultiplier()` (1x, 2x, 4x)
- Speed multiplier applied to DeltaTime before tick accumulation

**DefaultEngine.ini:**
- `GameInstanceClass=/Script/GlobalTelco.GTGameInstance`
- `GlobalDefaultGameMode=/Script/GlobalTelco.GTSinglePlayerGameMode`

## Hosting Architecture

```
Players ──► Cloudflare Workers (auth, matchmaking, APIs, CDN)
                │
                ▼
           Hetzner (UE5 dedicated server binary × 1-5 instances)
                │
                ▼
           Database (Cloudflare D1 or Hetzner managed PostgreSQL)
```

**Game servers:** Hetzner (hetzner.com) — x86_64 Linux dedicated servers
- Dev/testing: Hetzner Cloud CX22 (2 vCPU, 4GB RAM, ~€3.49/month)
- Production: Hetzner Dedicated AX42 (Ryzen 7 7700, 64GB RAM, ~€57/month, runs 3-5 sim instances)
- €20 signup credit covers ~5 months of dev usage

**Service layer:** Cloudflare Workers — authentication, account persistence, market APIs, matchmaking
- Free tier: 100k requests/day
- Paid ($5/month): 10M requests, Durable Objects, D1 database

## Dedicated Server Build (Source Engine Required)

The binary UE5 distribution from Epic Games Launcher **cannot build server targets**. Building a dedicated server requires a source-built engine:

1. Link Epic Games account to GitHub at unrealengine.com
2. Clone UE5 source: `git clone https://github.com/EpicGames/UnrealEngine.git -b 5.7`
3. Run `Setup.sh` then `GenerateProjectFiles.sh`
4. Build the engine: `make`
5. Use the source-built engine to build the server target:
   ```bash
   # Cross-compile Linux x86_64 server from Mac
   <source-engine-path>/Engine/Build/BatchFiles/RunUBT.sh \
     GlobalTelcoServer Linux Development \
     -project="/Users/kody/NuGit/GlobalTelco/GlobalTelco.uproject"
   ```

The current binary engine install at `/Users/Shared/Epic Games/UE_5.7/` works for editor and game client builds during development.
