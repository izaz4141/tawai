use std::path::Path;

use anyhow::{Context, Result};

use crate::audio::tags;
use crate::db::database::DatabasePool;
use crate::db::library;
use crate::signals::tools::{RomajizeLyricsRequest, RomajizeLyricsResponse, WriteLyricsResult};
use crate::utils::romajize;

fn is_valid_lrc_timestamp(s: &str) -> bool {
    let s = s.trim();
    if !s.starts_with('[') {
        return false;
    }
    let close = match s.find(']') {
        Some(c) => c,
        None => return false,
    };
    if close <= 1 {
        return false;
    }
    let inner = &s[1..close];
    let colon = match inner.find(':') {
        Some(c) => c,
        None => return false,
    };
    if colon == 0 || colon == inner.len() - 1 {
        return false;
    }
    if !inner[..colon].chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let sec_part = &inner[colon + 1..];
    let (secs, centis) = match sec_part.find('.') {
        Some(dot) => (&sec_part[..dot], Some(&sec_part[dot + 1..])),
        None => (sec_part, None),
    };
    if secs.is_empty() || !secs.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if let Some(cs) = centis {
        if cs.is_empty() || !cs.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    true
}

fn is_synced_lyrics(text: &str) -> bool {
    text.lines().any(|line| is_valid_lrc_timestamp(line))
}

fn split_lrc_line(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    if !is_valid_lrc_timestamp(trimmed) {
        return None;
    }
    let close = trimmed.find(']').unwrap();
    Some((&trimmed[..=close], &trimmed[close + 1..]))
}

pub fn romajize_lyrics(request: RomajizeLyricsRequest) -> Result<RomajizeLyricsResponse> {
    let synced = request.synced || is_synced_lyrics(&request.lyrics);
    let lang = request.lang.as_deref();

    let romajized = if synced {
        let mut out = String::new();
        for (i, line) in request.lyrics.lines().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            if let Some((ts, lyric)) = split_lrc_line(line) {
                match romajize::romajize(lyric, lang) {
                    Ok(r) if !r.trim().is_empty() => {
                        out.push_str(ts);
                        out.push_str(&r);
                    }
                    _ => out.push_str(line.trim()),
                }
            } else {
                match romajize::romajize(line.trim(), lang) {
                    Ok(r) => out.push_str(&r),
                    Err(_) => out.push_str(line.trim()),
                }
            }
        }
        out
    } else {
        romajize::romajize(&request.lyrics, lang)?
    };

    Ok(RomajizeLyricsResponse {
        romajized,
        synced,
        error: None,
    })
}

pub async fn write_track_lyrics(
    pool: &DatabasePool,
    track_id: &str,
    lyrics: &str,
) -> Result<WriteLyricsResult> {
    let track = library::lookup_track(pool, track_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Track not found: {}", track_id))?;

    let path = Path::new(&track.file_path);

    let (mut audio_tag, _duration, _sample_rate, _bitrate) =
        tags::read_audio_tags(path).context("Failed to read audio tags")?;

    audio_tag.lyrics = Some(lyrics.to_string());

    tags::write_audio_tags(path, &audio_tag).context("Failed to write audio tags")?;

    match pool {
        DatabasePool::Sqlite(p) => {
            sqlx::query("UPDATE tracks SET lyrics = ? WHERE id = ?")
                .bind(lyrics)
                .bind(track_id)
                .execute(p)
                .await?;
        }
        DatabasePool::Postgres(p) => {
            sqlx::query("UPDATE tracks SET lyrics = $1 WHERE id = $2")
                .bind(lyrics)
                .bind(track_id)
                .execute(p)
                .await?;
        }
    }

    Ok(WriteLyricsResult {
        success: true,
        error: None,
    })
}
