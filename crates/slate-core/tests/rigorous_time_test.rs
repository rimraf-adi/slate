use slate_core::time::{lcm, RationalTime, TimeError, TimeRange, Timecode};

#[test]
fn test_rational_time_constructors_and_conversions() {
    // Zero
    let zero = RationalTime::ZERO;
    assert_eq!(zero.ticks, 0);
    assert_eq!(zero.rate, 1);
    assert_eq!(zero.to_seconds(), 0.0);
    assert_eq!(zero.to_frames(), 0);

    // new & from_frames
    let t1 = RationalTime::new(48, 24);
    assert_eq!(t1.ticks, 48);
    assert_eq!(t1.rate, 24);
    assert_eq!(t1.to_seconds(), 2.0);
    assert_eq!(t1.to_frames(), 48);

    let t2 = RationalTime::from_frames(100, 50);
    assert_eq!(t2.to_seconds(), 2.0);

    // from_seconds
    let t3 = RationalTime::from_seconds(1.25, 100);
    assert_eq!(t3.ticks, 125);
    assert_eq!(t3.rate, 100);
    assert_eq!(t3.to_seconds(), 1.25);

    // Negative values
    let neg = RationalTime::from_seconds(-0.5, 30);
    assert_eq!(neg.ticks, -15);
    assert_eq!(neg.to_seconds(), -0.5);
    assert_eq!(neg.to_frames(), -15);
}

#[test]
fn test_rational_time_rescale_and_reduce() {
    let t = RationalTime::new(30, 30); // 1.0 second at 30 fps
    let rescaled_60 = t.rescale(60);
    assert_eq!(rescaled_60.rate, 60);
    assert_eq!(rescaled_60.ticks, 60);
    assert_eq!(rescaled_60.to_seconds(), 1.0);

    let rescaled_same = t.rescale(30);
    assert_eq!(rescaled_same, t);

    // Rescale odd rate: 24 fps to 48000 Hz audio rate
    let film_frame = RationalTime::new(1, 24);
    let audio_samples = film_frame.rescale(48000);
    assert_eq!(audio_samples.rate, 48000);
    assert_eq!(audio_samples.ticks, 2000); // 48000 / 24 = 2000 samples

    // Reduce fractions
    let unreduced = RationalTime::new(100, 200);
    let reduced = unreduced.reduce();
    assert_eq!(reduced.ticks, 1);
    assert_eq!(reduced.rate, 2);

    let already_reduced = RationalTime::new(7, 13);
    assert_eq!(already_reduced.reduce(), already_reduced);
}

#[test]
fn test_rational_time_arithmetic_all_operators() {
    let a = RationalTime::new(10, 24);
    let b = RationalTime::new(14, 24);

    // Add same rate
    let sum = a + b;
    assert_eq!(sum.ticks, 24);
    assert_eq!(sum.rate, 24);
    assert_eq!(sum.to_seconds(), 1.0);

    // AddAssign
    let mut c = a;
    c += b;
    assert_eq!(c, sum);

    // Sub same rate
    let diff = sum - a;
    assert_eq!(diff, b);

    // SubAssign
    let mut d = sum;
    d -= a;
    assert_eq!(d, b);

    // Negation
    let neg_a = -a;
    assert_eq!(neg_a.ticks, -10);
    assert_eq!(neg_a.rate, 24);

    // Multiplication with scalar
    let mul = a * 3;
    assert_eq!(mul.ticks, 30);
    assert_eq!(mul.rate, 24);

    // Division with scalar
    let div = mul / 3;
    assert_eq!(div, a);

    // Cross-rate addition (24 fps + 30 fps -> lcm is 120)
    let t24 = RationalTime::new(12, 24); // 0.5s
    let t30 = RationalTime::new(15, 30); // 0.5s
    let cross_sum = t24 + t30;
    assert_eq!(cross_sum.to_seconds(), 1.0);
    assert_eq!(cross_sum.rate, 120);

    let cross_diff = cross_sum - t24;
    assert_eq!(cross_diff.to_seconds(), 0.5);
}

#[test]
fn test_rational_time_ordering_and_comparison() {
    let t1 = RationalTime::new(10, 20); // 0.5s
    let t2 = RationalTime::new(15, 30); // 0.5s
    let t3 = RationalTime::new(20, 30); // 0.667s
    let t4 = RationalTime::new(5, 30);  // 0.167s

    assert_eq!(t1.cmp(&t2), std::cmp::Ordering::Equal);
    assert_eq!(t1.rescale(30), t2);
    assert!(t1 <= t2);
    assert!(t1 >= t2);
    assert!(t4 < t1);
    assert!(t3 > t2);
    assert_eq!(t4.cmp(&t1), std::cmp::Ordering::Less);
    assert_eq!(t3.cmp(&t1), std::cmp::Ordering::Greater);
}

#[test]
fn test_rational_time_lcm_and_display() {
    assert_eq!(lcm(24, 30), 120);
    assert_eq!(lcm(0, 10), 0);
    assert_eq!(lcm(10, 0), 0);
    assert_eq!(lcm(60, 60), 60);

    let t = RationalTime::new(45, 30);
    assert_eq!(format!("{}", t), "45/30s");
}

#[test]
fn test_time_range_rigorous() {
    let start = RationalTime::new(10, 10); // 1.0s
    let dur = RationalTime::new(20, 10);   // 2.0s
    let range = TimeRange::new(start, dur);

    assert_eq!(range.start, start);
    assert_eq!(range.duration, dur);
    assert_eq!(range.end_time(), RationalTime::new(30, 10)); // 3.0s
    assert!(!range.is_empty());

    // Empty range
    let empty_range = TimeRange::new(start, RationalTime::new(0, 10));
    assert!(empty_range.is_empty());

    // from_start_end
    let range_se = TimeRange::from_start_end(start, RationalTime::new(30, 10));
    assert_eq!(range_se, range);

    // contains_time [start, end)
    assert!(range.contains_time(RationalTime::new(10, 10)));
    assert!(range.contains_time(RationalTime::new(20, 10)));
    assert!(range.contains_time(RationalTime::new(29, 10)));
    assert!(!range.contains_time(RationalTime::new(30, 10))); // exclusive end
    assert!(!range.contains_time(RationalTime::new(9, 10)));  // before start

    // contains_range
    let sub_range = TimeRange::new(RationalTime::new(15, 10), RationalTime::new(10, 10));
    assert!(range.contains_range(&sub_range));
    assert!(!sub_range.contains_range(&range));

    // overlaps
    let overlapping = TimeRange::new(RationalTime::new(25, 10), RationalTime::new(15, 10));
    assert!(range.overlaps(&overlapping));
    assert!(overlapping.overlaps(&range));

    let adjacent = TimeRange::new(RationalTime::new(30, 10), RationalTime::new(10, 10));
    assert!(!range.overlaps(&adjacent)); // [10, 30) and [30, 40) do not overlap

    let disjoint = TimeRange::new(RationalTime::new(50, 10), RationalTime::new(10, 10));
    assert!(!range.overlaps(&disjoint));

    // intersection
    let inter = range.intersection(&overlapping).expect("should intersect");
    assert_eq!(inter.start, RationalTime::new(25, 10));
    assert_eq!(inter.end_time(), RationalTime::new(30, 10));
    assert_eq!(range.intersection(&disjoint), None);

    // union
    let bounding = range.union(&overlapping);
    assert_eq!(bounding.start, RationalTime::new(10, 10));
    assert_eq!(bounding.end_time(), RationalTime::new(40, 10));

    // clamped_to
    assert_eq!(range.clamped_to(&overlapping), Some(inter));

    // shift
    let shifted = range.shift(RationalTime::new(5, 10));
    assert_eq!(shifted.start, RationalTime::new(15, 10));
    assert_eq!(shifted.duration, dur);
    assert_eq!(shifted.end_time(), RationalTime::new(35, 10));
}

#[test]
fn test_timecode_rigorous_framerate_coverage() {
    // 24 fps
    let t_24 = RationalTime::from_frames(24 * 3600 + 24 * 60 + 24 + 12, 24);
    let tc_24 = Timecode::format(t_24, false);
    assert_eq!(tc_24, "01:01:01:12");
    let parsed_24 = Timecode::parse(&tc_24, 24, false).unwrap();
    assert_eq!(parsed_24, t_24);

    // 25 fps PAL
    let t_25 = RationalTime::from_frames(25 * 300, 25); // 5 minutes
    let tc_25 = Timecode::format(t_25, false);
    assert_eq!(tc_25, "00:05:00:00");
    let parsed_25 = Timecode::parse(&tc_25, 25, false).unwrap();
    assert_eq!(parsed_25, t_25);

    // 60 fps
    let t_60 = RationalTime::from_frames(59, 60);
    let tc_60 = Timecode::format(t_60, false);
    assert_eq!(tc_60, "00:00:00:59");
    let parsed_60 = Timecode::parse(&tc_60, 60, false).unwrap();
    assert_eq!(parsed_60, t_60);

    // Drop-frame semicolon syntax parsing
    let parsed_df = Timecode::parse("01;00;00;00", 30, true).unwrap();
    assert_eq!(parsed_df.to_seconds(), 3596.4);

    // Error cases
    assert!(matches!(
        Timecode::parse("invalid:timecode", 30, false),
        Err(TimeError::InvalidTimecode(_))
    ));
    assert!(matches!(
        Timecode::parse("00:00:00", 30, false),
        Err(TimeError::InvalidTimecode(_))
    ));
    assert!(matches!(
        Timecode::parse("aa:bb:cc:dd", 30, false),
        Err(TimeError::InvalidTimecode(_))
    ));
}
