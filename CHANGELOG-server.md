# Changelog

## 1.4.0 (2026-03-15)

### Features

- **desktop:** add 7 missing Tauri IPC query commands (2e510ae)

### Bug Fixes

- improve release naming and publish releases (not draft) (5c75f0e)
- clean up release pipeline and remove hardcoded secrets for public repo (83ced8d)
- **admin:** live state awareness, error handling, reactive polling, and proper transitions (4c9601b)

### Other

- **engine:** v2.0.0 (1900c05)
- crgo (16bcb14)
- **desktop:** v2.4.1 (b0e4b70)
- **web:** v1.11.1 (18e69ef)


## 1.3.0 (2026-03-15)

### Features

- **tools/gt:** add 'Force .env overwrite' option to deployer (1f88afa)

### Bug Fixes

- comprehensive codebase audit — crash fixes, logic bugs, dead code removal, and structural splits (92cf36e)
- restore map in Pure Worker mode and finalize weather removal (d7a1ab3)

### Performance

- exhaustive end-to-end removal of disasters and weather (aa3e822)
- exhaustive end-to-end removal of disasters, weather, and insurance (3e974dc)
- complete end-to-end removal of disasters and weather (9f717c9)
- disable weather and disaster systems for performance (a8ad21e)
- implement zero-lag overhaul with worker-side parsing and layer memoization (526144a)
- implement pure worker mode and throttled interactivity (872262f)
- implement zero-copy binary rendering and worker-first data sync (d2f30a5)
- finalize high-fidelity simulation and map rendering optimizations (5386082)

### Other

- Service file (dd72364)
- **web:** v1.10.1 (3f8d6d6)
- **engine:** v1.12.5 (07828ec)
- crgo (f6ad014)
- **web:** v1.10.0 (721271c)
- **admin:** v1.2.1 (d666378)


## 1.1.0 (2026-03-07)

### Features

- implement persistent world deletion and purge functionality (9a474b3)

### Performance

- overhaul simulation core and map rendering pipeline (84ab513)

### Other

- **engine:** v1.12.4 (f9909de)
- cargo (557db01)
- **web:** v1.9.3 (a280a27)
- **engine:** v1.12.3 (e82c312)
- crg (a16e4a1)
- **web:** v1.9.2 (6140543)
- **engine:** v1.12.2 (3b46a5f)
- cargo (cc9e5ba)
- **admin:** v1.2.0 (6fc639d)
- **web:** v1.9.1 (5f77149)


## 1.0.1 (2026-03-06)

### Features

- Reorganize documentation, update `gt-simulation` systems and world commands, refine admin panel components, and adjust server-side logic and dependencies. (244d9dd)
- Enhance data packing and querying with viewport support for infrastructure nodes and edges (f8ea329)
- Add Display implementation for EdgeType and NodeType enums (e13e6d0)
- Enhance WASM bridge with static definitions and improved data structures (5aa6ce1)
- add performance audit and overhaul plan documentation for rendering optimizations (2b85ede)
- enhance grant and co-ownership functionalities with new data structures and caching (e41048a)

### Chores

- Update gt packages to version 1.11.2 (1fd0568)
- update gt packages to version 1.11.1 (b7bd579)
- update package versions to 1.11.0 and 1.0.0 (4b3fb8a)

### Other

- **engine:** v1.12.1 (d877d32)
- nonsense (619ec84)
- **web:** v1.9.0 (b3c4a3b)
- **engine:** v1.12.0 (6c8f34a)
- **web:** v1.8.10 (a4a8520)
- **engine:** v1.11.2 (88bd2c0)
- **web:** v1.8.9 (4a13519)
- **engine:** v1.11.1 (a0fba1d)
- **web:** v1.8.8 (720efb7)
- **engine:** v1.11.0 (291102f)


## 1.0.0 (2026-03-05)

### Features

- add new grant and co-ownership functionalities (b9d3fc4)
- implement stock market buy/sell commands and UI integration (74b8181)

### Bug Fixes

- update package versions for gt-ai, gt-bridge, gt-common, gt-economy, gt-infrastructure, gt-population, gt-server, gt-simulation, gt-tauri, gt-wasm, and gt-world to 1.10.0 and gt-server to 0.13.4 (5359add)

### Other

- Refactor covert ops intel decay logic and adjust revenue calculations (1b978fd)
- **web:** v1.8.7 (46ecfb8)


## 0.13.4 (2026-03-04)

### Features

- Introduce road network module and refactor road graph functionality (58b8620)

### Bug Fixes

- bump package versions for gt-ai, gt-bridge, gt-common, gt-economy, gt-infrastructure, gt-population, gt-server, gt-simulation, gt-tauri, gt-wasm, and gt-world to latest releases (a16fdf4)

### Other

- **engine:** v1.10.0 (6a4698f)
- **web:** v1.8.6 (434f9d6)
- **engine:** v1.9.4 (bc1158a)


## 0.13.3 (2026-03-04)

### Bug Fixes

- correct string formatting in SQL queries and logging messages (4e46312)
- update PostgreSQL reference and bump package versions to 1.9.3 and 0.13.2 (effa553)

### Other

- **web:** v1.8.5 (84fc21c)


## 0.13.2 (2026-03-04)

### Features

- Update package versions and implement event pruning in the database (64811dc)

### Other

- **engine:** v1.9.3 (bfc8bfa)
- **web:** v1.8.4 (0fd30f7)


## 0.13.1 (2026-03-04)

### Features

- Enhance infrastructure rendering and revenue calculation by addressing race conditions, improving data fetching, and ensuring immediate updates for player corporations (935d1fc)
- Refactor Rust WASM conditional compilation and introduce latest tick result synchronization to the web bridge for UI consumers, alongside a type fix for player corporation ID. (49d950a)

### Chores

- update package versions to 1.9.1 and optimize database event logging (73c4e04)
- update gt-server version to 0.13.0 in Cargo.lock (366e337)

### Other

- **engine:** v1.9.2 (ebdc26e)
- **web:** v1.8.3 (2c45e84)
- **engine:** v1.9.1 (0a9e121)
- **web:** v1.8.2 (a97c3c9)
- **engine:** v1.9.0 (aa2ee6a)


## 0.13.0 (2026-03-03)

### Features

- Improve WorldCreator UI with input constraints, slider value display, and disabled template cards for full instances. (8e4f52c)
- enhance R2 storage client with improved error handling and logging (724a538)

### Chores

- update package versions to 1.8.1 in Cargo.lock (00fc4c8)


## 0.12.1 (2026-03-03)

### Features

- implement set_player_corp_id function in WASM bridge and GameWorld, update multiplayer initialization to handle player corporation ID (f940044)

### Bug Fixes

- handle optional player corporation ID and improve financial delta updates in multiplayer (6f5501f)
- update labels and input elements for better accessibility in WorldCreator component (7c6b5b2)
- enhance wasm build settings with additional optimization flags (9735a82)
- update wasm-pack output directory in Vercel configuration and add wasm-opt settings in Cargo.toml (9c40b4f)
- handle undefined data in DataTable component for filtering and pagination (03f6510)

### Chores

- update package versions to 1.8.0 in Cargo.lock (e6c51f5)
- update package versions to 1.7.0 and 0.12.0 (ee45a0e)

### Other

- **web:** v1.8.1 (90910d4)
- **engine:** v1.8.1 (d25e355)
- **web:** v1.8.0 (606ad02)
- **admin:** v1.1.0 (e12597b)
- **engine:** v1.8.0 (a098d68)
- **web:** v1.7.0 (4f2cda2)


## 0.12.0 (2026-03-03)

### Features

- update tick duration method to be asynchronous and improve log saving message (c57886b)
- update dependencies in Cargo.lock and add plain package (c3ea46c)

### Other

- **engine:** v1.7.0 (37d02e1)
- Merge branch 'main' of https://github.com/KodyDennon/GlobalTelco (5ee5346)
- **admin:** v1.0.1 (3cc7e02)
- Add funding information to FUNDING.yml (18097e6)
- **web:** v1.6.0 (2d17435)
- **desktop:** v2.3.0 (5a0cd0b)


## 0.11.0 (2026-03-02)

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

- **engine:** v1.6.0 (433cee3)
- **web:** v1.5.1 (7c9a47e)
- **desktop:** v2.2.0 (49e0cb2)

