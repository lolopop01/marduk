use core::cmp::Ordering;

use super::ZIndex;

/// Stable sort key for draw items.
///
/// Ordering rules:
/// 1) `z`: ascending (back-to-front)
/// 2) `order`: ascending (insertion order for equal z)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SortKey {
    pub z: ZIndex,
    pub order: u32,
}

impl SortKey {
    #[inline]
    pub const fn new(z: ZIndex, order: u32) -> Self {
        Self { z, order }
    }
}

impl Ord for SortKey {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        match self.z.cmp(&other.z) {
            Ordering::Equal => self.order.cmp(&other.order),
            o => o,
        }
    }
}

impl PartialOrd for SortKey {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}