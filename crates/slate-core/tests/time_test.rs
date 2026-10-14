use slate_core::time::{RationalTime, TimeRange, Timecode};

#[test]
fn test_rational_time_arithmetic() {
    let t1 = RationalTime::from_frames(30, 30);
    assert_eq!(t1.to_seconds(), 1.0);

    let t2 = RationalTime::from_seconds(1.5, 30);
    assert_eq!(t2.ticks, 45);

    let t3 = t1 + t2;
    assert_eq!(t3.ticks, 75);
    assert_eq!(t3.to_seconds(), 2.5);

    let t4 = t3 - t1;
    assert_eq!(t4, t2);

    let t5 = RationalTime::from_seconds(2.0, 30);
    let resampled = t5.rescale(60);
    assert_eq!(resampled.rate, 60);
    assert_eq!(resampled.ticks, 120);
    assert_eq!(resampled.to_seconds(), 2.0);
}

#[test]
fn test_time_range_operations() {
    let range1 = TimeRange::new(
        RationalTime::from_seconds(1.0, 30),
        RationalTime::from_seconds(3.0, 30),
    );
    assert_eq!(range1.end_time(), RationalTime::from_seconds(4.0, 30));

    // Overlaps
    let range2 = TimeRange::new(
        RationalTime::from_seconds(2.0, 30),
        RationalTime::from_seconds(2.0, 30),
    );
    assert!(range1.overlaps(&range2));

    let range3 = TimeRange::new(
        RationalTime::from_seconds(5.0, 30),
        RationalTime::from_seconds(2.0, 30),
    );
    assert!(!range1.overlaps(&range3));

    // Contains
    assert!(range1.contains_time(RationalTime::from_seconds(1.0, 30)));
    assert!(range1.contains_time(RationalTime::from_seconds(2.5, 30)));
    assert!(!range1.contains_time(RationalTime::from_seconds(4.0, 30))); // Half-open [start, end)
}

#[test]
fn test_timecode_formatting_and_parsing() {
    let t = RationalTime::from_frames(3600, 30); // 00:02:00:00 at 30fps
    let formatted = Timecode::format(t, false);
    assert_eq!(formatted, "00:02:00:00");

    let parsed = Timecode::parse("00:02:00:00", 30, false).expect("failed to parse timecode");
    assert_eq!(parsed, t);

    // Arbitrary timecode
    let parsed2 = Timecode::parse("01:23:45:12", 24, false).expect("valid parse");
    let formatted2 = Timecode::format(parsed2, false);
    assert_eq!(formatted2, "01:23:45:12");
}
