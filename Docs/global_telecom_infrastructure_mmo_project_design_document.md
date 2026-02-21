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

## 4.1 Node Types (~33 types across 6 eras)

**Telegraph Era (~1850s):**
- Telegraph office
- Relay station
- Cable landing house

**Telephone Era (~1900s):**
- Manual switchboard exchange
- Automatic exchange
- Toll exchange (long-distance)
- Telephone pole junction
- Cable landing station (early subsea)

**Early Digital Era (~1970s):**
- Digital switching center
- Microwave relay tower
- Coaxial distribution hub
- Satellite ground station (early)
- Trunk exchange

**Internet Era (~1990s):**
- Central office (CO/DSLAM)
- Fiber distribution hub
- Internet exchange point (IXP)
- Data center (small)
- Subsea cable landing station
- POP (Point of Presence)

**Modern Era (~2010s):**
- Macro cell tower (4G/5G)
- Small cell node
- Hyperscale data center
- Edge data center
- Fiber aggregation node
- Satellite ground station (modern)
- Content delivery node (CDN)

**Near Future Era (~2030s):**
- 6G mesh node
- Orbital ground relay
- Quantum repeater station
- AI-managed edge cluster
- Subsea cable landing station (high-capacity)
- Autonomous micro data center

## 4.2 Edge Types (~15 types across 6 eras)

**Telegraph Era (~1850s):**
- Overland telegraph line
- Undersea telegraph cable

**Telephone Era (~1900s):**
- Copper trunk line
- Early subsea telephone cable

**Early Digital Era (~1970s):**
- Coaxial trunk
- Microwave link
- Early satellite link

**Internet Era (~1990s):**
- Local fiber
- Regional fiber backbone
- Subsea fiber cable
- Satellite link (GEO)

**Modern Era (~2010s):**
- Metro fiber ring
- Long-haul DWDM fiber
- 5G wireless backhaul
- LEO satellite link

**Near Future Era (~2030s):**
- Quantum-secured fiber
- Orbital laser link
- 6G mesh link

Each edge contains:
- Capacity
- Latency weight
- Reliability rating
- Maintenance cost
- Construction time
- Terrain risk multiplier

---

# 4A. ERA SYSTEM & TECH TREE

## 4A.1 Eras
Six historical-to-future eras define the world's cosmetic milestone and starting conditions:
- Telegraph (~1850s)
- Telephone (~1900s)
- Early Digital (~1970s)
- Internet (~1990s)
- Modern (~2010s)
- Near Future (~2030s)

The player picks a starting era when creating a game. The world era advances as a cosmetic milestone when a critical mass of global infrastructure reaches a threshold.

## 4A.2 Research is NOT Era-Gated
Research is a free tech tree. Players can research any technology they can afford, regardless of the current world era. There are no era gates or locks on research. A player starting in the Telegraph era can aggressively push toward Internet-era technologies if they invest heavily in research. The world era label is a global cosmetic indicator, not a content gate.

## 4A.3 Patent & Licensing System
Technology is a primary economic commodity. Completed research can be patented, licensed, or open-sourced.

**Patent Enforcement (Hard Block):**
- Patented technology cannot be used by non-holders without a license
- Attempting to build patent-protected infrastructure without a license is rejected
- Patent holder sets licensing terms

**License Types:**
- **Permanent:** One-time payment, perpetual access
- **Royalty:** Per-tick recurring payment
- **Per-Unit:** Payment per node built using the technology
- **Lease:** Temporary access for a fixed duration

**Independent Research Workaround:**
- **150% cost:** Corporation gains access to the patented tech but cannot patent their version
- **200% cost:** Corporation develops an improved version (+10% performance bonus) and CAN patent the improvement

**Strategic Options:**
- Open-sourcing grants reputation bonus and accelerates adoption
- Exclusive licensing maximizes revenue from a single partner
- Broad licensing creates steady royalty income streams

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

**Connectivity-GDP Loop Dampening:**
The positive feedback loop between connectivity and GDP growth is governed by 4 dampening mechanisms that prevent runaway exponential growth:
1. **Market Saturation:** As connectivity penetration approaches 100% in a region, marginal GDP gains diminish sharply. The growth curve is logarithmic, not linear.
2. **Competition Splits the Pie:** Multiple providers in a region split subscriber revenue. More competitors = smaller share per corp. Total regional revenue grows slower than the number of providers.
3. **Rising Costs:** Land acquisition costs and labor costs increase as a region develops. Urban density drives up lease rates. Skilled labor becomes scarce and expensive in booming regions.
4. **Regulatory Intervention:** When any single corporation exceeds 60% market share in a region, the government triggers anti-monopoly actions: forced infrastructure sharing, price caps, or license revocation threats. This mechanically limits how much one player can extract from a region.

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

## 8.2 Alliance System
Corporations may form formal alliances with concrete mechanical effects:

**Formation & Limits:**
- Maximum 3 corporations per alliance
- All members must mutually agree to admit a new member
- Trust score tracked per pair of alliance members (starts at 50, max 100, decays on hostile actions)

**Routing & Revenue:**
- Free routing between allied networks (no transit fees)
- Revenue from traffic routed through allied infrastructure is split proportionally based on hop count (each member earns based on the number of hops their network contributes to the route)

**Joint Operations:**
- Joint spectrum bidding: alliance members pool funds to bid on spectrum auctions as a bloc
- Mutual defense: if an allied corp's infrastructure is sabotaged or attacked, allies receive an alert and can contribute repair resources at reduced cost

**Dissolution:**
- Any member can initiate dissolution
- 30-tick transition period during which routing agreements phase out gradually
- Trust score drops to 0 between departing members
- Transit fees resume immediately after the transition period

## 8.3 Legal System
Corporations can initiate lawsuits against competitors. Legal actions are simulation entities resolved over time.

**Lawsuit Types:**
- **Sabotage Claim:** Filed when infrastructure damage is suspected to be caused by a competitor. Requires evidence (proximity of competitor assets, recent espionage activity).
- **Patent Infringement:** Filed when a competitor uses technology covered by the claimant's patents or licenses. Requires active patent ownership.
- **Ownership Dispute:** Filed over contested land parcels, spectrum rights, or shared infrastructure stakes. Common after mergers or alliance dissolutions.
- **Regulatory Complaint:** Filed to trigger government investigation of a competitor's monopolistic behavior, regulatory violations, or SLA breaches.

**Resolution:**
- Lawsuits resolve over 20-50 ticks depending on complexity and jurisdiction
- Outcome influenced by Legal team strength, evidence quality, and regional regulatory strictness
- Possible outcomes: financial damages awarded, forced asset transfer, injunction (competitor must cease activity), case dismissed

**Requirements:**
- Filing corporation must have a Legal team (legal department or contracted legal staff)
- Filing costs scale with lawsuit type and jurisdiction
- Frivolous lawsuits (low evidence score) risk counter-suit penalties

**UX:**
- Auto-suggest system: when suspicious events occur (infrastructure damage near competitor, patent overlap detected), the UI suggests potential legal actions with estimated success probability

## 8.4 Player Pricing
Players set regional pricing tiers for their services in each region they operate:

**Pricing Tiers:**
- **Budget:** Low price, high churn, attracts price-sensitive consumer segment. Lower revenue per subscriber.
- **Standard:** Market-rate pricing. Balanced churn and revenue.
- **Premium:** High price, low churn, attracts business and enterprise segments. Higher revenue per subscriber but smaller addressable market.
- **Custom:** Player sets an exact price point. Demand response calculated dynamically based on regional income levels, competition, and service quality.

Pricing affects:
- Subscriber acquisition rate
- Churn rate
- Revenue per subscriber
- Market share growth
- Brand perception (affects contract negotiations and enterprise demand)

Players can change pricing per region at any time. Price changes take effect over 5 ticks (gradual market response).

## 8.5 Fog of War
Full fog of war applies to all competitor information. Players cannot see other corporations' infrastructure by default.

**Visibility Rules:**
- Geography is always visible: terrain, regions, cities, political borders, population data
- Infrastructure is hidden: all competitor nodes, edges, capacity, and financial data are invisible unless revealed through intel

**Intel Levels:**
- **None:** Default state. No information about competitor infrastructure in a region.
- **Basic:** Reveals node locations and edge connections. No capacity, financial, or operational details. Obtained through general espionage or alliance intel sharing.
- **Full:** Reveals complete infrastructure details: node types, edge capacities, utilization rates, maintenance state, and estimated revenue. Requires dedicated espionage investment.

**Intel Mechanics:**
- Intel is gathered through espionage actions (requires dedicated espionage staff or contracted intelligence)
- Intel decays over 50 ticks. After decay, the region reverts to the previous intel level unless refreshed.
- Alliance members automatically share Basic intel on all regions where any alliance member has coverage
- Espionage can be detected, leading to diplomatic penalties or legal action (see Legal System)

**Multiplayer Application:**
- Fog of war applies fully in player-vs-player multiplayer. Each player sees only their own infrastructure and whatever intel they have gathered on competitors.
- The server tracks full world state; clients receive only the information their corporation has access to.

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
- Insurance systems (per-node premium at 2% of construction cost, 60% payout on disaster damage)

## 9.4 Maintenance Priority System
Each infrastructure asset can be assigned a maintenance priority tier:
- **Critical:** Immediate repair, highest resource allocation, maximum uptime target
- **Standard:** Normal maintenance schedule, balanced cost/uptime
- **Low:** Deferred maintenance, reduced cost, acceptable degradation
- **Deferred:** Minimal maintenance, lowest cost, risk of failure

Auto-repair toggle per asset. Preventive maintenance at higher tiers reduces disaster damage severity.

---

# 10. GRANTS & DEVELOPMENT PROGRAMS

## 10.1 Regional Grant Opportunities
Governments in underserved or developing regions periodically issue grant opportunities. These are simulation-generated events tied to regional economic conditions.

**Grant Structure:**
- **Build Requirements:** Each grant specifies infrastructure that must be built (e.g., "Connect 3 rural towns in Region X to the regional backbone" or "Deploy 5 access nodes with minimum 100 Mbps capacity").
- **Deadline:** Grants have a tick deadline (typically 50-200 ticks depending on scope). Failure to meet the deadline forfeits the grant and may incur a reputation penalty.
- **Competitive Bidding:** Multiple corporations can bid on the same grant. The government evaluates bids based on price, estimated completion time, and corporate reputation. Lowest bid does not always win — reliability and track record factor in.

**Reward Types:**
- **Cash Payment:** Direct lump-sum payment upon completion.
- **Tax Reduction:** Reduced tax rate in the grant region for a set number of ticks (typically 50-100 ticks).
- **Exclusive License:** Temporary exclusive operating license in the region, blocking competitors from building for a set period.
- **Combined:** Some grants offer a mix of the above.

## 10.2 Long-Term Development Effects
Successful grant completion produces:
- Increased regional GDP
- Improved political stability
- Increased future telecom demand
- Improved corporate reputation in the region (affects future grant eligibility and contract negotiations)

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
- **Pure thin client in multiplayer:** Clients do NOT run WASM simulation ticks in MP mode. The server is the sole simulation authority. Clients send commands via WebSocket and receive state deltas. No client-side prediction or local tick execution.
- 250 concurrent players per world
- WebSocket communication (MessagePack binary or JSON debug)
- Fully offline single-player via WASM in browser (same sim code, no network needed — WASM module IS the server in SP)
- **Fog of war in MP:** Each client receives only the world state their corporation has intel on. Competitor infrastructure is filtered server-side before transmission. See Section 8.5 for full fog of war rules.

## 12.1 Lobby & World Browser
Multiplayer includes a full lobby system:
- **World Browser:** Search and filter available persistent worlds by era, player count, region focus, and ruleset
- **World Creation:** Players with sufficient reputation or subscription can create new persistent worlds with custom settings (starting era, map type, player cap, speed, ruleset)
- **Leaderboard:** Per-world and global leaderboards tracking corporation net worth, infrastructure coverage, reliability rating, and market share
- **Server Status:** Real-time display of player count, world age, and economic health indicators

**Hosting:** Currently Fly.io + Vercel. Production target: Hetzner + Cloudflare Workers.

Cloud services (Cloudflare Workers) handle:
- Authentication
- Logging
- Market APIs
- Account persistence

Simulation server (Hetzner, production target) handles:
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

