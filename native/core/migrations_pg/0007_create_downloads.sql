CREATE TABLE IF NOT EXISTS downloads (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source TEXT NOT NULL DEFAULT 'manual',
    source_id TEXT NOT NULL DEFAULT '',
    url TEXT NOT NULL DEFAULT '',
    dest_path TEXT NOT NULL DEFAULT '',
    filename TEXT NOT NULL DEFAULT '',
    total_size BIGINT NOT NULL DEFAULT 0,
    downloaded BIGINT NOT NULL DEFAULT 0,
    state TEXT NOT NULL DEFAULT 'queued',
    error TEXT NOT NULL DEFAULT '',
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_downloads_user ON downloads(user_id);
