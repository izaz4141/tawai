use anyhow::Result;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::metadata::musicbrainz;
use crate::signals::discovery::ValidateTokenResponse;
use crate::signals::library::TrackInfo;
use crate::signals::metadata::RecordingInfo;

#[derive(Serialize)]
struct Listen {
    track_metadata: TrackMetadata,
    listened_at: i64,
}

#[derive(Serialize)]
struct TrackMetadata {
    artist_name: String,
    track_name: String,
    release_name: Option<String>,
    additional_info: AdditionalInfo,
}

#[derive(Serialize)]
struct AdditionalInfo {
    submission_client: String,
    submission_client_version: String,
    musicbrainz_recording_mbid: Option<String>,
}

#[derive(Serialize)]
struct SubmitListensPayload {
    listen_type: String,
    payload: Vec<Listen>,
}

#[derive(Deserialize)]
struct LbRecording {
    recording_mbid: String,
    score: f64,
}

#[derive(Deserialize)]
struct LbPayload {
    mbids: Vec<LbRecording>,
}

#[derive(Deserialize)]
struct LbRecommendationsResponse {
    payload: LbPayload,
}

fn build_track_metadata(track: &TrackInfo) -> TrackMetadata {
    TrackMetadata {
        artist_name: track.artists_string.clone(),
        track_name: track.title.clone(),
        release_name: Some(track.album_title.clone()),
        additional_info: AdditionalInfo {
            submission_client: "tawai".to_string(),
            submission_client_version: "0.0.1".to_string(),
            musicbrainz_recording_mbid: track.mbid_recording.clone(),
        },
    }
}

async fn submit_listen(
    client: &reqwest::Client,
    token: &str,
    payload: &impl Serialize,
) -> Result<()> {
    client
        .post("https://api.listenbrainz.org/1/submit-listens")
        .header("Authorization", format!("Token {token}"))
        .json(payload)
        .send()
        .await?;
    Ok(())
}

pub async fn scrobble(client: &reqwest::Client, token: &str, track: &TrackInfo) -> Result<()> {
    let now = OffsetDateTime::now_utc();
    let payload = SubmitListensPayload {
        listen_type: "single".to_string(),
        payload: vec![Listen {
            listened_at: now.unix_timestamp(),
            track_metadata: build_track_metadata(track),
        }],
    };
    submit_listen(client, token, &payload).await
}

pub async fn update_now_playing(
    client: &reqwest::Client,
    token: &str,
    track: &TrackInfo,
) -> Result<()> {
    let payload = serde_json::json!({
        "listen_type": "playing_now",
        "payload": [{
            "track_metadata": build_track_metadata(track)
        }]
    });
    submit_listen(client, token, &payload).await
}

pub async fn validate_token(
    client: &reqwest::Client,
    token: &str,
) -> Result<ValidateTokenResponse> {
    let resp = client
        .get("https://api.listenbrainz.org/1/validate-token")
        .header("Authorization", format!("Token {token}"))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("failed to send validate-token request: {e}"))?;

    let body: ValidateTokenResponse = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("failed to parse validate-token response: {e}"))?;

    Ok(body)
}

async fn enrich_mbids(client: &reqwest::Client, mbids: Vec<(String, f64)>) -> Vec<RecordingInfo> {
    let results = futures::future::join_all(
        mbids
            .iter()
            .map(|(mbid, _)| musicbrainz::fetch_recording(client, mbid)),
    )
    .await;

    mbids
        .into_iter()
        .zip(results)
        .filter_map(|((_, score), res)| match res {
            Ok(mut info) => {
                info.score = score;
                Some(info)
            }
            Err(_) => None,
        })
        .collect()
}

pub async fn fetch_recommendations(
    client: &reqwest::Client,
    token: &str,
    user_name: &str,
    artist_type: &str,
    count: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<RecordingInfo>> {
    let mut url =
        format!("https://api.listenbrainz.org/1/cf/recommendation/user/{user_name}/recording");
    let mut params = vec![format!("artist_type={artist_type}")];
    if let Some(c) = count {
        params.push(format!("count={c}"));
    }
    if let Some(o) = offset {
        params.push(format!("offset={o}"));
    }
    url.push('?');
    url.push_str(&params.join("&"));

    let resp = client
        .get(&url)
        .header("Authorization", format!("Token {token}"))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("failed to send recommendations request: {e}"))?;

    if resp.status() == reqwest::StatusCode::NO_CONTENT {
        return Ok(Vec::new());
    }

    let body: LbRecommendationsResponse = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("failed to parse recommendations response: {e}"))?;

    let pairs: Vec<(String, f64)> = body
        .payload
        .mbids
        .into_iter()
        .map(|r| (r.recording_mbid, r.score))
        .collect();

    Ok(enrich_mbids(client, pairs).await)
}

// ---------------------------------------------------------------------------
// ListenBrainz Playlists (createdfor: weekly exploration, year in music, etc.)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct PlaylistIdentifiers {
    playlist: PlaylistWrapper,
}

#[derive(Deserialize)]
struct PlaylistWrapper {
    track: Vec<TrackIdentifier>,
}

#[derive(Deserialize)]
struct TrackIdentifier {
    identifier: Vec<String>,
}

#[derive(Deserialize)]
struct PlaylistJspf {
    playlist: PlaylistInfo,
}

#[derive(Deserialize)]
struct PlaylistInfo {
    identifier: String,
    title: String,
}

#[derive(Deserialize)]
struct CreatedForPlaylists {
    playlists: Vec<PlaylistJspf>,
    playlist_count: u32,
}

pub struct CreatedForResult {
    pub recordings: Vec<RecordingInfo>,
    pub playlist_title: String,
    pub playlist_id: String,
    pub playlist_count: u32,
}

async fn fetch_playlist_tracks(
    client: &reqwest::Client,
    token: &str,
    playlist_mbid: &str,
) -> Result<Vec<(String, f64)>> {
    let url = format!("https://api.listenbrainz.org/1/playlist/{playlist_mbid}");

    let resp = client
        .get(&url)
        .header("Authorization", format!("Token {token}"))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("failed to send playlist request: {e}"))?;

    let body: PlaylistIdentifiers = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("failed to parse playlist response: {e}"))?;

    let mbids: Vec<(String, f64)> = body
        .playlist
        .track
        .into_iter()
        .filter_map(|t| {
            let id_str = t.identifier.first()?;
            let mbid = id_str
                .strip_prefix("https://musicbrainz.org/recording/")
                .or_else(|| id_str.strip_prefix("https://listenbrainz.org/recording/"))
                .or_else(|| {
                    if id_str.len() == 36 && id_str.contains('-') {
                        Some(id_str.as_str())
                    } else {
                        None
                    }
                })
                .map(|s| s.to_string());
            mbid.map(|m| (m, 0.0))
        })
        .collect();

    Ok(mbids)
}

async fn fetch_createdfor_list(
    client: &reqwest::Client,
    token: &str,
    user_name: &str,
    count: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<PlaylistInfo>> {
    let mut url = format!("https://api.listenbrainz.org/1/user/{user_name}/playlists/createdfor");
    let mut params = vec![];
    if let Some(c) = count {
        params.push(format!("count={c}"));
    }
    if let Some(o) = offset {
        params.push(format!("offset={o}"));
    }
    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }

    let resp = client
        .get(&url)
        .header("Authorization", format!("Token {token}"))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("failed to send created-for playlists request: {e}"))?;

    let body: CreatedForPlaylists = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("failed to parse created-for playlists response: {e}"))?;

    Ok(body.playlists.into_iter().map(|p| p.playlist).collect())
}

pub async fn fetch_createdfor(
    client: &reqwest::Client,
    token: &str,
    user_name: &str,
    title_filter: &str,
    index: u32,
) -> Result<CreatedForResult> {
    let filter = title_filter.replace('-', " ");
    let playlists = fetch_createdfor_list(client, token, user_name, Some(100), None).await?;

    let matched: Vec<&PlaylistInfo> = playlists
        .iter()
        .filter(|p| p.title.to_lowercase().contains(&filter))
        .collect();

    let playlist_count = matched.len() as u32;
    let selected = matched.get(index as usize);

    let playlist_title = selected.map(|p| p.title.clone()).unwrap_or_default();
    let playlist_id = selected
        .and_then(|p| {
            Some(
                p.identifier
                    .strip_prefix("https://listenbrainz.org/playlist/")
                    .unwrap_or(&p.identifier)
                    .to_string(),
            )
        })
        .unwrap_or_default();

    match selected {
        Some(p) => {
            let mbid = p
                .identifier
                .strip_prefix("https://listenbrainz.org/playlist/")
                .unwrap_or(&p.identifier);
            let pairs = fetch_playlist_tracks(client, token, mbid).await?;
            let recordings = enrich_mbids(client, pairs).await;
            Ok(CreatedForResult {
                recordings,
                playlist_title,
                playlist_id,
                playlist_count,
            })
        }
        None => Ok(CreatedForResult {
            recordings: vec![],
            playlist_title,
            playlist_id,
            playlist_count,
        }),
    }
}
