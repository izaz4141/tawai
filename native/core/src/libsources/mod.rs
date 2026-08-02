pub mod jellyfin;
pub mod local;

use std::ops::Deref;
use std::path::Path;
use std::time::Duration;

use anyhow::Result;

// ── Recommendation source catalog (single source of truth on Rust side) ──

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ApiType {
    Recommendations,
    CreatedFor,
}

#[derive(Debug, Clone, Copy)]
pub struct RecommendationSource {
    pub rec_src: &'static str,
    pub rec_type: &'static str,
    pub display_name: &'static str,
    pub source_type: &'static str,
    pub api_type: ApiType,
    pub api_rec_type: &'static str,
    pub refresh_interval: Duration,
}

pub static ALL_RECOMMENDATION_SOURCES: &[RecommendationSource] = &[
    RecommendationSource {
        rec_src: "listenbrainz",
        rec_type: "weekly-explo",
        display_name: "Weekly Exploration",
        source_type: "recommendation:listenbrainz_weekly-explo",
        api_type: ApiType::CreatedFor,
        api_rec_type: "weekly-exploration",
        refresh_interval: Duration::from_secs(7 * 24 * 60 * 60),
    },
    RecommendationSource {
        rec_src: "listenbrainz",
        rec_type: "year",
        display_name: "Year in Music",
        source_type: "recommendation:listenbrainz_year",
        api_type: ApiType::CreatedFor,
        api_rec_type: "year",
        refresh_interval: Duration::from_secs(30 * 24 * 60 * 60),
    },
    RecommendationSource {
        rec_src: "listenbrainz",
        rec_type: "weekly",
        display_name: "Weekly",
        source_type: "recommendation:listenbrainz_weekly",
        api_type: ApiType::CreatedFor,
        api_rec_type: "weekly",
        refresh_interval: Duration::from_secs(7 * 24 * 60 * 60),
    },
    RecommendationSource {
        rec_src: "listenbrainz",
        rec_type: "daily",
        display_name: "Daily",
        source_type: "recommendation:listenbrainz_daily",
        api_type: ApiType::CreatedFor,
        api_rec_type: "daily",
        refresh_interval: Duration::from_secs(24 * 60 * 60),
    },
    RecommendationSource {
        rec_src: "listenbrainz",
        rec_type: "top",
        display_name: "Top Recommendations",
        source_type: "recommendation:listenbrainz_top",
        api_type: ApiType::Recommendations,
        api_rec_type: "top",
        refresh_interval: Duration::from_secs(7 * 24 * 60 * 60),
    },
    RecommendationSource {
        rec_src: "listenbrainz",
        rec_type: "raw",
        display_name: "Raw Recommendations",
        source_type: "recommendation:listenbrainz_raw",
        api_type: ApiType::Recommendations,
        api_rec_type: "raw",
        refresh_interval: Duration::from_secs(7 * 24 * 60 * 60),
    },
    RecommendationSource {
        rec_src: "listenbrainz",
        rec_type: "similar",
        display_name: "Similar Artists",
        source_type: "recommendation:listenbrainz_similar",
        api_type: ApiType::Recommendations,
        api_rec_type: "similar",
        refresh_interval: Duration::from_secs(7 * 24 * 60 * 60),
    },
];

impl RecommendationSource {
    pub fn from_key(key: &str) -> Option<&'static Self> {
        let rec_type = key.strip_prefix("recommendation:")?;
        ALL_RECOMMENDATION_SOURCES.iter().find(|s| {
            let full = format!("{}_{}", s.rec_src, s.rec_type);
            rec_type == full
        })
    }

    pub fn from_api_rec_type(api_rec_type: &str) -> Option<&'static Self> {
        ALL_RECOMMENDATION_SOURCES
            .iter()
            .find(|s| s.api_rec_type == api_rec_type)
    }

    pub fn display_name_with_user(&self, username: &str) -> String {
        format!("{} ({})", self.display_name, username)
    }

    pub fn source_url(&self) -> String {
        format!("{}://{}", self.rec_src, self.api_rec_type)
    }

    /// For CreatedFor sources: the filter type to pass to fetch_createdfor.
    /// Year maps to "discoveries", others use api_rec_type directly.
    pub fn created_for_filter(&self) -> &'static str {
        debug_assert_eq!(self.api_type, ApiType::CreatedFor);
        if self.rec_type == "year" {
            "discoveries"
        } else {
            self.api_rec_type
        }
    }
}

use crate::audio::tags::AudioTag;

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

pub enum SourceParser {
    Local,
    Jellyfin(jellyfin::JellyfinParser),
}

impl SourceParser {
    pub async fn enumerate_paths(&self, url: &str) -> Result<Vec<String>> {
        match self {
            SourceParser::Local => local::enumerate_paths(url),
            SourceParser::Jellyfin(p) => p.enumerate_paths(url).await,
        }
    }

    pub async fn scan_paths(&self, url: &str, paths: &[String]) -> Result<Vec<ParsedTrack>> {
        match self {
            SourceParser::Local => local::scan_paths(url, paths),
            SourceParser::Jellyfin(p) => p.scan_paths(url, paths).await,
        }
    }

    pub async fn scan_file(&self, url: &str, file_path: &str) -> Result<ParsedTrack> {
        match self {
            SourceParser::Local => local::scan_file(Path::new(file_path)),
            SourceParser::Jellyfin(p) => p.scan_file(url, file_path).await,
        }
    }

    pub async fn resolve_stream_url(
        &self,
        file_path: &str,
        source_url: &str,
    ) -> Result<(String, Vec<(String, String)>)> {
        match self {
            SourceParser::Local => Ok((file_path.to_string(), vec![])),
            SourceParser::Jellyfin(p) => p.resolve_stream_url(file_path, source_url).await,
        }
    }
}

pub fn get_parser(source_type: &str, client: reqwest::Client) -> Option<SourceParser> {
    match source_type {
        "local" => Some(SourceParser::Local),
        "jellyfin" => Some(SourceParser::Jellyfin(jellyfin::JellyfinParser::new(
            client,
        ))),
        _ => None,
    }
}
