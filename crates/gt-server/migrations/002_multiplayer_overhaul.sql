-- Migration 002: Multiplayer Overhaul
-- Run after 001_initial_schema.sql

-- ── Expand accounts table ────────────────────────────────────────────────
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS display_name VARCHAR(64);
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS avatar_id VARCHAR(32) DEFAULT 'tower_01';
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS github_id BIGINT UNIQUE;
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS auth_provider VARCHAR(16) NOT NULL DEFAULT 'local';
  -- auth_provider: 'local', 'github', 'guest'
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_accounts_github_id ON accounts(github_id) WHERE github_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_accounts_deleted ON accounts(deleted_at) WHERE deleted_at IS NOT NULL;

-- ── Password reset tokens ────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS password_reset_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id  UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    token_hash  VARCHAR(255) NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    used_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_password_reset_account ON password_reset_tokens(account_id);

-- ── Manual password reset queue (admin) ──────────────────────────────────
CREATE TABLE IF NOT EXISTS password_reset_requests (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id  UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    username    VARCHAR(64) NOT NULL,
    status      VARCHAR(16) NOT NULL DEFAULT 'pending',  -- pending, resolved, rejected
    resolved_by VARCHAR(64),  -- admin username or 'system'
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_reset_requests_status ON password_reset_requests(status);

-- ── World catalog templates (admin-defined) ──────────────────────────────
CREATE TABLE IF NOT EXISTS world_templates (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(128) NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    icon            VARCHAR(64) NOT NULL DEFAULT 'globe',
    config_defaults JSONB NOT NULL,       -- WorldConfig defaults
    config_bounds   JSONB NOT NULL,       -- min/max per customizable field
    max_instances   INTEGER NOT NULL DEFAULT 5,
    enabled         BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Track which template spawned each world ──────────────────────────────
ALTER TABLE game_worlds ADD COLUMN IF NOT EXISTS template_id UUID REFERENCES world_templates(id) ON DELETE SET NULL;
ALTER TABLE game_worlds ADD COLUMN IF NOT EXISTS created_by UUID REFERENCES accounts(id) ON DELETE SET NULL;
ALTER TABLE game_worlds ADD COLUMN IF NOT EXISTS invite_code VARCHAR(16) UNIQUE;

CREATE INDEX IF NOT EXISTS idx_game_worlds_template ON game_worlds(template_id);
CREATE INDEX IF NOT EXISTS idx_game_worlds_invite ON game_worlds(invite_code) WHERE invite_code IS NOT NULL;

-- ── Friends system ───────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS friendships (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_a   UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    account_b   UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    status      VARCHAR(16) NOT NULL DEFAULT 'pending',  -- pending, accepted, blocked
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT friendships_ordered CHECK (account_a < account_b),
    UNIQUE(account_a, account_b)
);
CREATE INDEX IF NOT EXISTS idx_friendships_a ON friendships(account_a);
CREATE INDEX IF NOT EXISTS idx_friendships_b ON friendships(account_b);

-- ── Friend requests (directional: who sent to whom) ─────────────────────
CREATE TABLE IF NOT EXISTS friend_requests (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_id     UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    to_id       UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    status      VARCHAR(16) NOT NULL DEFAULT 'pending',  -- pending, accepted, rejected
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(from_id, to_id)
);
CREATE INDEX IF NOT EXISTS idx_friend_requests_to ON friend_requests(to_id, status);

-- ── Recent players (who you've been in a world with) ────────────────────
CREATE TABLE IF NOT EXISTS recent_players (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id  UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    other_id    UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    world_id    UUID NOT NULL REFERENCES game_worlds(id) ON DELETE CASCADE,
    last_seen   TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(account_id, other_id, world_id)
);
CREATE INDEX IF NOT EXISTS idx_recent_players_account ON recent_players(account_id, last_seen DESC);

-- ── Expanded ban management ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS bans (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id  UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    world_id    UUID REFERENCES game_worlds(id) ON DELETE CASCADE,  -- NULL = global ban
    reason      TEXT NOT NULL DEFAULT '',
    banned_by   VARCHAR(64) NOT NULL DEFAULT 'admin',
    banned_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ,  -- NULL = permanent

    UNIQUE(account_id, world_id)
);
CREATE INDEX IF NOT EXISTS idx_bans_account ON bans(account_id);

-- ── Server audit log (persistent, replaces in-memory) ───────────────────
CREATE TABLE IF NOT EXISTS audit_log (
    id          BIGSERIAL PRIMARY KEY,
    actor       VARCHAR(128) NOT NULL,  -- admin key hash, player UUID, or 'system'
    action      VARCHAR(64) NOT NULL,
    target      VARCHAR(255),
    details     JSONB,
    ip_address  INET,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_audit_log_created ON audit_log(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_log_actor ON audit_log(actor);

-- ── Recent worlds history (per player) ──────────────────────────────────
CREATE TABLE IF NOT EXISTS world_history (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id  UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    world_id    UUID NOT NULL REFERENCES game_worlds(id) ON DELETE CASCADE,
    world_name  VARCHAR(128) NOT NULL,
    last_played TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(account_id, world_id)
);
CREATE INDEX IF NOT EXISTS idx_world_history_account ON world_history(account_id, last_played DESC);
