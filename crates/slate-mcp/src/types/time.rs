use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use slate_core::time::{RationalTime, Timecode};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum TimeArgError {
    #[error("Empty time string provided")]
    Empty,
    #[error("Invalid unit or suffix in '{0}': expected 'f' (frames), 's' (seconds), or SMPTE timecode 'HH:MM:SS:FF'")]
    InvalidSuffix(String),
    #[error("Could not parse numeric component: {0}")]
    ParseNumeric(#[from] std::num::ParseFloatError),
    #[error("Could not parse integer frames: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("Invalid SMPTE timecode format '{0}': must be HH:MM:SS:FF or HH:MM:SS;FF")]
    InvalidTimecode(String),
}

/// Flexible input time argument accepting frames, seconds, timecode, or raw integer ticks.
#[derive(Debug, Clone, Copy, PartialEq, JsonSchema)]
#[schemars(description = "Flexible time representation: '120f' (frames), '4.5s' (seconds), '00:01:23:15' (timecode), or raw frame integer 120")]
pub struct TimeArg {
    #[schemars(skip)]
    pub inner: RationalTime,
}

impl TimeArg {
    pub fn new(rational: RationalTime) -> Self {
        Self { inner: rational }
    }

    pub fn to_rational(&self) -> RationalTime {
        self.inner
    }

    /// Parses an input string in any supported format given the project frame rate
    pub fn parse(input: &str, project_rate: u32) -> Result<Self, TimeArgError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(TimeArgError::Empty);
        }

        // Case 1: Frames explicit suffix "123f" or "123F"
        if let Some(num_str) = trimmed.strip_suffix('f').or_else(|| trimmed.strip_suffix('F')) {
            let frames: i64 = num_str.parse()?;
            return Ok(Self::new(RationalTime::from_frames(frames, project_rate)));
        }

        // Case 2: Seconds explicit suffix "83.5s" or "83.5S"
        if let Some(num_str) = trimmed.strip_suffix('s').or_else(|| trimmed.strip_suffix('S')) {
            let seconds: f64 = num_str.parse()?;
            return Ok(Self::new(RationalTime::from_seconds(seconds, project_rate)));
        }

        // Case 3: SMPTE timecode "00:01:23:15" or drop-frame "00:01:23;15"
        if trimmed.contains(':') || trimmed.contains(';') {
            let drop_frame = trimmed.contains(';');
            let rt = Timecode::parse(trimmed, project_rate, drop_frame)
                .map_err(|_| TimeArgError::InvalidTimecode(trimmed.to_string()))?;
            return Ok(Self::new(rt));
        }

        // Case 4: Plain numeric string without suffix (treated as integer frames)
        if let Ok(frames) = trimmed.parse::<i64>() {
            return Ok(Self::new(RationalTime::from_frames(frames, project_rate)));
        }

        // Case 5: Plain floating point string without suffix (treated as seconds)
        if let Ok(seconds) = trimmed.parse::<f64>() {
            return Ok(Self::new(RationalTime::from_seconds(seconds, project_rate)));
        }

        Err(TimeArgError::InvalidSuffix(trimmed.to_string()))
    }
}

impl<'de> Deserialize<'de> for TimeArg {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum RawTime {
            String(String),
            Integer(i64),
            Float(f64),
        }

        let raw = RawTime::deserialize(deserializer)?;
        match raw {
            RawTime::String(s) => TimeArg::parse(&s, 30).map_err(serde::de::Error::custom),
            RawTime::Integer(f) => Ok(TimeArg::new(RationalTime::from_frames(f, 30))),
            RawTime::Float(s) => Ok(TimeArg::new(RationalTime::from_seconds(s, 30))),
        }
    }
}

impl Serialize for TimeArg {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&Timecode::format(self.inner, false))
    }
}

/// Universal Tri-Format Time Response returning frames, SMPTE timecode, and seconds simultaneously.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TimeResponse {
    #[schemars(description = "Total whole frame index at project frame rate")]
    pub frames: i64,
    #[schemars(description = "SMPTE timecode string (HH:MM:SS:FF)")]
    pub timecode: String,
    #[schemars(description = "Floating-point duration/timestamp in seconds")]
    pub seconds: f64,
}

impl TimeResponse {
    pub fn from_rational(rt: RationalTime, drop_frame: bool) -> Self {
        Self {
            frames: rt.to_frames(),
            timecode: Timecode::format(rt, drop_frame),
            seconds: (rt.to_seconds() * 1000.0).round() / 1000.0,
        }
    }
}
