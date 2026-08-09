use anyhow::Result;
use sqlx::{Row, SqlitePool};

use crate::db::database::DatabasePool;
use crate::signals::tools::{MissingMetadataCheck, MissingMetadataEntry};

fn is_missing_value(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() {
        return true;
    }
    matches!(
        t.to_lowercase().as_str(),
        "unknown"
            | "unknown artist"
            | "unknown album"
            | "unknown title"
            | "unknown genre"
            | "unknown year"
            | "unknown track"
            | "various artists"
            | "va"
            | "n/a"
            | "na"
            | "none"
            | "null"
            | "unknowns"
    )
}

pub async fn find_missing_metadata(
    pool: &DatabasePool,
    check: &MissingMetadataCheck,
) -> Result<Vec<MissingMetadataEntry>> {
    match pool {
        DatabasePool::Sqlite(p) => find_missing_metadata_sq(p, check).await,
        DatabasePool::Postgres(p) => find_missing_metadata_pg(p, check).await,
    }
}

async fn find_missing_metadata_sq(
    pool: &SqlitePool,
    check: &MissingMetadataCheck,
) -> Result<Vec<MissingMetadataEntry>> {
    // Build a query that returns tracks with their artist/album info
    // We'll filter missing fields in Rust for flexibility
    // Filter out recommendation sources
    let rows = sqlx::query(
        r#"SELECT t.id, t.file_path, t.title, COALESCE(ar.name, '') AS artist_name,
                  COALESCE(a.title, '') AS album_title, t.track_num, a.date,
                  t.cover,
                  (SELECT GROUP_CONCAT(g.name, '||') FROM track_genres tg
                   JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
           FROM tracks t
           JOIN albums a ON t.album_id = a.id
           JOIN artists ar ON t.artist_id = ar.id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'
           ORDER BY t.file_path"#,
    )
    .fetch_all(pool)
    .await?;

    let mut results = Vec::new();

    for row in rows {
        let mut missing = Vec::new();

        let track_id: String = row.get("id");
        let file_path: String = row.get("file_path");
        let title: String = row.get("title");
        let artist: String = row.get("artist_name");
        let album: String = row.get("album_title");
        let track_num: Option<i32> = row.get("track_num");
        let date: Option<String> = row.get("date");
        let cover: Option<Vec<u8>> = row.get("cover");
        let genres: Option<String> = row.get("genres");

        if check.check_title && is_missing_value(&title) {
            missing.push("title".to_string());
        }
        if check.check_artist && is_missing_value(&artist) {
            missing.push("artist".to_string());
        }
        if check.check_album && is_missing_value(&album) {
            missing.push("album".to_string());
        }
        if check.check_genre {
            let has_genre = genres.as_deref().map(|g| !g.is_empty()).unwrap_or(false);
            if !has_genre {
                missing.push("genre".to_string());
            }
        }
        if check.check_year {
            let has_year = date
                .as_deref()
                .and_then(|d| d.split('-').next())
                .map(|y| !y.is_empty())
                .unwrap_or(false);
            if !has_year {
                missing.push("year".to_string());
            }
        }
        if check.check_track_number && track_num.is_none() {
            missing.push("track_number".to_string());
        }
        if check.check_cover && cover.is_none() {
            missing.push("cover".to_string());
        }

        if !missing.is_empty() {
            results.push(MissingMetadataEntry {
                track_id,
                file_path,
                title,
                artist,
                album,
                missing_fields: missing,
            });
        }
    }

    Ok(results)
}

async fn find_missing_metadata_pg(
    pool: &sqlx::PgPool,
    check: &MissingMetadataCheck,
) -> Result<Vec<MissingMetadataEntry>> {
    let query_str = r#"SELECT t.id, t.file_path, t.title, COALESCE(ar.name, '') AS artist_name,
                  COALESCE(a.title, '') AS album_title, t.track_num, a.date,
                  t.cover,
                  (SELECT STRING_AGG(g.name, '||') FROM track_genres tg
                   JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
           FROM tracks t
           JOIN albums a ON t.album_id = a.id
           JOIN artists ar ON t.artist_id = ar.id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'
           ORDER BY t.file_path"#;

    let rows = sqlx::query(&query_str).fetch_all(pool).await?;

    let mut results = Vec::new();

    for row in rows {
        let mut missing = Vec::new();

        let track_id: String = row.get("id");
        let file_path: String = row.get("file_path");
        let title: String = row.get("title");
        let artist: String = row.get("artist_name");
        let album: String = row.get("album_title");
        let track_num: Option<i32> = row.get("track_num");
        let date: Option<String> = row.get("date");
        let cover: Option<Vec<u8>> = row.get("cover");
        let genres: Option<String> = row.get("genres");

        if check.check_title && is_missing_value(&title) {
            missing.push("title".to_string());
        }
        if check.check_artist && is_missing_value(&artist) {
            missing.push("artist".to_string());
        }
        if check.check_album && is_missing_value(&album) {
            missing.push("album".to_string());
        }
        if check.check_genre {
            let has_genre = genres.as_deref().map(|g| !g.is_empty()).unwrap_or(false);
            if !has_genre {
                missing.push("genre".to_string());
            }
        }
        if check.check_year {
            let has_year = date
                .as_deref()
                .and_then(|d| d.split('-').next())
                .map(|y| !y.is_empty())
                .unwrap_or(false);
            if !has_year {
                missing.push("year".to_string());
            }
        }
        if check.check_track_number && track_num.is_none() {
            missing.push("track_number".to_string());
        }
        if check.check_cover && cover.is_none() {
            missing.push("cover".to_string());
        }

        if !missing.is_empty() {
            results.push(MissingMetadataEntry {
                track_id,
                file_path,
                title,
                artist,
                album,
                missing_fields: missing,
            });
        }
    }

    Ok(results)
}
