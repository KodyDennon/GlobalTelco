# GlobalTelco: MVP to Production v1 Plan

Comprehensive phased implementation plan from current state to shippable production v1. Each item is a discrete deliverable that can be checked off. Phases are sequential — each builds on the previous.

---

## Current State Summary

**Built and functional:**
- 6-module C++ architecture (GlobalTelco, GTCore, GTInfrastructure, GTEconomy, GTMultiplayer, GTFrontend)
- Simulation engine with tick system, event queue, pause/speed controls
- Hex/geodesic grid, geo-coordinate math, world generation (terrain, regions, zoning)
- Network graph with Dijkstra routing, infrastructure nodes (6 types), edges (6 types)
- Corporation finance (balance sheet, income statement, credit rating, debt)
- Corporation manager, regional economy, alliance/contract basics
- Land parcel system with ownership, leasing, zoning
- Globe actor (Cesium + offline mode), globe pawn (orbit camera), globe interaction (click-to-select)
- Hex grid renderer (instanced mesh, LOD, terrain coloring)
- AI behavior tree system (4 archetypes, full BT tasks)
- Single-player game mode, save/load system, game instance session bridge
- Abstract C++ widget bases (main menu, new game, speed control, HUD, dashboard, parcel info)

**Not built:**
- All Blueprint/editor assets (widget layouts, maps, input actions, materials)
- Revenue-from-infrastructure pipeline
- Construction timer system
- Maintenance cost integration
- Congestion tracking
- Disaster system
- Technology/research tree
- Player build interaction
- Player financial management UI
- News/notification system
- Day/night cycle
- Tutorial
- Chat system
- Multiplayer server deployment
- Authentication & accounts
- Database persistence
- Cooperative ownership voting
- Sabotage, espionage, lobbying
- Hostile takeover, mergers, bankruptcy resolution
- Leaderboards, achievements
- Audio, polished visuals
- Steam/Epic integration
- Localization infrastructure

---

## Phase 1: Bootable Game

*Goal: The game compiles, launches, and you can see the globe and interact with it.*

- [ ] Fix all compilation errors across all 6 modules
- [ ] Create the main game map level (`/Game/Maps/GameWorld`)
- [ ] Create the main menu map level (`/Game/Maps/MainMenu`)
- [ ] Create Enhanced Input action assets (Orbit, Zoom, Pan, Select, BuildMode)
- [ ] Create Enhanced Input mapping context asset binding all input actions
- [ ] Create Blueprint subclass of `UGTMainMenuWidget` with basic UMG layout (buttons, text)
- [ ] Create Blueprint subclass of `UGTNewGameWidget` with basic UMG layout (dropdowns, sliders, text inputs)
- [ ] Create Blueprint subclass of `UGTSpeedControlWidget` with basic UMG layout (speed buttons, labels)
- [ ] Create Blueprint subclass of `UGTHUDWidget` with basic UMG layout (tick display, notification area)
- [ ] Create Blueprint subclass of `UGTDashboardWidget` with basic UMG layout (financials, infrastructure, regional data)
- [ ] Create Blueprint subclass of `UGTParcelInfoWidget` with basic UMG layout (parcel details panel)
- [ ] Wire the main menu map to display the main menu widget on load
- [ ] Wire the game map to spawn the globe actor, hex grid renderer, and HUD
- [ ] Verify: Launch game → see main menu → click New Game → configure settings → start → see globe → orbit/zoom → click hexes → see parcel info → pause/speed controls work → save/load works
- [ ] Verify: AI corporations spawn, make decisions, and build infrastructure over time

---

## Phase 2: Revenue Engine & Economic Integration

*Goal: Infrastructure generates money. Corporations earn revenue and pay costs based on what they own.*

### Revenue Calculation System
- [ ] Create `UGTRevenueCalculator` world subsystem
- [ ] Implement per-corporation revenue from owned nodes based on regional demand, node capacity, and network connectivity
- [ ] Implement per-corporation revenue from owned edges based on transit traffic and bandwidth utilization
- [ ] Implement SLA quality scoring per region (based on redundancy, latency, reliability of owned infrastructure)
- [ ] Implement demand satisfaction calculation (how much of a region's data demand is served by a corp's network)
- [ ] Wire revenue calculator to fire on every economic tick, feeding into each corporation's income statement

### Maintenance & Operating Costs
- [ ] Implement per-node maintenance cost deduction using actual `FGTNodeAttributes::MaintenanceCostPerTick` values
- [ ] Implement per-edge maintenance cost deduction using actual `FGTEdgeAttributes::MaintenanceCostPerTick` values
- [ ] Apply terrain multipliers to maintenance costs (mountainous/ocean = higher)
- [ ] Apply world settings `MaintenanceCostMultiplier` to all maintenance costs

### Construction Timer System
- [ ] Add `bUnderConstruction` flag and `RemainingConstructionTicks` to nodes and edges
- [ ] Implement construction countdown per economic tick
- [ ] Nodes/edges in construction state do not generate revenue or route traffic
- [ ] Apply terrain multipliers to construction time
- [ ] Apply world settings `ConstructionCostMultiplier` to all build costs
- [ ] Fire `InfrastructureBuilt` event when construction completes
- [ ] Update AI BT tasks to account for construction delays (don't double-build)

### Congestion Tracking
- [ ] Add `CurrentUtilization` tracking to nodes and edges
- [ ] Calculate utilization from routed traffic demand vs capacity
- [ ] Congestion penalties: degraded revenue, increased latency, customer churn risk
- [ ] Display congestion state in UI (green/yellow/red indicators)

### Individual Debt Instruments
- [ ] Replace single `TotalDebt` aggregate with `TArray<FGTDebtInstrument>` on corporation
- [ ] Each instrument: principal, interest rate, maturity tick, type (bank loan, bond, grant)
- [ ] Per-tick interest accrual on each instrument
- [ ] Maturity handling (auto-renewal or forced repayment)
- [ ] Wire AI finance task to use individual instruments

### Wiring & Integration
- [ ] Wire `ProcessEconomicTick()` on `UGTCorporation` to pull revenue from the revenue calculator
- [ ] Wire maintenance costs to deduct from corporation income each tick
- [ ] Wire construction timer processing into the simulation tick
- [ ] Verify: Start a game → AI corps build → revenue flows → maintenance deducted → corps grow/shrink financially over time

---

## Phase 3: Player Build UX

*Goal: The player can build infrastructure on the globe — place nodes, lay edges, manage a build queue.*

### Build Mode Interaction
- [ ] Add build mode toggle to `AGTGlobePawn` (input action to enter/exit build mode)
- [ ] In build mode: clicking a hex shows the build menu for that parcel
- [ ] Build menu: list available node types with costs, construction time, and requirements
- [ ] Selecting a node type places it on the parcel (starts construction)
- [ ] Edge creation mode: select source node → select target node → choose edge type → confirm
- [ ] Visual feedback: ghost/preview of node/edge before confirming placement
- [ ] Validation: check parcel ownership, zoning compatibility, sufficient funds, no duplicate nodes

### Build Queue & Construction UI
- [ ] Create `UGTBuildQueueWidget` — shows all pending constructions with progress bars
- [ ] Create `UGTBuildMenuWidget` — context menu when clicking a hex in build mode (node type selection)
- [ ] Create `UGTEdgeBuildWidget` — edge type selection between two selected nodes
- [ ] Display construction progress on globe (visual indicator on nodes/edges under construction)
- [ ] Cancel construction option (partial refund)

### Infrastructure Management Panel
- [ ] Create `UGTInfrastructurePanel` widget — lists all owned nodes and edges
- [ ] Show per-asset status (operational, degraded, under construction, destroyed)
- [ ] Show per-asset revenue contribution and maintenance cost
- [ ] Upgrade path display (future: upgrade node capacity via tech tree)
- [ ] Sell/demolish owned infrastructure option

### Player Financial Management
- [ ] Create `UGTFinanceWidget` — corporation financial overview
- [ ] Display: cash on hand, total revenue, total expenses, net income, total debt, credit rating, equity
- [ ] Take loan action (choose amount, see interest rate based on credit rating)
- [ ] Repay debt action (choose instrument to pay down)
- [ ] Income statement breakdown (revenue by source, expenses by category)
- [ ] Balance sheet view

### Blueprint Widget Assets
- [ ] Create Blueprint subclass of `UGTBuildQueueWidget` with UMG layout
- [ ] Create Blueprint subclass of `UGTBuildMenuWidget` with UMG layout
- [ ] Create Blueprint subclass of `UGTEdgeBuildWidget` with UMG layout
- [ ] Create Blueprint subclass of `UGTInfrastructurePanel` with UMG layout
- [ ] Create Blueprint subclass of `UGTFinanceWidget` with UMG layout
- [ ] Wire all build widgets into the HUD and input system

### Verification
- [ ] Verify: Enter build mode → click hex → see build menu → place tower → see construction timer → timer completes → node is operational → earning revenue
- [ ] Verify: Select two nodes → lay fiber edge → construction completes → traffic routes through edge
- [ ] Verify: Open finance panel → see revenue/expenses → take a loan → cash increases → interest accrues each tick

---

## Phase 4: Disaster System

*Goal: The world is dangerous. Disasters damage infrastructure, create risk, and force strategic decisions.*

### Disaster Event Generation
- [ ] Create `UGTDisasterSubsystem` world subsystem
- [ ] Implement disaster probability rolls per economic tick, per region (based on region's disaster risk profile)
- [ ] Apply world settings `DisasterFrequencyMultiplier` and `DisasterDamageMultiplier`
- [ ] Define disaster types: earthquake, hurricane/typhoon, flooding, landslide, volcanic eruption, political unrest, regulatory crackdown, cyber attack
- [ ] Each disaster type has: affected radius (number of hexes), severity range, duration, primary terrain targets

### Disaster Effects
- [ ] Disasters damage infrastructure in affected hexes (degrade or destroy nodes/edges based on severity)
- [ ] Damage scaled by node/edge `DisasterRisk` attribute and terrain exposure
- [ ] Political risk events: temporary regulatory changes (increased taxes, operating restrictions, forced shutdowns)
- [ ] Cascade effects: destroyed hub nodes disconnect downstream access nodes

### Repair System
- [ ] Degraded infrastructure can be repaired (costs money, takes time)
- [ ] Destroyed infrastructure must be rebuilt from scratch
- [ ] Auto-repair option (higher cost, AI corps use this)
- [ ] Repair queue visible in infrastructure management panel
- [ ] Emergency repair (instant, very expensive)

### Insurance System
- [ ] Implement basic insurance: per-asset insurance premium (ongoing cost), covers partial rebuild on destruction
- [ ] Insurance tier options (basic, comprehensive, premium) with different coverage levels
- [ ] Insurance payout on disaster damage

### Resilience & Prevention
- [ ] Redundancy bonus: regions with multiple independent paths suffer less from single-point failures
- [ ] Backup power attribute on nodes (reduces degradation from power-related disasters)

### Notification & Visualization
- [ ] Disaster event notifications in the HUD feed
- [ ] Visual effects on globe for active disasters (color overlay on affected hexes)
- [ ] Disaster history log accessible from dashboard

### Verification
- [ ] Verify: Play on Brutal difficulty → disasters occur → infrastructure gets damaged → repair costs money → some corps go bankrupt from repeated disasters
- [ ] Verify: Insurance reduces financial impact of disasters
- [ ] Verify: Notifications appear when disasters strike

---

## Phase 5: Technology & Research System

*Goal: Corporations invest in R&D to unlock competitive advantages.*

### Tech Tree Definition
- [ ] Define technology categories: Optical Networks, Wireless/5G, Satellite Systems, Data Center, Network Resilience, Operational Efficiency
- [ ] Define 5-8 technologies per category (30-48 total), each with: name, description, research cost, prerequisite techs, unlock effects
- [ ] Example techs: Advanced Optical Amplifiers (+50% fiber capacity), Automated Maintenance (-30% maintenance costs), Weather-Resistant Towers (-50% disaster damage to towers)
- [ ] Store tech definitions as code-defined data (no editor assets needed), similar to AI archetype pattern

### Research System
- [ ] Add `R&D Budget` to corporation (per-tick spending allocation)
- [ ] Research progress accumulates per tick based on budget and `ResearchSpeedMultiplier`
- [ ] Only one technology researched at a time per corporation
- [ ] Research completion fires event, unlocks bonuses

### Technology Effects
- [ ] Apply tech bonuses to infrastructure attributes (capacity, reliability, maintenance cost, construction time)
- [ ] Tech bonuses stack and apply globally to all owned infrastructure of the affected type
- [ ] Display active tech bonuses in infrastructure panel and finance panel

### Patent & Licensing System
- [ ] Completed research becomes a patent owned by the researching corporation
- [ ] Patents can be licensed to other corporations for ongoing royalty revenue
- [ ] License negotiation: set price, other corps accept/decline
- [ ] AI corps evaluate license offers based on archetype (tech innovator values patents higher)

### Research UI
- [ ] Create `UGTResearchWidget` — tech tree display with category tabs
- [ ] Show: available techs, prerequisites, current research progress, cost, unlock effects
- [ ] Set R&D budget slider
- [ ] Select next research target
- [ ] View owned patents and active licenses

### Blueprint Assets
- [ ] Create Blueprint subclass of `UGTResearchWidget` with UMG layout

### Verification
- [ ] Verify: Set R&D budget → select tech → progress bar advances → tech unlocks → bonuses applied to infrastructure
- [ ] Verify: AI corps research technologies based on archetype preference
- [ ] Verify: License a patent to AI corp → receive royalty income each tick

---

## Phase 6: Complete UI & HUD

*Goal: Every piece of information the player needs is accessible through polished UI panels.*

### Corporate Dashboard (full implementation)
- [ ] Wire `UGTDashboardWidget` to pull live data from corporation, revenue calculator, regional economy
- [ ] Financial overview tab: cash, revenue, expenses, net income, debt, credit rating (charts over time)
- [ ] Infrastructure tab: owned nodes/edges by type, status breakdown, total capacity, utilization
- [ ] Regional tab: market share per region, demand satisfaction, competitor presence
- [ ] Contracts tab: active contracts, pending proposals, revenue from contracts

### News & Notification System
- [ ] Create `UGTNewsSubsystem` world subsystem
- [ ] Generate news events from simulation events (corp bankruptcies, major builds, disasters, contract deals, tech breakthroughs)
- [ ] Global news ticker (scrolling text at top/bottom of screen)
- [ ] Personal notification panel (filterable: infrastructure, financial, competitor, disaster, contract)
- [ ] Notification urgency levels (info, warning, critical) with visual distinction
- [ ] Click notification to jump to relevant location on globe

### Map Overlays
- [ ] Create overlay toggle system on the globe
- [ ] Overlay: terrain type coloring (already exists in hex renderer — expose as toggle)
- [ ] Overlay: ownership coloring (color hexes by owning corporation)
- [ ] Overlay: regional demand heatmap (color by data demand intensity)
- [ ] Overlay: disaster risk heatmap
- [ ] Overlay: network coverage (which hexes have infrastructure serving them)
- [ ] Overlay: congestion heatmap (green/yellow/red on edges)
- [ ] Overlay toggle buttons in the HUD

### Minimap / World Overview
- [ ] Create `UGTMinimapWidget` — small globe thumbnail showing current viewport position
- [ ] Click minimap to jump to that region

### Tooltip System
- [ ] Hover over hex → show quick tooltip (terrain, owner, zoning, demand)
- [ ] Hover over node → show quick tooltip (type, status, capacity, utilization, owner)
- [ ] Hover over edge → show quick tooltip (type, status, bandwidth, latency, utilization)
- [ ] Hover over corporation name → show quick tooltip (cash, credit rating, node count)

### Settings / Options Menu
- [ ] Create `UGTSettingsWidget` — game settings panel
- [ ] Graphics settings (resolution, quality presets, render distance)
- [ ] Audio settings (master, music, SFX, ambient volume sliders)
- [ ] Gameplay settings (auto-save frequency, notification preferences, camera sensitivity)
- [ ] Keybinding display (show current bindings)
- [ ] Apply/Save/Cancel settings

### Blueprint Assets for All UI
- [ ] Create Blueprint subclasses for every new widget with UMG layout
- [ ] Consistent visual theme across all widgets (color palette, font choices, panel styling)
- [ ] UI animations (panel slide-in/out, button hover/press states, progress bar animations)

### Verification
- [ ] Verify: All dashboard tabs show live, accurate data
- [ ] Verify: News ticker updates as events occur in the simulation
- [ ] Verify: Map overlays toggle correctly and display accurate data
- [ ] Verify: Tooltips appear on hover with correct info
- [ ] Verify: Settings save and apply correctly

---

## Phase 7: Game Content & World Polish

*Goal: The game looks and feels like a real game, not a tech demo.*

### Visual Quality Pass
- [ ] Source or create cohesive hex tile materials (terrain-appropriate: grass, sand, rock, water, urban, snow)
- [ ] Node actor meshes — distinct 3D models or stylized meshes for each of the 6 node types (use free Unreal Marketplace assets or procedural meshes)
- [ ] Edge visualization — visible cables/connections between nodes on the globe (spline meshes or line renders)
- [ ] Corporation color coding — each corp gets a distinct color, applied to owned nodes/edges/hexes
- [ ] Atmosphere / skybox for the globe scene
- [ ] Globe lighting (sun position, ambient light, shadow quality)
- [ ] Post-processing: bloom, ambient occlusion, color grading for the "semi-realistic strategy game" aesthetic
- [ ] UI art pass: panel backgrounds, button styles, icon set, consistent typography

### Day/Night Cycle
- [ ] Implement accelerated day/night cycle (1 game day = configurable real-time duration)
- [ ] Sun position drives lighting on the globe
- [ ] City lights appear on urban hexes at night (emissive material parameter)
- [ ] Day/night is cosmetic only (no gameplay effect)

### Audio
- [ ] Ambient music: 3-5 tracks for gameplay (calm strategic mood, varying intensity)
- [ ] Main menu music: 1 track
- [ ] UI sound effects: button clicks, panel open/close, notification chimes, build placement confirmation
- [ ] Disaster sound effects: rumble for earthquakes, wind for storms
- [ ] Construction completion sound
- [ ] Revenue/income chime on economic tick
- [ ] Audio manager that handles crossfading and volume control

### Camera Polish
- [ ] Smooth camera transitions when selecting different regions
- [ ] Camera shake on nearby disaster events
- [ ] Camera zoom-to-fit when selecting a region from the dashboard
- [ ] Keyboard shortcuts for common camera positions (home to player HQ, zoom to event)

### Tutorial System
- [ ] Create `UGTTutorialSubsystem` — manages tutorial step progression
- [ ] Tutorial sequence: welcome → camera controls → select a parcel → buy your first parcel → build your first tower → connect two towers with fiber → view revenue → take a loan → set R&D → pause/speed → save
- [ ] Tutorial hint widgets (arrows, highlights, text boxes pointing at UI elements)
- [ ] Tutorial can be skipped
- [ ] Tutorial only triggers on first new game (flag stored in game instance)

### Verification
- [ ] Verify: Game looks cohesive and intentional (not placeholder-quality)
- [ ] Verify: Day/night cycle visually works
- [ ] Verify: Audio plays correctly with volume controls
- [ ] Verify: Tutorial guides a new player through all core mechanics

---

## Phase 8: Advanced Gameplay Systems

*Goal: Competitive depth — the systems that make the game strategically interesting beyond basic build-and-earn.*

### Bankruptcy Resolution
- [ ] Detect corporation insolvency (negative cash + maxed debt + no assets to sell)
- [ ] Present player choice: government bailout (high-interest emergency loan, keeps playing) or declare bankruptcy (assets liquidated, corporation dissolved, restart with new corp)
- [ ] AI corps auto-choose based on archetype (aggressive = bailout, prudent = bankruptcy + restart)
- [ ] Bankrupt corporation's assets go to auction

### Land Auctions
- [ ] Government auctions for unclaimed premium land parcels (periodic events)
- [ ] Auction for bankrupt corporation's assets (nodes, edges, parcels)
- [ ] Sealed-bid auction system (all corps submit bids, highest wins)
- [ ] AI bidding logic based on archetype and financial state

### Hostile Takeover & Mergers
- [ ] Propose acquisition of another corporation (offer price per share of equity)
- [ ] Target corporation can accept or reject (AI evaluates based on offer vs book value)
- [ ] Successful acquisition: absorb all assets, debt, and contracts
- [ ] Merger option: two corps merge into one (combined assets and shared control — AI only for v1)

### Sabotage & Espionage
- [ ] Espionage action: spend money to reveal a competitor's infrastructure stats in a region
- [ ] Sabotage action: spend money to temporarily degrade a competitor's infrastructure (risk of detection)
- [ ] Detection and consequences: caught sabotage triggers lawsuit, financial penalty, reputation damage
- [ ] Counter-espionage: invest in security to reduce sabotage vulnerability

### Lobbying & Political Influence
- [ ] Lobby regional government for favorable regulation (reduced taxes, relaxed zoning, fast-track permits)
- [ ] Lobby against competitors (increased regulatory burden on rivals in a region)
- [ ] Political influence has diminishing returns and can backfire (scandal event)
- [ ] AI corps lobby based on archetype (aggressive = heavy lobbying, budget = minimal)

### Cooperative Infrastructure
- [ ] Multi-owner nodes and edges (already have OwnerCorporationIds arrays)
- [ ] Revenue splitting proportional to ownership stake
- [ ] Upgrade voting: majority of owners must approve upgrades
- [ ] Buyout offers between co-owners

### Grants & Development Programs
- [ ] Government grants for building in underserved regions (free money with build requirements)
- [ ] Tax incentives for certain infrastructure types in developing regions
- [ ] Grant applications with requirements and deadlines
- [ ] Penalty for failing to meet grant obligations

### Win Conditions & Achievements
- [ ] Leaderboard system: rank corporations by revenue, net worth, coverage, tech level
- [ ] Achievement system: milestone unlocks (first international cable, first profitable quarter, survive 3 disasters, etc.)
- [ ] Optional victory conditions for single-player (dominate X% of global traffic, reach $X net worth, achieve AAA credit rating)
- [ ] End-game summary screen with stats and timeline

### Verification
- [ ] Verify: Corporation can go bankrupt → player chooses bailout or restart
- [ ] Verify: Land auctions occur and AI corps bid
- [ ] Verify: Espionage reveals competitor data
- [ ] Verify: Lobbying affects regional regulation
- [ ] Verify: Achievements trigger correctly

---

## Phase 9: Multiplayer Foundation

*Goal: Two or more players can connect to a dedicated server and play together.*

### Dedicated Server Build
- [ ] Set up UE5 source engine build (clone, build from source)
- [ ] Verify `GlobalTelcoServer` target compiles for Linux x86_64
- [ ] Package server binary for Hetzner deployment

### Network Replication Audit
- [ ] Audit all world subsystems for proper server-authority (simulation only runs on server)
- [ ] Audit `AGTNetworkNode` replication setup (already has `GetLifetimeReplicatedProps`)
- [ ] Ensure `AGTGameState` replicates simulation tick and time to clients
- [ ] Ensure `AGTPlayerController` properly replicates corporation assignment
- [ ] Ensure infrastructure changes replicate to all clients (node creation, edge creation, status changes)
- [ ] Ensure land parcel ownership changes replicate
- [ ] Ensure corporation financial data replicates (at least summary data for leaderboard)

### Server Game Mode
- [ ] Update `AGTGameMode` (multiplayer mode) to handle multiple player corporations
- [ ] Login flow: assign or create corporation for connecting player
- [ ] Logout flow: corporation persists, AI takes temporary control (or pauses if single remaining player)
- [ ] Handle mid-game joins (player gets a corporation in the existing world)
- [ ] Enforce 250-player cap (already exists)

### Client-Server Communication
- [ ] Server RPCs for player actions: buy parcel, build node, build edge, take loan, set R&D budget, propose contract
- [ ] Client RPCs for feedback: action accepted/rejected, notifications
- [ ] Ensure all player actions validate on server (no client-side cheating)

### Server Browser
- [ ] Create `UGTServerBrowserWidget` — lists available servers
- [ ] Query running servers (via HTTP API or UE5 online subsystem)
- [ ] Display: server name, player count, world age (tick count), ping
- [ ] Connect button
- [ ] Host game button (starts local dedicated server)

### Basic Account System
- [ ] Create account service (Cloudflare Workers or simple REST API)
- [ ] Register (email + password)
- [ ] Login (email + password → session token)
- [ ] Store player profile (display name, corporation history)
- [ ] Session token passed on server connection for identity

### Blueprint Assets
- [ ] Create Blueprint subclass of `UGTServerBrowserWidget` with UMG layout
- [ ] Update main menu to include "Multiplayer" button alongside "New Game" and "Load Game"

### Verification
- [ ] Verify: Build and run dedicated server on Linux
- [ ] Verify: Two clients connect to same server → both see the same world
- [ ] Verify: Player A builds a tower → Player B sees it appear
- [ ] Verify: Server browser shows running servers with accurate info

---

## Phase 10: Multiplayer Services & Persistence

*Goal: Multiplayer worlds persist across server restarts. Accounts are real. The online experience works.*

### Database Persistence
- [ ] Set up PostgreSQL on Hetzner (or Cloudflare D1 for lightweight)
- [ ] Define schema: worlds, corporations, parcels, infrastructure, contracts, alliances, accounts
- [ ] Implement world state serialization to database (periodic saves, like auto-save but to DB)
- [ ] Implement world state restoration from database on server startup
- [ ] Handle schema migrations for updates

### World Persistence
- [ ] Server saves world state to database every N ticks (configurable)
- [ ] On server restart: load world state from database, resume simulation
- [ ] Player corporation state persists between sessions (disconnect → reconnect later → same corp)
- [ ] World continues simulating when players are offline (AI corps keep playing)

### Cloudflare Workers API Layer
- [ ] Authentication endpoints (register, login, refresh token, password reset)
- [ ] World list API (available servers, player counts, world metadata)
- [ ] Player profile API (display name, stats, corporation history)
- [ ] Market data API (global leaderboards, world statistics)

### Text Chat System
- [ ] Implement chat channels: global, regional (based on player's HQ region), alliance, direct message
- [ ] Chat UI widget with channel tabs, message history, input box
- [ ] Server-side chat relay (messages sent via server, not P2P)
- [ ] Basic moderation: mute player, report message
- [ ] Chat message replication to relevant clients only (regional chat → only players in that region)

### Blueprint Assets
- [ ] Create Blueprint subclass of chat widget with UMG layout

### Verification
- [ ] Verify: Server saves to database → restart server → world state fully restored
- [ ] Verify: Player disconnects → reconnects next day → same corporation, same state
- [ ] Verify: Chat messages appear for the correct channel recipients
- [ ] Verify: Account creation, login, and session persistence work

---

## Phase 11: Localization & Accessibility

*Goal: The game is localization-ready and accessible.*

### Localization Infrastructure
- [ ] Audit all user-facing strings — convert raw `TEXT()` literals to `NSLOCTEXT()` or `LOCTEXT()` macros
- [ ] Create string tables for all UI text
- [ ] Create string tables for all game data text (node type names, tech names, disaster names, etc.)
- [ ] Set up FText-based formatting for all dynamic strings (numbers, dates, names)
- [ ] Test that switching locale changes all displayed text (even if only English exists)

### Accessibility
- [ ] Colorblind-friendly mode: provide alternative color schemes for map overlays and corporation colors
- [ ] UI scaling option (support different text/UI sizes)
- [ ] Keyboard navigation for all menus (tab through buttons, enter to select)
- [ ] Screen reader hints on critical UI elements (ARIA-like metadata on UMG widgets)

### Verification
- [ ] Verify: No hardcoded English strings in UI
- [ ] Verify: Colorblind mode is visually distinct
- [ ] Verify: All menus navigable by keyboard

---

## Phase 12: Platform Integration & Distribution

*Goal: The game can be packaged and distributed on Steam, Epic, and self-hosted.*

### Steam Integration
- [ ] Integrate Steamworks SDK (Online Subsystem Steam)
- [ ] Steam authentication (login via Steam)
- [ ] Steam achievements (map to in-game achievement system)
- [ ] Steam cloud saves (sync local save files)
- [ ] Steam rich presence (show current game state in friends list)
- [ ] Create Steam store page assets (capsule images, screenshots, description, tags)
- [ ] Steam build upload pipeline (SteamPipe depot configuration)

### Epic Games Store Integration
- [ ] Integrate Epic Online Services SDK
- [ ] Epic authentication
- [ ] Epic achievements
- [ ] Epic cloud saves
- [ ] EGS store page setup

### Self-Distribution
- [ ] Create standalone installer/package for Windows and Mac
- [ ] Auto-update mechanism (check version on launch, download patches)
- [ ] License key or account-based activation (optional, depending on business model)
- [ ] Distribution via website or itch.io

### Platform Packaging
- [ ] Configure UE5 packaging settings for Windows (Development and Shipping)
- [ ] Configure UE5 packaging settings for Mac (Development and Shipping)
- [ ] Strip debug symbols for shipping builds
- [ ] Configure pak file encryption (optional, for asset protection)
- [ ] Test packaged builds on clean machines (no UE5 installed)

### Verification
- [ ] Verify: Steam build launches, authenticates, uploads achievements, syncs saves
- [ ] Verify: Epic build launches and authenticates
- [ ] Verify: Self-distributed build installs and runs on a clean Windows/Mac machine

---

## Phase 13: Production Hardening

*Goal: The game is stable, performant, and ready for real players.*

### Performance Optimization
- [ ] Profile simulation tick performance (target: full tick processing in <50ms at 100k parcels)
- [ ] Profile hex grid rendering (target: 60fps with 100k hexes, LOD culling working)
- [ ] Profile network graph routing (target: Dijkstra <10ms per query)
- [ ] Memory profiling (identify and fix any leaks, reduce unnecessary allocations)
- [ ] Optimize AI behavior tree execution (target: 10 AI corps processing in <20ms total)
- [ ] World partition / level streaming for large worlds (if needed for performance)
- [ ] Texture streaming and LOD configuration for visual assets

### Stability & Error Handling
- [ ] Graceful handling of all nullptr subsystem scenarios (world not ready, subsystem not initialized)
- [ ] Save file corruption recovery (detect corrupt saves, offer to delete or attempt recovery)
- [ ] Network disconnection handling (reconnect prompt, local state preservation)
- [ ] Crash reporter integration (collect crash dumps for analysis)
- [ ] Extensive logging with log categories (replace `LogTemp` with proper `DECLARE_LOG_CATEGORY`)

### Save System Hardening
- [ ] Save migration system: handle loading saves from older versions (version-based migration functions)
- [ ] Save file integrity check (checksum/hash verification)
- [ ] Multiple auto-save slots (rotating: AutoSave1, AutoSave2, AutoSave3)

### Anti-Cheat (Multiplayer)
- [ ] Server-authoritative validation on all player actions (already designed, verify completeness)
- [ ] Rate limiting on player actions (prevent action spam)
- [ ] Sanity checks on client-reported data (position, selection)
- [ ] Admin tools: kick, ban, view player stats

### QA Checklist
- [ ] Full playthrough: new single-player game on each difficulty → play 100+ ticks → verify all systems function
- [ ] Save/load cycle: save at various game states → load each → verify state matches
- [ ] Multiplayer session: 2+ players, play 50+ ticks, verify sync and replication
- [ ] Edge cases: 0 AI corps, 10 AI corps, 0 world seed, max world seed
- [ ] Disaster stress test: Brutal difficulty, verify infrastructure damage and repair
- [ ] Financial stress test: force bankruptcy, verify resolution flow
- [ ] Performance test: 100k+ parcels, 10 AI corps, 4x speed, verify no frame drops

### Verification
- [ ] Verify: Game runs stable for 1000+ ticks with no crashes
- [ ] Verify: Memory usage stays bounded over long sessions
- [ ] Verify: Packaged builds perform same as editor builds

---

## Phase 14: Launch Preparation

*Goal: Everything is ready for players to download and play.*

### Content Completeness Audit
- [ ] All 6 node types have distinct visuals and are buildable
- [ ] All 6 edge types have distinct visuals and are buildable
- [ ] All terrain types render correctly on the globe
- [ ] All difficulty presets produce balanced gameplay
- [ ] All 4 AI archetypes exhibit distinct behavior
- [ ] All tech tree items are researchable and apply effects
- [ ] All disaster types can trigger and resolve
- [ ] All UI panels show accurate, live data
- [ ] All save/load scenarios work correctly
- [ ] Tutorial completes without errors
- [ ] Main menu → new game → gameplay → save → quit → load → resume cycle is flawless

### Documentation
- [ ] Update CLAUDE.md with all new systems, classes, and architecture
- [ ] Update game_design_decisions.md with all implementation decisions made during development
- [ ] Create player-facing "How to Play" guide (in-game or external)
- [ ] Create server admin guide (how to host a dedicated server)

### Final Polish
- [ ] Loading screen with tips/lore while levels transition
- [ ] Credits screen
- [ ] Version number display in main menu
- [ ] "What's New" dialog on first launch after update
- [ ] Splash screen / studio logo on launch

### Verification
- [ ] Fresh install on clean machine → complete the full new-player flow → everything works
- [ ] Hand the game to someone who has never seen it → they can figure out the basics via tutorial

---

## Milestone Summary

| Milestone | Phases | What It Means |
|-----------|--------|---------------|
| **Bootable** | 1 | Game compiles, launches, globe works, AI plays |
| **Playable** | 1-3 | Player can build, earn, manage finances — it's a game |
| **Feature-Complete SP** | 1-6 | All single-player systems working with full UI |
| **Content-Complete SP** | 1-8 | Polished visuals, audio, tutorial, all gameplay systems |
| **MVP** | 1-9 | Single-player complete + basic multiplayer works |
| **Online-Ready** | 1-10 | Persistent multiplayer with accounts, chat, database |
| **Ship-Ready** | 1-12 | Platform integration, localization-ready, distributed |
| **Production v1** | 1-14 | Fully hardened, tested, polished, launched |
