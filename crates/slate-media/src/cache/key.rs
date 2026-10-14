use slate_core::time::RationalTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub handle_id: u64,
    pub time: RationalTime,
}

impl CacheKey {
    pub fn new(handle_id: u64, time: RationalTime) -> Self {
        Self { handle_id, time }
    }
}
