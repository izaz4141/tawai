use crate::utils::logger;
use anyhow::Result;
use percent_encoding::percent_decode_str;
use reqwest::{Client, Url, header};
use std::time::Duration;

pub async fn build_browser_client() -> Client {
    let mut headers = header::HeaderMap::new();

    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36"),
    );
    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7",
        ),
    );
    headers.insert(
        header::ACCEPT_LANGUAGE,
        header::HeaderValue::from_static("en-US,en;q=0.9"),
    );
    headers.insert(
        header::ACCEPT_ENCODING,
        header::HeaderValue::from_static("gzip, deflate, br"),
    );
    headers.insert(
        "sec-fetch-dest",
        header::HeaderValue::from_static("document"),
    );
    headers.insert(
        "sec-fetch-mode",
        header::HeaderValue::from_static("navigate"),
    );
    headers.insert("sec-fetch-site", header::HeaderValue::from_static("none"));
    headers.insert("sec-fetch-user", header::HeaderValue::from_static("?1"));
    headers.insert(
        "upgrade-insecure-requests",
        header::HeaderValue::from_static("1"),
    );

    Client::builder()
        .default_headers(headers)
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::limited(10))
        // .connect_timeout(Duration::from_secs(60))
        // .timeout(Duration::from_secs(300))
        .pool_idle_timeout(None)
        .tcp_keepalive(Duration::from_secs(60))
        .build()
        .expect("Failed to build reqwest client")
}

#[derive(Debug, Clone)]
pub struct UrlInfo {
    pub url: String,
    pub name: String,
    pub total_size: Option<u64>,
    pub accept_ranges: bool,
    pub content_type: Option<String>,
}

pub async fn get_url_info(
    client: Client,
    url: &str,
    cookie: Option<String>,
    user_agent: Option<String>,
    _referer: Option<String>,
) -> Result<UrlInfo> {
    let response = match try_head_request(&client, url, cookie.as_deref(), user_agent.as_deref())
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            logger::warn(&format!(
                "HEAD request failed for {}: {}, falling back to GET with Range",
                url, e
            ));
            try_get_range_request(&client, url, cookie.as_deref(), user_agent.as_deref()).await?
        }
    };

    let total_size = response
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|hv| hv.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());

    let accept_ranges = response
        .headers()
        .get(header::ACCEPT_RANGES)
        .and_then(|hv| hv.to_str().ok())
        .map(|s| s.to_ascii_lowercase().contains("bytes"))
        .unwrap_or(false);

    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|hv| hv.to_str().ok())
        .map(|s| s.to_string());

    let name = response
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .and_then(|hv| hv.to_str().ok())
        .and_then(|cd| {
            cd.split(';').find_map(|part| {
                let trimmed = part.trim();
                if trimmed.starts_with("filename=") {
                    Some(
                        percent_decode_str(
                            trimmed.trim_start_matches("filename=").trim_matches('"'),
                        )
                        .decode_utf8_lossy()
                        .to_string(),
                    )
                } else {
                    None
                }
            })
        })
        .unwrap_or_else(|| {
            let parsed = Url::parse(url).ok();
            parsed
                .as_ref()
                .and_then(|u| {
                    u.path_segments()
                        .and_then(|mut segments| segments.next_back())
                        .map(|s| percent_decode_str(s).decode_utf8_lossy().to_string())
                })
                .unwrap_or_else(|| "download.bin".to_string())
        });

    Ok(UrlInfo {
        url: url.to_string(),
        name,
        total_size,
        accept_ranges,
        content_type,
    })
}

async fn try_head_request(
    client: &Client,
    url: &str,
    cookie: Option<&str>,
    user_agent: Option<&str>,
) -> Result<reqwest::Response> {
    let mut request_builder = client.head(url);
    if let Some(c) = cookie {
        request_builder = request_builder.header(header::COOKIE, header::HeaderValue::from_str(c)?);
    }
    if let Some(ua) = user_agent {
        request_builder =
            request_builder.header(header::USER_AGENT, header::HeaderValue::from_str(ua)?);
    }
    let response = request_builder.send().await?;

    // If server returns 405 Method Not Allowed, treat as failure to trigger fallback
    if response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
        return Err(anyhow::anyhow!("Server returned 405 Method Not Allowed"));
    }

    Ok(response)
}

/// Strip credentials from a URL for logging: userinfo, query, and fragment are
/// removed. Non-URL values (such as local paths) are returned unchanged.
pub fn redact_url(url: &str) -> String {
    match Url::parse(url) {
        Ok(mut parsed) => {
            if !parsed.username().is_empty() {
                let _ = parsed.set_username("");
            }
            if parsed.password().is_some() {
                let _ = parsed.set_password(None);
            }
            if parsed.query().is_some() {
                parsed.set_query(Some("redacted"));
            }
            parsed.set_fragment(None);
            parsed.to_string()
        }
        Err(_) => redact_url_fallback(url),
    }
}

/// Redact every URL embedded in `text`.
pub fn redact_urls(text: &str) -> String {
    let mut redacted = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let next = ["http://", "https://"]
            .iter()
            .filter_map(|scheme| rest.find(scheme))
            .min();
        let Some(index) = next else {
            redacted.push_str(rest);
            break;
        };
        redacted.push_str(&rest[..index]);
        let tail = &rest[index..];
        let end = tail
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | ')' | ']' | '>'))
            .unwrap_or(tail.len());
        redacted.push_str(&redact_url(&tail[..end]));
        rest = &tail[end..];
    }
    redacted
}

fn redact_url_fallback(url: &str) -> String {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return url.to_string();
    }
    match url.find(['?', '#']) {
        Some(index) => format!("{}?redacted", &url[..index]),
        None => url.to_string(),
    }
}

async fn try_get_range_request(
    client: &Client,
    url: &str,
    cookie: Option<&str>,
    user_agent: Option<&str>,
) -> Result<reqwest::Response> {
    let mut request_builder = client.get(url);
    if let Some(c) = cookie {
        request_builder = request_builder.header(header::COOKIE, header::HeaderValue::from_str(c)?);
    }
    if let Some(ua) = user_agent {
        request_builder =
            request_builder.header(header::USER_AGENT, header::HeaderValue::from_str(ua)?);
    }
    request_builder.send().await.map_err(|e| e.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_signed_url_query() {
        let redacted = redact_url("https://host/media.m4a?expire=1&sig=secret");
        assert_eq!(redacted, "https://host/media.m4a?redacted");
        assert!(!redacted.contains("secret"));
    }

    #[test]
    fn redacts_userinfo() {
        assert_eq!(redact_url("https://user:pass@host/a"), "https://host/a");
    }

    #[test]
    fn redacts_fragment() {
        assert_eq!(redact_url("https://host/a#token"), "https://host/a");
    }

    #[test]
    fn keeps_plain_paths() {
        assert_eq!(redact_url("/tmp/audio.mp3"), "/tmp/audio.mp3");
    }

    #[test]
    fn redacts_urls_embedded_in_text() {
        let redacted = redact_urls("ffprobe failed for https://host/a?token=secret: 403 Forbidden");
        assert!(redacted.contains("https://host/a?redacted"));
        assert!(!redacted.contains("secret"));
    }
}
