use std::path::{Path, PathBuf};

use anyhow::Result;
use sqlx::Row;

use crate::audio::tags::AudioTag;
use crate::db::database::DatabasePool;
use crate::signals::tools::{NamingViolation, RenamePreview};

/// Check if a value is non-empty (for conditional truthiness).
fn is_truthy(val: &str) -> bool {
    !val.is_empty()
}

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

/// Rename an audio file based on a naming pattern.
/// Creates parent directories if they don't exist.
/// Returns the new file path.
pub fn rename_audio_file(old_path: &Path, pattern: &str, tag: &AudioTag) -> Result<PathBuf> {
    let ext = old_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp3");
    let dir = old_path.parent().unwrap_or(Path::new("."));
    let new_name = format_naming_pattern(pattern, tag);
    let new_path = dir.join(format!("{}.{}", new_name, ext));

    if new_path == old_path {
        return Ok(new_path);
    }

    if let Some(parent) = new_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::rename(old_path, &new_path)?;

    Ok(new_path)
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
        .unwrap_or("track");
    let base_name = match pattern {
        Some(p) => {
            let formatted = format_naming_pattern(p, tag);
            if formatted.trim().is_empty() {
                stem.to_string()
            } else {
                formatted
            }
        }
        None => stem.to_string(),
    };
    let new_path = Path::new(source_url).join(format!("{}.{}", base_name, ext));
    if new_path == old_path {
        return Ok(new_path);
    }
    if let Some(parent) = new_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(old_path, &new_path)?;
    Ok(new_path)
}

pub async fn batch_rename_preview(
    file_paths: &[String],
    pattern: &str,
) -> Result<Vec<RenamePreview>> {
    let mut results = Vec::new();

    for path_str in file_paths {
        let path = Path::new(path_str);
        match crate::audio::tags::read_audio_tags(path) {
            Ok((tag, _duration, _track_num, _disc_num)) => {
                let parent = path.parent().unwrap_or(Path::new("."));
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("mp3");
                let new_name = format_naming_pattern(pattern, &tag);
                let expected_path = parent.join(format!("{}.{}", new_name, ext));
                results.push(RenamePreview {
                    file_path: path_str.clone(),
                    expected_path: expected_path.to_string_lossy().to_string(),
                });
            }
            Err(e) => {
                results.push(RenamePreview {
                    file_path: path_str.clone(),
                    expected_path: format!("<error: {}>", e),
                });
            }
        }
    }

    Ok(results)
}

/// Apply a batch rename.
pub async fn batch_rename_apply(
    file_paths: &[String],
    pattern: &str,
) -> Result<Vec<RenamePreview>> {
    let mut results = Vec::new();

    for path_str in file_paths {
        let path = Path::new(path_str);
        match crate::audio::tags::read_audio_tags(path) {
            Ok((tag, _duration, _track_num, _disc_num)) => {
                match rename_audio_file(path, pattern, &tag) {
                    Ok(new_path) => {
                        results.push(RenamePreview {
                            file_path: path_str.clone(),
                            expected_path: new_path.to_string_lossy().to_string(),
                        });
                    }
                    Err(e) => {
                        results.push(RenamePreview {
                            file_path: path_str.clone(),
                            expected_path: format!("<error: {}>", e),
                        });
                    }
                }
            }
            Err(e) => {
                results.push(RenamePreview {
                    file_path: path_str.clone(),
                    expected_path: format!("<error: {}>", e),
                });
            }
        }
    }

    Ok(results)
}

/// Preview renaming using database metadata (fast, no file I/O).
pub async fn batch_rename_preview_from_db(
    pool: &DatabasePool,
    source_id: Option<&str>,
    pattern: &str,
) -> Result<Vec<RenamePreview>> {
    match pool {
        DatabasePool::Sqlite(p) => preview_from_db_sq(p, source_id, pattern).await,
        DatabasePool::Postgres(p) => preview_from_db_pg(p, source_id, pattern).await,
    }
}

async fn preview_from_db_sq(
    pool: &sqlx::SqlitePool,
    source_id: Option<&str>,
    pattern: &str,
) -> Result<Vec<RenamePreview>> {
    let rows = if let Some(sid) = source_id {
        sqlx::query(
            r#"SELECT t.file_path, t.title, COALESCE(ar.name, '') AS artist_name,
                      COALESCE(a.title, '') AS album_title,
                      t.track_num, t.disc_num, a.date, a.disambiguation,
                      COALESCE(ar.name, '') AS album_artist,
                      a.total_discs
               FROM tracks t
               JOIN albums a ON t.album_id = a.id
               JOIN artists ar ON t.artist_id = ar.id
               JOIN library_sources ls ON t.source_id = ls.id
               WHERE t.source_id = ? AND ls.source_type NOT LIKE 'recommendation:%'
               ORDER BY t.file_path"#,
        )
        .bind(sid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"SELECT t.file_path, t.title, COALESCE(ar.name, '') AS artist_name,
                      COALESCE(a.title, '') AS album_title,
                      t.track_num, t.disc_num, a.date, a.disambiguation,
                      COALESCE(ar.name, '') AS album_artist,
                      a.total_discs
               FROM tracks t
               JOIN albums a ON t.album_id = a.id
               JOIN artists ar ON t.artist_id = ar.id
               JOIN library_sources ls ON t.source_id = ls.id
               WHERE ls.source_type NOT LIKE 'recommendation:%'
               ORDER BY t.file_path"#,
        )
        .fetch_all(pool)
        .await?
    };

    let mut previews = Vec::new();
    for row in rows {
        let file_path: String = row.get("file_path");
        let path = Path::new(&file_path);
        let parent = path.parent().unwrap_or(Path::new("."));
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("mp3");

        let tag = AudioTag {
            title: row.get("title"),
            artist: row.get("artist_name"),
            album_artist: row.get("album_artist"),
            album: row.get("album_title"),
            track_number: row.get::<Option<i32>, _>("track_num").unwrap_or(0),
            disc_number: row.get::<Option<i32>, _>("disc_num").unwrap_or(1),
            release_date: row.get("date"),
            album_disambiguation: row.get("disambiguation"),
            total_discs: row.get::<Option<i32>, _>("total_discs").unwrap_or(0),
            artist_sort: String::new(),
            artists: vec![],
            album_artist_sort: String::new(),
            album_artists: vec![],
            genres: vec![],
            mbid_recording: None,
            mbid_artist: None,
            mbid_release_artist: None,
            mbid_release: None,
            acoust_id: None,
            acoust_id_fingerprint: None,
            lyrics: None,
            cover: None,
            track_gain: None,
            track_peak: None,
        };

        let new_name = format_naming_pattern(pattern, &tag);
        let expected_path = parent.join(format!("{}.{}", new_name, ext));
        previews.push(RenamePreview {
            file_path,
            expected_path: expected_path.to_string_lossy().to_string(),
        });
    }

    Ok(previews)
}

async fn preview_from_db_pg(
    pool: &sqlx::PgPool,
    source_id: Option<&str>,
    pattern: &str,
) -> Result<Vec<RenamePreview>> {
    let rows = if let Some(sid) = source_id {
        sqlx::query(
            r#"SELECT t.file_path, t.title, COALESCE(ar.name, '') AS artist_name,
                      COALESCE(a.title, '') AS album_title,
                      t.track_num, t.disc_num, a.date, a.disambiguation,
                      COALESCE(ar.name, '') AS album_artist,
                      a.total_discs
               FROM tracks t
               JOIN albums a ON t.album_id = a.id
               JOIN artists ar ON t.artist_id = ar.id
               JOIN library_sources ls ON t.source_id = ls.id
               WHERE t.source_id = $1
                 AND ls.source_type NOT LIKE 'recommendation:%'
               ORDER BY t.file_path"#,
        )
        .bind(sid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"SELECT t.file_path, t.title, COALESCE(ar.name, '') AS artist_name,
                      COALESCE(a.title, '') AS album_title,
                      t.track_num, t.disc_num, a.date, a.disambiguation,
                      COALESCE(ar.name, '') AS album_artist,
                      a.total_discs
               FROM tracks t
               JOIN albums a ON t.album_id = a.id
               JOIN artists ar ON t.artist_id = ar.id
               JOIN library_sources ls ON t.source_id = ls.id
               WHERE ls.source_type NOT LIKE 'recommendation:%'
               ORDER BY t.file_path"#,
        )
        .fetch_all(pool)
        .await?
    };

    let mut previews = Vec::new();
    for row in rows {
        let file_path: String = row.get("file_path");
        let path = Path::new(&file_path);
        let parent = path.parent().unwrap_or(Path::new("."));
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("mp3");

        let tag = AudioTag {
            title: row.get("title"),
            artist: row.get("artist_name"),
            album_artist: row.get("album_artist"),
            album: row.get("album_title"),
            track_number: row.get::<Option<i32>, _>("track_num").unwrap_or(0),
            disc_number: row.get::<Option<i32>, _>("disc_num").unwrap_or(1),
            release_date: row.get("date"),
            album_disambiguation: row.get("disambiguation"),
            total_discs: row.get::<Option<i32>, _>("total_discs").unwrap_or(0),
            artist_sort: String::new(),
            artists: vec![],
            album_artist_sort: String::new(),
            album_artists: vec![],
            genres: vec![],
            mbid_recording: None,
            mbid_artist: None,
            mbid_release_artist: None,
            mbid_release: None,
            acoust_id: None,
            acoust_id_fingerprint: None,
            lyrics: None,
            cover: None,
            track_gain: None,
            track_peak: None,
        };

        let new_name = format_naming_pattern(pattern, &tag);
        let expected_path = parent.join(format!("{}.{}", new_name, ext));
        previews.push(RenamePreview {
            file_path,
            expected_path: expected_path.to_string_lossy().to_string(),
        });
    }

    Ok(previews)
}

/// Check which files in the library don't follow the naming convention.
pub async fn check_naming_convention(
    pool: &DatabasePool,
    source_id: Option<&str>,
    pattern: &str,
) -> Result<Vec<NamingViolation>> {
    match pool {
        DatabasePool::Sqlite(p) => check_naming_convention_sq(p, source_id, pattern).await,
        DatabasePool::Postgres(p) => check_naming_convention_pg(p, source_id, pattern).await,
    }
}

async fn check_naming_convention_sq(
    pool: &sqlx::SqlitePool,
    source_id: Option<&str>,
    pattern: &str,
) -> Result<Vec<NamingViolation>> {
    let rows = if let Some(sid) = source_id {
        sqlx::query(
            r#"SELECT t.id, t.file_path, t.title, COALESCE(ar.name, '') AS artist_name,
                      COALESCE(a.title, '') AS album_title,
                      t.track_num, t.disc_num, a.date, a.disambiguation,
                      COALESCE(ar.name, '') AS album_artist,
                      a.total_discs
               FROM tracks t
               JOIN albums a ON t.album_id = a.id
               JOIN artists ar ON t.artist_id = ar.id
               JOIN library_sources ls ON t.source_id = ls.id
               WHERE t.source_id = ?
                 AND ls.source_type NOT LIKE 'recommendation:%'
               ORDER BY t.file_path"#,
        )
        .bind(sid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"SELECT t.id, t.file_path, t.title, COALESCE(ar.name, '') AS artist_name,
                      COALESCE(a.title, '') AS album_title,
                      t.track_num, t.disc_num, a.date, a.disambiguation,
                      COALESCE(ar.name, '') AS album_artist,
                      a.total_discs
               FROM tracks t
               JOIN albums a ON t.album_id = a.id
               JOIN artists ar ON t.artist_id = ar.id
               JOIN library_sources ls ON t.source_id = ls.id
               WHERE ls.source_type NOT LIKE 'recommendation:%'
               ORDER BY t.file_path"#,
        )
        .fetch_all(pool)
        .await?
    };

    let mut violations = Vec::new();

    for row in rows {
        let file_path: String = row.get("file_path");
        let path = Path::new(&file_path);
        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let tag = AudioTag {
            title: row.get("title"),
            artist: row.get("artist_name"),
            album_artist: row.get("album_artist"),
            album: row.get("album_title"),
            track_number: row.get::<Option<i32>, _>("track_num").unwrap_or(0),
            disc_number: row.get::<Option<i32>, _>("disc_num").unwrap_or(1),
            release_date: row.get("date"),
            album_disambiguation: row.get("disambiguation"),
            total_discs: row.get::<Option<i32>, _>("total_discs").unwrap_or(0),
            artist_sort: String::new(),
            artists: vec![],
            album_artist_sort: String::new(),
            album_artists: vec![],
            genres: vec![],
            mbid_recording: None,
            mbid_artist: None,
            mbid_release_artist: None,
            mbid_release: None,
            acoust_id: None,
            acoust_id_fingerprint: None,
            lyrics: None,
            cover: None,
            track_gain: None,
            track_peak: None,
        };

        let expected_name = format_naming_pattern(pattern, &tag);

        if file_stem != expected_name {
            violations.push(NamingViolation {
                file_path: file_path.clone(),
                file_name: file_stem,
                expected_name,
                track_id: row.get("id"),
            });
        }
    }

    Ok(violations)
}

async fn check_naming_convention_pg(
    pool: &sqlx::PgPool,
    source_id: Option<&str>,
    pattern: &str,
) -> Result<Vec<NamingViolation>> {
    let rows = if let Some(sid) = source_id {
        sqlx::query(
            r#"SELECT t.id, t.file_path, t.title, COALESCE(ar.name, '') AS artist_name,
                      COALESCE(a.title, '') AS album_title,
                      t.track_num, t.disc_num, a.date, a.disambiguation,
                      COALESCE(ar.name, '') AS album_artist,
                      a.total_discs
               FROM tracks t
               JOIN albums a ON t.album_id = a.id
               JOIN artists ar ON t.artist_id = ar.id
               JOIN library_sources ls ON t.source_id = ls.id
               WHERE t.source_id = $1
                 AND ls.source_type NOT LIKE 'recommendation:%'
               ORDER BY t.file_path"#,
        )
        .bind(sid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"SELECT t.id, t.file_path, t.title, COALESCE(ar.name, '') AS artist_name,
                      COALESCE(a.title, '') AS album_title,
                      t.track_num, t.disc_num, a.date, a.disambiguation,
                      COALESCE(ar.name, '') AS album_artist,
                      a.total_discs
               FROM tracks t
               JOIN albums a ON t.album_id = a.id
               JOIN artists ar ON t.artist_id = ar.id
               JOIN library_sources ls ON t.source_id = ls.id
               WHERE ls.source_type NOT LIKE 'recommendation:%'
               ORDER BY t.file_path"#,
        )
        .fetch_all(pool)
        .await?
    };

    let mut violations = Vec::new();

    for row in rows {
        let file_path: String = row.get("file_path");
        let path = Path::new(&file_path);
        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let tag = AudioTag {
            title: row.get("title"),
            artist: row.get("artist_name"),
            album_artist: row.get("album_artist"),
            album: row.get("album_title"),
            track_number: row.get::<Option<i32>, _>("track_num").unwrap_or(0),
            disc_number: row.get::<Option<i32>, _>("disc_num").unwrap_or(1),
            release_date: row.get("date"),
            album_disambiguation: row.get("disambiguation"),
            total_discs: row.get::<Option<i32>, _>("total_discs").unwrap_or(0),
            artist_sort: String::new(),
            artists: vec![],
            album_artist_sort: String::new(),
            album_artists: vec![],
            genres: vec![],
            mbid_recording: None,
            mbid_artist: None,
            mbid_release_artist: None,
            mbid_release: None,
            acoust_id: None,
            acoust_id_fingerprint: None,
            lyrics: None,
            cover: None,
            track_gain: None,
            track_peak: None,
        };

        let expected_name = format_naming_pattern(pattern, &tag);

        if file_stem != expected_name {
            violations.push(NamingViolation {
                file_path: file_path.clone(),
                file_name: file_stem,
                expected_name,
                track_id: row.get("id"),
            });
        }
    }

    Ok(violations)
}
