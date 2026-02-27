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
- [ ] `commandRouter.ts:50` — After `bridge.processCommand(command)`, dispatch `window.dispatchEvent(new CustomEvent('map-dirty'))` immediately
- [ ] `tick.rs:19` — Change `node_ids.map(|n| n.len() as u32)` to `Some(node_ids.map(|n| n.len()).unwrap_or(0) as u32)` so node_count is always `Some`
- [ ] `GameLoop.ts` — On `CommandAck` for BuildNode, immediately query `getCorporationData()` to refresh counter
- [ ] Verify LOD filtering doesn't hide newly-placed nodes at low zoom (infraLayer.ts:527-528 tier culling)

### 1.2 — Multiplayer Chat Not Rendering (Audit #2)

**Root Cause:** Server handler `ws.rs:1209-1249` has 3 nested `if let` checks that silently drop messages with no error response. Client clears input optimistically before server confirms.

**Fixes:**
- [ ] `ws.rs:1209-1249` — Add explicit error responses for all failure paths (player is None, world_id is None, world lookup fails)
- [ ] `ws.rs:1209` — Add early player auth check (consistent with other handlers)
- [ ] `Chat.svelte:15` — Don't clear input until `ChatBroadcast` is received back, or implement optimistic display (add message locally, mark as pending, remove on timeout)
- [ ] Add lobby system messages (player joined, world started, etc.) per user requirement
- [ ] Test both in-game chat and lobby context

### 1.3 — Radial Build Menu Can't Select Items (Audit #3)

**Root Cause:** Mouseleave race condition. Moving mouse from SVG segment to flyout triggers segment's `onmouseleave` (line 253) which sets `hoveredCategory = null`, hiding the flyout before its `onmouseenter` (line 290) can fire. The 20px gap between segment and flyout (`RADIUS_OUTER + 20`) makes this worse.

**Fixes:**
- [ ] `RadialBuildMenu.svelte:253` — Add a 150ms delay before closing flyout on mouseleave (use `setTimeout`, cancel on flyout `onmouseenter`)
- [ ] Add `onmouseleave` handler on the flyout div (line 287) to also set hoveredCategory = null (currently missing)
- [ ] Reduce gap between segment and flyout, or add an invisible bridge element connecting them
- [ ] Verify backdrop click handler (line 152) doesn't intercept flyout item clicks
- [ ] Test edge placement mode (line 135 enters edge mode without closing menu)

### 1.4 — Build Tool Stuck on Insufficient Funds (Audit tech debt #3)

**Root Cause:** `BuildHotbar.svelte:113-124` — `enterPlacementMode()` has no affordability check. `MapView.svelte:110-118` sends BuildNode without pre-flight validation. On failure, placement mode stays active.

**Fixes:**
- [ ] `BuildHotbar.svelte:activateSlot()` — Before `enterPlacementMode()`, check `$playerCorp.cash >= minimumCost` for that node type
- [ ] Grey out unaffordable hotbar slots with visual disabled state
- [ ] Show "Insufficient funds" toast when clicking a greyed-out slot
- [ ] `RadialBuildMenu.svelte:selectItem()` — Same affordability check (line 126 already checks, verify it works)
- [ ] On command failure notification, auto-exit placement mode

---

## Phase 2 — UI/UX Bugs & Polish

*Goal: Fix all non-gameplay UI issues from the audit.*

### 2.1 — Favicon (Audit #4a)

**Current State:** SVG favicon linked via `<link>` tag, but no `favicon.ico` in `static/`. Browsers request `/favicon.ico` directly.

**Fixes:**
- [ ] Generate full favicon set from existing `favicon.svg`: 16x16, 32x32, 180x180 (apple-touch-icon), 192x192, 512x512
- [ ] Add `favicon.ico` (multi-size ICO) to `web/static/`
- [ ] Add PNG variants to `web/static/`
- [ ] Update `Layout.svelte` with full favicon `<link>` tags (ico, apple-touch-icon, sizes)
- [ ] Update `manifest.json` with icon sizes

### 2.2 — Password Fields Not in `<form>` (Audit #4b)

**Current State:** `WorldBrowser.svelte:174-221` — password inputs are bare `<div>` wrappers, not in `<form>` elements.

**Fixes:**
- [ ] `WorldBrowser.svelte` — Wrap auth inputs (username + password) in `<form>` element with `on:submit|preventDefault={connectAndAuth}`
- [ ] Add `autocomplete` attributes (`username`, `current-password`, `new-password`) to inputs
- [ ] Verify Enter key submits the form
- [ ] Check `admin/+page.svelte` for same issue and fix

### 2.3 — Panel Click Blocking (Audit UI #3)

**Current State:** `FloatingPanel.svelte` has no `pointer-events` CSS handling. Panels overlay the map and intercept clicks.

**Fixes:**
- [ ] When clicking outside a FloatingPanel (on the map area), close the panel AND pass the click through to the map
- [ ] Add `on:click|self` handler on the map container that closes any open floating panel
- [ ] Ensure panels don't capture clicks on transparent/empty areas

### 2.4 — Overlay Text Readability (Audit tech debt #2)

**Current State:** deck.gl TextLayer labels in `labelLayers.ts` use plain white text with no halo/shadow. Unreadable on bright satellite imagery.

**Fixes:**
- [ ] `labelLayers.ts` — Add text outline/halo to city labels (use `getTextAnchor`, `fontSettings` with `sdf: true` and `buffer` for halo effect, or switch to HTML overlay)
- [ ] `labelLayers.ts` — Add text outline to region labels
- [ ] Increase contrast on NotificationFeed and OverlayLegend text when in Real Earth mode

### 2.5 — Advisor Click-to-Pan (Audit tech debt #1)

**Current State:** `AdvisorPanel.svelte:84-93` — suggestion divs have no click handler. MapView already supports `map-fly-to` events.

**Fixes:**
- [ ] Add `onclick` handler to each `.suggestion` div in AdvisorPanel
- [ ] For spatial alerts (damaged node, unmet demand): dispatch `map-fly-to` with the relevant entity's coordinates
- [ ] For system alerts (low funds, no research): open the relevant management panel (Dashboard for finance, Research for R&D)
- [ ] Add cursor pointer and hover state to suggestion divs
- [ ] Map suggestion types to coordinates using bridge methods (e.g., `bridge.getDamagedNodes()` for damaged infrastructure)

### 2.6 — AI Node Color Hardening (Audit UI #1)

**Current State:** No magenta (#FF00FF) found in code. CORP_COLORS has 8 colors with grey fallback. Icon atlas has grey (#666) fallback. Magenta may be a GPU/driver artifact or uninitialized WebGL state.

**Fixes:**
- [ ] `iconAtlas.ts` — Add explicit error logging when SVG icon fails to load
- [ ] `infraLayer.ts:getCorpColor()` — Add validation that returned color has valid RGB values (not all zeros, not uninitialized)
- [ ] `infraLayer.ts` ScatterplotLayer fallback (line 788-806) — Ensure `getFillColor` always returns valid RGBA
- [ ] Add a "missing icon" placeholder that's clearly styled (not magenta) to the icon atlas

### 2.7 — Disaster Severity Slider

**Current State:** `WorldConfig.disaster_frequency` is an `f64` but only settable via DifficultyPreset (Easy/Normal/Hard/Expert). No standalone slider in NewGame UI.

**Fixes:**
- [ ] Add disaster severity slider (1-10) to `NewGame.svelte` in the game setup section
- [ ] Map slider value to `disaster_frequency` (1=0.1, 5=1.0, 10=3.0 or similar curve)
- [ ] Show current value label next to slider
- [ ] Default to difficulty preset value, allow override

---

## Phase 3 — Panel Fixes & Data Wiring

*Goal: Fix broken data and placeholder content in existing panels.*

### 3.1 — DashboardPanel Budget/Policy Placeholders

**Current State:** Budget & policy sliders use hardcoded values, not synced from backend.

**Fixes:**
- [ ] Wire budget sliders to actual `SetBudget` command responses
- [ ] Query current budget state from WASM bridge on panel open
- [ ] Show loading state while budget data is fetched

### 3.2 — NetworkDashboard Estimated Data (Gaps #19a-d, #28) --- COMPLETE

**Current State:** All 4 gaps wired with real simulation data. No remaining placeholder formulas.

**Fixes:**
- [x] **#19a — Per-infrastructure revenue:** Added `revenue_generated` field to InfraNode and InfraEdge components. Revenue system now tracks per-asset revenue. Exposed via bridge. Edge `maintenance_cost` also now exposed.
- [x] **#19b — SLA monitoring:** Added `sla_target`, `sla_current_performance`, `sla_penalty_accrued` fields to Contract component. Contract system now calculates SLA compliance per-tick based on provider health and capacity. Exposed via bridge with `sla_status` field.
- [x] **#19c — Repair state:** Exposed `repairing`, `repair_ticks_left`, `repair_health_per_tick` fields through the bridge's infrastructure query for both nodes and edges. Template now shows remaining repair ticks.
- [x] **#19d + #28 — Capacity planning:** Added `utilization_history` (HashMap<EntityId, VecDeque<f64>>) to GameWorld. Utilization system records last 100 ticks per node/edge. Exposed via bridge as `utilization_history: Vec<f64>`. `ticksUntilFull` now uses per-edge linear regression from real history.
- [x] All placeholder formulas and TODO markers replaced in NetworkDashboard.svelte

### 3.3 — WorkforcePanel Missing Field

**Current State:** References `$playerCorp?.infrastructure_count` which doesn't exist on the corporation type.

**Fixes:**
- [ ] Add `infrastructure_count` to CorporationData returned by bridge
- [ ] Or compute from existing `corp_infra_nodes` length in the query

### 3.4 — ContractPanel Hardcoded Terms

**Current State:** Propose form uses hardcoded string `'bandwidth:1000,price:5000,duration:100'` instead of structured UI.

**Fixes:**
- [ ] Replace string input with structured form fields: bandwidth slider, price input, duration input
- [ ] Validate inputs against player's network capacity and cash
- [ ] Show preview of contract terms before proposing

---

## Phase 4 — Coming Soon Panels (Frontend Only)

*Goal: Build UI panels for features that have backend support but no frontend.*

### 4.1 — Insurance Panel (Finance group)

**Backend Status:** READY — `PurchaseInsurance`, `CancelInsurance` commands, premium calculation in cost.rs, payout in disaster.rs.

**Build:**
- [ ] Create `InsurancePanel.svelte` in `web/src/lib/panels/`
- [ ] Show all owned infrastructure with insured/uninsured status
- [ ] Per-node toggle to purchase/cancel insurance
- [ ] Show premium cost per node (2% of construction cost)
- [ ] Show payout history (60% of damage cost on disaster)
- [ ] Summary: total premium cost/tick, total insured vs uninsured assets
- [ ] Wire to uiState.ts as Finance group tab

### 4.2 — Repair Panel (Operations group)

**Backend Status:** READY — `RepairNode`, `RepairEdge`, `EmergencyRepair` commands, repair tracking fields on InfraNode.

**Build:**
- [ ] Create `RepairPanel.svelte` in `web/src/lib/panels/`
- [ ] List all damaged infrastructure (health < 100%) sorted by severity
- [ ] Show repair cost estimate and time remaining for each
- [ ] Buttons: Standard Repair, Emergency Repair per node
- [ ] Show maintenance crew count and repair speed multiplier
- [ ] Repair progress bars for nodes currently being repaired
- [ ] Wire to uiState.ts as Operations group tab

### 4.3 — Co-ownership Panel (Diplomacy group)

**Backend Status:** READY — `ProposeCoOwnership`, `RespondCoOwnership`, `ProposeBuyout`, `VoteUpgrade` commands, co_ownership_proposals tracking.

**Build:**
- [ ] Create `CoOwnershipPanel.svelte` in `web/src/lib/panels/`
- [ ] List all co-owned infrastructure with ownership percentages
- [ ] Incoming/outgoing co-ownership proposals with accept/reject
- [ ] Propose co-ownership: select node, target corp, share percentage
- [ ] Buyout proposals: offer to buy partner's share
- [ ] Upgrade voting: show pending upgrade votes, cast vote
- [ ] Revenue/cost split breakdown per co-owned asset
- [ ] Wire to uiState.ts as Diplomacy group tab

---

## Phase 5 — Missing Backend Systems

*Goal: Implement all backend systems that are documented but don't exist.*

### 5.1 — Alliance System (Diplomacy group)

**Backend:** NOTHING EXISTS — no components, commands, events, or system.

**Build (Rust):**
- [ ] `crates/gt-simulation/src/components/alliance.rs` — Alliance struct (id, name, member_corp_ids max 3, trust_scores, revenue_share_pct, formed_tick)
- [ ] `crates/gt-common/src/commands.rs` — Add: ProposeAlliance, AcceptAlliance, DissolveAlliance, AllianceVote
- [ ] `crates/gt-common/src/events.rs` — Add: AllianceFormed, AllianceDissolved, AllianceTrustChanged
- [ ] `crates/gt-simulation/src/systems/alliance.rs` — Trust scoring, revenue sharing, dissolution checks (trust < threshold), 30-tick dissolution transition
- [ ] Benefits: free routing between allies, 50% license cost, shared basic intel, mutual defense notifications
- [ ] Add alliance_system to tick order in `systems/mod.rs`
- [ ] AI alliance behavior per archetype

**Build (Frontend):**
- [ ] Create `AlliancePanel.svelte` — propose/accept/dissolve alliances, trust score display, revenue sharing config, member list
- [ ] Wire to uiState.ts as Diplomacy group tab
- [ ] Add bridge queries: `getAlliances()`, `getAllianceProposals()`

### 5.2 — Legal System (Diplomacy group)

**Backend:** NOTHING EXISTS.

**Build (Rust):**
- [ ] `crates/gt-simulation/src/components/lawsuit.rs` — Lawsuit struct (id, plaintiff, defendant, type, damages_claimed, filed_tick, resolution_tick, status, outcome)
- [ ] LawsuitType enum: SabotageClaim, OwnershipDispute, PatentInfringement, RegulatoryComplaint
- [ ] LawsuitOutcome enum: DamagesAwarded, ForcedLicensing, AssetForfeiture, RegulatoryFine, Dismissed
- [ ] `crates/gt-common/src/commands.rs` — Add: FileLawsuit, SettleLawsuit, DefendLawsuit
- [ ] `crates/gt-common/src/events.rs` — Add: LawsuitFiled, LawsuitResolved, SettlementReached
- [ ] `crates/gt-simulation/src/systems/legal.rs` — Filing cost, resolution over 20-50 ticks, outcome calculation, damage payments
- [ ] Requires Legal team on staff (workforce check)
- [ ] Add legal_system to tick order
- [ ] AI lawsuit behavior per archetype

**Build (Frontend):**
- [ ] Create `LegalPanel.svelte` — file lawsuits, defend, settle, view active/resolved cases
- [ ] Wire to uiState.ts as Diplomacy group tab

### 5.3 — Patent System (Research group)

**Backend:** PARTIAL — PatentStatus enum exists on TechResearch, but no separate Patent entity, no licensing commands, no independent research, no patent enforcement system.

**Build (Rust):**
- [ ] `crates/gt-common/src/commands.rs` — Add: FilePatent, RequestLicense, SetLicensePrice, RevokeLicense, StartIndependentResearch
- [ ] Implement independent research at 150% cost (standard access) and 200% cost (improved version, can patent)
- [ ] Implement license types: Permanent (one-time), Royalty (per-tick), PerUnit (per-node-built), Lease (temporary)
- [ ] `crates/gt-simulation/src/systems/patent.rs` — License revenue collection per tick, patent expiration, enforcement checks
- [ ] Hard block enforcement: reject BuildNode if node type requires patented tech and corp has no license
- [ ] Add patent_system to tick order

**Build (Frontend):**
- [ ] Create `PatentPanel.svelte` — owned patents, license management, file new patents, request licenses, set pricing
- [ ] Wire to uiState.ts as Research group tab

### 5.4 — Government Grants System (Market group)

**Backend:** NOTHING EXISTS.

**Build (Rust):**
- [ ] `crates/gt-simulation/src/components/government_grant.rs` — Grant struct (id, region_id, requirements, reward_cash, tax_break, deadline_tick, progress, awarded_corp)
- [ ] `crates/gt-common/src/commands.rs` — Add: BidForGrant, CompleteGrant
- [ ] `crates/gt-common/src/events.rs` — Add: GrantAvailable, GrantAwarded, GrantCompleted, GrantExpired
- [ ] `crates/gt-simulation/src/systems/grants.rs` — Generate grants per region (underserved area incentives), track progress, process completion payouts
- [ ] AI grant bidding per archetype/strategy
- [ ] Add grants_system to tick order

**Build (Frontend):**
- [ ] Create `GrantPanel.svelte` — available grants, bid, track progress, completed history
- [ ] Wire to uiState.ts as Market group tab

### 5.5 — Regional Pricing System (Finance group)

**Backend:** NOTHING EXISTS (one line mention in market.rs).

**Build (Rust):**
- [ ] `crates/gt-simulation/src/components/pricing.rs` — PriceTier struct (region_id, corp_id, tier: Budget/Standard/Premium/Custom, price_per_unit)
- [ ] `crates/gt-common/src/commands.rs` — Add: SetRegionPricing
- [ ] Price elasticity: wealthy regions tolerate premium, poor regions are price-sensitive
- [ ] Pricing affects: customer acquisition rate, revenue per customer, churn rate, AI competitor response
- [ ] Integrate into revenue_system (replace flat calculation with pricing-aware)
- [ ] AI dynamic pricing per archetype

**Build (Frontend):**
- [ ] Create `PricingPanel.svelte` — per-region pricing tier selector, revenue impact preview, competitor pricing comparison
- [ ] Wire to uiState.ts as Finance group tab

### 5.6 — Maintenance Priority System (Operations group)

**Backend:** PARTIAL — maintenance.rs handles repairs but no priority tiers, no auto-repair toggle, no scheduling.

**Build (Rust):**
- [ ] Add MaintenancePriority component: entity_id, priority_tier (Critical/Standard/Low/Deferred), auto_repair bool
- [ ] `crates/gt-common/src/commands.rs` — Add: SetMaintenancePriority
- [ ] Update maintenance_system to respect priority tiers (Critical = immediate, Standard = scheduled, Low = when resources available, Deferred = no maintenance)
- [ ] Per-node maintenance budget allocation
- [ ] Maintenance teams amplify repair effectiveness

**Build (Frontend):**
- [ ] Create `MaintenancePanel.svelte` — set per-node priorities, view maintenance queue, budget allocation, crew status
- [ ] Wire to uiState.ts as Operations group tab

### 5.7 — Fog of War System

**Backend:** PARTIAL — espionage/sabotage missions exist via covert_ops, but no intel level tiers or decay.

**Build (Rust):**
- [ ] `crates/gt-simulation/src/components/intel.rs` — IntelLevel struct (target_corp, observer_corp, level: None/Basic/Full, last_updated_tick)
- [ ] Intel decay: all intel decays over 50 ticks unless refreshed
- [ ] Espionage missions set intel level (Basic or Full based on mission type)
- [ ] Alliance members automatically share Basic intel
- [ ] Server-side filtering: filter TickUpdate/CommandBroadcast/Snapshot per client based on intel levels

**Build (Frontend):**
- [ ] Competitor infrastructure gated by intel level (None/Basic = locations only, Full = capacity/revenue/strategy)

### 5.8 — Sandbox Mode

**Backend:** NOTHING EXISTS.

**Build (Rust):**
- [ ] Add `sandbox: bool` to WorldConfig
- [ ] Sandbox features: infinite money (skip finance checks), all tech unlocked, instant construction (0-tick build), 32x speed option
- [ ] Finance/bankruptcy systems check `sandbox` flag and skip failure states
- [ ] All management panels available, all infrastructure types available

**Build (Frontend):**
- [ ] Add Sandbox as game mode option in `NewGame.svelte`
- [ ] Add 32x speed button to `SpeedControls.svelte` when in sandbox
- [ ] Show "SANDBOX" indicator in HUD

---

## Phase 6 — Missing Gameplay Features

*Goal: Implement major gameplay features described in design docs but not yet built.*

### 6.1 — Stock Market & Shareholders

**Build:**
- [ ] Share/equity system for corporations (total shares, share price, dividends)
- [ ] Board of directors voting mechanics
- [ ] IPO event when corp reaches certain size
- [ ] Share price affected by performance metrics
- [ ] Shareholder satisfaction affecting governance decisions
- [ ] UI panel for stock management

### 6.2 — Dynamic AI Spawning Mid-Game

**Current State:** AI corps only created at game start via `create_corporations()`.

**Build:**
- [ ] Market system detects underserved regions (low competition, high demand)
- [ ] Spawn new AI corporations in underserved markets (with appropriate archetype and starting capital)
- [ ] AI corps can merge when both are Defensive/compatible archetype
- [ ] AI corps go bankrupt naturally (trigger liquidation auction)
- [ ] Configurable spawn rate in WorldConfig

### 6.3 — Management Scaling UI

**Design:** Small company (1-10 assets) = hands-on, Medium (10-100) = teams/budgets, Large (100+) = policies/departments.

**Build:**
- [ ] Detect company size tier from asset count
- [ ] Small: show individual employee hire/fire, per-node management
- [ ] Medium: show team management, regional budget allocation
- [ ] Large: show policy settings, department overview, AI execution summaries
- [ ] Panel layouts adapt based on tier (more aggregate data for larger companies)
- [ ] Quarterly reports for large companies

### 6.4 — Independent Research Workaround

**Design:** 150% cost = base access (can build, can't patent). 200% cost = improved version (+10% bonus, CAN patent).

**Build:**
- [ ] Add `StartIndependentResearch` command variant with cost multiplier
- [ ] Research system applies 1.5x or 2.0x cost multiplier
- [ ] At 200%, completed tech gets +10% performance bonus
- [ ] UI in ResearchPanel for choosing independent research path when tech is patented

### 6.5 — Full License Types

**Design:** Permanent (one-time), Royalty (per-tick), PerUnit (per-node-built), Lease (temporary duration).

**Build:**
- [ ] Add LicenseType enum to gt-common
- [ ] Patent holder sets license type and price
- [ ] Revenue collection varies by type in patent_system
- [ ] UI for license negotiation in PatentPanel

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

### 7.2 — Real Earth Building Footprints (Phase 3.1.2 — PARTIAL)

- [ ] Real Earth mode: render OpenFreeMap building footprints as visible map layer at zoom 7+
- [ ] Building color by type (residential grey, commercial blue-grey, industrial brown-grey)
- [ ] Currently loaded but not rendered — need to add visible layer configuration

### 7.3 — FTTH Access Network Game Loop (Phase 4.7 — NOT STARTED)

- [ ] Central Office → Feeder Fiber → FDH → Distribution Fiber → NAP → Building coverage
- [ ] NAP auto-covers buildings within service radius (revenue with overhead deduction)
- [ ] Manual drop cables from NAP to building (full revenue, no overhead)
- [ ] Tiered management: small = manual NAP placement, medium = auto-managed drops, large = policy-driven deployment
- [ ] Building-level demand model (replace abstract coverage-per-cell with subscribers-per-building)

### 7.4 — Submarine Cable Mechanics (Phase 7 — PARTIAL)

- [ ] Landing station placement on coastal cells
- [ ] Waypoint-based ocean routing
- [ ] Bathymetry-aware cost (shelf x1.0, deep x2.0, trench x3.0)
- [ ] Cable ship construction mechanic (one cable per ship at a time)
- [ ] Real TeleGeography reference overlay (toggle-able)
- [ ] Very high construction time and cost

### 7.5 — Weather System (Phase 9 — PARTIAL)

- [ ] Regional weather patterns based on terrain/latitude
- [ ] Weather events: storms, ice storms, flooding, extreme heat, earthquakes
- [ ] Deployment vulnerability matrix (aerial/underground/submarine different per event type)
- [ ] Weather forecast visible 5-10 ticks ahead
- [ ] Damage/repair integration with existing disaster system
- [ ] Weather visualization on map (storm icons, affected areas)

### 7.6 — Competitor Visualization on Main Map (Phase 12 — PARTIAL)

- [ ] Render all competitor infrastructure on the main map (not just minimap)
- [ ] Competitor nodes slightly dimmer/smaller than player's (visual hierarchy)
- [ ] Competitor edges in competitor's color, slightly thinner
- [ ] Competitive overlay: market share by region, coverage overlap, expansion patterns

### 7.7 — Remaining Overhaul Phases

- [ ] Phase 6: City density zones (downtown/commercial/residential/suburban) with building-level demand
- [ ] Phase 8: Spectrum frequency assignment per wireless node, interference modeling, spectrum visualization overlay
- [ ] Phase 10: Full network monitoring dashboard widgets (health overview, traffic flow, bottleneck detection, revenue by infra, SLA monitoring, coverage map, maintenance queue, capacity planning)
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

### 8.2 — Accessibility (Phase 13)

- [ ] Colorblind-friendly mode: alternative color schemes for overlays and corp colors
- [ ] UI scaling option (text/UI size slider)
- [ ] Full keyboard navigation for all menus (tab, enter, escape)
- [ ] ARIA attributes on all interactive elements
- [ ] Screen reader compatibility for key information

### 8.3 — Localization Expansion (Phase 13)

- [ ] i18n framework exists (en.json) — verify all strings go through `$tr()`
- [ ] Ensure no hardcoded English strings remain in components
- [ ] Structure supports adding more locales (already set up)

### 8.4 — Performance Profiling (Phase 14)

- [ ] Profile simulation tick (target: <50ms for 10,000+ entities)
- [ ] Profile deck.gl rendering (target: 60fps with 100,000+ visible entities)
- [ ] Profile WASM module size (target: <5MB gzipped)
- [ ] Memory profiling (target: <500MB in browser)
- [ ] WebSocket latency (target: <100ms round-trip)

### 8.5 — QA & Launch Prep (Phase 14)

- [ ] Loading screen with tips during world gen
- [ ] Credits screen
- [ ] Version number in main menu
- [ ] Splash screen on launch
- [ ] Full SP playthrough at each difficulty
- [ ] Multiplayer: 2+ players, 50+ ticks, sync verified
- [ ] Edge case testing: 0 AI, 10 AI, seed 0, max seed
- [ ] Disaster stress test (severity 10)
- [ ] Financial stress test (force bankruptcy)
- [ ] Performance test: 10k+ entities, 10 AI corps, 4x speed

---

## Phase 9 — Document Cleanup

*Goal: Update all docs to match reality with status tracking.*

### 9.1 — game_design_decisions.md

- [ ] Update NodeType count from "~33" to actual count (41)
- [ ] Update EdgeType count from "~15" to actual count (25)
- [ ] Add [x] / [ ] status markers to all features
- [ ] Mark implemented: world/map, eras, build menu, contracts, mergers, bankruptcy, patents (basic), AI archetypes, disasters, co-ownership, spectrum, espionage, lobbying
- [ ] Mark not implemented: alliance, legal, fog of war (full), pricing, management scaling, sandbox, stock market, dynamic AI spawning, independent research
- [ ] Add "Current Status" section at top with implementation summary

### 9.2 — technical_architecture.md

- [ ] Update system count from 20+4 to actual 23 (add spectrum, ftth; note missing patent/alliance/legal/grants)
- [ ] Update NodeType/EdgeType variant counts
- [ ] Update panel file structure to match reality (13 panels, not the directory-organized layout described)
- [ ] Update command list (actual 45 commands vs described 24)
- [ ] Update SVG asset counts (35 actual vs 24 described)
- [ ] Add new components not in doc: spectrum, road_graph, building, ftth, road_graph
- [ ] Note panels that exist in code but not in doc: SpectrumPanel, NetworkDashboard

### 9.3 — mvp_to_production_v1_plan.md

- [ ] Update system count in Phase 1 to 23
- [ ] Update all Phase 10 unchecked items with accurate status
- [ ] Mark alliance/legal/grants/fog_of_war/pricing/sandbox as still [ ] unchecked
- [ ] Update test count from "target ~120-150" to actual (44 currently)
- [ ] Add notes about spectrum and ftth systems added beyond original plan

### 9.4 — MAP_TERRAIN_FIBER_OVERHAUL_PLAN.md

- [ ] Add status markers to all phases: Phase 0 [DONE], Phase 1 [DONE], Phase 2 [MOSTLY DONE], Phase 3 [MOSTLY DONE], Phase 4 [DONE], Phase 5 [DONE], Phase 11 [DONE]
- [ ] Mark remaining gaps within mostly-done phases
- [ ] Update Phase 6-10, 12-13 as [NOT STARTED] or [PARTIAL]

### 9.5 — FULL_GAME_AUDIT.md

- [ ] Update with fix status as issues are resolved
- [ ] Add verification notes for each issue

### 9.6 — CLAUDE.md

- [ ] Update system count to 23
- [ ] Update NodeType/EdgeType counts
- [ ] Update crate list to include all 11 crates
- [ ] Note spectrum and ftth systems in tick order
- [ ] Update "Key gaps" section to reflect current state

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
