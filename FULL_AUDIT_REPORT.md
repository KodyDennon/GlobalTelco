# GlobalTelco Codebase Audit Report
**Date:** March 6, 2026

## Executive Summary
- **Rust Backend:** 🚨 **CRITICAL**. The simulation engine is currently broken with **19 compilation errors** and **74 warnings**. The errors are primarily due to type mismatches (`HashMap` vs `IndexMap`) and trait bound issues with `indexmap` updates.
- **Web Frontend:** ✅ **PASSING**. No errors or warnings found.
- **Admin Panel:** ⚠️ **WARNING**. **1 Type Error** preventing full type safety, and **48 Accessibility/Linter Warnings**.

---

## 1. Rust Backend (`crates/`)

### 🔴 Errors (Blocking Compilation)

#### A. Type Mismatches (`HashMap` vs `IndexMap`)
The codebase seems to be in a transition between `HashMap` and `IndexMap` (likely for determinism), but some parts are inconsistent.

- **`crates/gt-simulation/src/systems/mod.rs:295`**
  - Expected: `IndexMap<String, u64>`
  - Found: `HashMap<String, u64>`
- **`crates/gt-simulation/src/world/commands_infra.rs:785`**
  - Expected: `IndexMap<u64, bool>`
  - Found: `HashMap<u64, bool>`

#### B. `indexmap` Trait Bound Failures (`E0277`)
Newer versions of `indexmap` are stricter about key types. There are widespread errors where `&u64` is being used incorrectly with `get()` or `contains_key()`.

- **Locations:**
  - `crates/gt-simulation/src/systems/ai/satellite.rs`
  - `crates/gt-simulation/src/systems/maintenance.rs`
  - `crates/gt-simulation/src/systems/orbital.rs`
  - `crates/gt-simulation/src/systems/routing.rs`
  - `crates/gt-simulation/src/systems/satellite_revenue.rs`
  - `crates/gt-simulation/src/systems/spectrum.rs`
  - `crates/gt-simulation/src/systems/utilization.rs`

#### C. Simple Type Mismatches
- **`crates/gt-simulation/src/systems/ai/satellite.rs:406`**
  - Expected: `u64`
  - Found: `&u64` (Needs dereferencing `*sat_id`)

### 🟡 Warnings
- **Deprecation:** `indexmap::IndexMap::remove` is deprecated.
  - **Action:** Replace with `swap_remove` (faster, changes order) or `shift_remove` (preserves order). Since this is a deterministic simulation, **order preservation is likely critical**, so `shift_remove` might be safer unless the order doesn't matter for that specific map.

---

## 2. Admin Panel (`admin/`)

### 🔴 Errors
- **`src/routes/multiplayer/+page.svelte:49:15`**
  - `Property 'worlds' does not exist on type...`
  - *Context:* TypeScript cannot infer the type of the response from the API correctly.

### 🟡 Warnings
- **Accessibility (A11y):** 48 warnings.
  - Missing `aria-label` or `<label>` for form inputs.
  - Interactive elements (`div`, `svg`) with click handlers missing keyboard events.
- **Svelte 5 State:**
  - `src/lib/components/ConfigEditor.svelte`: "This reference only captures the initial value of `value`. Did you mean to reference it inside a derived instead?"

---

## 3. Web Frontend (`web/`)
- **Status:** Clean. 0 errors, 0 warnings.

## Recommended Action Plan

1.  **Fix Rust Compilation Errors:**
    -   Resolve `HashMap` vs `IndexMap` inconsistencies.
    -   Fix `indexmap` key referencing (`&id` vs `id`).
    -   Fix simple type mismatches.
2.  **Fix Admin Type Error:**
    -   Correct the type casting in `admin/src/routes/multiplayer/+page.svelte`.
3.  **Refactor Deprecated Rust Methods:**
    -   Replace `remove()` with `shift_remove()` (safer default for determinism) or `swap_remove()` (if order is proven irrelevant).
4.  **Address Admin Warnings:**
    -   Fix the Svelte 5 state reactivity issue.
    -   (Optional) Clean up A11y warnings.
