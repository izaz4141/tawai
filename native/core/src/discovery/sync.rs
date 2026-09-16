use std::collections::{HashMap, HashSet};

use crate::db::{database::DatabasePool, history, library, library_source};
use crate::discovery::listenbrainz;
use crate::libsources::{get_parser, RecommendationSource};
use crate::signals::library::LibrarySourceInfo;

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

async fn sync_one_source(
    pool: &DatabasePool,
    client: &reqwest::Client,
    token: &str,
    user_name: &str,
    source_type: &str,
    source_id: &str,
    discovery_collection_id: &str,
    source_collection_id: &str,
) -> Result<(u32, u32), String> {
    let parser = get_parser(source_type, client.clone(), pool)
        .ok_or_else(|| "no parser for source".to_string())?;

    let (added, removed) = parser
        .sync(pool, source_id, client, token, user_name)
        .await
        .map_err(|e| e.to_string())?;

    // Re-add synced tracks to the discovery + source collections.
    let tracks = library::list_tracks_by_source(pool, source_id)
        .await
        .map_err(|e| e.to_string())?;
    for track in &tracks {
        library::add_track_to_playlist(pool, discovery_collection_id, &track.id)
            .await
            .ok();
        library::add_track_to_playlist(pool, source_collection_id, &track.id)
            .await
            .ok();
    }

    Ok((added, removed))
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
                &source_type,
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
                &[url],
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
                &source_type,
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