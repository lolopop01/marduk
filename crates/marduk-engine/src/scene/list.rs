use crate::coords::{Rect, Vec2};
use crate::paint::Paint;
use crate::scene::shapes::Border;

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

    /// Stack of composed absolute transforms applied to draw-command coordinates.
    ///
    /// Each entry `(offset, scale)` represents the mapping: `screen = content * scale + offset`.
    /// Entries are composed on push so the active entry is always an absolute screen-space
    /// transform, not a relative one.  When empty the identity transform is implied.
    ///
    /// `push_transform` / `pop_transform` are the public API.  Shape push helpers call
    /// [`tx_pos`] / [`tx_rect`] / [`tx_f32`] / [`tx_paint`] / [`tx_border`] when recording
    /// draw commands so ZoomView-zoomed content is correctly mapped to screen pixels.
    transform_stack: Vec<(Vec2, f32)>,
}

impl DrawList {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears recorded items, the clip stack, and the transform stack.
    /// Keeps allocated capacity for reuse.
    #[inline]
    pub fn clear(&mut self) {
        self.items.clear();
        self.next_order = 0;
        self.sorted_dirty = true;
        self.sorted_indices.clear();
        self.clip_stack.clear();
        self.transform_stack.clear();
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
    /// `rect` is in the current draw coordinate space and is automatically transformed
    /// to screen space when a [`push_transform`] is active.
    ///
    /// Calls must be balanced with [`pop_clip`].
    #[inline]
    pub fn push_clip(&mut self, rect: Rect) {
        let screen_rect = self.tx_rect(rect);
        let effective = match self.clip_stack.last() {
            None => screen_rect,
            // Intersect with the parent; if no overlap, produce a zero-area rect so
            // the renderer skips those draw calls.
            Some(&parent) => parent.intersect(screen_rect).unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0)),
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

    // ── coordinate transform ───────────────────────────────────────────────

    /// Push a coordinate transform for subsequent draw commands.
    ///
    /// `scale` and `offset` are in the **current** draw coordinate space (before this push).
    /// The mapping applied to every draw-command coordinate is:
    /// `screen = content * scale + offset`.
    ///
    /// Transforms compose: pushing inside an already-transformed context multiplies
    /// scale and adds the offset adjusted by the outer scale, so nested `ZoomView`
    /// containers work correctly.
    ///
    /// Must be paired with a matching [`pop_transform`].
    #[inline]
    pub fn push_transform(&mut self, scale: f32, offset: Vec2) {
        let composed = if let Some(&(cur_off, cur_scale)) = self.transform_stack.last() {
            // Compose: new_abs_offset = offset * cur_scale + cur_offset
            let abs_offset = Vec2::new(
                offset.x * cur_scale + cur_off.x,
                offset.y * cur_scale + cur_off.y,
            );
            (abs_offset, scale * cur_scale)
        } else {
            (offset, scale)
        };
        self.transform_stack.push(composed);
    }

    /// Pop the most recently pushed coordinate transform.
    #[inline]
    pub fn pop_transform(&mut self) {
        debug_assert!(!self.transform_stack.is_empty(), "pop_transform called without matching push_transform");
        self.transform_stack.pop();
    }

    /// Remove and return the entire transform stack.
    ///
    /// Use together with [`restore_transforms`] to temporarily escape all
    /// active coordinate transforms (e.g. for overlay / popup draws that
    /// must render in screen space).
    #[inline]
    pub fn take_transforms(&mut self) -> Vec<(Vec2, f32)> {
        std::mem::take(&mut self.transform_stack)
    }

    /// Restore a transform stack previously saved with [`take_transforms`].
    #[inline]
    pub fn restore_transforms(&mut self, transforms: Vec<(Vec2, f32)>) {
        self.transform_stack = transforms;
    }

    // ── transform helpers (used by shape push methods) ─────────────────────

    /// Current absolute screen-space transform `(offset, scale)`.
    /// Returns the identity `(Vec2::ZERO, 1.0)` when the stack is empty.
    #[inline]
    pub(crate) fn current_transform(&self) -> (Vec2, f32) {
        self.transform_stack.last().copied().unwrap_or((Vec2::new(0.0, 0.0), 1.0))
    }

    /// Map a content-space position to screen space.
    #[inline]
    pub(crate) fn tx_pos(&self, p: Vec2) -> Vec2 {
        let (off, scale) = self.current_transform();
        Vec2::new(p.x * scale + off.x, p.y * scale + off.y)
    }

    /// Map a content-space rect to screen space (origin and size both scaled).
    #[inline]
    pub(crate) fn tx_rect(&self, r: Rect) -> Rect {
        let (off, scale) = self.current_transform();
        Rect::new(
            r.origin.x * scale + off.x,
            r.origin.y * scale + off.y,
            r.size.x * scale,
            r.size.y * scale,
        )
    }

    /// Scale a content-space scalar (size, radius, width…) to screen space.
    #[inline]
    pub(crate) fn tx_f32(&self, v: f32) -> f32 {
        let (_, scale) = self.current_transform();
        v * scale
    }

    /// Transform gradient start/end positions inside a `Paint`.
    /// `Solid` paints pass through unchanged.
    #[inline]
    pub(crate) fn tx_paint(&self, paint: Paint) -> Paint {
        match paint {
            Paint::Solid(_) => paint,
            Paint::LinearGradient(mut g) => {
                g.start = self.tx_pos(g.start);
                g.end   = self.tx_pos(g.end);
                Paint::LinearGradient(g)
            }
        }
    }

    /// Scale a border's width. Returns `None` when `border` is `None`.
    #[inline]
    pub(crate) fn tx_border(&self, border: Option<Border>) -> Option<Border> {
        border.map(|b| Border::new(self.tx_f32(b.width), b.color))
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
