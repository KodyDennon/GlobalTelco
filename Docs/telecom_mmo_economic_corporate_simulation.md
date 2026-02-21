# Document 2: Economic & Corporate Simulation Specification

**Purpose:** Defines all economic, corporate, and market simulation systems for AI-agentic management of corporations and regions.

---

## 1. Regional Economy Model
- Tracks population, GDP proxy, technology adoption index, political stability, data demand growth, business density, urbanization.
- Connectivity affects GDP growth, regional development, migration, and demand.

## 2. Corporate Finance
- Balance sheet: assets, liabilities, cash reserves.
- Income statement: revenue from bandwidth, transit fees, government grants, peering, patent/license royalties.
- Debt instruments: loans, bonds, default risk.
- Credit and risk rating influencing borrowing and market perception.
- Failure modes: insolvency, bankruptcy, asset liquidation, hostile takeovers.

### 2.1 Revenue Sources
- **Bandwidth & Transit Fees:** Primary revenue from selling network capacity and transit services.
- **Government Grants:** Competitive grant awards for regional infrastructure development (see Section 3.1).
- **Peering & Interconnection:** Revenue from peering agreements and interconnection fees.
- **Patent & License Revenue:** Major revenue stream from licensing patented technologies to other corporations. Revenue scales with the number of active licensees and license type (royalty-based licenses generate recurring income per tick; per-unit licenses scale with licensee usage). Corporations with strong R&D pipelines can become net-positive on licensing alone.

### 2.2 Insurance
- Per-node insurance policies protect against disaster damage, covering repair costs up to policy limits.
- Full insurance management panel: purchase or cancel policies per individual node, bulk purchase/cancel actions across selected nodes or entire regions.
- Premium costs scale with node value, disaster risk of the hex, and historical claim frequency.
- Uninsured nodes require full out-of-pocket repair costs when damaged.

## 3. Market Mechanics
- Dynamic competition management: legal disputes, sabotage, mergers and acquisitions.

### 3.1 Pricing
- Player-controlled regional pricing with four tiers: **Budget**, **Standard**, **Premium**, **Custom**.
- Each region can be assigned an independent pricing tier. Custom allows manual price-per-unit entry.
- **Price elasticity by region wealth:** Wealthy regions tolerate Premium pricing with low churn; low-GDP regions see sharp subscriber loss above Budget. Elasticity curves are calibrated per region based on GDP proxy and competition density.
- SLA performance modifies willingness-to-pay: high reliability supports premium pricing; frequent outages erode it.

### 3.2 Government Grants
- Regional grant opportunities appear dynamically based on government priorities (rural connectivity, disaster recovery, modernization).
- **Competitive bidding:** Multiple corporations can bid on the same grant. Bids specify proposed coverage, timeline, and requested funding. Lowest cost-per-coverage bid wins, with tie-breaking on track record and existing regional presence.
- **Rewards:** Grant awards provide upfront capital injection plus ongoing subsidies during the build-out period. Completed grant projects boost regional reputation and may unlock follow-on contracts.
- **Penalties:** Failure to meet grant milestones triggers clawback of funds and reputation damage.

### 3.3 Patent & Licensing System
- Technologies researched through the tech tree can be patented upon completion.
- **Hard block enforcement:** Corporations cannot build or deploy infrastructure requiring a patented technology unless they hold a valid license from the patent owner.
- **Independent research workaround:** A corporation may invest in independent R&D to bypass a patent. At 150% of the original research cost, they gain equivalent access (no patent of their own). At 200% of the original research cost, they develop an improved version that is independently patentable.
- **License types:**
  - **Permanent:** One-time fee, unlimited use, never expires.
  - **Royalty:** Ongoing percentage of revenue generated using the licensed tech, paid per tick.
  - **PerUnit:** Fixed fee per deployed node/edge using the licensed tech.
  - **Lease:** Time-limited access for a fixed duration; must be renewed or tech becomes inaccessible.
- Patent owners set license terms and can revoke or refuse licenses (subject to regulatory intervention at high market share).
- License/patent revenue feeds directly into the patent holder's income statement.

### 3.4 Dampening Mechanisms
- **Market Saturation:** As regional subscriber penetration approaches 100%, new subscriber acquisition slows exponentially. Growth-dependent strategies plateau in mature markets.
- **Competition Splits the Pie:** Revenue per subscriber decreases as more competitors enter a region. Price wars compress margins; monopoly rents only exist briefly before AI or player competitors respond.
- **Rising Costs:** Land acquisition and labor costs scale with regional GDP and urbanization. Expanding in wealthy, developed regions is significantly more expensive than in emerging markets. Cost inflation applies to construction, maintenance, and staffing.
- **Regulatory Intervention:** When any corporation exceeds 60% market share in a region, the regional government triggers anti-monopoly proceedings. Effects include forced infrastructure sharing, price caps, fines, and in extreme cases, forced divestiture of assets. Severity escalates with market share above the threshold.

## 4. Sandbox Objectives
- AI evaluates strategies for expansion, ethical growth, aggressive competition, and resilience.
- Balances risk and reward dynamically.
- Supports customizable player-defined goals in the future.

## 5. Integration with Infrastructure Module
- Revenue and demand calculations rely on routing performance and infrastructure availability.
- Regional disasters and network failures directly impact corporate finances and AI decisions.

