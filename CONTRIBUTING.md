# Contributing to GlobalTelco

Thanks for your interest in contributing! This project welcomes contributions of all kinds — bug reports, feature implementations, documentation improvements, and more.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/KodyDennon/GlobalTelco.git`
3. Create a feature branch: `git checkout -b my-feature`
4. Make your changes
5. Ensure builds pass:
   ```bash
   cargo check           # Rust — 0 errors, 0 warnings
   cargo test            # All tests pass
   cd web && bun run check  # TypeScript — 0 errors
   ```
6. Commit with a descriptive message
7. Push and open a Pull Request against `main`

## Development Rules

- **No stubs or placeholders** — all features must be fully coded and working
- **Deterministic simulation** — same inputs must produce same outputs
- **Crate isolation** — crates communicate through defined public APIs only
- **End-to-end integration** — no partial systems merged to main
- **0 errors, 0 warnings** — both `cargo check` and `bun run check` must be clean

## Code Style

- **Rust:** Standard `rustfmt` formatting. Run `cargo fmt` before committing.
- **TypeScript/Svelte:** Use Svelte 5 syntax (`$state`, `$derived`, `$effect`). Follow the existing Bloomberg Terminal dark theme aesthetic for UI components.
- **Commits:** Descriptive messages. Prefer atomic commits that each do one thing.

## Architecture Notes

- The simulation runs as an ECS (Entity Component System) with 20 deterministic systems
- The same Rust code compiles to WASM (browser single-player) and native binary (multiplayer server)
- Frontend communicates with the sim through a typed bridge (`web/src/lib/wasm/bridge.ts`)
- All state mutations flow through the centralized event queue

See `CLAUDE.md` for detailed architecture documentation and `Docs/` for design specifications.

## Contributor License

By submitting a contribution, you agree to the terms in [LICENSE](LICENSE) Section 4 — you grant the project maintainer a license to use your contribution as part of the project.

## Reporting Issues

- Use GitHub Issues for bug reports and feature requests
- Include steps to reproduce for bugs
- Check existing issues before opening a new one

## Questions?

Open a GitHub Discussion or Issue. We're happy to help.
