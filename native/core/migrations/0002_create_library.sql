CREATE TABLE IF NOT EXISTS artists (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    sort_name TEXT NOT NULL DEFAULT '',
    mbid TEXT DEFAULT NULL,
    bio TEXT NOT NULL DEFAULT '',
    thumbnail_url TEXT NOT NULL DEFAULT '',
    added_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS albums (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    artist_id TEXT NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    release_mbid TEXT DEFAULT NULL,
    release_group_mbid TEXT DEFAULT NULL,
    release_artist_mbid TEXT DEFAULT NULL,
    release_country TEXT DEFAULT NULL,
    release_type TEXT DEFAULT NULL,
    record_label TEXT DEFAULT NULL,
    media TEXT DEFAULT NULL,
    original_date TEXT DEFAULT NULL,
    original_year TEXT DEFAULT NULL,
    total_tracks INTEGER NOT NULL DEFAULT 0,
    disambiguation TEXT DEFAULT NULL,
    total_discs INTEGER NOT NULL DEFAULT 0,
    date TEXT DEFAULT NULL,
    cover BLOB DEFAULT NULL,
    added_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS library_sources (
    id TEXT PRIMARY KEY,
    source_type TEXT NOT NULL DEFAULT 'local',
    urls TEXT NOT NULL DEFAULT '[]',
    name TEXT NOT NULL DEFAULT '',
    last_sync_at TEXT,
    owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    access_rule TEXT NOT NULL DEFAULT 'all',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_library_sources_owner ON library_sources(owner_id);

CREATE TABLE IF NOT EXISTS tracks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    album_id TEXT REFERENCES albums(id) ON DELETE SET NULL,
    artist_id TEXT REFERENCES artists(id) ON DELETE SET NULL,
    disc_num INTEGER NOT NULL DEFAULT 1,
    track_num INTEGER NOT NULL DEFAULT 0,
    duration_secs REAL NOT NULL DEFAULT 0,
    file_path TEXT NOT NULL,
    file_hash TEXT DEFAULT NULL,
    source_id TEXT NOT NULL REFERENCES library_sources(id) ON DELETE CASCADE,
    mbid_recording TEXT DEFAULT NULL,
    mbid_track TEXT DEFAULT NULL,
    isrc TEXT DEFAULT NULL,
    comment TEXT DEFAULT NULL,
    copyright TEXT DEFAULT NULL,
    encoder TEXT DEFAULT NULL,
    script TEXT DEFAULT NULL,
    bitrate INTEGER DEFAULT NULL,
    sample_rate INTEGER DEFAULT NULL,
    file_size INTEGER DEFAULT NULL,
    cover BLOB DEFAULT NULL,
    lyrics TEXT DEFAULT NULL,
    track_gain REAL DEFAULT NULL,
    track_peak REAL DEFAULT NULL,
    added_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS genres (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS track_genres (
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    genre_id TEXT NOT NULL REFERENCES genres(id) ON DELETE CASCADE,
    PRIMARY KEY (track_id, genre_id)
);

CREATE TABLE IF NOT EXISTS track_artists (
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    artist_id TEXT NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    PRIMARY KEY (track_id, artist_id)
);

CREATE TABLE IF NOT EXISTS album_artists (
    album_id TEXT NOT NULL REFERENCES albums(id) ON DELETE CASCADE,
    artist_id TEXT NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    PRIMARY KEY (album_id, artist_id)
);
