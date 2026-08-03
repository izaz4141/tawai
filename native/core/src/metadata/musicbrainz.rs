use anyhow::Result;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::header;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::signals::metadata::{MBSearchInfo, RecordingInfo, ReleaseInfo, ReleaseTrackInfo};

static MB_UA: &str = "Tawai/0.0.1 ( https://github.com/izaz4141/tawai )";

const ACOUSTID_CLIENT_KEY: &str = "1WElrdlcln";

const MB_MAX_RETRIES: u32 = 3;

/// AcoustID usage guidelines require at most 3 requests per second.
/// Space each AcoustID call by this interval (~3/s).
const ACOUSTID_REQUEST_INTERVAL: Duration = Duration::from_millis(334);

struct AcoustIdRateLimiter {
    interval: Duration,
    next_slot: Instant,
}

static ACOUSTID_LIMITER: OnceLock<Mutex<AcoustIdRateLimiter>> = OnceLock::new();

async fn acoustid_throttle() {
    let limiter = ACOUSTID_LIMITER.get_or_init(|| {
        Mutex::new(AcoustIdRateLimiter {
            interval: ACOUSTID_REQUEST_INTERVAL,
            next_slot: Instant::now(),
        })
    });

    let mut guard = limiter.lock().await;
    let now = Instant::now();
    let interval = guard.interval;
    if now < guard.next_slot {
        tokio::time::sleep(guard.next_slot - now).await;
        guard.next_slot += interval;
    } else {
        guard.next_slot = now + interval;
    }
}

async fn mb_get_json<T: DeserializeOwned>(client: &reqwest::Client, url: &str) -> Result<T> {
    let mut last_err = None;
    for attempt in 0..MB_MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
        }

        let resp = match client
            .get(url)
            .header(header::USER_AGENT, MB_UA)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                last_err = Some(anyhow::anyhow!(
                    "MusicBrainz request failed for URL [{}]. Underlying cause: {}",
                    url,
                    e
                ));
                continue;
            }
        };

        let status = resp.status();
        if status.is_success() {
            match resp.json().await {
                Ok(body) => return Ok(body),
                Err(e) => {
                    last_err = Some(anyhow::anyhow!(
                        "Failed to parse MusicBrainz response for URL [{}]: {}",
                        url,
                        e
                    ));
                    continue;
                }
            }
        }

        let body_text = resp.text().await.unwrap_or_default();
        if status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            last_err = Some(anyhow::anyhow!(
                "MusicBrainz API returned {}: {}",
                status,
                body_text
            ));
            continue;
        }

        anyhow::bail!("MusicBrainz API returned {}: {}", status, body_text);
    }

    Err(last_err.unwrap_or_else(|| {
        anyhow::anyhow!(
            "MusicBrainz request failed after {} retries",
            MB_MAX_RETRIES
        )
    }))
}

async fn mb_post_form_json<T: DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    form: &[(&str, &str)],
) -> Result<T> {
    let mut last_err = None;
    for attempt in 0..MB_MAX_RETRIES {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
        }

        let resp = match client
            .post(url)
            .header(header::USER_AGENT, MB_UA)
            .form(form)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                last_err = Some(anyhow::anyhow!(
                    "AcoustID API request failed. Underlying cause: {}",
                    e
                ));
                continue;
            }
        };

        let status = resp.status();
        if status.is_success() {
            match resp.json().await {
                Ok(body) => return Ok(body),
                Err(e) => {
                    last_err = Some(anyhow::anyhow!("Failed to parse AcoustID response: {}", e));
                    continue;
                }
            }
        }

        let body_text = resp.text().await.unwrap_or_default();
        if status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            last_err = Some(anyhow::anyhow!(
                "AcoustID API returned {}: {}",
                status,
                body_text
            ));
            continue;
        }

        anyhow::bail!("AcoustID API returned {}: {}", status, body_text);
    }

    Err(last_err.unwrap_or_else(|| {
        anyhow::anyhow!(
            "AcoustID API request failed after {} retries",
            MB_MAX_RETRIES
        )
    }))
}

fn cover_url(release_id: &str) -> String {
    format!("https://coverartarchive.org/release/{release_id}/front-250.jpg")
}

async fn find_cover_url(client: &reqwest::Client, releases: &[ReleaseInfo]) -> Option<String> {
    for r in releases {
        let url = cover_url(&r.id);
        if let Ok(resp) = client
            .head(&url)
            .header(header::USER_AGENT, MB_UA)
            .send()
            .await
        {
            if resp.status().is_success() {
                return Some(url);
            }
        }
    }
    None
}

/// Format an AcoustID release date as `YYYY-MM-DD`, `YYYY-MM`, or `YYYY`,
/// matching the date convention used elsewhere (see audio::tags).
fn format_acoustid_date(date: AcoustIdDate) -> Option<String> {
    match (date.year, date.month, date.day) {
        (Some(y), Some(m), Some(d)) => Some(format!("{:04}-{:02}-{:02}", y, m, d)),
        (Some(y), Some(m), None) => Some(format!("{:04}-{:02}", y, m)),
        (Some(y), None, None) => Some(format!("{:04}", y)),
        _ => None,
    }
}

/// Look up a Chromaprint fingerprint via the AcoustID API.
/// Returns a RecordingInfo with the top recording, its artist, and releases.
pub async fn lookup_by_fingerprint(
    client: &reqwest::Client,
    fingerprint: &str,
    duration_secs: f64,
) -> Result<RecordingInfo> {
    let meta = "recordings releasegroups releases compress";
    let duration_str = (duration_secs.round() as i64).to_string();

    acoustid_throttle().await;

    let body: AcoustIdResponse = mb_post_form_json(
        client,
        "https://api.acoustid.org/v2/lookup",
        &[
            ("client", ACOUSTID_CLIENT_KEY),
            ("fingerprint", fingerprint),
            ("duration", &duration_str),
            ("meta", meta),
        ],
    )
    .await?;

    if body.status != "ok" {
        let msg = body
            .error
            .map(|e| e.message)
            .unwrap_or_else(|| "unknown error".to_string());
        anyhow::bail!("AcoustID API error: {msg}");
    }

    let top = match body.results {
        Some(mut results) => {
            results.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            results.into_iter().next()
        }
        None => anyhow::bail!("AcoustID returned no results"),
    };

    let result = top.ok_or_else(|| anyhow::anyhow!("AcoustID returned no results"))?;
    let acoust_id = Some(result.id);

    let recording = result
        .recordings
        .and_then(|r| r.into_iter().next())
        .ok_or_else(|| anyhow::anyhow!("AcoustID recording not found"))?;

    let id = recording.id.unwrap_or_default();
    let title = recording.title.unwrap_or_default();
    let artist = recording
        .artists
        .as_ref()
        .and_then(|a| a.first())
        .and_then(|a| a.name.clone())
        .unwrap_or_default();
    let artist_id = recording
        .artists
        .as_ref()
        .and_then(|a| a.first())
        .and_then(|a| a.id.clone());
    let duration = recording.duration.unwrap_or(duration_secs);

    let releases: Vec<_> = recording
        .releasegroups
        .unwrap_or_default()
        .into_iter()
        .flat_map(|rg| {
            let releasegroup_title = rg.title.clone();
            let artist = artist.clone();
            let artist_id = artist_id.clone();
            let releases = rg.releases.unwrap_or_default();
            releases.into_iter().map(move |r| ReleaseInfo {
                id: r.id.unwrap_or_default(),
                title: r
                    .title
                    .unwrap_or_else(|| releasegroup_title.clone().unwrap_or_default()),
                date: r.date.and_then(format_acoustid_date),
                country: r.country,
                artist: artist.clone(),
                artist_id: artist_id.clone(),
                tracks: Vec::new(),
                disambiguation: None,
                total_discs: r.medium_count,
                total_tracks: r.track_count,
            })
        })
        .collect();

    let cover = find_cover_url(client, &releases).await;

    Ok(RecordingInfo {
        id,
        title,
        score: 100.0,
        artist,
        artist_id,
        duration_secs: Some(duration),
        acoust_id,
        releases,
        cover,
    })
}

/// Search MusicBrainz by text query.
/// Returns MBSearchInfo with recordings, scores, and full release data.
pub async fn lookup_by_query(client: &reqwest::Client, query: &str) -> Result<MBSearchInfo> {
    let url = format!(
        "https://musicbrainz.org/ws/2/recording/?query={} AND video:false&fmt=json&limit=15",
        utf8_percent_encode(query, NON_ALPHANUMERIC)
    );

    let body: MusicBrainzSearchResponse = mb_get_json(client, &url).await?;

    let recordings = body.recordings.unwrap_or_default();
    let mut out = Vec::with_capacity(recordings.len());

    for rec in recordings {
        let artist = rec
            .artist_credit
            .as_ref()
            .filter(|c| !c.is_empty())
            .map(|credits| {
                let sep = credits[0].joinphrase.as_deref().unwrap_or("");
                let names: Vec<&str> = credits.iter().map(|c| c.artist.name.as_str()).collect();
                names.join(sep)
            })
            .unwrap_or("Unknown Artist".to_string());
        let artist_id = rec
            .artist_credit
            .as_ref()
            .and_then(|c| c.first())
            .map(|c| c.artist.id.clone());

        let releases: Vec<_> = rec
            .releases
            .unwrap_or_default()
            .into_iter()
            .map(|r| ReleaseInfo {
                id: r.id,
                title: r.title.unwrap_or_default(),
                date: r.date,
                country: r.country,
                artist: artist.clone(),
                artist_id: artist_id.clone(),
                tracks: Vec::new(),
                disambiguation: r.disambiguation,
                total_discs: None,
                total_tracks: None,
            })
            .collect();

        let cover = find_cover_url(client, &releases).await;

        out.push(RecordingInfo {
            id: rec.id,
            title: rec.title.unwrap_or_default(),
            score: rec.score.unwrap_or(0.0) / 100.0,
            artist,
            artist_id,
            duration_secs: rec.length.map(|l| l as f64 / 1000.0),
            acoust_id: None,
            releases,
            cover,
        });
    }

    Ok(MBSearchInfo { recordings: out })
}

/// Fetch a release by MusicBrainz ID, including its tracks.
pub async fn fetch_release(client: &reqwest::Client, release_id: &str) -> Result<ReleaseInfo> {
    let url = format!(
        "https://musicbrainz.org/ws/2/release/{release_id}?inc=recordings+artist-credits&fmt=json"
    );

    let release: MusicBrainzReleaseResponse = mb_get_json(client, &url).await?;

    let artist = release
        .artist_credit
        .as_ref()
        .filter(|c| !c.is_empty())
        .map(|credits| {
            let names: Vec<&str> = credits.iter().map(|c| c.artist.name.as_str()).collect();
            names.join("")
        })
        .unwrap_or_default();

    let artist_id = release
        .artist_credit
        .as_ref()
        .and_then(|c| c.first())
        .map(|c| c.artist.id.clone());

    let total_discs = release
        .media
        .as_ref()
        .and_then(|m| m.iter().filter_map(|med| med.position).max())
        .unwrap_or(0);

    let total_tracks: Option<i32> = release.media.as_ref().and_then(|m| {
        let counts: Vec<i32> = m.iter().filter_map(|med| med.track_count).collect();
        if counts.is_empty() {
            None
        } else {
            Some(counts.iter().sum())
        }
    });

    let tracks = release
        .media
        .unwrap_or_default()
        .into_iter()
        .flat_map(|m| {
            let disc = m.position;
            m.tracks
                .unwrap_or_default()
                .into_iter()
                .map(move |t| (disc, t))
        })
        .filter_map(|(disc, t)| {
            t.recording.map(|rec| ReleaseTrackInfo {
                id: rec.id,
                title: rec.title.unwrap_or(t.title.unwrap_or_default()),
                position: t.position,
                disc_number: disc,
                duration_secs: rec.length.map(|l| l as f64 / 1000.0),
                ..Default::default()
            })
        })
        .collect();

    Ok(ReleaseInfo {
        id: release.id,
        title: release.title.unwrap_or_default(),
        date: release.date,
        country: None,
        artist,
        artist_id,
        tracks,
        disambiguation: release.disambiguation,
        total_discs: Some(total_discs),
        total_tracks,
    })
}

/// Fetch a recording by MusicBrainz MBID along with its releases.
pub async fn fetch_recording(client: &reqwest::Client, mbid: &str) -> Result<RecordingInfo> {
    let url =
        format!("https://musicbrainz.org/ws/2/recording/{mbid}?inc=artists+releases&fmt=json");

    let recording: MusicBrainzRecording = mb_get_json(client, &url).await?;

    let id = recording.id;
    let title = recording.title.unwrap_or_default();
    let artist = recording
        .artist_credit
        .as_ref()
        .and_then(|c| c.first())
        .map(|c| c.artist.name.clone())
        .unwrap_or_default();
    let artist_id = recording
        .artist_credit
        .as_ref()
        .and_then(|c| c.first())
        .map(|c| c.artist.id.clone());
    let duration_secs = recording.length.map(|l| l as f64 / 1000.0);

    let releases: Vec<_> = recording
        .releases
        .unwrap_or_default()
        .into_iter()
        .map(|r| ReleaseInfo {
            id: r.id,
            title: r.title.unwrap_or_default(),
            date: r.date,
            country: r.country,
            artist: artist.clone(),
            artist_id: artist_id.clone(),
            tracks: Vec::new(),
            disambiguation: r.disambiguation,
            total_discs: None,
            total_tracks: None,
        })
        .collect();

    let cover = find_cover_url(client, &releases).await;

    Ok(RecordingInfo {
        id,
        title,
        score: 100.0,
        artist,
        artist_id,
        duration_secs,
        acoust_id: None,
        releases,
        cover,
    })
}

#[derive(Deserialize)]
pub struct AcoustIdResponse {
    pub status: String,
    pub results: Option<Vec<AcoustIdResult>>,
    pub error: Option<AcoustIdError>,
}

#[derive(Deserialize)]
pub struct AcoustIdError {
    pub message: String,
}

#[derive(Deserialize)]
pub struct AcoustIdResult {
    pub id: String,
    pub score: f64,
    pub recordings: Option<Vec<AcoustIdRecording>>,
}

#[derive(Deserialize)]
pub struct AcoustIdRecording {
    pub id: Option<String>,
    pub title: Option<String>,
    pub duration: Option<f64>,
    pub artists: Option<Vec<AcoustIdArtist>>,
    pub releasegroups: Option<Vec<AcoustIdReleaseGroup>>,
}

#[derive(Deserialize)]
pub struct AcoustIdArtist {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct AcoustIdReleaseGroup {
    pub id: Option<String>,
    pub title: Option<String>,
    pub releases: Option<Vec<AcoustIdRelease>>,
}

#[derive(Deserialize)]
pub struct AcoustIdRelease {
    pub id: Option<String>,
    pub title: Option<String>,
    pub date: Option<AcoustIdDate>,
    pub country: Option<String>,
    pub medium_count: Option<i32>,
    pub track_count: Option<i32>,
}

#[derive(Deserialize)]
pub struct AcoustIdDate {
    pub day: Option<i32>,
    pub month: Option<i32>,
    pub year: Option<i32>,
}

#[derive(Deserialize)]
pub struct MusicBrainzRecording {
    pub id: String,
    pub title: Option<String>,
    pub length: Option<i64>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<MusicBrainzArtistCredit>>,
    pub releases: Option<Vec<MusicBrainzRelease>>,
}

#[derive(Deserialize)]
pub struct MusicBrainzArtistCredit {
    pub artist: MusicBrainzArtist,
    pub joinphrase: Option<String>,
}

#[derive(Deserialize)]
pub struct MusicBrainzArtist {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct MusicBrainzRelease {
    pub id: String,
    pub title: Option<String>,
    pub date: Option<String>,
    pub country: Option<String>,
    #[serde(default)]
    pub disambiguation: Option<String>,
}

#[derive(Deserialize)]
pub struct MusicBrainzSearchResponse {
    pub recordings: Option<Vec<MusicBrainzSearchRecording>>,
}

#[derive(Deserialize)]
pub struct MusicBrainzSearchRecording {
    pub id: String,
    pub title: Option<String>,
    pub score: Option<f64>,
    pub length: Option<i64>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<MusicBrainzArtistCredit>>,
    pub releases: Option<Vec<MusicBrainzRelease>>,
}

#[derive(Deserialize)]
pub struct MusicBrainzReleaseResponse {
    pub id: String,
    pub title: Option<String>,
    pub date: Option<String>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<MusicBrainzArtistCredit>>,
    pub media: Option<Vec<MusicBrainzMedia>>,
    #[serde(default)]
    pub disambiguation: Option<String>,
}

#[derive(Deserialize)]
pub struct MusicBrainzMedia {
    pub position: Option<i32>,
    #[serde(rename = "track-count")]
    pub track_count: Option<i32>,
    pub tracks: Option<Vec<MusicBrainzTrack>>,
}

#[derive(Deserialize)]
pub struct MusicBrainzTrack {
    pub id: String,
    pub position: Option<i32>,
    pub title: Option<String>,
    pub recording: Option<MusicBrainzTrackRecording>,
}

#[derive(Deserialize)]
pub struct MusicBrainzTrackRecording {
    pub id: String,
    pub title: Option<String>,
    pub length: Option<i64>,
}
