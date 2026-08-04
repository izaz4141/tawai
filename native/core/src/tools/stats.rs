use std::path::Path;

use anyhow::Result;
use sqlx::{Row, SqlitePool};

use crate::audio::tags::AudioTag;
use crate::db::database::DatabasePool;
use crate::signals::tools::{DecadeEntry, FormatEntry, LibraryStats};
use crate::tools::rename::format_naming_pattern;

pub async fn get_library_stats(
    pool: &DatabasePool,
    naming_pattern: Option<&str>,
) -> Result<LibraryStats> {
    match pool {
        DatabasePool::Sqlite(p) => get_library_stats_sq(p, naming_pattern).await,
        DatabasePool::Postgres(p) => get_library_stats_pg(p, naming_pattern).await,
    }
}

async fn get_library_stats_sq(
    pool: &SqlitePool,
    naming_pattern: Option<&str>,
) -> Result<LibraryStats> {
    let total_tracks: i64 = sqlx::query(
        r#"SELECT COUNT(*) FROM tracks t
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let total_albums: i64 = sqlx::query(
        r#"SELECT COUNT(DISTINCT a.id) FROM albums a
           JOIN tracks t ON a.id = t.album_id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let total_artists: i64 = sqlx::query(
        r#"SELECT COUNT(DISTINCT ar.id) FROM artists ar
           JOIN tracks t ON ar.id = t.artist_id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let total_duration: f64 = sqlx::query(
        r#"SELECT COALESCE(SUM(t.duration_secs), 0.0) FROM tracks t
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let avg_bitrate: Option<f64> = sqlx::query(
        r#"SELECT AVG(t.bitrate * 1.0) FROM tracks t
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE t.bitrate > 0 AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_optional(pool)
    .await?
    .map(|r| r.get(0));

    let genre_row = sqlx::query(
        r#"SELECT g.name, COUNT(*) as cnt
           FROM track_genres tg
           JOIN genres g ON tg.genre_id = g.id
           JOIN tracks t ON tg.track_id = t.id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'
           GROUP BY g.id
           ORDER BY cnt DESC
           LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await?;

    let (most_common_genre, genre_count) = if let Some(row) = genre_row {
        (Some(row.get::<String, _>("name")), row.get::<i64, _>("cnt"))
    } else {
        (None, 0)
    };

    let format_rows = sqlx::query(
        r#"SELECT
               LOWER(
                 CASE
                   WHEN INSTR(t.file_path, '.') > 0
                     THEN SUBSTR(t.file_path, INSTR(t.file_path, '.') + 1)
                   ELSE 'unknown'
                 END
               ) AS ext,
               COUNT(*) as cnt
           FROM tracks t
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'
           GROUP BY ext
           ORDER BY cnt DESC"#,
    )
    .fetch_all(pool)
    .await?;

    let format_breakdown: Vec<FormatEntry> = format_rows
        .into_iter()
        .map(|row| FormatEntry {
            format: row.get("ext"),
            count: row.get("cnt"),
        })
        .collect();

    let decade_rows = sqlx::query(
        r#"SELECT
               CASE
                 WHEN a.date IS NOT NULL AND LENGTH(a.date) >= 4
                   THEN SUBSTR(CAST(SUBSTR(a.date, 1, 4) AS INTEGER) / 10 * 10, 1, 4) || 's'
                 ELSE 'Unknown'
               END AS decade,
               COUNT(*) as cnt
           FROM tracks t
           JOIN albums a ON t.album_id = a.id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'
           GROUP BY decade
           ORDER BY decade"#,
    )
    .fetch_all(pool)
    .await?;

    let decade_distribution: Vec<DecadeEntry> = decade_rows
        .into_iter()
        .map(|row| DecadeEntry {
            decade: row.get("decade"),
            count: row.get("cnt"),
        })
        .collect();

    let largest_album = sqlx::query(
        r#"SELECT a.title, COUNT(*) as cnt
           FROM tracks t
           JOIN albums a ON t.album_id = a.id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'
           GROUP BY t.album_id
           ORDER BY cnt DESC
           LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await?;

    let (largest_album_title, largest_album_tracks) = if let Some(row) = largest_album {
        (
            Some(row.get::<String, _>("title")),
            row.get::<i64, _>("cnt"),
        )
    } else {
        (None, 0)
    };

    let most_prolific = sqlx::query(
        r#"SELECT ar.name, COUNT(*) as cnt
           FROM tracks t
           JOIN artists ar ON t.artist_id = ar.id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'
           GROUP BY t.artist_id
           ORDER BY cnt DESC
           LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await?;

    let (most_prolific_artist, most_prolific_artist_tracks) = if let Some(row) = most_prolific {
        (Some(row.get::<String, _>("name")), row.get::<i64, _>("cnt"))
    } else {
        (None, 0)
    };

    let naming_conformity_pct = match naming_pattern.filter(|p| !p.is_empty()) {
        Some(pattern) if total_tracks > 0 => {
            let rows = sqlx::query(
                r#"SELECT t.file_path, t.title, COALESCE(ar.name, '') AS artist_name,
                          COALESCE(a.title, '') AS album_title,
                          t.track_num, t.disc_num, a.date, a.disambiguation,
                          COALESCE(ar.name, '') AS album_artist,
                          a.total_discs
                   FROM tracks t
                   JOIN albums a ON t.album_id = a.id
                   JOIN artists ar ON t.artist_id = ar.id"#,
            )
            .fetch_all(pool)
            .await?;

            let conforming = rows
                .iter()
                .filter(|row| {
                    let file_path: String = row.get("file_path");
                    let path = Path::new(&file_path);
                    let parent = path.parent().unwrap_or(Path::new("."));
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("mp3");

                    let tag = AudioTag {
                        title: row.get("title"),
                        artist: row.get("artist_name"),
                        album_artist: row.get("album_artist"),
                        album: row.get("album_title"),
                        track_number: row.get::<Option<i32>, _>("track_num").unwrap_or(0),
                        disc_number: row.get::<Option<i32>, _>("disc_num").unwrap_or(1),
                        release_date: row.get("date"),
                        album_disambiguation: row.get("disambiguation"),
                        total_discs: row.get::<Option<i32>, _>("total_discs").unwrap_or(0),
                        artist_sort: String::new(),
                        artists: vec![],
                        album_artist_sort: String::new(),
                        album_artists: vec![],
                        genres: vec![],
                        mbid_recording: None,
                        mbid_artist: None,
                        mbid_release_artist: None,
                        mbid_release: None,
                        acoust_id: None,
                        acoust_id_fingerprint: None,
                        lyrics: None,
                        cover: None,
                        track_gain: None,
                        track_peak: None,
                    };

                    let new_name = format_naming_pattern(pattern, &tag);
                    let expected_path = parent.join(format!("{}.{}", new_name, ext));
                    expected_path.to_string_lossy() == file_path.as_str()
                })
                .count() as f64;

            Some((conforming / total_tracks as f64) * 100.0)
        }
        _ => None,
    };

    let total_file_size: i64 = sqlx::query(
        r#"SELECT COALESCE(SUM(t.file_size), 0) FROM tracks t
        JOIN library_sources ls ON t.source_id = ls.id
        WHERE ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let tracks_with_cover: i64 = sqlx::query(
        r#"SELECT COUNT(*) FROM tracks t
        JOIN library_sources ls ON t.source_id = ls.id
        WHERE t.cover IS NOT NULL AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let tracks_without_cover: i64 = sqlx::query(
        r#"SELECT COUNT(*) FROM tracks t
        JOIN library_sources ls ON t.source_id = ls.id
        WHERE t.cover IS NULL AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let tracks_with_lyrics: i64 = sqlx::query(
        r#"SELECT COUNT(*) FROM tracks t
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE t.lyrics IS NOT NULL AND t.lyrics != ''
            AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let tracks_without_lyrics: i64 = sqlx::query(
        r#"SELECT COUNT(*) FROM tracks t
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE (t.lyrics IS NULL OR t.lyrics = '')
            AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let avg_track_duration: f64 = sqlx::query(
        r#"SELECT COALESCE(AVG(t.duration_secs), 0.0) FROM tracks t
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE t.duration_secs > 0 AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let shortest = sqlx::query(
        r#"SELECT t.title, t.duration_secs FROM tracks t
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE t.duration_secs = (SELECT MIN(duration_secs) FROM tracks WHERE duration_secs > 0)
             AND ls.source_type NOT LIKE 'recommendation:%'
           LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await?;

    let (shortest_track_title, shortest_track_duration) = if let Some(row) = shortest {
        (Some(row.get("title")), Some(row.get("duration_secs")))
    } else {
        (None, None)
    };

    let longest = sqlx::query(
        r#"SELECT t.title, t.duration_secs FROM tracks t
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE t.duration_secs = (SELECT MAX(duration_secs) FROM tracks)
             AND ls.source_type NOT LIKE 'recommendation:%'
           LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await?;

    let (longest_track_title, longest_track_duration) = if let Some(row) = longest {
        (Some(row.get("title")), Some(row.get("duration_secs")))
    } else {
        (None, None)
    };

    let tracks_per_album_avg: f64 = sqlx::query(
        r#"SELECT COALESCE(AVG(cnt), 0.0) FROM (
            SELECT COUNT(*) as cnt FROM tracks t
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE ls.source_type NOT LIKE 'recommendation:%'
            GROUP BY t.album_id
        )"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let tracks_per_artist_avg: f64 = sqlx::query(
        r#"SELECT COALESCE(AVG(cnt), 0.0) FROM (
            SELECT COUNT(*) as cnt FROM tracks t
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE ls.source_type NOT LIKE 'recommendation:%'
            GROUP BY t.artist_id
        )"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let tracks_with_mbid: i64 = sqlx::query(
        r#"SELECT COUNT(*) FROM tracks t
               JOIN library_sources ls ON t.source_id = ls.id
               WHERE t.mbid_recording IS NOT NULL
                 AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let oldest_year: Option<String> = sqlx::query_scalar(
        r#"SELECT MIN(SUBSTR(a.date, 1, 4)) FROM albums a
           JOIN tracks t ON a.id = t.album_id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE a.date IS NOT NULL AND LENGTH(a.date) >= 4
             AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?;

    let newest_year: Option<String> = sqlx::query_scalar(
        r#"SELECT MAX(SUBSTR(a.date, 1, 4)) FROM albums a
           JOIN tracks t ON a.id = t.album_id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE a.date IS NOT NULL AND LENGTH(a.date) >= 4
             AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?;

    Ok(LibraryStats {
        total_tracks,
        total_albums,
        total_artists,
        total_duration_secs: total_duration,
        average_bitrate: avg_bitrate,
        most_common_genre,
        genre_count,
        format_breakdown,
        decade_distribution,
        largest_album_title,
        largest_album_tracks,
        most_prolific_artist,
        most_prolific_artist_tracks,
        naming_conformity_pct,
        total_file_size,
        tracks_with_cover,
        tracks_without_cover,
        tracks_with_lyrics,
        tracks_without_lyrics,
        average_track_duration_secs: avg_track_duration,
        shortest_track_title,
        shortest_track_duration,
        longest_track_title,
        longest_track_duration,
        tracks_per_album_avg,
        tracks_per_artist_avg,
        tracks_with_mbid,
        oldest_year,
        newest_year,
    })
}

async fn get_library_stats_pg(
    pool: &sqlx::PgPool,
    naming_pattern: Option<&str>,
) -> Result<LibraryStats> {
    let total_tracks: i64 = sqlx::query(
        r#"SELECT COUNT(*)::bigint FROM tracks t
        JOIN library_sources ls ON t.source_id = ls.id
        WHERE ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let total_albums: i64 = sqlx::query(
        r#"SELECT COUNT(DISTINCT a.id)::bigint FROM albums a
           JOIN tracks t ON a.id = t.album_id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let total_artists: i64 = sqlx::query(
        r#"SELECT COUNT(DISTINCT ar.id)::bigint FROM artists ar
           JOIN tracks t ON ar.id = t.artist_id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let total_duration: f64 = sqlx::query(
        r#"SELECT COALESCE(SUM(t.duration_secs), 0.0) FROM tracks t
        JOIN library_sources ls ON t.source_id = ls.id
        WHERE ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let avg_bitrate: Option<f64> = sqlx::query(
        r#"SELECT AVG(t.bitrate::double precision) FROM tracks t
        JOIN library_sources ls ON t.source_id = ls.id
        WHERE t.bitrate > 0 AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_optional(pool)
    .await?
    .map(|r| r.get(0));

    let genre_row = sqlx::query(
        r#"SELECT g.name, COUNT(*)::bigint as cnt
           FROM track_genres tg
           JOIN genres g ON tg.genre_id = g.id
           JOIN tracks t ON tg.track_id = t.id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'
           GROUP BY g.id
           ORDER BY cnt DESC
           LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await?;

    let (most_common_genre, genre_count) = if let Some(row) = genre_row {
        (Some(row.get::<String, _>("name")), row.get::<i64, _>("cnt"))
    } else {
        (None, 0)
    };

    let format_rows = sqlx::query(
        r#"SELECT
               LOWER(
                 CASE
                   WHEN POSITION('.' IN t.file_path) > 0
                     THEN SUBSTR(t.file_path, POSITION('.' IN t.file_path) + 1)
                   ELSE 'unknown'
                 END
               ) AS ext,
               COUNT(*)::bigint as cnt
           FROM tracks t
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'
           GROUP BY ext
           ORDER BY cnt DESC"#,
    )
    .fetch_all(pool)
    .await?;

    let format_breakdown: Vec<FormatEntry> = format_rows
        .into_iter()
        .map(|row| FormatEntry {
            format: row.get("ext"),
            count: row.get("cnt"),
        })
        .collect();

    let decade_rows = sqlx::query(
        r#"SELECT
               CASE
                 WHEN a.date IS NOT NULL AND LENGTH(a.date) >= 4
                   THEN CAST((CAST(SUBSTR(a.date, 1, 4) AS INTEGER) / 10 * 10) AS TEXT) || 's'
                 ELSE 'Unknown'
               END AS decade,
               COUNT(*)::bigint as cnt
           FROM tracks t
           JOIN albums a ON t.album_id = a.id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'
           GROUP BY decade
           ORDER BY decade"#,
    )
    .fetch_all(pool)
    .await?;

    let decade_distribution: Vec<DecadeEntry> = decade_rows
        .into_iter()
        .map(|row| DecadeEntry {
            decade: row.get("decade"),
            count: row.get("cnt"),
        })
        .collect();

    let largest_album = sqlx::query(
        r#"SELECT a.title, COUNT(*)::bigint as cnt
           FROM tracks t
           JOIN albums a ON t.album_id = a.id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'
           GROUP BY t.album_id
           ORDER BY cnt DESC
           LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await?;

    let (largest_album_title, largest_album_tracks) = if let Some(row) = largest_album {
        (
            Some(row.get::<String, _>("title")),
            row.get::<i64, _>("cnt"),
        )
    } else {
        (None, 0)
    };

    let most_prolific = sqlx::query(
        r#"SELECT ar.name, COUNT(*)::bigint as cnt
           FROM tracks t
           JOIN artists ar ON t.artist_id = ar.id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ls.source_type NOT LIKE 'recommendation:%'
           GROUP BY t.artist_id
           ORDER BY cnt DESC
           LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await?;

    let (most_prolific_artist, most_prolific_artist_tracks) = if let Some(row) = most_prolific {
        (Some(row.get::<String, _>("name")), row.get::<i64, _>("cnt"))
    } else {
        (None, 0)
    };

    let naming_conformity_pct = match naming_pattern.filter(|p| !p.is_empty()) {
        Some(pattern) if total_tracks > 0 => {
            let rows = sqlx::query(
                r#"SELECT t.file_path, t.title, COALESCE(ar.name, '') AS artist_name,
                          COALESCE(a.title, '') AS album_title,
                          t.track_num, t.disc_num, a.date, a.disambiguation,
                          COALESCE(ar.name, '') AS album_artist,
                          a.total_discs
                   FROM tracks t
                   JOIN albums a ON t.album_id = a.id
                   JOIN artists ar ON t.artist_id = ar.id"#,
            )
            .fetch_all(pool)
            .await?;

            let conforming = rows
                .iter()
                .filter(|row| {
                    let file_path: String = row.get("file_path");
                    let path = Path::new(&file_path);
                    let parent = path.parent().unwrap_or(Path::new("."));
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("mp3");

                    let tag = AudioTag {
                        title: row.get("title"),
                        artist: row.get("artist_name"),
                        album_artist: row.get("album_artist"),
                        album: row.get("album_title"),
                        track_number: row.get::<Option<i32>, _>("track_num").unwrap_or(0),
                        disc_number: row.get::<Option<i32>, _>("disc_num").unwrap_or(1),
                        release_date: row.get("date"),
                        album_disambiguation: row.get("disambiguation"),
                        total_discs: row.get::<Option<i32>, _>("total_discs").unwrap_or(0),
                        artist_sort: String::new(),
                        artists: vec![],
                        album_artist_sort: String::new(),
                        album_artists: vec![],
                        genres: vec![],
                        mbid_recording: None,
                        mbid_artist: None,
                        mbid_release_artist: None,
                        mbid_release: None,
                        acoust_id: None,
                        acoust_id_fingerprint: None,
                        lyrics: None,
                        cover: None,
                        track_gain: None,
                        track_peak: None,
                    };

                    let new_name = format_naming_pattern(pattern, &tag);
                    let expected_path = parent.join(format!("{}.{}", new_name, ext));
                    expected_path.to_string_lossy() == file_path.as_str()
                })
                .count() as f64;

            Some((conforming / total_tracks as f64) * 100.0)
        }
        _ => None,
    };

    let total_file_size: i64 = sqlx::query(
        r#"SELECT COALESCE(SUM(t.file_size), 0)::bigint FROM tracks t
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let tracks_with_cover: i64 = sqlx::query(
        r#"SELECT COUNT(*)::bigint FROM tracks t
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE t.cover IS NOT NULL AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let tracks_without_cover: i64 = sqlx::query(
        r#"SELECT COUNT(*)::bigint FROM tracks t
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE t.cover IS NULL AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let tracks_with_lyrics: i64 = sqlx::query(
        r#"SELECT COUNT(*)::bigint FROM tracks t
        JOIN library_sources ls ON t.source_id = ls.id
        WHERE t.lyrics IS NOT NULL AND t.lyrics != ''
        AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let tracks_without_lyrics: i64 = sqlx::query(
        r#"SELECT COUNT(*)::bigint FROM tracks t
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE (t.lyrics IS NULL OR t.lyrics = '')
            AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let avg_track_duration: f64 = sqlx::query(
        r#"SELECT COALESCE(AVG(t.duration_secs), 0.0) FROM tracks t
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE t.duration_secs > 0 AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let shortest = sqlx::query(
        r#"SELECT t.title, t.duration_secs FROM tracks t
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE t.duration_secs = (SELECT MIN(duration_secs) FROM tracks WHERE duration_secs > 0)
             AND ls.source_type NOT LIKE 'recommendation:%'
           LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await?;

    let (shortest_track_title, shortest_track_duration) = if let Some(row) = shortest {
        (Some(row.get("title")), Some(row.get("duration_secs")))
    } else {
        (None, None)
    };

    let longest = sqlx::query(
        r#"SELECT t.title, t.duration_secs FROM tracks t
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE t.duration_secs = (SELECT MAX(duration_secs) FROM tracks)
             AND ls.source_type NOT LIKE 'recommendation:%'
           LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await?;

    let (longest_track_title, longest_track_duration) = if let Some(row) = longest {
        (Some(row.get("title")), Some(row.get("duration_secs")))
    } else {
        (None, None)
    };

    let tracks_per_album_avg: f64 = sqlx::query(
        r#"SELECT COALESCE(AVG(cnt)::double precision, 0.0) FROM (
            SELECT COUNT(*)::bigint as cnt FROM tracks t
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE ls.source_type NOT LIKE 'recommendation:%'
            GROUP BY t.album_id
        ) sub"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let tracks_per_artist_avg: f64 = sqlx::query(
        r#"SELECT COALESCE(AVG(cnt)::double precision, 0.0) FROM (
            SELECT COUNT(*)::bigint as cnt FROM tracks t
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE ls.source_type NOT LIKE 'recommendation:%'
            GROUP BY t.artist_id
        ) sub"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let tracks_with_mbid: i64 = sqlx::query(
        r#"SELECT COUNT(*)::bigint FROM tracks t
               JOIN library_sources ls ON t.source_id = ls.id
               WHERE t.mbid_recording IS NOT NULL
                 AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?
    .get(0);

    let oldest_year: Option<String> = sqlx::query_scalar(
        r#"SELECT MIN(SUBSTR(a.date::text, 1, 4)) FROM albums a
           JOIN tracks t ON a.id = t.album_id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE a.date IS NOT NULL AND LENGTH(a.date::text) >= 4
             AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?;

    let newest_year: Option<String> = sqlx::query_scalar(
        r#"SELECT MAX(SUBSTR(a.date::text, 1, 4)) FROM albums a
           JOIN tracks t ON a.id = t.album_id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE a.date IS NOT NULL AND LENGTH(a.date::text) >= 4
             AND ls.source_type NOT LIKE 'recommendation:%'"#,
    )
    .fetch_one(pool)
    .await?;

    Ok(LibraryStats {
        total_tracks,
        total_albums,
        total_artists,
        total_duration_secs: total_duration,
        average_bitrate: avg_bitrate,
        most_common_genre,
        genre_count,
        format_breakdown,
        decade_distribution,
        largest_album_title,
        largest_album_tracks,
        most_prolific_artist,
        most_prolific_artist_tracks,
        naming_conformity_pct,
        total_file_size,
        tracks_with_cover,
        tracks_without_cover,
        tracks_with_lyrics,
        tracks_without_lyrics,
        average_track_duration_secs: avg_track_duration,
        shortest_track_title,
        shortest_track_duration,
        longest_track_title,
        longest_track_duration,
        tracks_per_album_avg,
        tracks_per_artist_avg,
        tracks_with_mbid,
        oldest_year,
        newest_year,
    })
}
