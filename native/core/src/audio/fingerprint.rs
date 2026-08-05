use anyhow::{Context, Result};
use chromaprint::{Algorithm, Fingerprinter};
use std::path::Path;
use symphonia::core::audio::Audio;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

#[derive(Debug, Clone)]
pub struct FingerprintResult {
    pub fingerprint: String,
    pub duration: f64,
}

pub fn compute_fingerprint(path: &Path) -> Result<FingerprintResult> {
    let src = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let mut format = symphonia::default::get_probe().probe(
        &hint,
        mss,
        FormatOptions::default(),
        MetadataOptions::default(),
    )?;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.as_ref().is_some_and(|p| p.is_audio()))
        .context("No audio track found")?;

    let codec_params = track.codec_params.clone();
    let audio_params = codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .context("No audio codec parameters")?
        .clone();

    let sample_rate = audio_params.sample_rate.unwrap_or(44100);
    let num_channels = audio_params
        .channels
        .as_ref()
        .map(|c| c.count() as u16)
        .unwrap_or(2);

    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&audio_params, &AudioDecoderOptions::default())?;

    let mut fingerprinter = Fingerprinter::new(Algorithm::default());
    fingerprinter.start(sample_rate, num_channels)?;

    let mut total_frames: usize = 0;

    loop {
        let packet = match format.next_packet() {
            Ok(Some(pkt)) => pkt,
            Ok(None) => break,
            Err(_) => break,
        };

        if packet.track_id != track_id {
            continue;
        }

        if let Ok(decoded) = decoder.decode(&packet) {
            let frames = decoded.frames();
            if frames == 0 {
                continue;
            }

            let spec = decoded.spec().clone();
            let ch_count = spec.channels().count();

            let mut interleaved = Vec::with_capacity(frames * ch_count);
            for f in 0..frames {
                for c in 0..ch_count {
                    let sample: i16 = match &decoded {
                        symphonia::core::audio::GenericAudioBufferRef::S8(buf) => {
                            (buf.plane(c).unwrap_or(&[])[f] as i16) << 8
                        }
                        symphonia::core::audio::GenericAudioBufferRef::S16(buf) => {
                            buf.plane(c).unwrap_or(&[])[f]
                        }
                        symphonia::core::audio::GenericAudioBufferRef::S24(buf) => {
                            (buf.plane(c).unwrap_or(&[])[f].0 >> 8) as i16
                        }
                        symphonia::core::audio::GenericAudioBufferRef::S32(buf) => {
                            (buf.plane(c).unwrap_or(&[])[f] >> 16) as i16
                        }
                        symphonia::core::audio::GenericAudioBufferRef::U8(buf) => {
                            (buf.plane(c).unwrap_or(&[])[f] as i16 - 128) << 8
                        }
                        symphonia::core::audio::GenericAudioBufferRef::U16(buf) => {
                            (buf.plane(c).unwrap_or(&[])[f] as i32 - 32768) as i16
                        }
                        symphonia::core::audio::GenericAudioBufferRef::U24(buf) => {
                            ((buf.plane(c).unwrap_or(&[])[f].0 as i64 - 8388608) >> 8) as i16
                        }
                        symphonia::core::audio::GenericAudioBufferRef::U32(buf) => {
                            ((buf.plane(c).unwrap_or(&[])[f] as i64 >> 16) - 32768) as i16
                        }
                        symphonia::core::audio::GenericAudioBufferRef::F32(buf) => {
                            (buf.plane(c).unwrap_or(&[])[f] * 32768.0) as i16
                        }
                        symphonia::core::audio::GenericAudioBufferRef::F64(buf) => {
                            (buf.plane(c).unwrap_or(&[])[f] * 32768.0) as i16
                        }
                    };
                    interleaved.push(sample);
                }
            }

            if !interleaved.is_empty() {
                fingerprinter.feed(&interleaved)?;
                total_frames += frames;
            }
        }
    }

    fingerprinter.finish()?;

    let fingerprint = fingerprinter.encode();
    let duration = total_frames as f64 / sample_rate as f64;

    Ok(FingerprintResult {
        fingerprint,
        duration,
    })
}

pub fn fingerprint_supported_format(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "wav"
            | "flac"
            | "mp3"
            | "ogg"
            | "oga"
            | "opus"
            | "m4a"
            | "m4b"
            | "aac"
            | "aiff"
            | "wma"
            | "webm"
            | "mkv"
            | "caf"
    )
}
