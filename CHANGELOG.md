# Changelog

## 1.7.0 (2026-03-03)

### Features

- update tick duration method to be asynchronous and improve log saving message (c57886b)
- update dependencies in Cargo.lock and add plain package (c3ea46c)

### Other

- Merge branch 'main' of https://github.com/KodyDennon/GlobalTelco (5ee5346)
- **admin:** v1.0.1 (3cc7e02)
- Add funding information to FUNDING.yml (18097e6)
- **web:** v1.6.0 (2d17435)
- **desktop:** v2.3.0 (5a0cd0b)
- **server:** v0.11.0 (63eb8c0)


## 1.6.0 (2026-03-02)

### Features

- Update admin panel setup instructions and deployment process for Cloudflare Pages (9195811)
- Add world config form, server limits, direct lobby world creation, fix web build (e1e9a28)
- Make admin dashboard fully responsive for mobile and tablet (e929da3)
- Complete admin dashboard overhaul with full deployment (06e06f0)
- Introduce a new Svelte-based admin panel, adding server API routes, simulation profiling, and chat migration, while removing old changelog files. (9e9c8b9)
- Add in-game player list, integrate friends/invite features into the lobby and game view, implement multiplayer speed vote handling, and enhance lobby registration with client-side validation. (496cb65)
- Expand ECS systems to 36, increase infrastructure node and edge types, and document crate modularization. (8c5504f)

### Bug Fixes

- **mp:** clear snapshot store after init to optimize memory usage (2fbd6e3)
- **mp:** prevent race condition when joining world by using persistent snapshot store (ded0496)
- **mp:** increase snapshot timeout to 60s and handle disconnection (55bada4)
- Correct worlds API response shape — server returns bare array, not wrapped object (583eb70)
- Use rustls for reqwest to fix cross-compilation (17d9afb)

### Chores

- Update desktop Cargo.lock for crate version bumps (985b737)

### Other

- **web:** v1.5.1 (7c9a47e)
- **desktop:** v2.2.0 (49e0cb2)
- **server:** v0.10.1 (1b2266f)


## 1.5.1 (2026-03-01)

### Refactoring

- Restructure modules into subdirectories, introduce a new query system, and add spectrum management types and commands. (06bbc65)

### Other

- **web:** v1.5.0 (c6babaa)
- **server:** v0.10.0 (d09cd1b)


## 1.5.0 (2026-03-01)

### Features

- Implement Cloudflare R2 storage for snapshots and saves, and update deployment to use Cloudflare proxying. (80c3fb5)

### Other

- **desktop:** v2.1.0 (231facd)
- **server:** v0.9.0 (375de96)


## 1.4.0 (2026-03-01)

### Features

- Implement 8-phase multiplayer overhaul (460e54c)
- Implement new bridge queries for alliances, pricing, lawsuits, and stock market data, and add a `create_player_corporation` function to the simulation. (58cd4d2)

### Chores

- Update Rust dependencies. (458da55)


## 1.3.1 (2026-03-01)

### Other

- **desktop:** v2.0.0 (ce2569e)
- **web:** v1.4.0 (60f18b1)
- **server:** v0.8.0 (694df53)
- Refactor game command parameters for consistency, remove the MapRenderer, and add new satellite-related achievements and configurable auto-save. (90d8eb8)


## 1.3.0 (2026-03-01)

### Features

- **desktop:** Native Rust simulation with SimThread, binary typed arrays, and release pipeline (305b431)
- Add Real Earth building footprint rendering with dual color palettes, refactor the FTTH builder to use Network Access Points, and configure Vercel for clean URLs. (89e99ea)
- **web:** Overhaul radial build menu with supersession, edge grouping, satellite segment, FTTH builder, and icons (15d4e14)
- Add satellite and terminal supply chain panels, new infrastructure icons, and satellite AI systems. (cf77979)

### Bug Fixes

- **web:** Real Earth building visibility, FTTH builder UX, and Vercel /admin route (5a3f4b6)

### Other

- **web:** v1.2.0 (3e96083)
- **server:** v0.7.0 (ce5d6d0)


## 1.2.0 (2026-02-27)

### Features

- Introduce comprehensive satellite simulation, including orbital mechanics, constellations, manufacturing, launch, debris, and revenue systems. (6ee6482)

### Other

- **desktop:** v0.1.1 (240ba46)
- **web:** v1.1.0 (268a152)


## 1.1.0 (2026-02-27)

### Features

- **gt:** overhaul TUI with cards, context-aware footer, and esc-safety (b92bbc8)

### Chores

- Update Rust dependencies. (1842bae)

### Other

- **server:** v0.6.0 (f736c7a)
- **web:** v1.0.0 (2205166)

