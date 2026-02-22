use core::cmp::Ordering;

/// Z-ordering key for draw items.
///
/// Higher values appear on top of lower values.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Default)]
pub struct ZIndex(pub i32);

impl ZIndex {
    #[inline]
    pub const fn new(v: i32) -> Self {
        Self(v)
    }
}

impl Ord for ZIndex {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for ZIndex {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}