# GlobalTelco — Qwen Context Guide

## Project Overview

**GlobalTelco** is a 2D infrastructure empire builder — a mix of city builder, tycoon/business sim, and grand strategy. Players build and operate telecom networks on a political map, growing from a local ISP to a global telecom empire.

- **Live demo:** [globaltelco.online](https://globaltelco.online)
- **Frontend:** Svelte 5 + deck.gl (2D map) + D3.js (charts)
- **Simulation:** Rust ECS engine (36 systems) → compiles to WASM (browser) + native binary (servers)
- **Desktop:** Tauri (Rust wrapper with system webview)
- **Multiplayer:** Server-authoritative with WebSocket delta sync
- **Database:** PostgreSQL (Neon)

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Simulation | Rust (ECS architecture) → WASM (browser) + native binary (servers) |
| Frontend | Svelte 5 + deck.gl + D3.js |
| Desktop | Tauri (Rust + system webview) |
| Server | Rust (Axum) with WebSocket |
| Database | PostgreSQL (Neon) |
| Build | Bun + wasm-pack + cargo-zigbuild |
| Hosting | Oracle Cloud (game server) + Vercel (frontend) + Cloudflare (DNS/proxy) |

## Project Structure

```
globaltelco/
├── crates/                    # Rust workspace (11 crates)
│   ├── gt-common/             # Shared types, traits, protocol
│   ├── gt-simulation/         # Core ECS engine (36 systems)
│   ├── gt-world/              # World generation, terrain, rivers
│   ├── gt-economy/            # Finance, markets, contracts
│   ├── gt-infrastructure/     # Network graph, routing
│   ├── gt-population/         # Demographics, demand
│   ├── gt-ai/                 # AI corporation controllers
│   ├── gt-bridge/             # Shared bridge trait + query functions
│   ├── gt-wasm/               # WASM bindings (wasm-bindgen)
│   ├── gt-tauri/              # Tauri native bridge
│   └── gt-server/             # Multiplayer server (Axum + WebSocket)
├── web/                       # Svelte frontend
│   └── src/lib/
│       ├── wasm/              # TypeScript WASM bridge
│       ├── game/              # Game UI (map, HUD, build tools)
│       ├── panels/            # Management panels
│       └── stores/            # Svelte stores
├── admin/                     # Admin panel (SvelteKit, Cloudflare Pages)
├── desktop/                   # Tauri desktop app
│   └── src-tauri/             # Tauri Rust backend
├── deploy/                    # Server deployment scripts
├── Docs/                      # Design specification documents
└── scripts/                   # Build/deploy scripts
```

## Building and Running

### Prerequisites

- [Rust](https://rustup.rs/) (stable, 1.93+)
- [Bun](https://bun.sh/) (v1.0+)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- Node.js (for Tauri, if building desktop app)

### Development Workflow

```bash
# Clone and enter project
git clone https://github.com/KodyDennon/GlobalTelco.git
cd GlobalTelco

# Build WASM module (required before running frontend)
wasm-pack build crates/gt-wasm --target web --out-dir ../../web/src/lib/wasm/pkg

# Frontend dev server (hot reload)
cd web && bun install && bun run dev
# Opens at http://localhost:5173

# Rust development
cargo build                    # Debug build
cargo test                     # Run all tests
cargo build --release          # Release build
cargo fmt                      # Format code
cargo check                    # Type check (0 errors, 0 warnings required)

# Multiplayer server
cargo run --bin gt-server      # Run locally on port 3001

# Desktop app (Tauri)
cd desktop && cargo tauri dev  # Dev mode
cd desktop && cargo tauri build # Production build

# Admin panel
cd admin && bun install && bun run dev  # Dev server on port 5174
```

### Full Build Pipeline

```bash
# Production-ready build
wasm-pack build crates/gt-wasm --target web --out-dir ../../web/src/lib/wasm/pkg
cd web && bun run build

# Docker build (for server deployment)
docker build -t globaltelco-server .
```

### Environment Configuration

Copy `.env.example` to `.env` and configure:

```bash
# Server configuration
GT_HOST=0.0.0.0
GT_PORT=3001
GT_JWT_SECRET=<generate-with-openssl-rand-base64-32>
GT_MAX_PLAYERS=8

# PostgreSQL (optional for local dev)
DATABASE_URL=postgresql://user:password@host/dbname?sslmode=require

# Frontend
PUBLIC_API_URL=http://localhost:3001
PUBLIC_WS_URL=ws://localhost:3001/ws
```

## Development Conventions

### Code Quality Rules

- **No stubs or placeholders** — all features must be fully coded and working
- **Deterministic simulation** — same inputs must produce same outputs
- **Crate isolation** — crates communicate through defined public APIs only
- **End-to-end integration** — no partial systems merged to main
- **0 errors, 0 warnings** — both `cargo check` and `bun run check` must be clean

### Code Style

- **Rust:** Standard `rustfmt` formatting. Run `cargo fmt` before committing.
- **TypeScript/Svelte:** Svelte 5 syntax (`$state`, `$derived`, `$effect`). Follow the Bloomberg Terminal dark theme aesthetic.
- **Commits:** Descriptive messages. Prefer atomic commits.

### Testing Practices

```bash
# Rust tests
cargo test

# Frontend type check
cd web && bun run check

# Full verification
cargo check && cargo test && cd web && bun run check
```

### Branch Strategy

- `main` — production-ready
- `dev` — active development
- Feature branches merged via PRs

## Architecture Overview

### ECS Tick Order (36 Systems)

1. construction → 2. orbital → 3. satellite_network → 4. maintenance → 5. population → 6. coverage → 7. demand → 8. routing → 9. utilization → 10. spectrum → 11. ftth → 12. manufacturing → 13. launch → 14. terminal_distribution → 15. satellite_revenue → 16. revenue → 17. cost → 18. finance → 19. contract → 20. ai → 21. weather → 22. disaster → 23. debris → 24. servicing → 25. regulation → 26. research → 27. patent → 28. market → 29. auction → 30. covert_ops → 31. lobbying → 32. alliance → 33. legal → 34. grants → 35. achievement → 36. stock_market

### WASM Bridge Pattern

```
Single-player: Svelte Component → bridge.ts → gt-wasm (wasm-bindgen) → ECS World
Multiplayer:   Svelte Component → commandRouter.ts → WebSocketClient → Server
Desktop:       Same as single-player (WASM in Tauri webview), Tauri IPC for native filesystem
```

### Key Commands

```
Commands: 61 total (build_node, build_edge, hire_employee, take_loan, start_research, etc.)
Queries: JSON queries (get_visible_entities, get_corporation_data, etc.) + Typed arrays (hot-path)
```

## Key Files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Rust workspace configuration |
| `web/package.json` | Frontend dependencies and scripts |
| `admin/package.json` | Admin panel dependencies |
| `desktop/src-tauri/Cargo.toml` | Tauri app configuration |
| `.env.example` | Environment variable template |
| `Dockerfile` | Server container build |
| `Docs/technical_architecture.md` | Comprehensive architecture documentation |
| `Docs/game_design_decisions.md` | Definitive gameplay/design reference |
| `CLAUDE.md` | Detailed development guidance |
| `CONTRIBUTING.md` | Contribution guidelines |

## Performance Targets

- Simulation tick: < 50ms for 10,000+ entities
- Map rendering: 60fps at all zoom levels
- WASM module: < 5MB gzipped
- Initial page load: < 3 seconds
- WebSocket latency: < 100ms round-trip
- Memory: < 500MB in browser

## Deployment

### Current Production

```
Players ──► globaltelco.online (Vercel CDN, Cloudflare DNS)
                │
Players ──► server.globaltelco.online (Cloudflare proxy, SSL)
                │
                ▼
           Oracle Cloud (nginx → Rust game server)
                │
                ▼
           Oracle Managed PostgreSQL
```

### Deploy Commands

```bash
# Deploy server to Oracle Cloud
ORACLE_IP=<ip> ./deploy/deploy.sh

# Deploy admin panel to Cloudflare Pages
cd admin && npx wrangler pages deploy build --project-name=globaltelco-admin

# Frontend auto-deploys to Vercel on push to main
```

## Important Notes

- The existing `QWEN.md` file was empty — this file replaces it with comprehensive context
- This project uses a custom **Source Available** license (see `LICENSE`)
- Commercial use requires written permission
- The simulation is fully deterministic for multiplayer sync
- AI corporations use all game systems autonomously
