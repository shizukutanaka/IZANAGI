//! Runtime diagnostics and frame metrics.
//!
//! Tracks frame times, min/max/avg FPS, and a rolling history.
//! Zero-cost when you don't call it.

/// Frame-rate and timing diagnostics.
pub struct Metrics {
    frames: u64,
    elapsed: f32,
    min_dt: f32,
    max_dt: f32,
    sum_dt: f32,
    history: Vec<f32>, // last N frame times
    capacity: usize,
}

impl Metrics {
    /// New metrics tracker with a rolling history of `history_len` frames.
    pub fn new(history_len: usize) -> Self {
        Self {
            frames: 0,
            elapsed: 0.0,
            min_dt: f32::MAX,
            max_dt: 0.0,
            sum_dt: 0.0,
            history: Vec::with_capacity(history_len),
            capacity: history_len.max(1),
        }
    }

    /// Record a frame. Call once per frame with the real dt.
    pub fn record(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        self.frames += 1;
        self.elapsed += dt;
        self.sum_dt += dt;
        if dt < self.min_dt {
            self.min_dt = dt;
        }
        if dt > self.max_dt {
            self.max_dt = dt;
        }
        if self.history.len() >= self.capacity {
            self.history.remove(0);
        }
        self.history.push(dt);
    }

    /// Frames recorded so far.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// Total elapsed time.
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// Average FPS over all recorded frames.
    pub fn avg_fps(&self) -> f32 {
        if self.frames == 0 {
            return 0.0;
        }
        self.frames as f32 / self.elapsed
    }

    /// Smoothed FPS from the rolling history.
    pub fn smooth_fps(&self) -> f32 {
        if self.history.is_empty() {
            return 0.0;
        }
        let avg = self.history.iter().sum::<f32>() / self.history.len() as f32;
        1.0 / avg
    }

    /// Worst-case frame time (maximum dt seen).
    pub fn worst_ms(&self) -> f32 {
        self.max_dt * 1000.0
    }

    /// Best-case frame time (minimum dt seen).
    pub fn best_ms(&self) -> f32 {
        if self.min_dt == f32::MAX {
            0.0
        } else {
            self.min_dt * 1000.0
        }
    }

    /// Last frame time in milliseconds.
    pub fn last_ms(&self) -> f32 {
        self.history.last().copied().unwrap_or(0.0) * 1000.0
    }

    /// Format a one-line summary for a HUD or terminal status line.
    pub fn summary(&self) -> String {
        format!(
            "fps={:.0}  avg={:.0}  worst={:.1}ms  frames={}",
            self.smooth_fps(),
            self.avg_fps(),
            self.worst_ms(),
            self.frames
        )
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new(60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_frames() {
        let mut m = Metrics::new(10);
        for _ in 0..60 {
            m.record(1.0 / 60.0);
        }
        assert_eq!(m.frames(), 60);
        assert!((m.avg_fps() - 60.0).abs() < 1.0);
    }

    #[test]
    fn history_bounded() {
        let mut m = Metrics::new(5);
        for _ in 0..20 {
            m.record(0.016);
        }
        assert_eq!(m.history.len(), 5);
    }

    #[test]
    fn worst_ms_tracks_spikes() {
        let mut m = Metrics::new(10);
        m.record(0.016);
        m.record(0.1);
        m.record(0.016);
        assert!((m.worst_ms() - 100.0).abs() < 1.0);
    }

    #[test]
    fn summary_is_nonempty() {
        let mut m = Metrics::new(10);
        m.record(0.016);
        assert!(!m.summary().is_empty());
    }

    #[test]
    fn zero_frames_returns_zero_fps() {
        let m = Metrics::default();
        assert_eq!(m.avg_fps(), 0.0);
        assert_eq!(m.smooth_fps(), 0.0);
    }
}
