use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use futures::StreamExt;
use tokio::io::AsyncWriteExt;

use crate::audio::tags::{AudioTag, derive_sort_name, parse_artists};
use crate::db::account::DEFAULT_USERNAME;
use crate::db::library_source;
use crate::db::user_settings;
use crate::db::database::DatabasePool;
use crate::libsources::{local, ParsedTrack};
use crate::signals::discovery::{JellyfinLibraryInfo, ServerTestResult};
use crate::signals::library::{
    ListLibrarySourcesResponse, ListTracksResponse, TrackInfo,
};
use crate::tools::rename::{dest_from_root, DEFAULT_PATTERN};
use crate::utils::config::AppConfig;
use crate::utils::logger;

// ── Remote connection ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RemoteConn {
    pub http_base: String,
    pub api_key: String,
    /// Set when the URL carries `?source_id=<id>` (scan/stream time). The
    /// connection-test flow lists sources from a server before the id is known.
    pub source_id: Option<String>,
}

/// Parse `tawai://host:port@api_key?source_id=<id>&scheme=http|https` URL.
/// `source_id` is optional; `scheme` is required.
pub fn parse_tawai_url(url: &str) -> Result<RemoteConn> {
    let without = url
        .strip_prefix("tawai://")
        .context("tawai URL must start with tawai://")?;
    let (authority, query) = match without.find('?') {
        Some(i) => (&without[..i], &without[i + 1..]),
        None => (without, ""),
    };
    let (hostport, api_key) = authority
        .split_once('@')
        .map(|(hp, key)| (hp, key.to_string()))
        .unwrap_or((authority, String::new()));
    let mut source_id: Option<String> = None;
    let mut scheme: Option<String> = None;
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        match (kv.next(), kv.next()) {
            (Some("source_id"), Some(id)) if !id.is_empty() => source_id = Some(id.to_string()),
            (Some("scheme"), Some(s)) if !s.is_empty() => {
                let s = s.to_ascii_lowercase();
                if s == "http" || s == "https" {
                    scheme = Some(s);
                }
            }
            _ => {}
        }
    }
    let scheme = scheme.context("tawai URL must contain ?scheme=http|https")?;
    Ok(RemoteConn {
        http_base: format!("{}://{}", scheme, hostport),
        api_key,
        source_id,
    })
}

/// Check if a remote tawai server is reachable (cached per base URL).
async fn is_remote_reachable(client: &reqwest::Client, conn: &RemoteConn) -> bool {
    let now = Instant::now();
    {
        let cache = REACH_CACHE.lock().unwrap();
        if let Some(entry) = cache.get(&conn.http_base) {
            if entry.checked_at.elapsed() < REACH_TTL {
                return entry.reachable;
            }
        }
    }
    let url = format!("{}/api/tawai/system/status", conn.http_base);
    let reachable = client
        .get(&url)
        .header("X-API-Key", &conn.api_key)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    {
        let mut cache = REACH_CACHE.lock().unwrap();
        cache.insert(
            conn.http_base.clone(),
            ReachEntry {
                reachable,
                checked_at: now,
            },
        );
    }
    reachable
}

/// Reachability probe used by `SourceUrlResolver` for `tawai://` scheme URLs.
pub async fn url_reachable(client: &reqwest::Client, url: &str) -> bool {
    match parse_tawai_url(url) {
        Ok(conn) => is_remote_reachable(client, &conn).await,
        Err(_) => false,
    }
}

/// Test a set of `tawai://` URLs (home + remote fallbacks). Each URL is
/// verified with a single request: list the library sources accessible to the
/// API key. A successful list doubles as both the reachability probe and the
/// API key validity check. The first successful server's library sources are
/// returned so the caller can pick which to add.
pub async fn test_remote_urls(
    client: &reqwest::Client,
    urls: &[String],
) -> (Vec<JellyfinLibraryInfo>, Vec<ServerTestResult>) {
    let mut libraries = Vec::new();
    let mut results = Vec::with_capacity(urls.len());
    for url in urls {
        let conn = match parse_tawai_url(url) {
            Ok(conn) => conn,
            Err(e) => {
                results.push(ServerTestResult {
                    url: url.clone(),
                    reachable: false,
                    track_count: 0,
                    error: Some(format!("invalid tawai URL: {e}")),
                });
                continue;
            }
        };
        match fetch_remote_libraries(client, &conn).await {
            Ok(list) => {
                if libraries.is_empty() {
                    libraries = list;
                }
                results.push(ServerTestResult {
                    url: url.clone(),
                    reachable: true,
                    track_count: 0,
                    error: None,
                });
            }
            Err(e) => results.push(ServerTestResult {
                url: url.clone(),
                reachable: false,
                track_count: 0,
                error: Some(e.to_string()),
            }),
        }
    }
    (libraries, results)
}

/// List the library sources accessible to the API key on the remote server.
pub async fn fetch_remote_libraries(
    client: &reqwest::Client,
    conn: &RemoteConn,
) -> Result<Vec<JellyfinLibraryInfo>> {
    let url = format!("{}/api/tawai/library/sources", conn.http_base);
    let resp = client
        .get(&url)
        .header("X-API-Key", &conn.api_key)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!(
            "tawai list sources failed with status {}: {}",
            status,
            text
        );
    }
    let data: ListLibrarySourcesResponse = resp.json().await?;
    Ok(data
        .sources
        .into_iter()
        .map(|s| JellyfinLibraryInfo {
            id: s.id,
            name: s.name,
        })
        .collect())
}

/// Pick the first reachable tawai:// URL from the list.
pub async fn pick_remote(urls: &[String], client: &reqwest::Client) -> Option<RemoteConn> {
    for url in urls {
        if !url.starts_with("tawai://") {
            continue;
        }
        if let Ok(conn) = parse_tawai_url(url) {
            if is_remote_reachable(client, &conn).await {
                return Some(conn);
            }
        }
    }
    None
}

/// The first non-tawai:// URL in the list is the local backup root.
fn local_root(urls: &[String]) -> Option<&str> {
    urls.iter()
        .find(|u| !u.starts_with("tawai://"))
        .map(|s| s.as_str())
}

// ── Remote track list fetch + cache ───────────────────────────────────

#[derive(Debug, Clone)]
pub struct RemoteTrack {
    pub id: String,
    pub title: String,
    pub artists_string: String,
    pub album_title: String,
    pub track_num: Option<i32>,
    pub disc_num: Option<i32>,
    pub duration_secs: Option<f64>,
    pub file_size: Option<i64>,
    pub bitrate: Option<i32>,
    pub mbid_recording: Option<String>,
    pub release_date: Option<String>,
    pub genres: Vec<String>,
    pub remote_file_path: String,
}

impl From<TrackInfo> for RemoteTrack {
    fn from(t: TrackInfo) -> Self {
        Self {
            id: t.id,
            title: t.title,
            artists_string: t.artists_string,
            album_title: t.album_title,
            track_num: t.track_num,
            disc_num: t.disc_num,
            duration_secs: t.duration_secs,
            file_size: t.file_size,
            bitrate: t.bitrate,
            mbid_recording: t.mbid_recording,
            release_date: t.release_date,
            genres: t.genres,
            remote_file_path: t.file_path,
        }
    }
}

pub async fn fetch_remote_tracks(
    client: &reqwest::Client,
    conn: &RemoteConn,
) -> Result<Vec<RemoteTrack>> {
    let source_id = conn.source_id.as_deref().context("source_id required")?;
    let url = format!(
        "{}/api/tawai/library/tracks/by-source/{}",
        conn.http_base, source_id
    );
    let resp = client
        .get(&url)
        .header("X-API-Key", &conn.api_key)
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("tawai list tracks failed with status {}", resp.status());
    }
    let body: ListTracksResponse = resp.json().await?;
    Ok(body.tracks.into_iter().map(RemoteTrack::from).collect())
}

// ── Process-local cache────────────────────────────────────────────────

struct CacheEntry {
    tracks: Vec<RemoteTrack>,
    fetched_at: Instant,
}

struct ReachEntry {
    reachable: bool,
    checked_at: Instant,
}

static REMOTE_CACHE: LazyLock<Mutex<HashMap<String, CacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static REACH_CACHE: LazyLock<Mutex<HashMap<String, ReachEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const CACHE_TTL: Duration = Duration::from_secs(60);

const REACH_TTL: Duration = Duration::from_secs(60);

pub async fn cached_remote_tracks(
    client: &reqwest::Client,
    conn: &RemoteConn,
) -> Result<Vec<RemoteTrack>> {
    let key = format!("{}|{}", conn.http_base, conn.source_id.as_deref().unwrap_or(""));
    {
        let cache = REMOTE_CACHE.lock().unwrap();
        if let Some(entry) = cache.get(&key) {
            if entry.fetched_at.elapsed() < CACHE_TTL {
                return Ok(entry.tracks.clone());
            }
        }
    }
    let tracks = fetch_remote_tracks(client, conn).await?;
    {
        let mut cache = REMOTE_CACHE.lock().unwrap();
        cache.insert(
            key,
            CacheEntry {
                tracks: tracks.clone(),
                fetched_at: Instant::now(),
            },
        );
    }
    Ok(tracks)
}

// ── AudioTag synthesis from remote metadata────────────────────────────

fn remote_track_to_tag(rt: &RemoteTrack) -> AudioTag {
    let artist = if rt.artists_string.is_empty() {
        "Unknown Artist".to_string()
    } else {
        rt.artists_string.clone()
    };
    let mut tag = AudioTag {
        title: rt.title.clone(),
        artist: artist.clone(),
        artists: parse_artists(&artist),
        album: rt.album_title.clone(),
        album_artist: artist.clone(),
        album_artists: parse_artists(&artist),
        genres: rt.genres.clone(),
        release_date: rt.release_date.clone(),
        track_number: rt.track_num.unwrap_or(0),
        disc_number: rt.disc_num.unwrap_or(1),
        mbid_recording: rt.mbid_recording.clone(),
        ..Default::default()
    };
    tag.artist_sort = derive_sort_name(&tag.artist);
    tag.album_artist_sort = derive_sort_name(&tag.album_artist);
    tag
}

/// Deterministic local backup path for a remote track.
fn dest_for(local_root: &str, naming_pattern: &str, rt: &RemoteTrack) -> PathBuf {
    let tag = remote_track_to_tag(rt);
    let ext = ext_from_path(&rt.remote_file_path);
    let fallback = format!("{} - {}", rt.artists_string, rt.title);
    dest_from_root(local_root, naming_pattern, &tag, ext, &fallback)
}

fn ext_from_path(path: &str) -> &str {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp3")
}

// ── Download a single track from remote ───────────────────────────────

async fn download_track(
    client: &reqwest::Client,
    conn: &RemoteConn,
    rt: &RemoteTrack,
    dest: &Path,
) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let stream_url = format!(
        "{}/api/tawai/playback/stream/{}",
        conn.http_base, rt.id
    );
    let mut resp = client
        .get(&stream_url)
        .header("X-API-Key", &conn.api_key)
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!(
            "tawai stream download failed for {} ({}): {}",
            rt.id,
            rt.title,
            resp.status()
        );
    }
    let mut file = tokio::fs::File::create(dest).await?;
    let mut total: u64 = 0;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        total += chunk.len() as u64;
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    if total == 0 {
        let _ = std::fs::remove_file(dest);
        anyhow::bail!("downloaded file is empty: {}", dest.display());
    }
    Ok(())
}

/// Map a local backup path back to the remote connection + track that backs it.
async fn resolve_remote_track(
    pool: &DatabasePool,
    client: &reqwest::Client,
    urls: &[String],
    local_path: &str,
) -> Result<(RemoteConn, RemoteTrack)> {
    let root = local_root(urls).unwrap_or("/tmp");
    let conn = pick_remote(urls, client).await.context("remote tawai unreachable")?;
    let tracks = cached_remote_tracks(client, &conn).await?;
    let pattern = user_settings::get_setting(pool, DEFAULT_USERNAME, "identify_naming_pattern")
        .await
        .unwrap_or_else(|| DEFAULT_PATTERN.to_string());
    let dest = Path::new(local_path);
    let rt = tracks
        .into_iter()
        .find(|rt| dest_for(root, &pattern, rt).as_path() == dest)
        .context("no matching remote track for local path")?;
    Ok((conn, rt))
}

// ── Parser struct──────────────────────────────────────────────────────

pub struct TawaiParser {
    pub client: reqwest::Client,
}

impl TawaiParser {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Enumerate paths: remote's authoritative list → deterministic local dest paths.
    /// Falls back to local dir walk when remote is unreachable (offline mode).
    pub async fn enumerate_paths(
        &self,
        pool: &DatabasePool,
        _url: &str,
        urls: &[String],
    ) -> Result<Vec<String>> {
        if let Some(conn) = pick_remote(urls, &self.client).await {
            let root = local_root(urls).unwrap_or("/tmp");
            let pattern =
                user_settings::get_setting(pool, DEFAULT_USERNAME, "identify_naming_pattern")
                    .await
                    .unwrap_or_else(|| DEFAULT_PATTERN.to_string());
            match cached_remote_tracks(&self.client, &conn).await {
                Ok(tracks) => {
                    return Ok(tracks
                        .iter()
                        .map(|rt| dest_for(root, &pattern, rt).to_string_lossy().to_string())
                        .collect());
                }
                Err(e) => {
                    // Invalid/stale API key or source_id: degrade to offline mode
                    // (scan local backups) instead of silently skipping the source.
                    logger::warn(&format!(
                        "tawai remote track list failed for {}: {} — falling back to local backup dir",
                        conn.http_base, e
                    ));
                }
            }
        }
        // Offline fallback: enumerate local backup dir
        if let Some(root) = local_root(urls) {
            return local::enumerate_paths(root);
        }
        Ok(vec![])
    }

    /// Bulk scan paths (unused by current scan pipeline but kept for completeness).
    pub async fn scan_paths(
        &self,
        pool: &DatabasePool,
        _url: &str,
        urls: &[String],
        paths: &[String],
    ) -> Result<Vec<ParsedTrack>> {
        let mut tracks = Vec::new();
        for p in paths {
            match self.scan_file(pool, _url, urls, p).await {
                Ok(t) => tracks.push(t),
                Err(e) => logger::warn(&format!("tawai scan_paths skip {}: {}", p, e)),
            }
        }
        Ok(tracks)
    }

    /// Scan a single file. If local backup exists, scan as local.
    /// If missing, download from remote then scan locally.
    pub async fn scan_file(
        &self,
        pool: &DatabasePool,
        _url: &str,
        urls: &[String],
        file_path: &str,
    ) -> Result<ParsedTrack> {
        let dest = Path::new(file_path);
        if dest.is_file() {
            return local::scan_file(dest);
        }
        // Local file missing — download from remote
        let (conn, rt) = resolve_remote_track(pool, &self.client, urls, file_path).await?;
        download_track(&self.client, &conn, &rt, dest).await?;
        local::scan_file(dest)
    }

    /// Resolve stream URL: local file if present, else remote stream.
    pub async fn resolve_stream_url(
        &self,
        pool: &DatabasePool,
        file_path: &str,
        _url: &str,
        urls: &[String],
        client: &reqwest::Client,
        _cfg: Option<&AppConfig>,
    ) -> Result<(String, Vec<(String, String)>)> {
        let dest = Path::new(file_path);
        if dest.is_file() {
            return Ok((file_path.to_string(), vec![]));
        }
        // Fallback: remote stream
        let (conn, rt) = resolve_remote_track(pool, client, urls, file_path).await?;
        let stream_url = format!("{}/api/tawai/playback/stream/{}", conn.http_base, rt.id);
        Ok((stream_url, vec![("X-API-Key".into(), conn.api_key)]))
    }

    /// Delete the local backup file and, when `mirror_remote` is set, mirror
    /// the deletion to the remote source (user-initiated deletes). Scan-time
    /// duplicate cleanup passes `mirror_remote = false` so the remote server's
    /// copy is never touched. The remote delete is best-effort: if it fails,
    /// the local backup is still removed and the next scan will re-fetch it
    /// while the remote still lists it.
    pub async fn delete(
        &self,
        pool: &DatabasePool,
        file_path: &str,
        _url: &str,
        urls: &[String],
        mirror_remote: bool,
    ) -> Result<()> {
        if mirror_remote {
            match resolve_remote_track(pool, &self.client, urls, file_path).await {
                Ok((conn, rt)) => {
                    let delete_url =
                        format!("{}/api/tawai/library/tracks/{}", conn.http_base, rt.id);
                    match self
                        .client
                        .delete(&delete_url)
                        .header("X-API-Key", &conn.api_key)
                        .send()
                        .await
                    {
                        Ok(resp) if resp.status().is_success() => {}
                        Ok(resp) => logger::warn(&format!(
                            "tawai remote delete failed for {} ({}): {}",
                            rt.id,
                            rt.title,
                            resp.status()
                        )),
                        Err(e) => logger::warn(&format!(
                            "tawai remote delete request failed for {} ({}): {}",
                            rt.id, rt.title, e
                        )),
                    }
                }
                Err(e) => logger::warn(&format!("tawai delete remote lookup failed: {e}")),
            }
        }

        match std::fs::remove_file(file_path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

/// Forward a local tag write to the remote tawai source so the remote copy stays in sync.
/// Best-effort: returns `Ok(())` (no-op) when the file is not part of a tawai source.
pub async fn mirror_tag_write(
    pool: &DatabasePool,
    local_path: &str,
    tag: &AudioTag,
    client: &reqwest::Client,
) -> Result<()> {
    let sources = library_source::list_all_sources(pool).await?;
    let source = match sources.iter().find(|s| {
        s.source_type == "tawai"
            && local_root(&s.urls)
                .map(|root| Path::new(local_path).starts_with(root))
                .unwrap_or(false)
    }) {
        Some(s) => s,
        None => return Ok(()),
    };

    let (conn, rt) = resolve_remote_track(pool, client, &source.urls, local_path).await?;
    let body = serde_json::json!({
        "path": rt.remote_file_path,
        "title": tag.title,
        "artist": tag.artist,
        "album": tag.album,
        "album_artist": tag.album_artist,
        "genres": tag.genres,
        "track_number": tag.track_number,
        "disc_number": tag.disc_number,
        "release_date": tag.release_date,
        "lyrics": tag.lyrics,
        "cover": tag.cover,
    });
    let url = format!("{}/api/tawai/library/identify/tags/write", conn.http_base);
    let resp = client
        .post(&url)
        .header("X-API-Key", &conn.api_key)
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        anyhow::bail!("tawai remote tag write failed: {}", resp.status());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tawai_url_valid() {
        let conn = parse_tawai_url("tawai://myhost:8080?source_id=abc123&scheme=http")
            .expect("should parse");
        assert_eq!(conn.http_base, "http://myhost:8080");
        assert_eq!(conn.api_key, "");
        assert_eq!(conn.source_id.as_deref(), Some("abc123"));
    }

    #[test]
    fn parse_tawai_url_with_key() {
        let conn = parse_tawai_url(
            "tawai://192.168.1.5:3000@secretkey?scheme=http&source_id=xyz&foo=bar",
        )
        .expect("should parse");
        assert_eq!(conn.http_base, "http://192.168.1.5:3000");
        assert_eq!(conn.api_key, "secretkey");
        assert_eq!(conn.source_id.as_deref(), Some("xyz"));
    }

    #[test]
    fn parse_tawai_url_https() {
        let conn =
            parse_tawai_url("tawai://myhost:8443?source_id=abc123&scheme=https")
                .expect("should parse");
        assert_eq!(conn.http_base, "https://myhost:8443");
        assert_eq!(conn.api_key, "");
        assert_eq!(conn.source_id.as_deref(), Some("abc123"));
    }

    #[test]
    fn parse_tawai_url_scheme_case_insensitive() {
        let conn =
            parse_tawai_url("tawai://myhost:8443?source_id=abc123&scheme=HTTPS")
                .expect("should parse");
        assert_eq!(conn.http_base, "https://myhost:8443");
    }

    #[test]
    fn parse_tawai_url_missing_scheme() {
        assert!(parse_tawai_url("tawai://host:8080@key?source_id=x").is_err());
    }

    #[test]
    fn parse_tawai_url_bad_scheme() {
        assert!(parse_tawai_url("tawai://host:8080@key?source_id=x&scheme=ftp").is_err());
    }

    #[test]
    fn parse_tawai_url_missing_source_id() {
        let conn = parse_tawai_url("tawai://host:8080@key?scheme=http")
            .expect("should parse");
        assert_eq!(conn.http_base, "http://host:8080");
        assert_eq!(conn.api_key, "key");
        assert_eq!(conn.source_id, None);
    }

    #[test]
    fn parse_tawai_url_source_id_optional() {
        let conn = parse_tawai_url("tawai://host:8080@key?scheme=https")
            .expect("should parse");
        assert_eq!(conn.http_base, "https://host:8080");
        assert_eq!(conn.api_key, "key");
        assert_eq!(conn.source_id, None);

        let conn = parse_tawai_url(
            "tawai://host:8080@key?source_id=abc&scheme=https",
        )
        .expect("should parse");
        assert_eq!(conn.source_id.as_deref(), Some("abc"));
    }

    #[test]
    fn parse_tawai_url_rejects_url_scheme() {
        assert!(parse_tawai_url("http://host:8080@key?source_id=x").is_err());
    }

    #[test]
    fn dest_for_deterministic() {
        let root = "/music";
        let pattern = DEFAULT_PATTERN;
        let rt = RemoteTrack {
            id: "1".into(),
            title: "My Song".into(),
            artists_string: "Artist A".into(),
            album_title: "The Album".into(),
            track_num: Some(3),
            disc_num: Some(1),
            duration_secs: Some(240.0),
            file_size: Some(10_000_000),
            bitrate: Some(320_000),
            mbid_recording: None,
            release_date: None,
            genres: vec![],
            remote_file_path: "/srv/music/track.mp3".into(),
        };
        let d1 = dest_for(root, pattern, &rt);
        let d2 = dest_for(root, pattern, &rt);
        assert_eq!(d1, d2);
        assert!(d1.to_string_lossy().contains("My Song"));
    }
}
