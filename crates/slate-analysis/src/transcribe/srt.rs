use crate::types::Transcript;
use slate_core::time::RationalTime;

fn format_srt_time(rt: RationalTime) -> String {
    let total_millis = (rt.to_seconds() * 1000.0).round() as u64;
    let hours = total_millis / 3_600_000;
    let mins = (total_millis % 3_600_000) / 60_000;
    let secs = (total_millis % 60_000) / 1000;
    let millis = total_millis % 1000;
    format!("{hours:02}:{mins:02}:{secs:02},{millis:03}")
}

fn format_vtt_time(rt: RationalTime) -> String {
    let total_millis = (rt.to_seconds() * 1000.0).round() as u64;
    let hours = total_millis / 3_600_000;
    let mins = (total_millis % 3_600_000) / 60_000;
    let secs = (total_millis % 60_000) / 1000;
    let millis = total_millis % 1000;
    format!("{hours:02}:{mins:02}:{secs:02}.{millis:03}")
}

pub fn export_srt(transcript: &Transcript) -> String {
    let mut out = String::new();
    for (idx, seg) in transcript.segments.iter().enumerate() {
        out.push_str(&(idx + 1).to_string());
        out.push('\n');
        out.push_str(&format!(
            "{} --> {}\n",
            format_srt_time(seg.start),
            format_srt_time(seg.end)
        ));
        out.push_str(&seg.text);
        out.push_str("\n\n");
    }
    out
}

pub fn export_vtt(transcript: &Transcript) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for (idx, seg) in transcript.segments.iter().enumerate() {
        out.push_str(&(idx + 1).to_string());
        out.push('\n');
        out.push_str(&format!(
            "{} --> {}\n",
            format_vtt_time(seg.start),
            format_vtt_time(seg.end)
        ));
        out.push_str(&seg.text);
        out.push_str("\n\n");
    }
    out
}
