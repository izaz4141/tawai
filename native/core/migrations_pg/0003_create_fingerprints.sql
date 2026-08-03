CREATE TABLE IF NOT EXISTS fingerprints (
    id TEXT PRIMARY KEY,
    track_id TEXT NOT NULL UNIQUE REFERENCES tracks(id) ON DELETE CASCADE,
    fingerprint TEXT NOT NULL,
    acoust_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_fingerprints_hash ON fingerprints(fingerprint);
