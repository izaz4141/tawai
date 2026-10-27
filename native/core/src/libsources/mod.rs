pub mod jellyfin;
pub mod local;
pub mod recommendation;
pub mod tawai;

pub use recommendation::{ALL_RECOMMENDATION_SOURCES, ApiType, RecommendationSource};

use std::collections::HashSet;
use std::ops::Deref;
use std::path::Path;
use std::sync::Weak;

use anyhow::{Result, anyhow};

use crate::app_context::AppContext;
use crate::audio::scan::LogSender;
use crate::audio::tags::AudioTag;
use crate::db::database::DatabasePool;
use crate::utils::config::AppConfig;

#[derive(Debug, Clone)]
pub struct ParsedTrack {
    pub tag: AudioTag,
    pub file_path: String,
    pub file_hash: Option<String>,
    pub duration_secs: f64,
    pub sample_rate: Option<u32>,
    pub bitrate: Option<u32>,
    pub file_size: u64,
}

impl Deref for ParsedTrack {
    type Target = AudioTag;
    fn deref(&self) -> &Self::Target {
        &self.tag
    }
}

/// One path a source wants scanned, paired with the source-owned identity when
/// the source can supply one. Tawai fills `id` with the remote track id; the
/// other parsers leave it empty and rely on path identity.
#[derive(Debug, Clone)]
pub struct EnumeratedTrack {
    pub id: Option<String>,
    pub path: String,
}

impl EnumeratedTrack {
    pub fn path_only(path: impl Into<String>) -> Self {
        Self {
            id: None,
            path: path.into(),
        }
    }

    pub fn with_id(id: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            path: path.into(),
        }
    }
}

fn path_only_tracks(paths: Vec<String>) -> Vec<EnumeratedTrack> {
    paths.into_iter().map(EnumeratedTrack::path_only).collect()
}

/// Per-source scan delta: which paths to insert, which DB entries are stale, and
/// which existing rows must be dropped before their replacement is inserted.
#[derive(Debug, Clone, Default)]
pub struct ScanDiff {
    /// Enumerate (tag-parse) and insert: new tracks plus any to re-provision.
    pub to_scan: Vec<String>,
    /// DB entries whose backing material no longer exists for this source,
    /// keyed by file path. Only parsers without federated ids use this: a path
    /// is not unique across sources, so an id-keyed parser must fill
    /// `to_delete_ids` instead.
    pub to_delete: Vec<String>,
    /// Stale rows to drop by track id. A track id is unique, so this is the
    /// only unambiguous key when a path may be shared or reused.
    pub to_delete_ids: HashSet<String>,
    /// Track ids whose stale row must be dropped before the re-provisioned
    /// track is inserted under the same id.
    pub replace_ids: HashSet<String>,
    /// Remote ids already owned by a different local source as `(id, path)`.
    /// They are reported instead of downloaded so one id never maps to two
    /// local files.
    pub conflicts: Vec<(String, String)>,
}

/// Deserialize an HTTP response body, embedding the HTTP status, byte count,
/// the serde detail (which names the offending field) and a truncated body
/// preview in the error so the real cause reaches the scan pipeline's log
/// channel verbatim instead of a bare `error decoding response body`.
pub(crate) fn decode_json<T: serde::de::DeserializeOwned>(
    what: &str,
    status: reqwest::StatusCode,
    bytes: &[u8],
) -> Result<T> {
    serde_json::from_slice(bytes).map_err(|e| {
        let preview = bytes
            .iter()
            .take(300)
            .map(|&b| {
                (b.is_ascii_graphic() || b == b' ')
                    .then_some(b as char)
                    .unwrap_or('.')
            })
            .collect::<String>();
        anyhow!(
            "error decoding response body for {what} ({status}, {} bytes): {e}; body preview: {preview}",
            bytes.len()
        )
    })
}

/// Resolves the first reachable URL from a source's ordered URL list. Caches
/// the result so subsequent calls skip probing. If the cached URL becomes
/// unreachable, re-iterates all URLs (up to 5 full rounds) before failing.
/// Never falls back to `.first()`.
pub struct SourceUrlResolver {
    cached: Option<String>,
}

impl SourceUrlResolver {
    pub fn new() -> Self {
        Self { cached: None }
    }

    pub fn clear_cache(&mut self) {
        self.cached = None;
    }

    /// Strict local reachability check: the root must be a directory and, when
    /// a `file_path` is given, that file must actually live inside the root.
    fn local_check(path: &Path, file_path: Option<&str>) -> bool {
        if !path.is_dir() {
            return false;
        }
        match file_path {
            Some(fp) => {
                let p = Path::new(fp);
                p.starts_with(path) && p.is_file()
            }
            None => true,
        }
    }

    /// Probe whether a single URL is reachable, classified by URL scheme.
    /// `tawai://` URLs are probed against the remote server's version endpoint;
    /// `file://` and bare path URLs are inspected on the local filesystem
    /// (strict containment when `file_path` is given); `http(s)://` URLs are
    /// probed with an HTTP HEAD against the server.
    async fn is_reachable(
        url: &str,
        client: Option<&reqwest::Client>,
        file_path: Option<&str>,
    ) -> bool {
        let trimmed = url.trim();
        if trimmed.starts_with("tawai://") {
            return match client {
                Some(c) => tawai::url_reachable(c, trimmed).await,
                None => false,
            };
        }
        match reqwest::Url::parse(trimmed) {
            Ok(parsed) if parsed.scheme() == "file" => match parsed.to_file_path() {
                Ok(path) => Self::local_check(&path, file_path),
                Err(_) => false,
            },
            Ok(parsed) if matches!(parsed.scheme(), "http" | "https") => match client {
                Some(c) => c.head(url).send().await.is_ok(),
                None => false,
            },
            Ok(_) => client.is_some(),
            Err(_) => Self::local_check(Path::new(url), file_path),
        }
    }

    pub async fn resolve(
        &mut self,
        urls: &[String],
        client: Option<&reqwest::Client>,
        file_path: Option<&str>,
    ) -> Result<String> {
        const MAX_RETRIES: u32 = 5;
        for _ in 0..MAX_RETRIES {
            if let Some(ref url) = self.cached {
                if Self::is_reachable(url, client, file_path).await {
                    return Ok(url.clone());
                }
                self.cached = None;
            }
            for url in urls {
                if Self::is_reachable(url, client, file_path).await {
                    self.cached = Some(url.clone());
                    return Ok(url.clone());
                }
            }
        }
        anyhow::bail!("No reachable URL found after {} attempts", MAX_RETRIES)
    }
}

pub enum SourceParser {
    Local,
    Jellyfin(jellyfin::JellyfinParser),
    Tawai(tawai::TawaiParser),
    Recommendation(RecommendationSource),
}

impl SourceParser {
    pub async fn enumerate_tracks(
        &self,
        pool: &DatabasePool,
        url: &str,
        urls: &[String],
        master_key: &str,
    ) -> Result<Vec<EnumeratedTrack>> {
        match self {
            SourceParser::Local => local::enumerate_paths(url).map(path_only_tracks),
            SourceParser::Jellyfin(p) => p.enumerate_paths(url).await.map(path_only_tracks),
            SourceParser::Tawai(p) => p.enumerate_tracks(url, urls).await,
            SourceParser::Recommendation(rec) => {
                recommendation::enumerate_paths(pool, rec, master_key)
                    .await
                    .map(path_only_tracks)
            }
        }
    }

    pub async fn scan_paths(
        &self,
        url: &str,
        urls: &[String],
        paths: &[String],
    ) -> Result<Vec<ParsedTrack>> {
        match self {
            SourceParser::Local => local::scan_paths(url, paths),
            SourceParser::Jellyfin(p) => p.scan_paths(url, paths).await,
            SourceParser::Tawai(p) => p.scan_paths(url, urls, paths).await,
            SourceParser::Recommendation(_) => Ok(vec![]),
        }
    }

    /// Compute this source's scan delta. `own_rows` are this source's
    /// `(track id, file path)` rows; `foreign_ids` are ids owned by other
    /// sources and must never be claimed. Detection is id-first where the
    /// parser supplies ids (tawai) and scoped to `own_rows` otherwise.
    pub async fn detect_difference(
        &self,
        entries: &[EnumeratedTrack],
        own_rows: &[(String, String)],
        foreign_ids: &HashSet<String>,
    ) -> Result<ScanDiff> {
        match self {
            SourceParser::Local | SourceParser::Jellyfin(_) => {
                let enumerated: HashSet<String> = entries.iter().map(|e| e.path.clone()).collect();
                let db_paths: HashSet<String> =
                    own_rows.iter().map(|(_, path)| path.clone()).collect();
                local::detect_difference(&enumerated, &db_paths)
            }
            SourceParser::Tawai(p) => p.detect_difference(entries, own_rows, foreign_ids).await,
            SourceParser::Recommendation(_) => {
                // Synced, not scanned: nothing to diff.
                Ok(ScanDiff::default())
            }
        }
    }

    pub async fn scan_file(
        &self,
        url: &str,
        urls: &[String],
        file_path: &str,
        track_id: Option<&str>,
        log_tx: &Option<LogSender>,
    ) -> Result<ParsedTrack> {
        match self {
            SourceParser::Local => local::scan_file(Path::new(file_path)),
            SourceParser::Jellyfin(p) => p.scan_file(url, file_path, log_tx).await,
            SourceParser::Tawai(p) => p.scan_file(url, urls, file_path, track_id, log_tx).await,
            SourceParser::Recommendation(_) => {
                anyhow::bail!("recommendation sources are synced, not scanned")
            }
        }
    }

    pub async fn resolve_stream_url(
        &self,
        pool: &DatabasePool,
        file_path: &str,
        url: &str,
        urls: &[String],
        client: &reqwest::Client,
        cfg: Option<&AppConfig>,
    ) -> Result<(String, Vec<(String, String)>)> {
        match self {
            SourceParser::Local => Ok((file_path.to_string(), vec![])),
            SourceParser::Jellyfin(p) => p.resolve_stream_url(file_path, url).await,
            SourceParser::Tawai(p) => {
                p.resolve_stream_url(pool, file_path, url, urls, client, cfg)
                    .await
            }
            SourceParser::Recommendation(_) => {
                recommendation::resolve_stream_url(pool, file_path, client, cfg).await
            }
        }
    }

    /// Delete a track's materials. `mirror_remote` controls whether the delete
    /// is propagated back to the source's remote server: user-initiated deletes
    /// mirror (`true`), while automated scan-time duplicate cleanup must only
    /// touch local state (`false`) so it can never destroy a shared remote copy.
    pub async fn delete(
        &self,
        pool: &DatabasePool,
        file_path: &str,
        url: &str,
        urls: &[String],
        mirror_remote: bool,
    ) -> Result<()> {
        match self {
            SourceParser::Local => local::delete_file(file_path),
            SourceParser::Jellyfin(p) => p.delete(file_path, url, mirror_remote).await,
            SourceParser::Tawai(p) => p.delete(pool, file_path, url, urls, mirror_remote).await,
            SourceParser::Recommendation(_) => recommendation::delete(pool, file_path).await,
        }
    }

    pub async fn download(
        &self,
        pool: &DatabasePool,
        file_path: &str,
        dest_path: &str,
        client: &reqwest::Client,
        cfg: &AppConfig,
        user_id: &str,
        extra: Option<&str>,
        master_key: &str,
        context: Weak<AppContext>,
    ) -> Result<String> {
        match self {
            SourceParser::Local | SourceParser::Jellyfin(_) | SourceParser::Tawai(_) => {
                anyhow::bail!("download not supported for this source type")
            }
            SourceParser::Recommendation(_) => {
                recommendation::download(
                    pool, file_path, dest_path, client, cfg, user_id, extra, master_key, context,
                )
                .await
            }
        }
    }

    pub async fn sync(
        &self,
        pool: &DatabasePool,
        source_id: &str,
        client: &reqwest::Client,
        token: &str,
        user_name: &str,
    ) -> Result<(u32, u32)> {
        match self {
            SourceParser::Local | SourceParser::Jellyfin(_) | SourceParser::Tawai(_) => Ok((0, 0)),
            SourceParser::Recommendation(rec) => {
                recommendation::sync(pool, rec, source_id, client, token, user_name).await
            }
        }
    }
}

/// Build the parser for a source type.
///
/// `context` is the live app context handle: parsers that need the global
/// naming pattern read it from the config at every use. Sources that don't
/// ignore it.
pub fn get_parser(
    source_type: &str,
    client: reqwest::Client,
    context: Weak<AppContext>,
) -> Option<SourceParser> {
    match source_type {
        "local" => Some(SourceParser::Local),
        "jellyfin" => Some(SourceParser::Jellyfin(jellyfin::JellyfinParser::new(
            client,
        ))),
        "tawai" => Some(SourceParser::Tawai(tawai::TawaiParser::new(
            client, context,
        ))),
        _ => RecommendationSource::from_key(source_type)
            .map(|rec| SourceParser::Recommendation(*rec)),
    }
}

/// Whether a source's media can be edited in place (rename, move, metadata
/// edits). Local sources and `tawai` backups are real files on this machine, so
/// they are editable; anything served remotely (jellyfin, recommendation
/// sources) is not. Single source of truth for this decision across the
/// codebase.
pub fn is_editable(source_type: &str) -> bool {
    matches!(source_type, "local" | "tawai")
}
