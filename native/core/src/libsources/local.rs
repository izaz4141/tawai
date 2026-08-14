use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use sha2::{Digest, Sha256};
use std::io::Read;
use walkdir::WalkDir;

use crate::audio::fingerprint::{compute_fingerprint, fingerprint_supported_format};
use crate::audio::tags;
use crate::libsources::ParsedTrack;
use crate::utils::logger;

pub fn walk_directory(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if !dir.is_dir() {
        return files;
    }

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if tags::supported_lofty_format(ext) {
                files.push(path.to_path_buf());
            }
        }
    }

    files.sort();
    files
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn scan_file(path: &Path) -> Result<ParsedTrack> {
    let (mut tag, duration_secs, sample_rate, bitrate) = tags::read_audio_tags(path)?;
    let file_hash = hash_file(path)?;
    let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    if tag.acoust_id_fingerprint.is_none() {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if fingerprint_supported_format(ext) {
            if let Ok(result) = compute_fingerprint(path) {
                tag.acoust_id_fingerprint = Some(result.fingerprint);
                if let Err(e) = tags::write_audio_tags(path, &tag) {
                    logger::warn(&format!(
                        "Failed to write acoust_id_fingerprint to {}: {}",
                        path.display(),
                        e
                    ));
                }
            }
        }
    }

    Ok(ParsedTrack {
        tag,
        file_path: path.to_string_lossy().to_string(),
        file_hash: Some(file_hash),
        duration_secs,
        sample_rate,
        bitrate,
        file_size,
    })
}

/// Physically removes a local audio file. Missing files are treated as
/// already-deleted (idempotent).
pub fn delete_file(path: &str) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Cheap enumeration: walk directory and return sorted audio file paths.
/// No tag reading, no hashing.
pub fn enumerate_paths(url: &str) -> Result<Vec<String>> {
    let dir = Path::new(url);
    Ok(walk_directory(dir)
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}

/// Expensive: parse tags + hashes only for the given subset of paths.
/// `paths` must be a subset of what `enumerate_paths` would return.
pub fn scan_paths(url: &str, paths: &[String]) -> Result<Vec<ParsedTrack>> {
    let wanted: HashSet<&str> = paths.iter().map(|s| s.as_str()).collect();
    let dir = Path::new(url);
    let mut tracks = Vec::new();
    for file_path in walk_directory(dir) {
        let path_str = file_path.to_string_lossy().to_string();
        if !wanted.contains(path_str.as_str()) {
            continue;
        }
        match scan_file(&file_path) {
            Ok(track) => tracks.push(track),
            Err(e) => {
                logger::warn(&format!("Failed to scan {}: {}", file_path.display(), e));
            }
        }
    }
    Ok(tracks)
}
