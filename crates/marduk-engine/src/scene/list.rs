use super::{DrawCmd, SortKey, ZIndex};

/// A single draw item: sort key + command.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawItem {
    pub key: SortKey,
    pub cmd: DrawCmd,
}

/// Recorded draw stream for a frame.
///
/// Performance characteristics:
/// - `push()` is O(1)
/// - paint-order iteration reuses an internal index buffer; no per-frame allocation once warmed
#[derive(Debug, Default)]
pub struct DrawList {
    items: Vec<DrawItem>,
    next_order: u32,

    sorted_indices: Vec<usize>,
    sorted_dirty: bool,
}

impl DrawList {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears recorded items. Keeps allocated capacity for reuse.
    #[inline]
    pub fn clear(&mut self) {
        self.items.clear();
        self.next_order = 0;
        self.sorted_dirty = true;
        self.sorted_indices.clear();
    }

    /// Returns items in insertion order.
    #[inline]
    pub fn items(&self) -> &[DrawItem] {
        &self.items
    }

    /// Pushes a draw command with the given z-index.
    #[inline]
    pub fn push(&mut self, z: ZIndex, cmd: DrawCmd) {
        let order = self.next_order;
        self.next_order = self.next_order.wrapping_add(1);

        self.items.push(DrawItem {
            key: SortKey::new(z, order),
            cmd,
        });

        self.sorted_dirty = true;
    }

    /// Returns indices into `items` in paint order (back-to-front).
    ///
    /// This buffer is owned by `DrawList` and reused across frames.
    pub fn indices_in_paint_order(&mut self) -> &[usize] {
        if self.sorted_dirty {
            self.rebuild_sorted_indices();
        }
        &self.sorted_indices
    }

    /// Iterates items in paint order without cloning draw commands.
    pub fn iter_in_paint_order(&mut self) -> impl Iterator<Item = &DrawItem> {
        if self.sorted_dirty {
            self.rebuild_sorted_indices();
        }

        let items_ptr: *const Vec<DrawItem> = &self.items;
        self.sorted_indices.iter().map(move |&i| unsafe { &(*items_ptr)[i] })
    }

    fn rebuild_sorted_indices(&mut self) {
        self.sorted_indices.clear();
        self.sorted_indices.extend(0..self.items.len());

        // Stable ordering is ensured by SortKey including insertion order.
        self.sorted_indices
            .sort_by(|&a, &b| self.items[a].key.cmp(&self.items[b].key));

        self.sorted_dirty = false;
    }
}