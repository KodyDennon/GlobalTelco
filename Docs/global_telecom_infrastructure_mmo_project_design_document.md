# GLOBAL TELECOM INFRASTRUCTURE MMO

Comprehensive Project Design & Systems Architecture Document

---

# 1. OVERARCHING VISION

A unified, persistent, authoritative global simulation where players build, operate, compete, cooperate, finance, sabotage, stabilize, and expand telecom infrastructure within a 2D political map world.

This is not a "tycoon game." It is a systemic infrastructure civilization simulator.

No pillar dominates:
- Engineering realism
- Corporate finance
- Geographic land control
- Geopolitics
- Population & macroeconomics
- Disaster modeling
- Market competition

All systems must interlock and influence one another continuously.

---

# 2. CORE DESIGN PHILOSOPHY

## 2.1 Unified World Per Server
- 1 authoritative simulation per server
- 250 concurrent players cap (initial target)
- Fully persistent state
- Players can operate local-only or globally
- Multi-subsidiary corporate structures supported
- No isolated regional shards inside a server

## 2.2 Physically Grounded World
- 2D political map (Victoria 3 / Risk style) with multi-layer zoom: World → Country → Region → City
- Hex-based land parcels (geodesic grid)
- Real geographic constraints abstracted but meaningful
- Player choice: Real Earth (open data) or procedural world

Geography influences:
- Construction cost
- Maintenance cost
- Latency
- Failure probability
- Repair time
- Regulatory difficulty
- Political risk

---

# 3. WORLD STRUCTURE

## 3.1 Land Parcel System
Each parcel contains:
- Parcel ID
- Geographic coordinates
- Terrain classification
- Zoning category
- Ownership (player / government / public)
- Lease rate
- Tax rate
- Regulatory strictness
- Political stability index
- Disaster risk profile
- Labor cost modifier
- Power grid reliability modifier

Players can:
- Purchase land
- Lease land
- Lease from other players
- Use public land via permit

## 3.2 Terrain Abstraction
Not hyper-detailed meshes. Instead:
- Urban density tier
- Mountainous
- Desert
- Coastal
- Ocean depth zones (for subsea)

These apply multipliers to cost, reliability, and maintenance.

---

# 4. INFRASTRUCTURE MODEL

## 4.1 Node Types
- Access towers
- Fiber distribution hubs
- Data centers
- IXPs
- Subsea landing stations
- Satellite ground stations
- Future: power plants, substations

## 4.2 Edge Types
- Local fiber
- Regional fiber
- National backbone
- Subsea cable
- Microwave
- Satellite link

Each edge contains:
- Capacity
- Latency weight
- Reliability rating
- Maintenance cost
- Construction time
- Terrain risk multiplier

---

# 5. NETWORK GRAPH ARCHITECTURE

Hierarchical global graph:

Level 1: Local cluster
Level 2: Regional cluster
Level 3: National cluster
Level 4: Continental backbone
Level 5: Global backbone

Graph recalculation rules:
- Event-driven only
- Dirty-node invalidation
- Cluster-based recomputation
- Cached shortest-path trees
- Aggregate bandwidth modeling

NO packet-level simulation.
Only aggregate throughput and latency weighting.

---

# 6. ROUTING & LATENCY SYSTEM

When infrastructure changes:
- Affected cluster marked dirty
- Recalculate shortest paths for impacted nodes
- Update congestion state
- Recalculate SLA reliability scores

Latency influences:
- Enterprise contract value
- Financial market perception
- High-frequency traffic demand
- Competitive advantage

Congestion influences:
- Revenue degradation
- Customer churn
- Political dissatisfaction

---

# 7. ECONOMIC SYSTEM

## 7.1 Regional Economic Model
Each region tracks:
- Population
- GDP proxy
- Technology adoption index
- Political stability
- Data demand growth rate
- Business density
- Urbanization index

Connectivity affects:
- GDP growth
- Business formation
- Political stability
- Migration patterns

## 7.2 Revenue Calculation
Revenue derived from:
- Bandwidth delivered
- Latency tier
- Reliability SLA
- Regional demand
- Transit agreements
- Peering contracts

## 7.3 Corporate Finance
Each player corporation has:
- Balance sheet
- Income statement
- Debt ledger
- Credit rating
- Liquidity rating
- Equity valuation

Financial instruments:
- Bonds
- Bank loans
- Equity issuance (optional later)
- Grants
- Development funding

Failure conditions:
- Insolvency
- Bankruptcy
- Asset liquidation
- Hostile takeover

---

# 8. COMPETITION SYSTEM

Players may:
- Negotiate peering
- Charge transit fees
- Refuse interconnection
- Undercut pricing
- Lease land to competitors
- Acquire competitors
- Perform basic sabotage actions (limited, regulated)

Contracts are simulation entities with:
- Duration
- Pricing terms
- Capacity guarantees
- Breach penalties

---

# 9. DISASTER & RISK MODEL

## 9.1 Natural Disasters
- Storm systems
- Earthquakes
- Flooding
- Landslides

## 9.2 Political Risk
- Regulatory shifts
- Nationalization events
- Civil unrest
- Sanctions

## 9.3 Infrastructure Effects
- Capacity degradation
- Edge destruction
- Latency increase
- Repair time requirement
- Regional GDP impact

Resilience systems:
- Redundant routing
- Backup power
- Preventive maintenance budgets
- Insurance systems (future)

---

# 10. GRANTS & DEVELOPMENT PROGRAMS

Underserved regions may:
- Offer subsidies
- Provide tax incentives
- Provide infrastructure grants
- Offer development contracts

Long-term effects:
- Increased regional GDP
- Improved political stability
- Increased future telecom demand

Players can choose ethical expansion or exploitative dominance.

---

# 11. TIME & SIMULATION FLOW

Economic tick: 3–5 seconds

Each tick:
- Demand recalculation
- Revenue computation
- Congestion penalty update
- Corporate financial update
- Stock valuation update

Routing recalculation:
- Triggered only by infrastructure changes or destruction

Construction:
- Timed build cycles
- Variable by terrain & region

---

# 12. MULTIPLAYER ARCHITECTURE

- Dedicated authoritative simulation server (Rust native binary)
- Deterministic simulation kernel (Rust ECS — same code compiles to WASM for browser SP and native for MP servers)
- Rendering client separated from simulation logic (Svelte + Three.js frontend)
- Clients are thin, server authoritative
- 250 concurrent players per world
- WebSocket communication (MessagePack binary or JSON debug)
- Fully offline single-player via WASM in browser (same sim code, no network needed)

Cloud services (Cloudflare Workers) handle:
- Authentication
- Logging
- Market APIs
- Account persistence

Simulation server (Hetzner) handles:
- Global graph
- Economic tick
- Routing computation
- Disaster modeling
- World persistence (PostgreSQL)

---

# 13. MODULAR INDUSTRY EXPANSION

Core abstraction model:

Node
Edge
Resource
Dependency
Throughput
Risk
Ownership
Jurisdiction

Telecom = first industry module
Energy grid = future module
Water systems = future module
Transportation = future module

The simulation core is industry-agnostic.

---

# 14. PLAYER EXPERIENCE GOALS

A player session should feel like:
- Managing real infrastructure
- Responding to global risk
- Negotiating market power
- Building resilient systems
- Making strategic financial decisions
- Influencing regional development

Possible emotional outcomes:
- Pride in resilience
- Tension from over-leverage
- Satisfaction from low-latency dominance
- Moral reward from development expansion
- Shock from disaster impact

---

# 15. LONG-TERM END STATE

A living, persistent infrastructure civilization sandbox where:
- Connectivity drives growth
- Growth drives demand
- Demand drives competition
- Competition drives innovation and conflict
- Resilience determines survival

No single system is dominant.
All systems are interdependent.

This is a distributed infrastructure simulation MMO, not a simple management game.

---

END OF DOCUMENT

