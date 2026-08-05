use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use anyhow::Context as _;
use futures::Stream;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::{Child, Command};
use tokio_util::bytes::Bytes;
use tokio_util::io::ReaderStream;

fn output_format(source_path: &str) -> &'static str {
    let ext = Path::new(source_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "flac" => "flac",
        "ogg" => "ogg",
        "opus" => "opus",
        "m4a" | "aac" => "ipod",
        "wav" => "wav",
        "mp3" => "mp3",
        "webm" => "webm",
        _ => "mp3",
    }
}

pub struct TranscodeStream {
    _child: Child,
    stream: ReaderStream<Box<dyn AsyncRead + Unpin + Send>>,
}

impl Stream for TranscodeStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.stream).poll_next(cx)
    }
}

pub fn transcode_stream(source_path: &str, bitrate: &str) -> anyhow::Result<TranscodeStream> {
    let fmt = output_format(source_path);
    let mut child = Command::new("ffmpeg")
        .arg("-i")
        .arg(source_path)
        .arg("-f")
        .arg(fmt)
        .arg("-b:a")
        .arg(format!("{}k", bitrate))
        .arg("-")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("failed to capture ffmpeg stdout"))?;

    let stream = ReaderStream::new(Box::new(stdout) as Box<dyn AsyncRead + Unpin + Send>);

    Ok(TranscodeStream {
        _child: child,
        stream,
    })
}

/// Integrated loudness (LUFS) and true peak (dBFS) of an audio file.
pub struct LoudnessMeasurement {
    pub integrated_lufs: f64,
    pub peak_dbfs: f64,
}

/// Measure integrated loudness (EBU R128 / LUFS) and true peak (dBFS) of an
/// audio file via ffmpeg's ebur128 filter.
pub async fn measure_loudness(path: &str) -> anyhow::Result<LoudnessMeasurement> {
    let mut child = Command::new("ffmpeg")
        .arg("-nostdin")
        .arg("-i")
        .arg(path)
        .arg("-filter:a")
        .arg("ebur128=peak=true")
        .arg("-f")
        .arg("null")
        .arg("-")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to run ffmpeg (is it installed?): {}", e))?;

    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("failed to capture ffmpeg stderr"))?;
    let mut buf = String::new();

    let read = async {
        AsyncReadExt::read_to_string(&mut tokio::io::BufReader::new(stderr), &mut buf).await
    };
    match tokio::time::timeout(Duration::from_secs(60), read).await {
        Ok(_) => {}
        Err(_) => {
            let _ = child.kill().await;
            anyhow::bail!("ffmpeg loudness measurement timed out for {}", path);
        }
    }

    let status = child.wait().await?;
    if !status.success() {
        anyhow::bail!("ffmpeg failed for {}: {}", path, status);
    }

    parse_ebur128_output(&buf).with_context(|| format!("loudness measurement for {}", path))
}

/// Parse ffmpeg's ebur128 stderr output into a [`LoudnessMeasurement`].
///
/// Tolerates both legacy output, where every line carries a
/// `[Parsed_ebur128_0 @ 0x...]` prefix, and modern output, where the summary
/// block is printed bare. Per-frame lines (`t: ... I: ... Peak: ...`) are
/// ignored; only the summary block values are used, and since it is printed
/// last, the last `I:`/`Peak:` match wins.
pub fn parse_ebur128_output(stderr: &str) -> anyhow::Result<LoudnessMeasurement> {
    let mut integrated: Option<f64> = None;
    let mut peak: Option<f64> = None;
    let mut unmeasurable = false;

    for line in stderr.lines() {
        let content = strip_filter_prefix(line);
        if let Some(rest) = content.strip_prefix("I:") {
            match parse_db_value(rest, "LUFS") {
                DbValue::Finite(v) => integrated = Some(v),
                DbValue::Unmeasurable => unmeasurable = true,
                DbValue::Unparseable => {}
            }
        } else if let Some(rest) = content.strip_prefix("Peak:") {
            match parse_db_value(rest, "dBFS") {
                DbValue::Finite(v) => peak = Some(v),
                DbValue::Unmeasurable => unmeasurable = true,
                DbValue::Unparseable => {}
            }
        }
    }

    let integrated = integrated.ok_or_else(|| {
        if unmeasurable {
            anyhow::anyhow!("audio file appears to be silent (no measurable loudness)")
        } else {
            anyhow::anyhow!("could not parse ebur128 loudness from ffmpeg output")
        }
    })?;
    let peak = peak.ok_or_else(|| {
        if unmeasurable {
            anyhow::anyhow!("audio file appears to be silent (no measurable true peak)")
        } else {
            anyhow::anyhow!("could not parse ebur128 true peak from ffmpeg output")
        }
    })?;

    Ok(LoudnessMeasurement {
        integrated_lufs: integrated,
        peak_dbfs: peak,
    })
}

/// Remove the `[Parsed_ebur128_0 @ 0x...]` prefix found on legacy ffmpeg
/// output lines (modern ffmpeg prints the summary block without it).
fn strip_filter_prefix(line: &str) -> &str {
    let content = line.trim_start();
    match content.strip_prefix('[') {
        Some(rest) => match rest.split_once("] ") {
            Some((_, after)) => after.trim_start(),
            None => content,
        },
        None => content,
    }
}

/// A dB value parsed from ffmpeg's ebur128 summary.
#[derive(Debug, Clone, Copy, PartialEq)]
enum DbValue {
    /// A finite value (e.g. `-7.9`).
    Finite(f64),
    /// `-inf`, `+inf` or `nan` — no measurable signal (e.g. silence).
    Unmeasurable,
    /// Not a number at all (unexpected output shape).
    Unparseable,
}

/// Parse a dB value such as `"-7.9 LUFS"` or `" 1.9 dBFS"`, tolerating the
/// U+2212 minus sign emitted by some toolchains and case differences in the
/// unit suffix.
fn parse_db_value(s: &str, unit: &str) -> DbValue {
    let trimmed = s.trim();
    let lower = trimmed.to_ascii_lowercase();
    let unit_lower = unit.to_ascii_lowercase();
    let number = match lower.strip_suffix(&unit_lower) {
        Some(stripped) => &trimmed[..stripped.len()],
        None => trimmed,
    };
    match number.trim().replace('\u{2212}', "-").parse::<f64>() {
        Ok(v) if v.is_finite() => DbValue::Finite(v),
        Ok(_) => DbValue::Unmeasurable,
        Err(_) => DbValue::Unparseable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEGACY_OUTPUT: &str = "\
[Parsed_ebur128_0 @ 0x558b3b0f04c0] t: 0.5  I: -50.0 LUFS  LRA: 0.0 LU  Peak: -20.0 dBFS
[Parsed_ebur128_0 @ 0x558b3b0f04c0] Summary:
[Parsed_ebur128_0 @ 0x558b3b0f04c0] Integrated loudness:
[Parsed_ebur128_0 @ 0x558b3b0f04c0]   I:         -13.8 LUFS
[Parsed_ebur128_0 @ 0x558b3b0f04c0]   Threshold: -26.8 LUFS
[Parsed_ebur128_0 @ 0x558b3b0f04c0] Loudness range:
[Parsed_ebur128_0 @ 0x558b3b0f04c0]   LRA:         2.0 LU
[Parsed_ebur128_0 @ 0x558b3b0f04c0] True peak:
[Parsed_ebur128_0 @ 0x558b3b0f04c0]   Peak:       -0.8 dBFS
";

    const MODERN_OUTPUT: &str = "\
[Parsed_ebur128_0 @ 0x7f20f80018c0] t: 233.399977 TARGET:-23 LUFS    M:   nan S:-138.7     I:  -7.9 LUFS       LRA:   5.3 LU
[Parsed_ebur128_0 @ 0x7f20f80018c0] Summary:

  Integrated loudness:
    I:          -7.9 LUFS
    Threshold: -17.9 LUFS

  Loudness range:
    LRA:         5.3 LU

  True peak:
    Peak:        1.9 dBFS
";

    const SILENT_OUTPUT: &str = "\
  Integrated loudness:
    I:          -inf LUFS
    Threshold:   0.0 LUFS

  Loudness range:
    LRA:         0.0 LU

  True peak:
    Peak:       -inf dBFS
";

    #[test]
    fn parses_legacy_output() {
        let m = parse_ebur128_output(LEGACY_OUTPUT).unwrap();
        assert_eq!(m.integrated_lufs, -13.8);
        assert_eq!(m.peak_dbfs, -0.8);
    }

    #[test]
    fn parses_modern_output() {
        let m = parse_ebur128_output(MODERN_OUTPUT).unwrap();
        assert_eq!(m.integrated_lufs, -7.9);
        assert_eq!(m.peak_dbfs, 1.9);
    }

    #[test]
    fn ignores_frame_lines_when_summary_present() {
        let out = "\
[Parsed_ebur128_0 @ 0x7f20f80018c0] t: 233.399977 TARGET:-23 LUFS    I:  -7.9 LUFS       LRA:   5.3 LU  TPK:   1.9   1.5 dBFS
[Parsed_ebur128_0 @ 0x7f20f80018c0] Summary:

  Integrated loudness:
    I:          -6.0 LUFS

  True peak:
    Peak:        0.5 dBFS
";
        let m = parse_ebur128_output(out).unwrap();
        assert_eq!(m.integrated_lufs, -6.0);
        assert_eq!(m.peak_dbfs, 0.5);
    }

    #[test]
    fn tolerates_unicode_minus() {
        let out = MODERN_OUTPUT.replace("-7.9 LUFS", "\u{2212}7.9 LUFS");
        let m = parse_ebur128_output(&out).unwrap();
        assert_eq!(m.integrated_lufs, -7.9);
    }

    #[test]
    fn rejects_silent_output() {
        let err = parse_ebur128_output(SILENT_OUTPUT).unwrap_err();
        assert!(format!("{err:#}").contains("silent"), "{err:#}");
    }
}
