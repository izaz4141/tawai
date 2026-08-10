use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use sqlx::Row;

use crate::audio::tags::{AudioTag, derive_sort_name, parse_artists};
use crate::db::database::DatabasePool;
use crate::signals::tools::{NamingViolation, RenamePreview};

/// Format a naming pattern with tag values.
///
/// Variables: `{title}`, `{artist}`, `{album_artist}`, `{album}`,
/// `{track_number}`, `{track_padded}`, `{disc_number}`, `{disc_padded}`,
/// `{disc_prefix}`, `{year}`, `{release_date}`, `{album_disambiguation}`,
/// `{total_discs}`, `{multi_artist}`.
///
/// Syntax:
/// - `{var}` — value (sanitized for paths)
/// - `{var?expr|suffix}` — if var truthy: if expr has `{...}`, evaluate expr
///   recursively then append suffix; else prepend expr, append suffix.
/// - `{var??fallback[|suffix]}` — use var if non-empty (+ suffix), else
///   recursively evaluate fallback.
/// - `{var>N?expr|suffix}` — numeric comparison instead of empty-check.
/// - `\|` for literal pipe. `/` for subdirectories.
pub fn format_naming_pattern(pattern: &str, tag: &AudioTag) -> String {
    let safe = |s: &str| -> String {
        s.chars()
            .map(|c| {
                if c == '/' || c == '\\' || c == ':' || c == '\0' {
                    '_'
                } else {
                    c
                }
            })
            .collect::<String>()
            .trim()
            .to_string()
    };

    let resolve = |var: &str| -> String { get_naming_var_value(var, tag) };
    let unescape = |s: &str| -> String { s.replace("\\|", "|").replace("\\\\", "\\") };

    let mut result = String::new();
    let chars: Vec<char> = pattern.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '{' {
            let mut depth = 1u32;
            let mut j = i + 1;
            while j < len && depth > 0 {
                if chars[j] == '{' {
                    depth += 1;
                } else if chars[j] == '}' {
                    depth -= 1;
                }
                j += 1;
            }
            if depth == 0 {
                let inner: String = chars[i + 1..j - 1].iter().collect();
                result.push_str(&process_pattern_token(
                    &inner, tag, &resolve, &safe, &unescape,
                ));
                i = j;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

fn process_pattern_token(
    inner: &str,
    tag: &AudioTag,
    resolve: &dyn Fn(&str) -> String,
    safe: &dyn Fn(&str) -> String,
    unescape: &dyn Fn(&str) -> String,
) -> String {
    // Fallback: {var??fallback[|suffix]}
    if let Some(qq_pos) = inner.find("??") {
        let var_name = &inner[..qq_pos];
        let rest = &inner[qq_pos + 2..];
        let (fallback_expr, suffix) = split_at_pipe_depth0(rest, unescape);

        let raw = resolve(var_name);
        if !raw.is_empty() {
            return format!("{}{}", safe(&raw), suffix);
        }
        return format_naming_pattern(fallback_expr, tag);
    }

    // Conditional: {var[>N]?expr|suffix}
    if let Some(q_pos) = inner.find('?') {
        let var_part = &inner[..q_pos];
        let cond = &inner[q_pos + 1..];

        let (var_name, compare_val) = if let Some(gt_pos) = var_part.find('>') {
            let name = &var_part[..gt_pos];
            let val: i32 = var_part[gt_pos + 1..].parse().unwrap_or(0);
            (name, Some(val))
        } else {
            (var_part, None)
        };

        let raw = resolve(var_name);
        let is_truthy = match compare_val {
            Some(cmp) => raw.parse::<i32>().unwrap_or(0) > cmp,
            None => !raw.is_empty(),
        };

        if is_truthy {
            let (expr, suffix) = split_at_pipe_depth0(cond, unescape);
            if expr.is_empty() {
                return format!("{}{}", safe(&raw), suffix);
            }
            if expr.contains('{') || expr.contains('}') {
                let formatted = format_naming_pattern(expr, tag);
                return format!("{}{}", formatted, suffix);
            }
            return format!("{}{}{}", expr, safe(&raw), suffix);
        }
        return String::new();
    }

    let raw = resolve(inner);
    if matches!(inner, "release_date" | "year") {
        raw
    } else {
        safe(&raw)
    }
}

/// Split `s` at the first `|` at brace-depth 0.
/// Returns (part_before_pipe, unescaped_part_after_pipe).
/// If no pipe found, returns (s, "").
fn split_at_pipe_depth0<'a>(s: &'a str, unescape: &dyn Fn(&str) -> String) -> (&'a str, String) {
    let mut depth = 0u32;
    for (pos, ch) in s.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' if depth > 0 => depth -= 1,
            '|' if depth == 0 => return (&s[..pos], unescape(&s[pos + 1..])),
            _ => {}
        }
    }
    (s, String::new())
}

fn get_naming_var_value(var: &str, tag: &AudioTag) -> String {
    match var {
        "title" => tag.title.clone(),
        "artist" => tag.artist.clone(),
        "album_artist" => tag.album_artist.clone(),
        "album" => tag.album.clone(),
        "release_date" => tag.release_date.clone().unwrap_or_default(),
        "year" => tag
            .release_date
            .as_ref()
            .and_then(|d| d.split('-').next().map(|y| y.to_string()))
            .unwrap_or_default(),
        "track_number" => tag.track_number.to_string(),
        "track_padded" => format!("{:02}", tag.track_number),
        "disc_number" => tag.disc_number.to_string(),
        "disc_padded" => format!("{:02}", tag.disc_number),
        "disc_prefix" => {
            if tag.disc_number > 1 {
                format!("{:02}-", tag.disc_number)
            } else {
                String::new()
            }
        }
        "album_disambiguation" => tag.album_disambiguation.clone().unwrap_or_default(),
        "total_discs" => tag.total_discs.to_string(),
        "multi_artist" => {
            if tag.artists.len() > 1 {
                "1".to_string()
            } else if !tag.artist.is_empty()
                && !tag.album_artist.is_empty()
                && tag.artist != tag.album_artist
            {
                "1".to_string()
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

/// Compute the destination file name for `pattern`, falling back to
/// `fallback_stem` when the formatted name is empty. Shared by preview and apply
/// so both produce identical paths.
fn dest_from_root(
    source_root: &str,
    pattern: &str,
    tag: &AudioTag,
    ext: &str,
    fallback_stem: &str,
) -> PathBuf {
    let formatted = format_naming_pattern(pattern, tag);
    let base = if formatted.trim().is_empty() {
        fallback_stem
    } else {
        formatted.as_str()
    };
    Path::new(source_root).join(format!("{}.{}", base, ext))
}

/// Compute the full destination path for a file rooted at the library source
/// directory. The naming pattern may contain subdirectories (`/`), so the
/// result is joined to the source root instead of the file's current parent,
/// which would otherwise duplicate the directory hierarchy.
pub(crate) fn expected_path_from_root(
    source_root: &str,
    pattern: &str,
    tag: &AudioTag,
    ext: &str,
) -> PathBuf {
    dest_from_root(source_root, pattern, tag, ext, "track")
}

/// Move (and rename) an audio file into `source_url`, rooting the naming
/// pattern's subdirectories at that folder.
///
/// When `pattern` is `None` the file keeps its current stem name. When a
/// pattern is given, `format_naming_pattern` is applied and the result is
/// used as the file stem.
pub fn move_file_into_source(
    old_path: &Path,
    source_url: &str,
    pattern: Option<&str>,
    tag: &AudioTag,
) -> Result<PathBuf> {
    let ext = old_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp3");
    let stem = old_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("track")
        .to_string();
    let new_path = match pattern {
        Some(p) => dest_from_root(source_url, p, tag, &ext, &stem),
        None => Path::new(source_url).join(format!("{}.{}", stem, ext)),
    };
    if new_path == old_path {
        return Ok(new_path);
    }
    if let Some(parent) = new_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(old_path, &new_path)?;
    Ok(new_path)
}

/// Apply a batch rename.
///
/// The naming pattern is rooted at each track's library source directory using
/// the same database metadata as [`batch_rename_preview`], so preview and apply
/// agree. Only tracks with a matching database record are renamed; anything
/// else produces an `<error: ...>` entry.
pub async fn batch_rename_apply(
    pool: &DatabasePool,
    file_paths: &[String],
    track_ids: &[String],
    pattern: &str,
) -> Result<Vec<RenamePreview>> {
    let ids: Vec<String> = track_ids
        .iter()
        .filter(|s| !s.is_empty())
        .cloned()
        .collect();
    let rows = match pool {
        DatabasePool::Sqlite(p) => fetch_rename_tracks_by_ids_sq(p, &ids).await?,
        DatabasePool::Postgres(p) => fetch_rename_tracks_by_ids_pg(p, &ids).await?,
    };
    let by_id: HashMap<String, RenameTrackDb> =
        rows.into_iter().map(|r| (r.track_id.clone(), r)).collect();

    let mut results = Vec::new();
    for (i, path_str) in file_paths.iter().enumerate() {
        let track_id = track_ids.get(i).map(String::as_str).unwrap_or("");
        let path = Path::new(path_str);
        let result = match by_id.get(track_id) {
            Some(row) => {
                let tag = build_audio_tag(row);
                move_file_into_source(path, &row.source_url, Some(pattern), &tag)
            }
            None => Err(anyhow::anyhow!("no database record for track")),
        };
        match result {
            Ok(new_path) => {
                if let Err(e) = crate::db::library::set_track_file_path(
                    pool,
                    track_id,
                    &new_path.to_string_lossy(),
                )
                .await
                {
                    crate::utils::logger::warn(&format!(
                        "failed to persist new path for track {}: {e}",
                        track_id
                    ));
                }
                results.push(RenamePreview {
                    file_path: path_str.clone(),
                    expected_path: new_path.to_string_lossy().to_string(),
                    track_id: track_id.to_string(),
                });
            }
            Err(e) => results.push(RenamePreview {
                file_path: path_str.clone(),
                expected_path: format!("<error: {}>", e),
                track_id: track_id.to_string(),
            }),
        }
    }

    Ok(results)
}

/// Preview renaming using database metadata (fast, no file I/O).
pub async fn batch_rename_preview(
    pool: &DatabasePool,
    source_id: Option<&str>,
    pattern: &str,
) -> Result<Vec<RenamePreview>> {
    let rows = match pool {
        DatabasePool::Sqlite(p) => fetch_rename_tracks_sq(p, source_id).await?,
        DatabasePool::Postgres(p) => fetch_rename_tracks_pg(p, source_id).await?,
    };
    Ok(build_previews(rows, pattern))
}

/// Check which files in the library don't follow the naming convention.
pub async fn check_naming_convention(
    pool: &DatabasePool,
    source_id: Option<&str>,
    pattern: &str,
) -> Result<Vec<NamingViolation>> {
    let rows = match pool {
        DatabasePool::Sqlite(p) => fetch_rename_tracks_sq(p, source_id).await?,
        DatabasePool::Postgres(p) => fetch_rename_tracks_pg(p, source_id).await?,
    };
    Ok(build_violations(rows, pattern))
}

/// Naming-relevant track data loaded from the database. Built once per row and
/// shared by preview, apply, and convention checks so all three use the same
/// `AudioTag`.
#[derive(Debug, Clone)]
pub(crate) struct RenameTrackDb {
    pub track_id: String,
    pub file_path: String,
    pub source_url: String,
    pub title: String,
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    pub track_number: i32,
    pub disc_number: i32,
    pub release_date: Option<String>,
    pub album_disambiguation: Option<String>,
    pub total_discs: i32,
    pub track_artists: Vec<String>,
    pub album_artists: Vec<String>,
    pub genres: Vec<String>,
    pub mbid_recording: Option<String>,
    pub mbid_artist: Option<String>,
    pub mbid_release: Option<String>,
    pub lyrics: Option<String>,
    pub track_gain: Option<f64>,
    pub track_peak: Option<f64>,
}

/// Build an `AudioTag` from database metadata, mirroring how tags are read from
/// files (see `extract_audio_tags`) so rename preview and apply produce the
/// same names. `{album_artist}` resolves to the album's artist and
/// `{multi_artist}` reflects the track's full artist list.
pub(crate) fn build_audio_tag(row: &RenameTrackDb) -> AudioTag {
    let mut tag = AudioTag {
        title: row.title.clone(),
        artist: row.artist.clone(),
        artists: row.track_artists.clone(),
        album: row.album.clone(),
        album_artist: row.album_artist.clone(),
        album_artists: row.album_artists.clone(),
        genres: row.genres.clone(),
        release_date: row.release_date.clone(),
        track_number: row.track_number,
        disc_number: row.disc_number,
        mbid_recording: row.mbid_recording.clone(),
        mbid_artist: row.mbid_artist.clone(),
        mbid_release: row.mbid_release.clone(),
        lyrics: row.lyrics.clone(),
        album_disambiguation: row.album_disambiguation.clone(),
        total_discs: row.total_discs,
        track_gain: row.track_gain,
        track_peak: row.track_peak,
        ..Default::default()
    };
    if tag.artists.is_empty() {
        tag.artists = parse_artists(&tag.artist);
    }
    if tag.album_artists.is_empty() {
        tag.album_artists = parse_artists(&tag.album_artist);
    }
    if tag.artist_sort.is_empty() {
        tag.artist_sort = derive_sort_name(&tag.artist);
    }
    if tag.album_artist_sort.is_empty() {
        tag.album_artist_sort = derive_sort_name(&tag.album_artist);
    }
    tag
}

fn split_pipe_list(s: Option<String>) -> Vec<String> {
    s.map(|v| {
        v.split("||")
            .filter(|x| !x.is_empty())
            .map(|x| x.to_string())
            .collect()
    })
    .unwrap_or_default()
}

pub(crate) fn ext_of(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp3")
        .to_string()
}

fn stem_of(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("track")
        .to_string()
}

fn build_previews(rows: Vec<RenameTrackDb>, pattern: &str) -> Vec<RenamePreview> {
    rows.into_iter()
        .map(|row| {
            let ext = ext_of(&row.file_path);
            let fallback = stem_of(&row.file_path);
            let expected_path = dest_from_root(
                &row.source_url,
                pattern,
                &build_audio_tag(&row),
                &ext,
                &fallback,
            );
            RenamePreview {
                file_path: row.file_path,
                expected_path: expected_path.to_string_lossy().to_string(),
                track_id: row.track_id,
            }
        })
        .collect()
}

fn build_violations(rows: Vec<RenameTrackDb>, pattern: &str) -> Vec<NamingViolation> {
    let mut violations = Vec::new();
    for row in rows {
        let tag = build_audio_tag(&row);
        let ext = ext_of(&row.file_path);
        let fallback = stem_of(&row.file_path);
        let expected_name = format_naming_pattern(pattern, &tag);
        let expected_path = dest_from_root(&row.source_url, pattern, &tag, &ext, &fallback);

        if expected_path.to_string_lossy() != row.file_path {
            let file_name = Path::new(&row.file_path)
                .strip_prefix(&row.source_url)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| row.file_path.clone());
            violations.push(NamingViolation {
                file_path: row.file_path,
                file_name,
                expected_name: format!("{}.{}", expected_name, ext),
                track_id: row.track_id,
            });
        }
    }
    violations
}

const SQ_RENAME_TRACK_SELECT: &str = r#"SELECT t.id AS track_id, t.file_path, t.title,
    COALESCE(ar.name, '') AS artist_name,
    COALESCE(aa.name, '') AS album_artist,
    COALESCE(a.title, '') AS album_title,
    t.track_num, t.disc_num, a.date, a.disambiguation, a.total_discs,
    ls.url AS source_url,
    COALESCE((SELECT GROUP_CONCAT(ta2.name, '||') FROM track_artists ta1 JOIN artists ta2 ON ta1.artist_id = ta2.id WHERE ta1.track_id = t.id), '') AS track_artists,
    COALESCE((SELECT GROUP_CONCAT(aa2.name, '||') FROM album_artists aa1 JOIN artists aa2 ON aa1.artist_id = aa2.id WHERE aa1.album_id = a.id), '') AS album_artists,
    COALESCE((SELECT GROUP_CONCAT(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id), '') AS genres,
    t.mbid_recording, ar.mbid AS mbid_artist, a.mbid AS mbid_release,
    t.lyrics, t.track_gain, t.track_peak
    FROM tracks t
    JOIN albums a ON t.album_id = a.id
    JOIN artists ar ON t.artist_id = ar.id
    LEFT JOIN artists aa ON a.artist_id = aa.id
    JOIN library_sources ls ON t.source_id = ls.id"#;

const PG_RENAME_TRACK_SELECT: &str = r#"SELECT t.id AS track_id, t.file_path, t.title,
    COALESCE(ar.name, '') AS artist_name,
    COALESCE(aa.name, '') AS album_artist,
    COALESCE(a.title, '') AS album_title,
    t.track_num, t.disc_num, a.date, a.disambiguation, a.total_discs,
    ls.url AS source_url,
    COALESCE((SELECT string_agg(ta2.name, '||') FROM track_artists ta1 JOIN artists ta2 ON ta1.artist_id = ta2.id WHERE ta1.track_id = t.id), '') AS track_artists,
    COALESCE((SELECT string_agg(aa2.name, '||') FROM album_artists aa1 JOIN artists aa2 ON aa1.artist_id = aa2.id WHERE aa1.album_id = a.id), '') AS album_artists,
    COALESCE((SELECT string_agg(g.name, '||') FROM track_genres tg JOIN genres g ON tg.genre_id = g.id WHERE tg.track_id = t.id), '') AS genres,
    t.mbid_recording, ar.mbid AS mbid_artist, a.mbid AS mbid_release,
    t.lyrics, t.track_gain, t.track_peak
    FROM tracks t
    JOIN albums a ON t.album_id = a.id
    JOIN artists ar ON t.artist_id = ar.id
    LEFT JOIN artists aa ON a.artist_id = aa.id
    JOIN library_sources ls ON t.source_id = ls.id"#;

async fn fetch_rename_tracks_sq(
    pool: &sqlx::SqlitePool,
    source_id: Option<&str>,
) -> Result<Vec<RenameTrackDb>> {
    let rows = if let Some(sid) = source_id {
        sqlx::query(&format!(
            "{SQ_RENAME_TRACK_SELECT} WHERE t.source_id = ? AND ls.source_type NOT LIKE 'recommendation:%' ORDER BY t.file_path"
        ))
        .bind(sid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(&format!(
            "{SQ_RENAME_TRACK_SELECT} WHERE ls.source_type NOT LIKE 'recommendation:%' ORDER BY t.file_path"
        ))
        .fetch_all(pool)
        .await?
    };
    Ok(rows.iter().map(rename_track_from_sq_row).collect())
}

async fn fetch_rename_tracks_pg(
    pool: &sqlx::PgPool,
    source_id: Option<&str>,
) -> Result<Vec<RenameTrackDb>> {
    let rows = if let Some(sid) = source_id {
        sqlx::query(&format!(
            "{PG_RENAME_TRACK_SELECT} WHERE t.source_id = $1 AND ls.source_type NOT LIKE 'recommendation:%' ORDER BY t.file_path"
        ))
        .bind(sid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(&format!(
            "{PG_RENAME_TRACK_SELECT} WHERE ls.source_type NOT LIKE 'recommendation:%' ORDER BY t.file_path"
        ))
        .fetch_all(pool)
        .await?
    };
    Ok(rows.iter().map(rename_track_from_pg_row).collect())
}

async fn fetch_rename_tracks_by_ids_sq(
    pool: &sqlx::SqlitePool,
    track_ids: &[String],
) -> Result<Vec<RenameTrackDb>> {
    if track_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders: Vec<String> = track_ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "{SQ_RENAME_TRACK_SELECT} WHERE t.id IN ({})",
        placeholders.join(",")
    );
    let mut query = sqlx::query(&sql);
    for id in track_ids {
        query = query.bind(id);
    }
    let rows = query.fetch_all(pool).await?;
    Ok(rows.iter().map(rename_track_from_sq_row).collect())
}

async fn fetch_rename_tracks_by_ids_pg(
    pool: &sqlx::PgPool,
    track_ids: &[String],
) -> Result<Vec<RenameTrackDb>> {
    if track_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders: Vec<String> = (1..=track_ids.len()).map(|i| format!("${i}")).collect();
    let sql = format!(
        "{PG_RENAME_TRACK_SELECT} WHERE t.id IN ({})",
        placeholders.join(",")
    );
    let mut query = sqlx::query(&sql);
    for id in track_ids {
        query = query.bind(id);
    }
    let rows = query.fetch_all(pool).await?;
    Ok(rows.iter().map(rename_track_from_pg_row).collect())
}

pub(crate) fn rename_track_from_sq_row(row: &sqlx::sqlite::SqliteRow) -> RenameTrackDb {
    RenameTrackDb {
        track_id: row.get("track_id"),
        file_path: row.get("file_path"),
        source_url: row.get("source_url"),
        title: row.get("title"),
        artist: row.get("artist_name"),
        album_artist: row.get("album_artist"),
        album: row.get("album_title"),
        track_number: row.get::<Option<i32>, _>("track_num").unwrap_or(0),
        disc_number: row.get::<Option<i32>, _>("disc_num").unwrap_or(1),
        release_date: row.get("date"),
        album_disambiguation: row.get("disambiguation"),
        total_discs: row.get::<Option<i32>, _>("total_discs").unwrap_or(0),
        track_artists: split_pipe_list(row.get("track_artists")),
        album_artists: split_pipe_list(row.get("album_artists")),
        genres: split_pipe_list(row.get("genres")),
        mbid_recording: row.get("mbid_recording"),
        mbid_artist: row.get("mbid_artist"),
        mbid_release: row.get("mbid_release"),
        lyrics: row.get("lyrics"),
        track_gain: row.get("track_gain"),
        track_peak: row.get("track_peak"),
    }
}

pub(crate) fn rename_track_from_pg_row(row: &sqlx::postgres::PgRow) -> RenameTrackDb {
    RenameTrackDb {
        track_id: row.get("track_id"),
        file_path: row.get("file_path"),
        source_url: row.get("source_url"),
        title: row.get("title"),
        artist: row.get("artist_name"),
        album_artist: row.get("album_artist"),
        album: row.get("album_title"),
        track_number: row.get::<Option<i32>, _>("track_num").unwrap_or(0),
        disc_number: row.get::<Option<i32>, _>("disc_num").unwrap_or(1),
        release_date: row.get("date"),
        album_disambiguation: row.get("disambiguation"),
        total_discs: row.get::<Option<i32>, _>("total_discs").unwrap_or(0),
        track_artists: split_pipe_list(row.get("track_artists")),
        album_artists: split_pipe_list(row.get("album_artists")),
        genres: split_pipe_list(row.get("genres")),
        mbid_recording: row.get("mbid_recording"),
        mbid_artist: row.get("mbid_artist"),
        mbid_release: row.get("mbid_release"),
        lyrics: row.get("lyrics"),
        track_gain: row.get("track_gain"),
        track_peak: row.get("track_peak"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_PATTERN: &str = "{album_artist??{artist?|/}|/}{album_artist?{album?|/}}{total_discs>1?{disc_padded}|-}{album_artist?{track_padded}| }{multi_artist?{artist}| - }{title}";

    fn tag(
        title: &str,
        artist: &str,
        album_artist: &str,
        album: &str,
        track: i32,
        disc: i32,
        total_discs: i32,
        artists: &[&str],
    ) -> AudioTag {
        AudioTag {
            title: title.to_string(),
            artist: artist.to_string(),
            artist_sort: String::new(),
            artists: artists.iter().map(|s| s.to_string()).collect(),
            album: album.to_string(),
            album_artist: album_artist.to_string(),
            album_artist_sort: String::new(),
            album_artists: vec![],
            genres: vec![],
            release_date: Some("2024-01-01".to_string()),
            track_number: track,
            disc_number: disc,
            mbid_recording: None,
            mbid_artist: None,
            mbid_release_artist: None,
            mbid_release: None,
            acoust_id: None,
            acoust_id_fingerprint: None,
            lyrics: None,
            cover: None,
            album_disambiguation: None,
            total_discs,
            track_gain: None,
            track_peak: None,
        }
    }

    #[test]
    fn default_pattern_all_fields_set() {
        let t = tag(
            "Title",
            "Some Artist",
            "Album Artist",
            "Album",
            7,
            2,
            2,
            &["Some Artist"],
        );
        assert_eq!(
            format_naming_pattern(DEFAULT_PATTERN, &t),
            "Album Artist/Album/02-07 Some Artist - Title"
        );
    }

    #[test]
    fn default_pattern_no_album_artist() {
        let t = tag(
            "Title",
            "Some Artist",
            "",
            "Album",
            7,
            2,
            2,
            &["Some Artist"],
        );
        assert_eq!(
            format_naming_pattern(DEFAULT_PATTERN, &t),
            "Some Artist/02-Title"
        );
    }

    #[test]
    fn default_pattern_single_disc() {
        let t = tag(
            "Title",
            "Some Artist",
            "Album Artist",
            "Album",
            7,
            1,
            1,
            &["Some Artist"],
        );
        assert_eq!(
            format_naming_pattern(DEFAULT_PATTERN, &t),
            "Album Artist/Album/07 Some Artist - Title"
        );
    }

    #[test]
    fn default_pattern_single_artist_album() {
        let t = tag(
            "Title",
            "Album Artist",
            "Album Artist",
            "Album",
            7,
            1,
            1,
            &["Album Artist"],
        );
        assert_eq!(
            format_naming_pattern(DEFAULT_PATTERN, &t),
            "Album Artist/Album/07 Title"
        );
    }

    #[test]
    fn simple_variables() {
        let t = tag("Title", "Artist", "", "Album", 3, 1, 1, &["Artist"]);
        assert_eq!(format_naming_pattern("{album}/{title}", &t), "Album/Title");
    }

    #[test]
    fn sanitize_path_chars() {
        let t = tag("A:Title", "Art\\ist", "", "Album", 3, 1, 1, &[]);
        assert_eq!(
            format_naming_pattern("{artist} {title}", &t),
            "Art_ist A_Title"
        );
    }

    #[test]
    fn literal_pipe_in_suffix() {
        let t = tag("Title", "Artist", "", "Album", 3, 1, 1, &["Artist"]);
        assert_eq!(
            format_naming_pattern("{artist?{title}| \\| ok}", &t),
            "Title | ok"
        );
    }

    #[test]
    fn numeric_compare_false() {
        let t = tag("Title", "Artist", "", "Album", 3, 1, 1, &["Artist"]);
        assert_eq!(
            format_naming_pattern("{total_discs>1?{disc_padded}|-}{title}", &t),
            "Title"
        );
    }

    #[test]
    fn numeric_compare_true() {
        let t = tag("Title", "Artist", "", "Album", 3, 2, 2, &["Artist"]);
        assert_eq!(
            format_naming_pattern("{total_discs>1?{disc_padded}|-}{title}", &t),
            "02-Title"
        );
    }

    #[test]
    fn fallback_recursion() {
        let t = tag("Title", "Artist", "", "Album", 3, 1, 1, &["Artist"]);
        assert_eq!(
            format_naming_pattern("{album_artist??{artist}}/", &t),
            "Artist/"
        );
        let t2 = tag("Title", "Artist", "AA", "Album", 3, 1, 1, &["Artist"]);
        assert_eq!(
            format_naming_pattern("{album_artist??{artist}}/", &t2),
            "AA/"
        );
    }

    #[test]
    fn expected_path_rooted_at_source_no_duplication() {
        let t = tag(
            "Title",
            "Some Artist",
            "Album Artist",
            "Album",
            7,
            2,
            2,
            &["Some Artist"],
        );
        let path = expected_path_from_root("/music", DEFAULT_PATTERN, &t, "flac");
        assert_eq!(
            path.to_string_lossy(),
            "/music/Album Artist/Album/02-07 Some Artist - Title.flac"
        );
        assert!(!path.to_string_lossy().contains("Album Artist/Album/Album"));
    }

    fn rename_track(artist: &str, album_artist: &str, track_artists: &[&str]) -> RenameTrackDb {
        RenameTrackDb {
            track_id: "t1".to_string(),
            file_path: "/music/raw.mp3".to_string(),
            source_url: "/music".to_string(),
            title: "Title".to_string(),
            artist: artist.to_string(),
            album_artist: album_artist.to_string(),
            album: "Album".to_string(),
            track_number: 7,
            disc_number: 1,
            release_date: Some("2024-01-01".to_string()),
            album_disambiguation: None,
            total_discs: 1,
            track_artists: track_artists.iter().map(|s| s.to_string()).collect(),
            album_artists: vec![],
            genres: vec![],
            mbid_recording: None,
            mbid_artist: None,
            mbid_release: None,
            lyrics: None,
            track_gain: None,
            track_peak: None,
        }
    }

    #[test]
    fn build_audio_tag_uses_album_artist_and_track_artists() {
        let row = rename_track(
            "Main Artist",
            "Album Owner",
            &["Main Artist", "Feat Artist"],
        );
        let tag = build_audio_tag(&row);
        assert_eq!(tag.album_artist, "Album Owner");
        assert_eq!(tag.artist, "Main Artist");
        assert_eq!(tag.artists.len(), 2);
        assert_eq!(tag.album_artist_sort, derive_sort_name("Album Owner"));
        assert_eq!(tag.artist_sort, derive_sort_name("Main Artist"));

        let multi = format_naming_pattern("{multi_artist?{artist}| - }{title}", &tag);
        assert_eq!(multi, "Main Artist - Title");
    }

    #[test]
    fn build_audio_tag_falls_back_to_parse_artists() {
        let row = rename_track("Main Artist feat. Guest", "", &[]);
        let tag = build_audio_tag(&row);
        assert_eq!(tag.album_artist, "");
        assert_eq!(tag.artists.len(), 3);
        assert_eq!(
            format_naming_pattern("{album_artist??{artist}}/", &tag),
            "Main Artist feat. Guest/"
        );
    }

    #[test]
    fn dest_from_root_empty_name_uses_fallback_stem() {
        let row = rename_track("Artist", "", &["Artist"]);
        let tag = build_audio_tag(&row);
        let path = dest_from_root("/music", "{album_artist}", &tag, "flac", "raw");
        assert_eq!(path.to_string_lossy(), "/music/raw.flac");
    }
}
