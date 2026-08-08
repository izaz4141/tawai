use std::collections::{HashMap, HashSet};

use tokio::time::{Duration, sleep};

use crate::db::{database::DatabasePool, history, library, library_source};
use crate::discovery::listenbrainz;
use crate::metadata::musicbrainz;
use crate::signals::library::LibrarySourceInfo;
use crate::signals::metadata::RecordingInfo;
use crate::tools::duplicates;

const MAX_TRACKS: usize = 25;

pub struct SyncRecsParams<'a> {
    pub pool: &'a DatabasePool,
    pub client: &'a reqwest::Client,
    pub master_key: &'a str,
    pub user_id: &'a str,
    pub included_keys: &'a str,
}

#[derive(Debug, Clone, Default)]
pub struct SyncRecsResult {
    pub success: bool,
    pub added_sources: Vec<String>,
    pub removed_sources: Vec<String>,
    pub tracks_added: u32,
    pub tracks_removed: u32,
    pub error: Option<String>,
}

use crate::libsources::{ApiType, RecommendationSource};

async fn fetch_recording_mbids(
    client: &reqwest::Client,
    token: &str,
    user_name: &str,
    source: &RecommendationSource,
) -> Result<Vec<RecordingInfo>, String> {
    match source.api_type {
        ApiType::Recommendations => listenbrainz::fetch_recommendations(
            client,
            token,
            user_name,
            source.api_rec_type,
            Some(MAX_TRACKS as i32),
            None,
        )
        .await
        .map_err(|e| e.to_string()),
        ApiType::CreatedFor => {
            listenbrainz::fetch_createdfor(client, token, user_name, source.created_for_filter(), 0)
                .await
                .map(|cr| {
                    let _count = cr.recordings.len() as u32;
                    cr.recordings
                })
                .map_err(|e| e.to_string())
        }
    }
}

async fn fetch_recording_mb_data(client: &reqwest::Client, mbid: &str) -> Option<RecordingInfo> {
    for attempt in 0..3 {
        if attempt > 0 {
            sleep(Duration::from_millis(500)).await;
        }
        match musicbrainz::fetch_recording(client, mbid).await {
            Ok(e) => return Some(e),
            Err(e) => {
                crate::utils::logger::debug(&format!(
                    "attempt {} failed for {}: {}",
                    attempt + 1,
                    mbid,
                    e
                ));
            }
        }
    }
    None
}

async fn dedup_or_insert_track(
    pool: &DatabasePool,
    rec: &RecordingInfo,
    enriched: &RecordingInfo,
    source_id: &str,
) -> Option<String> {
    let mbid = &rec.id;
    let file_path = format!("recommendation://{}/{}", source_id, mbid);

    let artist_mbid = enriched.artist_id.clone();
    let artist_name = if enriched.artist.is_empty() {
        "Unknown Artist"
    } else {
        &enriched.artist
    };
    let artist_id = match library::insert_artist(pool, artist_name, artist_name, artist_mbid).await
    {
        Ok(id) => id,
        Err(e) => {
            crate::utils::logger::error(&format!("failed to insert artist: {}", e));
            return None;
        }
    };

    let release = enriched.releases.first();
    let album_title = release.map(|r| r.title.as_str()).unwrap_or("Unknown Album");
    let album_mbid = release.map(|r| r.id.clone());
    let release_date = release.and_then(|r| r.date.clone());
    let disambiguation = release.and_then(|r| r.disambiguation.clone());
    let total_discs = release.and_then(|r| r.total_discs).unwrap_or(0);

    let album_id = match library::insert_album(
        pool,
        album_title,
        &artist_id,
        release_date,
        album_mbid,
        None,
        disambiguation,
        total_discs,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            crate::utils::logger::error(&format!("failed to insert album: {}", e));
            return None;
        }
    };

    library::insert_album_artists(pool, &album_id, &[artist_id.clone()])
        .await
        .ok();

    let cover: Option<Vec<u8>> = match &enriched.cover {
        Some(url) => match reqwest::get(url).await {
            Ok(resp) if resp.status().is_success() => resp.bytes().await.ok().map(|b| b.to_vec()),
            _ => None,
        },
        None => None,
    };
    let track_title = &enriched.title;
    let duration = enriched.duration_secs.unwrap_or(0.0);

    let track_id = match library::insert_track(
        pool,
        track_title,
        &album_id,
        &artist_id,
        1,
        1,
        duration,
        &file_path,
        None,
        source_id,
        None,
        None,
        None,
        Some(mbid.clone()),
        cover.as_deref(),
        None,
        None,
        None,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            crate::utils::logger::error(&format!("failed to insert track: {}", e));
            return None;
        }
    };

    library::insert_track_artists(pool, &track_id, &[artist_id])
        .await
        .ok();

    Some(track_id)
}

async fn sync_one_source(
    pool: &DatabasePool,
    client: &reqwest::Client,
    token: &str,
    user_name: &str,
    source: &RecommendationSource,
    source_id: &str,
    discovery_collection_id: &str,
    source_collection_id: &str,
) -> Result<(u32, u32), String> {
    let recs = fetch_recording_mbids(client, token, user_name, source).await?;

    let fresh_mbids: HashSet<&str> = recs.iter().map(|r| r.id.as_str()).collect();

    let existing_tracks = library::list_tracks_by_source(pool, source_id)
        .await
        .map_err(|e| e.to_string())?;

    let existing_mbids: HashSet<&str> = existing_tracks
        .iter()
        .filter_map(|t| t.mbid_recording.as_deref())
        .collect();

    if existing_mbids == fresh_mbids {
        return Ok((0, 0));
    }

    let removed_count = existing_mbids.len() as u32;

    if let Err(e) = library::delete_tracks_by_source_id(pool, source_id).await {
        crate::utils::logger::error(&format!("failed to delete old rec tracks: {}", e));
    }

    // Phase 1: Check which tracks already exist in DB (dedup) and collect existing IDs
    let mut to_insert: Vec<&RecordingInfo> = Vec::new();
    let mut existing_ids: Vec<String> = Vec::new();
    for rec in &recs {
        let artist_name = if rec.artist.is_empty() {
            "Unknown Artist"
        } else {
            &rec.artist
        };
        match duplicates::find_track_by_recording(pool, Some(&rec.id), &rec.title, artist_name)
            .await
        {
            Ok(Some(track_id)) => existing_ids.push(track_id),
            _ => to_insert.push(rec),
        }
    }

    // Phase 2+3: Fetch MusicBrainz data sequentially (1/request per second) then insert
    let mut added = 0u32;
    for rec in &to_insert {
        let Some(enriched) = fetch_recording_mb_data(client, &rec.id).await else {
            continue;
        };
        if let Some(track_id) = dedup_or_insert_track(pool, rec, &enriched, source_id).await {
            library::add_track_to_playlist(pool, discovery_collection_id, &track_id)
                .await
                .ok();
            library::add_track_to_playlist(pool, source_collection_id, &track_id)
                .await
                .ok();
            added += 1;
        }
        sleep(Duration::from_millis(1100)).await;
    }

    for track_id in &existing_ids {
        library::add_track_to_playlist(pool, discovery_collection_id, track_id)
            .await
            .ok();
        library::add_track_to_playlist(pool, source_collection_id, track_id)
            .await
            .ok();
        added += 1;
    }

    library_source::touch_source_sync_at(pool, source_id)
        .await
        .ok();

    Ok((added, removed_count))
}

pub async fn sync_recs(params: SyncRecsParams<'_>) -> SyncRecsResult {
    let active_keys: HashSet<String> = params
        .included_keys
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let all_sources = match library_source::list_all_sources(params.pool).await {
        Ok(s) => s,
        Err(e) => {
            return SyncRecsResult {
                error: Some(format!("failed to list sources: {}", e)),
                ..Default::default()
            };
        }
    };

    let existing_sources: HashMap<String, LibrarySourceInfo> = all_sources
        .into_iter()
        .filter(|s| s.owner_id == params.user_id && s.source_type.starts_with("recommendation:"))
        .map(|s| (s.source_type.clone(), s))
        .collect();

    let mut result = SyncRecsResult::default();

    for source_type in existing_sources.keys() {
        if let Some(source) = existing_sources.get(source_type) {
            if !active_keys.contains(source_type) {
                if let Err(e) = library_source::remove_source(params.pool, &source.id).await {
                    crate::utils::logger::error(&format!(
                        "failed to remove source {}: {}",
                        source_type, e
                    ));
                } else {
                    result.removed_sources.push(source.name.clone());
                }
            }
        }
    }

    if active_keys.is_empty() {
        result.success = true;
        return result;
    }

    let token =
        match history::get_listenbrainz_token(params.pool, params.user_id, params.master_key).await
        {
            Some(t) => t,
            None => {
                return SyncRecsResult {
                    error: Some("ListenBrainz token not configured".to_string()),
                    ..Default::default()
                };
            }
        };

    let validated = match listenbrainz::validate_token(params.client, &token).await {
        Ok(v) if v.valid => v,
        _ => {
            return SyncRecsResult {
                error: Some("Invalid ListenBrainz token".to_string()),
                ..Default::default()
            };
        }
    };

    let user_name = validated.user_name.unwrap_or_default();

    let discovery_collection_id = match library::find_or_create_collection(
        params.pool,
        "Discovery",
        "All recommended tracks from discovery sources",
        params.user_id,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            return SyncRecsResult {
                error: Some(format!("failed to create Discovery collection: {}", e)),
                ..Default::default()
            };
        }
    };

    for active_key in &active_keys {
        let cat = match RecommendationSource::from_key(active_key) {
            Some(c) => c,
            None => continue,
        };
        let source_type = active_key.clone();
        let display_name = cat.display_name_with_user(&user_name);

        let source_collection_id = match library::find_or_create_collection(
            params.pool,
            &display_name,
            &format!("Personalized {} from ListenBrainz", display_name),
            params.user_id,
        )
        .await
        {
            Ok(id) => id,
            Err(e) => {
                crate::utils::logger::error(&format!(
                    "failed to create collection {}: {}",
                    display_name, e
                ));
                continue;
            }
        };

        if let Some(source) = existing_sources.get(&source_type) {
            if let Some(last_sync) = &source.last_sync_at {
                if let Ok(parsed) = time::OffsetDateTime::parse(
                    last_sync,
                    &time::format_description::well_known::Rfc3339,
                ) {
                    let elapsed = time::OffsetDateTime::now_utc() - parsed;
                    if elapsed.unsigned_abs() < cat.refresh_interval {
                        continue;
                    }
                }
            }
            let source_id = source.id.clone();
            match sync_one_source(
                params.pool,
                params.client,
                &token,
                &user_name,
                cat,
                &source_id,
                &discovery_collection_id,
                &source_collection_id,
            )
            .await
            {
                Ok((added, removed)) => {
                    result.tracks_added += added;
                    result.tracks_removed += removed;
                }
                Err(e) => {
                    crate::utils::logger::error(&format!("sync failed for {}: {}", source_type, e));
                }
            }
        } else {
            let url = cat.source_url();

            let source_id = match library_source::upsert_source(
                params.pool,
                &source_type,
                &url,
                &display_name,
                params.user_id,
            )
            .await
            {
                Ok(id) => id,
                Err(e) => {
                    crate::utils::logger::error(&format!(
                        "failed to create source {}: {}",
                        source_type, e
                    ));
                    continue;
                }
            };

            match sync_one_source(
                params.pool,
                params.client,
                &token,
                &user_name,
                cat,
                &source_id,
                &discovery_collection_id,
                &source_collection_id,
            )
            .await
            {
                Ok((added, removed)) => {
                    result.tracks_added += added;
                    result.tracks_removed += removed;
                    result.added_sources.push(display_name);
                }
                Err(e) => {
                    crate::utils::logger::error(&format!("sync failed for {}: {}", source_type, e));
                }
            }
        }
    }

    result.success = true;
    result
}
