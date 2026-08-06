use std::collections::HashMap;

use anyhow::Result;
use sqlx::{Row, SqlitePool};

use crate::db::database::DatabasePool;
use crate::db::library;
use crate::signals::library::TrackInfo;
use crate::signals::tools::{DuplicateGroup, DuplicateTrackEntry, FindDuplicatesResponse};

pub struct FindDuplicatesOptions {
    pub check_fingerprint: bool,
    pub check_mbid: bool,
    pub check_file_size_duration: bool,
    pub check_title_artist: bool,
    pub min_confidence: f64,
    pub source_id: Option<String>,
}

fn source_filter(source_id: &Option<String>) -> &'static str {
    match source_id {
        Some(_) => " AND ls.source_type NOT LIKE 'recommendation:%' AND t.source_id = ?",
        None => " AND ls.source_type NOT LIKE 'recommendation:%'",
    }
}

fn source_filter_pg(source_id: &Option<String>) -> &'static str {
    match source_id {
        Some(_) => " AND ls.source_type NOT LIKE 'recommendation:%' AND t.source_id = $1",
        None => " AND ls.source_type NOT LIKE 'recommendation:%'",
    }
}

async fn fetch_tracks_by_ids_sq(
    pool: &SqlitePool,
    track_ids: &[String],
) -> Result<Vec<DuplicateTrackEntry>> {
    if track_ids.is_empty() {
        return Ok(vec![]);
    }
    let placeholders: Vec<String> = track_ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("?{}", i + 1))
        .collect();
    let sql = format!(
        r#"SELECT t.id, t.title, COALESCE(ar.name, '') AS artist,
                  COALESCE(a.title, '') AS album, t.file_path, t.file_size,
                  t.duration_secs, t.mbid_recording,
                  (SELECT 1 FROM fingerprints f WHERE f.track_id = t.id LIMIT 1) AS has_fp
           FROM tracks t
           JOIN artists ar ON t.artist_id = ar.id
           JOIN albums a ON t.album_id = a.id
           WHERE t.id IN ({})"#,
        placeholders.join(","),
    );
    let mut query = sqlx::query(&sql);
    for id in track_ids {
        query = query.bind(id);
    }
    let rows = query.fetch_all(pool).await?;
    let entries = rows
        .into_iter()
        .map(|row| DuplicateTrackEntry {
            track_id: row.get("id"),
            title: row.get("title"),
            artist: row.get("artist"),
            album: row.get("album"),
            file_path: row.get("file_path"),
            file_size: row.get("file_size"),
            duration_secs: row.get("duration_secs"),
            mbid_recording: row.get("mbid_recording"),
            has_fingerprint: row.get::<Option<i32>, _>("has_fp").unwrap_or(0) != 0,
        })
        .collect();
    Ok(entries)
}

async fn fetch_tracks_by_ids_pg(
    pool: &sqlx::PgPool,
    track_ids: &[String],
) -> Result<Vec<DuplicateTrackEntry>> {
    if track_ids.is_empty() {
        return Ok(vec![]);
    }
    let placeholders: Vec<String> = track_ids
        .iter()
        .enumerate()
        .map(|(i, _)| format!("${}", i + 1))
        .collect();
    let sql = format!(
        r#"SELECT t.id, t.title, COALESCE(ar.name, '') AS artist,
                  COALESCE(a.title, '') AS album, t.file_path, t.file_size,
                  t.duration_secs, t.mbid_recording,
                  (SELECT 1 FROM fingerprints f WHERE f.track_id = t.id LIMIT 1) AS has_fp
           FROM tracks t
           JOIN artists ar ON t.artist_id = ar.id
           JOIN albums a ON t.album_id = a.id
           WHERE t.id IN ({})"#,
        placeholders.join(","),
    );
    let mut query = sqlx::query(&sql);
    for id in track_ids {
        query = query.bind(id);
    }
    let rows = query.fetch_all(pool).await?;
    let entries = rows
        .into_iter()
        .map(|row| DuplicateTrackEntry {
            track_id: row.get("id"),
            title: row.get("title"),
            artist: row.get("artist"),
            album: row.get("album"),
            file_path: row.get("file_path"),
            file_size: row.get("file_size"),
            duration_secs: row.get("duration_secs"),
            mbid_recording: row.get("mbid_recording"),
            has_fingerprint: row.get::<Option<i32>, _>("has_fp").unwrap_or(0) != 0,
        })
        .collect();
    Ok(entries)
}

async fn find_by_file_size_duration_sq(
    pool: &SqlitePool,
    source_id: &Option<String>,
    filter_clause: &str,
) -> Result<Vec<DuplicateGroup>> {
    let sql = format!(
        r#"SELECT t.file_size, t.duration_secs, GROUP_CONCAT(t.id) AS track_ids
           FROM tracks t
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE t.file_size IS NOT NULL{}
           GROUP BY t.file_size, t.duration_secs
           HAVING COUNT(*) > 1"#,
        filter_clause,
    );
    let mut query = sqlx::query(&sql);
    if let Some(sid) = source_id {
        query = query.bind(sid);
    }
    let rows = query.fetch_all(pool).await?;
    let mut groups = Vec::new();
    for row in rows {
        let file_size: i64 = row.get("file_size");
        let duration_secs: f64 = row.get("duration_secs");
        let ids_str: String = row.get("track_ids");
        let track_ids: Vec<String> = ids_str.split(',').map(String::from).collect();
        let tracks = fetch_tracks_by_ids_sq(pool, &track_ids).await?;
        if tracks.len() > 1 {
            groups.push(DuplicateGroup {
                method: "file_size_duration".into(),
                key: format!("{}|{}", file_size, duration_secs as i64),
                tracks,
                confidence: 0.85,
            });
        }
    }
    Ok(groups)
}

async fn find_by_file_size_duration_pg(
    pool: &sqlx::PgPool,
    source_id: &Option<String>,
    filter_clause: &str,
) -> Result<Vec<DuplicateGroup>> {
    let sql = format!(
        r#"SELECT t.file_size, t.duration_secs,
                  STRING_AGG(t.id, ',') AS track_ids
           FROM tracks t
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE t.file_size IS NOT NULL{}
           GROUP BY t.file_size, t.duration_secs
           HAVING COUNT(*) > 1"#,
        filter_clause,
    );
    let mut query = sqlx::query(&sql);
    if let Some(sid) = source_id {
        query = query.bind(sid);
    }
    let rows = query.fetch_all(pool).await?;
    let mut groups = Vec::new();
    for row in rows {
        let file_size: i64 = row.get("file_size");
        let duration_secs: f64 = row.get("duration_secs");
        let ids_str: String = row.get("track_ids");
        let track_ids: Vec<String> = ids_str.split(',').map(String::from).collect();
        let tracks = fetch_tracks_by_ids_pg(pool, &track_ids).await?;
        if tracks.len() > 1 {
            groups.push(DuplicateGroup {
                method: "file_size_duration".into(),
                key: format!("{}|{}", file_size, duration_secs as i64),
                tracks,
                confidence: 0.85,
            });
        }
    }
    Ok(groups)
}

async fn find_by_mbid_sq(
    pool: &SqlitePool,
    source_id: &Option<String>,
    filter_clause: &str,
) -> Result<Vec<DuplicateGroup>> {
    let sql = format!(
        r#"SELECT t.mbid_recording, GROUP_CONCAT(t.id) AS track_ids
           FROM tracks t
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE t.mbid_recording IS NOT NULL{}
           GROUP BY t.mbid_recording
           HAVING COUNT(*) > 1"#,
        filter_clause,
    );
    let mut query = sqlx::query(&sql);
    if let Some(sid) = source_id {
        query = query.bind(sid);
    }
    let rows = query.fetch_all(pool).await?;
    let mut groups = Vec::new();
    for row in rows {
        let mbid: String = row.get("mbid_recording");
        let ids_str: String = row.get("track_ids");
        let track_ids: Vec<String> = ids_str.split(',').map(String::from).collect();
        let tracks = fetch_tracks_by_ids_sq(pool, &track_ids).await?;
        if tracks.len() > 1 {
            groups.push(DuplicateGroup {
                method: "mbid".into(),
                key: mbid,
                tracks,
                confidence: 0.95,
            });
        }
    }
    Ok(groups)
}

async fn find_by_mbid_pg(
    pool: &sqlx::PgPool,
    source_id: &Option<String>,
    filter_clause: &str,
) -> Result<Vec<DuplicateGroup>> {
    let sql = format!(
        r#"SELECT t.mbid_recording,
                  STRING_AGG(t.id, ',') AS track_ids
           FROM tracks t
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE t.mbid_recording IS NOT NULL{}
           GROUP BY t.mbid_recording
           HAVING COUNT(*) > 1"#,
        filter_clause,
    );
    let mut query = sqlx::query(&sql);
    if let Some(sid) = source_id {
        query = query.bind(sid);
    }
    let rows = query.fetch_all(pool).await?;
    let mut groups = Vec::new();
    for row in rows {
        let mbid: String = row.get("mbid_recording");
        let ids_str: String = row.get("track_ids");
        let track_ids: Vec<String> = ids_str.split(',').map(String::from).collect();
        let tracks = fetch_tracks_by_ids_pg(pool, &track_ids).await?;
        if tracks.len() > 1 {
            groups.push(DuplicateGroup {
                method: "mbid".into(),
                key: mbid,
                tracks,
                confidence: 0.95,
            });
        }
    }
    Ok(groups)
}

async fn find_by_fingerprint_sq(
    pool: &SqlitePool,
    source_id: &Option<String>,
    filter_clause: &str,
) -> Result<Vec<DuplicateGroup>> {
    let sql = format!(
        r#"SELECT f.fingerprint, GROUP_CONCAT(f.track_id) AS track_ids
           FROM fingerprints f
           JOIN tracks t ON f.track_id = t.id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE 1=1{}
           GROUP BY f.fingerprint
           HAVING COUNT(*) > 1"#,
        filter_clause,
    );
    let mut query = sqlx::query(&sql);
    if let Some(sid) = source_id {
        query = query.bind(sid);
    }
    let rows = query.fetch_all(pool).await?;
    let mut groups = Vec::new();
    for row in rows {
        let _fingerprint: String = row.get("fingerprint");
        let ids_str: String = row.get("track_ids");
        let track_ids: Vec<String> = ids_str.split(',').map(String::from).collect();
        let tracks = fetch_tracks_by_ids_sq(pool, &track_ids).await?;
        if tracks.len() > 1 {
            groups.push(DuplicateGroup {
                method: "fingerprint".into(),
                key: _fingerprint.clone(),
                tracks,
                confidence: 0.99,
            });
        }
    }
    Ok(groups)
}

async fn find_by_fingerprint_pg(
    pool: &sqlx::PgPool,
    source_id: &Option<String>,
    filter_clause: &str,
) -> Result<Vec<DuplicateGroup>> {
    let sql = format!(
        r#"SELECT f.fingerprint,
                  STRING_AGG(f.track_id, ',') AS track_ids
           FROM fingerprints f
           JOIN tracks t ON f.track_id = t.id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE 1=1{}
           GROUP BY f.fingerprint
           HAVING COUNT(*) > 1"#,
        filter_clause,
    );
    let mut query = sqlx::query(&sql);
    if let Some(sid) = source_id {
        query = query.bind(sid);
    }
    let rows = query.fetch_all(pool).await?;
    let mut groups = Vec::new();
    for row in rows {
        let _fingerprint: String = row.get("fingerprint");
        let ids_str: String = row.get("track_ids");
        let track_ids: Vec<String> = ids_str.split(',').map(String::from).collect();
        let tracks = fetch_tracks_by_ids_pg(pool, &track_ids).await?;
        if tracks.len() > 1 {
            groups.push(DuplicateGroup {
                method: "fingerprint".into(),
                key: _fingerprint.clone(),
                tracks,
                confidence: 0.99,
            });
        }
    }
    Ok(groups)
}

async fn find_by_title_artist_sq(
    pool: &SqlitePool,
    source_id: &Option<String>,
    filter_clause: &str,
) -> Result<Vec<DuplicateGroup>> {
    let sql = format!(
        r#"SELECT LOWER(t.title) AS norm_title, LOWER(COALESCE(ar.name, '')) AS norm_artist,
                  GROUP_CONCAT(t.id) AS track_ids
           FROM tracks t
           JOIN artists ar ON t.artist_id = ar.id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE 1=1 {}
           GROUP BY norm_title, norm_artist
           HAVING COUNT(*) > 1"#,
        filter_clause,
    );
    let mut query = sqlx::query(&sql);
    if let Some(sid) = source_id {
        query = query.bind(sid);
    }
    let rows = query.fetch_all(pool).await?;
    let mut groups = Vec::new();
    for row in rows {
        let norm_title: String = row.get("norm_title");
        let norm_artist: String = row.get("norm_artist");
        let ids_str: String = row.get("track_ids");
        let track_ids: Vec<String> = ids_str.split(',').map(String::from).collect();
        let tracks = fetch_tracks_by_ids_sq(pool, &track_ids).await?;
        if tracks.len() > 1 {
            groups.push(DuplicateGroup {
                method: "title_artist".into(),
                key: format!("{}|{}", norm_title, norm_artist),
                tracks,
                confidence: 0.7,
            });
        }
    }
    Ok(groups)
}

async fn find_by_title_artist_pg(
    pool: &sqlx::PgPool,
    source_id: &Option<String>,
    filter_clause: &str,
) -> Result<Vec<DuplicateGroup>> {
    let sql = format!(
        r#"SELECT LOWER(t.title) AS norm_title, LOWER(COALESCE(ar.name, '')) AS norm_artist,
                  STRING_AGG(t.id, ',') AS track_ids
           FROM tracks t
           JOIN artists ar ON t.artist_id = ar.id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE 1=1 {}
           GROUP BY norm_title, norm_artist
           HAVING COUNT(*) > 1"#,
        filter_clause,
    );
    let mut query = sqlx::query(&sql);
    if let Some(sid) = source_id {
        query = query.bind(sid);
    }
    let rows = query.fetch_all(pool).await?;
    let mut groups = Vec::new();
    for row in rows {
        let norm_title: String = row.get("norm_title");
        let norm_artist: String = row.get("norm_artist");
        let ids_str: String = row.get("track_ids");
        let track_ids: Vec<String> = ids_str.split(',').map(String::from).collect();
        let tracks = fetch_tracks_by_ids_pg(pool, &track_ids).await?;
        if tracks.len() > 1 {
            groups.push(DuplicateGroup {
                method: "title_artist".into(),
                key: format!("{}|{}", norm_title, norm_artist),
                tracks,
                confidence: 0.7,
            });
        }
    }
    Ok(groups)
}

pub async fn find_duplicates(
    pool: &DatabasePool,
    options: &FindDuplicatesOptions,
) -> Result<FindDuplicatesResponse> {
    let mut all_groups: Vec<DuplicateGroup> = Vec::new();
    let mut seen_keys = HashMap::new();

    if options.check_file_size_duration {
        let groups = match pool {
            DatabasePool::Sqlite(p) => {
                let filter = source_filter(&options.source_id);
                find_by_file_size_duration_sq(p, &options.source_id, filter).await?
            }
            DatabasePool::Postgres(p) => {
                let filter = source_filter_pg(&options.source_id);
                find_by_file_size_duration_pg(p, &options.source_id, filter).await?
            }
        };
        for g in groups {
            seen_keys.insert(g.key.clone(), true);
            all_groups.push(g);
        }
    }

    if options.check_mbid {
        let groups = match pool {
            DatabasePool::Sqlite(p) => {
                let filter = source_filter(&options.source_id);
                find_by_mbid_sq(p, &options.source_id, filter).await?
            }
            DatabasePool::Postgres(p) => {
                let filter = source_filter_pg(&options.source_id);
                find_by_mbid_pg(p, &options.source_id, filter).await?
            }
        };
        for g in groups {
            if !seen_keys.contains_key(&g.key) {
                seen_keys.insert(g.key.clone(), true);
                all_groups.push(g);
            }
        }
    }

    if options.check_fingerprint {
        let groups = match pool {
            DatabasePool::Sqlite(p) => {
                let filter = source_filter(&options.source_id);
                find_by_fingerprint_sq(p, &options.source_id, filter).await?
            }
            DatabasePool::Postgres(p) => {
                let filter = source_filter_pg(&options.source_id);
                find_by_fingerprint_pg(p, &options.source_id, filter).await?
            }
        };
        for g in groups {
            if !seen_keys.contains_key(&g.key) {
                seen_keys.insert(g.key.clone(), true);
                all_groups.push(g);
            }
        }
    }

    if options.check_title_artist {
        let groups = match pool {
            DatabasePool::Sqlite(p) => {
                let filter = source_filter(&options.source_id);
                find_by_title_artist_sq(p, &options.source_id, filter).await?
            }
            DatabasePool::Postgres(p) => {
                let filter = source_filter_pg(&options.source_id);
                find_by_title_artist_pg(p, &options.source_id, filter).await?
            }
        };
        for g in groups {
            if !seen_keys.contains_key(&g.key) {
                seen_keys.insert(g.key.clone(), true);
                all_groups.push(g);
            }
        }
    }

    let min_conf = options.min_confidence;
    let groups: Vec<DuplicateGroup> = all_groups
        .into_iter()
        .filter(|g| g.confidence >= min_conf)
        .collect();

    let total_duplicates: u32 = groups.iter().map(|g| g.tracks.len() as u32).sum();
    let total_groups = groups.len() as u32;

    Ok(FindDuplicatesResponse {
        id: String::new(),
        groups,
        total_duplicates,
        total_groups,
        error: None,
    })
}

async fn find_track_by_recording_sq(
    pool: &SqlitePool,
    mbid_recording: Option<&str>,
    title: &str,
    artist_name: &str,
) -> Result<Option<String>> {
    let result: Option<String> = sqlx::query_scalar(
        r#"SELECT t.id FROM tracks t
         JOIN artists ar ON t.artist_id = ar.id
         WHERE (t.mbid_recording = ?) OR (t.title = ? AND ar.name = ?)
         LIMIT 1"#,
    )
    .bind(mbid_recording)
    .bind(title)
    .bind(artist_name)
    .fetch_optional(pool)
    .await?;
    Ok(result)
}

async fn find_track_by_recording_pg(
    pool: &sqlx::PgPool,
    mbid_recording: Option<&str>,
    title: &str,
    artist_name: &str,
) -> Result<Option<String>> {
    let result: Option<String> = sqlx::query_scalar(
        r#"SELECT t.id FROM tracks t
         JOIN artists ar ON t.artist_id = ar.id
         WHERE (t.mbid_recording = $1) OR (t.title = $2 AND ar.name = $3)
         LIMIT 1"#,
    )
    .bind(mbid_recording)
    .bind(title)
    .bind(artist_name)
    .fetch_optional(pool)
    .await?;
    Ok(result)
}

pub async fn find_track_by_recording(
    pool: &DatabasePool,
    mbid_recording: Option<&str>,
    title: &str,
    artist_name: &str,
) -> Result<Option<String>> {
    match pool {
        DatabasePool::Sqlite(p) => {
            find_track_by_recording_sq(p, mbid_recording, title, artist_name).await
        }
        DatabasePool::Postgres(p) => {
            find_track_by_recording_pg(p, mbid_recording, title, artist_name).await
        }
    }
}

async fn is_recording_owned_sq(
    pool: &SqlitePool,
    mbid_recording: Option<&str>,
    title: &str,
    artist_name: &str,
) -> Result<bool> {
    let result: Option<String> = sqlx::query_scalar(
        r#"SELECT t.id FROM tracks t
         JOIN artists ar ON t.artist_id = ar.id
         JOIN library_sources ls ON t.source_id = ls.id
         WHERE ((t.mbid_recording = ?) OR (t.title = ? AND ar.name = ?))
           AND ls.source_type NOT LIKE 'recommendation:%'
         LIMIT 1"#,
    )
    .bind(mbid_recording)
    .bind(title)
    .bind(artist_name)
    .fetch_optional(pool)
    .await?;
    Ok(result.is_some())
}

async fn is_recording_owned_pg(
    pool: &sqlx::PgPool,
    mbid_recording: Option<&str>,
    title: &str,
    artist_name: &str,
) -> Result<bool> {
    let result: Option<String> = sqlx::query_scalar(
        r#"SELECT t.id FROM tracks t
         JOIN artists ar ON t.artist_id = ar.id
         JOIN library_sources ls ON t.source_id = ls.id
         WHERE ((t.mbid_recording = $1) OR (t.title = $2 AND ar.name = $3))
           AND ls.source_type NOT LIKE 'recommendation:%'
         LIMIT 1"#,
    )
    .bind(mbid_recording)
    .bind(title)
    .bind(artist_name)
    .fetch_optional(pool)
    .await?;
    Ok(result.is_some())
}

pub async fn is_recording_owned(
    pool: &DatabasePool,
    mbid_recording: Option<&str>,
    title: &str,
    artist_name: &str,
) -> Result<bool> {
    match pool {
        DatabasePool::Sqlite(p) => {
            is_recording_owned_sq(p, mbid_recording, title, artist_name).await
        }
        DatabasePool::Postgres(p) => {
            is_recording_owned_pg(p, mbid_recording, title, artist_name).await
        }
    }
}

pub async fn find_matching_track(
    pool: &DatabasePool,
    title: &str,
    artist: &str,
    album: Option<&str>,
    recording_mbid: Option<&str>,
    min_score: f64,
) -> anyhow::Result<Option<(TrackInfo, f64)>> {
    let all = library::list_tracks(pool, None).await?;

    let best = all
        .into_iter()
        .filter(|t| !t.file_path.starts_with("recommendation://"))
        .filter_map(|t| {
            let score = track_similarity(&t, title, artist, album, recording_mbid);
            if score >= min_score {
                Some((t, score))
            } else {
                None
            }
        })
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    Ok(best)
}

fn track_similarity(
    track: &TrackInfo,
    query_title: &str,
    query_artist: &str,
    query_album: Option<&str>,
    query_mbid: Option<&str>,
) -> f64 {
    if let Some(mbid) = query_mbid {
        if track.mbid_recording.as_deref() == Some(mbid) {
            return 1.0;
        }
    }

    let title_sim = str_similarity(&track.title, query_title);
    let artist_sim = str_similarity(&track.artists_string, query_artist);
    let album_sim = query_album
        .map(|a| str_similarity(&track.album_title, a))
        .unwrap_or(0.0);

    title_sim * 0.5 + artist_sim * 0.3 + album_sim * 0.2
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn str_similarity(a: &str, b: &str) -> f64 {
    let a = normalize(a);
    let b = normalize(b);

    if a == b {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    if a.contains(&b) || b.contains(&a) {
        return 0.9;
    }

    bigram_similarity(&a, &b)
}

fn bigram_similarity(a: &str, b: &str) -> f64 {
    fn bigrams(s: &str) -> Vec<[char; 2]> {
        let chars: Vec<char> = s.chars().collect();
        chars.windows(2).map(|w| [w[0], w[1]]).collect()
    }

    let ab = bigrams(a);
    let bb = bigrams(b);

    if ab.is_empty() && bb.is_empty() {
        return 1.0;
    }
    if ab.is_empty() || bb.is_empty() {
        return 0.0;
    }

    let intersection = ab.iter().filter(|bg| bb.contains(bg)).count();
    2.0 * intersection as f64 / (ab.len() + bb.len()) as f64
}
