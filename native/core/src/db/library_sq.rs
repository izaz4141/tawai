use anyhow::Result;
use sqlx::{Row, SqlitePool};

use crate::audio::tags::derive_sort_name;
use crate::signals::library::*;
use crate::utils::helper::sha256_hex;

pub async fn lookup_track(pool: &SqlitePool, track_id: &str) -> Result<Option<TrackInfo>> {
    let row = sqlx::query(
        r#"SELECT t.id, t.title, t.album_id, COALESCE(a.title, 'Unknown Album') AS album_title,
                  ar.name AS artists_string,
                  t.track_num, t.disc_num, t.duration_secs, t.file_path, t.file_size,
t.bitrate, t.mbid_recording, t.lyrics, t.track_gain, t.track_peak,
                   a.date AS release_date,
                   ar.mbid AS artist_mbid, a.mbid AS album_mbid,
                   ls.name AS source, ls.source_type AS source_type, t.cover,
                   (SELECT GROUP_CONCAT(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
            FROM tracks t
            JOIN albums a ON t.album_id = a.id
            JOIN artists ar ON t.artist_id = ar.id
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE t.id = ?"#,
    )
    .bind(track_id)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        let mut info = map_track_row(row);
        info.artists = get_track_artists(pool, &info.id).await?;
        Ok(Some(info))
    } else {
        Ok(None)
    }
}

pub async fn list_tracks(pool: &SqlitePool, album_id: Option<&str>) -> Result<Vec<TrackInfo>> {
    let rows = if let Some(aid) = album_id {
        sqlx::query(
            r#"SELECT t.id, t.title, t.album_id, COALESCE(a.title, 'Unknown Album') AS album_title,
                      ar.name AS artists_string,
                      t.track_num, t.disc_num, t.duration_secs, t.file_path, t.file_size,
                       t.bitrate, t.mbid_recording, t.lyrics, t.track_gain, t.track_peak,
                       a.date AS release_date,
                       ar.mbid AS artist_mbid, a.mbid AS album_mbid,
                      ls.name AS source, ls.source_type AS source_type, t.cover,
                      (SELECT GROUP_CONCAT(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
                FROM tracks t
                JOIN albums a ON t.album_id = a.id
                JOIN artists ar ON t.artist_id = ar.id
                JOIN library_sources ls ON t.source_id = ls.id
                WHERE t.album_id = ?
                ORDER BY t.disc_num, t.track_num"#,
        )
        .bind(aid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"SELECT t.id, t.title, t.album_id, COALESCE(a.title, 'Unknown Album') AS album_title,
                      ar.name AS artists_string,
                      t.track_num, t.disc_num, t.duration_secs, t.file_path, t.file_size,
                       t.bitrate, t.mbid_recording, t.lyrics, t.track_gain, t.track_peak,
                       a.date AS release_date,
                       ar.mbid AS artist_mbid, a.mbid AS album_mbid,
                      ls.name AS source, ls.source_type AS source_type, t.cover,
                      (SELECT GROUP_CONCAT(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
                FROM tracks t
                JOIN albums a ON t.album_id = a.id
                JOIN artists ar ON t.artist_id = ar.id
                JOIN library_sources ls ON t.source_id = ls.id
                ORDER BY ar.name, a.title, t.disc_num, t.track_num"#,
        )
        .fetch_all(pool)
        .await?
    };
    let mut tracks: Vec<TrackInfo> = rows.into_iter().map(|row| map_track_row(row)).collect();
    batch_populate_track_artists(pool, &mut tracks).await?;
    Ok(tracks)
}

pub async fn list_albums(pool: &SqlitePool, artist_id: Option<&str>) -> Result<Vec<AlbumInfo>> {
    let rows = if let Some(aid) = artist_id {
        sqlx::query(
            r#"SELECT al.id, al.title, ar.name AS artists_string,
                      al.date, al.cover,
                      (SELECT COUNT(*) FROM tracks t WHERE t.album_id = al.id) AS track_count
               FROM albums al
               JOIN artists ar ON al.artist_id = ar.id
               WHERE al.id IN (SELECT album_id FROM album_artists WHERE artist_id = ?)
               ORDER BY al.date DESC, al.title"#,
        )
        .bind(aid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"SELECT al.id, al.title, ar.name AS artists_string,
                      al.date, al.cover,
                      (SELECT COUNT(*) FROM tracks t WHERE t.album_id = al.id) AS track_count
               FROM albums al
               JOIN artists ar ON al.artist_id = ar.id
               ORDER BY al.date DESC, al.title"#,
        )
        .fetch_all(pool)
        .await?
    };
    let mut albums: Vec<AlbumInfo> = rows.into_iter().map(|row| map_album_row(row)).collect();
    batch_populate_album_artists(pool, &mut albums).await?;
    Ok(albums)
}

pub async fn list_artists(pool: &SqlitePool) -> Result<Vec<ArtistInfo>> {
    let rows = sqlx::query(
        r#"SELECT ar.id, ar.name, ar.sort_name, ar.mbid, ar.thumbnail_url,
                  (SELECT COUNT(*) FROM album_artists aa WHERE aa.artist_id = ar.id) AS album_count,
                  (SELECT COUNT(*) FROM track_artists ta WHERE ta.artist_id = ar.id) AS track_count
           FROM artists ar
           ORDER BY COALESCE(ar.sort_name, ar.name)"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|row| map_artist_row(row)).collect())
}

pub async fn list_playlists(pool: &SqlitePool) -> Result<Vec<PlaylistInfo>> {
    let rows = sqlx::query(
        r#"SELECT c.id, c.name, c.description, c.is_smart, c.created_at,
                  (SELECT COUNT(*) FROM collection_tracks ct WHERE ct.collection_id = c.id) AS track_count
           FROM collections c
           ORDER BY c.name"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|row| map_playlist_row(row)).collect())
}

pub async fn create_playlist(pool: &SqlitePool, user_id: &str, name: &str) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO collections (id, user_id, name) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(user_id)
        .bind(name)
        .execute(pool)
        .await?;
    Ok(id)
}

pub async fn delete_playlist(pool: &SqlitePool, playlist_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM collection_tracks WHERE collection_id = ?")
        .bind(playlist_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM collections WHERE id = ?")
        .bind(playlist_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_artist(
    pool: &SqlitePool,
    name: &str,
    sort_name: &str,
    mbid: Option<String>,
) -> Result<String> {
    let existing: Option<String> = sqlx::query_scalar("SELECT id FROM artists WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;

    if let Some(id) = existing {
        sqlx::query("UPDATE artists SET sort_name = ?, mbid = COALESCE(?, mbid), updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
            .bind(sort_name)
            .bind(mbid.as_deref())
            .bind(&id)
            .execute(pool)
            .await?;
        return Ok(id);
    }

    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO artists (id, name, sort_name, mbid) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(name)
        .bind(sort_name)
        .bind(mbid.as_deref())
        .execute(pool)
        .await?;
    Ok(id)
}

pub async fn insert_album(
    pool: &SqlitePool,
    title: &str,
    artist_id: &str,
    date: Option<String>,
    mbid: Option<String>,
    cover: Option<&[u8]>,
    disambiguation: Option<String>,
    total_discs: i32,
) -> Result<String> {
    let existing: Option<String> =
        sqlx::query_scalar("SELECT id FROM albums WHERE title = ? AND artist_id = ?")
            .bind(title)
            .bind(artist_id)
            .fetch_optional(pool)
            .await?;

    if let Some(id) = existing {
        sqlx::query(
            "UPDATE albums SET date = ?, mbid = COALESCE(?, mbid), disambiguation = COALESCE(?, disambiguation), total_discs = MAX(?, COALESCE(total_discs, 0)), updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
        )
        .bind(date)
        .bind(mbid.as_deref())
        .bind(disambiguation)
        .bind(total_discs)
        .bind(&id)
        .execute(pool)
        .await?;
        return Ok(id);
    }

    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO albums (id, title, artist_id, date, mbid, cover, disambiguation, total_discs) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(title)
    .bind(artist_id)
    .bind(date)
    .bind(mbid.as_deref())
    .bind(cover)
    .bind(disambiguation)
    .bind(total_discs)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn insert_genre(pool: &SqlitePool, name: &str) -> Result<String> {
    let existing: Option<String> = sqlx::query_scalar("SELECT id FROM genres WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;

    if let Some(id) = existing {
        return Ok(id);
    }

    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO genres (id, name) VALUES (?, ?)")
        .bind(&id)
        .bind(name)
        .execute(pool)
        .await?;
    Ok(id)
}

pub async fn insert_track_genre(pool: &SqlitePool, track_id: &str, genre_id: &str) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO track_genres (track_id, genre_id) VALUES (?, ?)")
        .bind(track_id)
        .bind(genre_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_album_cover(pool: &SqlitePool, album_id: &str, cover: &[u8]) -> Result<()> {
    sqlx::query("UPDATE albums SET cover = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(cover)
        .bind(album_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_track(
    pool: &SqlitePool,
    title: &str,
    album_id: &str,
    artist_id: &str,
    disc_num: i32,
    track_num: i32,
    duration_secs: f64,
    file_path: &str,
    file_hash: Option<&str>,
    source_id: &str,
    file_size: Option<i64>,
    bitrate: Option<i32>,
    sample_rate: Option<i32>,
    mbid_recording: Option<String>,
    cover: Option<&[u8]>,
    lyrics: Option<&str>,
    track_gain: Option<f64>,
    track_peak: Option<f64>,
) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        r#"INSERT INTO tracks (id, title, album_id, artist_id, disc_num, track_num,
            duration_secs, file_path, file_hash, source_id, file_size, bitrate, sample_rate, mbid_recording, cover, lyrics, track_gain, track_peak)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(title)
    .bind(album_id)
    .bind(artist_id)
    .bind(disc_num)
    .bind(track_num)
    .bind(duration_secs)
    .bind(file_path)
    .bind(file_hash)
    .bind(source_id)
    .bind(file_size)
    .bind(bitrate)
    .bind(sample_rate)
    .bind(mbid_recording.as_deref())
    .bind(cover)
    .bind(lyrics)
    .bind(track_gain)
    .bind(track_peak)
    .execute(pool)
    .await?;
    Ok(id)
}

// ---------------------------------------------------------------------------
// Track-artist junction
// ---------------------------------------------------------------------------

pub async fn insert_track_artists(
    pool: &SqlitePool,
    track_id: &str,
    artist_ids: &[String],
) -> Result<()> {
    for aid in artist_ids {
        sqlx::query("INSERT OR IGNORE INTO track_artists (track_id, artist_id) VALUES (?, ?)")
            .bind(track_id)
            .bind(aid)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn delete_track_artists(pool: &SqlitePool, track_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM track_artists WHERE track_id = ?")
        .bind(track_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_track_artists(pool: &SqlitePool, track_id: &str) -> Result<Vec<ArtistInfo>> {
    let rows = sqlx::query(
        r#"SELECT ar.id, ar.name, ar.sort_name, ar.mbid, ar.thumbnail_url,
                  0 AS album_count,
                  0 AS track_count
           FROM track_artists ta
           JOIN artists ar ON ta.artist_id = ar.id
           WHERE ta.track_id = ?
           ORDER BY ta.artist_id"#,
    )
    .bind(track_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(map_artist_row).collect())
}

// ---------------------------------------------------------------------------
// Album-artist junction
// ---------------------------------------------------------------------------

pub async fn insert_album_artists(
    pool: &SqlitePool,
    album_id: &str,
    artist_ids: &[String],
) -> Result<()> {
    for aid in artist_ids {
        sqlx::query("INSERT OR IGNORE INTO album_artists (album_id, artist_id) VALUES (?, ?)")
            .bind(album_id)
            .bind(aid)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn delete_album_artists(pool: &SqlitePool, album_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM album_artists WHERE album_id = ?")
        .bind(album_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_album_artists(pool: &SqlitePool, album_id: &str) -> Result<Vec<ArtistInfo>> {
    let rows = sqlx::query(
        r#"SELECT ar.id, ar.name, ar.sort_name, ar.mbid, ar.thumbnail_url,
                  0 AS album_count,
                  0 AS track_count
           FROM album_artists aa
           JOIN artists ar ON aa.artist_id = ar.id
           WHERE aa.album_id = ?
           ORDER BY aa.artist_id"#,
    )
    .bind(album_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(map_artist_row).collect())
}

// ---------------------------------------------------------------------------
// Batch populate track/album artists (avoids N+1 queries)
// ---------------------------------------------------------------------------

pub async fn batch_populate_track_artists(
    pool: &SqlitePool,
    tracks: &mut [TrackInfo],
) -> Result<()> {
    let ids: Vec<&str> = tracks.iter().map(|t| t.id.as_str()).collect();
    if ids.is_empty() {
        return Ok(());
    }
    let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("?{}", i)).collect();
    let sql = format!(
        r#"SELECT ta.track_id, ar.id, ar.name, ar.sort_name, ar.mbid, ar.thumbnail_url,
                  0 AS album_count, 0 AS track_count
            FROM track_artists ta
            JOIN artists ar ON ta.artist_id = ar.id
            WHERE ta.track_id IN ({})
            ORDER BY ta.track_id, ta.artist_id"#,
        placeholders.join(",")
    );
    let mut query = sqlx::query(&sql);
    for id in &ids {
        query = query.bind(id);
    }
    let rows = query.fetch_all(pool).await?;

    let mut map: std::collections::HashMap<String, Vec<ArtistInfo>> =
        std::collections::HashMap::new();
    for row in rows {
        let track_id: String = row.get("track_id");
        let artist = map_artist_row(row);
        map.entry(track_id).or_default().push(artist);
    }
    for track in tracks.iter_mut() {
        if let Some(artists) = map.remove(&track.id) {
            track.artists = artists;
        }
    }
    Ok(())
}

pub async fn batch_populate_album_artists(
    pool: &SqlitePool,
    albums: &mut [AlbumInfo],
) -> Result<()> {
    let ids: Vec<&str> = albums.iter().map(|a| a.id.as_str()).collect();
    if ids.is_empty() {
        return Ok(());
    }
    let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("?{}", i)).collect();
    let sql = format!(
        r#"SELECT aa.album_id, ar.id, ar.name, ar.sort_name, ar.mbid, ar.thumbnail_url,
                  0 AS album_count, 0 AS track_count
            FROM album_artists aa
            JOIN artists ar ON aa.artist_id = ar.id
            WHERE aa.album_id IN ({})
            ORDER BY aa.album_id, aa.artist_id"#,
        placeholders.join(",")
    );
    let mut query = sqlx::query(&sql);
    for id in &ids {
        query = query.bind(id);
    }
    let rows = query.fetch_all(pool).await?;

    let mut map: std::collections::HashMap<String, Vec<ArtistInfo>> =
        std::collections::HashMap::new();
    for row in rows {
        let album_id: String = row.get("album_id");
        let artist = map_artist_row(row);
        map.entry(album_id).or_default().push(artist);
    }
    for album in albums.iter_mut() {
        if let Some(artists) = map.remove(&album.id) {
            album.artists = artists;
        }
    }
    Ok(())
}

pub async fn track_exists_by_path(pool: &SqlitePool, file_path: &str) -> Result<bool> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracks WHERE file_path = ?")
        .bind(file_path)
        .fetch_one(pool)
        .await?;
    Ok(count > 0)
}

pub async fn track_exists_by_fingerprint(
    pool: &SqlitePool,
    fingerprint: &str,
) -> Result<Option<String>> {
    let track_id: Option<String> =
        sqlx::query_scalar("SELECT track_id FROM fingerprints WHERE fingerprint_hash = ?")
            .bind(sha256_hex(fingerprint))
            .fetch_optional(pool)
            .await?;
    Ok(track_id)
}

pub async fn insert_fingerprint(
    pool: &SqlitePool,
    track_id: &str,
    fingerprint: &str,
    acoust_id: Option<&str>,
) -> Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO fingerprints (id, track_id, fingerprint, fingerprint_hash, acoust_id) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(track_id)
    .bind(fingerprint)
    .bind(sha256_hex(fingerprint))
    .bind(acoust_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn all_track_file_paths(pool: &SqlitePool) -> Result<Vec<String>> {
    let rows: Vec<String> = sqlx::query_scalar("SELECT file_path FROM tracks")
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn delete_tracks_by_source_id(pool: &SqlitePool, source_id: &str) -> Result<()> {
    let track_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM tracks WHERE source_id = ?")
        .bind(source_id)
        .fetch_all(pool)
        .await?;

    let album_ids: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT album_id FROM tracks WHERE source_id = ? AND album_id IS NOT NULL",
    )
    .bind(source_id)
    .fetch_all(pool)
    .await?;

    let artist_ids: Vec<String> = {
        let mut ids: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT artist_id FROM tracks WHERE source_id = ? AND artist_id IS NOT NULL",
        )
        .bind(source_id)
        .fetch_all(pool)
        .await?;
        ids.extend(sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT artist_id FROM album_artists WHERE album_id IN (SELECT album_id FROM tracks WHERE source_id = ?)",
        )
        .bind(source_id)
        .fetch_all(pool)
        .await?);
        ids.extend(sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT artist_id FROM track_artists WHERE track_id IN (SELECT id FROM tracks WHERE source_id = ?)",
        )
        .bind(source_id)
        .fetch_all(pool)
        .await?);
        ids.sort();
        ids.dedup();
        ids
    };

    for track_id in &track_ids {
        sqlx::query("DELETE FROM track_genres WHERE track_id = ?")
            .bind(track_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM track_artists WHERE track_id = ?")
            .bind(track_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM fingerprints WHERE track_id = ?")
            .bind(track_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM playback_history WHERE track_id = ?")
            .bind(track_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM collection_tracks WHERE track_id = ?")
            .bind(track_id)
            .execute(pool)
            .await?;
    }

    sqlx::query("DELETE FROM tracks WHERE source_id = ?")
        .bind(source_id)
        .execute(pool)
        .await?;

    for album_id in &album_ids {
        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracks WHERE album_id = ?")
            .bind(album_id)
            .fetch_one(pool)
            .await?;
        if remaining == 0 {
            sqlx::query("DELETE FROM album_artists WHERE album_id = ?")
                .bind(album_id)
                .execute(pool)
                .await?;
            sqlx::query("DELETE FROM albums WHERE id = ?")
                .bind(album_id)
                .execute(pool)
                .await?;
        }
    }

    for artist_id in &artist_ids {
        let album_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM albums WHERE artist_id = ?")
                .bind(artist_id)
                .fetch_one(pool)
                .await?;
        let track_artist_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM track_artists WHERE artist_id = ?")
                .bind(artist_id)
                .fetch_one(pool)
                .await?;
        let album_artist_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM album_artists WHERE artist_id = ?")
                .bind(artist_id)
                .fetch_one(pool)
                .await?;
        if album_count == 0 && track_artist_count == 0 && album_artist_count == 0 {
            sqlx::query("DELETE FROM artists WHERE id = ?")
                .bind(artist_id)
                .execute(pool)
                .await?;
        }
    }

    sqlx::query("DELETE FROM genres WHERE id NOT IN (SELECT DISTINCT genre_id FROM track_genres)")
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn delete_track_by_file_path(pool: &SqlitePool, file_path: &str) -> Result<()> {
    let id: Option<String> = sqlx::query_scalar("SELECT id FROM tracks WHERE file_path = ?")
        .bind(file_path)
        .fetch_optional(pool)
        .await?;

    if let Some(track_id) = id {
        sqlx::query("DELETE FROM track_genres WHERE track_id = ?")
            .bind(&track_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM track_artists WHERE track_id = ?")
            .bind(&track_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM fingerprints WHERE track_id = ?")
            .bind(&track_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM playback_history WHERE track_id = ?")
            .bind(&track_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM collection_tracks WHERE track_id = ?")
            .bind(&track_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM tracks WHERE id = ?")
            .bind(&track_id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn delete_all_library(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DELETE FROM track_genres")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM track_artists")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM album_artists")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM fingerprints")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM playback_history")
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM tracks").execute(pool).await?;
    sqlx::query("DELETE FROM albums").execute(pool).await?;
    sqlx::query("DELETE FROM artists").execute(pool).await?;
    sqlx::query("DELETE FROM genres").execute(pool).await?;
    Ok(())
}

fn map_track_row(row: sqlx::sqlite::SqliteRow) -> TrackInfo {
    TrackInfo {
        id: row.get("id"),
        title: row.get("title"),
        album_id: row.get("album_id"),
        album_title: row.get("album_title"),
        artists: Vec::new(),
        artists_string: row.get("artists_string"),
        track_num: row.get("track_num"),
        disc_num: row.get("disc_num"),
        duration_secs: row.get("duration_secs"),
        file_path: row.get("file_path"),
        file_size: row.get("file_size"),
        bitrate: row.get("bitrate"),
        mbid_recording: row.get("mbid_recording"),
        artist_mbid: row.get("artist_mbid"),
        album_mbid: row.get("album_mbid"),
        lyrics: row.get("lyrics"),
        release_date: row.get("release_date"),
        source: row.get("source"),
        track_gain: row.get("track_gain"),
        track_peak: row.get("track_peak"),
        source_type: row.get("source_type"),
        genres: row
            .get::<Option<String>, _>("genres")
            .unwrap_or_default()
            .split("||")
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect(),
    }
}

fn map_album_row(row: sqlx::sqlite::SqliteRow) -> AlbumInfo {
    let title: String = row.get("title");
    AlbumInfo {
        id: row.get("id"),
        sort_name: Some(derive_sort_name(&title)),
        title,
        artists: Vec::new(),
        artists_string: row.get("artists_string"),
        release_date: row.get("date"),
        track_count: row.get("track_count"),
    }
}

fn map_artist_row(row: sqlx::sqlite::SqliteRow) -> ArtistInfo {
    ArtistInfo {
        id: row.get("id"),
        name: row.get("name"),
        sort_name: row.get("sort_name"),
        mbid: row.get("mbid"),
        thumbnail_url: row.get("thumbnail_url"),
        album_count: row.get("album_count"),
        track_count: row.get("track_count"),
    }
}

fn map_playlist_row(row: sqlx::sqlite::SqliteRow) -> PlaylistInfo {
    PlaylistInfo {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        is_smart: row.get::<bool, _>("is_smart"),
        track_count: row.get("track_count"),
        created_at: row.get("created_at"),
    }
}

pub async fn get_album_cover(pool: &SqlitePool, album_id: &str) -> Result<Option<Vec<u8>>> {
    let result: Option<Option<Vec<u8>>> =
        sqlx::query_scalar::<_, Option<Vec<u8>>>("SELECT cover FROM albums WHERE id = ?")
            .bind(album_id)
            .fetch_optional(pool)
            .await?;
    Ok(result.flatten().filter(|b| !b.is_empty()))
}

pub async fn get_track_cover(pool: &SqlitePool, track_id: &str) -> Result<Option<Vec<u8>>> {
    let result: Option<Option<Vec<u8>>> = sqlx::query_scalar::<_, Option<Vec<u8>>>(
        "SELECT COALESCE(t.cover, a.cover) FROM tracks t LEFT JOIN albums a ON t.album_id = a.id WHERE t.id = ?",
    )
    .bind(track_id)
    .fetch_optional(pool)
    .await?;
    Ok(result.flatten().filter(|b| !b.is_empty()))
}

// ---------------------------------------------------------------------------
// Playlist tracks
// ---------------------------------------------------------------------------

pub async fn get_playlist_tracks(pool: &SqlitePool, playlist_id: &str) -> Result<Vec<TrackInfo>> {
    let rows = sqlx::query(
        r#"SELECT t.id, t.title, t.album_id, COALESCE(a.title, 'Unknown Album') AS album_title,
                  ar.name AS artists_string,
                  t.track_num, t.disc_num, t.duration_secs, t.file_path, t.file_size,
t.bitrate, t.mbid_recording, t.lyrics, t.track_gain, t.track_peak,
                   a.date AS release_date,
                   ar.mbid AS artist_mbid, a.mbid AS album_mbid,
                   ls.name AS source, ls.source_type AS source_type, t.cover,
                   (SELECT GROUP_CONCAT(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
            FROM collection_tracks ct
           JOIN tracks t ON ct.track_id = t.id
           JOIN albums a ON t.album_id = a.id
           JOIN artists ar ON t.artist_id = ar.id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ct.collection_id = ?
           ORDER BY ct.position"#,
    )
    .bind(playlist_id)
    .fetch_all(pool)
    .await?;
    let mut tracks: Vec<TrackInfo> = rows.into_iter().map(map_track_row).collect();
    batch_populate_track_artists(pool, &mut tracks).await?;
    Ok(tracks)
}

pub async fn add_track_to_playlist(
    pool: &SqlitePool,
    playlist_id: &str,
    track_id: &str,
) -> Result<()> {
    let max_pos: Option<i32> =
        sqlx::query_scalar("SELECT MAX(position) FROM collection_tracks WHERE collection_id = ?")
            .bind(playlist_id)
            .fetch_optional(pool)
            .await?;
    let next_pos = max_pos.unwrap_or(-1) + 1;

    sqlx::query(
        "INSERT OR IGNORE INTO collection_tracks (collection_id, track_id, position) VALUES (?, ?, ?)",
    )
    .bind(playlist_id)
    .bind(track_id)
    .bind(next_pos)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_track_from_playlist(
    pool: &SqlitePool,
    playlist_id: &str,
    track_id: &str,
) -> Result<()> {
    sqlx::query("DELETE FROM collection_tracks WHERE collection_id = ? AND track_id = ?")
        .bind(playlist_id)
        .bind(track_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn reorder_playlist_tracks(
    pool: &SqlitePool,
    playlist_id: &str,
    track_ids: &[String],
) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM collection_tracks WHERE collection_id = ?")
        .bind(playlist_id)
        .execute(&mut *tx)
        .await?;
    for (i, tid) in track_ids.iter().enumerate() {
        sqlx::query(
            "INSERT INTO collection_tracks (collection_id, track_id, position) VALUES (?, ?, ?)",
        )
        .bind(playlist_id)
        .bind(tid)
        .bind(i as i32)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Identify / Tag Editor
// ---------------------------------------------------------------------------

pub async fn list_tracks_by_source(pool: &SqlitePool, source_id: &str) -> Result<Vec<TrackInfo>> {
    let rows = sqlx::query(
        r#"SELECT t.id, t.title, t.album_id, COALESCE(a.title, 'Unknown Album') AS album_title,
                  ar.name AS artists_string,
                  t.track_num, t.disc_num, t.duration_secs, t.file_path, t.file_size,
t.bitrate, t.mbid_recording, t.lyrics, t.track_gain, t.track_peak,
                   a.date AS release_date,
                   ar.mbid AS artist_mbid, a.mbid AS album_mbid,
                   ls.name AS source, ls.source_type AS source_type, t.cover,
                   (SELECT GROUP_CONCAT(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
            FROM tracks t
            JOIN albums a ON t.album_id = a.id
            JOIN artists ar ON t.artist_id = ar.id
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE t.source_id = ?
           ORDER BY ar.name, a.title, t.disc_num, t.track_num"#,
    )
    .bind(source_id)
    .fetch_all(pool)
    .await?;
    let mut tracks: Vec<TrackInfo> = rows.into_iter().map(map_track_row).collect();
    batch_populate_track_artists(pool, &mut tracks).await?;
    Ok(tracks)
}

pub async fn get_album_mbid(pool: &SqlitePool, album_id: &str) -> Result<Option<String>> {
    let result: Option<String> = sqlx::query_scalar("SELECT mbid FROM albums WHERE id = ?")
        .bind(album_id)
        .fetch_optional(pool)
        .await?;
    Ok(result.filter(|s| !s.is_empty()))
}

pub async fn list_unidentified_tracks(
    pool: &SqlitePool,
    source_id: Option<&str>,
) -> Result<Vec<TrackInfo>> {
    let sql = match source_id {
        Some(_) => {
            r#"SELECT t.id, t.title, t.album_id, COALESCE(a.title, 'Unknown Album') AS album_title,
                  ar.name AS artists_string,
                  t.track_num, t.disc_num, t.duration_secs, t.file_path, t.file_size,
t.bitrate, t.mbid_recording, t.lyrics, t.track_gain, t.track_peak,
                   a.date AS release_date,
                   ar.mbid AS artist_mbid, a.mbid AS album_mbid,
                   ls.name AS source, ls.source_type AS source_type, t.cover,
                   (SELECT GROUP_CONCAT(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
            FROM tracks t
            JOIN albums a ON t.album_id = a.id
            JOIN artists ar ON t.artist_id = ar.id
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE (t.mbid_recording IS NULL OR a.mbid IS NULL) AND t.source_id = ?
           ORDER BY ar.name, a.title, t.disc_num, t.track_num"#
        }
        None => {
            r#"SELECT t.id, t.title, t.album_id, COALESCE(a.title, 'Unknown Album') AS album_title,
                  ar.name AS artists_string,
                  t.track_num, t.disc_num, t.duration_secs, t.file_path, t.file_size,
t.bitrate, t.mbid_recording, t.lyrics, t.track_gain, t.track_peak,
                   a.date AS release_date,
                   ar.mbid AS artist_mbid, a.mbid AS album_mbid,
                   ls.name AS source, ls.source_type AS source_type, t.cover,
                   (SELECT GROUP_CONCAT(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
            FROM tracks t
            JOIN albums a ON t.album_id = a.id
            JOIN artists ar ON t.artist_id = ar.id
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE (t.mbid_recording IS NULL OR a.mbid IS NULL)
            ORDER BY ar.name, a.title, t.disc_num, t.track_num"#
        }
    };
    let mut query = sqlx::query(sql);
    if let Some(sid) = source_id {
        query = query.bind(sid);
    }
    let rows = query.fetch_all(pool).await?;
    let mut tracks: Vec<TrackInfo> = rows.into_iter().map(map_track_row).collect();
    batch_populate_track_artists(pool, &mut tracks).await?;
    Ok(tracks)
}

pub async fn lookup_fingerprint_by_id(
    pool: &SqlitePool,
    track_id: &str,
) -> Result<Option<(String, f64)>> {
    let row: Option<(String, f64)> = sqlx::query_as(
        "SELECT f.fingerprint, t.duration_secs AS duration FROM fingerprints f \
         JOIN tracks t ON t.id = f.track_id WHERE f.track_id = ?",
    )
    .bind(track_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn update_track(
    pool: &SqlitePool,
    track_id: &str,
    title: &str,
    artists: &[(String, Option<String>)],
    album_title: &str,
    album_artists: &[(String, Option<String>)],
    album_mbid: Option<&str>,
    date: Option<String>,
    track_num: Option<i32>,
    disc_num: Option<i32>,
    mbid_recording: Option<&str>,
    lyrics: Option<&str>,
    cover: Option<&[u8]>,
    new_file_path: Option<&str>,
    album_disambiguation: Option<String>,
    total_discs: i32,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    // Upsert each track artist
    let mut artist_ids: Vec<String> = Vec::new();
    for (name, mbid) in artists {
        let id = upsert_artist_in_tx(&mut *tx, name, mbid.as_deref()).await?;
        artist_ids.push(id);
    }

    // Upsert each album artist
    let mut album_artist_ids: Vec<String> = Vec::new();
    for (name, mbid) in album_artists {
        let id = upsert_artist_in_tx(&mut *tx, name, mbid.as_deref()).await?;
        album_artist_ids.push(id);
    }

    // Upsert album
    let album_id: String = {
        let existing: Option<String> =
            sqlx::query_scalar("SELECT id FROM albums WHERE title = ? AND artist_id = ?")
                .bind(album_title)
                .bind(&album_artist_ids[0])
                .fetch_optional(&mut *tx)
                .await?;

        if let Some(id) = existing {
            sqlx::query("UPDATE albums SET date = COALESCE(?, date), mbid = COALESCE(?, mbid), disambiguation = COALESCE(?, disambiguation), total_discs = MAX(?, COALESCE(total_discs, 0)), updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
                .bind(date)
                .bind(album_mbid)
                .bind(album_disambiguation.as_deref())
                .bind(total_discs)
                .bind(&id)
                .execute(&mut *tx)
                .await?;
            id
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO albums (id, title, artist_id, date, mbid, disambiguation, total_discs) VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(album_title)
            .bind(&album_artist_ids[0])
            .bind(date)
            .bind(album_mbid)
            .bind(album_disambiguation.as_deref())
            .bind(total_discs)
            .execute(&mut *tx)
            .await?;
            id
        }
    };

    // Update track
    let file_path_update = new_file_path.map(|p| p.to_string());
    sqlx::query(
        r#"UPDATE tracks SET title = ?, album_id = ?, artist_id = ?,
           disc_num = COALESCE(?, disc_num),
           track_num = COALESCE(?, track_num),
           mbid_recording = COALESCE(?, mbid_recording),
           lyrics = COALESCE(?, lyrics),
           cover = COALESCE(?, cover),
           file_path = COALESCE(?, file_path),
           updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
           WHERE id = ?"#,
    )
    .bind(title)
    .bind(&album_id)
    .bind(&artist_ids[0])
    .bind(disc_num)
    .bind(track_num)
    .bind(mbid_recording)
    .bind(lyrics)
    .bind(cover)
    .bind(file_path_update.as_deref())
    .bind(track_id)
    .execute(&mut *tx)
    .await?;

    // Rebuild track-artist links
    sqlx::query("DELETE FROM track_artists WHERE track_id = ?")
        .bind(track_id)
        .execute(&mut *tx)
        .await?;
    for aid in &artist_ids {
        sqlx::query("INSERT OR IGNORE INTO track_artists (track_id, artist_id) VALUES (?, ?)")
            .bind(track_id)
            .bind(aid)
            .execute(&mut *tx)
            .await?;
    }

    // Rebuild album-artist links
    sqlx::query("DELETE FROM album_artists WHERE album_id = ?")
        .bind(&album_id)
        .execute(&mut *tx)
        .await?;
    for aid in &album_artist_ids {
        sqlx::query("INSERT OR IGNORE INTO album_artists (album_id, artist_id) VALUES (?, ?)")
            .bind(&album_id)
            .bind(aid)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}

async fn upsert_artist_in_tx(
    conn: &mut sqlx::SqliteConnection,
    name: &str,
    mbid: Option<&str>,
) -> Result<String> {
    let existing: Option<String> = sqlx::query_scalar("SELECT id FROM artists WHERE name = ?")
        .bind(name)
        .fetch_optional(&mut *conn)
        .await?;

    if let Some(id) = existing {
        if let Some(mbid) = mbid {
            sqlx::query("UPDATE artists SET mbid = COALESCE(?, mbid), updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
                .bind(mbid)
                .bind(&id)
                .execute(&mut *conn)
                .await?;
        }
        Ok(id)
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        let sort_name = derive_sort_name(name);
        sqlx::query("INSERT INTO artists (id, name, sort_name, mbid) VALUES (?, ?, ?, ?)")
            .bind(&id)
            .bind(name)
            .bind(&sort_name)
            .bind(mbid)
            .execute(&mut *conn)
            .await?;
        Ok(id)
    }
}

pub async fn find_or_create_collection(
    pool: &SqlitePool,
    name: &str,
    description: &str,
    user_id: &str,
) -> Result<String> {
    let existing: Option<String> = sqlx::query_scalar("SELECT id FROM collections WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;

    if let Some(id) = existing {
        return Ok(id);
    }

    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO collections (id, name, description, user_id) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(name)
        .bind(description)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(id)
}
