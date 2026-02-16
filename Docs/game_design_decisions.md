# GlobalTelco: Game Design Decisions

Concrete implementation decisions derived from design questionnaire. These supplement the original design documents with specific answers needed for development.

---

## 1. Core Experience

- **New player experience:** Separate instanced tutorial region. Guided intro: claim a small region, build first tower, connect fiber, see revenue flow. Join the live world when ready.
- **Camera:** 3D globe view with zoom — start zoomed out on full globe, zoom into regions. Like Google Earth meets strategy game.
- **World geography:** Real Earth (abstracted into hex parcels) for multiplayer servers. Optional procedurally generated worlds for single-player custom games.
- **MVP scope:** Full server prototype — 10+ player server with all systems (infrastructure, economy, contracts, alliances).
- **Single-player:** Available from the start with AI corporations and custom proc-gen worlds.

---

## 2. Infrastructure & Building

- **Build interaction:** Click-to-place nodes on hex parcels, drag edges between nodes to lay fiber/cable. Direct manipulation.
- **Node types at launch:** All 6 — Access Towers, Fiber Distribution Hubs, Data Centers, IXPs, Subsea Landing Stations, Satellite Ground Stations.
- **Construction time:** Tick-based countdown scaled by terrain and edge type. Towers are fast, subsea cables are slow. Terrain multipliers affect duration. Predictable and easy to balance.
- **Competitor visibility:** Partial — players see node locations on the map but not capacity/stats. Need intelligence or espionage to learn detailed competitor infrastructure info.

---

## 3. Economy & Competition

- **Starting capital:** Difficulty-based presets. Single-player can choose difficulty when starting a new game. Multiplayer servers can be configured as easy/normal/hard economies.
- **AI corporations:** Single-player only. AI corps populate single-player and custom worlds. Multiplayer servers are player-only.
- **Revenue model:** Hybrid — base revenue auto-calculated from coverage, capacity, SLA quality, and regional demand. Players manually set premium pricing for transit/peering contracts. Undercutting competitors is a strategy.
- **Disaster severity:** Configurable per server/world — calm, moderate, or brutal. Different difficulty levels.
- **Technology system:** Full tech tree with research + patents + licensing. Companies invest in R&D (e.g., "advanced optical bandwidth"). Unlocked tech can be licensed to other corporations for revenue. Everyone gets base tech for free. Research should not be overcumbersome — enhance gameplay, not block it.
- **Bankruptcy:** Both paths available — player chooses to accept government bailout (high-interest debt, fight to recover) or declare bankruptcy and restart fresh with a new corporation.
- **Win conditions:** Optional objectives — leaderboards, achievements, seasonal goals. No "winner" but competitive milestones to chase. Pure sandbox with optional structure.
- **Land competition:** Full spectrum — market competition (buy/lease/outbid) + legal/political actions (lobbying, lawsuits, zoning disputes) + limited sabotage with checks and balances.

---

## 4. Multiplayer & Social

- **Authentication:** Custom account system (email/password via Cloudflare Workers) as primary + Steam/Epic integration as optional link. Both from the start.
- **Communication:** Text chat only — global, regional, alliance, and direct message channels.
- **Server joining:** Persistent world selection (each world has a name and identity, players commit long-term like an MMO server) + quick-join option for casual play.
- **Spectator mode:** Yes — non-playing spectators can watch the world evolve. Good for streams, tournaments, learning.
- **News system:** Both world news and personal alerts. Global news ticker ("Earthquake strikes South Asia", "Corp X acquires Corp Y") + personal notification panel for corporation-specific alerts.

---

## 5. Technical

- **Target platforms:** PC only — Windows and Mac.
- **World persistence:** Full state save to PostgreSQL database. Every node, edge, corporation, contract, parcel serialized. Full restore on restart. No data loss.
- **World size:** Large — 100,000+ hex parcels for full globe coverage. Requires LOD and world partition streaming.
- **Tick speed:** Adaptive — tick speed adjusts based on player count and server load. Slower when busy, faster when quiet.
- **Visual fidelity:** Semi-realistic — recognizable real-world look but not photorealistic. Stylized terrain, identifiable buildings. Like Civilization VI aesthetic.
- **Day/night cycle:** Yes, accelerated — day/night cycle runs faster than real-time (e.g., 1 day = 1 hour). Visual variety, shows global scale.
- **UI framework:** UMG + Slate hybrid — UMG for menus/HUD, Slate C++ for complex data-heavy panels (network topology viewer, financial charts).
- **Modding:** Not for now. Focus on core game.
- **Game name:** GlobalTelco (final).

---

## 6. Hosting Architecture

- **Game servers:** Hetzner (x86_64 Linux) — Cloud VPS for dev, Dedicated AX line for production.
- **Service layer:** Cloudflare Workers — authentication, APIs, matchmaking, CDN.
- **Database:** PostgreSQL (Hetzner managed or Cloudflare D1 for lightweight data).
- **No AWS, no Azure, no Oracle.**
