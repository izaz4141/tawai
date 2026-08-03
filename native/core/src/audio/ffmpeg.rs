use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

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
        .arg("ebur128")
        .arg("-f")
        .arg("null")
        .arg("-")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

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

    let mut integrated: Option<f64> = None;
    let mut peak: Option<f64> = None;
    for line in buf.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("I:") {
            let num = rest.trim().strip_suffix("LUFS").map(|s| s.trim());
            if let Some(num) = num {
                if let Ok(v) = num.replace('\u{2212}', "-").parse::<f64>() {
                    integrated = Some(v);
                }
            }
        } else if let Some(rest) = line.strip_prefix("Peak:") {
            let num = rest.trim().strip_suffix("dBFS").map(|s| s.trim());
            if let Some(num) = num {
                if let Ok(v) = num.replace('\u{2212}', "-").parse::<f64>() {
                    peak = Some(v);
                }
            }
        }
    }

    Ok(LoudnessMeasurement {
        integrated_lufs: integrated
            .ok_or_else(|| anyhow::anyhow!("could not parse ebur128 loudness for {}", path))?,
        peak_dbfs: peak
            .ok_or_else(|| anyhow::anyhow!("could not parse ebur128 true peak for {}", path))?,
    })
}
