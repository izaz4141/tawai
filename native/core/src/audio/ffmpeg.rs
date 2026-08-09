use std::time::Duration;
use std::time::UNIX_EPOCH;

use anyhow::Context as _;
use tokio::fs;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// Default DASH segment duration in seconds.
pub const DASH_SEGMENT_DURATION_SECS: u32 = 6;

/// Default adaptive bitrate ladder (kbps) for DASH representations.
pub const DASH_BITRATES: &[u32] = &[128, 192, 256, 320];

/// Transcode one `DASH_SEGMENT_DURATION_SECS`-second slice of `source_path`
/// at `bitrate_kbps` into a single fMP4 (CMAF) DASH media segment plus its
/// init segment.
///
/// The slice covers `[start_secs, start_secs + DASH_SEGMENT_DURATION_SECS)`.
/// On success `out_dir` contains `init-<bandwidth>.m4s` and
/// `seg-<bandwidth>-<number>.m4s`. The init segment is identical for every
/// segment of a representation, so it is (re)written on each call. Work
/// happens in a temporary subdirectory and is renamed into place, so a
/// partial or failed run never leaves a corrupt artifact behind.
pub async fn generate_dash_segment(
    source_path: &str,
    out_dir: &str,
    bitrate_kbps: u32,
    segment_number: u32,
    start_secs: f64,
) -> anyhow::Result<()> {
    if !fs::try_exists(out_dir).await.unwrap_or(false) {
        fs::create_dir_all(out_dir).await?;
    }

    let bandwidth = bitrate_kbps * 1000;
    let work_dir = format!("{}/.work-{}", out_dir, segment_number);
    if fs::try_exists(&work_dir).await.unwrap_or(false) {
        fs::remove_dir_all(&work_dir).await?;
    }
    fs::create_dir_all(&work_dir).await?;

    let result = transcode_dash_slice(
        source_path,
        &work_dir,
        out_dir,
        bitrate_kbps,
        segment_number,
        start_secs,
    )
    .await;
    let _ = fs::remove_dir_all(&work_dir).await;
    result
}

/// Run the per-slice ffmpeg transcode inside `work_dir`, renaming the produced
/// artifacts into `out_dir`. `work_dir` is always removed by the caller, so a
/// failed run never leaves a partial slice behind.
async fn transcode_dash_slice(
    source_path: &str,
    work_dir: &str,
    out_dir: &str,
    bitrate_kbps: u32,
    segment_number: u32,
    start_secs: f64,
) -> anyhow::Result<()> {
    let bandwidth = bitrate_kbps * 1000;
    let init_name = format!("init-{}.m4s", bandwidth);
    let media_name = format!("seg-{}-$Number$.m4s", bandwidth);
    let manifest = format!("{}/manifest.mpd", work_dir);

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-ss")
        .arg(format!("{:.6}", start_secs))
        .arg("-i")
        .arg(source_path)
        .arg("-map")
        .arg("0:a:0")
        .arg("-t")
        .arg(DASH_SEGMENT_DURATION_SECS.to_string())
        .arg("-c:a")
        .arg("aac")
        .arg("-ac")
        .arg("2")
        .arg("-ar")
        .arg("48000")
        .arg("-b:a")
        .arg(format!("{}k", bitrate_kbps))
        .arg("-seg_duration")
        .arg(DASH_SEGMENT_DURATION_SECS.to_string())
        .arg("-use_timeline")
        .arg("0")
        .arg("-use_template")
        .arg("1")
        .arg("-adaptation_sets")
        .arg("id=0,streams=a")
        .arg("-init_seg_name")
        .arg(&init_name)
        .arg("-media_seg_name")
        .arg(&media_name)
        .arg("-f")
        .arg("dash")
        .arg(&manifest)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd
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
    let _ = read.await;

    let status = child.wait().await?;
    if !status.success() {
        let detail = if buf.trim().is_empty() {
            status.to_string()
        } else {
            buf.trim().to_string()
        };
        anyhow::bail!(
            "ffmpeg DASH segment generation failed for {}: {}",
            source_path,
            detail
        );
    }

    let seg_src = format!("{}/seg-{}-1.m4s", work_dir, bandwidth);
    let seg_dst = format!("{}/seg-{}-{}.m4s", out_dir, bandwidth, segment_number);
    if !fs::try_exists(&seg_src).await.unwrap_or(false) {
        anyhow::bail!(
            "ffmpeg DASH segment generation produced no segment for {}",
            source_path
        );
    }
    fs::rename(&seg_src, &seg_dst).await?;
    if segment_number > 1 {
        patch_segment_tfdt(&seg_dst, segment_number).await?;
    }
    let init_src = format!("{}/{}", work_dir, init_name);
    let init_dst = format!("{}/{}", out_dir, init_name);
    if fs::try_exists(&init_src).await.unwrap_or(false) {
        let _ = fs::rename(&init_src, &init_dst).await;
    }

    Ok(())
}

/// Rewrite the `tfdt` box (`moof`/`traf`/`tfdt` -> `baseMediaDecodeTime`) of a
/// DASH media segment so it carries the segment's absolute position.
///
/// Every segment is transcoded independently, so ffmpeg normalizes
/// `baseMediaDecodeTime` to 0 and moves the start offset into the init
/// segment's edit list. Players fetch the init segment only once, so all
/// segments then read as zero-based and the presentation timeline is broken
/// (seeking hangs / playback stalls at segment boundaries). Rewriting the value
/// to `(segment_number - 1) * DASH_SEGMENT_DURATION_SECS` seconds in the track
/// timescale (48000) produces a continuous timeline identical to a single full
/// transcode. The patch is in-place and does not change the file size.
async fn patch_segment_tfdt(path: &str, segment_number: u32) -> anyhow::Result<()> {
    const TRACK_TIMESCALE: u64 = 48000;
    let delta = (segment_number as u64 - 1) * DASH_SEGMENT_DURATION_SECS as u64 * TRACK_TIMESCALE;
    let mut data = fs::read(path).await?;

    let be_u32 = |b: &[u8]| -> anyhow::Result<u32> {
        Ok(u32::from_be_bytes(
            b.try_into()
                .map_err(|_| anyhow::anyhow!("short box size"))?,
        ))
    };
    let be_u64 = |b: &[u8]| -> anyhow::Result<u64> {
        Ok(u64::from_be_bytes(
            b.try_into()
                .map_err(|_| anyhow::anyhow!("short tfdt value"))?,
        ))
    };

    let mut pos = 0usize;
    while pos + 8 <= data.len() {
        let size = be_u32(&data[pos..pos + 4])? as usize;
        if size < 8 || pos + size > data.len() {
            break;
        }
        if &data[pos + 4..pos + 8] == b"moof" {
            let (moof_start, moof_end) = (pos + 8, pos + size);
            let mut p = moof_start;
            while p + 8 <= moof_end {
                let size2 = be_u32(&data[p..p + 4])? as usize;
                if size2 < 8 || p + size2 > moof_end {
                    break;
                }
                if &data[p + 4..p + 8] == b"traf" {
                    let (traf_start, traf_end) = (p + 8, p + size2);
                    let mut q = traf_start;
                    while q + 8 <= traf_end {
                        let size3 = be_u32(&data[q..q + 4])? as usize;
                        if size3 < 8 || q + size3 > traf_end {
                            break;
                        }
                        if &data[q + 4..q + 8] == b"tfdt" && size3 >= 12 {
                            let version = data[q + 8];
                            if version == 1 && size3 >= 20 {
                                let current = be_u64(&data[q + 12..q + 20])?;
                                data[q + 12..q + 20]
                                    .copy_from_slice(&(current + delta).to_be_bytes());
                            } else if version == 0 && size3 >= 16 {
                                let current = be_u32(&data[q + 12..q + 16])?;
                                data[q + 12..q + 16]
                                    .copy_from_slice(&(current + delta as u32).to_be_bytes());
                            } else {
                                anyhow::bail!(
                                    "unsupported tfdt box (version={}, size={}) in {}",
                                    version,
                                    size3,
                                    path
                                );
                            }
                            fs::write(path, data).await?;
                            return Ok(());
                        }
                        q += size3;
                    }
                }
                p += size2;
            }
        }
        pos += size;
    }

    anyhow::bail!("no tfdt box found in {}", path);
}

async fn file_mtime_ms(path: &str) -> Option<u128> {
    fs::metadata(path)
        .await
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis())
}

/// Whether the DASH artifact at `artifact_path` is up to date for
/// `source_path`.
///
/// Returns `false` when the artifact is missing or the source file has been
/// modified more recently than the artifact (stale). When the source metadata
/// cannot be read (e.g. file deleted or on an unreachable mount) the artifact
/// is assumed fresh rather than destroyed; the missing-source case is handled
/// separately by `cache::cleanup_dash_cache` at init.
pub async fn dash_file_fresh(source_path: &str, artifact_path: &str) -> anyhow::Result<bool> {
    if !fs::try_exists(artifact_path).await.unwrap_or(false) {
        return Ok(false);
    }
    match (
        file_mtime_ms(source_path).await,
        file_mtime_ms(artifact_path).await,
    ) {
        (Some(source_mtime), Some(artifact_mtime)) => Ok(source_mtime <= artifact_mtime),
        _ => Ok(true),
    }
}

/// Integrated loudness (LUFS) and true peak (dBFS) of an audio file.
#[derive(Debug)]
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
