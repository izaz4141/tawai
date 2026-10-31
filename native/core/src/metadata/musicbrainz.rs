use anyhow::Result;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::header;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::signals::library::ArtistInfo;
use crate::signals::metadata::{MBSearchInfo, RecordingInfo, ReleaseInfo, ReleaseTrackInfo};
use crate::utils::MB_UA;

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

/// Build the canonical display credit and the structured artist list from a
/// MusicBrainz `artist-credit` array.
///
/// The display string concatenates each artist's name with its join phrase
/// (e.g. `["Foo", " & "]` + `["Bar"]` → `"Foo & Bar"`), matching how
/// MusicBrainz renders credits. Returns `("", [])` when there is no credit.
fn artists_from_credit(
    credits: Option<&Vec<MusicBrainzArtistCredit>>,
) -> (String, Vec<ArtistInfo>) {
    let Some(credits) = credits.filter(|c| !c.is_empty()) else {
        return (String::new(), Vec::new());
    };

    let display = credits
        .iter()
        .map(|c| {
            let join = c.joinphrase.as_deref().unwrap_or("");
            format!("{}{}", c.artist.name, join)
        })
        .collect();

    let artists = credits
        .iter()
        .map(|c| ArtistInfo {
            id: c.artist.id.clone(),
            name: c.artist.name.clone(),
            sort_name: c.artist.sort_name.clone(),
            mbid: Some(c.artist.id.clone()),
            thumbnail_url: None,
            album_count: 0,
            track_count: 0,
        })
        .collect();

    (display, artists)
}

/// Extract the genre names from a MusicBrainz genre array.
fn genre_names(genres: Option<&[MusicBrainzGenre]>) -> Vec<String> {
    genres
        .map(|genres| genres.iter().map(|g| g.name.clone()).collect())
        .unwrap_or_default()
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
    let artists: Vec<ArtistInfo> = recording
        .artists
        .unwrap_or_default()
        .into_iter()
        .map(|a| ArtistInfo {
            id: a.id.clone().unwrap_or_default(),
            name: a.name.unwrap_or_default(),
            sort_name: None,
            mbid: a.id,
            thumbnail_url: None,
            album_count: 0,
            track_count: 0,
        })
        .collect();
    let artist = artists
        .iter()
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join("; ");
    let artist_id = artists.first().map(|a| a.id.clone());
    let duration = recording.duration.unwrap_or(duration_secs);

    let releases: Vec<_> = recording
        .releasegroups
        .unwrap_or_default()
        .into_iter()
        .flat_map(|rg| {
            let releasegroup_title = rg.title.clone();
            let artist = artist.clone();
            let artist_id = artist_id.clone();
            let artists = artists.clone();
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
                artists: artists.clone(),
                genres: Vec::new(),
                tracks: Vec::new(),
                disambiguation: None,
                total_discs: r.medium_count,
                total_tracks: r.track_count,
                release_group_mbid: None,
                release_type: None,
                record_label: None,
                media: None,
                script: None,
                original_date: None,
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
        artists,
        isrcs: Vec::new(),
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
        let (mut artist, artists) = artists_from_credit(rec.artist_credit.as_ref());
        if artist.is_empty() {
            artist = "Unknown Artist".to_string();
        }
        let artist_id = artists.first().map(|a| a.id.clone());
        let isrcs = rec.isrcs.unwrap_or_default();

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
                artists: artists.clone(),
                genres: Vec::new(),
                tracks: Vec::new(),
                disambiguation: r.disambiguation,
                total_discs: None,
                total_tracks: None,
                release_group_mbid: None,
                release_type: None,
                record_label: None,
                media: None,
                script: None,
                original_date: None,
            })
            .collect();

        let cover = find_cover_url(client, &releases).await;

        out.push(RecordingInfo {
            id: rec.id,
            title: rec.title.unwrap_or_default(),
            score: rec.score.unwrap_or(0.0) / 100.0,
            artist,
            artist_id,
            artists,
            isrcs,
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
        "https://musicbrainz.org/ws/2/release/{release_id}?inc=recordings+artist-credits+labels+media+release-groups+isrcs+genres&fmt=json"
    );

    let release: MusicBrainzReleaseResponse = mb_get_json(client, &url).await?;

    let (artist, artists) = artists_from_credit(release.artist_credit.as_ref());

    let artist_id = artists.first().map(|a| a.id.clone());
    let genres = genre_names(release.genres.as_deref());

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

    let media = release.media.as_ref().and_then(|media| {
        let mut formats: Vec<String> = Vec::new();
        for medium in media {
            if let Some(format) = &medium.format {
                if !format.is_empty() && !formats.contains(format) {
                    formats.push(format.clone());
                }
            }
        }
        if formats.is_empty() {
            None
        } else {
            Some(formats.join(" + "))
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
            let recording = t.recording?;
            let credit = t
                .artist_credit
                .as_ref()
                .filter(|c| !c.is_empty())
                .or(recording.artist_credit.as_ref());
            let (_, track_artists) = artists_from_credit(credit);
            Some(ReleaseTrackInfo {
                id: recording.id,
                title: recording.title.unwrap_or(t.title.unwrap_or_default()),
                position: t.position,
                disc_number: disc,
                duration_secs: recording.length.map(|l| l as f64 / 1000.0),
                artists: track_artists,
                mbid_track: Some(t.id),
                isrcs: recording.isrcs.unwrap_or_default(),
                genres: genre_names(recording.genres.as_deref()),
                ..Default::default()
            })
        })
        .collect();

    let release_group = release.release_group;
    let release_group_mbid = release_group.as_ref().map(|rg| rg.id.clone());
    let original_date = release_group
        .as_ref()
        .and_then(|rg| rg.first_release_date.clone());
    let release_type = release_group.as_ref().and_then(|rg| {
        let mut types: Vec<String> = Vec::new();
        if let Some(primary) = &rg.primary_type {
            if !primary.is_empty() {
                types.push(primary.clone());
            }
        }
        if let Some(secondary) = &rg.secondary_types {
            types.extend(secondary.iter().filter(|s| !s.is_empty()).cloned());
        }
        if types.is_empty() {
            None
        } else {
            Some(types.join("; "))
        }
    });

    let record_label = release.label_info.as_ref().and_then(|infos| {
        let mut labels: Vec<String> = Vec::new();
        for info in infos {
            if let Some(label) = &info.label {
                if let Some(name) = &label.name {
                    if !name.is_empty() && !labels.contains(name) {
                        labels.push(name.clone());
                    }
                }
            }
        }
        if labels.is_empty() {
            None
        } else {
            Some(labels.join("; "))
        }
    });

    let script = release
        .text_representation
        .as_ref()
        .and_then(|tr| tr.script.clone());

    Ok(ReleaseInfo {
        id: release.id,
        title: release.title.unwrap_or_default(),
        date: release.date,
        country: release.country,
        artist,
        artist_id,
        artists,
        genres,
        tracks,
        disambiguation: release.disambiguation,
        total_discs: Some(total_discs),
        total_tracks,
        release_group_mbid,
        release_type,
        record_label,
        media,
        script,
        original_date,
    })
}

/// Fetch a recording by MusicBrainz MBID along with its releases.
pub async fn fetch_recording(client: &reqwest::Client, mbid: &str) -> Result<RecordingInfo> {
    let url = format!(
        "https://musicbrainz.org/ws/2/recording/{mbid}?inc=artists+releases+isrcs&fmt=json"
    );

    let recording: MusicBrainzRecording = mb_get_json(client, &url).await?;

    let (artist, artists) = artists_from_credit(recording.artist_credit.as_ref());
    let artist_id = artists.first().map(|a| a.id.clone());
    let isrcs = recording.isrcs.clone().unwrap_or_default();
    let id = recording.id;
    let title = recording.title.unwrap_or_default();
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
            artists: artists.clone(),
            genres: Vec::new(),
            tracks: Vec::new(),
            disambiguation: r.disambiguation,
            total_discs: None,
            total_tracks: None,
            release_group_mbid: None,
            release_type: None,
            record_label: None,
            media: None,
            script: None,
            original_date: None,
        })
        .collect();

    let cover = find_cover_url(client, &releases).await;

    Ok(RecordingInfo {
        id,
        title,
        score: 100.0,
        artist,
        artist_id,
        artists,
        isrcs,
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
    #[serde(default)]
    pub isrcs: Option<Vec<String>>,
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
    #[serde(rename = "sort-name", default)]
    pub sort_name: Option<String>,
}

#[derive(Deserialize)]
pub struct MusicBrainzGenre {
    #[serde(default)]
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
    #[serde(default)]
    pub isrcs: Option<Vec<String>>,
    pub releases: Option<Vec<MusicBrainzRelease>>,
}

#[derive(Deserialize)]
pub struct MusicBrainzReleaseResponse {
    pub id: String,
    pub title: Option<String>,
    pub date: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<MusicBrainzArtistCredit>>,
    pub media: Option<Vec<MusicBrainzMedia>>,
    #[serde(default)]
    pub disambiguation: Option<String>,
    #[serde(rename = "text-representation", default)]
    pub text_representation: Option<MusicBrainzTextRepresentation>,
    #[serde(rename = "release-group", default)]
    pub release_group: Option<MusicBrainzReleaseGroup>,
    #[serde(rename = "label-info", default)]
    pub label_info: Option<Vec<MusicBrainzLabelInfo>>,
    #[serde(default)]
    pub genres: Option<Vec<MusicBrainzGenre>>,
}

#[derive(Deserialize)]
pub struct MusicBrainzTextRepresentation {
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub script: Option<String>,
}

#[derive(Deserialize)]
pub struct MusicBrainzReleaseGroup {
    pub id: String,
    #[serde(rename = "first-release-date", default)]
    pub first_release_date: Option<String>,
    #[serde(rename = "primary-type", default)]
    pub primary_type: Option<String>,
    #[serde(rename = "secondary-types", default)]
    pub secondary_types: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct MusicBrainzLabelInfo {
    #[serde(rename = "catalog-number", default)]
    pub catalog_number: Option<String>,
    #[serde(default)]
    pub label: Option<MusicBrainzLabel>,
}

#[derive(Deserialize)]
pub struct MusicBrainzLabel {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct MusicBrainzMedia {
    pub position: Option<i32>,
    #[serde(rename = "track-count")]
    pub track_count: Option<i32>,
    #[serde(default)]
    pub format: Option<String>,
    pub tracks: Option<Vec<MusicBrainzTrack>>,
}

#[derive(Deserialize)]
pub struct MusicBrainzTrack {
    pub id: String,
    pub position: Option<i32>,
    pub title: Option<String>,
    #[serde(rename = "artist-credit", default)]
    pub artist_credit: Option<Vec<MusicBrainzArtistCredit>>,
    pub recording: Option<MusicBrainzTrackRecording>,
}

#[derive(Deserialize)]
pub struct MusicBrainzTrackRecording {
    pub id: String,
    pub title: Option<String>,
    pub length: Option<i64>,
    #[serde(rename = "artist-credit", default)]
    pub artist_credit: Option<Vec<MusicBrainzArtistCredit>>,
    #[serde(default)]
    pub isrcs: Option<Vec<String>>,
    #[serde(default)]
    pub genres: Option<Vec<MusicBrainzGenre>>,
}
