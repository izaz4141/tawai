use crate::db::database::DatabasePool;
use crate::signals::library::*;

/// Track-level tag metadata written alongside the positional insert columns.
/// `mbid_recording` lives here rather than as a positional argument so the
/// ever-growing tag set doesn't turn `insert_track` into an unreadable list.
#[derive(Debug, Clone, Default)]
pub struct TrackTagMeta {
    pub mbid_recording: Option<String>,
    pub mbid_track: Option<String>,
    pub isrc: Option<String>,
    pub comment: Option<String>,
    pub copyright: Option<String>,
    pub encoder: Option<String>,
    pub script: Option<String>,
}

/// Album-level metadata for `insert_album`. Absorbs the former positional
/// `date`/`mbid`/`disambiguation`/`total_discs` arguments plus the extended
/// release tag set.
#[derive(Debug, Clone, Default)]
pub struct AlbumWriteMeta {
    pub date: Option<String>,
    pub release_mbid: Option<String>,
    pub release_group_mbid: Option<String>,
    pub release_artist_mbid: Option<String>,
    pub release_country: Option<String>,
    /// Semicolon-separated multi-value.
    pub release_type: Option<String>,
    pub record_label: Option<String>,
    pub media: Option<String>,
    pub original_date: Option<String>,
    pub disambiguation: Option<String>,
    pub total_tracks: i32,
    pub total_discs: i32,
}

pub async fn lookup_track(
    pool: &DatabasePool,
    track_id: &str,
) -> anyhow::Result<Option<TrackInfo>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::lookup_track(p, track_id).await,
        DatabasePool::Postgres(p) => super::library_pg::lookup_track(p, track_id).await,
    }
}

pub async fn lookup_track_by_file_path(
    pool: &DatabasePool,
    file_path: &str,
) -> anyhow::Result<Option<TrackInfo>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::lookup_track_by_file_path(p, file_path).await,
        DatabasePool::Postgres(p) => {
            super::library_pg::lookup_track_by_file_path(p, file_path).await
        }
    }
}

pub async fn insert_artist(
    pool: &DatabasePool,
    name: &str,
    sort_name: &str,
    mbid: Option<String>,
) -> anyhow::Result<String> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::insert_artist(p, name, sort_name, mbid).await,
        DatabasePool::Postgres(p) => {
            super::library_pg::insert_artist(p, name, sort_name, mbid).await
        }
    }
}

pub async fn insert_album(
    pool: &DatabasePool,
    title: &str,
    artist_id: &str,
    cover: Option<&[u8]>,
    meta: &AlbumWriteMeta,
) -> anyhow::Result<String> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::insert_album(p, title, artist_id, cover, meta).await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::insert_album(p, title, artist_id, cover, meta).await
        }
    }
}

pub async fn insert_genre(pool: &DatabasePool, name: &str) -> anyhow::Result<String> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::insert_genre(p, name).await,
        DatabasePool::Postgres(p) => super::library_pg::insert_genre(p, name).await,
    }
}

pub async fn insert_track_genre(
    pool: &DatabasePool,
    track_id: &str,
    genre_id: &str,
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::insert_track_genre(p, track_id, genre_id).await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::insert_track_genre(p, track_id, genre_id).await
        }
    }
}

pub async fn update_album_cover(
    pool: &DatabasePool,
    album_id: &str,
    cover: &[u8],
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::update_album_cover(p, album_id, cover).await,
        DatabasePool::Postgres(p) => {
            super::library_pg::update_album_cover(p, album_id, cover).await
        }
    }
}

pub async fn insert_track(
    pool: &DatabasePool,
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
) -> anyhow::Result<String> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::insert_track(
                p,
                title,
                album_id,
                artist_id,
                disc_num,
                track_num,
                duration_secs,
                file_path,
                file_hash,
                source_id,
                file_size,
                bitrate,
                sample_rate,
                cover,
                lyrics,
                track_gain,
                track_peak,
                tags,
            )
            .await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::insert_track(
                p,
                title,
                album_id,
                artist_id,
                disc_num,
                track_num,
                duration_secs,
                file_path,
                file_hash,
                source_id,
                file_size,
                bitrate,
                sample_rate,
                cover,
                lyrics,
                track_gain,
                track_peak,
                tags,
            )
            .await
        }
    }
}

pub async fn track_exists_by_path(pool: &DatabasePool, file_path: &str) -> anyhow::Result<bool> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::track_exists_by_path(p, file_path).await,
        DatabasePool::Postgres(p) => super::library_pg::track_exists_by_path(p, file_path).await,
    }
}

pub async fn track_exists_by_fingerprint(
    pool: &DatabasePool,
    fingerprint: &str,
) -> anyhow::Result<Option<String>> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::track_exists_by_fingerprint(p, fingerprint).await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::track_exists_by_fingerprint(p, fingerprint).await
        }
    }
}

pub async fn fingerprint_paths_of(
    pool: &DatabasePool,
    paths: &std::collections::HashSet<String>,
) -> anyhow::Result<std::collections::HashMap<String, String>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::fingerprint_paths_of(p, paths).await,
        DatabasePool::Postgres(p) => super::library_pg::fingerprint_paths_of(p, paths).await,
    }
}

pub async fn insert_fingerprint(
    pool: &DatabasePool,
    track_id: &str,
    fingerprint: &str,
    acoust_id: Option<&str>,
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::insert_fingerprint(p, track_id, fingerprint, acoust_id).await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::insert_fingerprint(p, track_id, fingerprint, acoust_id).await
        }
    }
}

pub async fn all_track_file_paths(pool: &DatabasePool) -> anyhow::Result<Vec<String>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::all_track_file_paths(p).await,
        DatabasePool::Postgres(p) => super::library_pg::all_track_file_paths(p).await,
    }
}

pub async fn delete_track_by_file_path(pool: &DatabasePool, file_path: &str) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::delete_track_by_file_path(p, file_path).await,
        DatabasePool::Postgres(p) => {
            super::library_pg::delete_track_by_file_path(p, file_path).await
        }
    }
}

pub async fn delete_all_library(pool: &DatabasePool) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::delete_all_library(p).await,
        DatabasePool::Postgres(p) => super::library_pg::delete_all_library(p).await,
    }
}

pub async fn list_tracks(
    pool: &DatabasePool,
    album_id: Option<&str>,
) -> anyhow::Result<Vec<TrackInfo>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::list_tracks(p, album_id).await,
        DatabasePool::Postgres(p) => super::library_pg::list_tracks(p, album_id).await,
    }
}

pub async fn list_albums(
    pool: &DatabasePool,
    artist_id: Option<&str>,
) -> anyhow::Result<Vec<AlbumInfo>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::list_albums(p, artist_id).await,
        DatabasePool::Postgres(p) => super::library_pg::list_albums(p, artist_id).await,
    }
}

pub async fn get_album_cover(
    pool: &DatabasePool,
    album_id: &str,
) -> anyhow::Result<Option<Vec<u8>>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::get_album_cover(p, album_id).await,
        DatabasePool::Postgres(p) => super::library_pg::get_album_cover(p, album_id).await,
    }
}

pub async fn get_track_cover(
    pool: &DatabasePool,
    track_id: &str,
) -> anyhow::Result<Option<Vec<u8>>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::get_track_cover(p, track_id).await,
        DatabasePool::Postgres(p) => super::library_pg::get_track_cover(p, track_id).await,
    }
}

pub async fn list_artists(pool: &DatabasePool) -> anyhow::Result<Vec<ArtistInfo>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::list_artists(p).await,
        DatabasePool::Postgres(p) => super::library_pg::list_artists(p).await,
    }
}

pub async fn list_playlists(pool: &DatabasePool) -> anyhow::Result<Vec<PlaylistInfo>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::list_playlists(p).await,
        DatabasePool::Postgres(p) => super::library_pg::list_playlists(p).await,
    }
}

pub async fn create_playlist(
    pool: &DatabasePool,
    user_id: &str,
    name: &str,
) -> anyhow::Result<String> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::create_playlist(p, user_id, name).await,
        DatabasePool::Postgres(p) => super::library_pg::create_playlist(p, user_id, name).await,
    }
}

pub async fn delete_playlist(pool: &DatabasePool, playlist_id: &str) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::delete_playlist(p, playlist_id).await,
        DatabasePool::Postgres(p) => super::library_pg::delete_playlist(p, playlist_id).await,
    }
}

pub async fn get_playlist_tracks(
    pool: &DatabasePool,
    playlist_id: &str,
) -> anyhow::Result<Vec<TrackInfo>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::get_playlist_tracks(p, playlist_id).await,
        DatabasePool::Postgres(p) => super::library_pg::get_playlist_tracks(p, playlist_id).await,
    }
}

pub async fn add_track_to_playlist(
    pool: &DatabasePool,
    playlist_id: &str,
    track_id: &str,
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::add_track_to_playlist(p, playlist_id, track_id).await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::add_track_to_playlist(p, playlist_id, track_id).await
        }
    }
}

pub async fn remove_track_from_playlist(
    pool: &DatabasePool,
    playlist_id: &str,
    track_id: &str,
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::remove_track_from_playlist(p, playlist_id, track_id).await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::remove_track_from_playlist(p, playlist_id, track_id).await
        }
    }
}

pub async fn list_tracks_by_source(
    pool: &DatabasePool,
    source_id: &str,
) -> anyhow::Result<Vec<TrackInfo>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::list_tracks_by_source(p, source_id).await,
        DatabasePool::Postgres(p) => super::library_pg::list_tracks_by_source(p, source_id).await,
    }
}

pub async fn get_album_mbid(pool: &DatabasePool, album_id: &str) -> anyhow::Result<Option<String>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::get_album_mbid(p, album_id).await,
        DatabasePool::Postgres(p) => super::library_pg::get_album_mbid(p, album_id).await,
    }
}

pub async fn list_unidentified_tracks(
    pool: &DatabasePool,
    source_id: Option<&str>,
) -> anyhow::Result<Vec<TrackInfo>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::list_unidentified_tracks(p, source_id).await,
        DatabasePool::Postgres(p) => {
            super::library_pg::list_unidentified_tracks(p, source_id).await
        }
    }
}

pub async fn lookup_fingerprint_by_id(
    pool: &DatabasePool,
    track_id: &str,
) -> anyhow::Result<Option<(String, f64)>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::lookup_fingerprint_by_id(p, track_id).await,
        DatabasePool::Postgres(p) => super::library_pg::lookup_fingerprint_by_id(p, track_id).await,
    }
}

pub async fn update_track(
    pool: &DatabasePool,
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
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::update_track(
                p,
                track_id,
                title,
                artists,
                album_title,
                album_artists,
                album_meta,
                track_meta,
                track_num,
                disc_num,
                lyrics,
                cover,
                new_file_path,
                genres,
            )
            .await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::update_track(
                p,
                track_id,
                title,
                artists,
                album_title,
                album_artists,
                album_meta,
                track_meta,
                track_num,
                disc_num,
                lyrics,
                cover,
                new_file_path,
                genres,
            )
            .await
        }
    }
}

/// Persist a track's new on-disk location after a rename/move. Only the
/// `file_path` column is touched.
pub async fn set_track_file_path(
    pool: &DatabasePool,
    track_id: &str,
    new_file_path: &str,
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::set_track_file_path(p, track_id, new_file_path).await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::set_track_file_path(p, track_id, new_file_path).await
        }
    }
}

/// Refresh a track's stored SHA-256 after its on-disk bytes changed (e.g. a
/// tag write reused the file without a rescan). Matches on `file_path` because
/// the tag-edit flows only know the path, not the track id.
pub async fn update_file_hash_by_path(
    pool: &DatabasePool,
    file_path: &str,
    file_hash: &str,
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::update_file_hash_by_path(p, file_path, file_hash).await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::update_file_hash_by_path(p, file_path, file_hash).await
        }
    }
}

/// Apply a full tag-editor edit to an existing library track row.
pub async fn update_track_tags_from_audio_tag(
    pool: &DatabasePool,
    track_id: &str,
    tag: &crate::audio::tags::AudioTag,
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::update_track_tags_from_audio_tag(p, track_id, tag).await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::update_track_tags_from_audio_tag(p, track_id, tag).await
        }
    }
}

/// Sync an edited tag set to the library row matching `file_path`. Returns
/// `Ok(false)` when no library row matches (the caller should skip silently).
pub async fn sync_track_tags_by_file_path(
    pool: &DatabasePool,
    file_path: &str,
    tag: &crate::audio::tags::AudioTag,
) -> anyhow::Result<bool> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::sync_track_tags_by_file_path(p, file_path, tag).await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::sync_track_tags_by_file_path(p, file_path, tag).await
        }
    }
}

pub async fn delete_tracks_by_source_id(
    pool: &DatabasePool,
    source_id: &str,
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::delete_tracks_by_source_id(p, source_id).await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::delete_tracks_by_source_id(p, source_id).await
        }
    }
}

/// Deletes a single track after verifying the requesting user can access its
/// source. Physical deletion happens on the source itself , then the
/// library row (and its related rows) are removed.
pub async fn delete_track(
    pool: &DatabasePool,
    client: &reqwest::Client,
    user_id: &str,
    track_id: &str,
    master_key: &str,
) -> anyhow::Result<()> {
    let source = super::library_source::get_source_info_by_track_id(pool, track_id, master_key)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Track {} not found or has no source", track_id))?;

    let role = super::account::get_user_role(pool, user_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("User {} not found", user_id))?;

    if !super::library_source::can_access_source(
        &source.owner_id,
        user_id,
        &role,
        &source.access_rule,
    ) {
        anyhow::bail!(
            "User {} does not have access to track {}",
            user_id,
            track_id
        );
    }

    let track = lookup_track(pool, track_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Track {} not found", track_id))?;

    if let Some(parser) = crate::libsources::get_parser(&source.source_type, client.clone(), pool) {
        let mut resolver = crate::libsources::SourceUrlResolver::new();
        if let Ok(url) = resolver
            .resolve(&source.urls, Some(client), Some(&track.file_path))
            .await
        {
            parser
                .delete(pool, &track.file_path, &url, &source.urls, true)
                .await?;
        }
    }

    delete_track_by_file_path(pool, &track.file_path).await?;
    Ok(())
}

pub async fn insert_track_artists(
    pool: &DatabasePool,
    track_id: &str,
    artist_ids: &[String],
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::insert_track_artists(p, track_id, artist_ids).await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::insert_track_artists(p, track_id, artist_ids).await
        }
    }
}

pub async fn insert_album_artists(
    pool: &DatabasePool,
    album_id: &str,
    artist_ids: &[String],
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::insert_album_artists(p, album_id, artist_ids).await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::insert_album_artists(p, album_id, artist_ids).await
        }
    }
}

pub async fn reorder_playlist_tracks(
    pool: &DatabasePool,
    playlist_id: &str,
    track_ids: &[String],
) -> anyhow::Result<()> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::reorder_playlist_tracks(p, playlist_id, track_ids).await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::reorder_playlist_tracks(p, playlist_id, track_ids).await
        }
    }
}

pub async fn find_or_create_collection(
    pool: &DatabasePool,
    name: &str,
    description: &str,
    user_id: &str,
) -> anyhow::Result<String> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::find_or_create_collection(p, name, description, user_id).await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::find_or_create_collection(p, name, description, user_id).await
        }
    }
}
