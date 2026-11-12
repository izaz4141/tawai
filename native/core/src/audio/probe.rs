use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use lofty::file::AudioFile;
use serde::{Deserialize, Serialize};
use symphonia::core::formats::FormatOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::{TimeBase, Timestamp};
use tokio::process::Command;
use utoipa::ToSchema;

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

/// Probe a remote URL with `ffprobe`, forwarding the same headers used for
/// playback so authenticated sources can be read.
pub async fn probe_url(url: &str, headers: &[(String, String)]) -> Result<AudioProbe> {
    let mut probe = probe_with_ffprobe(url, headers).await?;
    probe.transport = Some("http".to_string());
    Ok(probe)
}

/// Probe a generated DASH init segment + media segment pair.
///
/// The two boxes are concatenated into a temporary fMP4 file so `ffprobe` can
/// read a complete stream. The bitrate is recomputed from the real media
/// segment byte count and its measured duration instead of trusting the
/// requested transcode bitrate.
pub async fn probe_dash_artifacts(
    init_path: &Path,
    segment_path: &Path,
    segment_duration_secs: Option<f64>,
) -> Result<AudioProbe> {
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

    let mut probe = probe_with_ffprobe(&probe_path.to_string_lossy(), &[]).await?;
    probe.transport = Some("dash".to_string());
    probe.size_bytes = Some(segment.len() as i64);

    let duration = probe.duration_secs.or(segment_duration_secs);
    if let Some(duration) = duration.filter(|duration| *duration > 0.0) {
        probe.bitrate_kbps =
            Some(((segment.len() as f64 * 8.0 / duration) / 1000.0).round() as i64);
        probe.duration_secs.get_or_insert(duration);
    }

    Ok(probe)
}

async fn probe_with_ffprobe(input: &str, headers: &[(String, String)]) -> Result<AudioProbe> {
    let mut command = Command::new("ffprobe");
    command
        .arg("-v")
        .arg("error")
        .arg("-print_format")
        .arg("json")
        .arg("-show_streams")
        .arg("-show_format")
        .arg("-rw_timeout")
        .arg("15000000");

    if !headers.is_empty() {
        let mut header_blob = String::new();
        for (name, value) in headers {
            // ffprobe expects CRLF-separated header lines.
            header_blob.push_str(name);
            header_blob.push_str(": ");
            header_blob.push_str(value);
            header_blob.push_str("\r\n");
        }
        command.arg("-headers").arg(header_blob);
    }

    command
        .arg(input)
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = tokio::time::timeout(Duration::from_secs(20), command.output())
        .await
        .map_err(|_| anyhow::anyhow!("ffprobe timed out"))?
        .with_context(|| format!("failed to run ffprobe (is it installed?) for {input}"))?;

    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        bail!("ffprobe failed for {input}: {}", detail.trim());
    }

    parse_ffprobe_json(&output.stdout)
}

#[derive(Deserialize)]
struct FfprobeOutput {
    #[serde(default)]
    streams: Vec<FfprobeStream>,
    format: Option<FfprobeFormat>,
}

#[derive(Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    codec_long_name: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u32>,
    bits_per_raw_sample: Option<String>,
    bits_per_sample: Option<u32>,
    bit_rate: Option<String>,
    duration: Option<String>,
}

#[derive(Deserialize)]
struct FfprobeFormat {
    format_name: Option<String>,
    format_long_name: Option<String>,
    duration: Option<String>,
    size: Option<String>,
    bit_rate: Option<String>,
}

fn parse_ffprobe_json(bytes: &[u8]) -> Result<AudioProbe> {
    let output: FfprobeOutput =
        serde_json::from_slice(bytes).context("parsing ffprobe JSON output")?;

    let stream = output
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("audio"))
        .or_else(|| output.streams.first());

    let mut probe = AudioProbe::default();
    if let Some(format) = &output.format {
        if let Some(name) = &format.format_name {
            // Containers such as MP4 report a comma-separated list of names.
            probe.container = Some(name.split(',').next().unwrap_or(name).to_string());
        }
        probe.container_long = format.format_long_name.clone();
        probe.duration_secs = format.duration.as_deref().and_then(parse_f64);
        probe.size_bytes = format.size.as_deref().and_then(parse_i64);
        probe.bitrate_kbps = format.bit_rate.as_deref().and_then(parse_i64).map(to_kbps);
    }

    if let Some(stream) = stream {
        probe.codec = stream.codec_name.clone();
        probe.codec_long = stream.codec_long_name.clone();
        probe.sample_rate_hz = stream.sample_rate.as_deref().and_then(parse_u32);
        probe.channels = stream.channels;
        probe.bit_depth = stream
            .bits_per_raw_sample
            .as_deref()
            .and_then(parse_u32)
            .or(stream.bits_per_sample);
        if probe.duration_secs.is_none() {
            probe.duration_secs = stream.duration.as_deref().and_then(parse_f64);
        }
        if let Some(bitrate) = stream.bit_rate.as_deref().and_then(parse_i64) {
            probe.bitrate_kbps = Some(to_kbps(bitrate));
        }
    }

    Ok(probe)
}

fn to_kbps(bits_per_second: i64) -> i64 {
    (bits_per_second as f64 / 1000.0).round() as i64
}

fn parse_f64(value: &str) -> Option<f64> {
    value.trim().parse().ok()
}

fn parse_i64(value: &str) -> Option<i64> {
    value.trim().parse().ok()
}

fn parse_u32(value: &str) -> Option<u32> {
    value.trim().parse().ok()
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

    #[test]
    fn parses_ffprobe_json() {
        let json = br#"{
            "streams": [{
                "codec_type": "audio",
                "codec_name": "flac",
                "codec_long_name": "Free Lossless Audio Codec",
                "sample_rate": "96000",
                "channels": 2,
                "bits_per_raw_sample": "24",
                "bit_rate": "2304000",
                "duration": "12.5"
            }],
            "format": {
                "format_name": "flac",
                "format_long_name": "raw FLAC",
                "duration": "12.5",
                "size": "3600000",
                "bit_rate": "2304000"
            }
        }"#;

        let probe = parse_ffprobe_json(json).unwrap();

        assert_eq!(probe.container.as_deref(), Some("flac"));
        assert_eq!(probe.codec.as_deref(), Some("flac"));
        assert_eq!(
            probe.codec_long.as_deref(),
            Some("Free Lossless Audio Codec")
        );
        assert_eq!(probe.sample_rate_hz, Some(96_000));
        assert_eq!(probe.channels, Some(2));
        assert_eq!(probe.bit_depth, Some(24));
        assert_eq!(probe.bitrate_kbps, Some(2304));
        assert_eq!(probe.duration_secs, Some(12.5));
        assert_eq!(probe.size_bytes, Some(3_600_000));
    }
}
