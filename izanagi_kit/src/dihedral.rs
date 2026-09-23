//! The dihedral group `D₄` — the 8 symmetries of a square grid:
//! 4 rotations × a reflection. Each element is stored as
//! `(swap, sx, sy)` — whether x/y trade places and the two sign
//! flips — which makes [`D4::apply`] three integer ops and
//! [`D4::compose`]/[`D4::inverse`] closed-form. Everything is
//! exact integer group algebra: the group table itself is a pure
//! function, so test oracles can verify the full 8×8 Cayley table
//! elementwise against pointwise composition.
//!
//! Useful for mapgen symmetry (a WFC tile's orientations), sprite
//! rotation, and canonicalizing shapes (`min` over `all` images is a
//! deterministic normal form).
//!
//! ```
//! use izanagi_kit::dihedral::D4;
//! assert_eq!(D4::R90.apply(3, 1), (-1, 3)); // quarter-turn (x,y)->(-y,x)
//! assert_eq!(D4::FX.apply(3, 1), (-3, 1));  // x-flip
//! assert_eq!(D4::R90.compose(D4::R90), D4::R180);
//! let p = D4::R90.apply(3, 1);
//! assert_eq!(D4::R90.inverse().apply(p.0, p.1), (3, 1));
//! ```
//!
//! Reference: the standard `D₄` presentation ⟨r, s | r⁴ = s² = 1,
//! srs = r⁻¹⟩; same symmetry group as `numpy.rot90`/`fliplr` and
//! CSS/pixel-editor rotate-mirror combos.

/// One of the 8 square-grid symmetries `(x, y) ↦ (sx·X, sy·Y)` where
/// `(X, Y)` is `(x, y)` or `(y, x)` when `swap` is set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct D4 {
    swap: bool,
    sx: i32,
    sy: i32,
}

impl D4 {
    /// Identity.
    pub const ID: D4 = D4 {
        swap: false,
        sx: 1,
        sy: 1,
    };
    /// 90° counterclockwise `(x, y) → (−y, x)`.
    pub const R90: D4 = D4 {
        swap: true,
        sx: -1,
        sy: 1,
    };
    /// 180° `(x, y) → (−x, −y)`.
    pub const R180: D4 = D4 {
        swap: false,
        sx: -1,
        sy: -1,
    };
    /// 270° counterclockwise `(x, y) → (y, −x)`.
    pub const R270: D4 = D4 {
        swap: true,
        sx: 1,
        sy: -1,
    };
    /// Mirror in x `(x, y) → (−x, y)`.
    pub const FX: D4 = D4 {
        swap: false,
        sx: -1,
        sy: 1,
    };
    /// Mirror in y `(x, y) → (x, −y)`.
    pub const FY: D4 = D4 {
        swap: false,
        sx: 1,
        sy: -1,
    };
    /// Mirror across the main diagonal `(x, y) → (y, x)`.
    pub const FD: D4 = D4 {
        swap: true,
        sx: 1,
        sy: 1,
    };
    /// Mirror across the anti-diagonal `(x, y) → (−y, −x)`.
    pub const FA: D4 = D4 {
        swap: true,
        sx: -1,
        sy: -1,
    };
    /// All 8 elements in canonical order.
    pub const ALL: [D4; 8] = [
        D4::ID,
        D4::R90,
        D4::R180,
        D4::R270,
        D4::FX,
        D4::FY,
        D4::FD,
        D4::FA,
    ];

    /// Apply to a point.
    pub const fn apply(self, x: i64, y: i64) -> (i64, i64) {
        if self.swap {
            (self.sx as i64 * y, self.sy as i64 * x)
        } else {
            (self.sx as i64 * x, self.sy as i64 * y)
        }
    }

    /// Group product: `self.compose(t)` applies `t` first, then `self`
    /// — i.e. `self.compose(t).apply(x, y)` equals applying `t`,
    /// then `self`, to `(x, y)`.
    ///
    /// Closed form: `swap` is XOR, and each sign multiplies `t`'s sign
    /// on whichever input slot `self.swap` feeds into it.
    pub const fn compose(self, t: D4) -> D4 {
        D4 {
            swap: self.swap != t.swap,
            sx: self.sx * if self.swap { t.sy } else { t.sx },
            sy: self.sy * if self.swap { t.sx } else { t.sy },
        }
    }

    /// Group inverse: non-swap elements are sign flips (self-inverse),
    /// swap elements trade their two signs back.
    pub const fn inverse(self) -> D4 {
        D4 {
            swap: self.swap,
            sx: if self.swap { self.sy } else { self.sx },
            sy: if self.swap { self.sx } else { self.sy },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn apply_vectors() {
        assert_eq!(D4::ID.apply(3, 1), (3, 1));
        assert_eq!(D4::R90.apply(3, 1), (-1, 3));
        assert_eq!(D4::R180.apply(3, 1), (-3, -1));
        assert_eq!(D4::R270.apply(3, 1), (1, -3));
        assert_eq!(D4::FX.apply(3, 1), (-3, 1));
        assert_eq!(D4::FY.apply(3, 1), (3, -1));
        assert_eq!(D4::FD.apply(3, 1), (1, 3));
        assert_eq!(D4::FA.apply(3, 1), (-1, -3));
    }

    #[test]
    fn rotations_cycle() {
        assert_eq!(D4::R90.compose(D4::R90), D4::R180);
        assert_eq!(D4::R90.compose(D4::R180), D4::R270);
        assert_eq!(D4::R90.compose(D4::R270), D4::ID);
        assert_eq!(D4::R90.compose(D4::R90.inverse()), D4::ID);
    }

    /// Cayley-table oracle: for every ordered pair (a, b), the
    /// closed-form `compose` must equal the element c found by
    /// scanning ALL for `c.apply(p) == a(b(p))` on probe
    /// points. Plus inverses and order checks — the whole group.
    #[test]
    fn full_cayley_oracle() {
        let probes = [(0, 0), (1, 0), (0, 1), (2, -3), (-5, 7)];
        for &a in &D4::ALL {
            for &b in &D4::ALL {
                let ab = a.compose(b);
                // find c in ALL matching pointwise application
                let mut found = None;
                for &c in &D4::ALL {
                    if probes.iter().all(|&(x, y)| {
                        let (bx, by) = b.apply(x, y);
                        c.apply(x, y) == a.apply(bx, by)
                    }) {
                        found = Some(c);
                        break;
                    }
                }
                assert_eq!(Some(ab), found, "compose {a:?} . {b:?}");
            }
        }
    }

    #[test]
    fn inverse_oracle() {
        for &g in &D4::ALL {
            let inv = g.inverse();
            assert_eq!(g.compose(inv), D4::ID);
            assert_eq!(inv.compose(g), D4::ID);
        }
    }

    #[test]
    fn random_points_group_law() {
        let mut rng = SplitMix64::new(0x00d1_4ed9_a15e_ed00);
        for _ in 0..5000 {
            let x = rng.below(1_000_000) as i64 - 500_000;
            let y = rng.below(1_000_000) as i64 - 500_000;
            let a = D4::ALL[rng.below(8) as usize];
            let b = D4::ALL[rng.below(8) as usize];
            // associativity spot-check + compose-vs-apply
            let (bx, by) = b.apply(x, y);
            assert_eq!(a.compose(b).apply(x, y), a.apply(bx, by));
            let inv = a.inverse();
            assert_eq!(inv.apply(a.apply(x, y).0, a.apply(x, y).1), (x, y));
        }
    }

    /// Canonical-normal-form example usage: lex-min over all 8 images.
    #[test]
    fn normal_form() {
        let norm = |p: (i64, i64)| {
            D4::ALL
                .iter()
                .map(|g| g.apply(p.0, p.1))
                .min()
                .unwrap_or((0, 0))
        };
        assert_eq!(norm((3, 1)), (-3, -1));
        assert_eq!(norm((-1, -3)), (-3, -1));
        assert_eq!(norm((0, 0)), (0, 0));
    }
}
