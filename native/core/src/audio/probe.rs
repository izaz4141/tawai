use std::path::Path;
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
use std::sync::Once;

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
use anyhow::Context;
use anyhow::{Result, bail};
use lofty::file::AudioFile;
use serde::{Deserialize, Serialize};
use symphonia::core::formats::FormatOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::{TimeBase, Timestamp};
use utoipa::ToSchema;

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
use crate::utils::url::redact_urls;

/// Metadata read from an actual audio file or stream.
///
/// Every field is optional: a probe that cannot determine a value must leave it
/// as `None` instead of guessing. The caller decides how to present partial
/// results.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(default)]
pub struct AudioProbe {
    /// How the media was read: `file`, `http`, `dash`, or `preview`.
    pub transport: Option<String>,
    /// Short container/format name reported by the probe.
    pub container: Option<String>,
    /// Long container/format name reported by the probe.
    pub container_long: Option<String>,
    /// Short codec name reported by the probe.
    pub codec: Option<String>,
    /// Long codec name reported by the probe.
    pub codec_long: Option<String>,
    /// Audio bitrate in kbps. For VBR files this is an average.
    pub bitrate_kbps: Option<i64>,
    /// Sample rate in Hz.
    pub sample_rate_hz: Option<u32>,
    /// Bit depth in bits, when reported.
    pub bit_depth: Option<u32>,
    /// Channel count.
    pub channels: Option<u32>,
    /// Duration in seconds.
    pub duration_secs: Option<f64>,
    /// Size of the probed source in bytes.
    pub size_bytes: Option<i64>,
}

impl AudioProbe {
    pub fn has_details(&self) -> bool {
        self.container.is_some()
            || self.codec.is_some()
            || self.bitrate_kbps.is_some()
            || self.sample_rate_hz.is_some()
            || self.bit_depth.is_some()
            || self.channels.is_some()
    }
}

/// Probe a local audio file with `lofty` for stream properties and `symphonia`
/// for container/codec identification.
pub fn probe_file(path: &Path) -> Result<AudioProbe> {
    let mut probe = AudioProbe {
        transport: Some("file".to_string()),
        size_bytes: std::fs::metadata(path)
            .ok()
            .map(|metadata| metadata.len() as i64),
        ..AudioProbe::default()
    };

    let mut read_something = false;

    match lofty::read_from_path(path) {
        Ok(tagged_file) => {
            read_something = true;
            let properties = tagged_file.properties();
            let duration = properties.duration().as_secs_f64();
            if duration > 0.0 {
                probe.duration_secs = Some(duration);
            }
            probe.bitrate_kbps = properties
                .audio_bitrate()
                .or_else(|| properties.overall_bitrate())
                .map(|bitrate| bitrate as i64);
            probe.sample_rate_hz = properties.sample_rate();
            probe.bit_depth = properties.bit_depth().map(u32::from);
            probe.channels = properties.channels().map(u32::from);
        }
        Err(_) => {}
    }

    match open_symphonia(path) {
        Ok(format) => {
            read_something = true;
            let info = format.format_info();
            probe.container = Some(info.short_name.to_string());
            probe.container_long = Some(info.long_name.to_string());

            if let Some(track) = format
                .tracks()
                .iter()
                .find(|track| track.codec_params.as_ref().is_some_and(|p| p.is_audio()))
            {
                if let Some(params) = track.codec_params.as_ref().and_then(|p| p.audio()) {
                    if let Some(decoder) =
                        symphonia::default::get_codecs().get_audio_decoder(params.codec)
                    {
                        probe.codec = Some(decoder.codec.info.short_name.to_string());
                        probe.codec_long = Some(decoder.codec.info.long_name.to_string());
                    }
                    probe.sample_rate_hz = probe.sample_rate_hz.or(params.sample_rate);
                    probe.channels = probe
                        .channels
                        .or(params.channels.as_ref().map(|c| c.count() as u32));
                    probe.bit_depth = probe.bit_depth.or(params.bits_per_sample);
                }

                if probe.duration_secs.is_none() {
                    probe.duration_secs =
                        duration_from_ticks(track.time_base, track.duration.map(|d| d.get()));
                }
            }

            if probe.duration_secs.is_none() {
                let media = format.media_info();
                probe.duration_secs =
                    duration_from_ticks(media.time_base, media.duration.map(|d| d.get()));
            }
        }
        Err(_) => {}
    }

    if !read_something {
        bail!("Could not read audio metadata from {}", path.display());
    }

    // Derive an average bitrate from the real byte count when the container/
    // tag reader did not report one.
    if probe.bitrate_kbps.is_none() {
        if let (Some(size), Some(duration)) = (probe.size_bytes, probe.duration_secs) {
            if duration > 0.0 {
                probe.bitrate_kbps = Some(((size as f64 * 8.0 / duration) / 1000.0).round() as i64);
            }
        }
    }

    Ok(probe)
}

fn open_symphonia(path: &Path) -> Result<Box<dyn symphonia::core::formats::FormatReader>> {
    let source = std::fs::File::open(path)?;
    let media_source = MediaSourceStream::new(Box::new(source), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(extension);
    }
    Ok(symphonia::default::get_probe().probe(
        &hint,
        media_source,
        FormatOptions::default(),
        MetadataOptions::default(),
    )?)
}

fn duration_from_ticks(time_base: Option<TimeBase>, ticks: Option<u64>) -> Option<f64> {
    let time_base = time_base?;
    let ticks = i64::try_from(ticks?).ok()?;
    time_base
        .calc_time(Timestamp::new(ticks))
        .map(|time| time.as_secs_f64())
}

/// Probe a remote URL with the FFmpeg libraries, forwarding the same headers
/// used for playback so authenticated sources can be read. Opening the URL
/// in-process keeps credentials out of process arguments.
///
/// Only builds that link FFmpeg can probe; other targets return an error.
pub async fn probe_url(url: &str, headers: &[(String, String)]) -> Result<AudioProbe> {
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    {
        let url = url.to_string();
        let headers = headers.to_vec();
        let mut probe = tokio::task::spawn_blocking(move || probe_with_libav(&url, &headers))
            .await
            .map_err(|error| anyhow::anyhow!("probe task failed: {error}"))??;
        probe.transport = Some("http".to_string());
        Ok(probe)
    }

    #[cfg(any(target_os = "android", target_os = "ios", target_arch = "wasm32"))]
    {
        let _ = (url, headers);
        bail!("stream probing is not supported on this platform")
    }
}

/// Probe a generated DASH init segment + media segment pair.
///
/// The two boxes are concatenated into a temporary fMP4 file so the probe can
/// read a complete stream. The reported bitrate comes from the probed audio
/// stream; the media segment byte count and its duration are only used when the
/// probe does not report one.
pub async fn probe_dash_artifacts(
    init_path: &Path,
    segment_path: &Path,
    segment_duration_secs: Option<f64>,
) -> Result<AudioProbe> {
    #[cfg(any(target_os = "android", target_os = "ios", target_arch = "wasm32"))]
    {
        let _ = (init_path, segment_path, segment_duration_secs);
        bail!("stream probing is not supported on this platform")
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    {
        let init = tokio::fs::read(init_path)
            .await
            .with_context(|| format!("reading DASH init segment {}", init_path.display()))?;
        let segment = tokio::fs::read(segment_path)
            .await
            .with_context(|| format!("reading DASH media segment {}", segment_path.display()))?;

        let directory = tempfile::tempdir()?;
        let probe_path = directory.path().join("dash-probe.mp4");
        let mut bytes = Vec::with_capacity(init.len() + segment.len());
        bytes.extend_from_slice(&init);
        bytes.extend_from_slice(&segment);
        tokio::fs::write(&probe_path, &bytes).await?;

        let display = probe_path.display().to_string();
        let mut probe = tokio::task::spawn_blocking(move || probe_with_libav(&display, &[]))
            .await
            .map_err(|error| anyhow::anyhow!("probe task failed: {error}"))??;

        probe.transport = Some("dash".to_string());
        probe.size_bytes = Some(segment.len() as i64);

        let duration = probe
            .duration_secs
            .or(segment_duration_secs)
            .filter(|duration| *duration > 0.0);
        probe.duration_secs = duration;

        if probe.bitrate_kbps.is_none() {
            if let Some(duration) = duration {
                probe.bitrate_kbps =
                    Some(((segment.len() as f64 * 8.0 / duration) / 1000.0).round() as i64);
            }
        }

        Ok(probe)
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
fn probe_with_libav(input: &str, headers: &[(String, String)]) -> Result<AudioProbe> {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = ffmpeg_next::init();
        ffmpeg_next::format::network::init();
        ffmpeg_next::log::set_level(ffmpeg_next::log::Level::Quiet);
    });

    let mut options = ffmpeg_next::Dictionary::new();
    options.set("rw_timeout", "15000000");
    if !headers.is_empty() {
        let mut blob = String::new();
        for (name, value) in headers {
            blob.push_str(name);
            blob.push_str(": ");
            blob.push_str(value);
            blob.push_str("\r\n");
        }
        options.set("headers", &blob);
    }

    let context = ffmpeg_next::format::input_with_dictionary(input, options).map_err(|error| {
        anyhow::anyhow!("ffmpeg probe failed: {}", redact_urls(&error.to_string()))
    })?;

    let mut probe = AudioProbe::default();
    let format = context.format();
    probe.container = Some(format.name().to_string());
    if !format.description().is_empty() {
        probe.container_long = Some(format.description().to_string());
    }

    if context.duration() > 0 {
        probe.duration_secs =
            Some(context.duration() as f64 / f64::from(ffmpeg_next::ffi::AV_TIME_BASE));
    }
    if context.bit_rate() > 0 {
        probe.bitrate_kbps = Some(to_kbps(context.bit_rate()));
    }

    if let Some(stream) = context.streams().best(ffmpeg_next::media::Type::Audio) {
        let parameters = stream.parameters();
        if let Some(codec) = ffmpeg_next::codec::decoder::find(parameters.id()) {
            probe.codec = Some(codec.name().to_string());
            if !codec.description().is_empty() {
                probe.codec_long = Some(codec.description().to_string());
            }
        }

        // SAFETY: `parameters` borrows codec parameters owned by `context`,
        // which is alive for this block.
        let raw = unsafe { &*parameters.as_ptr() };
        if raw.sample_rate > 0 {
            probe.sample_rate_hz = Some(raw.sample_rate as u32);
        }
        if raw.ch_layout.nb_channels > 0 {
            probe.channels = Some(raw.ch_layout.nb_channels as u32);
        }
        let bits = if raw.bits_per_raw_sample > 0 {
            raw.bits_per_raw_sample
        } else {
            raw.bits_per_coded_sample
        };
        if bits > 0 {
            probe.bit_depth = Some(bits as u32);
        }
        if raw.bit_rate > 0 {
            probe.bitrate_kbps = Some(to_kbps(raw.bit_rate));
        }

        if probe.duration_secs.is_none() {
            let time_base = stream.time_base();
            let duration = stream.duration();
            if duration > 0 && time_base.denominator() > 0 {
                probe.duration_secs = Some(
                    duration as f64 * f64::from(time_base.numerator())
                        / f64::from(time_base.denominator()),
                );
            }
        }
    }

    Ok(probe)
}

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
fn to_kbps(bits_per_second: i64) -> i64 {
    (bits_per_second as f64 / 1000.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_pcm_wav(path: &Path, sample_rate: u32, channels: u16, bits: u16, seconds: u32) {
        let block_align = channels * (bits / 8);
        let byte_rate = sample_rate * u32::from(block_align);
        let data_len = byte_rate * seconds;
        let mut bytes = Vec::with_capacity(44 + data_len as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&block_align.to_le_bytes());
        bytes.extend_from_slice(&bits.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        bytes.resize(44 + data_len as usize, 0);
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn probes_pcm_wav_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tone.wav");
        write_pcm_wav(&path, 44_100, 1, 16, 1);

        let probe = probe_file(&path).unwrap();

        assert_eq!(probe.container.as_deref(), Some("wave"));
        assert_eq!(probe.sample_rate_hz, Some(44_100));
        assert_eq!(probe.channels, Some(1));
        assert_eq!(probe.bit_depth, Some(16));
        assert_eq!(probe.duration_secs, Some(1.0));
        assert_eq!(probe.bitrate_kbps, Some(706));
        assert_eq!(probe.size_bytes, Some(44 + 44_100 * 2));
        assert!(probe.has_details());
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    #[test]
    fn probes_pcm_wav_with_libav() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tone.wav");
        write_pcm_wav(&path, 48_000, 2, 16, 2);

        let probe = probe_with_libav(&path.display().to_string(), &[]).unwrap();

        assert_eq!(probe.container.as_deref(), Some("wav"));
        assert_eq!(probe.codec.as_deref(), Some("pcm_s16le"));
        assert_eq!(probe.sample_rate_hz, Some(48_000));
        assert_eq!(probe.channels, Some(2));
        assert_eq!(probe.bit_depth, Some(16));
        assert_eq!(probe.duration_secs, Some(2.0));
        assert_eq!(probe.bitrate_kbps, Some(1536));
        assert!(probe.has_details());
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    #[test]
    fn libav_probe_error_redacts_url() {
        let error = probe_with_libav("http://127.0.0.1:1/stream?token=secret", &[]).unwrap_err();

        assert!(!error.to_string().contains("secret"));
    }
}
