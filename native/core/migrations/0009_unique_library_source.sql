-- Deduplicate recommendation library_sources on (source_type, owner_id),
-- keeping the most recently inserted row per group. Non-recommendation
-- sources are not deduplicated (they may have multiple URLs per type).
DELETE FROM library_sources
WHERE source_type LIKE 'recommendation:%'
  AND rowid NOT IN (
    SELECT MAX(rowid) FROM library_sources
    WHERE source_type LIKE 'recommendation:%'
    GROUP BY source_type, owner_id
  );

CREATE UNIQUE INDEX IF NOT EXISTS idx_library_sources_unique
    ON library_sources(source_type, owner_id)
    WHERE source_type LIKE 'recommendation:%';

-- Speeds up bulk operations that filter tracks by library source, e.g.
-- deleting a source (list-by-source scan).
CREATE INDEX idx_tracks_source ON tracks(source_id);
