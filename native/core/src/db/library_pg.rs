use anyhow::Result;
use sqlx::{PgPool, Row};

use super::library::{AlbumWriteMeta, TrackTagMeta};
use crate::audio::tags::{AudioTag, derive_sort_name};
use crate::signals::library::*;
use crate::utils::helper::sha256_hex;

pub async fn lookup_track(pool: &PgPool, track_id: &str) -> Result<Option<TrackInfo>> {
    let row = sqlx::query(
        r#"SELECT t.id, t.title, t.album_id, COALESCE(a.title, 'Unknown Album') AS album_title,
                  ar.name AS artists_string,
                  t.track_num, t.disc_num, t.duration_secs, t.file_path, t.file_size,
t.bitrate, t.mbid_recording, t.mbid_track, t.isrc, t.comment, t.copyright, t.encoder, t.script, t.lyrics, t.track_gain, t.track_peak,
                   a.date AS release_date,
                   ar.mbid AS artist_mbid, a.release_mbid AS release_mbid,
                   ls.name AS source, ls.source_type AS source_type, t.cover,
                   (SELECT STRING_AGG(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
            FROM tracks t
            JOIN albums a ON t.album_id = a.id
            JOIN artists ar ON t.artist_id = ar.id
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE t.id = $1"#,
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

pub async fn lookup_track_by_file_path(
    pool: &PgPool,
    file_path: &str,
) -> Result<Option<TrackInfo>> {
    let row = sqlx::query(
        r#"SELECT t.id, t.title, t.album_id, COALESCE(a.title, 'Unknown Album') AS album_title,
                  ar.name AS artists_string,
                  t.track_num, t.disc_num, t.duration_secs, t.file_path, t.file_size,
t.bitrate, t.mbid_recording, t.mbid_track, t.isrc, t.comment, t.copyright, t.encoder, t.script, t.lyrics, t.track_gain, t.track_peak,
                   a.date AS release_date,
                   ar.mbid AS artist_mbid, a.release_mbid AS release_mbid,
                   ls.name AS source, ls.source_type AS source_type, t.cover,
                   (SELECT STRING_AGG(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
            FROM tracks t
            JOIN albums a ON t.album_id = a.id
            JOIN artists ar ON t.artist_id = ar.id
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE t.file_path = $1"#,
    )
    .bind(file_path)
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

pub async fn list_tracks(pool: &PgPool, album_id: Option<&str>) -> Result<Vec<TrackInfo>> {
    let rows = if let Some(aid) = album_id {
        sqlx::query(
            r#"SELECT t.id, t.title, t.album_id, COALESCE(a.title, 'Unknown Album') AS album_title,
                      ar.name AS artists_string,
                      t.track_num, t.disc_num, t.duration_secs, t.file_path, t.file_size,
                       t.bitrate, t.mbid_recording, t.mbid_track, t.isrc, t.comment, t.copyright, t.encoder, t.script, t.lyrics, t.track_gain, t.track_peak,
                       a.date AS release_date,
                       ar.mbid AS artist_mbid, a.release_mbid AS release_mbid,
                      ls.name AS source, ls.source_type AS source_type, t.cover,
                      (SELECT STRING_AGG(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
                FROM tracks t
                JOIN albums a ON t.album_id = a.id
                JOIN artists ar ON t.artist_id = ar.id
                JOIN library_sources ls ON t.source_id = ls.id
                WHERE t.album_id = $1
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
                       t.bitrate, t.mbid_recording, t.mbid_track, t.isrc, t.comment, t.copyright, t.encoder, t.script, t.lyrics, t.track_gain, t.track_peak,
                       a.date AS release_date,
                       ar.mbid AS artist_mbid, a.release_mbid AS release_mbid,
                      ls.name AS source, ls.source_type AS source_type, t.cover,
                      (SELECT STRING_AGG(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
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

pub async fn list_albums(
    pool: &PgPool,
    artist_id: Option<&str>,
    album_id: Option<&str>,
) -> Result<Vec<AlbumInfo>> {
    let mut sql = String::from(
        r#"SELECT al.id, al.title, ar.name AS artists_string,
                  al.date, al.cover,
                  al.release_mbid, al.release_group_mbid, al.release_artist_mbid,
                  al.release_country, al.release_type, al.record_label, al.media,
                  al.original_date, al.total_tracks,
                  al.disambiguation, al.total_discs,
                  (SELECT COUNT(*) FROM tracks t WHERE t.album_id = al.id) AS track_count
           FROM albums al
           JOIN artists ar ON al.artist_id = ar.id"#,
    );
    let mut next_param = 1;
    if artist_id.is_some() {
        sql.push_str(&format!(
            " WHERE al.id IN (SELECT album_id FROM album_artists WHERE artist_id = ${next_param})"
        ));
        next_param += 1;
    }
    if album_id.is_some() {
        if artist_id.is_some() {
            sql.push_str(&format!(" AND al.id = ${next_param}"));
        } else {
            sql.push_str(&format!(" WHERE al.id = ${next_param}"));
        }
    }
    sql.push_str(" ORDER BY al.date DESC NULLS LAST, al.title");

    let mut query = sqlx::query(&sql);
    if let Some(aid) = artist_id {
        query = query.bind(aid);
    }
    if let Some(alb) = album_id {
        query = query.bind(alb);
    }
    let rows = query.fetch_all(pool).await?;
    let mut albums: Vec<AlbumInfo> = rows.into_iter().map(|row| map_album_row(row)).collect();
    batch_populate_album_artists(pool, &mut albums).await?;
    Ok(albums)
}

pub async fn list_artists(pool: &PgPool) -> Result<Vec<ArtistInfo>> {
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

pub async fn list_playlists(pool: &PgPool) -> Result<Vec<PlaylistInfo>> {
    let rows = sqlx::query(
        &format!(
            "SELECT c.id, c.name, c.description, c.is_smart, {}, (SELECT COUNT(*) FROM collection_tracks ct WHERE ct.collection_id = c.id) AS track_count FROM collections c ORDER BY c.name",
            super::ts_utc("c.created_at"),
        ),
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|row| map_playlist_row(row)).collect())
}

pub async fn create_playlist(pool: &PgPool, user_id: &str, name: &str) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO collections (id, user_id, name) VALUES ($1, $2, $3)")
        .bind(&id)
        .bind(user_id)
        .bind(name)
        .execute(pool)
        .await?;
    Ok(id)
}

pub async fn delete_playlist(pool: &PgPool, playlist_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM collection_tracks WHERE collection_id = $1")
        .bind(playlist_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM collections WHERE id = $1")
        .bind(playlist_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_artist(
    pool: &PgPool,
    name: &str,
    sort_name: &str,
    mbid: Option<String>,
) -> Result<String> {
    let existing: Option<String> = sqlx::query_scalar("SELECT id FROM artists WHERE name = $1")
        .bind(name)
        .fetch_optional(pool)
        .await?;

    if let Some(id) = existing {
        sqlx::query("UPDATE artists SET sort_name = $1, mbid = COALESCE($2, mbid), updated_at = NOW() WHERE id = $3")
            .bind(sort_name)
            .bind(mbid.as_deref())
            .bind(&id)
            .execute(pool)
            .await?;
        return Ok(id);
    }

    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO artists (id, name, sort_name, mbid) VALUES ($1, $2, $3, $4)")
        .bind(&id)
        .bind(name)
        .bind(sort_name)
        .bind(mbid.as_deref())
        .execute(pool)
        .await?;
    Ok(id)
}

pub async fn insert_album(
    pool: &PgPool,
    title: &str,
    artist_id: &str,
    cover: Option<&[u8]>,
    meta: &AlbumWriteMeta,
) -> Result<String> {
    let existing: Option<String> =
        sqlx::query_scalar("SELECT id FROM albums WHERE title = $1 AND artist_id = $2")
            .bind(title)
            .bind(artist_id)
            .fetch_optional(pool)
            .await?;

    if let Some(id) = existing {
        sqlx::query(
            "UPDATE albums SET date = $1, release_mbid = COALESCE($2, release_mbid), release_group_mbid = COALESCE($3, release_group_mbid), release_artist_mbid = COALESCE($4, release_artist_mbid), release_country = COALESCE($5, release_country), release_type = COALESCE($6, release_type), record_label = COALESCE($7, record_label), media = COALESCE($8, media), original_date = COALESCE($9, original_date), disambiguation = COALESCE($10, disambiguation), total_tracks = GREATEST($11, total_tracks), total_discs = GREATEST($12, COALESCE(total_discs, 0)), updated_at = NOW() WHERE id = $13",
        )
        .bind(meta.date.clone())
        .bind(meta.release_mbid.as_deref())
        .bind(meta.release_group_mbid.as_deref())
        .bind(meta.release_artist_mbid.as_deref())
        .bind(meta.release_country.as_deref())
        .bind(meta.release_type.as_deref())
        .bind(meta.record_label.as_deref())
        .bind(meta.media.as_deref())
        .bind(meta.original_date.as_deref())
        .bind(meta.disambiguation.clone())
        .bind(meta.total_tracks)
        .bind(meta.total_discs)
        .bind(&id)
        .execute(pool)
        .await?;
        return Ok(id);
    }

    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO albums (id, title, artist_id, date, release_mbid, release_group_mbid, release_artist_mbid, release_country, release_type, record_label, media, original_date, disambiguation, total_tracks, total_discs, cover) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
    )
    .bind(&id)
    .bind(title)
    .bind(artist_id)
    .bind(meta.date.clone())
    .bind(meta.release_mbid.as_deref())
    .bind(meta.release_group_mbid.as_deref())
    .bind(meta.release_artist_mbid.as_deref())
    .bind(meta.release_country.as_deref())
    .bind(meta.release_type.as_deref())
    .bind(meta.record_label.as_deref())
    .bind(meta.media.as_deref())
    .bind(meta.original_date.as_deref())
    .bind(meta.disambiguation.clone())
    .bind(meta.total_tracks)
    .bind(meta.total_discs)
    .bind(cover)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn insert_genre(pool: &PgPool, name: &str) -> Result<String> {
    let existing: Option<String> = sqlx::query_scalar("SELECT id FROM genres WHERE name = $1")
        .bind(name)
        .fetch_optional(pool)
        .await?;

    if let Some(id) = existing {
        return Ok(id);
    }

    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO genres (id, name) VALUES ($1, $2) ON CONFLICT (name) DO NOTHING")
        .bind(&id)
        .bind(name)
        .execute(pool)
        .await?;

    // If another connection inserted concurrently, fetch the existing id
    let final_id: String = sqlx::query_scalar("SELECT id FROM genres WHERE name = $1")
        .bind(name)
        .fetch_one(pool)
        .await?;
    Ok(final_id)
}

pub async fn insert_track_genre(pool: &PgPool, track_id: &str, genre_id: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO track_genres (track_id, genre_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(track_id)
    .bind(genre_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_album_cover(pool: &PgPool, album_id: &str, cover: &[u8]) -> Result<()> {
    sqlx::query("UPDATE albums SET cover = $1, updated_at = NOW() WHERE id = $2")
        .bind(cover)
        .bind(album_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_track(
    pool: &PgPool,
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
    cover: Option<&[u8]>,
    lyrics: Option<&str>,
    track_gain: Option<f64>,
    track_peak: Option<f64>,
    tags: &TrackTagMeta,
) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        r#"INSERT INTO tracks (id, title, album_id, artist_id, disc_num, track_num,
            duration_secs, file_path, file_hash, source_id, file_size, bitrate, sample_rate, mbid_recording, mbid_track, isrc, comment, copyright, encoder, script, cover, lyrics, track_gain, track_peak)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)"#,
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
    .bind(tags.mbid_recording.as_deref())
    .bind(tags.mbid_track.as_deref())
    .bind(tags.isrc.as_deref())
    .bind(tags.comment.as_deref())
    .bind(tags.copyright.as_deref())
    .bind(tags.encoder.as_deref())
    .bind(tags.script.as_deref())
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
    pool: &PgPool,
    track_id: &str,
    artist_ids: &[String],
) -> Result<()> {
    for aid in artist_ids {
        sqlx::query("INSERT INTO track_artists (track_id, artist_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(track_id)
            .bind(aid)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn delete_track_artists(pool: &PgPool, track_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM track_artists WHERE track_id = $1")
        .bind(track_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_track_artists(pool: &PgPool, track_id: &str) -> Result<Vec<ArtistInfo>> {
    let rows = sqlx::query(
        r#"SELECT ar.id, ar.name, ar.sort_name, ar.mbid, ar.thumbnail_url,
                  0::BIGINT AS album_count,
                  0::BIGINT AS track_count
           FROM track_artists ta
           JOIN artists ar ON ta.artist_id = ar.id
           WHERE ta.track_id = $1
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
    pool: &PgPool,
    album_id: &str,
    artist_ids: &[String],
) -> Result<()> {
    for aid in artist_ids {
        sqlx::query("INSERT INTO album_artists (album_id, artist_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(album_id)
            .bind(aid)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn delete_album_artists(pool: &PgPool, album_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM album_artists WHERE album_id = $1")
        .bind(album_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_album_artists(pool: &PgPool, album_id: &str) -> Result<Vec<ArtistInfo>> {
    let rows = sqlx::query(
        r#"SELECT ar.id, ar.name, ar.sort_name, ar.mbid, ar.thumbnail_url,
                  0::BIGINT AS album_count,
                  0::BIGINT AS track_count
           FROM album_artists aa
           JOIN artists ar ON aa.artist_id = ar.id
           WHERE aa.album_id = $1
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

pub async fn batch_populate_track_artists(pool: &PgPool, tracks: &mut [TrackInfo]) -> Result<()> {
    let ids: Vec<&str> = tracks.iter().map(|t| t.id.as_str()).collect();
    if ids.is_empty() {
        return Ok(());
    }
    let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("${}", i)).collect();
    let sql = format!(
        r#"SELECT ta.track_id, ar.id, ar.name, ar.sort_name, ar.mbid, ar.thumbnail_url,
                  0::BIGINT AS album_count, 0::BIGINT AS track_count
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

pub async fn batch_populate_album_artists(pool: &PgPool, albums: &mut [AlbumInfo]) -> Result<()> {
    let ids: Vec<&str> = albums.iter().map(|a| a.id.as_str()).collect();
    if ids.is_empty() {
        return Ok(());
    }
    let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("${}", i)).collect();
    let sql = format!(
        r#"SELECT aa.album_id, ar.id, ar.name, ar.sort_name, ar.mbid, ar.thumbnail_url,
                  0::BIGINT AS album_count, 0::BIGINT AS track_count
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

pub async fn track_exists_by_path(pool: &PgPool, file_path: &str) -> Result<bool> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracks WHERE file_path = $1")
        .bind(file_path)
        .fetch_one(pool)
        .await?;
    Ok(count > 0)
}

pub async fn track_exists_by_fingerprint(
    pool: &PgPool,
    fingerprint: &str,
) -> Result<Option<String>> {
    let track_id: Option<String> =
        sqlx::query_scalar("SELECT track_id FROM fingerprints WHERE fingerprint_hash = $1")
            .bind(sha256_hex(fingerprint))
            .fetch_optional(pool)
            .await?;
    Ok(track_id)
}

pub async fn fingerprint_paths_of(
    pool: &PgPool,
    paths: &std::collections::HashSet<String>,
) -> Result<std::collections::HashMap<String, String>> {
    if paths.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let paths: Vec<String> = paths.iter().cloned().collect();
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT f.fingerprint, t.file_path FROM fingerprints f \
         INNER JOIN tracks t ON t.id = f.track_id \
         WHERE t.file_path = ANY($1)",
    )
    .bind(&paths)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

pub async fn insert_fingerprint(
    pool: &PgPool,
    track_id: &str,
    fingerprint: &str,
    acoust_id: Option<&str>,
) -> Result<()> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO fingerprints (id, track_id, fingerprint, fingerprint_hash, acoust_id) VALUES ($1, $2, $3, $4, $5)",
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

pub async fn all_track_file_paths(pool: &PgPool) -> Result<Vec<String>> {
    let rows: Vec<String> = sqlx::query_scalar("SELECT file_path FROM tracks")
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn delete_tracks_by_source_id(pool: &PgPool, source_id: &str) -> Result<()> {
    let album_ids: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT album_id FROM tracks WHERE source_id = $1 AND album_id IS NOT NULL",
    )
    .bind(source_id)
    .fetch_all(pool)
    .await?;

    let artist_ids: Vec<String> = {
        let mut ids: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT artist_id FROM tracks WHERE source_id = $1 AND artist_id IS NOT NULL",
        )
        .bind(source_id)
        .fetch_all(pool)
        .await?;
        ids.extend(sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT artist_id FROM album_artists WHERE album_id IN (SELECT album_id FROM tracks WHERE source_id = $1)",
        )
        .bind(source_id)
        .fetch_all(pool)
        .await?);
        ids.extend(sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT artist_id FROM track_artists WHERE track_id IN (SELECT id FROM tracks WHERE source_id = $1)",
        )
        .bind(source_id)
        .fetch_all(pool)
        .await?);
        ids.sort();
        ids.dedup();
        ids
    };

    // All mutations below run in a single transaction. Deleting tracks cascades
    // via FK to track_genres, track_artists, fingerprints, playback_history and
    // collection_tracks, so thousands of per-track auto-commit writes become a
    // handful of statements with one commit.
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM tracks WHERE source_id = $1")
        .bind(source_id)
        .execute(&mut *tx)
        .await?;

    if !album_ids.is_empty() {
        let placeholders = (1..=album_ids.len())
            .map(|i| format!("${}", i))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "DELETE FROM albums WHERE id IN ({}) AND NOT EXISTS (SELECT 1 FROM tracks WHERE album_id = albums.id)",
            placeholders
        );
        let mut query = sqlx::query(&sql);
        for id in &album_ids {
            query = query.bind(id);
        }
        query.execute(&mut *tx).await?;
    }

    if !artist_ids.is_empty() {
        let placeholders = (1..=artist_ids.len())
            .map(|i| format!("${}", i))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            r#"DELETE FROM artists WHERE id IN ({})
               AND NOT EXISTS (SELECT 1 FROM albums WHERE artist_id = artists.id)
               AND NOT EXISTS (SELECT 1 FROM track_artists WHERE artist_id = artists.id)
               AND NOT EXISTS (SELECT 1 FROM album_artists WHERE artist_id = artists.id)"#,
            placeholders
        );
        let mut query = sqlx::query(&sql);
        for id in &artist_ids {
            query = query.bind(id);
        }
        query.execute(&mut *tx).await?;
    }

    sqlx::query("DELETE FROM genres WHERE id NOT IN (SELECT DISTINCT genre_id FROM track_genres)")
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn delete_track_by_file_path(pool: &PgPool, file_path: &str) -> Result<()> {
    let id: Option<String> = sqlx::query_scalar("SELECT id FROM tracks WHERE file_path = $1")
        .bind(file_path)
        .fetch_optional(pool)
        .await?;

    if let Some(track_id) = id {
        sqlx::query("DELETE FROM track_genres WHERE track_id = $1")
            .bind(&track_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM track_artists WHERE track_id = $1")
            .bind(&track_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM fingerprints WHERE track_id = $1")
            .bind(&track_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM playback_history WHERE track_id = $1")
            .bind(&track_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM collection_tracks WHERE track_id = $1")
            .bind(&track_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM tracks WHERE id = $1")
            .bind(&track_id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn delete_all_library(pool: &PgPool) -> Result<()> {
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

fn map_track_row(row: sqlx::postgres::PgRow) -> TrackInfo {
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
        release_mbid: row.get("release_mbid"),
        lyrics: row.get("lyrics"),
        release_date: row.get("release_date"),
        source: row.get("source"),
        source_type: row.get("source_type"),
        track_gain: row.get("track_gain"),
        track_peak: row.get("track_peak"),
        file_hash: row.try_get("file_hash").ok().flatten(),
        sample_rate: row.try_get("sample_rate").ok().flatten(),
        acoust_id_fingerprint: row.try_get("fingerprint").ok().flatten(),
        acoust_id: row.try_get("acoust_id").ok().flatten(),
        mbid_track: row.try_get("mbid_track").ok().flatten(),
        isrc: row.try_get("isrc").ok().flatten(),
        comment: row.try_get("comment").ok().flatten(),
        copyright: row.try_get("copyright").ok().flatten(),
        encoder: row.try_get("encoder").ok().flatten(),
        script: row.try_get("script").ok().flatten(),
        genres: row
            .get::<Option<String>, _>("genres")
            .unwrap_or_default()
            .split("||")
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect(),
    }
}

fn map_album_row(row: sqlx::postgres::PgRow) -> AlbumInfo {
    let title: String = row.get("title");
    AlbumInfo {
        id: row.get("id"),
        sort_name: Some(derive_sort_name(&title)),
        title,
        artists: Vec::new(),
        artists_string: row.get("artists_string"),
        release_date: row.get("date"),
        track_count: row.get("track_count"),
        disambiguation: row.try_get("disambiguation").ok().flatten(),
        release_mbid: row.try_get("release_mbid").ok().flatten(),
        release_group_mbid: row.try_get("release_group_mbid").ok().flatten(),
        release_artist_mbid: row.try_get("release_artist_mbid").ok().flatten(),
        release_country: row.try_get("release_country").ok().flatten(),
        release_type: row.try_get("release_type").ok().flatten(),
        record_label: row.try_get("record_label").ok().flatten(),
        media: row.try_get("media").ok().flatten(),
        original_date: row.try_get("original_date").ok().flatten(),
        total_tracks: row
            .try_get::<i32, _>("total_tracks")
            .map(|v| v as i64)
            .unwrap_or(0),
        total_discs: row
            .try_get::<i32, _>("total_discs")
            .map(|v| v as i64)
            .unwrap_or(0),
    }
}

fn map_artist_row(row: sqlx::postgres::PgRow) -> ArtistInfo {
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

fn map_playlist_row(row: sqlx::postgres::PgRow) -> PlaylistInfo {
    PlaylistInfo {
        id: row.get("id"),
        name: row.get("name"),
        description: row.get("description"),
        is_smart: row.get::<bool, _>("is_smart"),
        track_count: row.get("track_count"),
        created_at: row.get("created_at"),
    }
}

pub async fn get_album_cover(pool: &PgPool, album_id: &str) -> Result<Option<Vec<u8>>> {
    let result: Option<Option<Vec<u8>>> =
        sqlx::query_scalar::<_, Option<Vec<u8>>>("SELECT cover FROM albums WHERE id = $1")
            .bind(album_id)
            .fetch_optional(pool)
            .await?;
    Ok(result.flatten().filter(|b| !b.is_empty()))
}

pub async fn get_track_cover(pool: &PgPool, track_id: &str) -> Result<Option<Vec<u8>>> {
    let result: Option<Option<Vec<u8>>> = sqlx::query_scalar::<_, Option<Vec<u8>>>(
        "SELECT COALESCE(t.cover, a.cover) FROM tracks t LEFT JOIN albums a ON t.album_id = a.id WHERE t.id = $1",
    )
    .bind(track_id)
    .fetch_optional(pool)
    .await?;
    Ok(result.flatten().filter(|b| !b.is_empty()))
}

// ---------------------------------------------------------------------------
// Playlist tracks
// ---------------------------------------------------------------------------

pub async fn get_playlist_tracks(pool: &PgPool, playlist_id: &str) -> Result<Vec<TrackInfo>> {
    let rows = sqlx::query(
        r#"SELECT t.id, t.title, t.album_id, COALESCE(a.title, 'Unknown Album') AS album_title,
                  ar.name AS artists_string,
                  t.track_num, t.disc_num, t.duration_secs, t.file_path, t.file_size,
t.bitrate, t.mbid_recording, t.mbid_track, t.isrc, t.comment, t.copyright, t.encoder, t.script, t.lyrics, t.track_gain, t.track_peak,
                   a.date AS release_date,
                   ar.mbid AS artist_mbid, a.release_mbid AS release_mbid,
                   ls.name AS source, ls.source_type AS source_type, t.cover,
                   (SELECT STRING_AGG(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
            FROM collection_tracks ct
           JOIN tracks t ON ct.track_id = t.id
           JOIN albums a ON t.album_id = a.id
           JOIN artists ar ON t.artist_id = ar.id
           JOIN library_sources ls ON t.source_id = ls.id
           WHERE ct.collection_id = $1
           ORDER BY ct.position"#,
    )
    .bind(playlist_id)
    .fetch_all(pool)
    .await?;
    let mut tracks: Vec<TrackInfo> = rows.into_iter().map(map_track_row).collect();
    batch_populate_track_artists(pool, &mut tracks).await?;
    Ok(tracks)
}

pub async fn add_track_to_playlist(pool: &PgPool, playlist_id: &str, track_id: &str) -> Result<()> {
    let max_pos: Option<i32> =
        sqlx::query_scalar("SELECT MAX(position) FROM collection_tracks WHERE collection_id = $1")
            .bind(playlist_id)
            .fetch_optional(pool)
            .await?;
    let next_pos = max_pos.unwrap_or(-1) + 1;

    sqlx::query(
        "INSERT INTO collection_tracks (collection_id, track_id, position) VALUES ($1, $2, $3) ON CONFLICT (collection_id, track_id) DO NOTHING",
    )
    .bind(playlist_id)
    .bind(track_id)
    .bind(next_pos)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_track_from_playlist(
    pool: &PgPool,
    playlist_id: &str,
    track_id: &str,
) -> Result<()> {
    sqlx::query("DELETE FROM collection_tracks WHERE collection_id = $1 AND track_id = $2")
        .bind(playlist_id)
        .bind(track_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn reorder_playlist_tracks(
    pool: &PgPool,
    playlist_id: &str,
    track_ids: &[String],
) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM collection_tracks WHERE collection_id = $1")
        .bind(playlist_id)
        .execute(&mut *tx)
        .await?;
    for (i, tid) in track_ids.iter().enumerate() {
        sqlx::query(
            "INSERT INTO collection_tracks (collection_id, track_id, position) VALUES ($1, $2, $3)",
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

pub async fn list_tracks_by_source(pool: &PgPool, source_id: &str) -> Result<Vec<TrackInfo>> {
    let rows = sqlx::query(
        r#"SELECT t.id, t.title, t.album_id, COALESCE(a.title, 'Unknown Album') AS album_title,
                  ar.name AS artists_string,
                  t.track_num, t.disc_num, t.duration_secs, t.file_path, t.file_size,
t.bitrate, t.mbid_recording, t.mbid_track, t.isrc, t.comment, t.copyright, t.encoder, t.script, t.lyrics, t.track_gain, t.track_peak,
                   a.date AS release_date,
                   ar.mbid AS artist_mbid, a.release_mbid AS release_mbid,
                   ls.name AS source, ls.source_type AS source_type, t.cover,
                   t.file_hash, t.sample_rate,
                   (SELECT f.fingerprint FROM fingerprints f WHERE f.track_id = t.id) AS fingerprint,
                   (SELECT f.acoust_id FROM fingerprints f WHERE f.track_id = t.id) AS acoust_id,
                   (SELECT STRING_AGG(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
            FROM tracks t
            JOIN albums a ON t.album_id = a.id
            JOIN artists ar ON t.artist_id = ar.id
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE t.source_id = $1
           ORDER BY ar.name, a.title, t.disc_num, t.track_num"#,
    )
    .bind(source_id)
    .fetch_all(pool)
    .await?;
    let mut tracks: Vec<TrackInfo> = rows.into_iter().map(map_track_row).collect();
    batch_populate_track_artists(pool, &mut tracks).await?;
    Ok(tracks)
}

pub async fn get_album_mbid(pool: &PgPool, album_id: &str) -> Result<Option<String>> {
    let result: Option<String> =
        sqlx::query_scalar("SELECT release_mbid FROM albums WHERE id = $1")
            .bind(album_id)
            .fetch_optional(pool)
            .await?;
    Ok(result.filter(|s| !s.is_empty()))
}

pub async fn list_unidentified_tracks(
    pool: &PgPool,
    source_id: Option<&str>,
) -> Result<Vec<TrackInfo>> {
    let sql = match source_id {
        Some(_) => {
            r#"SELECT t.id, t.title, t.album_id, COALESCE(a.title, 'Unknown Album') AS album_title,
                  ar.name AS artists_string,
                  t.track_num, t.disc_num, t.duration_secs, t.file_path, t.file_size,
t.bitrate, t.mbid_recording, t.mbid_track, t.isrc, t.comment, t.copyright, t.encoder, t.script, t.lyrics, t.track_gain, t.track_peak,
                   a.date AS release_date,
                   ar.mbid AS artist_mbid, a.release_mbid AS release_mbid,
                   ls.name AS source, ls.source_type AS source_type, t.cover,
                   (SELECT STRING_AGG(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
            FROM tracks t
            JOIN albums a ON t.album_id = a.id
            JOIN artists ar ON t.artist_id = ar.id
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE (t.mbid_recording IS NULL OR a.release_mbid IS NULL) AND t.source_id = $1
           ORDER BY ar.name, a.title, t.disc_num, t.track_num"#
        }
        None => {
            r#"SELECT t.id, t.title, t.album_id, COALESCE(a.title, 'Unknown Album') AS album_title,
                  ar.name AS artists_string,
                  t.track_num, t.disc_num, t.duration_secs, t.file_path, t.file_size,
t.bitrate, t.mbid_recording, t.mbid_track, t.isrc, t.comment, t.copyright, t.encoder, t.script, t.lyrics, t.track_gain, t.track_peak,
                   a.date AS release_date,
                   ar.mbid AS artist_mbid, a.release_mbid AS release_mbid,
                   ls.name AS source, ls.source_type AS source_type, t.cover,
                   (SELECT STRING_AGG(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id) AS genres
            FROM tracks t
            JOIN albums a ON t.album_id = a.id
            JOIN artists ar ON t.artist_id = ar.id
            JOIN library_sources ls ON t.source_id = ls.id
            WHERE (t.mbid_recording IS NULL OR a.release_mbid IS NULL)
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
    pool: &PgPool,
    track_id: &str,
) -> Result<Option<(String, f64)>> {
    let row: Option<(String, f64)> = sqlx::query_as(
        "SELECT f.fingerprint, t.duration_secs AS duration FROM fingerprints f \
         JOIN tracks t ON t.id = f.track_id WHERE f.track_id = $1",
    )
    .bind(track_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn update_track(
    pool: &PgPool,
    track_id: &str,
    title: &str,
    artists: &[(String, Option<String>, Option<String>)],
    album_title: &str,
    album_artists: &[(String, Option<String>, Option<String>)],
    album_meta: &AlbumWriteMeta,
    track_meta: &TrackTagMeta,
    track_num: Option<i32>,
    disc_num: Option<i32>,
    lyrics: Option<&str>,
    cover: Option<&[u8]>,
    new_file_path: Option<&str>,
    genres: Option<&[String]>,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    // Upsert each track artist (name, mbid, sort name).
    let mut artist_ids: Vec<String> = Vec::new();
    for (name, mbid, sort) in artists {
        let id =
            upsert_artist_with_sort_in_tx(&mut *tx, name, sort.as_deref(), mbid.as_deref()).await?;
        artist_ids.push(id);
    }

    // Upsert each album artist.
    let mut album_artist_ids: Vec<String> = Vec::new();
    for (name, mbid, sort) in album_artists {
        let id =
            upsert_artist_with_sort_in_tx(&mut *tx, name, sort.as_deref(), mbid.as_deref()).await?;
        album_artist_ids.push(id);
    }
    let album_artist_id = album_artist_ids
        .first()
        .ok_or_else(|| anyhow::anyhow!("update_track needs at least one album artist"))?;

    // Upsert album. Every extended column is write-if-set (COALESCE), and the
    // totals only ever grow (GREATEST), matching the scan/import convention.
    let album_id: String = {
        let existing: Option<String> =
            sqlx::query_scalar("SELECT id FROM albums WHERE title = $1 AND artist_id = $2")
                .bind(album_title)
                .bind(album_artist_id)
                .fetch_optional(&mut *tx)
                .await?;

        if let Some(id) = existing {
            sqlx::query(
                "UPDATE albums SET date = COALESCE($1, date), release_mbid = COALESCE($2, release_mbid), release_group_mbid = COALESCE($3, release_group_mbid), release_artist_mbid = COALESCE($4, release_artist_mbid), release_country = COALESCE($5, release_country), release_type = COALESCE($6, release_type), record_label = COALESCE($7, record_label), media = COALESCE($8, media), original_date = COALESCE($9, original_date), disambiguation = COALESCE($10, disambiguation), total_tracks = GREATEST($11, COALESCE(total_tracks, 0)), total_discs = GREATEST($12, COALESCE(total_discs, 0)), updated_at = NOW() WHERE id = $13",
            )
            .bind(album_meta.date.clone())
            .bind(album_meta.release_mbid.as_deref())
            .bind(album_meta.release_group_mbid.as_deref())
            .bind(album_meta.release_artist_mbid.as_deref())
            .bind(album_meta.release_country.as_deref())
            .bind(album_meta.release_type.as_deref())
            .bind(album_meta.record_label.as_deref())
            .bind(album_meta.media.as_deref())
            .bind(album_meta.original_date.as_deref())
            .bind(album_meta.disambiguation.as_deref())
            .bind(album_meta.total_tracks)
            .bind(album_meta.total_discs)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
            id
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO albums (id, title, artist_id, date, release_mbid, release_group_mbid, release_artist_mbid, release_country, release_type, record_label, media, original_date, disambiguation, total_tracks, total_discs) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
            )
            .bind(&id)
            .bind(album_title)
            .bind(album_artist_id)
            .bind(album_meta.date.clone())
            .bind(album_meta.release_mbid.as_deref())
            .bind(album_meta.release_group_mbid.as_deref())
            .bind(album_meta.release_artist_mbid.as_deref())
            .bind(album_meta.release_country.as_deref())
            .bind(album_meta.release_type.as_deref())
            .bind(album_meta.record_label.as_deref())
            .bind(album_meta.media.as_deref())
            .bind(album_meta.original_date.as_deref())
            .bind(album_meta.disambiguation.as_deref())
            .bind(album_meta.total_tracks)
            .bind(album_meta.total_discs)
            .execute(&mut *tx)
            .await?;
            id
        }
    };

    // Update track. Every extended column is write-if-set (COALESCE).
    let file_path_update = new_file_path.map(|p| p.to_string());
    sqlx::query(
        r#"UPDATE tracks SET title = $1, album_id = $2, artist_id = $3,
           disc_num = COALESCE($4, disc_num),
           track_num = COALESCE($5, track_num),
           mbid_recording = COALESCE($6, mbid_recording),
           mbid_track = COALESCE($7, mbid_track),
           isrc = COALESCE($8, isrc),
           comment = COALESCE($9, comment),
           copyright = COALESCE($10, copyright),
           script = COALESCE($11, script),
           lyrics = COALESCE($12, lyrics),
           cover = COALESCE($13, cover),
           file_path = COALESCE($14, file_path),
           updated_at = NOW()
           WHERE id = $15"#,
    )
    .bind(title)
    .bind(&album_id)
    .bind(artist_ids.first().map(String::as_str))
    .bind(disc_num)
    .bind(track_num)
    .bind(track_meta.mbid_recording.as_deref())
    .bind(track_meta.mbid_track.as_deref())
    .bind(track_meta.isrc.as_deref())
    .bind(track_meta.comment.as_deref())
    .bind(track_meta.copyright.as_deref())
    .bind(track_meta.script.as_deref())
    .bind(lyrics)
    .bind(cover)
    .bind(file_path_update.as_deref())
    .bind(track_id)
    .execute(&mut *tx)
    .await?;

    // Rebuild track-artist links
    sqlx::query("DELETE FROM track_artists WHERE track_id = $1")
        .bind(track_id)
        .execute(&mut *tx)
        .await?;
    for aid in &artist_ids {
        sqlx::query("INSERT INTO track_artists (track_id, artist_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(track_id)
            .bind(aid)
            .execute(&mut *tx)
            .await?;
    }

    // Rebuild album-artist links
    sqlx::query("DELETE FROM album_artists WHERE album_id = $1")
        .bind(&album_id)
        .execute(&mut *tx)
        .await?;
    for aid in &album_artist_ids {
        sqlx::query("INSERT INTO album_artists (album_id, artist_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(&album_id)
            .bind(aid)
            .execute(&mut *tx)
            .await?;
    }

    // Rebuild genre links only when the caller supplied a genre list.
    if let Some(genres) = genres {
        sqlx::query("DELETE FROM track_genres WHERE track_id = $1")
            .bind(track_id)
            .execute(&mut *tx)
            .await?;
        for name in genres {
            if name.trim().is_empty() {
                continue;
            }
            let genre_id = upsert_genre_in_tx(&mut *tx, name).await?;
            sqlx::query("INSERT INTO track_genres (track_id, genre_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                .bind(track_id)
                .bind(&genre_id)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

/// Update a track's on-disk path. Kept lightweight on purpose: the batch rename
/// flow only needs to persist the new location, not rewrite metadata.
pub async fn set_track_file_path(pool: &PgPool, track_id: &str, new_file_path: &str) -> Result<()> {
    sqlx::query("UPDATE tracks SET file_path = $1, updated_at = NOW() WHERE id = $2")
        .bind(new_file_path)
        .bind(track_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_file_hash_by_path(
    pool: &PgPool,
    file_path: &str,
    file_hash: &str,
) -> Result<()> {
    sqlx::query("UPDATE tracks SET file_hash = $1, updated_at = NOW() WHERE file_path = $2")
        .bind(file_hash)
        .bind(file_path)
        .execute(pool)
        .await?;
    Ok(())
}

/// Apply a full tag-editor edit to an existing library track row.
///
/// Unlike [`update_track`] (partial and `COALESCE`-based, used by the identify
/// flow) this mirrors what the tag editor wrote to disk: editable fields are
/// set to the edited value directly, so a cleared tag clears in the library
/// too. The album, artists, artist links, and genre links are updated in the
/// same transaction.
pub async fn update_track_tags_from_audio_tag(
    pool: &PgPool,
    track_id: &str,
    tag: &AudioTag,
    clear_cover: bool,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    // Track artists: the primary/combined entry carries the recording artist
    // MBID and the edited sort name; the rest derive their sort name. Falls
    // back to the single `artist` field when the list is empty.
    let artists: Vec<&str> = if tag.artists.is_empty() {
        vec![tag.artist.as_str()]
    } else {
        tag.artists.iter().map(String::as_str).collect()
    };
    let mut artist_ids: Vec<String> = Vec::new();
    for (i, name) in artists.iter().enumerate() {
        let mbid = (i == 0 || artists.len() == 1)
            .then(|| tag.mbid_artist.as_deref())
            .flatten();
        let sort = (i == 0 && !tag.artist_sort.is_empty()).then(|| tag.artist_sort.as_str());
        artist_ids.push(upsert_artist_with_sort_in_tx(&mut *tx, name, sort, mbid).await?);
    }

    // Album artists: same primary-MBID rule as the scan insert flow.
    let album_artists: Vec<&str> = if tag.album_artists.is_empty() {
        vec![tag.album_artist.as_str()]
    } else {
        tag.album_artists.iter().map(String::as_str).collect()
    };
    let mut album_artist_ids: Vec<String> = Vec::new();
    for (i, name) in album_artists.iter().enumerate() {
        let mbid = (i == 0 || album_artists.len() == 1)
            .then(|| tag.mbid_release_artist.as_deref())
            .flatten();
        let sort =
            (i == 0 && !tag.album_artist_sort.is_empty()).then(|| tag.album_artist_sort.as_str());
        album_artist_ids.push(upsert_artist_with_sort_in_tx(&mut *tx, name, sort, mbid).await?);
    }

    let album_id = upsert_album_from_tag_in_tx(&mut *tx, tag, &album_artist_ids[0]).await?;

    // Set the editable track columns directly. Numeric fields use a `0`
    // sentinel for "absent", so keep the existing value in that case.
    sqlx::query(
        r#"UPDATE tracks SET title = $1, album_id = $2, artist_id = $3,
           disc_num = COALESCE($4, disc_num),
           track_num = COALESCE($5, track_num),
           mbid_recording = $6,
           mbid_track = $7,
           isrc = $8,
           comment = $9,
           copyright = $10,
           script = $11,
           lyrics = $12,
           cover = CASE WHEN $15 THEN $13 ELSE COALESCE($13, tracks.cover) END,
           updated_at = NOW()
           WHERE id = $14"#,
    )
    .bind(&tag.title)
    .bind(&album_id)
    .bind(artist_ids.first().map(String::as_str))
    .bind((tag.disc_number > 0).then_some(tag.disc_number))
    .bind((tag.track_number > 0).then_some(tag.track_number))
    .bind(tag.mbid_recording.as_deref())
    .bind(tag.mbid_track.as_deref())
    .bind(tag.isrc.as_deref())
    .bind(tag.comment.as_deref())
    .bind(tag.copyright.as_deref())
    .bind(tag.script.as_deref())
    .bind(tag.lyrics.as_deref())
    .bind(tag.cover.as_deref())
    .bind(track_id)
    .bind(clear_cover)
    .execute(&mut *tx)
    .await?;

    // Rebuild track-artist links
    sqlx::query("DELETE FROM track_artists WHERE track_id = $1")
        .bind(track_id)
        .execute(&mut *tx)
        .await?;
    for aid in &artist_ids {
        sqlx::query("INSERT INTO track_artists (track_id, artist_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(track_id)
            .bind(aid)
            .execute(&mut *tx)
            .await?;
    }

    // Rebuild album-artist links
    sqlx::query("DELETE FROM album_artists WHERE album_id = $1")
        .bind(&album_id)
        .execute(&mut *tx)
        .await?;
    for aid in &album_artist_ids {
        sqlx::query("INSERT INTO album_artists (album_id, artist_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(&album_id)
            .bind(aid)
            .execute(&mut *tx)
            .await?;
    }

    // Rebuild genre links
    sqlx::query("DELETE FROM track_genres WHERE track_id = $1")
        .bind(track_id)
        .execute(&mut *tx)
        .await?;
    for name in &tag.genres {
        if name.trim().is_empty() {
            continue;
        }
        let genre_id = upsert_genre_in_tx(&mut *tx, name).await?;
        sqlx::query(
            "INSERT INTO track_genres (track_id, genre_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(track_id)
        .bind(&genre_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Update the library row for `file_path` with an edited tag set. Returns
/// `Ok(false)` when the file is not part of the library (the caller should
/// treat that as a silent no-op).
pub async fn sync_track_tags_by_file_path(
    pool: &PgPool,
    file_path: &str,
    tag: &AudioTag,
    clear_cover: bool,
) -> Result<bool> {
    let track_id: Option<String> =
        sqlx::query_scalar("SELECT id FROM tracks WHERE file_path = $1 LIMIT 1")
            .bind(file_path)
            .fetch_optional(pool)
            .await?;
    match track_id {
        Some(id) => {
            update_track_tags_from_audio_tag(pool, &id, tag, clear_cover).await?;
            Ok(true)
        }
        None => Ok(false),
    }
}

async fn upsert_album_from_tag_in_tx(
    conn: &mut sqlx::PgConnection,
    tag: &AudioTag,
    album_artist_id: &str,
) -> Result<String> {
    // Multi-value release types are stored semicolon-separated (see scan.rs).
    let release_type = (!tag.release_type.is_empty()).then(|| tag.release_type.join(";"));

    let existing: Option<String> =
        sqlx::query_scalar("SELECT id FROM albums WHERE title = $1 AND artist_id = $2")
            .bind(&tag.album)
            .bind(album_artist_id)
            .fetch_optional(&mut *conn)
            .await?;

    if let Some(id) = existing {
        sqlx::query(
            "UPDATE albums SET date = $1, release_mbid = $2, release_group_mbid = $3, release_artist_mbid = $4, release_country = $5, release_type = $6, record_label = $7, media = $8, original_date = $9, total_tracks = $10, total_discs = $11, updated_at = NOW() WHERE id = $12",
        )
        .bind(tag.release_date.as_deref())
        .bind(tag.mbid_release.as_deref())
        .bind(tag.mbid_release_group.as_deref())
        .bind(tag.mbid_release_artist.as_deref())
        .bind(tag.release_country.as_deref())
        .bind(release_type.as_deref())
        .bind(tag.record_label.as_deref())
        .bind(tag.media.as_deref())
        .bind(tag.original_date.as_deref())
        .bind(tag.total_tracks)
        .bind(tag.total_discs)
        .bind(&id)
        .execute(&mut *conn)
        .await?;
        Ok(id)
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO albums (id, title, artist_id, date, release_mbid, release_group_mbid, release_artist_mbid, release_country, release_type, record_label, media, original_date, total_tracks, total_discs) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        )
        .bind(&id)
        .bind(&tag.album)
        .bind(album_artist_id)
        .bind(tag.release_date.as_deref())
        .bind(tag.mbid_release.as_deref())
        .bind(tag.mbid_release_group.as_deref())
        .bind(tag.mbid_release_artist.as_deref())
        .bind(tag.release_country.as_deref())
        .bind(release_type.as_deref())
        .bind(tag.record_label.as_deref())
        .bind(tag.media.as_deref())
        .bind(tag.original_date.as_deref())
        .bind(tag.total_tracks)
        .bind(tag.total_discs)
        .execute(&mut *conn)
        .await?;
        Ok(id)
    }
}

async fn upsert_artist_with_sort_in_tx(
    conn: &mut sqlx::PgConnection,
    name: &str,
    sort_name: Option<&str>,
    mbid: Option<&str>,
) -> Result<String> {
    let existing: Option<String> = sqlx::query_scalar("SELECT id FROM artists WHERE name = $1")
        .bind(name)
        .fetch_optional(&mut *conn)
        .await?;

    if let Some(id) = existing {
        sqlx::query("UPDATE artists SET mbid = COALESCE($1, mbid), sort_name = COALESCE($2, sort_name), updated_at = NOW() WHERE id = $3")
            .bind(mbid)
            .bind(sort_name)
            .bind(&id)
            .execute(&mut *conn)
            .await?;
        Ok(id)
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        let sort_name = sort_name
            .map(str::to_string)
            .unwrap_or_else(|| derive_sort_name(name));
        sqlx::query("INSERT INTO artists (id, name, sort_name, mbid) VALUES ($1, $2, $3, $4)")
            .bind(&id)
            .bind(name)
            .bind(&sort_name)
            .bind(mbid)
            .execute(&mut *conn)
            .await?;
        Ok(id)
    }
}

async fn upsert_genre_in_tx(conn: &mut sqlx::PgConnection, name: &str) -> Result<String> {
    let existing: Option<String> = sqlx::query_scalar("SELECT id FROM genres WHERE name = $1")
        .bind(name)
        .fetch_optional(&mut *conn)
        .await?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO genres (id, name) VALUES ($1, $2)")
        .bind(&id)
        .bind(name)
        .execute(&mut *conn)
        .await?;
    Ok(id)
}

pub async fn find_or_create_collection(
    pool: &PgPool,
    name: &str,
    description: &str,
    user_id: &str,
) -> Result<String> {
    let existing: Option<String> = sqlx::query_scalar("SELECT id FROM collections WHERE name = $1")
        .bind(name)
        .fetch_optional(pool)
        .await?;

    if let Some(id) = existing {
        return Ok(id);
    }

    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO collections (id, name, description, user_id) VALUES ($1, $2, $3, $4)")
        .bind(&id)
        .bind(name)
        .bind(description)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(id)
}
