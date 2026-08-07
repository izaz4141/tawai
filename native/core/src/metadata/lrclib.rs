use anyhow::Result;
use serde::Deserialize;

use crate::signals::metadata::LyricsResult;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrclibTrack {
    id: u64,
    track_name: String,
    artist_name: String,
    album_name: String,
    duration: f64,
    instrumental: bool,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

pub async fn get_lyrics(
    client: &reqwest::Client,
    title: &str,
    artist: &str,
    album: &str,
    duration: f64,
    prefer_sync: bool,
) -> Result<LyricsResult> {
    let url = reqwest::Url::parse_with_params(
        "https://lrclib.net/api/get",
        &[
            ("track_name", title),
            ("artist_name", artist),
            ("album_name", album),
            ("duration", &duration.to_string()),
        ],
    )?;

    let resp = client.get(url).send().await.map_err(|e| {
        anyhow::anyhow!(
            "LRCLIB get request failed for [{}]. Underlying cause: {}",
            title,
            e
        )
    })?;

    if !resp.status().is_success() {
        anyhow::bail!(
            "LRCLIB get returned {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        );
    }

    let track: LrclibTrack = resp.json().await?;

    Ok(convert_track(track, prefer_sync))
}

pub async fn search_lyrics(
    client: &reqwest::Client,
    query: &str,
    prefer_sync: bool,
) -> Result<Vec<LyricsResult>> {
    let url = format!("https://lrclib.net/api/search?q={}", urlencoding(query));

    let resp = client.get(&url).send().await.map_err(|e| {
        anyhow::anyhow!(
            "LRCLIB get search failed for [{}]. Underlying cause: {}",
            query,
            e
        )
    })?;

    if !resp.status().is_success() {
        anyhow::bail!(
            "LRCLIB search returned {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        );
    }

    let tracks: Vec<LrclibTrack> = resp.json().await?;

    Ok(tracks
        .into_iter()
        .map(|t| convert_track(t, prefer_sync))
        .collect())
}

fn convert_track(track: LrclibTrack, prefer_sync: bool) -> LyricsResult {
    let (lyrics, synced) = if prefer_sync {
        if let Some(ref synced) = track.synced_lyrics {
            (synced.clone(), true)
        } else {
            (track.plain_lyrics.clone().unwrap_or_default(), false)
        }
    } else {
        if let Some(ref plain) = track.plain_lyrics {
            (plain.clone(), false)
        } else if let Some(ref synced) = track.synced_lyrics {
            (synced.clone(), true)
        } else {
            (String::new(), false)
        }
    };

    LyricsResult {
        id: track.id,
        title: track.track_name,
        artist: track.artist_name,
        album: track.album_name,
        duration: track.duration,
        instrumental: track.instrumental,
        lyrics,
        synced,
    }
}

fn urlencoding(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}
