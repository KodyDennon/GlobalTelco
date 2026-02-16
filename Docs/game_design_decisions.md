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

---

## 7. Offline Single-Player Mode

- **Game mode:** `AGTSinglePlayerGameMode` (extends `AGameModeBase`, not the multiplayer `AGTGameMode`) — no networking assumptions.
- **Session flow:** Main Menu widget → New Game settings widget → `UGTGameInstance` stores config → Level transition → GameMode reads config from GameInstance, generates world, creates corps, starts simulation.
- **AI corporations:** Full UE5 Behavior Tree system (programmatic C++ construction, no .uasset files). Each AI corp has an `AGTAICorporationController` (AAIController without pawn) that runs a strategy tree with tasks: acquire land, build nodes/edges, manage finances, propose contracts.
- **AI archetype system:** 4 built-in personality archetypes defined in code (no editor assets):
  1. **Aggressive Expander** — fast growth, high debt, competitive pricing
  2. **Defensive Consolidator** — cautious, debt-averse, strong local networks
  3. **Tech Innovator** — high-capacity infrastructure, R&D focus, balanced
  4. **Budget Operator** — minimal spending, no debt, cost-effective only
- **Archetype assignment:** Round-robin from registry, varied by world seed for uniqueness. Each archetype has 8 company names in its pool.
- **AI strategy selection:** Dynamic based on financial health + archetype weights. 4 modes: Expand (buy land + build), Consolidate (improve existing + contracts), Compete (undercut competitors), Survive (cut costs when insolvent).
- **Save/load format:** UE5 `USaveGame` binary serialization (FMemoryArchive). Complete world snapshot: simulation tick, all parcels, all corporation financial state + owned assets, regional economy, contracts/alliances, world settings.
- **Save slot naming:** Internal format `GT_<SlotName>.sav` in `Saved/SaveGames/`. `UGTSaveLoadSubsystem` (GameInstance subsystem) manages enumeration, save, load, delete.
- **Auto-save:** Every 50 economic ticks to "AutoSave" slot, triggered by the GameMode's EconomicTick listener.
- **Save version:** Integer version field (`GT_SAVE_VERSION`) in save header. Forward-compatibility guard: refuse to load saves with higher version than current.
- **Speed controls:** Pause, 1x, 2x, 4x via `UGTSimulationSubsystem::SetPaused()` / `SetSpeedMultiplier()`. Speed multiplier scales DeltaTime before tick accumulation.
- **Difficulty presets:** Easy (3 AI, low aggression, calm disasters), Normal (5 AI, standard aggression, moderate disasters), Hard (8 AI, high aggression, brutal disasters), Custom (full override).
- **New game customization:** Corporation name, difficulty dropdown, AI corporation count slider (0-10), disaster severity dropdown, world seed input (0=random).
- **DefaultEngine.ini:** `GameInstanceClass=/Script/GlobalTelco.GTGameInstance`, `GlobalDefaultGameMode=/Script/GlobalTelco.GTSinglePlayerGameMode`.
