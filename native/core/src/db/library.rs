use crate::db::database::DatabasePool;
use crate::signals::library::*;

pub async fn lookup_track(
    pool: &DatabasePool,
    track_id: &str,
) -> anyhow::Result<Option<TrackInfo>> {
    match pool {
        DatabasePool::Sqlite(p) => super::library_sq::lookup_track(p, track_id).await,
        DatabasePool::Postgres(p) => super::library_pg::lookup_track(p, track_id).await,
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
    date: Option<String>,
    mbid: Option<String>,
    cover: Option<&[u8]>,
    disambiguation: Option<String>,
    total_discs: i32,
) -> anyhow::Result<String> {
    match pool {
        DatabasePool::Sqlite(p) => {
            super::library_sq::insert_album(
                p,
                title,
                artist_id,
                date,
                mbid,
                cover,
                disambiguation,
                total_discs,
            )
            .await
        }
        DatabasePool::Postgres(p) => {
            super::library_pg::insert_album(
                p,
                title,
                artist_id,
                date,
                mbid,
                cover,
                disambiguation,
                total_discs,
            )
            .await
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
    mbid_recording: Option<String>,
    cover: Option<&[u8]>,
    lyrics: Option<&str>,
    track_gain: Option<f64>,
    track_peak: Option<f64>,
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
                mbid_recording,
                cover,
                lyrics,
                track_gain,
                track_peak,
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
                mbid_recording,
                cover,
                lyrics,
                track_gain,
                track_peak,
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
                album_mbid,
                date,
                track_num,
                disc_num,
                mbid_recording,
                lyrics,
                cover,
                new_file_path,
                album_disambiguation,
                total_discs,
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
                album_mbid,
                date,
                track_num,
                disc_num,
                mbid_recording,
                lyrics,
                cover,
                new_file_path,
                album_disambiguation,
                total_discs,
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
