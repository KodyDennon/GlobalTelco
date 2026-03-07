# GlobalTelco Fix Plan

## Phase 1: Rust Simulation Integrity (Critical)
**Goal:** Fix compilation errors and ensure deterministic behavior in `gt-simulation` and `gt-infrastructure`.

1.  **Standardize Collections:**
    *   Convert `HashMap` to `IndexMap` in `crates/gt-simulation/src/systems/mod.rs` and `crates/gt-simulation/src/world/commands_infra.rs` to match the engine standard.
    *   *Reason:* Fixes type mismatch errors `E0308`.

2.  **Fix Trait Bounds (`IndexMap` Refactor):**
    *   Update `get`, `contains_key`, and `remove` calls to use `&key` or `key` correctly (dereferencing `&u64` to `u64` where needed) in:
        *   `gt-simulation/src/systems/ai/satellite.rs`
        *   `gt-simulation/src/systems/maintenance.rs`
        *   `gt-simulation/src/systems/orbital.rs`
        *   `gt-simulation/src/systems/routing.rs`
        *   `gt-simulation/src/systems/satellite_revenue.rs`
        *   `gt-simulation/src/systems/spectrum.rs`
        *   `gt-simulation/src/systems/utilization.rs`

3.  **Resolve Deprecations (Determinism):**
    *   Replace `indexmap::IndexMap::remove` with `shift_remove` to preserve iteration order (O(n)).
    *   *Reason:* Preserves the deterministic behavior of the simulation required for multiplayer sync.

## Phase 2: Admin Panel & API Sync
**Goal:** Fix TypeScript errors and ensure the Admin panel correctly reflects the Backend API.

1.  **API Type Synchronization:**
    *   Inspect `crates/gt-server` to identify the JSON response structure for the `/worlds` (or similar) endpoint.
    *   Update `admin/src/routes/multiplayer/+page.svelte` to match the actual backend response (fix `worlds` property error).

2.  **Component Reactivity:**
    *   Refactor `admin/src/lib/components/ConfigEditor.svelte` to use `$derived` or `$effect` (Svelte 5) to update local state when the `value` prop changes.

3.  **Accessibility (A11y) & Linting:**
    *   Add `onkeydown` handlers to interactive elements in `ConfirmDialog.svelte`, `WorldConfigForm.svelte`, and others.
    *   Add missing `<label>` associations in form fields.

## Phase 3: Final Verification
1.  Run `cargo check --workspace` to confirm zero Rust errors/warnings.
2.  Run `bun run check` in `web` and `admin` to confirm zero frontend errors/warnings.
