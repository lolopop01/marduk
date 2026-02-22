/// High-level response after a surface error.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SurfaceErrorAction {
    /// Surface was reconfigured; rendering may resume next frame.
    Reconfigured,
    /// Transient error; skip the current frame.
    SkipFrame,
    /// Fatal error (commonly OOM); terminate gracefully.
    Fatal,
}