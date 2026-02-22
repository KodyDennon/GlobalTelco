# Document 3: Multiplayer & Governance Simulation Specification

**Purpose:** Defines deterministic AI-handled multiplayer logistics, alliances, land ownership, conflict, and sandbox governance systems.

---

## 1. Player Identity & Cooperation
- Multi-player corporations with shared voting, revenue, and joint infrastructure decisions.
- Multi-player ownership of nodes and edges with revenue splits and shared decision-making.

### Alliance System
- **Formation:** Any corporation may propose an alliance to up to 2 other corporations (maximum 3 corps per alliance). All invited parties must accept. Formation requires a minimum trust score of 50 between all parties.
- **Trust Score:** Ranges 0–100. Starts at 50 on formation. Increases +1 per tick of active cooperation, +5 for fulfilling shared obligations. Decreases -10 for undercutting an ally's pricing, -20 for sabotage against an ally, -5 for rejecting a joint bid. Alliance dissolves automatically if any pair's trust drops below 20.
- **Free Routing:** Alliance members grant each other free transit across their networks. Revenue from traffic routed through allied infrastructure is split proportionally based on hop count (number of nodes traversed per member's network).
- **Joint Spectrum Bidding:** Alliance members may pool funds to bid on spectrum auctions as a single entity. Winning spectrum is co-owned with equal access rights. Costs split proportionally to contribution.
- **Mutual Defense:** If an allied corporation is targeted by sabotage or a hostile lawsuit, other alliance members automatically contribute +20% to the target's legal defense score and +10% to infrastructure repair speed.
- **Shared Research:** Alliance members may share completed research at 50% of the original license cost (paid to the researching member). Shared research unlocks immediately without additional research time.
- **Dissolution:** Any member may initiate dissolution. A 30-tick transition period begins during which free routing rates gradually increase to market rates, shared research licenses remain valid but no new sharing occurs, and joint spectrum ownership converts to proportional individual ownership based on original contribution. After 30 ticks, the alliance is fully dissolved.

## 2. Spatial & Terrain System
- Infrastructure placed freely at exact map coordinates (no grid snapping). Invisible hex cell grid manages terrain classification, coverage, demand, disaster impact, and influence.
- AI agents process auction outcomes, compliance, and infrastructure placement decisions.

## 3. Conflict & Sabotage
- Operational sabotage: network disruption, price undercutting, infrastructure delays, all resolved via rule-based mechanics.
- Multi-player disputes resolved automatically following defined game rules.

### Legal System
- **Filing Requirements:** Filing a lawsuit requires a Legal team (minimum 1 employee assigned to Legal department) and a filing cost that scales with lawsuit type. Corporations without a Legal team cannot file or respond effectively (automatic -30% to defense score).
- **Lawsuit Types:**
  - **Sabotage Claim:** Filed when a corporation suspects deliberate infrastructure interference. Filing cost: $50,000. Resolution: 20–30 ticks. Requires evidence (detected sabotage event within last 100 ticks targeting the plaintiff's assets).
  - **Patent Infringement:** Filed when a corporation uses technology covered by another's patent without a license. Filing cost: $100,000. Resolution: 30–40 ticks. Requires the defendant to be using the patented technology without a valid license.
  - **Ownership Dispute:** Filed over contested land parcels, shared infrastructure, or spectrum rights. Filing cost: $75,000. Resolution: 25–35 ticks. Applies when multiple parties claim the same asset or when co-ownership terms are violated.
  - **Regulatory Complaint:** Filed to trigger a government investigation into anti-competitive behavior. Filing cost: $25,000. Resolution: 40–50 ticks. Can target any corporation; outcome influenced by the target's actual market share and behavior history.
- **Resolution & Outcomes:**
  - **Damages:** Losing party pays monetary damages to the winner (1.5x–3x the filing cost, scaled by severity).
  - **Forced Licensing:** Court orders the patent holder to license technology to the plaintiff at regulated rates (applies to Patent Infringement).
  - **Asset Forfeiture:** Losing party forfeits contested assets to the winner (applies to Ownership Dispute).
  - **Regulatory Action:** Government imposes restrictions on the target (applies to Regulatory Complaint — see Regulatory Intervention below).
- **Arbitration:** Co-ownership disputes and contract disagreements use mandatory arbitration instead of lawsuits. Arbitration is faster (10–15 ticks), cheaper (filing cost halved), and outcomes are binding. Applies automatically when the dispute involves co-owned infrastructure or active contracts between the parties.
- **Auto-Suggest UX:** The UI monitors game events and surfaces lawsuit opportunities to the player as advisor notifications (e.g., "Detected sabotage on your tower in Region X — file a Sabotage Claim?" or "Competitor Y is using your patented tech without a license — file for Patent Infringement?"). Players can dismiss or act on suggestions.

## 4. Sandbox Governance & Political Influence
- Lobbying and regulatory influence tracked numerically with deterministic algorithms.
- Global events, disasters, economic shifts, and political instability simulated without external AI.

### Government Grants
- **Regional Opportunities:** Governments periodically post grant opportunities for underserved regions where connectivity is below a threshold (e.g., < 30% coverage). Grants specify a target region, required coverage level, deadline (in ticks), and funding amount.
- **Competitive Bidding:** All corporations (player and AI) may bid on available grants. Bids specify proposed coverage plan, timeline, and requested funding (up to the grant maximum). The government selects the bid with the best coverage-to-cost ratio, with ties broken by existing regional presence and reputation score.
- **AI Competition:** AI corporations evaluate and bid on grants according to their archetype strategy. Grants are not guaranteed to players — AI corps actively compete and may win.
- **Fulfillment:** Winning corporation must meet the grant's coverage target by the deadline. Funding is disbursed in installments: 30% upfront, 40% at midpoint verification, 30% on completion. Failure to meet targets results in clawback of disbursed funds and a reputation penalty.

### Regulatory Intervention
- **Anti-Monopoly Threshold:** When any corporation exceeds 60% market share in a region, regulatory intervention triggers automatically.
- **Price Caps:** The government imposes maximum pricing in the affected region, capping the dominant corporation's rates at 110% of the regional average.
- **Forced Infrastructure Sharing:** The dominant corporation must allow competitors to use its infrastructure at regulated wholesale rates (cost + 15% margin), enabling new market entrants.
- **Blocked Acquisitions:** The dominant corporation cannot acquire additional companies or assets in the affected region until market share drops below 50%.
- **Duration:** Regulatory restrictions persist until the corporation's market share in the region drops below 50%. Restrictions are evaluated and updated every 10 ticks.

## 5. Fog of War
- **Default State:** All competitor corporations (AI and player) are hidden by default. Players cannot see competitor infrastructure, finances, research, or strategic actions until intel is obtained.
- **Geography Visible:** The physical map (terrain, regions, cities, zoning) is always fully visible. Only competitor-owned assets and corporate data are hidden.
- **Intel Levels:**
  - **None:** Competitor exists on the map but no details are visible. Infrastructure, finances, and strategy are completely unknown. Represented as a named entity with no data.
  - **Basic:** Reveals competitor's infrastructure locations (nodes and edges visible on map), approximate market share (rounded to nearest 10%), number of employees (approximate), and general financial health (Healthy / Struggling / Critical). Does not reveal exact finances, research, pricing, or strategy.
  - **Full:** Reveals all competitor data — exact infrastructure details (capacity, utilization, maintenance state), full financial statements, research progress, pricing per region, employee details, contract terms, and active strategy mode.
- **Obtaining Intel:**
  - **Espionage:** Players assign employees to an Espionage team. Each espionage action targets a specific competitor and costs funds + time. Success chance based on team size, target's counter-intelligence, and random factor. Successful espionage grants Basic intel (first success) or upgrades to Full intel (second success on same target).
  - **Market Presence:** Operating in the same region as a competitor automatically grants Basic intel on that competitor's assets in that region only (not company-wide).
  - **Alliance Sharing:** Alliance members automatically share Basic intel on all competitors they have intel on. Full intel is not shared automatically but can be traded.
- **Intel Decay:** Intel decays over time. Full intel degrades to Basic after 50 ticks without refresh. Basic intel degrades to None after 50 additional ticks. Players must maintain espionage operations or regional presence to keep intel current.
- **Counter-Intelligence:** Corporations can assign employees to counter-intelligence, which reduces enemy espionage success chance by 10% per assigned employee (up to 50% reduction).

## 6. Persistent World Management
- Authoritative simulation per server (250 concurrent players max).
- Tick-based updates: economic, routing, disasters.
- Deterministic AI ensures consistency of shared assets, multi-owner nodes/edges, and player interactions.
- Agents can enforce alliance agreements, multi-owner infrastructure management, and resource competition fairly.

### Lobby System
- **Persistent Worlds:** The lobby displays all active persistent worlds with real-time status. Worlds run 24/7 on dedicated servers and persist between player sessions.
- **World Browser:** Filterable and searchable list of available worlds. Filter by: era (Telegraph through Near Future), player count (current / max), world type (Real Earth / Procedural), region focus, game speed, difficulty, and open slots.
- **Search:** Text search by world name, host name, or tag. Results update in real-time as filters change.
- **World Creation:** Players with sufficient account standing can create new persistent worlds. Configuration includes: world name, world type (Real Earth / Procedural), starting era, max players (up to 250), game speed multiplier, difficulty settings, and optional password protection.
- **Leaderboard:** Each world maintains a leaderboard ranking corporations by configurable metrics: revenue, market share, network coverage, customer count, and overall score (weighted composite). Leaderboards are visible from the lobby before joining a world. Global cross-world leaderboards track lifetime achievements.

### Hosting
- Currently Fly.io + Vercel. Production target: Hetzner + Cloudflare Workers.

## 7. Integration with Core Simulation
- Player actions feed into infrastructure and economic modules.
- AI agents handle cooperative, aggressive, or balanced corporate behaviors based on difficulty and player choices.
- Deterministic algorithms simulate negotiation, conflict, and alliance outcomes in persistent world context.

