# Changelog

## 1.9.2 (2026-03-06)

### Other

- **engine:** v1.12.2 (3b46a5f)
- cargo (cc9e5ba)
- **admin:** v1.2.0 (6fc639d)


## 1.9.1 (2026-03-06)

### Features

- Reorganize documentation, update `gt-simulation` systems and world commands, refine admin panel components, and adjust server-side logic and dependencies. (244d9dd)

### Other

- **server:** v1.0.1 (da54a24)
- **engine:** v1.12.1 (d877d32)
- nonsense (619ec84)


## 1.9.0 (2026-03-06)

### Features

- Enhance data packing and querying with viewport support for infrastructure nodes and edges (f8ea329)

### Chores

- Update gt packages to version 1.11.2 (1fd0568)

### Other

- **engine:** v1.12.0 (6c8f34a)


## 1.8.10 (2026-03-06)

### Features

- Add Display implementation for EdgeType and NodeType enums (e13e6d0)
- Enhance WASM bridge with static definitions and improved data structures (5aa6ce1)
- add performance audit and overhaul plan documentation for rendering optimizations (2b85ede)

### Chores

- update gt packages to version 1.11.1 (b7bd579)

### Other

- **engine:** v1.11.2 (88bd2c0)


## 1.8.9 (2026-03-05)

### Features

- enhance grant and co-ownership functionalities with new data structures and caching (e41048a)

### Chores

- update package versions to 1.11.0 and 1.0.0 (4b3fb8a)

### Other

- **engine:** v1.11.1 (a0fba1d)


## 1.8.8 (2026-03-05)

### Features

- add new grant and co-ownership functionalities (b9d3fc4)
- implement stock market buy/sell commands and UI integration (74b8181)

### Bug Fixes

- update package versions for gt-ai, gt-bridge, gt-common, gt-economy, gt-infrastructure, gt-population, gt-server, gt-simulation, gt-tauri, gt-wasm, and gt-world to 1.10.0 and gt-server to 0.13.4 (5359add)

### Other

- **engine:** v1.11.0 (291102f)
- **server:** v1.0.0 (cbf79b6)
- Refactor covert ops intel decay logic and adjust revenue calculations (1b978fd)


## 1.8.7 (2026-03-04)

### Features

- Introduce road network module and refactor road graph functionality (58b8620)

### Bug Fixes

- bump package versions for gt-ai, gt-bridge, gt-common, gt-economy, gt-infrastructure, gt-population, gt-server, gt-simulation, gt-tauri, gt-wasm, and gt-world to latest releases (a16fdf4)

### Other

- **server:** v0.13.4 (292f687)
- **engine:** v1.10.0 (6a4698f)


## 1.8.6 (2026-03-04)

### Bug Fixes

- correct string formatting in SQL queries and logging messages (4e46312)
- update PostgreSQL reference and bump package versions to 1.9.3 and 0.13.2 (effa553)

### Other

- **engine:** v1.9.4 (bc1158a)
- **server:** v0.13.3 (e50793b)


## 1.8.5 (2026-03-04)

### Features

- Update package versions and implement event pruning in the database (64811dc)

### Other

- **server:** v0.13.2 (d5fb3b3)
- **engine:** v1.9.3 (bfc8bfa)


## 1.8.4 (2026-03-04)

### Features

- Enhance infrastructure rendering and revenue calculation by addressing race conditions, improving data fetching, and ensuring immediate updates for player corporations (935d1fc)
- Refactor Rust WASM conditional compilation and introduce latest tick result synchronization to the web bridge for UI consumers, alongside a type fix for player corporation ID. (49d950a)

### Chores

- update package versions to 1.9.1 and optimize database event logging (73c4e04)

### Other

- **server:** v0.13.1 (afa8cf5)
- **engine:** v1.9.2 (ebdc26e)


## 1.8.3 (2026-03-03)

### Chores

- update gt-server version to 0.13.0 in Cargo.lock (366e337)

### Other

- **engine:** v1.9.1 (0a9e121)


## 1.8.2 (2026-03-03)

### Features

- Improve WorldCreator UI with input constraints, slider value display, and disabled template cards for full instances. (8e4f52c)
- enhance R2 storage client with improved error handling and logging (724a538)

### Chores

- update package versions to 1.8.1 in Cargo.lock (00fc4c8)

### Other

- **engine:** v1.9.0 (aa2ee6a)
- **server:** v0.13.0 (872f348)
- **server:** v0.12.1 (d1b8cf9)


## 1.8.1 (2026-03-03)

### Bug Fixes

- handle optional player corporation ID and improve financial delta updates in multiplayer (6f5501f)

### Chores

- update package versions to 1.8.0 in Cargo.lock (e6c51f5)

### Other

- **engine:** v1.8.1 (d25e355)


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

- **admin:** v1.1.0 (e12597b)
- **engine:** v1.8.0 (a098d68)


## 1.7.0 (2026-03-03)

### Features

- update tick duration method to be asynchronous and improve log saving message (c57886b)
- update dependencies in Cargo.lock and add plain package (c3ea46c)

### Other

- **server:** v0.12.0 (28d7690)
- **engine:** v1.7.0 (37d02e1)
- Merge branch 'main' of https://github.com/KodyDennon/GlobalTelco (5ee5346)
- **admin:** v1.0.1 (3cc7e02)
- Add funding information to FUNDING.yml (18097e6)


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

- **desktop:** v2.3.0 (5a0cd0b)
- **server:** v0.11.0 (63eb8c0)
- **engine:** v1.6.0 (433cee3)

