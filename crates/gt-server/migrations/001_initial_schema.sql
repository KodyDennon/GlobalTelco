-- GlobalTelco Database Schema
-- Migration 001: Initial schema

-- ── Accounts ──────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS accounts (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username    VARCHAR(64) NOT NULL UNIQUE,
    email       VARCHAR(255) UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login  TIMESTAMPTZ,
    is_guest    BOOLEAN NOT NULL DEFAULT FALSE,
    is_banned   BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_accounts_username ON accounts(username);
CREATE INDEX IF NOT EXISTS idx_accounts_email ON accounts(email);

-- ── Game Worlds ───────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS game_worlds (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        VARCHAR(128) NOT NULL,
    config_json JSONB NOT NULL,
    current_tick BIGINT NOT NULL DEFAULT 0,
    speed       VARCHAR(16) NOT NULL DEFAULT 'Paused',
    status      VARCHAR(16) NOT NULL DEFAULT 'active',  -- active, paused, archived
    max_players INTEGER NOT NULL DEFAULT 8,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_game_worlds_status ON game_worlds(status);

-- ── Player Sessions ───────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS player_sessions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id  UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    world_id    UUID NOT NULL REFERENCES game_worlds(id) ON DELETE CASCADE,
    corp_id     BIGINT NOT NULL,
    joined_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_active TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_connected BOOLEAN NOT NULL DEFAULT FALSE,
    is_ai_proxy  BOOLEAN NOT NULL DEFAULT FALSE,

    UNIQUE(account_id, world_id)
);

CREATE INDEX IF NOT EXISTS idx_player_sessions_world ON player_sessions(world_id);
CREATE INDEX IF NOT EXISTS idx_player_sessions_account ON player_sessions(account_id);

-- ── Cloud Saves ───────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS cloud_saves (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id  UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    world_id    UUID REFERENCES game_worlds(id) ON DELETE SET NULL,
    name        VARCHAR(128) NOT NULL,
    slot        INTEGER NOT NULL DEFAULT 0,
    save_data   BYTEA NOT NULL,                          -- zstd-compressed binary ECS state
    tick        BIGINT NOT NULL,
    config_json JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    size_bytes  BIGINT NOT NULL DEFAULT 0,

    UNIQUE(account_id, slot)
);

CREATE INDEX IF NOT EXISTS idx_cloud_saves_account ON cloud_saves(account_id);

-- ── World Snapshots (periodic server-side backups) ────────────────────────

CREATE TABLE IF NOT EXISTS world_snapshots (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    world_id    UUID NOT NULL REFERENCES game_worlds(id) ON DELETE CASCADE,
    tick        BIGINT NOT NULL,
    state_data  BYTEA NOT NULL,                          -- full serialized world state
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_world_snapshots_world_tick ON world_snapshots(world_id, tick DESC);

-- ── Event Log (for replay and debugging) ──────────────────────────────────

CREATE TABLE IF NOT EXISTS event_log (
    id          BIGSERIAL PRIMARY KEY,
    world_id    UUID NOT NULL REFERENCES game_worlds(id) ON DELETE CASCADE,
    tick        BIGINT NOT NULL,
    event_type  VARCHAR(64) NOT NULL,
    event_data  JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_event_log_world_tick ON event_log(world_id, tick DESC);

-- ── Leaderboard ───────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS leaderboard (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id  UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    world_id    UUID NOT NULL REFERENCES game_worlds(id) ON DELETE CASCADE,
    corp_name   VARCHAR(128) NOT NULL,
    score       BIGINT NOT NULL DEFAULT 0,
    net_worth   BIGINT NOT NULL DEFAULT 0,
    tick        BIGINT NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(account_id, world_id)
);

CREATE INDEX IF NOT EXISTS idx_leaderboard_score ON leaderboard(score DESC);
CREATE INDEX IF NOT EXISTS idx_leaderboard_world ON leaderboard(world_id, score DESC);
