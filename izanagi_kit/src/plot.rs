//! Terminal ASCII/Unicode plotting — sparklines (`▁▂▃▄▅▆▇`), character line
//! plots, and horizontal histograms. Integer-only scaling over `Fixed` or
//! `i64` series; the classic character-grid sibling of the [`crate::braille`]
//! sub-cell canvas.
//!
//! ```
//! use izanagi_kit::plot;
//! let s = plot::sparkline(&[1, 4, 2, 8, 5]);
//! assert_eq!(s.chars().count(), 5);
//! ```

use crate::fixed::Fixed;

/// Eight-level block sparkline `▁▂▃▄▅▆▇` (`_` for the zero floor). The range
/// is `max(min, smallest positive)`…`max` mapped onto 8 levels; an all-equal
/// series renders all-mid so relative flatness stays visible.
pub fn sparkline(data: &[i64]) -> String {
    if data.is_empty() {
        return String::new();
    }
    let lo = *data.iter().min().unwrap_or(&0);
    let hi = *data.iter().max().unwrap_or(&0);
    const BLOCKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let span = hi - lo;
    data.iter()
        .map(|&v| {
            if span == 0 {
                return BLOCKS[3];
            }
            // level = (v - lo) * 7 / span, clamped to 0..=7
            let lvl = (((v - lo) * 7 + span / 2) / span).clamp(0, 7) as usize;
            BLOCKS[lvl]
        })
        .collect()
}

/// A character canvas for line plots: `w` columns × `h` rows of `char`,
/// filled with `fill`. Row 0 is the top (y increases downward in output
/// order; `plot` maps data y upward).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canvas {
    /// Column count.
    pub w: usize,
    /// Row count.
    pub h: usize,
    /// Cells, row-major.
    pub cells: Vec<char>,
}

impl Canvas {
    /// New blank canvas filled with `' '`.
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            cells: vec![' '; w.max(1) * h.max(1)],
        }
    }

    /// Draw `ch` at (x, y) — `y` is measured from the bottom (0 = lowest row),
    /// matching mathematical convention. Out-of-bounds writes are ignored.
    pub fn set(&mut self, x: i64, y: i64, ch: char) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x < self.w && y < self.h {
            self.cells[(self.h - 1 - y) * self.w + x] = ch;
        }
    }

    /// Render to `Vec<String>` (one per row, top first).
    pub fn render(&self) -> Vec<String> {
        self.cells
            .chunks(self.w.max(1))
            .map(|r| r.iter().collect())
            .collect()
    }

    /// Render as one newline-joined `String`.
    pub fn render_string(&self) -> String {
        self.render().join("\n")
    }
}

/// Line-plot `series` onto a `w`×`h` canvas with `ch`: x spans 0..w across
/// `series.len()` buckets (each column takes the mean of its bucket), y is
/// scaled min..max to 0..h-1. Buckets with no samples stay blank.
pub fn line(canvas: &mut Canvas, series: &[Fixed], ch: char) {
    if series.is_empty() || canvas.w == 0 || canvas.h == 0 {
        return;
    }
    let lo = series.iter().map(|f| f.raw()).min().unwrap_or(0) as i64;
    let hi = series.iter().map(|f| f.raw()).max().unwrap_or(0) as i64;
    let span = (hi - lo).max(1);
    let n = series.len() as i64;
    let h = canvas.h as i64;
    for col in 0..canvas.w as i64 {
        // Bucket bounds: [col*n/w, (col+1)*n/w)
        let from = (col * n) / (canvas.w as i64);
        let to = ((col + 1) * n) / (canvas.w as i64);
        if to <= from {
            continue;
        }
        let mut sum = 0i64;
        for i in from..to {
            sum += series[i as usize].raw() as i64;
        }
        let mean = sum / (to - from);
        let y = ((mean - lo) * (h - 1) + span / 2) / span;
        canvas.set(col, y, ch);
    }
}

/// Horizontal bar histogram: each `(label, value)` becomes
/// `"label |████ n"` — `bar_w` is the character width of the longest bar,
/// proportional to `value / max`. Returns the rows top-first.
pub fn histogram(entries: &[(&str, i64)], bar_w: usize) -> Vec<String> {
    let max = entries.iter().map(|e| e.1).max().unwrap_or(0).max(1);
    let label_w = entries
        .iter()
        .map(|e| e.0.chars().count())
        .max()
        .unwrap_or(0);
    entries
        .iter()
        .map(|(label, v)| {
            let filled = ((*v).max(0) as usize * bar_w) / (max as usize).max(1);
            let bar = "█".repeat(filled);
            format!("{label:>label_w$} |{bar} {v}")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fi(n: i32) -> Fixed {
        Fixed::from_int(n)
    }

    #[test]
    fn sparkline_levels_and_edges() {
        // 8-point ramp hits all 8 blocks ascending.
        let s = sparkline(&[0, 1, 2, 3, 4, 5, 6, 7]);
        let chars: Vec<char> = s.chars().collect();
        assert_eq!(chars[0], '▁');
        assert_eq!(chars[7], '█');
        // Monotone: levels non-decreasing.
        for w in s.chars().collect::<Vec<_>>().windows(2) {
            assert!(w[0] <= w[1]);
        }
        // Flat series → mid blocks.
        assert_eq!(sparkline(&[5, 5, 5]), "▄▄▄");
        // Empty → empty.
        assert_eq!(sparkline(&[]), "");
        // Two-point: min → ▁, max → █.
        assert_eq!(sparkline(&[0, 10]), "▁█");
    }

    #[test]
    fn line_plots_a_sineish_arc() {
        let mut c = Canvas::new(20, 10);
        // V shape: 4,3,2,1,0,1,2,3,4 values → bottom in the middle.
        let series: Vec<Fixed> = [4, 3, 2, 1, 0, 1, 2, 3, 4].iter().map(|&n| fi(n)).collect();
        line(&mut c, &series, '•');
        let rows = c.render();
        assert_eq!(rows.len(), 10);
        // Peak value (4) sits on the top row, valley (0) on the bottom.
        assert!(rows[0].contains('•'));
        assert!(rows[9].contains('•'));
        // Middle rows carry the slopes.
        let marked: usize = rows
            .iter()
            .map(|r| r.chars().filter(|&ch| ch == '•').count())
            .sum();
        assert!(marked >= 9);
    }

    #[test]
    fn line_scales_outlier() {
        let mut c = Canvas::new(8, 4);
        let series: Vec<Fixed> = [0, 0, 0, 0, 8, 0, 0, 0].iter().map(|&n| fi(n)).collect();
        line(&mut c, &series, '#');
        // The spike column is the only mark on the top row.
        let top: String = c.render()[0].clone();
        assert_eq!(top.chars().filter(|&ch| ch == '#').count(), 1);
    }

    #[test]
    fn histogram_bars_proportional_and_labels() {
        let rows = histogram(&[("a", 4), ("bb", 2), ("c", 0)], 8);
        assert_eq!(rows.len(), 3);
        // a: 4/4 → 8 cells; bb: 2/4 → 4 cells; c: 0.
        assert!(rows[0].contains("████████ 4"));
        assert!(rows[1].contains("████ 2"));
        assert!(rows[2].contains("| 0"));
        // Labels right-aligned to widest.
        assert!(rows[1].starts_with("bb |"));
    }

    #[test]
    fn canvas_bounds_and_render() {
        let mut c = Canvas::new(3, 2);
        c.set(0, 0, 'a'); // bottom-left
        c.set(2, 1, 'b'); // top-right
        c.set(-1, 5, 'x'); // ignored
        c.set(9, 0, 'y'); // ignored
        assert_eq!(c.render(), vec!["  b", "a  "]);
        assert_eq!(c.render_string(), "  b\na  ");
    }
}
