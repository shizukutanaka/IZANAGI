//! Two-pass connected-component labeling (Rosenfeld–Pfaltz) over a
//! binary grid — the image-analysis primitive behind blob detection,
//! island counting, and region stats, done with a union-find over
//! provisional labels instead of recursive flood fill.
//!
//! Labels are compacted in first-seen raster order starting at 1
//! (0 = background), so the whole map is a pure function of the grid —
//! replay-stable across platforms.
//!
//! ```
//! use izanagi_kit::ccl::label4;
//!
//! // Two blobs: a domino and a singleton.
//! let lab = label4(3, 2, |x, y| (x == 0) || (x == 2 && y == 1));
//! assert_eq!(lab.count(), 2);
//! assert_eq!(lab.id(0, 0), lab.id(0, 1)); // same component
//! assert_ne!(lab.id(0, 0), lab.id(2, 1));
//! ```

/// Compact label map: `id(x,y)` in `0..=count`, 0 = background.
#[derive(Clone, Debug)]
pub struct Labels {
    width: i32,
    height: i32,
    ids: Vec<u32>,
    count: u32,
}

impl Labels {
    /// Grid width.
    pub fn width(&self) -> i32 {
        self.width
    }
    /// Grid height.
    pub fn height(&self) -> i32 {
        self.height
    }
    /// Label at `(x,y)` — 0 for background/out-of-bounds.
    pub fn id(&self, x: i32, y: i32) -> u32 {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return 0;
        }
        self.ids[(y * self.width + x) as usize]
    }
    /// Number of components (labels run contiguously `1..=count`).
    pub fn count(&self) -> u32 {
        self.count
    }
    /// Cell counts per label, indexed `1..=count` (index 0 unused).
    pub fn areas(&self) -> Vec<u32> {
        let mut a = vec![0u32; self.count as usize + 1];
        for &l in &self.ids {
            if l > 0 {
                a[l as usize] += 1;
            }
        }
        a
    }
    /// Bounding boxes `(x0, y0, x1, y1)` per label `1..=count`.
    pub fn bboxes(&self) -> Vec<(i32, i32, i32, i32)> {
        let mut b = vec![(i32::MAX, i32::MAX, i32::MIN, i32::MIN); self.count as usize + 1];
        for (i, &l) in self.ids.iter().enumerate() {
            if l == 0 {
                continue;
            }
            let (x, y) = ((i as i32) % self.width, (i as i32) / self.width);
            let bb = &mut b[l as usize];
            bb.0 = bb.0.min(x);
            bb.1 = bb.1.min(y);
            bb.2 = bb.2.max(x);
            bb.3 = bb.3.max(y);
        }
        b
    }
    /// All cells carrying label `l` in raster order.
    pub fn cells(&self, l: u32) -> Vec<(i32, i32)> {
        self.ids
            .iter()
            .enumerate()
            .filter(|(_, &id)| id == l)
            .map(|(i, _)| ((i as i32) % self.width, (i as i32) / self.width))
            .collect()
    }
}

/// Union-find over provisional labels (index 0 = background).
fn root(parent: &mut [u32], mut x: u32) -> u32 {
    while parent[x as usize] != x {
        // Path halving keeps the tree flat without recursion.
        parent[x as usize] = parent[parent[x as usize] as usize];
        x = parent[x as usize];
    }
    x
}
fn unite(parent: &mut [u32], a: u32, b: u32) {
    let (ra, rb) = (root(parent, a), root(parent, b));
    if ra != rb {
        // Smaller label wins — deterministic canonicalization.
        let (lo, hi) = (ra.min(rb), ra.max(rb));
        parent[hi as usize] = lo;
    }
}

/// Label with `conn` = 4 (edge neighbors) or 8 (plus diagonals).
/// Pass 1 assigns each foreground cell the smallest already-given
/// neighbor label (or a fresh one) and records merges; pass 2 maps
/// every cell to its compacted root in first-seen order.
fn label_impl(
    width: i32,
    height: i32,
    conn: u8,
    mut is_fg: impl FnMut(i32, i32) -> bool,
) -> Labels {
    let n = (width.max(0) as usize) * (height.max(0) as usize);
    let mut provisional = vec![0u32; n];
    let mut parent: Vec<u32> = vec![0]; // parent[0] = background
    for y in 0..height {
        for x in 0..width {
            if !is_fg(x, y) {
                continue;
            }
            let idx = (y * width + x) as usize;
            // Neighbors that can already hold a label.
            let mut best = 0u32;
            let mut consider = |nx: i32, ny: i32, parent: &mut Vec<u32>, provisional: &[u32]| {
                if nx >= 0 && ny >= 0 && nx < width {
                    let l = provisional[(ny * width + nx) as usize];
                    if l != 0 {
                        if best == 0 {
                            best = l;
                        } else {
                            unite(parent, best, l);
                        }
                    }
                }
            };
            consider(x - 1, y, &mut parent, &provisional);
            consider(x, y - 1, &mut parent, &provisional);
            if conn == 8 {
                consider(x - 1, y - 1, &mut parent, &provisional);
                consider(x + 1, y - 1, &mut parent, &provisional);
            }
            if best == 0 {
                best = parent.len() as u32;
                parent.push(best);
            }
            provisional[idx] = best;
        }
    }
    // Pass 2: resolve roots and renumber first-seen.
    let mut remap = vec![0u32; parent.len()];
    let mut next = 1u32;
    let mut ids = vec![0u32; n];
    for (i, &p) in provisional.iter().enumerate() {
        if p == 0 {
            continue;
        }
        let r = root(&mut parent, p);
        if remap[r as usize] == 0 {
            remap[r as usize] = next;
            next += 1;
        }
        ids[i] = remap[r as usize];
    }
    Labels {
        width,
        height,
        ids,
        count: next - 1,
    }
}

/// 4-connected labeling — edge-adjacent cells only.
pub fn label4(width: i32, height: i32, is_fg: impl FnMut(i32, i32) -> bool) -> Labels {
    label_impl(width, height, 4, is_fg)
}

/// 8-connected labeling — diagonals connect too.
pub fn label8(width: i32, height: i32, is_fg: impl FnMut(i32, i32) -> bool) -> Labels {
    label_impl(width, height, 8, is_fg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// Brute oracle: BFS flood-fill component count.
    fn oracle_count(w: i32, h: i32, fg: &[bool]) -> u32 {
        let mut seen = vec![false; (w * h) as usize];
        let mut count = 0;
        for i in 0..(w * h) as usize {
            if !fg[i] || seen[i] {
                continue;
            }
            count += 1;
            let mut q = VecDeque::from([i]);
            seen[i] = true;
            while let Some(c) = q.pop_front() {
                let (cx, cy) = ((c as i32) % w, (c as i32) / w);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if nx >= 0 && ny >= 0 && nx < w && ny < h {
                        let ni = (ny * w + nx) as usize;
                        if fg[ni] && !seen[ni] {
                            seen[ni] = true;
                            q.push_back(ni);
                        }
                    }
                }
            }
        }
        count
    }

    #[test]
    fn matches_brute_oracle_on_random_grids() {
        let mut g = crate::rng::SplitMix64::new(0xC61);
        for w in [3, 8, 17] {
            for h in [3, 7] {
                let fg: Vec<bool> = (0..w * h).map(|_| g.below(3) == 0).collect();
                let lab = label4(w, h, |x, y| fg[(y * w + x) as usize]);
                assert_eq!(lab.count(), oracle_count(w, h, &fg));
                // Areas sum to the foreground population.
                let areas = lab.areas();
                let total: u32 = areas.iter().sum();
                assert_eq!(total, fg.iter().filter(|&&b| b).count() as u32);
            }
        }
    }

    #[test]
    fn labels_are_contiguous_and_same_component() {
        let lab = label4(5, 5, |x, y| (x + y) % 2 == 0);
        // Checkerboard: every fg cell is its own 4-conn component.
        assert_eq!(lab.count(), 13);
        for y in 0..5 {
            for x in 0..5 {
                let id = lab.id(x, y);
                if (x + y) % 2 == 0 {
                    assert!((1..=13).contains(&id));
                } else {
                    assert_eq!(id, 0);
                }
            }
        }
        // 8-conn merges the whole board into one.
        let lab8 = label8(5, 5, |x, y| (x + y) % 2 == 0);
        assert_eq!(lab8.count(), 1);
    }

    #[test]
    fn areas_bboxes_and_cells() {
        // L-triomino {(0,0),(0,1),(1,0)} + singleton {(3,2)}.
        let lab = label4(4, 3, |x, y| {
            (x == 0 && y <= 1) || (x == 1 && y == 0) || (x == 3 && y == 2)
        });
        assert_eq!(lab.count(), 2);
        let areas = lab.areas();
        assert_eq!(areas[1..].to_vec(), vec![3, 1]);
        let bb = lab.bboxes();
        assert_eq!(bb[1], (0, 0, 1, 1));
        assert_eq!(bb[2], (3, 2, 3, 2));
        assert_eq!(lab.cells(1).len(), 3);
        assert_eq!(lab.cells(2), vec![(3, 2)]);
    }

    #[test]
    fn empty_and_full_grids() {
        assert_eq!(label4(0, 0, |_, _| true).count(), 0);
        assert_eq!(label4(4, 4, |_, _| false).count(), 0);
        let full = label4(4, 4, |_, _| true);
        assert_eq!(full.count(), 1);
        assert_eq!((full.width(), full.height()), (4, 4));
        assert_eq!(full.id(-1, 0), 0);
    }

    #[test]
    fn deterministic_twice() {
        let f = |x: i32, y: i32| ((x * x + y * y) % 7) < 3;
        let a = label8(11, 9, f);
        let b = label8(11, 9, f);
        assert_eq!(a.count(), b.count());
        for y in 0..9 {
            for x in 0..11 {
                assert_eq!(a.id(x, y), b.id(x, y));
            }
        }
    }
}
