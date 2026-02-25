# Map, Terrain & Fiber Planning Overhaul — Comprehensive Implementation Plan

> **Scope:** Full end-to-end overhaul of map quality, terrain systems, city visualization, infrastructure hierarchy, build UX, fiber routing, and supporting systems for both Real Earth and Procgen modes.
>
> **Philosophy:** What to do, not how to do it. Each item describes the desired outcome and behavior.
>
> **Date:** 2026-02-25

---

## Table of Contents

1. [Design Decisions (Locked In)](#1-design-decisions-locked-in)
2. [Phase 0 — Foundation Fixes](#2-phase-0--foundation-fixes)
3. [Phase 1 — Build UX Revolution](#3-phase-1--build-ux-revolution)
4. [Phase 2 — Terrain Quality Overhaul](#4-phase-2--terrain-quality-overhaul)
5. [Phase 3 — Road Networks & Urban Fabric](#5-phase-3--road-networks--urban-fabric)
6. [Phase 4 — Full Infrastructure Hierarchy (All 6 Eras)](#6-phase-4--full-infrastructure-hierarchy-all-6-eras)
7. [Phase 5 — Spline-Based Fiber Routing & Waypoint System](#7-phase-5--spline-based-fiber-routing--waypoint-system)
8. [Phase 6 — City Density, Buildings & Demand](#8-phase-6--city-density-buildings--demand)
9. [Phase 7 — Submarine Cable System](#9-phase-7--submarine-cable-system)
10. [Phase 8 — Spectrum & Frequency Management](#10-phase-8--spectrum--frequency-management)
11. [Phase 9 — Weather, Disasters & Cable Vulnerability](#11-phase-9--weather-disasters--cable-vulnerability)
12. [Phase 10 — Network Monitoring Dashboard](#12-phase-10--network-monitoring-dashboard)
13. [Phase 11 — Minimap & Navigation](#13-phase-11--minimap--navigation)
14. [Phase 12 — Competitor Visualization](#14-phase-12--competitor-visualization)
15. [Phase 13 — Polish, Performance & Integration Testing](#15-phase-13--polish-performance--integration-testing)
16. [Cross-Cutting Concerns](#16-cross-cutting-concerns)
17. [Dependency Graph](#17-dependency-graph)

---

## 1. Design Decisions (Locked In)

These decisions are final and apply across all phases:

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Map mode priority | Equal investment in Real Earth + Procgen | Both must be excellent |
| Build UX | Radial/pie menu (right-click) + bottom hotbar (pinnable, 1-9 keys) | Fast placement, minimal clutter, discoverable |
| FTTH granularity | Neighborhood-level + tiered management. Manual drops cheaper than NAP auto-coverage | Avoids tedium at scale, rewards micro-management |
| Real Earth data source | Actual OSM roads + Overture/Microsoft building footprints via vector tiles | Authentic city layouts |
| Fiber deployment | Aerial vs. underground per-edge decision (different cost, vulnerability, visuals). No conduit/duct reuse system | Meaningful cost decision without over-complexity |
| Era gating | None. All items visible in build menu always. Research unlocks capabilities but nothing is locked | Matches existing design: freely explorable research tree |
| Zoom behavior | Seamless/continuous progressive detail. No hard transition between strategy and planning views | Natural, immersive |
| Cable rendering | Hybrid: glow lines at low zoom (strategic), road-hugging realistic lines at high zoom (planning) | Best of both worlds |
| Cable geometry | Catmull-Rom splines through waypoints. Smooth curves, no sharp corners | Real, professional look |
| Cable capacity visual | Line thickness = strand count/capacity. Feeder thick, distribution medium, drop thin | At-a-glance capacity reading |
| Submarine cables | Full system with landing stations, bathymetry-aware routing, real TeleGeography reference data | Complete global network gameplay |
| Competitor visibility | Always visible. All competitor infrastructure shown on map at all times | Competitive awareness, strategic play |
| Fiber routing | Both manual waypoints AND auto-route along roads. Click-and-drag to adjust. Default auto-routes | Flexible, efficient, real GIS feel |
| Node type count | All ~33 node types across all 6 eras | Complete gameplay across eras |
| City layouts (Procgen) | Varied: grid (American), radial (European), organic (Asian/old-world), mixed | Visual diversity, cultural flavor |
| Minimap | Corner minimap showing world overview, infrastructure coverage, current viewport | Standard strategy game navigation |
| Additional systems | Spectrum management, weather/disaster cable effects, network monitoring dashboard | Full telecom simulation depth |

---

## 2. Phase 0 — Foundation Fixes

**Goal:** Fix remaining bugs that would interfere with new systems.

### 0.1 — City Water Placement Fix
- The `place_cities` function in world generation must validate that candidate cells are land before placement
- For Real Earth mode: cities snapped from `earth.json` to the nearest grid cell must snap to the nearest **land** cell, not just the nearest cell
- Add a hard `terrain.is_land()` filter on all city candidate cells before scoring
- No city should ever appear in ocean/water terrain

### 0.2 — Verify Region Boundary Rendering
- Confirm that region boundary polygons render correctly in both Real Earth and Procgen modes
- Confirm overlays are visible (opacity values are appropriate)
- If any rendering artifacts remain from the AUDIT issues, fix them before building new layers on top

### 0.3 — Stabilize WASM Bridge for New Entity Types
- The bridge (gt-bridge, gt-wasm) must be prepared to handle the many new entity types coming in Phase 4
- Ensure the typed array system can accommodate new node types, edge types, and metadata fields
- Verify that the delta sync system (CommandBroadcast, DeltaOps) can handle new entity creation types

**Exit criteria:** Cities never appear in water. Region boundaries render cleanly. Bridge is extensible.

---

## 3. Phase 1 — Build UX Revolution

**Goal:** Replace the current "click location → pick item" flow with a professional tool-palette build system.

### 1.1 — Radial/Pie Build Menu
- Right-clicking anywhere on the map opens a radial/pie menu
- Top-level categories arranged radially: **Backbone**, **Distribution**, **Access**, **Cables**, **Wireless**, **Infrastructure**, **Tools**
- Hovering a category expands its sub-items in a second ring or flyout
- Each item shows: icon, name, cost (terrain-adjusted for cursor location), and research status
- Clicking an item enters **placement mode** for that item
- Menu dismisses on item selection or clicking away
- ESC cancels the menu
- The menu is context-aware: at high zoom it emphasizes Access/Distribution items, at low zoom it emphasizes Backbone items

### 1.2 — Placement Mode (Point Items)
- After selecting a point item (node), the cursor changes to a placement cursor showing the item icon
- A ghost/preview of the item follows the cursor on the map
- Valid placement locations are indicated (green highlight on valid terrain, red on invalid)
- Terrain cost multiplier shown near cursor in real-time
- Left-click places the item
- Placement mode persists after placement (can keep placing same item type) — click again to place another
- Right-click or ESC exits placement mode
- Invalid placements show a brief error tooltip explaining why (wrong terrain, too close to existing node, can't afford, etc.)

### 1.3 — Placement Mode (Linear Items — Cables/Fiber)
- After selecting a cable type, the cursor enters **cable drawing mode**
- Click to set the start point (must be an existing node or snap point)
- Click additional waypoints along the desired route — each click adds a waypoint
- The cable preview renders as a Catmull-Rom spline through all placed waypoints in real-time
- **Click-and-drag** on any placed waypoint to adjust its position
- **Double-click** on the target node to complete the cable
- While drawing, the route auto-suggests along the nearest road (shown as a guide line) — player can accept or override with manual waypoints
- Running cost updates in real-time as the route lengthens (cost/km × total spline length, adjusted for terrain per segment)
- Right-click removes the last waypoint (undo)
- ESC cancels the entire cable
- Show aerial vs. underground indicator per segment based on terrain (can toggle per segment)

### 1.4 — Bottom Hotbar
- A slim horizontal bar at the bottom of the screen, always visible during gameplay
- Contains 9 slots (mapped to keys 1-9)
- Items are dragged from the radial menu to pin them to hotbar slots
- Pressing a number key immediately enters placement mode for that item
- Each slot shows: item icon, shorthand name, cost
- Slots can be rearranged by drag-and-drop
- Empty slots show as dim outlines
- Hotbar configuration persists across sessions (saved to local storage / settings)

### 1.5 — Build Mode HUD Overlay
- When in any placement mode, a contextual info bar appears at the top or side showing:
  - Currently selected item name and icon
  - Total cost so far (for cables: running total as waypoints are added)
  - Terrain at cursor position
  - For cables: total length, number of waypoints, aerial vs. underground status
  - Current cash balance
  - Shortcut hints (ESC to cancel, right-click to undo waypoint, double-click to finish)

### 1.6 — Remove Old Build Flow
- Remove the current "click map → BuildMenu popup" flow entirely
- Remove the "Build Node" and "Build Link" buttons from the HUD
- The radial menu and hotbar are the only build entry points

**Exit criteria:** Player can right-click → select item → place on map. Cables drawn with waypoints and splines. Hotbar works with number keys. Old build flow removed.

---

## 4. Phase 2 — Terrain Quality Overhaul

**Goal:** Make both Real Earth and Procgen maps look excellent at all zoom levels.

### 2.1 — Real Earth Mode: Enhanced Satellite + Vector Detail

#### 2.1.1 — Base Map Enhancement
- Keep ESRI World Imagery satellite tiles as the base layer (already working)
- Enhance the dimming/styling to create a more atmospheric night-earth aesthetic
- Add terrain relief shading (hillshade) overlay for topographic depth — use a free elevation tile source or pre-rendered hillshade tiles
- Rivers and water bodies should be clearly visible with a subtle blue tint/glow
- Coastlines should have a distinct treatment (subtle glow or gradient from land to water)

#### 2.1.2 — Progressive Detail by Zoom
- **Zoom 0-2 (World):** Satellite base + country borders + ocean coloring + major city glows
- **Zoom 2-4 (Continental):** Add state/province borders, rivers, terrain relief, more city labels
- **Zoom 4-6 (Regional):** Add major roads (highways, motorways from OSM vector tiles), terrain detail, all city labels
- **Zoom 6-8 (Metro):** Add all roads (primary + secondary), building footprints in urban areas, neighborhood detail
- **Zoom 8-10 (Local/Street):** Full road network (including residential), individual building footprints, parcels, detailed terrain
- Each zoom level smoothly transitions — no pop-in

#### 2.1.3 — Night-Earth Aesthetic
- Cities should glow at night with warm light (already partially implemented via ScatterplotLayer glow)
- The glow intensity should be proportional to population and development level
- At low zoom, cities appear as warm blobs of light on a dark satellite base
- At high zoom, the glow resolves into individual lit building footprints and street lights along roads
- Ocean areas should have a deep dark blue, not black
- The overall palette should feel like a mission control / war room map

### 2.2 — Procgen Mode: Vector Cell Rendering

#### 2.2.1 — Replace Gaussian Splat Bitmap with Vector Polygons
- Remove the terrain bitmap system (Gaussian splat canvas) for Procgen
- Render each Voronoi cell as a filled vector polygon using deck.gl PolygonLayer
- Cell polygons should have sharp, clean edges at all zoom levels
- Cell boundaries should be subtly visible as thin dark lines (or not visible — let the terrain color difference define boundaries)
- Colors follow the existing SATELLITE_COLORS palette but with richer variation within each terrain type (slight randomization per cell for natural look)

#### 2.2.2 — Terrain Detail Within Cells
- Large Voronoi cells should have internal detail to avoid looking flat:
  - Mountain cells: procedural ridge/peak patterns (subtle elevation contour lines or darker patches)
  - Forest/Rural cells: subtle tree canopy texture or green variation
  - Desert cells: sand dune patterns (lighter/darker bands)
  - Ocean cells: depth gradient (lighter near coast, darker in deep ocean)
  - Urban cells: grid-like pattern suggesting city blocks
- This detail can be achieved with sub-cell noise patterns or procedural texture overlays

#### 2.2.3 — Water & Coastlines
- Ocean cells should render with a distinct deep blue, not just dark
- Shallow ocean near coastlines should be visibly lighter blue
- Coastline edges should have a subtle foam/wave line or glow to clearly delineate land from water
- Rivers generated during world gen should render as visible blue lines connecting cells
- Lakes (if any) should render as filled blue polygons

#### 2.2.4 — Elevation Visualization
- Terrain coloring should subtly incorporate elevation data
- Higher elevation = slightly lighter/whiter tint (snow on mountains)
- Lower elevation = slightly richer/darker color
- Optional contour line overlay (toggle-able) for topographic map feel

### 2.3 — Shared Rendering Improvements (Both Modes)

#### 2.3.1 — Smooth Zoom Transitions
- All layers must smoothly appear/disappear as the player zooms
- Use opacity transitions, not hard cutoffs
- Layer visibility ranges should overlap slightly for seamless blending

#### 2.3.2 — Performance Budget
- Procgen vector cells: target 60fps with up to 6000 visible polygons
- Real Earth roads/buildings: use vector tile LOD (tiles only load what's visible at current zoom)
- Both modes: maintain <500MB browser memory target

**Exit criteria:** Real Earth looks like a professional satellite/night-earth map with roads and buildings at high zoom. Procgen looks like sharp, detailed vector terrain with visible rivers, coastlines, and terrain variation. Both modes smooth across all zoom levels.

---

## 5. Phase 3 — Road Networks & Urban Fabric

**Goal:** Add visible road networks and building footprints that serve as fiber routing corridors and demand points.

### 3.1 — Real Earth: OSM Road & Building Integration

#### 3.1.1 — Road Network Layer
- Integrate OpenStreetMap road data via vector tiles (OpenFreeMap or similar tile source already in use)
- Render roads as styled lines with classification-based width and color:
  - Motorway/highway: wide, bright line
  - Primary/secondary: medium, lighter line
  - Residential/tertiary: thin, subtle line
- Roads appear progressively: highways at zoom 4+, primary at zoom 5+, secondary at zoom 6+, residential at zoom 8+
- Roads serve as the **routing graph** for fiber cable placement (auto-route follows road geometry)
- Road data must be queryable: given a point, find the nearest road segment and its geometry

#### 3.1.2 — Building Footprint Layer
- Integrate building footprint data (Overture Maps Foundation or Microsoft Building Footprints via vector tiles)
- Render buildings as filled polygons with subtle 3D effect (shadow/offset for height)
- Building color by type if available (residential = grey, commercial = blue-grey, industrial = brown-grey)
- Buildings appear at zoom 7+
- Building density is the visual indicator of urban vs. suburban vs. rural
- Buildings serve as **demand points** — each building (or building cluster) represents potential subscribers

#### 3.1.3 — Road-Infrastructure Interaction
- When placing fiber in cable-drawing mode, the cursor should snap to the nearest road segment
- The auto-route pathfinder uses the road network graph
- Fiber cables render with a slight offset from road centerlines (so road and cable are both visible)
- At intersections, cables can branch (splice points auto-placed at road junctions)

### 3.2 — Procgen: Procedural Road & Building Generation

#### 3.2.1 — Inter-City Road Network
- Generate highways connecting cities within a region (Delaunay triangulation of city positions, pruned by distance)
- Generate major roads connecting cities across adjacent regions
- Road class hierarchy:
  - Highways: between major cities, straight-ish with gentle curves
  - Primary roads: between all cities in a region
  - Secondary roads: between cities and nearby rural areas
- Roads should follow terrain — avoid going straight through mountains (pathfind around high elevation)
- Road geometry stored as polylines with intermediate points for smooth curves

#### 3.2.2 — Intra-City Street Generation
- For each city, generate a street network within the city radius based on city layout style:
  - **Grid (American):** Rectangular block grid, main avenues wider, numbered/lettered streets
  - **Radial (European):** Central square/plaza, ring roads at increasing radius, radial avenues connecting center to edges
  - **Organic (Asian/Old-World):** Irregular winding streets, narrow alleys, occasional wider roads, grown over time feel
  - **Mixed:** Combination — historic core is organic, newer outer areas are grid
- City layout style assigned based on region cultural seed (randomized but consistent per region)
- Street density proportional to city population:
  - Hamlet (<50k): just a main road + a few side streets
  - Town (50k-250k): basic grid/radial core + residential streets
  - City (250k-1M): full street network, commercial district
  - Metropolis (1M-5M): dense street grid, multiple districts, ring roads
  - Megalopolis (5M+): massive multi-center street network, highways, rail corridors
- Streets within cities serve as fiber routing corridors just like Real Earth OSM roads

#### 3.2.3 — Building Placement Along Streets
- Generate building footprints along street edges:
  - Downtown core: tall buildings (large footprints), commercial, dense packing
  - Commercial district: medium buildings, storefronts along main streets
  - Residential: small rectangular footprints, set back from street, regular spacing
  - Industrial: large irregular footprints on outskirts
  - Suburban: small houses with yards (gaps between buildings)
- Building density decreases from city center outward
- Buildings serve as demand points (same as Real Earth mode)
- Building footprints are rectangles with slight random rotation and size variation for natural look

#### 3.2.4 — Terrain-Road Interaction
- Roads should avoid ocean/water cells
- Roads through mountainous terrain should follow valleys and passes (use elevation-aware pathfinding)
- Roads along coastlines should stay on land side
- Bridge markers where roads cross rivers (visual indicator, higher construction cost for fiber)

### 3.3 — Road Network as Data Structure
- The road network must exist as a queryable graph in the simulation backend (not just a visual layer)
- Each road segment has: start point, end point, intermediate geometry points, road class, surface type, length
- The road graph is used by:
  - Fiber auto-routing (pathfind along roads)
  - Cost calculation (fiber along existing roads is cheaper than cross-country)
  - AI corporations (they also route fiber along roads)
  - Coverage calculation (buildings along roads are the demand points)

**Exit criteria:** Both Real Earth and Procgen show road networks at appropriate zoom levels. Procgen cities have varied street layouts. Buildings are visible at high zoom. Roads are a queryable data structure usable for fiber routing.

---

## 6. Phase 4 — Full Infrastructure Hierarchy (All 6 Eras)

**Goal:** Implement all ~33 node types and ~15 edge types across all 6 eras, with the full FTTH access network hierarchy.

### 4.1 — Era 1: Telegraph (~1850s)

**Node Types:**
- **TelegraphOffice** — Urban hub for sending/receiving telegrams. Small building, manual operators.
- **TelegraphRelay** — Intermediate signal booster station along telegraph lines. Rural placement.
- **CableHut** — Shore-side facility for submarine telegraph cable landing.

**Edge Types:**
- **TelegraphWire** — Overhead single-wire line between poles. Low bandwidth, high latency.
- **SubseaTelegraphCable** — Undersea telegraph cable between landing stations. Very expensive, very slow.

**Era characteristics:** Very low throughput, high latency, long construction times, low cost per node but expensive per km of wire. Manual operation.

### 4.2 — Era 2: Telephone (~1900s)

**Node Types:**
- **ManualExchange** — Human-operated telephone switchboard. Urban. Limited capacity.
- **AutomaticExchange** — Mechanical automatic switch (Strowger). Urban. Higher capacity than manual.
- **TelephonePole** — Utility pole carrying telephone lines. Cheapest node, ubiquitous.
- **LongDistanceRelay** — Amplifier station for long-distance copper trunk lines.

**Edge Types:**
- **CopperTrunkLine** — Multi-pair copper cable between exchanges. Medium bandwidth.
- **LongDistanceCopper** — Heavy gauge copper for long-haul voice circuits. Expensive, limited bandwidth.

**Era characteristics:** Voice-centric, copper everywhere, manual/mechanical switching, party lines, operator-assisted long distance.

### 4.3 — Era 3: Early Digital (~1970s)

**Node Types:**
- **DigitalSwitch** — Electronic digital telephone switch (ESS). High capacity.
- **MicrowaveTower** — Point-to-point microwave relay for long-distance. Line-of-sight requirement.
- **CoaxHub** — Cable TV distribution hub. Coaxial cable network center.
- **EarlyDataCenter** — Mainframe computer center. Expensive, low by modern standards.
- **SatelliteGroundStation** — Earth station for geostationary satellite links. Global reach but high latency.

**Edge Types:**
- **CoaxialCable** — Broadband coaxial cable for TV/data. Higher bandwidth than copper.
- **MicrowaveLink** — Point-to-point microwave between towers. Distance limited by line-of-sight.
- **EarlySatelliteLink** — GEO satellite connection. High latency (~600ms), moderate bandwidth.

**Era characteristics:** Transition from analog to digital, first data networks, CATV infrastructure, microwave backbone, early satellite.

### 4.4 — Era 4: Internet (~1990s)

**Node Types:**
- **CentralOffice_DSL** — Central office with DSLAM for DSL broadband over copper. Serves neighborhood radius.
- **CellTower_2G3G** — Mobile cellular tower (2G/3G). Wireless coverage radius.
- **FiberPOP** — Fiber Point of Presence. Regional aggregation point.
- **InternetExchangePoint** — IXP where ISPs peer and exchange traffic. High throughput.
- **SubseaLandingStation** — Modern submarine fiber cable landing facility.
- **ColocationFacility** — Multi-tenant data center for hosting. Revenue from tenants.
- **ISPGateway** — Internet service provider edge router. Customer-facing.

**Edge Types:**
- **FiberLocal** — Short-range fiber optic cable. Access/distribution network.
- **FiberRegional** — Metro/regional fiber ring. Medium distance.
- **FiberNational** — Long-haul intercity fiber. National backbone.
- **SubseaFiberCable** — Submarine fiber optic cable. Transoceanic backbone.

**Era characteristics:** Internet explosion, DSL over existing copper, first fiber deployments, dot-com boom, cell towers everywhere, IXPs form.

### 4.5 — Era 5: Modern (~2010s)

**Node Types:**
- **MacroCell_4G5G** — Modern cellular tower (4G LTE / 5G NR). High bandwidth wireless.
- **SmallCell** — Low-power urban cell for 5G densification. Mounted on poles/buildings.
- **EdgeDataCenter** — Small data center at network edge for low-latency compute.
- **HyperscaleDataCenter** — Massive cloud data center (AWS/Google/Azure scale). Enormous capacity and cost.
- **CloudOnRamp** — Direct connection point to cloud providers. Premium peering.
- **ContentDeliveryNode** — CDN edge cache for content delivery. Reduces backbone traffic.
- **FiberSplicePoint** — Passive fiber junction/splice closure along routes. Enables branching.
- **DWDM_Terminal** — Dense Wavelength Division Multiplexing endpoint. Multiplies fiber capacity.
- **FiberDistributionHub** — Outdoor cabinet connecting feeder fiber to splitters for FTTH.
- **NetworkAccessPoint** — Pole/pedestal-mounted access point serving 4-12 nearby buildings for FTTH.

**Edge Types:**
- **FiberMetro** — Metro fiber ring with DWDM. Very high bandwidth within a metro area.
- **FiberLongHaul** — Cross-country fiber backbone with DWDM. Terabit capacity.
- **DWDM_Backbone** — Ultra-high-capacity wavelength-multiplexed backbone link.
- **SatelliteLEOLink** — Low-Earth-orbit satellite connection (Starlink-style). Low latency, moderate bandwidth.
- **FeederFiber** — High strand count (48-288) fiber from CO to FDH.
- **DistributionFiber** — Medium strand count (12-48) fiber from FDH to NAPs.
- **DropCable** — Single fiber (1-2 strands) from NAP to individual premises.

**Era characteristics:** FTTH rollouts, 5G densification, hyperscale cloud, CDN edge caching, DWDM capacity multiplication, LEO satellite constellations.

### 4.6 — Era 6: Near Future (~2030s)

**Node Types:**
- **LEO_SatelliteGateway** — Ground station for dense LEO constellation. Global broadband.
- **QuantumRepeater** — Quantum key distribution relay for unhackable communication links.
- **MeshDroneRelay** — Autonomous drone providing temporary/emergency wireless coverage.
- **UnderwaterDataCenter** — Sealed ocean-floor data center (Project Natick style). Free cooling.
- **NeuromorphicEdgeNode** — AI-optimized edge compute for real-time network management.
- **TerahertzRelay** — Extremely high frequency short-range relay for ultra-dense areas.

**Edge Types:**
- **QuantumFiberLink** — Quantum-secured fiber connection. Unhackable but expensive.
- **TerahertzBeam** — THz frequency point-to-point beam. Extremely high bandwidth, very short range.
- **LaserInterSatelliteLink** — Free-space optical link between satellites. Space backbone.

**Era characteristics:** Quantum security, AI-managed networks, LEO mega-constellations, terahertz short-range, underwater infrastructure, autonomous systems.

### 4.7 — FTTH Access Network Game Loop (Modern+ Eras)

This is the core fiber-to-the-home gameplay loop that becomes available with Modern era infrastructure:

1. **Place Central Office** in or near a city → houses OLT equipment
2. **Lay Feeder Fiber** from CO along major roads toward neighborhoods → thick cable, high strand count
3. **Place Fiber Distribution Hub** at strategic neighborhood locations → connects to feeder, houses splitters
4. **Lay Distribution Fiber** from FDH along neighborhood streets → medium cable
5. **Place Network Access Points** on poles/pedestals along distribution routes → serve 4-12 nearby buildings each
6. **NAP auto-covers nearby buildings** within its service radius → each covered building = subscriber = revenue
7. **Optional: Lay Drop Cables** manually from NAP to specific buildings for a cost discount vs. NAP auto-coverage → incentivizes micro-management when small, lets policy handle it when large

The tiered management system applies:
- **Small company (1-50 NAPs):** Player manually places each NAP and optionally draws drop cables for cost savings
- **Medium company (50-200 NAPs):** Player places FDHs and NAPs; drop cable connections are auto-managed by maintenance teams
- **Large company (200+ NAPs):** Player sets policies (coverage targets, budget allocation); AI/department managers handle FDH+NAP placement and drop cable routing within policy parameters

### 4.8 — Node Type Data Requirements

Each node type needs:
- **Name and description** (user-facing, per-era flavor text)
- **Era** (which era it becomes available)
- **Network tier** (Access / Aggregation / Core / Backbone / Global)
- **Base construction cost** (before terrain multiplier)
- **Construction time** (in ticks)
- **Maintenance cost per tick**
- **Max throughput / capacity**
- **Coverage radius** (wireless nodes only; 0 for wired-only nodes)
- **Coverage type** (wireless broadcast vs. wired service radius)
- **Coverage capacity fraction** (% of throughput dedicated to local coverage vs. backbone transit)
- **Jobs created** (employees needed to operate)
- **Terrain restrictions** (e.g., SubmarineLanding only on coastal cells, UnderwaterDataCenter only in ocean)
- **Prerequisites** (required research, if any — but NOT era-locked)
- **Icon** (unique icon for map rendering and UI)
- **Size category** (affects map icon size and LOD visibility zoom threshold)

### 4.9 — Edge Type Data Requirements

Each edge type needs:
- **Name and description**
- **Era**
- **Allowed node connections** (which node type pairs can this edge connect)
- **Max distance** (distance multiplier × cell spacing, or unlimited for satellite)
- **Deployment method** (aerial or underground — or player choice for modern fiber)
- **Cost model** (per-km, flat, or hybrid)
- **Bandwidth / capacity**
- **Latency characteristics**
- **Strand count** (for fiber types — affects visual thickness)
- **Vulnerability profile** (aerial: weather vulnerable. Underground: earthquake vulnerable. Submarine: anchor strike vulnerable)
- **Construction time per km**
- **Visual style** (color, thickness, dash pattern, glow)

**Exit criteria:** All ~33 node types and ~15 edge types implemented with full data. FTTH game loop functional. Tiered management for access network deployment. All eras playable.

---

## 7. Phase 5 — Spline-Based Fiber Routing & Waypoint System

**Goal:** Replace straight-line edges with smooth Catmull-Rom spline curves that follow terrain and roads, with full waypoint editing.

### 5.1 — Edge Data Model Upgrade
- Every edge stores an ordered list of waypoints (lon, lat pairs) in addition to source and target node IDs
- Minimum 2 waypoints (source and target positions) for simple edges
- No upper limit on waypoints for complex routes
- Waypoints are stored in the ECS as part of the edge component
- Waypoints are synchronized in multiplayer (included in delta broadcasts)

### 5.2 — Catmull-Rom Spline Rendering
- Edge geometry is computed as a Catmull-Rom spline passing through all waypoints
- The spline is tessellated into a polyline (e.g., 10-20 segments per span) for rendering
- Rendering uses deck.gl PathLayer (not LineLayer) to support the polyline geometry
- Spline is recomputed whenever waypoints change

### 5.3 — Visual Style by Zoom Level

**Low zoom (0-4) — Strategic view:**
- Edges render as glowing lines (current style) but following the spline path
- Color by edge type (existing color scheme)
- Backbone edges pulse with animated traffic flow
- Regional edges have steady glow
- Access/local edges not visible at this zoom

**High zoom (5+) — Planning view:**
- Edges render as road-hugging realistic lines following the spline
- Offset slightly from road centerline so road and cable are both visible
- **Aerial edges:** Dashed line with small dots at regular intervals representing utility poles
- **Underground edges:** Solid line
- Color by edge type
- **Line thickness by strand count / capacity:**
  - Drop cable (1-2 strands): 1px
  - Distribution fiber (12-48 strands): 2-3px
  - Feeder fiber (48-288 strands): 4-5px
  - Backbone (DWDM): 6-8px
  - Submarine: 8-10px
- Animated traffic flow particles on edges with high utilization (>50%)
- Health coloring: green (healthy) → amber (degraded) → red (damaged)

### 5.4 — Waypoint Editing (Post-Build)
- Clicking on an existing edge enters **edge edit mode**
- All waypoints become visible as draggable handles on the spline
- **Click-and-drag** any waypoint to move it — spline updates in real-time
- **Click on the spline** between waypoints to insert a new waypoint at that position
- **Right-click a waypoint** to delete it (minimum 2 waypoints enforced)
- **Double-click** or press Enter to confirm edits
- ESC to cancel and revert to original waypoints
- Moving waypoints recalculates cost (longer route = higher cost)
- In multiplayer, waypoint edits are sent as commands and validated server-side

### 5.5 — Auto-Route Along Roads
- When drawing a new cable, the system pathfinds along the road network between source and target
- The pathfinder uses the road graph (from Phase 3) as its navigation mesh
- Road segments have weights based on: distance + terrain cost multiplier + road class (highway cheaper, residential more expensive per km due to permits)
- The auto-generated route is presented as a set of waypoints snapped to road geometry
- Player can accept the auto-route or modify individual waypoints
- If no road connection exists between source and target, the system falls back to direct terrain pathfinding (cross-country, higher cost)
- Cost comparison shown: "Along roads: $X | Direct: $Y"

### 5.6 — Click-and-Drag Cable Drawing
- While in cable-drawing mode, player can click-and-drag to draw a freeform path
- The system converts the drag path into a series of waypoints (simplified/smoothed)
- Waypoints snap to nearest road segments if within snapping distance
- The drag trail shows in real-time with cost accumulating
- Releasing the mouse button sets the waypoints; double-click or click on target node to complete

### 5.7 — Legacy Edge Migration
- Existing edges (built before this system) are displayed with a straight-line fallback (2 waypoints only)
- An auto-fix tool can retroactively route existing edges along roads (optional, player-triggered)

**Exit criteria:** All edges render as smooth Catmull-Rom splines. Waypoints are click-and-draggable. Auto-routing follows roads. Aerial and underground have distinct visual styles. Thickness reflects capacity.

---

## 8. Phase 6 — City Density, Buildings & Demand

**Goal:** Cities are living, dense environments with building-level demand, not just dots on a map.

### 6.1 — City Zone System
- Each city has concentric zones radiating from its center:
  - **Downtown Core** (0-1km): Highest density, tallest buildings, commercial/office, highest demand per building
  - **Commercial District** (1-3km): Medium-tall buildings, mixed commercial/residential
  - **Inner Residential** (3-5km): Dense residential, apartment buildings, moderate demand
  - **Outer Residential** (5-10km): Lower density, houses, lower demand per building but more buildings
  - **Suburban Fringe** (10-20km): Sparse residential, large lots, lowest demand
- Zone radii scale with city population (megalopolis downtown is larger than hamlet's)
- Each zone has a characteristic building density, building height, and demand per building

### 6.2 — Building-as-Demand-Point
- Every building (from OSM in Real Earth, or procedurally generated in Procgen) is a potential subscriber
- Each building has:
  - **Position** (lon, lat)
  - **Type** (residential, commercial, industrial)
  - **Demand value** (based on type + zone + development level)
  - **Connected status** (unserved, covered by NAP, manually connected via drop cable)
  - **Service provider** (which corporation, if any)
- **NAP auto-coverage:** A placed NAP automatically serves all buildings within its coverage radius
  - Coverage radius varies by NAP type and era
  - Revenue per auto-covered building = base rate × demand value × (1 - overhead deduction)
  - The "overhead deduction" represents the cost of not having a dedicated drop — e.g., 15-20% less revenue than a manual drop
- **Manual drop cable:** Player can draw a drop cable from NAP to a specific building
  - Costs a small amount per drop
  - Revenue per manually-connected building = base rate × demand value (full, no overhead deduction)
  - This makes manual drops ~15-20% more profitable per building, rewarding micro-management
- **Tiered management:** At large scale, policies auto-place NAPs and auto-connect buildings. The overhead deduction simulates the inefficiency of automated vs. hand-crafted deployment.

### 6.3 — Demand Visualization
- Buildings change color/icon based on connection status:
  - **Unserved:** Dark/grey (no provider)
  - **Covered by NAP (auto):** Dim provider color (e.g., dim green for player)
  - **Manually connected (drop cable):** Bright provider color with a small connection indicator
  - **Served by competitor:** Competitor's color
- At high zoom (8+), individual building status is visible
- At medium zoom (5-7), demand is shown as a heat map overlay (aggregate per block/area)
- At low zoom (0-4), demand is shown as city glow intensity

### 6.4 — Revenue Model Update
- Revenue shifts from abstract "coverage per cell" to concrete "subscribers per building"
- Total revenue = sum of all connected buildings' demand values × pricing tier × service quality modifier
- This makes infrastructure placement tangible: every NAP placed opens up revenue from specific buildings
- Market competition: if a competitor also serves a building, revenue is split based on pricing and service quality

### 6.5 — Population Growth Impact
- As cities grow (population system), new buildings appear in the suburban fringe
- Growing cities create new demand that can be served by expanding infrastructure
- Declining cities lose buildings (abandoned, greyed out) — revenue drops

**Exit criteria:** Buildings are demand points with connection status. NAP auto-coverage works with overhead deduction. Manual drops are cheaper. Demand visualized on map. Revenue tied to building connections.

---

## 9. Phase 7 — Submarine Cable System

**Goal:** Full submarine cable gameplay with landing stations, bathymetry-aware routing, and real-world reference data.

### 7.1 — Submarine Cable Placement
- Player places **Landing Stations** on coastal cells (must be within 1-2 cells of ocean)
- To build a submarine cable: select Submarine Cable from build menu → click source Landing Station → draw waypoints across ocean → double-click target Landing Station
- Waypoints can be placed in ocean to route around obstacles (mid-ocean ridges, island chains, shipping lanes)
- Catmull-Rom spline rendering through ocean waypoints

### 7.2 — Bathymetry Visualization
- Ocean areas should show depth when relevant (during submarine cable placement):
  - Continental shelf: lighter blue, lower cost
  - Deep ocean: darker blue, higher cost
  - Trenches/ridges: visual indicators, highest cost
- Depth data can be simplified (Real Earth: elevation data from world gen. Procgen: generated ocean floor)
- Cost per km increases with depth (shallow shelf × 1.0, deep ocean × 2.0, trench × 3.0)

### 7.3 — Submarine Cable Properties
- Very high capacity (terabits), very high cost ($millions per cable)
- Long construction time (months/years in game ticks)
- Vulnerability: anchor strikes, earthquakes, shark bites (disasters), sabotage (covert ops)
- Repair is extremely expensive and slow (specialized cable ship needed)
- Each cable has a defined strand/fiber count and capacity
- Revenue from transit traffic between continents

### 7.4 — Real-World Reference (Real Earth Mode)
- Use TeleGeography submarine cable map data as reference overlay
- Show existing real-world submarine cable routes as a toggle-able reference layer (dimmed, non-interactive)
- Landing station locations from real data can be used as suggested placement points
- This gives players a sense of where cables actually go and why

### 7.5 — Cable Ship Mechanic
- Submarine cable construction requires a "cable ship" (abstractly — a construction unit)
- Cable ships are expensive to commission and slow
- Only one cable can be under construction per cable ship at a time
- This creates strategic decisions about cable construction sequencing

**Exit criteria:** Submarine cables can be placed between landing stations with ocean waypoints. Bathymetry affects cost. Real TeleGeography reference data available. Cable ships as construction constraint.

---

## 10. Phase 8 — Spectrum & Frequency Management

**Goal:** Wireless infrastructure requires licensed spectrum bands, creating a strategic resource management layer.

### 8.1 — Spectrum Bands
- Define spectrum bands per era:
  - Telephone: n/a (wired only)
  - Early Digital: VHF/UHF bands (limited)
  - Internet: 800MHz, 1900MHz, 2100MHz (2G/3G bands)
  - Modern: 700MHz, 850MHz, 1900MHz, 2500MHz, 3500MHz (mid-band 5G), 28GHz/39GHz (mmWave)
  - Near Future: additional THz bands, LEO satellite spectrum
- Each band has properties: coverage range (lower freq = farther), capacity (higher freq = more bandwidth), penetration (lower freq penetrates buildings better)

### 8.2 — Spectrum Auctions
- Regional spectrum licenses are auctioned periodically (every N ticks per region)
- Players bid for spectrum licenses in specific regions
- AI corporations also bid (competitively, based on their archetype)
- Winning a license grants exclusive use of that band in that region for a defined period
- License expiry triggers re-auction
- Cost of spectrum licenses scales with region population and competition

### 8.3 — Frequency Assignment
- Wireless nodes (cell towers, small cells, relays) must be assigned a spectrum band
- A node can only use bands the player has licensed in that region
- Capacity of the node depends on the band assigned (mmWave = high capacity but short range, low-band = long range but lower capacity)
- Multiple bands can be assigned to a single node (carrier aggregation) for combined capacity
- Interference: too many nodes on the same band in close proximity degrade each other's performance

### 8.4 — Spectrum Visualization
- Overlay showing spectrum allocation per region (which bands owned by which corporation)
- Wireless coverage overlay colored by frequency band
- Interference heat map showing congested spectrum areas

**Exit criteria:** Spectrum bands defined per era. Auctions work. Wireless nodes require licensed spectrum. Frequency assignment affects capacity and coverage. Spectrum overlay visualization.

---

## 11. Phase 9 — Weather, Disasters & Cable Vulnerability

**Goal:** Infrastructure is vulnerable to weather and disasters based on deployment method and location. Aerial vs. underground choice matters.

### 9.1 — Deployment Vulnerability Matrix

| Deployment | Weather Vulnerability | Earthquake Vulnerability | Flood Vulnerability | Sabotage Vulnerability |
|------------|----------------------|-------------------------|--------------------|-----------------------|
| Aerial | HIGH (wind, ice storms, trees) | LOW | MEDIUM | HIGH (easy to access) |
| Underground | LOW | HIGH | HIGH (water ingress) | LOW (hard to find) |
| Submarine | LOW (surface weather) | HIGH (seabed movement) | N/A | MEDIUM (anchor strikes) |
| Wireless | MEDIUM (signal degradation in rain/snow) | LOW | LOW | LOW |

### 9.2 — Weather System
- Regions have weather patterns based on terrain and latitude
- Weather events affect infrastructure:
  - **Storms/hurricanes:** Damage aerial cables, reduce wireless signal, no effect on underground
  - **Ice storms:** Heavy damage to aerial cables (ice loading), minor wireless degradation
  - **Flooding:** Damage underground cables (water ingress), minor aerial impact
  - **Extreme heat:** Minor degradation to all electronics, increased cooling costs for data centers
  - **Earthquakes:** Heavy damage to underground and submarine, minor to aerial (poles may shift)
- Weather events are regional and time-limited (last N ticks)
- Weather forecast visible 5-10 ticks ahead (allows preparation)

### 9.3 — Damage & Repair
- Damaged edges lose capacity proportional to damage severity
- Severely damaged edges go offline entirely
- Repair options:
  - **Emergency repair:** Expensive, fast (restore partial service quickly)
  - **Standard repair:** Normal cost, multi-tick (full restoration)
  - **Crew-based:** Having maintenance crews in the region speeds repair
- Aerial repairs are faster and cheaper than underground repairs
- Submarine repairs are the slowest and most expensive (cable ship required)

### 9.4 — Insurance Integration
- The existing insurance system (already in ECS) should interact with the vulnerability system
- Insurance premiums scale with:
  - Deployment method vulnerability
  - Regional disaster risk
  - Historical damage in region
- Insurance payouts offset repair costs
- Uninsured infrastructure is a gamble — cheaper running costs but catastrophic if damaged

### 9.5 — Disaster Visualization
- Active weather events shown on map (storm icon, affected area highlighted)
- Damaged infrastructure shown with visual indicators (red pulsing, broken line effect)
- Repair progress shown on damaged edges (progress bar)
- Weather forecast overlay showing predicted events

**Exit criteria:** Aerial vs. underground have different vulnerability profiles. Weather events damage infrastructure realistically. Repair system works. Insurance interacts with vulnerability. Weather visualized on map.

---

## 12. Phase 10 — Network Monitoring Dashboard

**Goal:** A Bloomberg Terminal-style real-time view of your fiber network's health, traffic, and performance.

### 10.1 — Dashboard Panel
- A new panel type accessible from the management panels (alongside finance, operations, etc.)
- Layout: grid of real-time widgets, each showing a different aspect of network health
- Dark theme, monospace numbers, green/red/amber color coding (existing Bloomberg aesthetic)

### 10.2 — Widgets

#### 10.2.1 — Network Health Overview
- Total nodes: online / degraded / offline / under construction
- Total edges: healthy / degraded / damaged / offline
- Overall network health percentage (weighted by capacity)
- Historical health graph (last 100 ticks)

#### 10.2.2 — Traffic Flow Visualization
- Real-time traffic volume across the network
- Top 10 busiest edges (by utilization %)
- Top 10 busiest nodes (by load)
- Traffic matrix showing origin-destination flows between cities
- Total traffic served vs. dropped (and trend)

#### 10.2.3 — Bottleneck Detection
- Automatically identify edges at >80% utilization (bottlenecks)
- Show bottleneck locations on map (highlight in red)
- Suggest capacity upgrades (which edges to upgrade, estimated cost)
- Historical bottleneck frequency per edge

#### 10.2.4 — Revenue by Infrastructure
- Revenue breakdown by node type, edge type, region
- Revenue per subscriber by zone (downtown vs. suburban)
- Most profitable and least profitable infrastructure pieces
- Revenue trend graphs

#### 10.2.5 — SLA Monitoring
- Active contracts and their SLA requirements
- Current performance vs. SLA targets (latency, uptime, throughput)
- At-risk contracts (approaching breach thresholds)
- Contract breach history and penalty costs

#### 10.2.6 — Coverage Map
- Interactive overlay showing your network's coverage footprint
- Gaps in coverage highlighted
- Potential revenue in uncovered areas
- Coverage comparison vs. competitors (market share by region)

#### 10.2.7 — Maintenance Queue
- Infrastructure needing repair (sorted by priority)
- Maintenance crew allocation
- Scheduled maintenance vs. emergency repairs
- Maintenance cost forecast

#### 10.2.8 — Capacity Planning
- Projected demand growth by region
- Current capacity headroom by node/edge
- Recommended infrastructure investments
- "What if" scenario: if traffic grows X%, which edges hit capacity first?

### 10.3 — Map Integration
- Clicking any item in the dashboard highlights it on the map
- Dashboard widgets can be pinned as floating overlays on the map view
- Network health overlay: color edges by health/utilization directly on the map

**Exit criteria:** Network monitoring dashboard panel with all widgets. Real-time data from ECS. Bottleneck detection. SLA monitoring. Map integration for highlighting.

---

## 13. Phase 11 — Minimap & Navigation

**Goal:** Corner minimap for spatial awareness plus search/jump-to navigation.

### 11.1 — Corner Minimap
- Small (200x150px) minimap in the bottom-right or top-right corner of the screen
- Shows the entire world at a glance
- Displays:
  - Land/ocean base (simplified terrain colors)
  - Your infrastructure as colored dots/lines (player's corp color)
  - Competitor infrastructure as grey dots (if visible)
  - City locations as small dots (sized by population)
  - Current viewport rectangle (white outline showing what's on the main map)
- Click on the minimap to jump the main view to that location
- Click-and-drag on the minimap to pan the main view
- Minimap can be toggled on/off
- Minimap can be resized (drag corner)

### 11.2 — Search/Jump-To
- A search bar (keyboard shortcut: /) that allows searching:
  - City names
  - Region names
  - Your infrastructure by type ("my cell towers", "my data centers")
  - Competitor names
- Search results show as a dropdown list with location
- Clicking a result jumps the map to that location and zooms to appropriate level
- Recent searches saved for quick access

### 11.3 — Bookmarks
- Player can bookmark map locations (right-click → "Bookmark this location")
- Bookmarks appear in a sidebar list and on the minimap
- Quick-jump to bookmarks via keyboard shortcuts or list click

**Exit criteria:** Minimap shows world overview with infrastructure and viewport rectangle. Search bar finds cities, regions, infrastructure. Bookmarks system works.

---

## 14. Phase 12 — Competitor Visualization

**Goal:** All competitor infrastructure is always visible on the map with clear visual distinction.

### 12.1 — Competitor Infrastructure Rendering
- All competitor nodes rendered on the map at all times (no fog of war for infrastructure)
- Competitor nodes use their corporation color (existing 8-color cycle)
- Competitor nodes are slightly dimmer/smaller than player's own nodes (visual hierarchy: your stuff stands out)
- Competitor edges (cables) rendered in competitor's color, slightly thinner than player's cables
- Competitor labels show corporation name on hover

### 12.2 — Competitive Overlay
- Toggle-able overlay showing:
  - Market share by region (pie chart per region or color gradient)
  - Coverage overlap areas (where multiple providers compete)
  - Competitor expansion patterns (where they're currently building — under-construction shown as dashed)
- This overlay helps strategic planning: where to compete, where to avoid, where to expand into unserved areas

### 12.3 — Competitor Detail (On Hover/Click)
- Hovering a competitor node shows: type, owner name, online status
- Clicking a competitor node shows: basic stats (capacity, health if visible)
- No espionage required for basic infrastructure visibility (design decision: always visible)
- Detailed financials/utilization data still requires espionage intel (existing system)

**Exit criteria:** All competitor infrastructure always visible. Color-coded by corporation. Competitive overlay shows market dynamics. Click for details.

---

## 15. Phase 13 — Polish, Performance & Integration Testing

**Goal:** Everything works together smoothly, performs well, and feels polished.

### 13.1 — Performance Optimization
- Verify 60fps rendering with:
  - 6000+ Voronoi cells (Procgen vector rendering)
  - 10,000+ building footprints visible at high zoom
  - 1000+ road segments visible
  - 500+ infrastructure nodes + edges with spline rendering
  - All overlays active
- Implement LOD culling for buildings (only render buildings in viewport)
- Implement LOD culling for road segments (only render visible roads at current zoom)
- Spline tessellation quality reduces at low zoom (fewer segments per span)
- Verify WASM module size stays <5MB gzipped with all new entity types
- Verify simulation tick <50ms with expanded entity count
- Profile and optimize hot paths

### 13.2 — Multiplayer Sync
- All new entity types (FTTH nodes, waypoint edges, building connections) sync correctly via delta broadcasts
- Waypoint data included in edge creation/update deltas
- Building connection status updates broadcast to all players
- Submarine cable construction progress synced
- Spectrum auction bidding works in real-time multiplayer
- Weather events broadcast to all players simultaneously

### 13.3 — AI Corporation Integration
- AI corporations use the new FTTH game loop:
  - AI places COs, feeder fiber, FDHs, NAPs to serve cities
  - AI routes fiber along roads (uses the same road-based pathfinder as players)
  - AI makes aerial vs. underground decisions based on archetype (Budget Operator prefers aerial, Defensive Consolidator prefers underground)
  - AI participates in spectrum auctions (bid amounts based on archetype)
  - AI repairs damaged infrastructure based on priority
- AI behavior should create competitive pressure: AI expands into unserved areas, competes for subscribers

### 13.4 — Save/Load Compatibility
- New entity types, waypoint data, building connections, spectrum licenses, weather state all included in save files
- Save file format versioned to handle migration from old saves (old saves load with default/empty new fields)
- Cloud saves work with expanded state

### 13.5 — Tauri Desktop Compatibility
- All new features work in Tauri desktop app
- gt-tauri bridge updated for new entity types and queries
- Native file save/load handles expanded save format

### 13.6 — UI/UX Polish
- All new panels and widgets follow existing Bloomberg Terminal aesthetic
- Consistent color coding across all new systems
- Tooltip system updated for all new entity types
- Keyboard shortcuts documented and consistent
- Tutorial hints for new systems (build menu, cable drawing, FTTH loop)
- Loading states for heavy operations (road network loading, building data loading)

### 13.7 — Testing
- Rust unit tests for all new entity types, terrain generation improvements, pathfinding
- Integration tests for FTTH game loop (CO → feeder → FDH → distribution → NAP → building)
- Frontend component tests for new UI elements (radial menu, hotbar, cable drawing, dashboard widgets)
- Multiplayer sync tests for new entity types
- Performance benchmarks for rendering and simulation with expanded entity counts
- Cross-browser testing (Chrome, Firefox, Safari) for WebGL/deck.gl rendering

**Exit criteria:** 60fps with full entity load. Multiplayer syncs all new types. AI uses new systems. Saves work. Tauri works. All tests pass.

---

## 16. Cross-Cutting Concerns

These apply across all phases:

### 16.1 — Data Model Consistency
- All new node/edge types follow the same ECS component pattern as existing types
- New components are added to gt-common, implementations in gt-simulation
- Bridge trait (gt-bridge) extended for new queries
- TypeScript types (wasm/types.ts) updated in lockstep with Rust types

### 16.2 — Determinism
- All new systems must be deterministic (same inputs = same outputs)
- Road generation from seed must produce identical results
- Building placement from seed must produce identical results
- Weather events seeded from world seed + tick number
- Spectrum auction resolution deterministic (tie-breaking by entity ID)

### 16.3 — Multiplayer Protocol
- New DeltaOps variants for all new entity types
- Waypoint data serialized efficiently (array of coordinate pairs)
- Building connection status changes as lightweight deltas (not full building data)
- Weather events as broadcast events (not per-entity)
- Spectrum auction bids as commands, results as broadcasts

### 16.4 — Backward Compatibility
- Old save files load into new system (missing fields get defaults)
- Old worlds without roads/buildings still playable (legacy mode: no FTTH, original node types only)
- Multiplayer protocol version negotiation (old clients see simplified view)

---

## 17. Dependency Graph

```
Phase 0 (Foundation Fixes)
    ↓
Phase 1 (Build UX) ←────────────────────────────────────────┐
    ↓                                                         │
Phase 2 (Terrain Quality) ← independent, can parallel with 1 │
    ↓                                                         │
Phase 3 (Roads & Urban Fabric) ← depends on Phase 2          │
    ↓                                                         │
Phase 4 (Infrastructure Hierarchy) ← depends on Phase 1      ┘
    ↓
Phase 5 (Spline Routing) ← depends on Phase 3 (roads) + Phase 4 (new edge types)
    ↓
Phase 6 (City Density & Demand) ← depends on Phase 3 (buildings) + Phase 4 (FTTH nodes)
    ↓
Phase 7 (Submarine Cables) ← depends on Phase 5 (spline system) + Phase 2 (ocean terrain)
    ↓
Phase 8 (Spectrum) ← depends on Phase 4 (wireless node types)
    ↓
Phase 9 (Weather & Disasters) ← depends on Phase 5 (aerial/underground edges)
    ↓
Phase 10 (Network Dashboard) ← depends on Phase 6 (building demand) + Phase 9 (health data)
    ↓
Phase 11 (Minimap) ← can be done anytime after Phase 2
    ↓
Phase 12 (Competitor Viz) ← depends on Phase 4 (all node types) + Phase 5 (spline edges)
    ↓
Phase 13 (Polish & Testing) ← depends on all above

Parallelizable pairs:
  - Phase 1 + Phase 2 (independent)
  - Phase 7 + Phase 8 (independent after Phase 5)
  - Phase 11 (anytime after Phase 2)
  - Phase 9 + Phase 10 (partially parallel)
```

---

## Summary

| Phase | Name | Key Deliverables |
|-------|------|-----------------|
| 0 | Foundation Fixes | City water fix, bridge extensibility |
| 1 | Build UX Revolution | Radial menu, hotbar, placement modes, cable drawing |
| 2 | Terrain Quality | Real Earth enhancement, Procgen vector cells, coastlines, rivers |
| 3 | Roads & Urban Fabric | OSM roads (Real Earth), procgen streets + buildings (Procgen), road graph |
| 4 | Infrastructure Hierarchy | All ~33 node types, ~15 edge types, 6 eras, FTTH game loop |
| 5 | Spline Routing | Catmull-Rom edges, waypoints, click-and-drag, auto-route along roads |
| 6 | City Density & Demand | Building-as-demand-point, NAP coverage, manual drops, tiered management |
| 7 | Submarine Cables | Landing stations, bathymetry routing, cable ships, TeleGeography reference |
| 8 | Spectrum Management | Spectrum bands, auctions, frequency assignment, interference |
| 9 | Weather & Disasters | Aerial/underground vulnerability, weather events, repair, insurance |
| 10 | Network Dashboard | Bloomberg-style monitoring, bottleneck detection, SLA, capacity planning |
| 11 | Minimap & Navigation | Corner minimap, search/jump-to, bookmarks |
| 12 | Competitor Viz | Always-visible competitor infra, market share overlay, competitive analysis |
| 13 | Polish & Testing | Performance, multiplayer sync, AI integration, save compat, testing |
