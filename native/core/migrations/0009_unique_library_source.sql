-- Deduplicate library_sources on (source_type, url, owner_id), keeping the
-- most recently inserted row per group.
DELETE FROM library_sources
WHERE rowid NOT IN (
    SELECT MAX(rowid) FROM library_sources
    GROUP BY source_type, url, owner_id
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_library_sources_unique
    ON library_sources(source_type, url, owner_id);
