use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum TimeError {
    #[error("Invalid timecode format: '{0}' (expected HH:MM:SS:FF or HH:MM:SS;FF)")]
    InvalidTimecode(String),
    #[error("Division by zero in rational time calculation")]
    DivisionByZero,
    #[error("Framerate rate must be positive and non-zero: {0}")]
    InvalidTimebase(u32),
    #[error("Time conversion overflow: {0}")]
    Overflow(String),
}

/// Exact frame-accurate time using rational numbers.
/// Numerator (`ticks`) is the tick count; denominator (`rate`) is the timebase.
/// Standard timebases include 24, 25, 30, 48, 50, 60, 30000 (for 29.97), and 48000 (audio samples).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct RationalTime {
    /// Number of integer time slices
    pub ticks: i64,
    /// Timebase frequency (ticks per second)
    pub rate: u32,
}

impl RationalTime {
    pub const ZERO: Self = Self { ticks: 0, rate: 1 };

    /// Constructs a new RationalTime with the specified ticks and rate.
    pub const fn new(ticks: i64, rate: u32) -> Self {
        assert!(rate > 0, "Rate must be non-zero");
        Self { ticks, rate }
    }

    /// Constructs time from an exact frame index at a specified frame rate.
    pub fn from_frames(frames: i64, rate: u32) -> Self {
        Self::new(frames, rate)
    }

    /// Constructs time from fractional seconds, rounding to the nearest tick at the given rate.
    pub fn from_seconds(seconds: f64, rate: u32) -> Self {
        let ticks = (seconds * rate as f64).round() as i64;
        Self::new(ticks, rate)
    }

    /// Converts internal rational time to total whole frames at its current rate.
    pub fn to_frames(&self) -> i64 {
        self.ticks
    }

    /// Converts internal rational time to fractional floating-point seconds.
    pub fn to_seconds(&self) -> f64 {
        self.ticks as f64 / self.rate as f64
    }

    /// Rescales this time to a new timebase without loss of precision where possible,
    /// rounding to the nearest tick.
    pub fn rescale(&self, new_rate: u32) -> Self {
        if self.rate == new_rate {
            return *self;
        }
        // Using 128-bit integer to prevent overflow during intermediate multiplication
        let new_ticks = ((self.ticks as i128 * new_rate as i128 + (self.rate as i128 / 2))
            / self.rate as i128) as i64;
        Self::new(new_ticks, new_rate)
    }

    /// Reduces fraction by greatest common divisor.
    pub fn reduce(&self) -> Self {
        let g = gcd(self.ticks.unsigned_abs(), self.rate as u64);
        if g > 1 {
            Self {
                ticks: self.ticks / g as i64,
                rate: (self.rate as u64 / g) as u32,
            }
        } else {
            *self
        }
    }
}

impl Default for RationalTime {
    fn default() -> Self {
        Self::ZERO
    }
}

impl fmt::Display for RationalTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}s", self.ticks, self.rate)
    }
}

impl Ord for RationalTime {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.rate == other.rate {
            self.ticks.cmp(&other.ticks)
        } else {
            let lhs = self.ticks as i128 * other.rate as i128;
            let rhs = other.ticks as i128 * self.rate as i128;
            lhs.cmp(&rhs)
        }
    }
}

impl PartialOrd for RationalTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add for RationalTime {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        if self.rate == rhs.rate {
            Self::new(self.ticks + rhs.ticks, self.rate)
        } else {
            let common_rate = lcm(self.rate as u64, rhs.rate as u64) as u32;
            let left_ticks = self.ticks * (common_rate as i64 / self.rate as i64);
            let right_ticks = rhs.ticks * (common_rate as i64 / rhs.rate as i64);
            Self::new(left_ticks + right_ticks, common_rate)
        }
    }
}

impl AddAssign for RationalTime {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for RationalTime {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        if self.rate == rhs.rate {
            Self::new(self.ticks - rhs.ticks, self.rate)
        } else {
            let common_rate = lcm(self.rate as u64, rhs.rate as u64) as u32;
            let left_ticks = self.ticks * (common_rate as i64 / self.rate as i64);
            let right_ticks = rhs.ticks * (common_rate as i64 / rhs.rate as i64);
            Self::new(left_ticks - right_ticks, common_rate)
        }
    }
}

impl SubAssign for RationalTime {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Neg for RationalTime {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self::new(-self.ticks, self.rate)
    }
}

impl Mul<i64> for RationalTime {
    type Output = Self;
    fn mul(self, rhs: i64) -> Self::Output {
        Self::new(self.ticks * rhs, self.rate)
    }
}

impl Div<i64> for RationalTime {
    type Output = Self;
    fn div(self, rhs: i64) -> Self::Output {
        assert!(rhs != 0, "Division by zero");
        Self::new(self.ticks / rhs, self.rate)
    }
}

pub fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

pub fn lcm(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 {
        0
    } else {
        (a / gcd(a, b)) * b
    }
}
