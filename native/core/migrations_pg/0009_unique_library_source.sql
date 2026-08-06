-- Deduplicate library_sources on (source_type, url, owner_id), keeping the
-- newest row (ties broken by physical row id).
DELETE FROM library_sources a
USING library_sources b
WHERE a.source_type = b.source_type
  AND a.url = b.url
  AND a.owner_id = b.owner_id
  AND (a.updated_at < b.updated_at
       OR (a.updated_at = b.updated_at AND a.ctid < b.ctid));

CREATE UNIQUE INDEX IF NOT EXISTS idx_library_sources_unique
    ON library_sources(source_type, url, owner_id);
