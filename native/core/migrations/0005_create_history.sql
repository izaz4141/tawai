CREATE TABLE IF NOT EXISTS playback_history (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    played_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    source TEXT NOT NULL DEFAULT 'local',
    scrobbled INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_playback_history_user ON playback_history(user_id);
CREATE INDEX IF NOT EXISTS idx_playback_history_played ON playback_history(played_at);
