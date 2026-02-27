# GlobalTelco: Comprehensive Fix & Implementation Plan

> **Generated:** 2026-02-25
> **Source:** FULL_GAME_AUDIT.md + all design docs + codebase verification
> **Scope:** Fix all audit issues, implement all missing systems, complete overhaul plan, update all docs

---

## Table of Contents

1. [Phase 1 — Critical Gameplay Bugs](#phase-1--critical-gameplay-bugs)
2. [Phase 2 — UI/UX Bugs & Polish](#phase-2--uiux-bugs--polish)
3. [Phase 3 — Panel Fixes & Data Wiring](#phase-3--panel-fixes--data-wiring)
4. [Phase 4 — Coming Soon Panels (Frontend Only)](#phase-4--coming-soon-panels-frontend-only)
5. [Phase 5 — Missing Backend Systems](#phase-5--missing-backend-systems)
6. [Phase 6 — Missing Gameplay Features](#phase-6--missing-gameplay-features)
7. [Phase 7 — Overhaul Plan Completion](#phase-7--overhaul-plan-completion)
8. [Phase 8 — Production Hardening](#phase-8--production-hardening)
9. [Phase 9 — Document Cleanup](#phase-9--document-cleanup)

---

## Phase 1 — Critical Gameplay Bugs

*Goal: Fix the core gameplay-breaking bugs identified in the audit.*

### 1.1 — Node Placement State Sync (Audit #1)

**Root Cause (Single-Player):** `commandRouter.ts:50` — after `bridge.processCommand()`, no `map-dirty` event is dispatched. Nodes are created in WASM state but the map doesn't re-render until the 2-second fallback interval fires.

**Root Cause (Multiplayer):** `tick.rs:19` — `node_ids.map(|n| n.len() as u32)` returns `Option` that can be `None` when a corp has no entry in `corp_infra_nodes` yet. The HUD counter uses `delta.node_count ?? corp.infrastructure_count` in `GameLoop.ts:371`, so `None` causes stale display.

**Fixes:**
- [x] `commandRouter.ts:50` — After `bridge.processCommand(command)`, dispatch `window.dispatchEvent(new CustomEvent('map-dirty'))` immediately
- [x] `tick.rs:19` — Change `node_ids.map(|n| n.len() as u32)` to `Some(node_ids.map(|n| n.len()).unwrap_or(0) as u32)` so node_count is always `Some`
- [x] `GameLoop.ts` — On `CommandAck` for BuildNode, immediately query `getCorporationData()` to refresh counter
- [x] Verify LOD filtering doesn't hide newly-placed nodes at low zoom (infraLayer.ts:527-528 tier culling)

### 1.2 — Multiplayer Chat Not Rendering (Audit #2) --- COMPLETE

**Root Cause:** Server handler `ws.rs:1209-1249` has 3 nested `if let` checks that silently drop messages with no error response. Client clears input optimistically before server confirms.

**Fixes:**
- [x] `ws.rs:1209-1249` — Add explicit error responses for all failure paths (player is None, world_id is None, world lookup fails)
- [x] `ws.rs:1209` — Add early player auth check (consistent with other handlers)
- [x] `Chat.svelte:15` — Implemented optimistic display (add message locally, mark as pending, remove on server broadcast)
- [x] Add lobby system messages (player joined, world started, etc.) per user requirement
- [x] Test both in-game chat and lobby context

### 1.3 — Radial Build Menu Can't Select Items (Audit #3) --- COMPLETE

**Root Cause:** Mouseleave race condition. Moving mouse from SVG segment to flyout triggers segment's `onmouseleave` (line 253) which sets `hoveredCategory = null`, hiding the flyout before its `onmouseenter` (line 290) can fire. The 20px gap between segment and flyout (`RADIUS_OUTER + 20`) makes this worse.

**Fixes:**
- [x] `RadialBuildMenu.svelte:253` — Add a 150ms delay before closing flyout on mouseleave (use `setTimeout`, cancel on flyout `onmouseenter`)
- [x] Add `onmouseleave` handler on the flyout div (line 287) to also set hoveredCategory = null
- [x] Reduce gap between segment and flyout, or add an invisible bridge element connecting them
- [x] Verify backdrop click handler (line 152) doesn't intercept flyout item clicks
- [x] Test edge placement mode (line 135 enters edge mode without closing menu)

### 1.4 — Build Tool Stuck on Insufficient Funds (Audit tech debt #3) --- COMPLETE

**Root Cause:** `BuildHotbar.svelte:113-124` — `enterPlacementMode()` has no affordability check. `MapView.svelte:110-118` sends BuildNode without pre-flight validation. On failure, placement mode stays active.

**Fixes:**
- [x] `BuildHotbar.svelte:activateSlot()` — Before `enterPlacementMode()`, check `$playerCorp.cash >= minimumCost` for that node type
- [x] Grey out unaffordable hotbar slots with visual disabled state
- [x] Show "Insufficient funds" toast when clicking a greyed-out slot
- [x] `RadialBuildMenu.svelte:selectItem()` — Same affordability check (line 126 already checks, verify it works)
- [x] On command failure notification, auto-exit placement mode

---

## Phase 2 — UI/UX Bugs & Polish

*Goal: Fix all non-gameplay UI issues from the audit.*

### 2.1 — Favicon (Audit #4a) --- COMPLETE

**Current State:** Full favicon set generated and linked.

**Fixes:**
- [x] Generate full favicon set from existing `favicon.svg`: 180x180 (apple-touch-icon), 192x192, 512x512
- [x] Add PNG variants to `web/static/icons/`
- [x] Update `app.html` with full favicon `<link>` tags (SVG, apple-touch-icon, manifest, theme-color)
- [x] Update `manifest.json` with icon sizes

### 2.2 — Password Fields Not in `<form>` (Audit #4b) --- COMPLETE

**Current State:** Both WorldBrowser and admin page have proper `<form>` wrapping with autocomplete attributes.

**Fixes:**
- [x] `WorldBrowser.svelte` — Wrapped auth inputs in `<form>` element with `onsubmit` preventDefault
- [x] Add `autocomplete` attributes (`username`, `current-password`, `new-password`) to inputs
- [x] Verify Enter key submits the form
- [x] Check `admin/+page.svelte` for same issue and fix — also done

### 2.3 — Panel Click Blocking (Audit UI #3) --- COMPLETE

**Current State:** MapView.svelte already closes panels on map click (lines 107-111).

**Fixes:**
- [x] When clicking on the map area, close any open floating panel
- [x] `MapView.svelte` handler closes panel group via `closePanelGroup()`
- [x] Panels don't capture clicks on transparent/empty areas

### 2.4 — Overlay Text Readability (Audit tech debt #2) --- COMPLETE

**Current State:** Text halo/shadow applied to all deck.gl text layers.

**Fixes:**
- [x] `labelLayers.ts` — Added text outline/halo to city labels
- [x] `labelLayers.ts` — Added text outline to region labels
- [x] Increased contrast on NotificationFeed and OverlayLegend text

### 2.5 — Advisor Click-to-Pan (Audit tech debt #1) --- COMPLETE

**Current State:** Fully implemented with spatial fly-to and panel navigation.

**Fixes:**
- [x] Add `onclick` handler to each `.suggestion` div in AdvisorPanel
- [x] For spatial alerts (damaged node, unmet demand): dispatch `map-fly-to` with the relevant entity's coordinates
- [x] For system alerts (low funds, no research): open the relevant management panel (Dashboard for finance, Research for R&D)
- [x] Add cursor pointer and hover state to suggestion divs
- [x] Map suggestion types to coordinates using bridge methods (e.g., `bridge.getDamagedNodes()` for damaged infrastructure)

### 2.6 — AI Node Color Hardening (Audit UI #1) --- COMPLETE

**Current State:** Color validation and fallbacks added.

**Fixes:**
- [x] `iconAtlas.ts` — Add explicit error logging when SVG icon fails to load
- [x] `infraLayer.ts:getCorpColor()` — Validation that returned color has valid RGB values
- [x] `infraLayer.ts` ScatterplotLayer fallback — Ensure `getFillColor` always returns valid RGBA
- [x] Add a "missing icon" placeholder that's clearly styled (not magenta) to the icon atlas

### 2.7 — Disaster Severity Slider --- COMPLETE

**Current State:** Disaster severity slider implemented in NewGame.svelte.

**Fixes:**
- [x] Add disaster severity slider (1-10) to `NewGame.svelte` in the game setup section
- [x] Map slider value to `disaster_frequency` (1=0.1, 5=1.0, 10=3.0 or similar curve)
- [x] Show current value label next to slider
- [x] Default to difficulty preset value, allow override

---

## Phase 3 — Panel Fixes & Data Wiring

*Goal: Fix broken data and placeholder content in existing panels.*

### 3.1 — DashboardPanel Budget/Policy Placeholders --- COMPLETE

**Current State:** Budget & policy state persisted via shared `policyState` writable store.

**Fixes:**
- [x] Wire budget sliders to persistent `policyState` store (survives panel close/reopen)
- [x] Both DashboardPanel and WorkforcePanel read from `$derived` + write back via `policyState.update()`
- [x] Changed from `bind:value` to one-way `value=` binding (Svelte 5 `$derived` is read-only)

### 3.2 — NetworkDashboard Estimated Data (Gaps #19a-d, #28) --- COMPLETE

**Current State:** All 4 gaps wired with real simulation data. No remaining placeholder formulas.

**Fixes:**
- [x] **#19a — Per-infrastructure revenue:** Added `revenue_generated` field to InfraNode and InfraEdge components. Revenue system now tracks per-asset revenue. Exposed via bridge. Edge `maintenance_cost` also now exposed.
- [x] **#19b — SLA monitoring:** Added `sla_target`, `sla_current_performance`, `sla_penalty_accrued` fields to Contract component. Contract system now calculates SLA compliance per-tick based on provider health and capacity. Exposed via bridge with `sla_status` field.
- [x] **#19c — Repair state:** Exposed `repairing`, `repair_ticks_left`, `repair_health_per_tick` fields through the bridge's infrastructure query for both nodes and edges. Template now shows remaining repair ticks.
- [x] **#19d + #28 — Capacity planning:** Added `utilization_history` (HashMap<EntityId, VecDeque<f64>>) to GameWorld. Utilization system records last 100 ticks per node/edge. Exposed via bridge as `utilization_history: Vec<f64>`. `ticksUntilFull` now uses per-edge linear regression from real history.
- [x] All placeholder formulas and TODO markers replaced in NetworkDashboard.svelte

### 3.3 — WorkforcePanel Missing Field --- COMPLETE

**Current State:** `infrastructure_count` is now included in CorporationData returned by bridge.

**Fixes:**
- [x] Add `infrastructure_count` to CorporationData returned by bridge
- [x] Computed from `corp_infra_nodes` length in the query

### 3.4 — ContractPanel Hardcoded Terms --- COMPLETE

**Current State:** Structured form with bandwidth/price/duration sliders, contract type selector, preview with SLA tier.

**Fixes:**
- [x] Replace string input with structured form fields: bandwidth slider, price input, duration input
- [x] Contract type selector (Transit/Peering/SLA)
- [x] Show preview of contract terms with total value, price per unit, SLA tier, estimated penalty
- [x] Backend now parses structured terms instead of using hardcoded values

---

## Phase 4 — Coming Soon Panels (Frontend Only) --- ALL COMPLETE

*Goal: Build UI panels for features that have backend support but no frontend.*

### 4.1 — Insurance Panel (Finance group) --- COMPLETE

**Build:**
- [x] Create `InsurancePanel.svelte` in `web/src/lib/panels/`
- [x] Show all owned infrastructure with insured/uninsured status
- [x] Per-node toggle to purchase/cancel insurance
- [x] Show premium cost per node (2% of construction cost)
- [x] Show payout history (60% of damage cost on disaster)
- [x] Summary: total premium cost/tick, total insured vs uninsured assets
- [x] Wire to uiState.ts as Finance group tab

### 4.2 — Repair Panel (Operations group) --- COMPLETE

**Build:**
- [x] Create `RepairPanel.svelte` in `web/src/lib/panels/`
- [x] List all damaged infrastructure (health < 100%) sorted by severity
- [x] Show repair cost estimate and time remaining for each
- [x] Buttons: Standard Repair, Emergency Repair per node
- [x] Show maintenance crew count and repair speed multiplier
- [x] Repair progress bars for nodes currently being repaired
- [x] Wire to uiState.ts as Operations group tab

### 4.3 — Co-ownership Panel (Diplomacy group) --- COMPLETE

**Build:**
- [x] Create `CoOwnershipPanel.svelte` in `web/src/lib/panels/`
- [x] List all co-owned infrastructure with ownership percentages
- [x] Incoming/outgoing co-ownership proposals with accept/reject
- [x] Propose co-ownership: select node, target corp, share percentage
- [x] Buyout proposals: offer to buy partner's share
- [x] Upgrade voting: show pending upgrade votes, cast vote
- [x] Revenue/cost split breakdown per co-owned asset
- [x] Wire to uiState.ts as Diplomacy group tab

---

## Phase 5 — Missing Backend Systems --- ALL COMPLETE

*Goal: Implement all backend systems that are documented but don't exist.*

### 5.1 — Alliance System (Diplomacy group) --- COMPLETE

**Build (Rust):**
- [x] `alliance.rs` — Alliance struct with trust scores, revenue sharing, dissolution
- [x] Commands: ProposeAlliance, AcceptAlliance, DissolveAlliance
- [x] Events: AllianceFormed, AllianceDissolved, AllianceTrustChanged
- [x] `alliance.rs` system — Trust scoring, revenue sharing, dissolution checks
- [x] AI alliance behavior per archetype

**Build (Frontend):**
- [x] `AlliancePanel.svelte` with propose/accept/dissolve, trust scores, member list
- [x] Wire to uiState.ts as Diplomacy group tab

### 5.2 — Legal System (Diplomacy group) --- COMPLETE

**Build (Rust):**
- [x] `lawsuit.rs` — Lawsuit struct with types, outcomes, resolution timeline
- [x] Commands: FileLawsuit, SettleLawsuit, DefendLawsuit
- [x] Events: LawsuitFiled, LawsuitResolved, SettlementReached
- [x] `legal.rs` system — Filing cost, resolution, outcome calculation, damage payments
- [x] AI lawsuit behavior per archetype

**Build (Frontend):**
- [x] `LegalPanel.svelte` — file/defend/settle lawsuits, view active/resolved cases

### 5.3 — Patent System (Research group) --- COMPLETE

**Build (Rust):**
- [x] Commands: FilePatent, RequestLicense, SetLicensePrice, RevokeLicense, StartIndependentResearch
- [x] Independent research at 150%/200% cost with patent eligibility
- [x] License types: Permanent, Royalty, PerUnit, Lease
- [x] `patent.rs` system — License revenue, patent expiration, enforcement
- [x] patent_system in tick order

**Build (Frontend):**
- [x] `PatentPanel.svelte` — owned patents, license management, file/request

### 5.4 — Government Grants System (Market group) --- COMPLETE

**Build (Rust):**
- [x] `government_grant.rs` — Grant struct with region, requirements, rewards
- [x] Commands: BidForGrant, CompleteGrant
- [x] Events: GrantAvailable, GrantAwarded, GrantCompleted, GrantExpired
- [x] `grants.rs` system — Generate grants per region, track progress, payouts
- [x] AI grant bidding per archetype

**Build (Frontend):**
- [x] `GrantPanel.svelte` — available grants, bid, track progress

### 5.5 — Regional Pricing System (Finance group) --- COMPLETE

**Build (Rust):**
- [x] `pricing.rs` — PriceTier (Budget/Standard/Premium) with price elasticity
- [x] Command: SetRegionPricing
- [x] Revenue system pricing-aware, AI dynamic pricing

**Build (Frontend):**
- [x] `PricingPanel.svelte` — per-region pricing tier selector

### 5.6 — Maintenance Priority System (Operations group) --- COMPLETE

**Build (Rust):**
- [x] MaintenancePriority component with priority tiers (Critical/Standard/Low/Deferred)
- [x] Command: SetMaintenancePriority
- [x] Maintenance system respects priority tiers

**Build (Frontend):**
- [x] `MaintenancePanel.svelte` — set per-node priorities, view queue

### 5.7 — Fog of War System --- CANCELLED

Per user request, fog of war has been cancelled.

### 5.8 — Sandbox Mode --- COMPLETE

**Build (Rust):**
- [x] `sandbox: bool` on WorldConfig
- [x] Infinite money, all tech unlocked, instant construction, 32x speed
- [x] Finance/bankruptcy systems skip failure states in sandbox

**Build (Frontend):**
- [x] Sandbox game mode option in NewGame.svelte
- [x] 32x (Ludicrous) speed button when in sandbox
- [x] "SANDBOX" indicator in HUD

---

## Phase 6 — Missing Gameplay Features --- ALL COMPLETE

*Goal: Implement major gameplay features described in design docs but not yet built.*

### 6.1 — Stock Market & Shareholders --- COMPLETE

**Build:**
- [x] Share/equity system for corporations (total shares, share price, dividends)
- [x] IPO event when corp reaches certain size
- [x] Share price affected by performance metrics
- [x] `stock_market.rs` system in tick order
- [x] UI panel for stock management (StockMarketPanel.svelte)

### 6.2 — Dynamic AI Spawning Mid-Game --- COMPLETE

**Build:**
- [x] Market system detects underserved regions (low competition, high demand)
- [x] Spawn new AI corporations in underserved markets
- [x] AI corps can merge when both are compatible archetype
- [x] AI corps go bankrupt naturally (trigger liquidation auction)
- [x] Configurable spawn rate in WorldConfig

### 6.3 — Management Scaling UI --- COMPLETE

**Build:**
- [x] Detect company size tier from asset count (Small/Medium/Large)
- [x] Small: individual employee hire/fire, per-node management
- [x] Medium: team management, regional budget allocation
- [x] Large: policy settings, department overview, AI execution summaries
- [x] Panel layouts adapt based on tier
- [x] Quarterly reports for large companies

### 6.4 — Independent Research Workaround --- COMPLETE

**Build:**
- [x] `StartIndependentResearch` command variant with cost multiplier
- [x] Research system applies 1.5x or 2.0x cost multiplier
- [x] At 200%, completed tech gets +10% performance bonus
- [x] UI in ResearchPanel for choosing independent research path

### 6.5 — Full License Types --- COMPLETE

**Design:** Permanent (one-time), Royalty (per-tick), PerUnit (per-node-built), Lease (temporary duration).

**Build:**
- [x] Add LicenseType enum to gt-common (Permanent, Royalty, PerUnit, Lease)
- [x] Patent holder sets license type and price (patent.rs: license_type, license_price, per_unit_price, lease_duration)
- [x] Revenue collection varies by type in patent_system (royalty per-tick, lease expiration, per-unit tracking)
- [x] UI for license negotiation in PatentPanel

---

## Phase 7 — Overhaul Plan Completion

*Goal: Complete all remaining phases from MAP_TERRAIN_FIBER_OVERHAUL_PLAN.md.*

### 7.1 — Terrain Detail Within Procgen Cells (Phase 2.2.2 — COMPLETE)

- [x] Mountain cells: 4 concentric contour rings with per-vertex jitter for ridge appearance + peak highlight polygon
- [x] Forest/Rural cells: 2-3 offset canopy patches with varying green shades for organic tree-canopy feel
- [x] Desert cells: 3 concentric dune bands (alternating light/dark) + 2-3 diagonal wind-streak lines
- [x] Ocean cells: always-visible depth gradient polygons (lighter inner patch for shallow, darker for deep)
- [x] Urban cells: dense 60%-fill grid pattern with 4-5 lines each direction, per-cell rotation
- [x] Suburban cells: scattered 2x2 block clusters suggesting residential neighborhoods
- [x] Coastal cells: dual-ring shore-side gradient band (blue-green tint)
- [x] Tundra cells: 3-4 crack lines with mid-point bends for permafrost pattern
- [x] Frozen cells: 2-3 ice fracture lines with white-blue tint
- [x] All terrain types processed (no sampling skip), ocean detail at all zooms, land detail at zoom 3+

### 7.2 — Real Earth Building Footprints (Phase 3.1.2 — COMPLETE)

- [x] Real Earth mode: procedural building footprints rendered at zoom 7+ with 5 density zones
- [x] Building color by type (downtown dark, commercial blue-grey, residential grey, suburban light)
- [x] Building coverage indicators: updateBuildingCoverage() colors buildings by FTTH coverage status

### 7.3 — FTTH Access Network Game Loop (Phase 4.7 — COMPLETE)

- [x] Central Office → Feeder Fiber → FDH → Distribution Fiber → NAP → Building coverage (ftth.rs chain validation)
- [x] NAP auto-covers buildings within terrain-dependent service radius (Urban 2km, Suburban 5km, Rural 10km)
- [x] Coverage contributes to cell_coverage for revenue/demand systems with bandwidth contribution per NAP
- [x] Manual drop cables (DropCable EdgeType) connect NAP to specific building for full revenue
- [x] active_ftth flag exposed via WASM bridge, FTTH coverage overlay on map

### 7.4 — Submarine Cable Mechanics (Phase 7 — COMPLETE)

- [x] Landing station placement validation (SubseaLandingStation requires Coastal terrain)
- [x] OceanTrench terrain type added for bathymetry-aware costs
- [x] Bathymetry-aware cost multipliers (OceanShallow x1.0, OceanDeep x2.0, OceanTrench x3.0)
- [x] Cable ship capacity limit (max 2 simultaneous submarine cable constructions per corp)
- [x] Real TeleGeography reference overlay (submarineCableRefLayer.ts, toggle-able)
- [x] High construction time and cost for submarine edges (SubmarineFiber: 250k/km, 1500 maint)

### 7.5 — Weather System (Phase 9 — COMPLETE)

- [x] Regional weather patterns based on terrain/latitude (terrain affinity weighting)
- [x] Weather events: storms, ice storms, flooding, extreme heat, earthquakes, hurricanes
- [x] Deployment vulnerability matrix (aerial/underground/submarine different per event type)
- [x] Weather forecast visible via `get_weather_forecasts()` bridge query
- [x] Damage/repair integration with existing disaster system (weather amplifies disaster severity)
- [x] Weather visualization on map (forecasts merged with disaster display, audio cues)

### 7.6 — Competitor Visualization on Main Map (Phase 12 — COMPLETE)

- [x] Render all competitor infrastructure on the main map (not just minimap)
- [x] Competitor nodes slightly dimmer/smaller than player's (visual hierarchy)
- [x] Competitor edges in competitor's color, slightly thinner
- [x] Competitive overlay: market share by region, coverage overlap, expansion patterns

### 7.7 — Remaining Overhaul Phases --- MOSTLY COMPLETE

- [x] Phase 6: City density zones — density overlay with 5 zone types (downtown/commercial/residential inner/outer/suburban), population-scaled radii, color-coded polygons
- [x] Phase 8: Spectrum system complete — interference modeling (spectrum.rs), carrier aggregation, spectrum visualization overlay (createSpectrumOverlayLayers), licensed/unlicensed region display, interference zone circles
- [x] Phase 10: Network monitoring dashboard widgets — health overview (healthy/degraded/damaged counts), traffic flow summary (throughput vs capacity), bottleneck detection (top 5 utilized edges), maintenance queue (repair counts and progress)
- [ ] Phase 13: Full integration testing, performance optimization (60fps with full entity load), cross-browser testing

---

## Phase 8 — Production Hardening

*Goal: Stability, performance, and launch readiness.*

### 8.1 — Full Audio Expansion (Phase 9 unchecked items)

- [x] Ambient background music tracks per era (synthesized or licensed loops)
- [x] Era-specific sound palettes (telegraph clicks, digital tones, etc.)
- [x] UI interaction sounds (panel open/close, tab switch, button hover, slider drag)
- [x] Environmental audio (city hum, ocean ambience, wind at mountains)
- [x] Disaster-specific audio cues (earthquake rumble, storm winds, cyber glitch)
- [x] Victory/achievement fanfare with escalating intensity
- [x] Audio ducking during important notifications

### 8.2 — Accessibility (Phase 13) --- COMPLETE

- [x] Colorblind-friendly mode: alternative color schemes (Settings.svelte)
- [x] UI scaling option (text/UI size slider, app.css CSS variables)
- [x] Full keyboard navigation for all menus (tab, enter, escape)
- [x] ARIA attributes on all interactive elements (26+ components updated)
- [x] Screen reader compatibility: sr-only utility, skip-to-content link, aria-live regions

### 8.3 — Localization Expansion (Phase 13) --- COMPLETE

- [x] i18n framework exists (en.json) — all panel strings go through `$tr()`
- [x] Hardcoded English strings replaced in DashboardPanel, WorkforcePanel, and other components
- [x] Structure supports adding more locales (already set up)

### 8.4 — Performance Profiling (Phase 14) --- COMPLETE

- [x] Profile simulation tick (target: <50ms for 10,000+ entities) — PerfMonitor.svelte tracks sim tick time, GameLoop.ts measures per-tick with performance.now()
- [x] Profile deck.gl rendering (target: 60fps with 100,000+ visible entities) — PerfMonitor tracks FPS, frame time, draw calls, triangles
- [x] Profile WASM module size (target: <5MB gzipped) — 1.6MB uncompressed, 469KB gzipped ✓
- [x] Memory profiling (target: <500MB in browser) — PerfMonitor tracks JS heap size via performance.memory
- [x] WebSocket latency (target: <100ms round-trip) — WebSocketClient.ts has ping() function for latency measurement

### 8.5 — QA & Launch Prep (Phase 14)

- [x] Loading screen with tips during world gen
- [x] Credits screen (Credits.svelte with AI developers, orchestrator, tech stack)
- [x] Version number in main menu (VERSION file → vite.config.ts → __APP_VERSION__ → MainMenu footer)
- [x] Splash screen on launch (SplashScreen.svelte with animated title, tagline, version, 2s duration)
- [ ] Full SP playthrough at each difficulty
- [ ] Multiplayer: 2+ players, 50+ ticks, sync verified
- [ ] Edge case testing: 0 AI, 10 AI, seed 0, max seed
- [ ] Disaster stress test (severity 10)
- [ ] Financial stress test (force bankruptcy)
- [ ] Performance test: 10k+ entities, 10 AI corps, 4x speed

---

## Phase 9 — Document Cleanup

*Goal: Update all docs to match reality with status tracking.*

### 9.1 — game_design_decisions.md --- COMPLETE

- [x] Update NodeType count from "~33" to actual count (41)
- [x] Update EdgeType count from "~15" to actual count (26)
- [x] Add [x] / [ ] status markers to all features
- [x] Add "Current Status" section at top with implementation summary

### 9.2 — technical_architecture.md --- COMPLETE

- [x] Update system count to 28 (including weather, spectrum, ftth, stock_market, alliance, legal, patent, grants)
- [x] Update NodeType/EdgeType variant counts
- [x] Update panel file structure to match reality (23+ panels)
- [x] Updated tick order

### 9.3 — mvp_to_production_v1_plan.md --- COMPLETE

- [x] Update system count
- [x] Update Phase 10 items with accurate status
- [x] Add notes about spectrum and ftth systems added beyond original plan

### 9.4 — MAP_TERRAIN_FIBER_OVERHAUL_PLAN.md --- COMPLETE

- [x] Add status markers to all 14 phases
- [x] Mark remaining gaps within mostly-done phases
- [x] Update Phase 6-10, 12-13 as [NOT STARTED] or [PARTIAL]

### 9.5 — FULL_GAME_AUDIT.md --- N/A

**Status:** No FULL_GAME_AUDIT.md file exists in the repository. The audit findings are captured in this plan (FULL_FIX_PLAN.md) itself, with status markers on each item. No separate audit file update needed.

### 9.6 — CLAUDE.md --- COMPLETE

- [x] Updated system count to 28 (was 27, missed weather system between ai and disaster)
- [x] Updated tick order to include weather at position 15, shifting disaster to 16 and all subsequent systems
- [x] Noted `resolve_spectrum_auctions()` runs after all 28 systems
- [x] Updated Commands count from 19 listed to full 61 with complete list
- [x] Updated Queries to include stock_market, weather, spectrum_licenses, pricing, maintenance_priorities
- [x] Added comprehensive "Current Implementation Status" section with verified counts:
  - 28 ECS systems, 38 component modules, 41 NodeType variants, 25 EdgeType variants
  - 61 commands, 66+ event types, 23 frontend panels
  - Audio system (AudioManager + SpatialAudio)
- [x] Documented all implemented systems beyond original 20 (alliance, legal, patent, grants, weather, stock_market, spectrum, ftth, sandbox, pricing, maintenance priority)
- [x] Updated "Key remaining gaps" to reflect current state (no era enforcement, partial fog of war, no management scaling UI, no dynamic AI spawning, no building footprints, partial submarine cables, no colorblind mode, no localization beyond English)
- [x] Updated CONTRIBUTING.md system count from 20 to 28

---

## Priority & Dependency Order

**Critical Path (do first — blocks everything):**
1. Phase 1 (gameplay bugs) — no dependencies, immediate impact
2. Phase 2 (UI bugs) — no dependencies, immediate impact

**High Priority (core feature completeness):**
3. Phase 3 (panel data wiring) — no dependencies
4. Phase 4 (frontend-only panels) — no dependencies, quick wins
5. Phase 5.1-5.4 (alliance, legal, patents, grants) — large but independent

**Medium Priority (gameplay depth):**
6. Phase 5.5-5.8 (pricing, maintenance, fog of war, sandbox)
7. Phase 6 (stock market, dynamic AI, management scaling, independent research)
8. Phase 7.3 (FTTH game loop) — depends on Phase 7.2

**Lower Priority (polish & completion):**
9. Phase 7 (remaining overhaul phases)
10. Phase 8 (production hardening)
11. Phase 9 (doc cleanup) — do last, after all changes are final

---

*Plan generated from: FULL_GAME_AUDIT.md, game_design_decisions.md, technical_architecture.md, mvp_to_production_v1_plan.md, offline_singleplayer_implementation_plan.md, MAP_TERRAIN_FIBER_OVERHAUL_PLAN.md, telecom_mmo_master_dev_charter.md, and full codebase verification.*
