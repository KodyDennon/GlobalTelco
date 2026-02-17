# GlobalTelco: Game Design Decisions

Concrete design decisions for the GlobalTelco infrastructure empire builder. This is the definitive reference for all gameplay, technical, and design choices.

---

## 1. Core Identity

- **Genre:** 2D infrastructure empire builder — mix of city builder + tycoon/business sim + grand strategy.
- **Core fantasy:** Watch your company grow from a local ISP to a global telecom empire.
- **Inspiration:** Victoria 3 (deep economy), OpenTTD (building routes), Dwarf Fortress (management abstraction at scale), Factorio (interconnected systems).
- **Platform:** Web-based (browser playable). Also available as downloadable desktop app (Electron/Tauri wrapper).
- **Perspective:** Full 2D, top-down political map (like Risk / Victoria 3). Multi-layer zoom: World → Country → Region → City.
- **Time system:** Real-time with pause. Paradox-style speed controls (pause, 1x, 2x, 4x, 8x).
- **Target audience:** Strategy/sim enthusiasts who enjoy deep systems. Not casual.

---

## 2. World & Map

- **World choice:** Player picks at game start — play on **real Earth** (using open data) OR generate a **procedural world**.
- **Real Earth data sources:**
  - Geography: OpenStreetMap (borders, cities, terrain)
  - Economics: World Bank / UN open datasets (GDP, population, development indicators)
  - The simulation changes everything dynamically — infrastructure drives city growth, migration, economic shifts
- **Procedural worlds:** Generated fictional continents, countries, cities. Different map each playthrough.
- **Map rendering:** Political map style with colored regions. Infrastructure overlays as network diagrams — lines connecting nodes, icons for facilities.
- **Multi-layer zoom:**
  - **World level:** See continents, countries as colored regions. Major infrastructure routes visible.
  - **Country level:** See regions/states, major cities. Regional infrastructure networks visible.
  - **Region level:** See cities, towns. Individual infrastructure assets visible.
  - **City level:** See neighborhoods, individual tower placements, fiber routes. Full detail.

---

## 3. Era Progression & Infrastructure

- **Starting era:** Player picks starting era at game setup:
  - **Telegraph Era (~1850s):** Copper wire, telegraph stations, manual relay
  - **Telephone Era (~1900s):** Telephone exchanges, copper trunk lines
  - **Early Digital (~1970s):** Microwave links, early fiber, satellite ground stations
  - **Internet Era (~1990s):** Dial-up, DSL, early broadband, data centers
  - **Modern Era (~2010s):** 4G/5G, FTTH fiber, cloud data centers, subsea cables
  - **Near Future (~2030s):** 6G, LEO satellite constellations, quantum networking
- **Infrastructure types (full telecom stack):**
  - Access Towers (cell towers, radio masts)
  - Fiber Distribution Hubs (fiber optic routing/switching)
  - Data Centers (compute, storage, cloud)
  - Internet Exchange Points (IXPs — peering locations)
  - Subsea Landing Stations (submarine cable terminals)
  - Satellite Ground Stations
  - New types unlock with era progression and tech research
- **Multi-industry extensibility:** Architecture designed to add energy, water, transport industries in future updates. Infrastructure types are data-driven, not hardcoded.

---

## 4. Player Identity & Progression

- **Player arc:**
  1. **CEO** — Start running one small telecom company. Hands-on: place every tower, hire every worker.
  2. **Holding Company** — Create regional subsidiaries (e.g., "TelcoCorp UK", "TelcoCorp Asia"). Each has own financials, workforce, operations. You set strategy and budgets.
  3. **Market-Shaping Tycoon** — Acquire competitors, influence regulation, shape entire markets. Your decisions move the industry.
- **Subsidiaries:** Create regional subsidiaries as you expand. Each subsidiary:
  - Has its own balance sheet, workforce, and regional operations
  - Reports to the parent holding company
  - Can be assigned autonomous management policies or micromanaged
  - Named by the player (like Vodafone UK, Vodafone DE pattern)

---

## 5. Management & Scaling (Tiered Abstraction)

The management interface changes as the company grows — Dwarf Fortress style:

- **Small company (1-10 assets):**
  - Hire individual employees (technicians, engineers, managers) with names, skills, salaries
  - Place every tower and cable yourself
  - Manage individual maintenance tasks
  - Track each contract manually

- **Medium company (10-100 assets):**
  - Hire teams: maintenance crews, construction teams, engineering departments
  - Assign teams to regions
  - Set maintenance budgets per region
  - Teams have experience levels that affect efficiency
  - Can still drill down to individual asset level

- **Large company (100+ assets, multiple regions):**
  - Manage departments and regional offices
  - Set policies: "Maintain at 95% uptime" or "Budget $X/month for maintenance"
  - AI workers execute based on policies
  - Hire regional managers with different skill levels
  - If maintenance budget is insufficient or teams are understaffed, infrastructure degrades
  - Review quarterly reports, intervene on exceptions

- **Key mechanic:** If you don't have enough maintenance crews or budget to maintain a major interconnect, it will degrade and eventually fail. Staffing and budget allocation matter as much as building.

---

## 6. Business Simulation (Progressive Complexity)

Complexity scales with company size:

### Early Game (Simple)
- Buy land, build infrastructure, earn revenue from coverage
- Revenue auto-calculated from capacity × regional demand
- Take bank loans at fixed interest rates

### Mid Game (Contracts)
- Negotiate peering, transit, and SLA contracts with AI and players
- Contracts have terms, capacity guarantees, breach penalties, renewal dates
- Lease infrastructure to/from other companies
- Regional pricing strategy (undercut competitors, premium positioning)

### Late Game (Full Corporate Sim)
- Mergers and acquisitions (hostile takeovers, friendly mergers)
- Stock market / shareholder management
- Board of directors voting
- Bankruptcy proceedings (bailout vs liquidation)
- Antitrust investigations if you get too dominant
- Patent licensing revenue (see Tech Tree section)

---

## 7. Technology & Research

- **R&D system:** Allocate R&D budget → choose research focus area → technologies unlock over time.
- **Branching paths:** Choose specialization (fiber specialist vs wireless specialist vs satellite). Can't research everything — forces strategic choices.
- **Patent system:**
  - When you research a technology, you can:
    - **Patent it** (requires legal team/office, costs money) → receive licensing fees from anyone who uses it
    - **Open-source it** → everyone gets it free, builds goodwill, no licensing revenue
    - **Keep it proprietary** → only your company can use it, no licensing
  - Other companies can license patented tech for a fee
  - Late joiners to a game can license all existing tech and start building modern infrastructure
- **Market conditions:** Some technologies only become available when world conditions are right (sufficient demand, prerequisite infrastructure exists).

---

## 8. Population & Economy

- **Population modeling:** Track actual populations per city/region:
  - Birth/death rates, migration patterns, employment levels
  - Infrastructure availability drives population growth and migration
  - A small town with amazing fiber may attract internet companies → population boom → demand surge
  - A city losing jobs may see population decline → demand drops → infrastructure becomes unprofitable
- **Economic simulation:**
  - Regional GDP affected by connectivity, infrastructure quality, population
  - Dynamic demand: grows with population and economic development
  - Market crashes, booms, and cycles emerge from simulation
- **Open data base:** Real-world data provides starting conditions, but the simulation drives ALL changes dynamically.

---

## 9. AI Corporations

- **Dynamic market:** AI companies spawn, grow, merge, and go bankrupt naturally. The competitive landscape evolves.
- **Configurable start:** Player sets initial AI company count and base difficulty.
- **Archetype system (4 base personalities):**
  1. **Aggressive Expander** — rapid growth, high debt tolerance, competitive pricing
  2. **Defensive Consolidator** — cautious, debt-averse, strong local networks
  3. **Tech Innovator** — R&D focused, high-capacity infrastructure, balanced growth
  4. **Budget Operator** — minimal spending, no debt, value infrastructure only
- **AI depth:** Each AI has strategy adaptation, financial management, market awareness. They respond to player actions and market conditions.
- **AI management:** AI companies have their own workforce, budgets, and management structures that follow the same rules as the player.

---

## 10. Regulatory & Political Simulation

- **Full regulatory simulation:**
  - Spectrum auctions (bid for wireless frequencies)
  - Net neutrality regulations (varies by country)
  - Antitrust enforcement (dominant companies face scrutiny)
  - Environmental regulations (construction permits, protected areas)
  - Data privacy regulations (GDPR-style rules in some regions)
- **Government interaction:**
  - Lobby for favorable regulation (costs money, not guaranteed)
  - Fund political campaigns (influence regulation indirectly)
  - Apply for government grants/subsidies for underserved areas
  - Different countries have different regulatory frameworks
- **Political events:** Elections change regulation, coups disrupt operations, trade wars affect cross-border infrastructure.

---

## 11. Disasters & World Events

- **Realistic simulation:** Earthquakes, hurricanes, flooding, political instability, regulation changes, market crashes.
- **Severity slider:** Player-controlled, scale 1-10.
  - 1 = Almost no disasters, stable world
  - 5 = Realistic probability of natural and political events
  - 10 = Constant crises, maximum chaos
- **Effects:** Infrastructure damage/destruction, service disruption, population displacement, economic impact.
- **Response:** Repair crews (if you have them staffed), insurance payouts, emergency contracts.

---

## 12. Competition & Cooperation

- **Cooperative possible:** Players can cooperate, share infrastructure, form alliances. PvP is organic through market competition, not forced.
- **Competitive mechanics:**
  - Fight for customers through pricing and coverage
  - Race for valuable land/spectrum
  - Compete in government contract bids
- **Server/game settings** can control competition level (peaceful to aggressive).

---

## 13. Multiplayer Architecture

- **Async multiplayer from day 1:** Designed into the architecture, not bolted on later.
- **Real-time persistent worlds:** Servers run 24/7. Your company operates on autopilot when you're away (based on policies you set).
- **Multiple worlds:** Different persistent worlds with different settings, eras, and player communities. Players pick a world to commit to.
- **AI proxy:** When offline, AI manages your company based on your policies. You review actions when you return.
- **Single-player:** Fully featured offline mode. Same simulation engine (Rust → WASM in browser). Save locally + cloud sync.

---

## 14. UI & UX

- **UI philosophy:** Clean modern SaaS aesthetic but still game-y and accessible. Not a boring dashboard, not an overwhelming game UI.
- **Data visualization:** Balanced — key metrics always visible, detailed charts/graphs available on demand (D3.js for custom viz). Don't overwhelm but depth is there.
- **Onboarding:** AI advisor system — in-game advisor suggests actions and explains why. Always available, never forced. Like a smart assistant.
- **Responsive design:** Optimized for desktop browsers. Works on tablets. Basic mobile support (simplified UI).
- **No modding:** Fully controlled experience.

### Visual Design Specifics

- **Map base:** Satellite-inspired dark base showing terrain (green land, blue ocean, brown mountains) with political borders overlaid. Night-earth vibes with city lights.
- **Infrastructure icons:** Realistic miniature icons. Tower looks like a tower, data center looks like a building. Detailed but readable at all zoom levels.
- **UI panels:** Solid dark panels with clean borders. Bloomberg Terminal / trading software aesthetic. Opaque, professional, information-dense.
- **Player/company branding:** Players choose their company color + logo. Custom branding appears on the map for owned infrastructure and territory. Like real telecom brands on a map.
- **Typography:** Clean sans-serif. Monospace for financial numbers and data tables.
- **Color palette:** Dark base (navy/charcoal), accent colors for data (green=profit, red=loss, blue=neutral, amber=warning). Company brand colors for ownership.

---

## 15. Audio

- **Style:** Ambient/atmospheric. Subtle background music, environmental sounds, data-flow ambience.
- **Think:** Quiet office, server room hum, subtle electronic textures. Calming but present.
- **Dynamic elements:** Subtle changes based on game state (slightly more tense during crises).

---

## 16. Save System

- **Local + cloud saves:** Both. Local for offline single-player, cloud syncs when online.
- **Auto-save:** Continuous auto-save (database-style persistence in multiplayer, periodic auto-save in SP).
- **Multiple save slots** for single-player.

---

## 17. Technical Stack

### Simulation Engine (Rust)
- **Language:** Rust
- **Architecture:** Entity Component System (Bevy ECS or custom ECS)
- **Compilation targets:**
  - WASM (via wasm-pack) for browser single-player
  - Native binary for multiplayer servers
  - Native binary for Tauri desktop app
- **Deterministic:** Same inputs = same outputs. Critical for multiplayer sync.

### Frontend (TypeScript/Svelte)
- **Framework:** Svelte (compile-time, tiny bundles, fast runtime)
- **Map rendering:** Three.js in 2D mode (flexible, supports future 3D close-ups)
- **Charts/data viz:** D3.js (custom financial charts, network diagrams, analytics)
- **Styling:** CSS (Svelte scoped styles) — dark theme, responsive
- **State:** Rust/WASM owns ALL game state via ECS. Svelte UI reads from WASM bridge.

### Build & Runtime
- **JS runtime:** Bun (all-in-one: runtime, bundler, package manager, test runner)
- **Build pipeline:** Bun + wasm-pack (Bun bundles TS/Svelte, wasm-pack compiles Rust→WASM)
- **Frontend hosting:** Vercel (automatic deployments, edge delivery, Svelte support)

### Backend/Servers
- **Persistent world servers:** Rust native binary (same simulation code as WASM, compiled natively)
- **API layer:** Rust (Axum) for game APIs, or Bun for lightweight services
- **Real-time:** WebSocket for live game updates
- **Auth:** Cloudflare Workers or Rust API server

### Database
- **Primary:** PostgreSQL for persistent world state, user accounts, cloud saves
- **Hosted:** Hetzner managed PostgreSQL or self-hosted on Hetzner VPS

### Desktop App
- **Wrapper:** Tauri (Rust-based, ~10MB bundles, uses system webview)
- **Why Tauri:** Already using Rust for sim engine, tiny bundles, native performance

### Infrastructure
- **Game servers:** Hetzner (x86_64 Linux) — Cloud VPS for dev, Dedicated AX line for production
- **Service layer:** Cloudflare Workers — authentication, lightweight APIs, CDN
- **Frontend CDN:** Vercel (Svelte app) + Cloudflare (static assets)
- **Database:** PostgreSQL on Hetzner
- **No AWS, no Azure, no Oracle. No Unreal Engine.**

---

## 18. Project Structure

```
globaltelco/
├── crates/                    # Rust workspace
│   ├── gt-simulation/         # Core ECS simulation engine
│   ├── gt-world/              # World generation, map data, geography
│   ├── gt-economy/            # Economic simulation, corporations, finance
│   ├── gt-infrastructure/     # Network graph, nodes, edges, routing
│   ├── gt-population/         # Population modeling, migration, employment
│   ├── gt-ai/                 # AI corporation controllers, strategy
│   ├── gt-wasm/               # WASM bindings (bridge to JS frontend)
│   ├── gt-server/             # Multiplayer server binary
│   └── gt-common/             # Shared types, traits, serialization
├── web/                       # Svelte frontend
│   ├── src/
│   │   ├── lib/               # Svelte components
│   │   ├── game/              # Game-specific components (map, panels, HUD)
│   │   ├── ui/                # Reusable UI components (buttons, charts, tables)
│   │   ├── wasm/              # WASM bridge / Rust interop
│   │   └── stores/            # Svelte stores for UI state
│   ├── static/                # Static assets (icons, fonts, map data)
│   └── package.json
├── desktop/                   # Tauri desktop app wrapper
├── data/                      # Open data sources (OSM, World Bank, UN)
├── docs/                      # Design documents
└── Cargo.toml                 # Rust workspace root
```

---

## 19. Game Name

**GlobalTelco** (working title, may change).
