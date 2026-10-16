use anyhow::Result;
use lofty::config::WriteOptions;
use lofty::file::AudioFile;
use lofty::file::FileType;
use lofty::file::TaggedFile;
use lofty::file::TaggedFileExt;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::probe::Probe;
use lofty::read_from_path;
use lofty::tag::Accessor;
use lofty::tag::ItemKey;
use lofty::tag::ItemValue;
use lofty::tag::Tag;
use lofty::tag::TagItem;
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
        let lower = remaining.to_lowercase();
        let mut best_pos = usize::MAX;
        let mut best_sep = "";
        for sep in &separators {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagWriteMode {
    /// Write-if-set: only non-empty fields are written; empty fields are left
    /// untouched. Used by flows that build a partial `AudioTag` (download,
    /// import, lyrics, scan, identify) where an empty field means "unknown".
    Patch,
    /// Full-set: every editable field is written, and an empty field (empty
    /// string, `None`, empty list, or `0`) removes the stored value. Used by
    /// the tag editor, which always sends the complete editable set, so a
    /// field the user cleared actually gets cleared. Never affects pictures or
    /// unmanaged keys (acoustid, encoder, replaygain).
    Replace,
    /// Like [`TagWriteMode::Patch`] for scalar fields, but the explicit
    /// `TrackArtists` / `AlbumArtists` multi-value lists are written when
    /// non-empty. Used by the identify flow, which supplies those lists from
    /// the comparison sheet but otherwise patches the on-disk tags.
    PatchExplicitArtists,
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
    pub mbid_track: Option<String>,
    pub mbid_artist: Option<String>,
    pub mbid_release_artist: Option<String>,
    pub mbid_release: Option<String>,
    pub mbid_release_group: Option<String>,
    pub acoust_id: Option<String>,
    pub acoust_id_fingerprint: Option<String>,
    pub lyrics: Option<String>,
    pub cover: Option<Vec<u8>>,
    pub album_disambiguation: Option<String>,
    pub release_country: Option<String>,
    pub release_type: Vec<String>,
    pub record_label: Option<String>,
    pub media: Option<String>,
    pub script: Option<String>,
    pub original_date: Option<String>,
    pub comment: Option<String>,
    pub copyright: Option<String>,
    pub encoder: Option<String>,
    pub isrc: Option<String>,
    pub total_discs: i32,
    pub total_tracks: i32,
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
    let file_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown");
    Ok(extract_audio_tags(&tagged_file, file_stem))
}

/// Read tags from an audio file held entirely in memory.
pub fn read_audio_tags_from_bytes(
    filename: &str,
    bytes: &[u8],
) -> Result<(AudioTag, f64, Option<u32>, Option<u32>)> {
    let probe = match Probe::new(Cursor::new(bytes)).guess_file_type() {
        Ok(probe) => probe,
        Err(_) => {
            let file_type = FileType::from_path(filename)
                .ok_or_else(|| anyhow::anyhow!("Unknown file type for {filename}"))?;
            Probe::new(Cursor::new(bytes)).set_file_type(file_type)
        }
    };
    let tagged_file = probe.read()?;
    let file_stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown");
    Ok(extract_audio_tags(&tagged_file, file_stem))
}

/// `read_opt!` — fill an `Option<String>` field from a text item, if the field
/// is still empty. `read_i32!` — same for an integer field stored as text.
///
/// These collapse the ~18 identical "read one known key into the tag" blocks in
/// `extract_audio_tags` into one line each.
macro_rules! read_opt {
    ($tag:expr, $out:expr, $field:ident, $key:expr) => {
        if $out.$field.is_none() {
            $out.$field = $tag.get_string($key).map(str::to_string);
        }
    };
}
macro_rules! read_i32 {
    ($tag:expr, $out:expr, $field:ident, $key:expr) => {
        if $out.$field == 0 {
            $out.$field = $tag
                .get_string($key)
                .and_then(|s| s.trim().parse::<i32>().ok())
                .unwrap_or(0);
        }
    };
}

fn extract_audio_tags(
    tagged_file: &TaggedFile,
    file_stem: &str,
) -> (AudioTag, f64, Option<u32>, Option<u32>) {
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
                .get_string(ItemKey::AlbumArtist)
                .unwrap_or_default()
                .to_string();
        }
        if result.album_artist_sort.is_empty() {
            result.album_artist_sort = tag
                .get_string(ItemKey::AlbumArtistSortOrder)
                .unwrap_or_default()
                .to_string();
        }
        if result.artist_sort.is_empty() {
            result.artist_sort = tag
                .get_string(ItemKey::TrackArtistSortOrder)
                .unwrap_or_default()
                .to_string();
        }
        if result.genres.is_empty() {
            result.genres = parse_genres(&tag.genre().unwrap_or_default());
        }
        if result.artists.is_empty() {
            result.artists = tag
                .get_strings(ItemKey::TrackArtists)
                .map(str::to_string)
                .collect();
        }
        if result.album_artists.is_empty() {
            result.album_artists = tag
                .get_strings(ItemKey::AlbumArtists)
                .map(str::to_string)
                .collect();
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
        if result.release_type.is_empty() {
            result.release_type = tag
                .get_strings(ItemKey::MusicBrainzReleaseType)
                .map(str::to_string)
                .collect();
        }

        read_opt!(tag, result, mbid_recording, ItemKey::MusicBrainzRecordingId);
        read_opt!(tag, result, mbid_track, ItemKey::MusicBrainzTrackId);
        read_opt!(tag, result, mbid_artist, ItemKey::MusicBrainzArtistId);
        read_opt!(
            tag,
            result,
            mbid_release_artist,
            ItemKey::MusicBrainzReleaseArtistId
        );
        read_opt!(tag, result, mbid_release, ItemKey::MusicBrainzReleaseId);
        read_opt!(
            tag,
            result,
            mbid_release_group,
            ItemKey::MusicBrainzReleaseGroupId
        );
        read_opt!(tag, result, release_country, ItemKey::ReleaseCountry);
        read_opt!(tag, result, record_label, ItemKey::Label);
        read_opt!(tag, result, media, ItemKey::OriginalMediaType);
        read_opt!(tag, result, script, ItemKey::Script);
        read_opt!(tag, result, original_date, ItemKey::OriginalReleaseDate);
        read_opt!(tag, result, comment, ItemKey::Comment);
        read_opt!(tag, result, copyright, ItemKey::CopyrightMessage);
        read_opt!(tag, result, encoder, ItemKey::EncodedBy);
        read_opt!(tag, result, isrc, ItemKey::Isrc);
        read_opt!(tag, result, acoust_id, ItemKey::AcoustId);
        read_opt!(
            tag,
            result,
            acoust_id_fingerprint,
            ItemKey::AcoustIdFingerprint
        );
        // Known limitation: ItemKey::Lyrics is a no-op on ID3v2 (MP3); other
        // formats read it normally.
        read_opt!(tag, result, lyrics, ItemKey::Lyrics);
        read_i32!(tag, result, total_tracks, ItemKey::TrackTotal);
        read_i32!(tag, result, total_discs, ItemKey::DiscTotal);

        if result.track_gain.is_none() {
            result.track_gain = tag
                .get_string(ItemKey::ReplayGainTrackGain)
                .and_then(parse_gain_db);
        }
        if result.track_peak.is_none() {
            result.track_peak = tag
                .get_string(ItemKey::ReplayGainTrackPeak)
                .and_then(|s| s.trim().parse::<f64>().ok())
                .map(|v| v.clamp(0.0, 1.0));
        }
    }

    if result.title.is_empty() {
        result.title = file_stem.to_string();
    }
    if result.artist.is_empty() {
        result.artist = "Unknown Artist".to_string();
    }
    if result.artists.is_empty() {
        result.artists = parse_artists(&result.artist);
    }
    if result.album.is_empty() {
        result.album = "Unknown Album".to_string();
    }
    if result.album_artist.is_empty() {
        result.album_artist = "Unknown Artist".to_string();
    }
    if result.album_artists.is_empty() {
        result.album_artists = parse_artists(&result.album_artist);
    }
    if result.artist_sort.is_empty() {
        result.artist_sort = derive_sort_name(&result.artist);
    }
    if result.album_artist_sort.is_empty() {
        result.album_artist_sort = derive_sort_name(&result.album_artist);
    }

    (result, duration_secs, sample_rate, bitrate)
}

/// Write tags to an audio file using lofty.
///
/// `mode` selects the write contract: [`TagWriteMode::Patch`] writes only
/// non-empty fields (used by partial-tag flows); [`TagWriteMode::Replace`]
/// writes the full editable set and clears empty ones (used by the tag editor);
/// [`TagWriteMode::PatchExplicitArtists`] is patch-like but also writes an
/// explicit, non-empty `artists`/`album_artists` list (used by identify).
pub fn write_audio_tags(path: &Path, audio_tag: &AudioTag, mode: TagWriteMode) -> Result<()> {
    let mut tagged_file = read_from_path(path)?;
    apply_audio_tag(&mut tagged_file, audio_tag, mode)?;
    tagged_file.save_to_path(path, WriteOptions::default())?;
    Ok(())
}

/// Apply tags to an audio file held entirely in memory, returning the modified bytes.
pub fn write_audio_tags_to_bytes(
    filename: &str,
    bytes: &[u8],
    audio_tag: &AudioTag,
    mode: TagWriteMode,
) -> Result<Vec<u8>> {
    let probe = match Probe::new(Cursor::new(bytes)).guess_file_type() {
        Ok(probe) => probe,
        Err(_) => {
            let file_type = FileType::from_path(filename)
                .ok_or_else(|| anyhow::anyhow!("Unknown file type for {filename}"))?;
            Probe::new(Cursor::new(bytes)).set_file_type(file_type)
        }
    };
    let mut tagged_file = probe.read()?;
    apply_audio_tag(&mut tagged_file, audio_tag, mode)?;
    let mut output = Cursor::new(Vec::new());
    tagged_file.save_to(&mut output, WriteOptions::default())?;
    Ok(output.into_inner())
}

/// Set a single-value text item, replacing any existing values for the key.
///
/// In [`TagWriteMode::Replace`], an empty `value` removes the key instead, so a
/// field cleared in the tag editor does not keep its previous file value. In
/// [`TagWriteMode::Patch`] an empty value is left untouched.
fn set_text(tag: &mut Tag, key: ItemKey, value: &str, mode: TagWriteMode) {
    if !value.trim().is_empty() {
        tag.insert_text(key, value.to_string());
    } else if mode == TagWriteMode::Replace {
        tag.remove_key(key);
    }
}

/// [`set_text`] for optional fields; `None` behaves like an empty string.
fn set_opt_text(tag: &mut Tag, key: ItemKey, value: Option<&str>, mode: TagWriteMode) {
    set_text(tag, key, value.unwrap_or_default(), mode);
}

/// Write a multi-value item list under a single key, replacing any existing
/// values. lofty shapes the result per format at save time: formats that allow
/// repeated items get one item per value; single-value formats join the values
/// (and lofty splits them again on read).
fn set_multi(tag: &mut Tag, key: ItemKey, values: &[String]) {
    tag.remove_key(key);
    let mut iter = values
        .iter()
        .map(String::as_str)
        .filter(|v| !v.trim().is_empty());
    if let Some(first) = iter.next() {
        tag.insert_text(key, first.to_string());
        for value in iter {
            tag.push(TagItem::new(key, ItemValue::Text(value.to_string())));
        }
    }
}

/// `write_opt!` — write an `Option<String>` field via `set_opt_text`.
/// `write_num!` — write a positive integer through lofty's track/disk accessors.
/// `write_total!` — write a positive integer stored under a text `ItemKey`.
///
/// These collapse the repeated write blocks in `apply_audio_tag` into one line
/// each; the write-if-set / clear-on-empty behaviour lives in `set_text`.
macro_rules! write_opt {
    ($tag:expr, $audio:expr, $field:ident, $key:expr, $mode:expr) => {
        set_opt_text($tag, $key, $audio.$field.as_deref(), $mode);
    };
}
macro_rules! write_num {
    ($tag:expr, $value:expr, $set:ident, $remove:ident, $mode:expr) => {
        if $value > 0 {
            $tag.$set($value as u32);
        } else if $mode == TagWriteMode::Replace {
            $tag.$remove();
        }
    };
}
macro_rules! write_total {
    ($tag:expr, $value:expr, $key:expr, $mode:expr) => {
        if $value > 0 {
            $tag.insert_text($key, $value.to_string());
        } else if $mode == TagWriteMode::Replace {
            $tag.remove_key($key);
        }
    };
}

fn apply_audio_tag(
    tagged_file: &mut TaggedFile,
    audio_tag: &AudioTag,
    mode: TagWriteMode,
) -> Result<()> {
    let tag = if let Some(tag) = tagged_file.primary_tag_mut() {
        tag
    } else {
        let tag_type = tagged_file.primary_tag_type();
        tagged_file.insert_tag(lofty::tag::Tag::new(tag_type));
        tagged_file
            .primary_tag_mut()
            .ok_or_else(|| anyhow::anyhow!("Failed to create tag"))?
    };

    let replace = mode == TagWriteMode::Replace;

    if replace && audio_tag.title.trim().is_empty() {
        tag.remove_title();
    } else {
        tag.set_title(audio_tag.title.clone());
    }
    if replace && audio_tag.artist.trim().is_empty() {
        tag.remove_artist();
    } else {
        tag.set_artist(audio_tag.artist.clone());
    }
    if replace && audio_tag.album.trim().is_empty() {
        tag.remove_album();
    } else {
        tag.set_album(audio_tag.album.clone());
    }
    if replace && audio_tag.genres.iter().all(|g| g.trim().is_empty()) {
        tag.remove_genre();
    } else {
        tag.set_genre(audio_tag.genres.join("; "));
    }

    match audio_tag
        .release_date
        .as_deref()
        .filter(|d| !d.trim().is_empty())
    {
        Some(date) => {
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
        None if replace => tag.remove_date(),
        None => {}
    }

    write_num!(tag, audio_tag.track_number, set_track, remove_track, mode);
    write_num!(tag, audio_tag.disc_number, set_disk, remove_disk, mode);
    write_total!(tag, audio_tag.total_tracks, ItemKey::TrackTotal, mode);
    write_total!(tag, audio_tag.total_discs, ItemKey::DiscTotal, mode);

    set_text(tag, ItemKey::AlbumArtist, &audio_tag.album_artist, mode);
    set_text(
        tag,
        ItemKey::TrackArtistSortOrder,
        &audio_tag.artist_sort,
        mode,
    );
    set_text(
        tag,
        ItemKey::AlbumArtistSortOrder,
        &audio_tag.album_artist_sort,
        mode,
    );

    write_opt!(
        tag,
        audio_tag,
        mbid_recording,
        ItemKey::MusicBrainzRecordingId,
        mode
    );
    write_opt!(
        tag,
        audio_tag,
        mbid_track,
        ItemKey::MusicBrainzTrackId,
        mode
    );
    write_opt!(
        tag,
        audio_tag,
        mbid_artist,
        ItemKey::MusicBrainzArtistId,
        mode
    );
    write_opt!(
        tag,
        audio_tag,
        mbid_release_artist,
        ItemKey::MusicBrainzReleaseArtistId,
        mode
    );
    write_opt!(
        tag,
        audio_tag,
        mbid_release,
        ItemKey::MusicBrainzReleaseId,
        mode
    );
    write_opt!(
        tag,
        audio_tag,
        mbid_release_group,
        ItemKey::MusicBrainzReleaseGroupId,
        mode
    );
    write_opt!(tag, audio_tag, comment, ItemKey::Comment, mode);
    write_opt!(tag, audio_tag, copyright, ItemKey::CopyrightMessage, mode);
    write_opt!(tag, audio_tag, isrc, ItemKey::Isrc, mode);
    write_opt!(tag, audio_tag, record_label, ItemKey::Label, mode);
    write_opt!(tag, audio_tag, media, ItemKey::OriginalMediaType, mode);
    write_opt!(tag, audio_tag, script, ItemKey::Script, mode);
    write_opt!(
        tag,
        audio_tag,
        release_country,
        ItemKey::ReleaseCountry,
        mode
    );
    write_opt!(
        tag,
        audio_tag,
        original_date,
        ItemKey::OriginalReleaseDate,
        mode
    );
    // Known limitation: ItemKey::Lyrics is a no-op on ID3v2 (MP3); other
    // formats write it normally.
    write_opt!(tag, audio_tag, lyrics, ItemKey::Lyrics, mode);

    // Only the tag editor and the identify flow send an explicit artist list;
    // the other flows derive `artists`/`album_artists` from
    // `artist`/`album_artist`, and writing those derived values back would
    // pollute the tag. `Replace` always (re)writes the lists so a cleared
    // artist list is cleared; `PatchExplicitArtists` writes a list only when it
    // is non-empty, leaving any existing on-disk list untouched otherwise.
    match mode {
        TagWriteMode::Replace => {
            set_multi(tag, ItemKey::TrackArtists, &audio_tag.artists);
            set_multi(tag, ItemKey::AlbumArtists, &audio_tag.album_artists);
        }
        TagWriteMode::PatchExplicitArtists => {
            if !audio_tag.artists.is_empty() {
                set_multi(tag, ItemKey::TrackArtists, &audio_tag.artists);
            }
            if !audio_tag.album_artists.is_empty() {
                set_multi(tag, ItemKey::AlbumArtists, &audio_tag.album_artists);
            }
        }
        TagWriteMode::Patch => {}
    }
    if !audio_tag.release_type.is_empty() {
        set_multi(
            tag,
            ItemKey::MusicBrainzReleaseType,
            &audio_tag.release_type,
        );
    } else if replace {
        tag.remove_key(ItemKey::MusicBrainzReleaseType);
    }

    // AcoustID is not editable in the tag editor; keep write-if-present.
    if let Some(acoust_id) = &audio_tag.acoust_id {
        tag.insert_text(ItemKey::AcoustId, acoust_id.clone());
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
