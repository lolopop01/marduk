use crate::coords::Rect;

use super::{DrawCmd, SortKey, ZIndex};

/// A single draw item: sort key + command + clip rect.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawItem {
    pub key: SortKey,
    pub cmd: DrawCmd,
    /// Scissor rect in logical pixels. `None` = no clipping (draw everywhere).
    pub clip_rect: Option<Rect>,
}

/// Recorded draw stream for a frame.
///
/// Performance characteristics:
/// - `push()` is O(1)
/// - paint-order iteration reuses an internal index buffer; no per-frame allocation once warmed
///
/// # Clipping
///
/// Use [`push_clip`] / [`pop_clip`] to scope draw commands to a scissor rect.
/// Clips are intersected with the current parent, so nested scroll containers work correctly.
///
/// ```
/// draw_list.push_clip(scroll_container_rect);
/// // ... push children ...
/// draw_list.pop_clip();
/// ```
#[derive(Debug, Default)]
pub struct DrawList {
    items: Vec<DrawItem>,
    next_order: u32,

    sorted_indices: Vec<usize>,
    sorted_dirty: bool,

    /// Stack of active scissor rects (logical pixels).
    /// The top is always the current effective clip, already intersected with all parents.
    clip_stack: Vec<Rect>,
}

impl DrawList {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears recorded items and the clip stack. Keeps allocated capacity for reuse.
    #[inline]
    pub fn clear(&mut self) {
        self.items.clear();
        self.next_order = 0;
        self.sorted_dirty = true;
        self.sorted_indices.clear();
        self.clip_stack.clear();
    }

    /// Returns items in insertion order.
    #[inline]
    pub fn items(&self) -> &[DrawItem] {
        &self.items
    }

    /// Pushes a draw command with the given z-index.
    ///
    /// The item inherits the current clip rect from the clip stack.
    #[inline]
    pub fn push(&mut self, z: ZIndex, cmd: DrawCmd) {
        let order = self.next_order;
        self.next_order = self.next_order.wrapping_add(1);

        self.items.push(DrawItem {
            key: SortKey::new(z, order),
            cmd,
            clip_rect: self.clip_stack.last().copied(),
        });

        self.sorted_dirty = true;
    }

    /// Begins a scissor region. All draw commands pushed until [`pop_clip`] are clipped
    /// to `rect` (intersected with any parent clip rect).
    ///
    /// Calls must be balanced with [`pop_clip`].
    #[inline]
    pub fn push_clip(&mut self, rect: Rect) {
        let effective = match self.clip_stack.last() {
            None => rect,
            // Intersect with the parent; if no overlap, produce a zero-area rect so
            // the renderer skips those draw calls.
            Some(&parent) => parent.intersect(rect).unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0)),
        };
        self.clip_stack.push(effective);
    }

    /// Ends the most recent scissor region started by [`push_clip`].
    ///
    /// # Panics
    /// Panics (debug only) if called without a matching `push_clip`.
    #[inline]
    pub fn pop_clip(&mut self) {
        debug_assert!(!self.clip_stack.is_empty(), "pop_clip called without matching push_clip");
        self.clip_stack.pop();
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

        self.sorted_indices.iter().map(|&i| &self.items[i])
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
