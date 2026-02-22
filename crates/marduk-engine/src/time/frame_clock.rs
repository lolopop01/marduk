use std::time::{Duration, Instant};

/// Frame timing snapshot.
#[derive(Debug, Copy, Clone)]
pub struct FrameTime {
    /// Time elapsed since the previous frame tick, in seconds.
    pub dt: f32,

    /// Monotonic timestamp taken at the tick.
    pub now: Instant,

    /// Monotonic frame counter.
    pub frame_index: u64,
}

/// Frame clock producing `FrameTime` snapshots.
///
/// `FrameClock` is designed to be used per window (or per loop) so that multi-window
/// applications do not share delta-time state.
///
/// Delta time is clamped to avoid pathological values when the application is paused
/// by the debugger, minimized, or stalls.
#[derive(Debug, Clone)]
pub struct FrameClock {
    last: Instant,
    frame_index: u64,
    dt_min: Duration,
    dt_max: Duration,
}

impl FrameClock {
    /// Creates a new clock with default clamps.
    ///
    /// Clamp rationale:
    /// - minimum prevents zero-dt behavior from tight loops on some platforms
    /// - maximum prevents simulation explosions after long stalls
    pub fn new() -> Self {
        Self {
            last: Instant::now(),
            frame_index: 0,
            dt_min: Duration::from_micros(100),  // 0.0001s
            dt_max: Duration::from_millis(250),  // 0.25s
        }
    }

    /// Creates a clock with custom delta-time clamps.
    pub fn with_clamps(dt_min: Duration, dt_max: Duration) -> Self {
        debug_assert!(dt_min <= dt_max);
        Self {
            last: Instant::now(),
            frame_index: 0,
            dt_min,
            dt_max,
        }
    }

    /// Resets the clock baseline.
    ///
    /// Useful after surface reconfigure events or when resuming from suspension.
    pub fn reset(&mut self) {
        self.last = Instant::now();
    }

    /// Advances the clock and returns a new `FrameTime`.
    pub fn tick(&mut self) -> FrameTime {
        let now = Instant::now();
        let mut dt = now.saturating_duration_since(self.last);

        // Clamp delta time to keep downstream systems stable.
        if dt < self.dt_min {
            dt = self.dt_min;
        } else if dt > self.dt_max {
            dt = self.dt_max;
        }

        self.last = now;

        let ft = FrameTime {
            dt: dt.as_secs_f32(),
            now,
            frame_index: self.frame_index,
        };

        self.frame_index = self
            .frame_index
            .wrapping_add(1);

        ft
    }
}

impl Default for FrameClock {
    fn default() -> Self {
        Self::new()
    }
}