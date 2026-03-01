-- ── R2 Object Storage Support ──────────────────────────────────────────────
-- Add R2 key columns to snapshots and cloud saves.
-- When R2 is configured, binary blobs are stored in R2 and these columns
-- contain the object key. The BYTEA columns become NULL for R2-backed rows.

-- World snapshots: add r2_key and size_bytes, make state_data nullable
ALTER TABLE world_snapshots ADD COLUMN IF NOT EXISTS r2_key TEXT;
ALTER TABLE world_snapshots ADD COLUMN IF NOT EXISTS size_bytes BIGINT;
ALTER TABLE world_snapshots ALTER COLUMN state_data DROP NOT NULL;

-- Cloud saves: add r2_key, make save_data nullable
ALTER TABLE cloud_saves ADD COLUMN IF NOT EXISTS r2_key TEXT;
ALTER TABLE cloud_saves ALTER COLUMN save_data DROP NOT NULL;
