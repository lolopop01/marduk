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
/// ```rust,ignore
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

    /// Optional z-range filter applied by [`iter_in_paint_order`].
    ///
    /// When `Some((min, max))`, only items with `z ∈ [min, max]` are yielded.
    /// Set via [`set_z_range`] / [`reset_z_range`] from the render loop to implement
    /// two-pass rendering (normal content first, overlay content second).
    z_filter: Option<(i32, i32)>,
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
    /// In debug builds, panics if called without a matching [`push_clip`].
    /// In release builds, the call is silently ignored to avoid crashing production apps.
    #[inline]
    pub fn pop_clip(&mut self) {
        debug_assert!(!self.clip_stack.is_empty(), "pop_clip called without matching push_clip");
        self.clip_stack.pop();
    }

    /// Remove and return the entire clip stack.
    ///
    /// Use this together with [`restore_clips`] to temporarily escape all parent
    /// scissor regions (e.g. for overlay / popup draws that must not be clipped).
    #[inline]
    pub fn take_clips(&mut self) -> Vec<Rect> {
        std::mem::take(&mut self.clip_stack)
    }

    /// Restore a clip stack previously saved with [`take_clips`].
    #[inline]
    pub fn restore_clips(&mut self, clips: Vec<Rect>) {
        self.clip_stack = clips;
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

    /// Restrict [`iter_in_paint_order`] to items with z ∈ `[min_z, max_z]` (inclusive).
    ///
    /// Call this before invoking renderers to implement two-pass overlay rendering.
    /// Reset with [`reset_z_range`] after the pass is complete.
    #[inline]
    pub fn set_z_range(&mut self, min_z: i32, max_z: i32) {
        self.z_filter = Some((min_z, max_z));
    }

    /// Remove any z-range restriction set by [`set_z_range`].
    #[inline]
    pub fn reset_z_range(&mut self) {
        self.z_filter = None;
    }

    /// Iterates items in paint order without cloning draw commands.
    ///
    /// If a z-range filter is active (set via [`set_z_range`]), only items in
    /// that range are yielded.
    pub fn iter_in_paint_order(&mut self) -> impl Iterator<Item = &DrawItem> {
        if self.sorted_dirty {
            self.rebuild_sorted_indices();
        }

        let z_filter = self.z_filter;
        // Split the borrow explicitly so the closure doesn't capture `self`.
        let items = &self.items;
        self.sorted_indices.iter().filter_map(move |&i| {
            let item = &items[i];
            if let Some((min_z, max_z)) = z_filter {
                if item.key.z.0 < min_z || item.key.z.0 > max_z {
                    return None;
                }
            }
            Some(item)
        })
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
