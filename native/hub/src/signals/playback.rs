use rinf::{DartSignal, RustSignal, SignalPiece};
use serde::{Deserialize, Serialize};

use super::library::TrackInfo;

#[derive(Deserialize, DartSignal)]
pub struct PlayTrackRequest {
    pub id: String,
    pub track_id: Option<String>,
    pub track: Option<TrackInfo>,
}

#[derive(Serialize, RustSignal)]
pub struct PlayTrackResponse {
    pub id: String,
    pub file_path: String,
    pub error: Option<String>,
    pub headers: Option<Vec<(String, String)>>,
}

/// Metadata read from the actual stream/file that is playing.
#[derive(Debug, Clone, Default, Serialize, Deserialize, SignalPiece)]
pub struct AudioProbe {
    pub transport: Option<String>,
    pub container: Option<String>,
    pub container_long: Option<String>,
    pub codec: Option<String>,
    pub codec_long: Option<String>,
    pub bitrate_kbps: Option<i64>,
    pub sample_rate_hz: Option<u32>,
    pub bit_depth: Option<u32>,
    pub channels: Option<u32>,
    pub duration_secs: Option<f64>,
    pub size_bytes: Option<i64>,
}

impl From<tawai_core::audio::probe::AudioProbe> for AudioProbe {
    fn from(probe: tawai_core::audio::probe::AudioProbe) -> Self {
        Self {
            transport: probe.transport,
            container: probe.container,
            container_long: probe.container_long,
            codec: probe.codec,
            codec_long: probe.codec_long,
            bitrate_kbps: probe.bitrate_kbps,
            sample_rate_hz: probe.sample_rate_hz,
            bit_depth: probe.bit_depth,
            channels: probe.channels,
            duration_secs: probe.duration_secs,
            size_bytes: probe.size_bytes,
        }
    }
}

#[derive(Deserialize, DartSignal)]
pub struct ProbeStreamRequest {
    pub id: String,
    pub track_id: Option<String>,
    pub track: Option<TrackInfo>,
    pub bitrate: Option<String>,
}

#[derive(Serialize, RustSignal)]
pub struct ProbeStreamResponse {
    pub id: String,
    /// `direct`, `transcode`, `local`, `preview`, or `external`.
    pub mode: Option<String>,
    pub info: Option<AudioProbe>,
    pub error: Option<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct PreviewTrackRequest {
    pub id: String,
    pub track: TrackInfo,
}

#[derive(Serialize, RustSignal)]
pub struct PreviewTrackResponse {
    pub id: String,
    pub url: Option<String>,
    pub source: Option<String>,
    pub error: Option<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct ReportPlaybackRequest {
    pub id: String,
    pub user_id: String,
    pub track_id: String,
    pub played_at: String,
    pub source: String,
}

#[derive(Serialize, RustSignal)]
pub struct ReportPlaybackResponse {
    pub id: String,
    pub success: bool,
}

#[derive(Deserialize, DartSignal)]
pub struct GetHistoryRequest {
    pub id: String,
    pub user_id: String,
    pub limit: Option<i32>,
}

#[derive(Serialize, RustSignal)]
pub struct GetHistoryResponse {
    pub id: String,
    pub records: Vec<PlaybackRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SignalPiece)]
pub struct PlaybackRecord {
    pub id: String,
    pub track_id: String,
    pub track_title: String,
    pub album_title: String,
    pub artist_name: String,
    pub played_at: String,
    pub source: String,
    pub scrobbled: bool,
    pub duration_secs: Option<f64>,
}

#[derive(Deserialize, DartSignal)]
pub struct UpdateNowPlayingRequest {
    pub id: String,
    pub user_id: String,
    pub track_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct UpdateNowPlayingResponse {
    pub id: String,
    pub success: bool,
}
