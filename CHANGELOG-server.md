# Changelog

## 0.10.0 (2026-03-01)

### Features

- Implement Cloudflare R2 storage for snapshots and saves, and update deployment to use Cloudflare proxying. (80c3fb5)

### Other

- **engine:** v1.5.0 (00e48c7)
- **desktop:** v2.1.0 (231facd)


## 0.9.0 (2026-03-01)

### Features

- Implement 8-phase multiplayer overhaul (460e54c)
- Implement new bridge queries for alliances, pricing, lawsuits, and stock market data, and add a `create_player_corporation` function to the simulation. (58cd4d2)

### Chores

- Update Rust dependencies. (458da55)

### Other

- **engine:** v1.4.0 (de30b1b)
- **engine:** v1.3.1 (cc91c7e)
- **desktop:** v2.0.0 (ce2569e)
- **web:** v1.4.0 (60f18b1)


## 0.8.0 (2026-03-01)

### Features

- **desktop:** Native Rust simulation with SimThread, binary typed arrays, and release pipeline (305b431)
- Add Real Earth building footprint rendering with dual color palettes, refactor the FTTH builder to use Network Access Points, and configure Vercel for clean URLs. (89e99ea)
- **web:** Overhaul radial build menu with supersession, edge grouping, satellite segment, FTTH builder, and icons (15d4e14)
- Add satellite and terminal supply chain panels, new infrastructure icons, and satellite AI systems. (cf77979)

### Bug Fixes

- **web:** Real Earth building visibility, FTTH builder UX, and Vercel /admin route (5a3f4b6)

### Other

- Refactor game command parameters for consistency, remove the MapRenderer, and add new satellite-related achievements and configurable auto-save. (90d8eb8)
- **engine:** v1.3.0 (3351059)
- **web:** v1.2.0 (3e96083)


## 0.7.0 (2026-02-27)

### Features

- Introduce comprehensive satellite simulation, including orbital mechanics, constellations, manufacturing, launch, debris, and revenue systems. (6ee6482)
- **gt:** overhaul TUI with cards, context-aware footer, and esc-safety (b92bbc8)

### Chores

- Update Rust dependencies. (1842bae)

### Other

- **engine:** v1.2.0 (60aae7f)
- **desktop:** v0.1.1 (240ba46)
- **web:** v1.1.0 (268a152)
- **engine:** v1.1.0 (e7c6f45)

