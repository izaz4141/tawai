use anyhow::Result;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Deserialize;

#[derive(Deserialize)]
struct ItunesResponse {
    #[serde(rename = "resultCount")]
    result_count: u32,
    results: Vec<ItunesResult>,
}

#[derive(Deserialize)]
struct ItunesResult {
    #[serde(rename = "previewUrl")]
    preview_url: Option<String>,
}

pub async fn fetch_preview(
    client: &reqwest::Client,
    artist: &str,
    title: &str,
) -> Result<Option<String>> {
    let search_term = format!("{} {}", artist, title);
    let encoded = utf8_percent_encode(&search_term, NON_ALPHANUMERIC);
    let url = format!(
        "https://itunes.apple.com/search?term={}&limit=1&entity=song",
        encoded
    );

    let resp = client.get(&url).send().await?;

    let body: ItunesResponse = resp.json().await?;

    Ok(body.results.into_iter().next().and_then(|r| r.preview_url))
}
