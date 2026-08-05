CREATE TABLE IF NOT EXISTS playback_history (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    played_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    source TEXT NOT NULL DEFAULT 'local',
    scrobbled BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_playback_history_user ON playback_history(user_id);
CREATE INDEX IF NOT EXISTS idx_playback_history_played ON playback_history(played_at);
