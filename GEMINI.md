# GlobalTelco - Project Guide

**GlobalTelco** is a 2D infrastructure empire builder where players build and operate telecom networks on a political map, growing from a local ISP to a global telecom empire. It features a deep simulation of network operations, economics, and corporate competition.

## 🚀 Quick Start

### Prerequisites
- **Rust** (stable)
- **Bun** (v1.0+)
- **wasm-pack**

### Development Commands
```bash
# 1. Build WASM simulation module (Required for Web/Desktop)
wasm-pack build crates/gt-wasm --target web --out-dir ../../web/src/lib/wasm/pkg

# 2. Start Web Frontend
cd web && bun install && bun run dev

# 3. Start Multiplayer Server
cargo run --bin gt-server

# 4. Start Desktop App (Tauri)
cd desktop && cargo tauri dev

# 5. Start Admin Panel
cd admin && bun install && bun run dev
```

---

## 🏗️ Architecture & Tech Stack

- **Simulation Engine:** Rust (ECS architecture).
    - Compiles to **WASM** for the browser (Single-player).
    - Compiles to **Native Binary** for the server (Multiplayer).
- **Frontend:** Svelte 5 + deck.gl (2D map) + D3.js (data viz).
- **Desktop:** Tauri (Rust wrapper for system webview).
- **Backend:** Axum (Rust) with WebSocket for state sync.
- **Database:** PostgreSQL (Neon) for world state and accounts.
- **Build/Runtime:** Bun, wasm-pack, cargo-zigbuild.

### Determinism Requirement
The simulation MUST be deterministic. All state mutations occur through a centralized event queue. Same inputs + Same Tick = Same Output. This is critical for multiplayer synchronization.

---

## 📁 Project Structure

```bash
globaltelco/
├── crates/                    # Rust Workspace (Core Logic)
│   ├── gt-common/             # Shared types, traits, and protocol
│   ├── gt-simulation/         # Core ECS engine (36 systems)
│   ├── gt-world/              # World gen, terrain, geography
│   ├── gt-economy/            # Finance, markets, contracts, AI
│   ├── gt-infrastructure/     # Network graph, nodes, edges, routing
│   ├── gt-wasm/               # WASM bindings (wasm-bindgen)
│   ├── gt-server/             # Multiplayer server (Axum + WebSocket)
│   └── ...                    # Other specialized crates
├── web/                       # Svelte frontend
│   └── src/lib/wasm/          # Bridge to Rust simulation
├── desktop/                   # Tauri desktop application
├── admin/                     # SvelteKit admin dashboard
├── tools/                     # Go-based management tools
├── Docs/                      # Extensive design documentation
└── data/                      # Open data sources (OSM, ESRI)
```

---

## 🛠️ Development Conventions

### Simulation (Rust)
- **ECS (Entity Component System):** Use entities for IDs and components for data. Systems process entities in a strict [deterministic order](CLAUDE.md#ecs-architecture).
- **Commands & Queries:** Players interact with the world via **Commands** (mutations) and **Queries** (read-only state requests).
- **Serialization:** Uses `serde` with `MessagePack` for binary efficiency and `JSON` for debug/WASM interop.

### Frontend (Svelte)
- **Bridge Pattern:** `web/src/lib/wasm/bridge.ts` is the central interface. It proxies calls to either the WASM module (Web) or Tauri IPC (Desktop).
- **Map Rendering:** Uses `deck.gl` for high-performance 2D rendering of infrastructure layers.

### Testing
- **Rust:** `cargo test` (Run from root or crate dir).
- **Frontend:** `cd web && bun test`.
- **Validation:** Always run `cargo check` and `cd web && bun run check` before submitting changes.

---

## 💡 Gemini CLI Tips

- **Validation:** When modifying Rust code, use `cargo check` to catch errors quickly without a full build.
- **WASM Integration:** If you change types in `gt-common` or logic in `gt-simulation`, you MUST rebuild the WASM module using `wasm-pack` for the changes to reflect in the web frontend.
- **Context:** Refer to `CLAUDE.md` for an extremely detailed technical breakdown of systems, commands, and implementation status. It is the "Master Reference" for AI agents in this repo.
- **Documentation:** `Docs/game_design_decisions.md` contains the rationale behind gameplay and technical choices.
