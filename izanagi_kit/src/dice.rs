//! Dice notation parsing and rolling (`"3d6+2"`).
//!
//! Tabletop dice strings are the standard data-driven way to author variable
//! quantities — weapon damage `"2d6+1"`, hit dice `"3d8"`, a saving throw
//! `"1d20"`. [`Dice`] parses that notation (panic-free, returning `None` on
//! malformed input) and rolls it from a seeded [`SplitMix64`], so the result is
//! replay-deterministic like the rest of the kit. It complements the lower-level
//! [`SplitMix64::dice`](crate::SplitMix64::dice) primitive with a typed,
//! authorable form and `min`/`max`/`average` range queries (cf. `bracket-random`'s
//! `roll_str`).
//!
//! Grammar: `[count] 'd' sides [ ('+'|'-') modifier ]`, e.g. `d20`, `3d6`,
//! `2d6+1`, `1d8-2`. Whitespace around the string is ignored; `count` defaults
//! to 1. `sides` must be ≥ 1.

use crate::rng::SplitMix64;
use crate::world_hash::{DetHash, Fnv1a};

/// A parsed dice expression: roll `count` dice of `sides` faces and add
/// `modifier`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dice {
    pub count: u32,
    pub sides: u32,
    pub modifier: i32,
}

impl Dice {
    /// Construct directly. `sides` of 0 is accepted here but
    /// [`roll`](Self::roll) will then contribute only the modifier.
    #[inline]
    pub const fn new(count: u32, sides: u32, modifier: i32) -> Self {
        Dice {
            count,
            sides,
            modifier,
        }
    }

    /// Parse dice notation `"[count]d sides [±modifier]"`. Returns `None` for
    /// malformed input or `sides == 0`. Never panics.
    ///
    /// Accepted: `"d20"`, `"3d6"`, `"2d6+1"`, `"1d8-2"`, with surrounding
    /// whitespace tolerated. A missing `count` defaults to 1.
    pub fn parse(s: &str) -> Option<Dice> {
        let s = s.trim();
        // Locate the 'd' / 'D' separator.
        let dpos = s.find(['d', 'D'])?;
        let (count_str, after_d) = s.split_at(dpos);
        let rest = &after_d[1..]; // drop the 'd'

        let count = if count_str.is_empty() {
            1
        } else {
            count_str.parse::<u32>().ok()?
        };

        // Split sides from an optional signed modifier.
        let (sides_str, modifier) = match rest.find(['+', '-']) {
            Some(p) => {
                let (sd, md) = rest.split_at(p);
                (sd, md.parse::<i32>().ok()?) // md keeps its leading sign
            }
            None => (rest, 0),
        };
        let sides = sides_str.parse::<u32>().ok()?;
        if sides == 0 {
            return None;
        }
        Some(Dice {
            count,
            sides,
            modifier,
        })
    }

    /// Roll the expression, drawing `count` values from `rng` then adding the
    /// modifier. Uses [`SplitMix64::dice`], so it consumes exactly `count`
    /// draws (when `sides > 0`) and the sum saturates rather than overflowing.
    #[inline]
    pub fn roll(&self, rng: &mut SplitMix64) -> i32 {
        let sum = rng.dice(self.count, self.sides) as i64;
        (sum + self.modifier as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }

    /// Smallest possible result: `count·1 + modifier` (0 dice when `sides == 0`).
    #[inline]
    pub fn min(&self) -> i32 {
        let dice_min = if self.sides == 0 {
            0
        } else {
            self.count as i64
        };
        (dice_min + self.modifier as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }

    /// Largest possible result: `count·sides + modifier`.
    #[inline]
    pub fn max(&self) -> i32 {
        let dice_max = self.count as i64 * self.sides as i64;
        (dice_max + self.modifier as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32
    }

    /// Expected value times 100 (to stay integer): `count·(sides+1)·50 +
    /// modifier·100`. Divide by 100 for the mean. Useful for balancing without
    /// floats.
    #[inline]
    pub fn average_x100(&self) -> i64 {
        if self.sides == 0 {
            return self.modifier as i64 * 100;
        }
        self.count as i64 * (self.sides as i64 + 1) * 50 + self.modifier as i64 * 100
    }
}

impl DetHash for Dice {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.count);
        hasher.write_u32(self.sides);
        hasher.write_i32(self.modifier);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_forms() {
        assert_eq!(Dice::parse("3d6"), Some(Dice::new(3, 6, 0)));
        assert_eq!(Dice::parse("d20"), Some(Dice::new(1, 20, 0)));
        assert_eq!(Dice::parse("2d6+1"), Some(Dice::new(2, 6, 1)));
        assert_eq!(Dice::parse("1d8-2"), Some(Dice::new(1, 8, -2)));
    }

    #[test]
    fn test_parse_uppercase_and_whitespace() {
        assert_eq!(Dice::parse("  4D10+3 "), Some(Dice::new(4, 10, 3)));
    }

    #[test]
    fn test_parse_rejects_malformed() {
        assert_eq!(Dice::parse(""), None);
        assert_eq!(Dice::parse("6"), None); // no 'd'
        assert_eq!(Dice::parse("3d"), None); // no sides
        assert_eq!(Dice::parse("3d0"), None); // zero-sided die
        assert_eq!(Dice::parse("xdy"), None); // non-numeric
        assert_eq!(Dice::parse("3d6+"), None); // dangling modifier
    }

    #[test]
    fn test_min_max_average() {
        let d = Dice::new(3, 6, 2); // 3d6+2
        assert_eq!(d.min(), 5); // 3*1 + 2
        assert_eq!(d.max(), 20); // 3*6 + 2
                                 // mean of 3d6 = 10.5, +2 = 12.5 → x100 = 1250
        assert_eq!(d.average_x100(), 1250);
    }

    #[test]
    fn test_roll_within_bounds_and_deterministic() {
        let d = Dice::parse("4d6+1").unwrap();
        let mut rng = SplitMix64::new(0xD1CE);
        for _ in 0..1000 {
            let r = d.roll(&mut rng);
            assert!(
                r >= d.min() && r <= d.max(),
                "roll {r} out of [{},{}]",
                d.min(),
                d.max()
            );
        }
        // Same seed reproduces the same sequence.
        let seq = |seed: u64| {
            let mut r = SplitMix64::new(seed);
            (0..8).map(|_| d.roll(&mut r)).collect::<Vec<_>>()
        };
        assert_eq!(seq(7), seq(7));
    }

    #[test]
    fn test_roll_negative_modifier_can_go_below_zero() {
        let d = Dice::new(1, 4, -10); // 1d4-10 → range [-9, -6]
        let mut rng = SplitMix64::new(1);
        for _ in 0..200 {
            let r = d.roll(&mut rng);
            assert!((-9..=-6).contains(&r), "got {r}");
        }
    }

    #[test]
    fn test_det_hash_distinguishes_specs() {
        use crate::world_hash::hash_state;
        assert_ne!(
            hash_state(&Dice::new(2, 6, 0)),
            hash_state(&Dice::new(3, 6, 0))
        );
    }
}
