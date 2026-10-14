use super::key::CacheKey;
use parking_lot::Mutex;
use slate_core::traits::decoder::VideoFrame;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

pub struct FrameCache {
    max_bytes: usize,
    current_bytes: Mutex<usize>,
    entries: Mutex<HashMap<CacheKey, (Arc<VideoFrame>, usize)>>,
    order: Mutex<VecDeque<CacheKey>>,
}

impl FrameCache {
    /// Constructs a new FrameCache with a byte limit (e.g. 512 MB or 1 GB).
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            current_bytes: Mutex::new(0),
            entries: Mutex::new(HashMap::new()),
            order: Mutex::new(VecDeque::new()),
        }
    }

    pub fn get(&self, key: &CacheKey) -> Option<Arc<VideoFrame>> {
        let entries = self.entries.lock();
        if let Some((frame, _)) = entries.get(key) {
            let mut order = self.order.lock();
            if let Some(pos) = order.iter().position(|k| k == key) {
                order.remove(pos);
            }
            order.push_back(*key);
            Some(frame.clone())
        } else {
            None
        }
    }

    pub fn insert(&self, key: CacheKey, frame: VideoFrame) -> Arc<VideoFrame> {
        let frame_bytes = frame.data.len();
        let arc_frame = Arc::new(frame);

        let mut entries = self.entries.lock();
        let mut order = self.order.lock();
        let mut current_bytes = self.current_bytes.lock();

        // Evict until under budget
        while *current_bytes + frame_bytes > self.max_bytes && !order.is_empty() {
            if let Some(oldest_key) = order.pop_front() {
                if let Some((_, size)) = entries.remove(&oldest_key) {
                    *current_bytes = current_bytes.saturating_sub(size);
                }
            }
        }

        if let Some((_, old_size)) = entries.insert(key, (arc_frame.clone(), frame_bytes)) {
            *current_bytes = current_bytes.saturating_sub(old_size);
        }
        *current_bytes += frame_bytes;

        if let Some(pos) = order.iter().position(|k| k == &key) {
            order.remove(pos);
        }
        order.push_back(key);

        arc_frame
    }

    pub fn clear(&self) {
        let mut entries = self.entries.lock();
        let mut order = self.order.lock();
        let mut current_bytes = self.current_bytes.lock();

        entries.clear();
        order.clear();
        *current_bytes = 0;
    }

    pub fn current_bytes(&self) -> usize {
        *self.current_bytes.lock()
    }
}
