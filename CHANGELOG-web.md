# Changelog

## 1.5.1 (2026-03-01)

### Refactoring

- Restructure modules into subdirectories, introduce a new query system, and add spectrum management types and commands. (06bbc65)

### Other

- **desktop:** v2.2.0 (49e0cb2)
- **server:** v0.10.1 (1b2266f)
- **engine:** v1.5.1 (7d5bf2c)


## 1.5.0 (2026-03-01)

### Features

- Implement Cloudflare R2 storage for snapshots and saves, and update deployment to use Cloudflare proxying. (80c3fb5)
- Implement 8-phase multiplayer overhaul (460e54c)
- Implement new bridge queries for alliances, pricing, lawsuits, and stock market data, and add a `create_player_corporation` function to the simulation. (58cd4d2)

### Chores

- Update Rust dependencies. (458da55)

### Other

- **server:** v0.10.0 (d09cd1b)
- **engine:** v1.5.0 (00e48c7)
- **desktop:** v2.1.0 (231facd)
- **server:** v0.9.0 (375de96)
- **engine:** v1.4.0 (de30b1b)
- **engine:** v1.3.1 (cc91c7e)
- **desktop:** v2.0.0 (ce2569e)


## 1.4.0 (2026-03-01)

### Features

- **desktop:** Native Rust simulation with SimThread, binary typed arrays, and release pipeline (305b431)
- Add Real Earth building footprint rendering with dual color palettes, refactor the FTTH builder to use Network Access Points, and configure Vercel for clean URLs. (89e99ea)
- **web:** Overhaul radial build menu with supersession, edge grouping, satellite segment, FTTH builder, and icons (15d4e14)
- Add satellite and terminal supply chain panels, new infrastructure icons, and satellite AI systems. (cf77979)

### Bug Fixes

- **web:** Real Earth building visibility, FTTH builder UX, and Vercel /admin route (5a3f4b6)

### Other

- **server:** v0.8.0 (694df53)
- Refactor game command parameters for consistency, remove the MapRenderer, and add new satellite-related achievements and configurable auto-save. (90d8eb8)
- **engine:** v1.3.0 (3351059)


## 1.2.0 (2026-02-27)

### Features

- Introduce comprehensive satellite simulation, including orbital mechanics, constellations, manufacturing, launch, debris, and revenue systems. (6ee6482)

### Other

- **server:** v0.7.0 (ce5d6d0)
- **engine:** v1.2.0 (60aae7f)
- **desktop:** v0.1.1 (240ba46)


## 1.1.0 (2026-02-27)

### Features

- **gt:** overhaul TUI with cards, context-aware footer, and esc-safety (b92bbc8)

### Chores

- Update Rust dependencies. (1842bae)

### Other

- **engine:** v1.1.0 (e7c6f45)
- **server:** v0.6.0 (f736c7a)

