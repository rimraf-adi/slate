use slate_core::time::RationalTime;
use slate_core::traits::decoder::VideoFrame;
use slate_media::cache::{CacheKey, FrameCache};

#[test]
fn test_frame_cache_lru_eviction() {
    // Budget: 1000 bytes. Each test frame: 400 bytes (10x10 RGBA).
    let cache = FrameCache::new(1000);

    let key1 = CacheKey::new(1, RationalTime::from_frames(1, 30));
    let frame1 = VideoFrame {
        pts: RationalTime::from_frames(1, 30),
        width: 10,
        height: 10,
        data: vec![0u8; 400],
        stride: 40,
    };

    let key2 = CacheKey::new(1, RationalTime::from_frames(2, 30));
    let frame2 = VideoFrame {
        pts: RationalTime::from_frames(2, 30),
        width: 10,
        height: 10,
        data: vec![1u8; 400],
        stride: 40,
    };

    cache.insert(key1, frame1);
    cache.insert(key2, frame2);
    assert_eq!(cache.current_bytes(), 800);
    assert!(cache.get(&key1).is_some());
    assert!(cache.get(&key2).is_some());

    // Access key1 so key2 becomes the least recently used
    let _ = cache.get(&key1);

    // Insert key3 (400 bytes) -> total would be 1200 > 1000, so key2 should be evicted!
    let key3 = CacheKey::new(1, RationalTime::from_frames(3, 30));
    let frame3 = VideoFrame {
        pts: RationalTime::from_frames(3, 30),
        width: 10,
        height: 10,
        data: vec![2u8; 400],
        stride: 40,
    };
    cache.insert(key3, frame3);

    assert_eq!(cache.current_bytes(), 800);
    assert!(cache.get(&key1).is_some(), "key1 should remain cached");
    assert!(cache.get(&key2).is_none(), "key2 should have been evicted");
    assert!(cache.get(&key3).is_some(), "key3 should be cached");
}
