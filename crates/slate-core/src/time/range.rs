use super::rational::RationalTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Closed-open bounded temporal interval `[start, start + duration)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct TimeRange {
    pub start: RationalTime,
    pub duration: RationalTime,
}

impl TimeRange {
    /// Creates a TimeRange from start and duration. Duration is rescaled to match start's timebase.
    pub fn new(start: RationalTime, duration: RationalTime) -> Self {
        assert!(duration.ticks >= 0, "Duration must be non-negative");
        let duration = duration.rescale(start.rate);
        Self { start, duration }
    }

    /// Creates a TimeRange spanning from start to end.
    pub fn from_start_end(start: RationalTime, end: RationalTime) -> Self {
        let end_rescaled = end.rescale(start.rate);
        assert!(
            end_rescaled >= start,
            "End time must be greater than or equal to start"
        );
        Self {
            start,
            duration: end_rescaled - start,
        }
    }

    /// Returns the end time of the interval: `start + duration`.
    pub fn end_time(&self) -> RationalTime {
        self.start + self.duration
    }

    /// Returns true if this range has zero duration.
    pub fn is_empty(&self) -> bool {
        self.duration.ticks == 0
    }

    /// Returns true if the given time point falls within `[start, end_time)`.
    pub fn contains_time(&self, time: RationalTime) -> bool {
        let t = time.rescale(self.start.rate);
        t >= self.start && t < self.end_time()
    }

    /// Returns true if `other` is entirely contained within this interval.
    pub fn contains_range(&self, other: &TimeRange) -> bool {
        let other_start = other.start.rescale(self.start.rate);
        let other_end = other.end_time().rescale(self.start.rate);
        other_start >= self.start && other_end <= self.end_time()
    }

    /// Returns true if this interval shares at least one common time point with `other`.
    pub fn overlaps(&self, other: &TimeRange) -> bool {
        let other_start = other.start.rescale(self.start.rate);
        let other_end = other.end_time().rescale(self.start.rate);
        self.start < other_end && other_start < self.end_time()
    }

    /// Computes the intersection of two intervals, or None if disjoint.
    pub fn intersection(&self, other: &TimeRange) -> Option<TimeRange> {
        let other_start = other.start.rescale(self.start.rate);
        let other_end = other.end_time().rescale(self.start.rate);

        let max_start = self.start.max(other_start);
        let min_end = self.end_time().min(other_end);

        if min_end > max_start {
            Some(TimeRange::from_start_end(max_start, min_end))
        } else {
            None
        }
    }

    /// Computes the convex hull / bounding union spanning both intervals.
    pub fn union(&self, other: &TimeRange) -> TimeRange {
        let other_start = other.start.rescale(self.start.rate);
        let other_end = other.end_time().rescale(self.start.rate);

        let min_start = self.start.min(other_start);
        let max_end = self.end_time().max(other_end);

        TimeRange::from_start_end(min_start, max_end)
    }

    /// Clamps this range to the boundaries of another range.
    pub fn clamped_to(&self, boundary: &TimeRange) -> Option<TimeRange> {
        self.intersection(boundary)
    }

    /// Offsets the interval by a given temporal delta.
    pub fn shift(&self, delta: RationalTime) -> Self {
        Self {
            start: self.start + delta,
            duration: self.duration,
        }
    }
}
