use crate::audio::ffmpeg::{DASH_BITRATES, DASH_SEGMENT_DURATION_SECS};

/// Build a VOD-style static MPD describing the on-demand transcode output of
/// `crate::audio::ffmpeg::generate_dash_segment`: one audio `AdaptationSet`
/// with a `SegmentTemplate` addressing `init-$Bandwidth$.m4s` and
/// `seg-$Bandwidth$-$Number$.m4s` (1-based numbers, one
/// `DASH_SEGMENT_DURATION_SECS`-second segment each).
///
/// When `duration_secs` is `None` or non-positive, the `mediaPresentationDuration`
/// attribute is omitted and the manifest is served as-is (players will then
/// estimate the total length from downloaded segments).
pub fn build_dash_manifest(duration_secs: Option<f64>) -> String {
    let duration_attr = duration_secs
        .filter(|d| *d > 0.0)
        .map(|d| {
            format!(
                "\n\tmediaPresentationDuration=\"{}\"",
                format_dash_duration(d)
            )
        })
        .unwrap_or_default();

    let representations = DASH_BITRATES
        .iter()
        .map(|&bitrate| {
            let bandwidth = bitrate * 1000;
            format!(
                "\t\t\t<Representation id=\"{bandwidth}\" mimeType=\"audio/mp4\" codecs=\"mp4a.40.2\" bandwidth=\"{bandwidth}\" audioSamplingRate=\"48000\">\n\
                 \t\t\t\t<AudioChannelConfiguration schemeIdUri=\"urn:mpeg:dash:23003:3:audio_channel_configuration:2011\" value=\"2\" />\n\
                 \t\t\t\t<SegmentTemplate timescale=\"1000000\" duration=\"{}\" initialization=\"init-$Bandwidth$.m4s\" media=\"seg-$Bandwidth$-$Number$.m4s\" startNumber=\"1\">\n\
                 \t\t\t\t</SegmentTemplate>\n\
                 \t\t\t</Representation>",
                DASH_SEGMENT_DURATION_SECS * 1_000_000,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
<MPD xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\n\
\txmlns=\"urn:mpeg:dash:schema:mpd:2011\"\n\
\txmlns:xlink=\"http://www.w3.org/1999/xlink\"\n\
\txsi:schemaLocation=\"urn:mpeg:DASH:schema:MPD:2011 http://standards.iso.org/ittf/PubliclyAvailableStandards/MPEG-DASH_schema_files/DASH-MPD.xsd\"\n\
\tprofiles=\"urn:mpeg:dash:profile:isoff-live:2011\"\n\
\ttype=\"static\"{duration_attr}\n\
\tmaxSegmentDuration=\"PT{DASH_SEGMENT_DURATION_SECS}.0S\"\n\
\tminBufferTime=\"PT10.0S\">\n\
\t<Period id=\"0\" start=\"PT0.0S\">\n\
\t\t<AdaptationSet id=\"0\" contentType=\"audio\" startWithSAP=\"1\" segmentAlignment=\"true\" bitstreamSwitching=\"true\">\n\
{representations}\n\
\t\t</AdaptationSet>\n\
\t</Period>\n\
</MPD>"
    )
}

/// Format seconds as an ISO-8601 `PT<n>S` duration with up to 3 decimal
/// places (e.g. `90.5` -> `PT90.5S`).
fn format_dash_duration(secs: f64) -> String {
    let rounded = (secs * 1000.0).round() / 1000.0;
    format!("PT{}S", rounded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_static_manifest_with_duration() {
        let mpd = build_dash_manifest(Some(90.5));
        assert!(mpd.contains("type=\"static\""));
        assert!(mpd.contains("mediaPresentationDuration=\"PT90.5S\""));
        for &bw in DASH_BITRATES {
            let bandwidth = bw * 1000;
            assert!(mpd.contains(&format!("bandwidth=\"{bandwidth}\"")));
            assert!(mpd.contains("media=\"seg-$Bandwidth$-$Number$.m4s\""));
        }
    }

    #[test]
    fn omits_duration_when_unknown() {
        let mpd = build_dash_manifest(None);
        assert!(!mpd.contains("mediaPresentationDuration"));
        let mpd = build_dash_manifest(Some(0.0));
        assert!(!mpd.contains("mediaPresentationDuration"));
    }

    #[test]
    fn formats_duration() {
        assert_eq!(format_dash_duration(30.0), "PT30S");
        assert_eq!(format_dash_duration(30.5), "PT30.5S");
        assert_eq!(format_dash_duration(30.1234), "PT30.123S");
    }
}
