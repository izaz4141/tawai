CREATE TABLE IF NOT EXISTS sync_log (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    table_name TEXT NOT NULL,
    record_id TEXT NOT NULL,
    action TEXT NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_sync_log_user ON sync_log(user_id);
CREATE INDEX IF NOT EXISTS idx_sync_log_timestamp ON sync_log(timestamp);
