//! Filesystem helpers used by remote clients to browse server-side folders.
//!
//! These are intentionally minimal and read-only: they only enumerate
//! directories and normalize paths. They never create, modify, or delete
//! anything on disk.

use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use utoipa::ToSchema;

/// A single entry in a directory listing.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FsEntry {
    /// Entry name (last path segment).
    pub name: String,
    /// Absolute path to this entry.
    pub path: String,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// File size in bytes for regular files; `None` for directories.
    pub size: Option<u64>,
}

/// The result of listing a directory.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FsListing {
    /// The canonical directory that was listed.
    pub path: String,
    /// Parent directory, or `None` when at the filesystem root.
    pub parent: Option<String>,
    /// Directory entries (directories first, then alphabetically). File
    /// entries are not included; this is a folder browser only.
    pub entries: Vec<FsEntry>,
}

/// Normalizes `path` to an absolute, canonical path.
///
/// The path must exist and be a directory. Returns an error when the path is
/// missing or cannot be resolved, otherwise the canonical form used for
/// browsering (and returned in listings).
pub fn resolve_path(path: &str) -> Result<String> {
    let expanded = shellexpand_full_path(path);
    let canonical = fs::canonicalize(&expanded)?;
    let as_string = canonical.to_string_lossy().into_owned();
    if !canonical.is_dir() {
        anyhow::bail!("{as_string} is not a directory");
    }
    Ok(as_string)
}

/// Lists the immediate subdirectories of `path`, sorted directories-first
/// then alphabetically. `path` is normalized via [`resolve_path`] first.
pub fn list_dir(path: &str) -> Result<FsListing> {
    let canonical = resolve_path(path)?;
    let canonical_path = Path::new(&canonical);

    let parent = canonical_path
        .parent()
        .map(|p| p.to_string_lossy().into_owned());

    let mut entries = Vec::new();
    let read = fs::read_dir(canonical_path)?;
    for item in read {
        let item = match item {
            Ok(i) => i,
            Err(_) => continue,
        };
        let entry_path = item.path();
        let name = item.file_name().to_string_lossy().into_owned();
        let meta = match fs::symlink_metadata(&entry_path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let is_dir = meta.is_dir();
        if !is_dir {
            // Folder browser only: skip regular files and symlinks to files.
            continue;
        }
        let size = if meta.is_file() {
            Some(meta.len())
        } else {
            None
        };
        entries.push(FsEntry {
            name,
            path: entry_path.to_string_lossy().into_owned(),
            is_dir,
            size,
        });
    }

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(FsListing {
        path: canonical,
        parent,
        entries,
    })
}

/// Expands a leading tilde (`~` or `~/`) to the current user's home directory,
/// leaving absolute and relative paths unchanged. Relative paths are made
/// absolute against the current working directory.
fn shellexpand_full_path(path: &str) -> PathBuf {
    let trimmed = path.trim();
    if trimmed == "~" {
        return home_dir().unwrap_or_else(|| PathBuf::from("/"));
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        return home_dir()
            .map(|h| h.join(rest))
            .unwrap_or_else(|| PathBuf::from(trimmed));
    }
    let p = PathBuf::from(trimmed);
    if p.is_absolute() {
        p
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(p),
            Err(_) => p,
        }
    }
}

/// Resolves the current user's home directory by checking common environment
/// variables. Returns `None` when it cannot be determined.
fn home_dir() -> Option<PathBuf> {
    for key in ["HOME", "USERPROFILE"] {
        if let Ok(v) = std::env::var(key)
            && !v.is_empty()
        {
            return Some(PathBuf::from(v));
        }
    }
    None
}
