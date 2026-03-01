//! Keyboard focus management.
//!
//! [`FocusManager`] lives on [`crate::scene::UiScene`] and is threaded into
//! [`crate::painter::Painter`] and [`crate::constraints::LayoutCtx`] each frame.
//!
//! # How focus works
//!
//! 1. During **paint**, focusable widgets call `painter.register_focusable(id)` to
//!    enroll in Tab-key cycling, and `painter.is_focused(id)` to query current state.
//! 2. During **on_event**, focusable widgets call `ctx.request_focus(id)` when they
//!    receive a `Click`.
//! 3. At **end of frame**, [`FocusManager::end_frame`] applies any pending focus
//!    request and clears the registered list for the next frame.
//! 4. [`FocusManager::advance`] is called by `UiScene::frame` when Tab is pressed,
//!    cycling through the `registered` list in paint order.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FOCUS_ID: AtomicU64 = AtomicU64::new(1);

// ── FocusId ───────────────────────────────────────────────────────────────

/// Unique identifier for a focusable widget.
///
/// Allocated once per widget construction via [`FocusId::new()`].
/// Persistent across frames as long as the widget is alive.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct FocusId(u64);

impl FocusId {
    /// Allocate a new, globally unique `FocusId`.
    pub fn new() -> Self {
        FocusId(NEXT_FOCUS_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for FocusId {
    fn default() -> Self {
        Self::new()
    }
}

// ── FocusManager ──────────────────────────────────────────────────────────

/// Tracks keyboard focus across the widget tree.
///
/// Owned by [`crate::scene::UiScene`]; references are threaded into
/// [`crate::painter::Painter`] (paint pass) and
/// [`crate::constraints::LayoutCtx`] (event pass).
pub struct FocusManager {
    /// The currently focused widget, if any.
    pub focused: Option<FocusId>,
    /// Focus IDs registered this frame in paint order (for Tab cycling).
    registered: Vec<FocusId>,
    /// Focus requested this frame (applied in [`end_frame`]).
    requested: Option<FocusId>,
    /// The focused ID from the previous frame (for FocusLost detection).
    pub(crate) prev_focused: Option<FocusId>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            focused:      None,
            registered:   Vec::new(),
            requested:    None,
            prev_focused: None,
        }
    }

    /// Returns `true` if `id` is the currently focused widget.
    #[inline]
    pub fn is_focused(&self, id: FocusId) -> bool {
        self.focused == Some(id)
    }

    /// Request that `id` becomes focused at end of frame.
    #[inline]
    pub fn request_focus(&mut self, id: FocusId) {
        self.requested = Some(id);
    }

    /// Register `id` as focusable this frame (adds to Tab-cycle order).
    ///
    /// Must be called during the paint pass, in paint order.
    #[inline]
    pub fn register(&mut self, id: FocusId) {
        self.registered.push(id);
    }

    /// Advance focus to the next (or previous, if `reverse`) registered widget.
    ///
    /// Called by `UiScene::frame` when the Tab key is pressed.
    pub fn advance(&mut self, reverse: bool) {
        if self.registered.is_empty() {
            return;
        }
        let n = self.registered.len();
        self.focused = Some(match self.focused {
            None => self.registered[if reverse { n - 1 } else { 0 }],
            Some(current) => {
                match self.registered.iter().position(|&x| x == current) {
                    None => self.registered[0],
                    Some(i) => {
                        if reverse {
                            self.registered[(i + n - 1) % n]
                        } else {
                            self.registered[(i + 1) % n]
                        }
                    }
                }
            }
        });
    }

    /// Clear focus (e.g., on Escape key).
    #[inline]
    pub fn clear(&mut self) {
        self.focused  = None;
        self.requested = None;
    }

    /// Apply pending focus request and reset the registered list.
    ///
    /// Called at the end of each frame by `UiScene::frame`.
    pub fn end_frame(&mut self) {
        self.prev_focused = self.focused;
        if let Some(req) = self.requested.take() {
            self.focused = Some(req);
        }
        self.registered.clear();
    }

    /// Returns the ID that just gained focus this frame (if any).
    pub fn just_gained(&self) -> Option<FocusId> {
        if self.focused != self.prev_focused {
            self.focused
        } else {
            None
        }
    }

    /// Returns the ID that just lost focus this frame (if any).
    pub fn just_lost(&self) -> Option<FocusId> {
        if self.focused != self.prev_focused {
            self.prev_focused
        } else {
            None
        }
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}
