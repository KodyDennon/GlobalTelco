# Document 1: Infrastructure & Routing Simulation Specification

**Purpose:** Full simulation of telecom infrastructure, routing, disasters, and multi-owner cooperative assets. Designed for AI-agentic management.

---

## 1. Nodes & Edges
- **Attributes (all nodes/edges):** Capacity, latency, reliability, maintenance cost, ownership (multi-player support), disaster risk, era unlock, network tier.

### 1.1 Node Types (~33 era-specific types, flat enum)
- **Telegraph Era (~1850s):** TelegraphOffice, TelegraphRelay, CableHut.
- **Telephone Era (~1900s):** ManualExchange, AutomaticExchange, TelephonePole, LongDistanceRelay.
- **Early Digital Era (~1970s):** DigitalSwitch, MicrowaveTower, CoaxHub, EarlyDataCenter, SatelliteGroundStation.
- **Internet Era (~1990s):** CentralOffice (DSL), CellTower (2G/3G), FiberPOP, InternetExchangePoint, SubseaLandingStation, ColocationFacility, ISPGateway.
- **Modern Era (~2010s):** MacroCell (4G/5G), SmallCell, EdgeDataCenter, HyperscaleDataCenter, CloudOnRamp, ContentDeliveryNode, FiberSplicePoint, DWDM_Terminal.
- **Near Future (~2030s):** LEO_SatelliteGateway, QuantumRepeater, MeshDroneRelay, UnderwaterDataCenter, NeuromorphicEdgeNode, TerahertzRelay.

### 1.2 Edge Types (~15 era-specific types, flat enum)
- **Telegraph Era:** TelegraphWire, SubseaTelegraphCable.
- **Telephone Era:** CopperTrunkLine, LongDistanceCopper.
- **Early Digital Era:** CoaxialCable, MicrowaveLink, EarlySatelliteLink.
- **Internet Era:** FiberLocal, FiberRegional, FiberNational, SubseaFiberCable.
- **Modern Era:** FiberMetro, FiberLongHaul, DWDM_Backbone, SatelliteLEOLink.
- **Near Future:** QuantumFiberLink, TerahertzBeam, LaserInterSatelliteLink.

### 1.3 Visualization
- **Node sizing:** Nodes are rendered at sizes proportional to their network tier. Access-level nodes are smallest; Global Backbone nodes are largest.
- **Edge thickness:** Edge rendering thickness scales by type — local copper is thin, backbone fiber is thick, subsea cables are the most visually prominent.
- **Backbone routes:** Core and backbone routes are drawn with enhanced visual weight (glow, thicker stroke, brighter color) to make the network spine immediately readable.
- **Coverage icons:** SVG icon sprites for all node types, designed to remain readable and distinct at all zoom levels. Icons use silhouette style with era-appropriate visual cues.

### 1.4 Build Menu
- Build menu categorizes available node and edge types by network tier:
  - **Access:** End-user connection infrastructure (small cells, telephone poles, cable huts, etc.).
  - **Aggregation:** Traffic collection and local distribution (central offices, coax hubs, fiber POPs, etc.).
  - **Core:** Regional and national backbone switching (digital switches, DWDM terminals, IXPs, etc.).
  - **Backbone:** Long-haul and continental transport (fiber long-haul, microwave relays, subsea cables, etc.).
  - **Global:** Intercontinental and space-based infrastructure (satellite gateways, subsea landing stations, quantum repeaters, etc.).
- Items within each category are filtered by the current era and unlocked technologies.

## 2. Hierarchical Network Graph
- **Levels:** Local → Regional → National → Continental → Global Backbone.
- **Routing:** Event-driven recalculation triggered by infrastructure changes or disasters.
- **Shared Ownership:** Multi-player nodes/edges with shared revenue and upgrade voting.

## 3. Construction & Maintenance
- Construction timers scaled by terrain, edge type, and era complexity.
- AI manages preventive maintenance schedules for AI-controlled corporations.
- AI can decide upgrade, abandonment, or auction of assets.

### 3.1 Maintenance Priority Tiers
- Player-controlled maintenance priority system with four tiers per node/edge:
  - **Critical:** Highest priority. Maintenance teams address these first. Budget allocation: up to 40% of maintenance funds.
  - **Standard:** Normal priority. Addressed after Critical items. Budget allocation: up to 35% of maintenance funds.
  - **Low:** Reduced priority. Addressed when higher-priority work is complete. Budget allocation: up to 15% of maintenance funds.
  - **Deferred:** Maintenance postponed. No budget allocated. Reliability degrades over time; risk of failure increases.
- **Maintenance teams:** Hiring dedicated maintenance teams amplifies effectiveness. Each team increases the maintenance throughput (number of nodes/edges serviced per tick) and reduces degradation rates for assets under their coverage area.
- **Budget allocation:** Player sets overall maintenance budget; funds are distributed across tiers according to the allocation percentages. Underfunded tiers see slower repair and faster degradation.

### 3.2 Repair System
- **Damage alerts:** When nodes or edges are damaged (by disaster, neglect, or sabotage), alerts appear in the notification feed with severity, location, and affected capacity.
- **Repair panel:** Dedicated repair panel lists all damaged assets with current condition, estimated repair cost, and impact on network performance.
- **Repair options:**
  - **Emergency Repair:** 60% of the asset's full replacement cost. Repair is instant (completes within the current tick). Use for critical backbone or revenue-critical infrastructure.
  - **Normal Repair:** 20% of the asset's full replacement cost. Repair takes time (multiple ticks scaled by damage severity and terrain). Suitable for non-urgent or redundant infrastructure.
- Insurance coverage (if active) offsets repair costs according to policy terms.

## 4. Disaster & Risk
- **Natural Disasters:** Storms, earthquakes, floods, landslides.
- **Political Risk:** Regulatory changes, nationalization, civil unrest.
- **Effects:** Capacity degradation, latency increase, partial/full destruction.

## 5. Player Interaction & Cooperative Ownership
- Cooperative ownership of infrastructure with fractional ownership shares.
- Sabotage and legal actions handled through AI arbitration.
- Auctions automatically managed by AI when assets are abandoned or under-maintained.

### 5.1 Co-Ownership UI Panel
- Full co-ownership management panel for jointly held assets:
  - **View co-owners:** Displays all co-owners with their ownership percentage, contribution history, and revenue share.
  - **Vote on upgrades:** Co-owners vote on proposed upgrades. Voting weight is proportional to ownership share. Majority (>50% of shares) required to approve. Upgrade costs are split by ownership percentage.
  - **Manage buyouts:** Any co-owner can propose to buy out another co-owner's share at a negotiated price. The target co-owner can accept, reject, or counter-offer. Hostile buyout attempts require regulatory approval if ownership exceeds regional thresholds.

## 6. Simulation Mechanics
- Event-driven updates to routing graphs.
- Hex-based land parcels for infrastructure placement.
- Terrain and geography influence costs, latency, and disaster probabilities.
- Infrastructure performance feeds into economic and corporate modules.

