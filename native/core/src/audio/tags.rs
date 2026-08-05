use anyhow::Result;
use lofty::config::WriteOptions;
use lofty::file::AudioFile;
use lofty::file::TaggedFileExt;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::read_from_path;
use lofty::tag::Accessor;
use lofty::tag::ItemKey;
use lofty::tag::items::Timestamp;
use std::io::Cursor;
use std::path::Path;

pub fn parse_artists(s: &str) -> Vec<String> {
    if s.is_empty() || s == "Unknown Artist" {
        return vec!["Unknown Artist".to_string()];
    }
    let separators = [
        " feat. ",
        " featuring ",
        " ft. ",
        " feat ",
        " ft ",
        " & ",
        " and ",
        ", ",
        "; ",
        " vs. ",
        " vs ",
        " with ",
    ];
    let mut individuals: Vec<String> = Vec::new();
    let mut remaining = s.to_string();
    loop {
        let mut best_pos = usize::MAX;
        let mut best_sep = "";
        for sep in &separators {
            let lower = remaining.to_lowercase();
            if let Some(pos) = lower.find(sep) {
                if pos < best_pos {
                    best_pos = pos;
                    best_sep = sep;
                }
            }
        }
        if best_pos == usize::MAX {
            break;
        }
        let before = remaining[..best_pos].trim().to_string();
        if !before.is_empty() && !individuals.contains(&before) {
            individuals.push(before);
        }
        remaining = remaining[best_pos + best_sep.len()..].trim().to_string();
    }
    if !remaining.is_empty() && !individuals.contains(&remaining) {
        individuals.push(remaining);
    }
    if individuals.is_empty() {
        individuals.push(s.to_string());
    }

    let combined = s.trim().to_string();
    let mut result = vec![combined.clone()];
    for a in individuals {
        if a != combined && !result.contains(&a) {
            result.push(a);
        }
    }
    result
}

pub fn derive_sort_name(name: &str) -> String {
    let trimmed = name.trim();
    for prefix in &["The ", "A ", "An "] {
        if trimmed.starts_with(prefix) {
            let rest = trimmed[prefix.len()..].trim();
            if !rest.is_empty() {
                return format!("{}, {}", rest, prefix.trim());
            }
        }
    }
    trimmed.to_string()
}

#[derive(Debug, Clone, Default)]
pub struct AudioTag {
    pub title: String,
    pub artist: String,
    pub artist_sort: String,
    pub artists: Vec<String>,
    pub album: String,
    pub album_artist: String,
    pub album_artist_sort: String,
    pub album_artists: Vec<String>,
    pub genres: Vec<String>,
    pub release_date: Option<String>,
    pub track_number: i32,
    pub disc_number: i32,
    pub mbid_recording: Option<String>,
    pub mbid_artist: Option<String>,
    pub mbid_release_artist: Option<String>,
    pub mbid_release: Option<String>,
    pub acoust_id: Option<String>,
    pub acoust_id_fingerprint: Option<String>,
    pub lyrics: Option<String>,
    pub cover: Option<Vec<u8>>,
    pub album_disambiguation: Option<String>,
    pub total_discs: i32,
    pub track_gain: Option<f64>,
    pub track_peak: Option<f64>,
}

pub fn parse_genres(s: &str) -> Vec<String> {
    s.split(&['/', ';', ',', '\0'][..])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parse a ReplayGain value like `"-3.22 dB"` or `"-3.22 LUFS"` into decibels.
/// Handles the U+2212 (minus sign) that some taggers emit.
pub fn parse_gain_db(s: &str) -> Option<f64> {
    let trimmed = s.trim();
    let number = trimmed
        .strip_suffix("dB")
        .or_else(|| trimmed.strip_suffix("db"))
        .or_else(|| trimmed.strip_suffix("LUFS"))
        .or_else(|| trimmed.strip_suffix("lufs"))
        .unwrap_or(trimmed)
        .trim();
    number.replace('\u{2212}', "-").parse::<f64>().ok()
}

pub fn read_audio_tags(path: &Path) -> Result<(AudioTag, f64, Option<u32>, Option<u32>)> {
    let tagged_file = read_from_path(path)?;
    let properties = tagged_file.properties();
    let tags = tagged_file.tags();

    let duration_secs = properties.duration().as_secs_f64();
    let sample_rate = properties.sample_rate();
    let bitrate = properties.audio_bitrate();

    let mut result = AudioTag::default();

    // Extract embedded cover art — prefer CoverFront, fall back to any picture
    for tag in tags.iter() {
        if result.cover.is_some() {
            break;
        }
        for picture in tag.pictures() {
            let data = picture.data();
            if data.is_empty() {
                continue;
            }
            let bytes = if let Ok(img) = image::load_from_memory(data) {
                let mut jpeg_buf = Vec::new();
                if img
                    .write_to(&mut Cursor::new(&mut jpeg_buf), image::ImageFormat::Jpeg)
                    .is_ok()
                {
                    jpeg_buf
                } else {
                    data.to_vec()
                }
            } else {
                data.to_vec()
            };

            let is_front = matches!(picture.pic_type(), PictureType::CoverFront);
            if is_front || result.cover.is_none() {
                result.cover = Some(bytes);
                if is_front {
                    break;
                }
            }
        }
    }

    for tag in tags {
        if result.title.is_empty() {
            result.title = tag.title().unwrap_or_default().to_string();
        }
        if result.artist.is_empty() {
            result.artist = tag.artist().unwrap_or_default().to_string();
        }
        if result.album.is_empty() {
            result.album = tag.album().unwrap_or_default().to_string();
        }
        if result.album_artist.is_empty() {
            result.album_artist = tag
                .get_string(lofty::tag::ItemKey::AlbumArtist)
                .unwrap_or_default()
                .to_string();
        }
        if result.album_artist_sort.is_empty() {
            result.album_artist_sort = tag
                .get_string(lofty::tag::ItemKey::AlbumArtistSortOrder)
                .unwrap_or_default()
                .to_string();
        }
        if result.artist_sort.is_empty() {
            result.artist_sort = tag
                .get_string(lofty::tag::ItemKey::TrackArtistSortOrder)
                .unwrap_or_default()
                .to_string();
        }
        if result.genres.is_empty() {
            result.genres = parse_genres(&tag.genre().unwrap_or_default());
        }
        if result.release_date.is_none() {
            result.release_date = tag.date().map(|t| {
                let base = format!("{:04}", t.year);
                match (t.month, t.day) {
                    (Some(m), Some(d)) => format!("{}-{:02}-{:02}", base, m, d),
                    (Some(m), None) => format!("{}-{:02}", base, m),
                    _ => base,
                }
            });
        }
        if result.track_number == 0 {
            result.track_number = tag.track().unwrap_or(0) as i32;
        }
        if result.disc_number == 0 {
            result.disc_number = tag.disk().unwrap_or(0) as i32;
        }
        if result.mbid_recording.is_none() {
            result.mbid_recording = tag
                .get_string(lofty::tag::ItemKey::MusicBrainzRecordingId)
                .map(|s| s.to_string());
        }
        if result.mbid_artist.is_none() {
            result.mbid_artist = tag
                .get_string(lofty::tag::ItemKey::MusicBrainzArtistId)
                .map(|s| s.to_string());
        }
        if result.mbid_release_artist.is_none() {
            result.mbid_release_artist = tag
                .get_string(lofty::tag::ItemKey::MusicBrainzReleaseArtistId)
                .map(|s| s.to_string());
        }
        if result.mbid_release.is_none() {
            result.mbid_release = tag
                .get_string(lofty::tag::ItemKey::MusicBrainzReleaseId)
                .map(|s| s.to_string());
        }
        if result.acoust_id.is_none() {
            result.acoust_id = tag
                .get_string(lofty::tag::ItemKey::AcoustId)
                .map(|s| s.to_string());
        }
        if result.acoust_id_fingerprint.is_none() {
            result.acoust_id_fingerprint = tag
                .get_string(lofty::tag::ItemKey::AcoustIdFingerprint)
                .map(|s| s.to_string());
        }
        if result.lyrics.is_none() {
            result.lyrics = tag.get_string(ItemKey::Lyrics).map(|s| s.to_string());
        }
        if result.track_gain.is_none() {
            result.track_gain = tag
                .get_string(lofty::tag::ItemKey::ReplayGainTrackGain)
                .and_then(parse_gain_db);
        }
        if result.track_peak.is_none() {
            result.track_peak = tag
                .get_string(lofty::tag::ItemKey::ReplayGainTrackPeak)
                .and_then(|s| s.trim().parse::<f64>().ok())
                .map(|v| v.clamp(0.0, 1.0));
        }
    }

    let file_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown");
    if result.title.is_empty() {
        result.title = file_name.to_string();
    }
    if result.artist.is_empty() {
        result.artist = "Unknown Artist".to_string();
    }
    result.artists = parse_artists(&result.artist);
    if result.album.is_empty() {
        result.album = "Unknown Album".to_string();
    }
    if result.album_artist.is_empty() {
        result.album_artist = "Unknown Artist".to_string();
    }
    result.album_artists = parse_artists(&result.album_artist);
    if result.artist_sort.is_empty() {
        result.artist_sort = derive_sort_name(&result.artist);
    }
    if result.album_artist_sort.is_empty() {
        result.album_artist_sort = derive_sort_name(&result.album_artist);
    }

    Ok((result, duration_secs, sample_rate, bitrate))
}

/// Write tags to an audio file using lofty.
/// Writes all metadata fields from the provided `AudioTag`.
pub fn write_audio_tags(path: &Path, audio_tag: &AudioTag) -> Result<()> {
    let mut tagged_file = read_from_path(path)?;

    let tag = if let Some(tag) = tagged_file.primary_tag_mut() {
        tag
    } else {
        let tag_type = tagged_file.primary_tag_type();
        tagged_file.insert_tag(lofty::tag::Tag::new(tag_type));
        tagged_file
            .primary_tag_mut()
            .ok_or_else(|| anyhow::anyhow!("Failed to create tag"))?
    };

    tag.set_title(audio_tag.title.clone());
    tag.set_artist(audio_tag.artist.clone());
    tag.set_album(audio_tag.album.clone());
    tag.set_genre(audio_tag.genres.join("; "));
    if let Some(date) = &audio_tag.release_date {
        let parts: Vec<&str> = date.split('-').collect();
        let year = parts.first().and_then(|y| y.parse().ok()).unwrap_or(0);
        let month = parts
            .get(1)
            .and_then(|m| m.parse::<u16>().ok().map(|v| v as u8));
        let day = parts
            .get(2)
            .and_then(|d| d.parse::<u16>().ok().map(|v| v as u8));
        tag.set_date(Timestamp {
            year,
            month,
            day,
            hour: None,
            minute: None,
            second: None,
        });
    }
    tag.set_track(audio_tag.track_number as u32);
    tag.set_disk(audio_tag.disc_number as u32);

    if !audio_tag.album_artist.is_empty() {
        tag.insert_text(ItemKey::AlbumArtist, audio_tag.album_artist.clone());
    }
    if !audio_tag.artist_sort.is_empty() {
        tag.insert_text(ItemKey::TrackArtistSortOrder, audio_tag.artist_sort.clone());
    }
    if !audio_tag.album_artist_sort.is_empty() {
        tag.insert_text(
            ItemKey::AlbumArtistSortOrder,
            audio_tag.album_artist_sort.clone(),
        );
    }
    if let Some(mbid) = &audio_tag.mbid_recording {
        tag.insert_text(ItemKey::MusicBrainzRecordingId, mbid.clone());
    }
    if let Some(mbid) = &audio_tag.mbid_artist {
        tag.insert_text(ItemKey::MusicBrainzArtistId, mbid.clone());
    }
    if let Some(mbid) = &audio_tag.mbid_release_artist {
        tag.insert_text(ItemKey::MusicBrainzReleaseArtistId, mbid.clone());
    }
    if let Some(mbid) = &audio_tag.mbid_release {
        tag.insert_text(ItemKey::MusicBrainzReleaseId, mbid.clone());
    }
    if let Some(acoust_id) = &audio_tag.acoust_id {
        tag.insert_text(ItemKey::AcoustId, acoust_id.clone());
    }
    if let Some(lyrics) = &audio_tag.lyrics {
        tag.insert_text(ItemKey::Lyrics, lyrics.clone());
    }

    if let Some(cover_bytes) = &audio_tag.cover {
        tag.remove_picture_type(PictureType::CoverFront);

        let mime_type = if cover_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            MimeType::Png
        } else if cover_bytes.starts_with(&[0xFF, 0xD8]) {
            MimeType::Jpeg
        } else if cover_bytes.starts_with(b"GIF8") || cover_bytes.starts_with(b"GIF89") {
            MimeType::Gif
        } else if cover_bytes.starts_with(b"BM") {
            MimeType::Bmp
        } else if cover_bytes.len() > 4
            && (cover_bytes[..4] == [0x49, 0x49, 0x2A, 0x00]
                || cover_bytes[..4] == [0x4D, 0x4D, 0x00, 0x2A])
        {
            MimeType::Tiff
        } else {
            MimeType::Jpeg
        };

        let picture = Picture::unchecked(cover_bytes.to_vec())
            .pic_type(PictureType::CoverFront)
            .mime_type(mime_type)
            .build();
        tag.push_picture(picture);
    }

    tagged_file.save_to_path(path, WriteOptions::default())?;
    Ok(())
}

/// Parse artist and title from a filename using common patterns.
/// Returns `(artist, title, track_number)` — all optional.
pub fn parse_filename_tags(stem: &str) -> (Option<String>, Option<String>, Option<i32>) {
    // Pattern: "Artist - Title"
    if let Some(dash_pos) = stem.find(" - ") {
        let artist = stem[..dash_pos].trim().to_string();
        let title = stem[dash_pos + 3..].trim().to_string();
        if !title.is_empty() && !artist.is_empty() {
            return (Some(artist), Some(title), None);
        }
    }

    // Pattern: "01 Title", "01. Title", "1 Title"
    let stem_trimmed = stem.trim();
    let digits: String = stem_trimmed
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if !digits.is_empty() {
        let after_digits = stem_trimmed[digits.len()..].trim();
        let rest = after_digits
            .strip_prefix('.')
            .unwrap_or(after_digits)
            .trim();
        if !rest.is_empty() {
            let track_num: i32 = digits.parse().unwrap_or(0);

            // Within rest, check for "Artist - Title"
            if let Some(dash_pos) = rest.find(" - ") {
                let artist = rest[..dash_pos].trim().to_string();
                let title = rest[dash_pos + 3..].trim().to_string();
                return (Some(artist), Some(title), Some(track_num));
            }
            return (None, Some(rest.to_string()), Some(track_num));
        }
    }

    // Fallback: filename is the title
    let title = stem_trimmed.to_string();
    if !title.is_empty() {
        (None, Some(title), None)
    } else {
        (None, None, None)
    }
}

pub fn supported_lofty_format(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "mp3"
            | "flac"
            | "ogg"
            | "m4a"
            | "mp4"
            | "wav"
            | "aiff"
            | "aac"
            | "ape"
            | "mpc"
            | "wv"
            | "opus"
            | "spx"
    )
}
