use super::rational::{RationalTime, TimeError};

/// SMPTE Timecode formatting and parsing support.
pub struct Timecode;

impl Timecode {
    /// Parses SMPTE timecode (e.g. `01:00:12:15` or drop-frame `01:00:12;15`).
    pub fn parse(tc: &str, rate: u32, drop_frame: bool) -> Result<RationalTime, TimeError> {
        let parts: Vec<&str> = tc.split([':', ';']).collect();
        if parts.len() != 4 {
            return Err(TimeError::InvalidTimecode(tc.to_string()));
        }

        let hours: i64 = parts[0]
            .parse()
            .map_err(|_| TimeError::InvalidTimecode(tc.to_string()))?;
        let minutes: i64 = parts[1]
            .parse()
            .map_err(|_| TimeError::InvalidTimecode(tc.to_string()))?;
        let seconds: i64 = parts[2]
            .parse()
            .map_err(|_| TimeError::InvalidTimecode(tc.to_string()))?;
        let frames: i64 = parts[3]
            .parse()
            .map_err(|_| TimeError::InvalidTimecode(tc.to_string()))?;

        if drop_frame {
            // SMPTE drop-frame algorithm (drops frame 0 and 1 of every minute except minute 0, 10, 20, 30, 40, 50)
            let total_minutes = hours * 60 + minutes;
            let dropped_frames = 2 * (total_minutes - (total_minutes / 10));
            let total_frames =
                (hours * 3600 + minutes * 60 + seconds) * 30 + frames - dropped_frames;
            Ok(RationalTime::new(total_frames, rate))
        } else {
            let total_frames =
                (hours * 3600 + minutes * 60 + seconds) * (rate as i64) + frames;
            Ok(RationalTime::new(total_frames, rate))
        }
    }

    /// Formats RationalTime as SMPTE timecode `HH:MM:SS:FF` or drop-frame `HH:MM:SS;FF`.
    pub fn format(time: RationalTime, drop_frame: bool) -> String {
        let fps = time.rate as i64;
        let total_frames = time.ticks.abs();
        let sign = if time.ticks < 0 { "-" } else { "" };

        if drop_frame && (time.rate == 30 || time.rate == 29970 || time.rate == 30000) {
            let d = 2;
            let m = 30 * 60;
            let ten_m = 10 * m;
            let f = total_frames;
            let ten_m_chunks = f / (ten_m - 9 * d);
            let rem = f % (ten_m - 9 * d);
            let frame_adjusted = if rem >= 2 * d {
                f + 9 * d * ten_m_chunks + d * ((rem - 2 * d) / (m - d) + 1)
            } else {
                f + 9 * d * ten_m_chunks
            };
            let frames = frame_adjusted % 30;
            let seconds = (frame_adjusted / 30) % 60;
            let minutes = (frame_adjusted / (30 * 60)) % 60;
            let hours = frame_adjusted / (30 * 3600);
            format!("{}{:02}:{:02}:{:02};{:02}", sign, hours, minutes, seconds, frames)
        } else {
            let frames = total_frames % fps;
            let seconds = (total_frames / fps) % 60;
            let minutes = (total_frames / (fps * 60)) % 60;
            let hours = total_frames / (fps * 3600);
            format!("{}{:02}:{:02}:{:02}:{:02}", sign, hours, minutes, seconds, frames)
        }
    }
}
