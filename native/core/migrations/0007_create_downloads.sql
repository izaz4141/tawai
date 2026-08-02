CREATE TABLE IF NOT EXISTS downloads (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source TEXT NOT NULL DEFAULT 'manual',
    source_id TEXT NOT NULL DEFAULT '',
    url TEXT NOT NULL DEFAULT '',
    dest_path TEXT NOT NULL DEFAULT '',
    filename TEXT NOT NULL DEFAULT '',
    total_size INTEGER NOT NULL DEFAULT 0,
    downloaded INTEGER NOT NULL DEFAULT 0,
    state TEXT NOT NULL DEFAULT 'queued',
    error TEXT NOT NULL DEFAULT '',
    added_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_downloads_user ON downloads(user_id);
