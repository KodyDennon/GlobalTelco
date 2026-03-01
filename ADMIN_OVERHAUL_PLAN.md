# Admin Dashboard Full Rebuild — Implementation Plan

## Context

The current admin dashboard (`web/src/routes/admin/+page.svelte`) is a single 1,553-line page with:
- **3 breaking integration bugs** (ban field mismatch, audit log type mismatch, metrics type mismatch)
- **8 missing WorldConfig fields** in world creation
- **Completely hollow templates** (config_defaults/config_bounds always sent as `{}`)
- **No real-time updates**, no search/filter, no pagination
- **Missing features**: account management, world detail views, multiplayer connection management, chat history, per-system metrics, server config viewer
- **Poor UX**: single scroll page, no navigation, no confirmations, no copy-to-clipboard, redundant controls

We're building a **separate SvelteKit app** (`admin/`) with sidebar navigation, auto-refresh polling, full world/player/multiplayer management, and backend fixes. Types will be shared. All current features will be preserved and expanded.

---

## Architecture Overview

```
globaltelco/
├── admin/                     # NEW — Separate SvelteKit admin app
│   ├── src/
│   │   ├── routes/
│   │   │   ├── +layout.svelte       # Sidebar layout + auth gate
│   │   │   ├── +layout.ts           # SPA config (prerender, no SSR)
│   │   │   ├── +page.svelte         # Login page (when not authed)
│   │   │   ├── overview/+page.svelte
│   │   │   ├── worlds/+page.svelte
│   │   │   ├── worlds/[id]/+page.svelte  # World detail
│   │   │   ├── players/+page.svelte
│   │   │   ├── multiplayer/+page.svelte
│   │   │   ├── monitoring/+page.svelte
│   │   │   ├── audit/+page.svelte
│   │   │   └── settings/+page.svelte
│   │   ├── lib/
│   │   │   ├── api/              # Admin API client (typed, from existing api.ts + new endpoints)
│   │   │   │   ├── client.ts     # Base fetch wrapper with admin key header
│   │   │   │   ├── health.ts     # Health + server info
│   │   │   │   ├── worlds.ts     # World CRUD + debug + templates
│   │   │   │   ├── players.ts    # Connected players + accounts + bans
│   │   │   │   ├── multiplayer.ts # Connections, chat, speed votes
│   │   │   │   ├── metrics.ts    # Server + world + per-system metrics
│   │   │   │   ├── audit.ts      # Audit log (paginated, filtered)
│   │   │   │   ├── settings.ts   # Server config, broadcast, reset queue
│   │   │   │   └── types.ts      # All shared TypeScript types
│   │   │   ├── stores/
│   │   │   │   ├── auth.ts       # Admin key + authed state (sessionStorage)
│   │   │   │   ├── polling.ts    # Auto-refresh manager (configurable interval)
│   │   │   │   └── preferences.ts # Admin UI prefs (localStorage)
│   │   │   ├── components/
│   │   │   │   ├── Sidebar.svelte
│   │   │   │   ├── StatCard.svelte
│   │   │   │   ├── DataTable.svelte    # Sortable, searchable, paginated
│   │   │   │   ├── Sparkline.svelte    # Pure SVG sparkline
│   │   │   │   ├── ConfirmDialog.svelte
│   │   │   │   ├── Toast.svelte        # Toast notification system
│   │   │   │   ├── Badge.svelte
│   │   │   │   ├── SearchInput.svelte
│   │   │   │   ├── CopyButton.svelte
│   │   │   │   ├── EmptyState.svelte
│   │   │   │   ├── LoadingSkeleton.svelte
│   │   │   │   └── ConfigEditor.svelte # JSON editor for template configs
│   │   │   └── config.ts         # API_URL detection (same pattern as web/)
│   │   ├── app.html
│   │   ├── app.css               # Copied from web/src/app.css (shared design system)
│   │   └── app.d.ts
│   ├── package.json
│   ├── svelte.config.js
│   ├── vite.config.ts
│   └── tsconfig.json
├── web/                          # Existing — remove admin files when ready
│   ├── src/lib/admin/            # DELETE after admin app is working
│   └── src/routes/admin/         # DELETE after admin app is working
└── crates/gt-server/             # Backend fixes + new endpoints
```

---

## Phase 1: Project Scaffolding + Backend Fixes

### 1A. Create `admin/` SvelteKit project

**New files:**
- `admin/package.json` — Minimal deps: `@sveltejs/kit`, `svelte`, `vite`, `typescript`, `@sveltejs/adapter-static`
- `admin/svelte.config.js` — Copy from web/, same static adapter config
- `admin/vite.config.ts` — Simplified (no deck.gl/maplibre/d3 chunks)
- `admin/tsconfig.json` — Copy from web/
- `admin/src/app.html` — Same structure, "GlobalTelco Admin" title
- `admin/src/app.css` — Copy from `web/src/app.css` (full design system with CSS variables)
- `admin/src/app.d.ts` — Same globals
- `admin/src/routes/+layout.ts` — `prerender = true, ssr = false`

### 1B. Fix backend breaking bugs

**File: `crates/gt-server/src/routes/admin.rs`**

1. **Fix BanRequest struct** (line 406):
```rust
// BEFORE (broken):
struct BanRequest { world_id: Uuid, player_id: Uuid }

// AFTER (matches frontend):
struct BanRequest {
    account_id: Uuid,          // renamed from player_id
    reason: String,            // NEW — actually use the reason
    #[serde(default)]
    world_id: Option<Uuid>,    // NOW OPTIONAL — supports global bans
    #[serde(default)]
    expires_at: Option<String>, // NEW — expiration support
}
```
Update `admin_ban_player()` to pass `body.reason` and parsed `expires_at` to `db.create_ban()`.

2. **Fix audit log response format** — The DB audit returns `{ id, actor, action, target, details, ip_address, created_at }` but the in-memory fallback returns `{ tick, player_id, command_type, timestamp }`. Normalize both to the DB format. Update the in-memory `AuditEntry` struct to match.

3. **Fix metrics response** — Add missing fields the frontend expects:
   - Add `max_tick_us` tracking (track max across last N ticks)
   - Add `p99_tick_us` tracking (keep last 100 tick durations, compute percentile)
   - Return `memory_mb` (convert from bytes) alongside `memory_estimate_bytes`
   - Add `ws_messages_per_sec` counter to state (increment on each WS message, compute rate)

### 1C. Add new backend endpoints

**File: `crates/gt-server/src/routes/admin.rs`** (add to existing)

1. **`GET /api/admin/accounts`** — Paginated account list
   - Query params: `?search=&page=0&per_page=50&sort=created_at&order=desc`
   - Returns: `{ accounts: [...], total: N, page: N, per_page: N }`
   - Each account: `{ id, username, email, display_name, avatar_id, auth_provider, is_guest, is_banned, created_at, last_login, deleted_at }`

2. **`GET /api/admin/connections`** — Active WebSocket connections
   - Returns: connected players + online presence data merged
   - `{ connections: [{ id, username, world_id, world_name, corp_id, is_guest, is_spectator, connected_at, ip_address }] }`

3. **`GET /api/admin/worlds/{id}/chat`** — Chat history (requires chat persistence, see 1D)
   - Query params: `?limit=100&before=<timestamp>`
   - Returns: `{ messages: [{ id, username, message, created_at }] }`

4. **`POST /api/admin/worlds/{id}/assign`** — Force-assign player to corp
   - Body: `{ player_id, corp_id }`

5. **`POST /api/admin/worlds/{id}/spectator`** — Toggle spectator mode
   - Body: `{ player_id, spectator: bool }`

6. **`POST /api/admin/worlds/{id}/transfer`** — Transfer world ownership
   - Body: `{ new_owner_id }`

7. **`GET /api/admin/worlds/{id}/votes`** — Current speed votes
   - Returns: `{ votes: { player_id: speed_string }, current_speed, creator_id }`

8. **`GET /api/admin/config`** — Server configuration status
   - Returns: `{ env_vars: { ADMIN_KEY: true, DATABASE_URL: true, ... }, database: { connected: bool, pool_size: N }, features: { postgres: bool, r2: bool } }`

**File: `crates/gt-server/src/db/accounts.rs`** (add):
- `list_accounts(search, limit, offset, sort, order)` — Paginated account query

**File: `crates/gt-server/src/db/mod.rs`** (add):
- `chat_messages` table support (see 1D)

### 1D. Add chat message persistence

**New file: `crates/gt-server/src/db/chat.rs`**
- `insert_chat_message(world_id, account_id, username, message)`
- `list_chat_messages(world_id, limit, before_timestamp)`

**Modify: `crates/gt-server/src/ws/handler.rs`**
- After chat broadcast, also persist to DB (non-blocking spawn)

**Migration SQL** (for reference, applied manually):
```sql
CREATE TABLE IF NOT EXISTS chat_messages (
    id BIGSERIAL PRIMARY KEY,
    world_id UUID NOT NULL,
    account_id UUID NOT NULL,
    username VARCHAR(64) NOT NULL,
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_chat_world_time ON chat_messages (world_id, created_at DESC);
```

### 1E. Add per-system tick profiling

**Modify: `crates/gt-simulation/src/world/mod.rs`**
- Add `pub system_times: HashMap<&'static str, u64>` to GameWorld (microseconds per system last tick)
- In `tick()`, wrap each system call with `Instant::now()` / `elapsed()`
- Store results in `system_times`

**Modify: `crates/gt-server/src/routes/admin.rs`** (`admin_metrics`):
- Include `system_times` in per-world metrics response

### 1F. Expand audit logging

**Modify: `crates/gt-server/src/routes/admin.rs`**
- Add audit log calls to: `admin_create_world`, `admin_set_speed`, `admin_broadcast`, `admin_create_template`, `admin_update_template`, `admin_delete_template`, `admin_resolve_reset`

### 1G. Add tick history tracking for sparklines

**Modify: `crates/gt-server/src/state.rs`** (WorldInstance):
- Add `tick_history: RwLock<VecDeque<u64>>` — Last 60 tick durations
- In `record_tick_duration()`, push to history, cap at 60

**Modify metrics endpoint:**
- Include `tick_history: Vec<u64>` per world

---

## Phase 2: Admin App Core — Layout, Auth, Routing

### 2A. Auth system

**`admin/src/lib/stores/auth.ts`**
- Same pattern as existing `web/src/lib/admin/store.ts`
- `adminKey` writable (sessionStorage-backed)
- `adminAuthed` derived
- `clearAdmin()` function
- Add `loginAttempts` counter for rate limiting (exponential backoff: 1s, 2s, 4s, 8s, 16s, 30s max)

### 2B. API client

**`admin/src/lib/api/client.ts`**
- Base `adminFetch(path, options)` wrapper
- Automatically adds `X-Admin-Key` header
- Handles 401 (auto-logout), 503 (admin not configured), network errors
- Returns typed responses

**`admin/src/lib/api/types.ts`**
- All TypeScript interfaces for API responses
- Properly typed to match actual backend responses (fixing the current mismatches)

Split into domain modules (`health.ts`, `worlds.ts`, `players.ts`, etc.) for clean imports.

### 2C. Sidebar layout

**`admin/src/routes/+layout.svelte`**
- Auth gate: if not authed, show login page
- If authed: sidebar + content area
- Sidebar sections: Overview, Worlds, Players, Multiplayer, Monitoring, Audit Log, Settings
- Active route highlighting
- Collapse toggle for sidebar
- Auto-refresh indicator + toggle in header
- Logout button in sidebar footer

### 2D. Auto-refresh polling

**`admin/src/lib/stores/polling.ts`**
- Configurable interval (default 10s)
- Per-section polling (health every 10s, players every 5s, metrics every 10s)
- Pause when tab is not visible (`document.hidden`)
- Toggle on/off from header
- Stale data indicator (show how old the data is)

### 2E. Toast notification system

**`admin/src/lib/components/Toast.svelte`**
- Stack-based toasts (bottom-right)
- Types: success, error, warning, info
- Auto-dismiss (5s) with manual close
- Used for all action confirmations (kick, ban, create world, etc.)

---

## Phase 3: Dashboard Pages

### 3A. Overview page (`/overview`)

**Stats cards row:**
- Server version, Uptime, Active Worlds, Connected Players, Registered Accounts, DB status
- Each card shows current value + sparkline of recent history where applicable

**Recent activity feed:**
- Last 10 audit log entries
- Formatted as timeline (icon + action + target + time ago)

**Quick actions:**
- Create World button
- Broadcast Message button

### 3B. Worlds page (`/worlds`)

**World list (DataTable):**
- Columns: Name, Players, Tick, Era, Map Size, Speed, Created, Actions
- Sortable columns, search by name
- Row click → navigate to world detail
- Actions: Speed dropdown, Pause/Resume, Delete (with confirm)

**Create World form (expandable):**
- ALL 14 WorldConfig fields exposed:
  - Name, Max Players
  - Type toggle (Procedural / Real Earth)
  - Era, Difficulty, Map Size (selects)
  - AI Corps, Max AI Corps (number inputs)
  - Seed (number + randomize button)
  - Continent Count (1-8 slider)
  - Ocean Percentage (0.3-0.9 slider)
  - Terrain Roughness (0.0-1.0 slider)
  - Climate Variation (0.0-1.0 slider)
  - City Density (0.0-1.0 slider)
  - Disaster Frequency (0.1-3.0 slider)
  - Sandbox toggle
- Success toast on creation

**Templates section (collapsible):**
- Template list with enable/disable toggle, edit button, delete button
- Create template form with **config_defaults editor** (JSON form fields or raw JSON)
- **config_bounds editor** (define min/max/allowed per field)
- Sort order control
- Preview: show what the player-facing WorldCreator would look like

### 3C. World detail page (`/worlds/[id]`)

Uses the existing `/api/admin/debug/{id}` endpoint (currently unused).

**Header:** World name, ID, speed, tick count, uptime
**Tabs:**
1. **Corporations** — Table of all corps: name, cash, revenue, costs, debt, node count. Color-coded profit/loss.
2. **Players** — Connected players in this world with kick/ban/spectator buttons
3. **Entities** — Count breakdown: corps, nodes, edges, regions, cities
4. **Config** — Read-only display of world config
5. **Chat** — Chat history (from new endpoint) with search
6. **Speed Votes** — Current votes from all players, creator override indicator

### 3D. Players page (`/players`)

**Tabs:**
1. **Connected** — Currently connected players (auto-refreshed)
   - DataTable: Username, World, Corp, Status badges (admin/guest/registered/spectator), Connection duration
   - Actions: Kick, Ban (with reason/expiry dialog), View Profile
   - Search/filter by username, world

2. **All Accounts** — Full account list from DB (new endpoint)
   - DataTable: Username, Email, Auth Provider, Created, Last Login, Status (active/banned/deleted)
   - Search, sort by any column, pagination
   - Actions: Ban, View Profile, Reset Password

3. **Bans** — Active ban management
   - DataTable: Username, Reason, Scope (Global/World), Banned At, Expires At
   - Create Ban: Player search/autocomplete (from accounts), reason, optional world scope, optional expiry
   - Unban button with confirmation

### 3E. Multiplayer page (`/multiplayer`)

**Tabs:**
1. **Connections** — Active WebSocket connections (new endpoint)
   - Username, World, IP, Connected Duration, Messages Sent
   - Disconnect button

2. **Speed Votes** — Per-world speed vote status (new endpoint)
   - Show each world's current votes, who voted what
   - Admin override button

3. **World Ownership** — Transfer world creator/owner (new endpoint)
   - List worlds with current owner
   - Transfer button → select new owner from connected players

4. **Force Assign** — Assign player to corp (new endpoint)
   - Select world → select player → select corp (or create new)

### 3F. Monitoring page (`/monitoring`)

**Server metrics row (sparklines):**
- Memory usage (MB) + sparkline
- Connected players + sparkline
- WebSocket messages/sec + sparkline
- Uptime

**Per-world table:**
- World name, Avg tick (us), Max tick (us), P99 tick (us), Entity count
- Click to expand: tick history sparkline + per-system breakdown

**Per-system breakdown (expandable per world):**
- Bar chart or sorted list of 36 systems with their last tick duration
- Highlight systems over threshold (e.g., >5ms)
- Color gradient: green (fast) → red (slow)

### 3G. Audit Log page (`/audit`)

**Full-featured log viewer:**
- DataTable with: Timestamp, Actor, Action, Target, Details (expandable), IP
- Filters: action type dropdown, actor filter, date range picker
- Pagination (backend supports it, currently unused)
- Export button (download as CSV/JSON)
- Auto-refresh toggle

### 3H. Settings page (`/settings`)

**Tabs:**
1. **Server Config** — Environment variable status (set/unset indicators)
   - ADMIN_KEY, DATABASE_URL, JWT_SECRET, GITHUB_CLIENT_ID, GITHUB_CLIENT_SECRET, TILE_DIR, R2 config
   - Database connection status + pool info
   - Feature flags (postgres, r2)

2. **Broadcast** — Send messages to players
   - Message input, world selector (all or specific)
   - Message history (from audit log, filter action=broadcast)

3. **Reset Queue** — Pending password reset requests
   - DataTable: Username, Status, Created At
   - Generate Temp Password button with confirmation
   - Copy-to-clipboard for generated password

4. **Admin Preferences**
   - Auto-refresh interval (5s, 10s, 30s, 60s)
   - Auto-refresh on/off
   - Theme preference (if we add light mode later)

---

## Phase 4: Shared Components

### DataTable component
- Props: columns config, data array, searchable, sortable, paginated
- Built-in search input, column sort arrows, page controls
- Loading skeleton state
- Empty state with custom message
- Row click handler

### Sparkline component
- Props: data (number[]), width, height, color
- Pure SVG (no D3 dependency)
- Optional min/max labels
- Hover tooltip with value

### ConfirmDialog component
- Props: title, message, confirmLabel, cancelLabel, variant (danger/warning/info)
- Promise-based: `const confirmed = await confirm("Delete world?")`

### StatCard component
- Props: label, value, unit, sparklineData, trend (up/down/flat), color
- Shows value prominently with optional sparkline below

### Other components
- `Badge.svelte` — Colored badge (green, red, blue, amber, gray)
- `SearchInput.svelte` — Search with debounce
- `CopyButton.svelte` — Click to copy, shows "Copied!" feedback
- `EmptyState.svelte` — Icon + message + optional action button
- `LoadingSkeleton.svelte` — Animated placeholder blocks
- `ConfigEditor.svelte` — Form-based JSON editor for template config_defaults/config_bounds

---

## Phase 5: Polish + Cleanup

1. **Login page hardening:**
   - Exponential backoff on failed attempts (1s, 2s, 4s, 8s... up to 30s)
   - Failed attempt counter shown
   - Cloudflare Turnstile placeholder (ready to add later)

2. **Responsive layout:**
   - Sidebar collapses to icons on narrow screens
   - Tables switch to card layout on mobile
   - Min-width: 768px (admin is primarily desktop)

3. **Keyboard shortcuts:**
   - `Ctrl+K` → global search
   - `Escape` → close dialogs
   - `R` → refresh current section

4. **Remove old admin from web/:**
   - Delete `web/src/lib/admin/` (api.ts, store.ts)
   - Delete `web/src/routes/admin/`
   - Clean up any imports

5. **CORS configuration:**
   - Backend needs to allow the admin app's origin in CORS headers
   - Add `admin.globaltelco.online` (or localhost:5174 for dev) to allowed origins

---

## File Manifest — Backend Changes

| File | Action | Changes |
|------|--------|---------|
| `crates/gt-server/src/routes/admin.rs` | Modify | Fix BanRequest, fix audit format, add 8 new endpoints, expand audit logging |
| `crates/gt-server/src/routes/mod.rs` | Modify | Register new admin routes, update CORS |
| `crates/gt-server/src/state.rs` | Modify | Add tick_history, ws_message_counter, max_tick_us, p99 tracking |
| `crates/gt-server/src/db/accounts.rs` | Modify | Add `list_accounts()` paginated query |
| `crates/gt-server/src/db/chat.rs` | Create | Chat message persistence (insert + query) |
| `crates/gt-server/src/db/mod.rs` | Modify | Add `chat` submodule |
| `crates/gt-server/src/ws/handler.rs` | Modify | Persist chat messages to DB after broadcast |
| `crates/gt-simulation/src/world/mod.rs` | Modify | Add `system_times` HashMap, wrap each system in timer |

## File Manifest — Admin App (New)

| File | Purpose |
|------|---------|
| `admin/package.json` | Minimal SvelteKit deps |
| `admin/svelte.config.js` | Static adapter config |
| `admin/vite.config.ts` | Simple Vite config |
| `admin/tsconfig.json` | TypeScript config |
| `admin/src/app.html` | HTML shell |
| `admin/src/app.css` | Design system (from web/) |
| `admin/src/app.d.ts` | Global types |
| `admin/src/routes/+layout.svelte` | Sidebar layout + auth gate |
| `admin/src/routes/+layout.ts` | SPA config |
| `admin/src/routes/+page.svelte` | Login redirect |
| `admin/src/routes/overview/+page.svelte` | Overview dashboard |
| `admin/src/routes/worlds/+page.svelte` | World management |
| `admin/src/routes/worlds/[id]/+page.svelte` | World detail |
| `admin/src/routes/players/+page.svelte` | Player management |
| `admin/src/routes/multiplayer/+page.svelte` | Multiplayer controls |
| `admin/src/routes/monitoring/+page.svelte` | Metrics & performance |
| `admin/src/routes/audit/+page.svelte` | Audit log viewer |
| `admin/src/routes/settings/+page.svelte` | Settings & config |
| `admin/src/lib/api/client.ts` | Base fetch wrapper |
| `admin/src/lib/api/types.ts` | All TypeScript types |
| `admin/src/lib/api/health.ts` | Health API |
| `admin/src/lib/api/worlds.ts` | Worlds API |
| `admin/src/lib/api/players.ts` | Players API |
| `admin/src/lib/api/multiplayer.ts` | Multiplayer API |
| `admin/src/lib/api/metrics.ts` | Metrics API |
| `admin/src/lib/api/audit.ts` | Audit API |
| `admin/src/lib/api/settings.ts` | Settings API |
| `admin/src/lib/stores/auth.ts` | Auth state |
| `admin/src/lib/stores/polling.ts` | Auto-refresh |
| `admin/src/lib/stores/preferences.ts` | UI preferences |
| `admin/src/lib/components/Sidebar.svelte` | Navigation sidebar |
| `admin/src/lib/components/DataTable.svelte` | Reusable table |
| `admin/src/lib/components/Sparkline.svelte` | SVG sparkline |
| `admin/src/lib/components/ConfirmDialog.svelte` | Confirmation modal |
| `admin/src/lib/components/Toast.svelte` | Toast notifications |
| `admin/src/lib/components/Badge.svelte` | Status badges |
| `admin/src/lib/components/SearchInput.svelte` | Debounced search |
| `admin/src/lib/components/CopyButton.svelte` | Copy to clipboard |
| `admin/src/lib/components/EmptyState.svelte` | Empty state |
| `admin/src/lib/components/LoadingSkeleton.svelte` | Loading skeleton |
| `admin/src/lib/components/StatCard.svelte` | Stat card with sparkline |
| `admin/src/lib/components/ConfigEditor.svelte` | Template config editor |
| `admin/src/lib/config.ts` | API URL detection |

---

## Build & Dev Commands

```bash
# Admin app dev
cd admin && bun install && bun run dev   # → localhost:5174

# Admin app build
cd admin && bun run build                # → admin/build/

# Backend (no changes to build command)
cargo build --release --features postgres
```

## Verification

1. **Backend fixes:** `cargo test` passes, `cargo build --features postgres` clean
2. **Admin app builds:** `cd admin && bun run build` succeeds, `bun run check` clean
3. **Integration test:** Start server locally → open admin app → login → verify each page loads data
4. **Ban fix:** Create a ban with reason + expiry → verify stored in DB with correct fields
5. **Audit log:** Perform admin actions → verify all show in audit log with correct format
6. **Metrics:** Check that max_tick_us, p99_tick_us, system_times appear in monitoring page
7. **Templates:** Create template with config_defaults/bounds → verify WorldCreator renders controls
8. **Chat history:** Send chat messages in a world → verify they appear in admin world detail chat tab
9. **Accounts:** Search accounts, sort by columns, paginate through results
10. **Auto-refresh:** Verify polling updates data without manual refresh, pauses when tab hidden

## Deployment Notes

- Admin app deploys separately (Vercel or Cloudflare Pages)
- Point `admin.globaltelco.online` DNS to admin app deployment
- Backend CORS must include admin app origin
- Same `ADMIN_KEY` env var authenticates both old and new admin
- Old admin at `/admin` can be removed once new app is verified
