//! Swept AABB — continuous collision between a moving box and a
//! static box: the Minkowski-expanded slab test that answers *when*
//! the boxes first touch, not just *whether* the end position
//! overlaps. This is the fast-bullet / thin-wall gap discrete
//! overlap tests can't see, and the narrowphase a [`sap`](crate::sap)
//! broadphase list feeds into.
//!
//! Times are exact `Frac` rationals in `[0,1]` (0 = start of the
//! step, 1 = end) — no float, no fixed-point rounding: `t = 1/3`
//! stays a third.
//!
//! ```
//! use izanagi_kit::swept::swept_aabb;
//!
//! // A bullet moving +8 in x hits a wall at x=10 halfway through
//! // the step — naive end-overlap would miss nothing here (it's
//! // inside the wall at t=1) but time matters for the response.
//! let hit = swept_aabb((4, 0), (2, 2), (8, 0), (10, 0, 4, 4)).unwrap();
//! assert_eq!(hit.t.num, 1); // t = 1/2
//! assert_eq!(hit.t.den, 2);
//! assert_eq!(hit.normal, (-1, 0)); // approached from the left
//! ```

use crate::frac::Frac;
use std::cmp::Ordering;

/// Outcome of [`swept_aabb`]: contact fraction, face normal, and the
/// moving box's top-left at contact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SweptHit {
    /// Contact time `t ∈ [0,1]` — `0` means already overlapping.
    pub t: Frac,
    /// Face normal on the *target*: the axis of entry and which side
    /// the mover came from (e.g. `(-1,0)` = hit the target's left
    /// face). `(0,0)` when the boxes already overlapped.
    pub normal: (i8, i8),
    /// The mover's top-left at the contact time (truncated toward
    /// the start on exact hits).
    pub pos: (i64, i64),
}

/// Per-axis slab times: `(entry, exit)` as `Frac`s, or `None` when a
/// stationary mover never reaches the target on this axis.
fn slab(m_min: i64, m_size: i64, vel: i64, t_min: i64, t_size: i64) -> Option<(Frac, Frac)> {
    if vel == 0 {
        // Never moves on this axis: must already overlap to ever touch.
        if m_min + m_size <= t_min || t_min + t_size <= m_min {
            return None;
        }
        // Entry −∞ / exit +∞ — the axis constrains nothing.
        return Some((Frac::new(-1, 1), Frac::new(2, 1)));
    }
    let (m_max, t_max) = (m_min + m_size, t_min + t_size);
    let v = vel as i128;
    // Entry: the gap-closing edge reaches the target's near face.
    // Exit: the trailing edge passes the far face.
    let (entry, exit) = if vel > 0 {
        (
            Frac::new((t_min - m_max) as i128, v),
            Frac::new((t_max - m_min) as i128, v),
        )
    } else {
        (
            Frac::new((t_max - m_min) as i128, v),
            Frac::new((t_min - m_max) as i128, v),
        )
    };
    Some((entry, exit))
}

/// First contact of box `pos,size` translated by `vel` against
/// `target = (x,y,w,h)`, or `None` when they never overlap within the
/// step. `size` and the target's `w,h` must be ≥ 0.
pub fn swept_aabb(
    pos: (i64, i64),
    size: (i64, i64),
    vel: (i64, i64),
    target: (i64, i64, i64, i64),
) -> Option<SweptHit> {
    let (ex, tx) = slab(pos.0, size.0, vel.0, target.0, target.2)?;
    let (ey, ty) = slab(pos.1, size.1, vel.1, target.1, target.3)?;
    // Entry = the later of the two entries; exit = the earlier exit.
    let (t_entry, x_entry) = if ex.cmp_frac(&ey) != Ordering::Less {
        (ex, true)
    } else {
        (ey, false)
    };
    let t_exit = if tx.cmp_frac(&ty) == Ordering::Less {
        tx
    } else {
        ty
    };
    // No hit: entry after exit, or the whole window outside [0,1].
    let one = Frac::from_int(1);
    let zero = Frac::from_int(0);
    if t_entry.cmp_frac(&t_exit) != Ordering::Less
        || t_entry.cmp_frac(&one) != Ordering::Less
        || t_exit.cmp_frac(&zero) != Ordering::Greater
    {
        return None;
    }
    let (t, overlapped) = if t_entry.cmp_frac(&zero) == Ordering::Less {
        (zero, true)
    } else {
        (t_entry, false)
    };
    // Contact position: pos + vel·t, truncating toward the start so
    // the box stops *before* the face (fractional remainder dropped
    // toward 0 keeps it contact-exact on exact hits).
    let mul_trunc = |v: i64, t: Frac| -> i64 {
        let n = v as i128 * t.num;
        let d = t.den;
        if n >= 0 {
            (n / d) as i64
        } else {
            -(((-n) / d) as i64)
        }
    };
    let normal = if overlapped {
        (0, 0)
    } else if x_entry {
        (-(vel.0.signum() as i8), 0)
    } else {
        (0, -(vel.1.signum() as i8))
    };
    Some(SweptHit {
        t,
        normal,
        pos: (pos.0 + mul_trunc(vel.0, t), pos.1 + mul_trunc(vel.1, t)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dense-stepping oracle: smallest k/2000 at which the boxes
    /// overlap, or none. Exactness checked separately.
    fn oracle(
        pos: (i64, i64),
        size: (i64, i64),
        vel: (i64, i64),
        target: (i64, i64, i64, i64),
    ) -> Option<i64> {
        let overlap = |px: i64, py: i64| {
            px + size.0 > target.0
                && px < target.0 + target.2
                && py + size.1 > target.1
                && py < target.1 + target.3
        };
        if overlap(pos.0, pos.1) {
            return Some(0);
        }
        for k in 1..=2000i64 {
            let px = pos.0 + vel.0 * k / 2000;
            let py = pos.1 + vel.1 * k / 2000;
            // Check continuous coverage: test both corners of the
            // swept interval [t_{k-1}, t_k] by bounding-box sampling.
            let (x0, x1) = (pos.0 + vel.0 * (k - 1) / 2000, pos.0 + vel.0 * k / 2000);
            let (y0, y1) = (pos.1 + vel.1 * (k - 1) / 2000, pos.1 + vel.1 * k / 2000);
            for px in [x0.min(x1), x0.max(x1)] {
                for py in [y0.min(y1), y0.max(y1)] {
                    if overlap(px, py) {
                        return Some(k);
                    }
                }
            }
            if overlap(px, py) {
                return Some(k);
            }
        }
        None
    }

    #[test]
    fn matches_dense_oracle() {
        let mut g = crate::rng::SplitMix64::new(0x5ED);
        for _case in 0..80 {
            let pos = (g.range(-10, 10) as i64, g.range(-10, 10) as i64);
            let size = (g.range(1, 5) as i64, g.range(1, 5) as i64);
            let vel = (g.range(-16, 16) as i64, g.range(-8, 8) as i64);
            let target = (
                g.range(-5, 5) as i64,
                g.range(-5, 5) as i64,
                g.range(1, 6) as i64,
                g.range(1, 6) as i64,
            );
            let hit = swept_aabb(pos, size, vel, target);
            match (hit, oracle(pos, size, vel, target)) {
                (None, None) => {}
                (Some(h), Some(_)) => {
                    // Contact time must be an actual touch: position
                    // the box at h.pos and verify edge contact or
                    // overlap (t=0 case).
                    let (px, py) = h.pos;
                    let touching = px + size.0 >= target.0
                        && px <= target.0 + target.2
                        && py + size.1 >= target.1
                        && py <= target.1 + target.3;
                    assert!(touching, "{h:?} pos={pos:?} vel={vel:?} tgt={target:?}");
                }
                (h, o) => {
                    panic!("mismatch hit={h:?} oracle={o:?} pos={pos:?} vel={vel:?} tgt={target:?}")
                }
            }
        }
    }

    #[test]
    fn exact_contact_times_and_normals() {
        // From the left: gap of 4, moving +8 → t = 1/2, normal (−1,0).
        let h = swept_aabb((4, 0), (2, 2), (8, 0), (10, 0, 4, 4)).unwrap();
        assert_eq!((h.t.num, h.t.den), (1, 2));
        assert_eq!(h.normal, (-1, 0));
        assert_eq!(h.pos, (8, 0));
        // From the right: moving −8 from x=16 → gap 16−(10+4)=2 → t=1/4.
        let h = swept_aabb((16, 0), (2, 2), (-8, 0), (10, 0, 4, 4)).unwrap();
        assert_eq!((h.t.num, h.t.den), (1, 4));
        assert_eq!(h.normal, (1, 0));
        // Diagonal: x-entry (1/2) wins over y-entry (1/4) → x normal.
        let h = swept_aabb((0, 0), (2, 2), (20, 8), (12, 4, 4, 4)).unwrap();
        assert_eq!((h.t.num, h.t.den), (1, 2));
        assert_eq!(h.normal, (-1, 0));
        assert_eq!(h.pos, (10, 4));
    }

    #[test]
    fn tunneling_and_misses() {
        // Bullet through a thin wall: end position clear, but the
        // sweep hits — the case discrete overlap misses entirely.
        let h = swept_aabb((0, 1), (1, 1), (20, 0), (9, 0, 1, 3)).unwrap();
        assert_eq!(h.normal, (-1, 0));
        // Parallel glide next to a wall: never touches.
        assert_eq!(swept_aabb((0, 0), (2, 2), (8, 0), (0, 10, 4, 4)), None);
        // Stationary disjoint: never.
        assert_eq!(swept_aabb((0, 0), (2, 2), (0, 0), (5, 5, 2, 2)), None);
        // Stationary overlapping: t = 0, no normal.
        let h = swept_aabb((3, 3), (2, 2), (0, 0), (4, 4, 4, 4)).unwrap();
        assert_eq!((h.t.num, h.normal), (0, (0, 0)));
    }

    #[test]
    fn deterministic_twice() {
        let f = || swept_aabb((1, 2), (3, 1), (9, -4), (7, 0, 4, 8));
        assert_eq!(f(), f());
    }
}
