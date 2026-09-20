use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use futures::StreamExt;
use tokio::io::AsyncWriteExt;

use crate::audio::scan::{LogSender, stream_log};
use crate::audio::tags::{AudioTag, derive_sort_name, parse_artists};
use crate::db::account::DEFAULT_USERNAME;
use crate::db::database::DatabasePool;
use crate::db::library_source;
use crate::db::user_settings;
use crate::libsources::{ParsedTrack, local};
use crate::signals::discovery::{JellyfinLibraryInfo, ServerTestResult};
use crate::signals::library::{
    LibrarySourceInfo, ListLibrarySourcesResponse, ListTracksResponse, TrackInfo,
};
use crate::tools::rename::{DEFAULT_PATTERN, dest_from_root};
use crate::utils::config::AppConfig;
use crate::utils::logger;

// ── Remote connection ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RemoteConn {
    pub http_base: String,
    pub api_key: String,
    /// The `?source_id=<id>` from the URL, once known.
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

/// Encrypt the API-key segment of a `tawai://` URL for storage. Non-tawai
/// URLs, missing keys, and `NDK:` keys pass through unchanged.
pub fn encrypt_url(url: &str, master_key: &str) -> String {
    let Some(rest) = url.strip_prefix("tawai://") else {
        return url.to_string();
    };
    let (authority, query) = match rest.split_once('?') {
        Some((a, q)) => (a, format!("?{q}")),
        None => (rest, String::new()),
    };
    let Some((hostport, key)) = authority.split_once('@') else {
        return url.to_string();
    };
    if key.is_empty() || key.starts_with("NDK:") {
        return url.to_string();
    }
    let encrypted =
        crate::utils::encryption::encrypt(key, master_key).unwrap_or_else(|_| key.to_string());
    format!("tawai://{hostport}@{encrypted}{query}")
}

/// Decrypt the API key of a URL persisted with `encrypt_url`. Non-tawai URLs,
/// plaintext keys, and undecryptable keys pass through unchanged.
pub fn decrypt_url(url: &str, master_key: &str) -> String {
    if !url.starts_with("tawai://") || !url.contains("@NDK:") {
        return url.to_string();
    }
    let Some(rest) = url.strip_prefix("tawai://") else {
        return url.to_string();
    };
    let (authority, query) = match rest.split_once('?') {
        Some((a, q)) => (a, format!("?{q}")),
        None => (rest, String::new()),
    };
    let Some((hostport, key)) = authority.split_once('@') else {
        return url.to_string();
    };
    let decrypted =
        crate::utils::encryption::decrypt(key, master_key).unwrap_or_else(|_| key.to_string());
    format!("tawai://{hostport}@{decrypted}{query}")
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

/// Probe each `tawai://` URL with one sources-list request (reachability +
/// key check); return the first server's libraries and per-URL results.
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
        anyhow::bail!("tawai list sources failed with status {}: {}", status, text);
    }
    let status = resp.status();
    let bytes = resp.bytes().await?;
    let data: ListLibrarySourcesResponse =
        crate::libsources::decode_json("tawai library sources list", status, &bytes)?;
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

/// Upload an audio file to a remote tawai source, which imports and rescans
/// it so it appears in the remote library.
pub async fn upload_to_remote(
    client: &reqwest::Client,
    conn: &RemoteConn,
    file_bytes: Vec<u8>,
    filename: &str,
) -> Result<()> {
    let source_id = conn
        .source_id
        .as_deref()
        .context("tawai upload requires source_id in the remote URL")?;
    let url = format!("{}/api/tawai/library/tracks/import", conn.http_base);
    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(filename.to_string())
        .mime_str("application/octet-stream")?;
    let form = reqwest::multipart::Form::new()
        .text("source_id", source_id.to_string())
        .text("name", filename.to_string())
        .part("file", part);
    let resp = client
        .post(&url)
        .header("X-API-Key", &conn.api_key)
        .multipart(form)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("tawai import failed with status {status}: {text}");
    }
    Ok(())
}

/// Import an audio file into an editable local source: tag it, move it into
/// the source root, rescan, and return the final path.
pub async fn import_into_source(
    pool: &DatabasePool,
    source: &LibrarySourceInfo,
    file_bytes: &[u8],
    filename: &str,
    client: &reqwest::Client,
    master_key: &str,
) -> Result<String> {
    if !crate::libsources::is_editable(&source.source_type) {
        anyhow::bail!(
            "cannot import into non-editable source: {}",
            source.source_type
        );
    }
    let root = local_root(&source.urls).context("import source has no local directory root")?;

    let temp_name = Path::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty())
        .unwrap_or("import.mp3");
    let temp_path = std::env::temp_dir().join(temp_name);
    tokio::fs::write(&temp_path, file_bytes).await?;

    let (tag, _, _, _) = crate::audio::tags::read_audio_tags(&temp_path)?;
    let pattern = user_settings::get_setting(pool, DEFAULT_USERNAME, "identify_naming_pattern")
        .await
        .filter(|s| !s.is_empty());

    let final_path =
        crate::tools::rename::move_file_into_source(&temp_path, root, pattern.as_deref(), &tag)?;

    if let Err(e) = crate::audio::tags::write_audio_tags(&final_path, &tag) {
        logger::warn(&format!(
            "failed to write tags to imported file {}: {}",
            final_path.display(),
            e
        ));
    }

    let scan_result = crate::audio::scan::run_scan(
        pool,
        client.clone(),
        std::slice::from_ref(source),
        false,
        None,
        master_key,
        None,
    )
    .await;
    if let Some(err) = scan_result.error {
        logger::warn(&format!(
            "failed to scan source '{}' after import: {}",
            source.name, err
        ));
    }

    Ok(final_path.to_string_lossy().to_string())
}

// ── Remote track list fetch + cache ───────────────────────────────────

pub async fn fetch_remote_tracks(
    client: &reqwest::Client,
    conn: &RemoteConn,
) -> Result<Vec<TrackInfo>> {
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
    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("tawai list tracks failed with status {status}");
    }
    let declared_len = resp.content_length();
    let encoded = resp
        .headers()
        .get(reqwest::header::CONTENT_ENCODING)
        .is_some();
    let bytes = resp.bytes().await?;
    if !encoded {
        if let Some(expected) = declared_len {
            if expected as usize != bytes.len() {
                anyhow::bail!(
                    "tawai list tracks response truncated: declared {expected} bytes but received {} bytes",
                    bytes.len()
                );
            }
        }
    }
    let body: ListTracksResponse = serde_json::from_slice(&bytes).map_err(|e| {
        // Embed the serde detail (which names the offending field, e.g.
        // "missing field 'track_num'") plus a body preview so the cause
        // reaches the scan pipeline's log channel without extra local logging.
        let preview = bytes
            .iter()
            .take(300)
            .map(|&b| {
                (b.is_ascii_graphic() || b == b' ')
                    .then_some(b as char)
                    .unwrap_or('.')
            })
            .collect::<String>();
        anyhow::anyhow!(
            "tawai: error decoding response body ({status}, {} bytes): {e}; body preview: {preview}",
            bytes.len()
        )
    })?;
    Ok(body.tracks)
}

// ── Process-local cache────────────────────────────────────────────────

struct CacheEntry {
    tracks: Vec<TrackInfo>,
    fetched_at: Instant,
}

struct FailEntry {
    error: String,
    failed_at: Instant,
}

struct ReachEntry {
    reachable: bool,
    checked_at: Instant,
}

static REMOTE_CACHE: LazyLock<Mutex<HashMap<String, CacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Caches failed track-list fetches per connection for `CACHE_TTL`, so a
/// failing server is probed once per TTL rather than per file.
static REMOTE_FAIL_CACHE: LazyLock<Mutex<HashMap<String, FailEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static REACH_CACHE: LazyLock<Mutex<HashMap<String, ReachEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const CACHE_TTL: Duration = Duration::from_secs(60);

const REACH_TTL: Duration = Duration::from_secs(60);

/// Max redownload attempts per track; download errors and hash mismatches
/// share this budget.
const MAX_DOWNLOAD_ATTEMPTS: u32 = 5;
/// Stream chunk retries, resuming from the written offset each pass.
const MAX_STREAM_RETRIES: u32 = 5;
/// Max bytes per `Range` request, small enough to finish within a flaky
/// connection's useful lifetime.
const STREAM_CHUNK_SIZE: u64 = 8 * 1024 * 1024;

pub async fn cached_remote_tracks(
    client: &reqwest::Client,
    conn: &RemoteConn,
) -> Result<Vec<TrackInfo>> {
    let key = format!(
        "{}|{}",
        conn.http_base,
        conn.source_id.as_deref().unwrap_or("")
    );
    {
        let cache = REMOTE_CACHE.lock().unwrap();
        if let Some(entry) = cache.get(&key) {
            if entry.fetched_at.elapsed() < CACHE_TTL {
                return Ok(entry.tracks.clone());
            }
        }
    }
    {
        let fails = REMOTE_FAIL_CACHE.lock().unwrap();
        if let Some(entry) = fails.get(&key) {
            if entry.failed_at.elapsed() < CACHE_TTL {
                return Err(anyhow::anyhow!(
                    "tawai remote track list (cached failure): {}",
                    entry.error
                ));
            }
        }
    }
    match fetch_remote_tracks(client, conn).await {
        Ok(tracks) => {
            let mut cache = REMOTE_CACHE.lock().unwrap();
            cache.insert(
                key,
                CacheEntry {
                    tracks: tracks.clone(),
                    fetched_at: Instant::now(),
                },
            );
            Ok(tracks)
        }
        Err(e) => {
            let mut fails = REMOTE_FAIL_CACHE.lock().unwrap();
            fails.insert(
                key,
                FailEntry {
                    error: e.to_string(),
                    failed_at: Instant::now(),
                },
            );
            Err(e)
        }
    }
}

// ── AudioTag synthesis from remote metadata────────────────────────────

fn remote_track_to_tag(rt: &TrackInfo) -> AudioTag {
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
        mbid_artist: rt.artist_mbid.clone(),
        mbid_release: rt.album_mbid.clone(),
        lyrics: rt.lyrics.clone(),
        track_gain: rt.track_gain,
        track_peak: rt.track_peak,
        acoust_id_fingerprint: rt.acoust_id_fingerprint.clone(),
        acoust_id: rt.acoust_id.clone(),
        ..Default::default()
    };
    tag.artist_sort = derive_sort_name(&tag.artist);
    tag.album_artist_sort = derive_sort_name(&tag.album_artist);
    tag
}

/// Deterministic local backup path for a remote track.
fn dest_for(local_root: &str, naming_pattern: &str, rt: &TrackInfo) -> PathBuf {
    let tag = remote_track_to_tag(rt);
    let ext = ext_from_path(&rt.file_path);
    let fallback = format!("{} - {}", rt.artists_string, rt.title);
    dest_from_root(local_root, naming_pattern, &tag, ext, &fallback)
}

fn ext_from_path(path: &str) -> &str {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp3")
}

/// Build a `ParsedTrack` from remote metadata, skipping a full local rescan.
/// Only call after the file's SHA-256 matches the remote `file_hash`.
fn tawai_track_to_parsed(rt: &TrackInfo, dest: &Path, file_hash: &str) -> ParsedTrack {
    let mut tag = remote_track_to_tag(rt);
    // Best-effort: keep the embedded cover art so album art still populates,
    // without paying for fingerprinting or loudness measurement.
    if let Ok((local_tag, _, _, _)) = crate::audio::tags::read_audio_tags(dest) {
        if local_tag.cover.is_some() {
            tag.cover = local_tag.cover;
        }
    }
    let file_size = std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
    ParsedTrack {
        tag,
        file_path: dest.to_string_lossy().to_string(),
        file_hash: Some(file_hash.to_string()),
        duration_secs: rt.duration_secs.unwrap_or(0.0),
        sample_rate: rt.sample_rate.map(|s| s as u32),
        bitrate: rt.bitrate.map(|b| b as u32),
        file_size,
    }
}

// ── Download a single track from remote ───────────────────────────────

/// Download `rt` into `dest` in byte-range chunks, retrying each chunk up to
/// `MAX_STREAM_RETRIES` and resuming the partial file. Restarts from byte 0
/// only on 416 or a mid-file 200 ignoring Range.
async fn download_track(
    client: &reqwest::Client,
    conn: &RemoteConn,
    rt: &TrackInfo,
    dest: &Path,
    log_tx: &Option<LogSender>,
) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let stream_url = format!("{}/api/tawai/playback/stream/{}", conn.http_base, rt.id);
    let mut expected: Option<u64> = None;
    let mut last_err: Option<anyhow::Error> = None;
    loop {
        let offset = dest.metadata().map(|m| m.len()).unwrap_or(0);
        if let Some(len) = expected {
            if len > 0 && offset >= len {
                return Ok(());
            }
        }
        let mut attempts = 0u32;
        loop {
            if attempts >= MAX_STREAM_RETRIES {
                return Err(last_err.unwrap_or_else(|| {
                    anyhow::anyhow!("download failed for {} '{}'", rt.id, rt.title)
                }));
            }
            attempts += 1;
            match download_chunk(client, conn, rt, dest, &stream_url, offset).await {
                Ok(ChunkOutcome::Done) => return Ok(()),
                Ok(ChunkOutcome::Advanced(total)) => {
                    expected = total;
                    break;
                }
                Ok(ChunkOutcome::Restart) => {
                    expected = None;
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                    stream_log(
                        log_tx,
                        "WARN",
                        format!(
                            "tawai download pass {attempts}/{} failed for {} '{}' at offset {}, resuming",
                            MAX_STREAM_RETRIES,
                            rt.id,
                            rt.title,
                            offset
                        ),
                    );
                }
            }
        }
    }
}

/// Outcome of a single `download_chunk` request.
enum ChunkOutcome {
    /// The whole file was written in this request (Range ignored from byte 0).
    Done,
    /// This chunk was written; the payload carries the remote total discovered
    /// from Content-Range/Content-Length (None if unknown).
    Advanced(Option<u64>),
    /// The server refused to resume (416, or a plain 200 against a mid-file
    /// range); the destination was purged and the caller restarts from byte 0.
    Restart,
}

/// Fetch one bounded byte-range chunk: `206` writes up to `STREAM_CHUNK_SIZE`
/// bytes; a byte-0 `200` writes the whole body; `416` or a mid-file `200`
/// purges `dest`.
async fn download_chunk(
    client: &reqwest::Client,
    conn: &RemoteConn,
    rt: &TrackInfo,
    dest: &Path,
    stream_url: &str,
    offset: u64,
) -> Result<ChunkOutcome> {
    let resp = client
        .get(stream_url)
        .header("X-API-Key", &conn.api_key)
        // Ask for raw bytes: the backup must be byte-identical for the stored
        // file_hash to match, and negotiated compression (gzip/br) can fail
        // mid-stream with a bare body-decode error that otherwise surfaces as
        // an opaque "error decoding response body".
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .header(
            reqwest::header::RANGE,
            format!("bytes={}-{}", offset, offset.saturating_add(STREAM_CHUNK_SIZE - 1)),
        )
        .send()
        .await?;
    let status = resp.status();

    // Remote total: prefer the total from Content-Range (`bytes a-b/total`),
    // else this response's Content-Length.
    let content_range = resp
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let total_from_range = content_range
        .as_deref()
        .and_then(|cr| cr.rsplit('/').next())
        .and_then(|t| t.parse::<u64>().ok());
    let declared_len = resp.content_length();
    let expected_total: Option<u64> = total_from_range.or(declared_len);

    match status {
        s if s == reqwest::StatusCode::RANGE_NOT_SATISFIABLE => {
            let _ = std::fs::remove_file(dest);
            return Ok(ChunkOutcome::Restart);
        }
        // A plain 200 answering a Range request means the server ignored it:
        // from byte 0 we accept the whole body in one pass; past byte 0 the
        // on-disk copy cannot be continued, so restart from scratch.
        s if s == reqwest::StatusCode::OK && offset > 0 => {
            let _ = std::fs::remove_file(dest);
            return Ok(ChunkOutcome::Restart);
        }
        s if !s.is_success() => {
            anyhow::bail!(
                "tawai stream download failed for {} ({}): {}",
                rt.id,
                rt.title,
                s
            );
        }
        _ => {}
    }
    let full_body = status == reqwest::StatusCode::OK;

    let content_encoding = resp
        .headers()
        .get(reqwest::header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    // Bytes this request is expected to deliver.
    let want: Option<u64> = if full_body {
        expected_total
    } else {
        Some(match expected_total {
            Some(total) => (total - offset).min(STREAM_CHUNK_SIZE),
            None => STREAM_CHUNK_SIZE,
        })
    };

    let mut file = if offset > 0 && !full_body {
        tokio::fs::OpenOptions::new().append(true).open(dest).await?
    } else {
        tokio::fs::File::create(dest).await?
    };
    let mut written: u64 = 0;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| {
            anyhow::anyhow!(
                "download stream failed for {} '{}' at offset {} after {} bytes (content-range: {}, content-encoding: {}, url: {}): {e}",
                rt.id,
                rt.title,
                offset,
                written,
                content_range.as_deref().unwrap_or("none"),
                if content_encoding.is_empty() {
                    "none".to_string()
                } else {
                    content_encoding.clone()
                },
                stream_url
            )
        })?;
        let take = match want {
            Some(w) => (w - written).min(chunk.len() as u64),
            None => chunk.len() as u64,
        };
        if take > 0 {
            file.write_all(&chunk[..take as usize]).await?;
            written += take;
        }
        if let Some(w) = want {
            if written >= w {
                // This request delivered everything expected; drop any excess.
                break;
            }
        }
    }
    file.flush().await?;

    if let Some(w) = want {
        if written < w {
            // The connection dropped before this chunk's bytes were delivered
            // (this is what hyper reports as "error decoding response body").
            // Leave the partial in place so the next pass resumes the chunk.
            anyhow::bail!(
                "download stream truncated for {} '{}': got {} of {} bytes for chunk at {} (content-range: {}, url: {})",
                rt.id,
                rt.title,
                written,
                w,
                offset,
                content_range.as_deref().unwrap_or("none"),
                stream_url
            );
        }
    }
    if full_body {
        return Ok(ChunkOutcome::Done);
    }
    Ok(ChunkOutcome::Advanced(expected_total))
}

/// Map a local backup path back to the remote connection + track that backs it.
async fn resolve_remote_track(
    pool: &DatabasePool,
    client: &reqwest::Client,
    urls: &[String],
    local_path: &str,
) -> Result<(RemoteConn, TrackInfo)> {
    let root = local_root(urls).unwrap_or("/tmp");
    let conn = pick_remote(urls, client)
        .await
        .context("remote tawai unreachable")?;
    let tracks = cached_remote_tracks(client, &conn)
        .await
        .context("tawai remote track list failed")?;
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

    /// Enumerate deterministic local dest paths from the remote's track list,
    /// falling back to the local backup walk when unreachable.
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
            let tracks = cached_remote_tracks(&self.client, &conn)
                .await
                .context("tawai remote track list failed for remote source")?;
            return Ok(tracks
                .iter()
                .map(|rt| dest_for(root, &pattern, rt).to_string_lossy().to_string())
                .collect());
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
            match self.scan_file(pool, _url, urls, p, &None).await {
                Ok(t) => tracks.push(t),
                Err(e) => logger::warn(&format!("tawai scan_paths skip {}: {}", p, e)),
            }
        }
        Ok(tracks)
    }

    /// Scan one file: import remote metadata when the local SHA-256 matches
    /// the remote `file_hash`, else (re)download and rescan. Falls back to a
    /// local scan when the remote is unreachable or the hash never matches.
    pub async fn scan_file(
        &self,
        pool: &DatabasePool,
        _url: &str,
        urls: &[String],
        file_path: &str,
        log_tx: &Option<LogSender>,
    ) -> Result<ParsedTrack> {
        let dest = Path::new(file_path);
        match resolve_remote_track(pool, &self.client, urls, file_path).await {
            Ok((conn, rt)) => {
                let Some(remote_hash) = rt.file_hash.as_deref() else {
                    // No remote hash to verify against: download a missing file
                    // once, then fully rescan it.
                    if !dest.is_file() {
                        download_track(&self.client, &conn, &rt, dest, log_tx).await?;
                    }
                    return local::scan_file(dest);
                };

                // This loop's budget covers hash-validation redownloads only:
                // stream failures are retried (and resumed) inside
                // `download_track`, so an error here means the download fully
                // failed and is surfaced immediately.
                let mut attempts = 0u32;
                let mut last_local_hash: Option<String> = None;
                loop {
                    if dest.is_file() {
                        if let Ok(local_hash) = local::hash_file(dest) {
                            last_local_hash = Some(local_hash.clone());
                            if local_hash.eq_ignore_ascii_case(remote_hash) {
                                return Ok(tawai_track_to_parsed(&rt, dest, &local_hash));
                            }
                        }
                    }
                    if attempts >= MAX_DOWNLOAD_ATTEMPTS {
                        break;
                    }
                    attempts += 1;
                    // A complete-but-mismatched file yields a 416 from the range
                    // request inside `download_track`, which purges the file and
                    // redownloads it whole. A partial file is resumed.
                    download_track(&self.client, &conn, &rt, dest, log_tx).await?;
                }
                stream_log(
                    log_tx,
                    "WARN",
                    format!(
                        "tawai hash still mismatched after {} attempts, keeping local copy: {}\n  local : {}\n  remote: {}",
                        MAX_DOWNLOAD_ATTEMPTS,
                        dest.display(),
                        last_local_hash.as_deref().unwrap_or("unknown"),
                        remote_hash,
                    ),
                );
                local::scan_file(dest)
            }
            // Remote unreachable: still scan an existing local backup (offline
            // mode); a missing file is a genuine error.
            Err(e) => {
                if dest.is_file() {
                    local::scan_file(dest)
                } else {
                    Err(e)
                }
            }
        }
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

    /// Delete the local backup, mirroring to the remote when `mirror_remote`
    /// is set (best-effort).
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

/// Forward a local tag write to the remote source so the remote copy stays in
/// sync; no-op for non-tawai files.
pub async fn mirror_tag_write(
    pool: &DatabasePool,
    local_path: &str,
    tag: &AudioTag,
    client: &reqwest::Client,
    master_key: &str,
) -> Result<()> {
    let sources = library_source::list_all_sources(pool, master_key).await?;
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
        "path": rt.file_path,
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
        let conn =
            parse_tawai_url("tawai://192.168.1.5:3000@secretkey?scheme=http&source_id=xyz&foo=bar")
                .expect("should parse");
        assert_eq!(conn.http_base, "http://192.168.1.5:3000");
        assert_eq!(conn.api_key, "secretkey");
        assert_eq!(conn.source_id.as_deref(), Some("xyz"));
    }

    #[test]
    fn parse_tawai_url_https() {
        let conn = parse_tawai_url("tawai://myhost:8443?source_id=abc123&scheme=https")
            .expect("should parse");
        assert_eq!(conn.http_base, "https://myhost:8443");
        assert_eq!(conn.api_key, "");
        assert_eq!(conn.source_id.as_deref(), Some("abc123"));
    }

    #[test]
    fn parse_tawai_url_scheme_case_insensitive() {
        let conn = parse_tawai_url("tawai://myhost:8443?source_id=abc123&scheme=HTTPS")
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
        let conn = parse_tawai_url("tawai://host:8080@key?scheme=http").expect("should parse");
        assert_eq!(conn.http_base, "http://host:8080");
        assert_eq!(conn.api_key, "key");
        assert_eq!(conn.source_id, None);
    }

    #[test]
    fn parse_tawai_url_source_id_optional() {
        let conn = parse_tawai_url("tawai://host:8080@key?scheme=https").expect("should parse");
        assert_eq!(conn.http_base, "https://host:8080");
        assert_eq!(conn.api_key, "key");
        assert_eq!(conn.source_id, None);

        let conn = parse_tawai_url("tawai://host:8080@key?source_id=abc&scheme=https")
            .expect("should parse");
        assert_eq!(conn.source_id.as_deref(), Some("abc"));
    }

    #[test]
    fn parse_tawai_url_rejects_url_scheme() {
        assert!(parse_tawai_url("http://host:8080@key?source_id=x").is_err());
    }

    #[test]
    fn encrypt_url_roundtrip() {
        let mk = crate::utils::encryption::generate_master_key();
        let url = "tawai://192.168.1.5:3000@secretkey?scheme=http&source_id=xyz&foo=bar";
        let enc = encrypt_url(url, &mk);
        assert!(enc.starts_with("tawai://192.168.1.5:3000@NDK:"));
        assert!(enc.contains("?scheme=http&source_id=xyz&foo=bar"));
        assert_eq!(decrypt_url(&enc, &mk), url);
    }

    #[test]
    fn encrypt_url_skips_plaintext_and_non_tawai() {
        let mk = crate::utils::encryption::generate_master_key();
        assert_eq!(encrypt_url("/music/path", &mk), "/music/path");
        assert_eq!(
            encrypt_url("tawai://host:8080?source_id=x&scheme=http", &mk),
            "tawai://host:8080?source_id=x&scheme=http"
        );
        let already = format!("tawai://host:8080@NDK:{}?scheme=http", "ab01");
        assert_eq!(encrypt_url(&already, &mk), already);
    }

    #[test]
    fn decrypt_url_passthrough_and_legacy() {
        let mk = crate::utils::encryption::generate_master_key();
        assert_eq!(decrypt_url("/music/path", &mk), "/music/path");
        assert_eq!(
            decrypt_url("tawai://host:8080@plainkey?scheme=http", &mk),
            "tawai://host:8080@plainkey?scheme=http"
        );
    }

    #[test]
    fn dest_for_deterministic() {
        use crate::signals::library::ArtistInfo;
        let root = "/music";
        let pattern = DEFAULT_PATTERN;
        let rt = TrackInfo {
            id: "1".into(),
            title: "My Song".into(),
            album_id: "a1".into(),
            album_title: "The Album".into(),
            artists: vec![ArtistInfo {
                id: "ar1".into(),
                name: "Artist A".into(),
                sort_name: None,
                mbid: None,
                thumbnail_url: None,
                album_count: 0,
                track_count: 0,
            }],
            artists_string: "Artist A".into(),
            track_num: Some(3),
            disc_num: Some(1),
            duration_secs: Some(240.0),
            file_path: "/srv/music/track.mp3".into(),
            file_size: Some(10_000_000),
            bitrate: Some(320_000),
            mbid_recording: None,
            artist_mbid: None,
            album_mbid: None,
            lyrics: None,
            release_date: None,
            track_gain: None,
            track_peak: None,
            source: "tawai".into(),
            source_type: "tawai".into(),
            genres: vec![],
            file_hash: None,
            sample_rate: None,
            acoust_id_fingerprint: None,
            acoust_id: None,
        };
        let d1 = dest_for(root, pattern, &rt);
        let d2 = dest_for(root, pattern, &rt);
        assert_eq!(d1, d2);
        assert!(d1.to_string_lossy().contains("My Song"));
    }
}
