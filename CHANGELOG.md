# Changelog

## 1.11.1 (2026-03-05)

### Features

- enhance grant and co-ownership functionalities with new data structures and caching (e41048a)

### Chores

- update package versions to 1.11.0 and 1.0.0 (4b3fb8a)

### Other

- **web:** v1.8.8 (720efb7)


## 1.11.0 (2026-03-05)

### Features

- add new grant and co-ownership functionalities (b9d3fc4)
- implement stock market buy/sell commands and UI integration (74b8181)

### Bug Fixes

- update package versions for gt-ai, gt-bridge, gt-common, gt-economy, gt-infrastructure, gt-population, gt-server, gt-simulation, gt-tauri, gt-wasm, and gt-world to 1.10.0 and gt-server to 0.13.4 (5359add)

### Other

- **server:** v1.0.0 (cbf79b6)
- Refactor covert ops intel decay logic and adjust revenue calculations (1b978fd)
- **web:** v1.8.7 (46ecfb8)
- **server:** v0.13.4 (292f687)


## 1.10.0 (2026-03-04)

### Features

- Introduce road network module and refactor road graph functionality (58b8620)

### Bug Fixes

- bump package versions for gt-ai, gt-bridge, gt-common, gt-economy, gt-infrastructure, gt-population, gt-server, gt-simulation, gt-tauri, gt-wasm, and gt-world to latest releases (a16fdf4)

### Other

- **web:** v1.8.6 (434f9d6)


## 1.9.4 (2026-03-04)

### Bug Fixes

- correct string formatting in SQL queries and logging messages (4e46312)
- update PostgreSQL reference and bump package versions to 1.9.3 and 0.13.2 (effa553)

### Other

- **server:** v0.13.3 (e50793b)
- **web:** v1.8.5 (84fc21c)
- **server:** v0.13.2 (d5fb3b3)


## 1.9.3 (2026-03-04)

### Features

- Update package versions and implement event pruning in the database (64811dc)

### Other

- **web:** v1.8.4 (0fd30f7)
- **server:** v0.13.1 (afa8cf5)


## 1.9.2 (2026-03-04)

### Features

- Enhance infrastructure rendering and revenue calculation by addressing race conditions, improving data fetching, and ensuring immediate updates for player corporations (935d1fc)
- Refactor Rust WASM conditional compilation and introduce latest tick result synchronization to the web bridge for UI consumers, alongside a type fix for player corporation ID. (49d950a)

### Chores

- update package versions to 1.9.1 and optimize database event logging (73c4e04)

### Other

- **web:** v1.8.3 (2c45e84)


## 1.9.1 (2026-03-03)

### Chores

- update gt-server version to 0.13.0 in Cargo.lock (366e337)

### Other

- **web:** v1.8.2 (a97c3c9)


## 1.9.0 (2026-03-03)

### Features

- Improve WorldCreator UI with input constraints, slider value display, and disabled template cards for full instances. (8e4f52c)
- enhance R2 storage client with improved error handling and logging (724a538)

### Chores

- update package versions to 1.8.1 in Cargo.lock (00fc4c8)

### Other

- **server:** v0.13.0 (872f348)
- **server:** v0.12.1 (d1b8cf9)
- **web:** v1.8.1 (90910d4)


## 1.8.1 (2026-03-03)

### Bug Fixes

- handle optional player corporation ID and improve financial delta updates in multiplayer (6f5501f)

### Chores

- update package versions to 1.8.0 in Cargo.lock (e6c51f5)

### Other

- **web:** v1.8.0 (606ad02)
- **admin:** v1.1.0 (e12597b)


## 1.8.0 (2026-03-03)

### Features

- implement set_player_corp_id function in WASM bridge and GameWorld, update multiplayer initialization to handle player corporation ID (f940044)

### Bug Fixes

- update labels and input elements for better accessibility in WorldCreator component (7c6b5b2)
- enhance wasm build settings with additional optimization flags (9735a82)
- update wasm-pack output directory in Vercel configuration and add wasm-opt settings in Cargo.toml (9c40b4f)
- handle undefined data in DataTable component for filtering and pagination (03f6510)

### Chores

- update package versions to 1.7.0 and 0.12.0 (ee45a0e)

### Other

- **web:** v1.7.0 (4f2cda2)
- **server:** v0.12.0 (28d7690)


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

