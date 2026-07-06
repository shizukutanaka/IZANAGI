//! Item crafting — recipes that transform ingredient sets into new items.
//!
//! The kit could *store* items ([`inventory`](crate::inventory)),
//! *enchant* them ([`affix`](crate::affix)), and *randomly generate*
//! encounter drops ([`encounter`](crate::encounter)), but had no
//! *transformation* primitive: "consume these specific quantities of materials
//! and produce something new." [`Recipe`] is that primitive.
//!
//! A [`Recipe`] is intentionally decoupled from any specific inventory
//! representation — it does not reference [`Inventory`](crate::inventory::Inventory)
//! directly. Instead, `can_craft` and `try_craft` accept closures, so the same
//! recipe works with any storage that supports "how many of K do you have?" and
//! "remove N of K from storage":
//!
//! ```
//! use izanagi_kit::recipe::{Recipe, Ingredient};
//!
//! #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
//! enum Mat { IronOre, Coal, WoodLog }
//!
//! // Item IDs: 1 = iron_ore, 2 = coal, 100 = steel_sword.
//! let steel_sword = Recipe::new(
//!     "Steel Sword",
//!     vec![
//!         Ingredient { key: Mat::IronOre, count: 2 },
//!         Ingredient { key: Mat::Coal,    count: 1 },
//!     ],
//!     100u32, // steel_sword item id
//! );
//!
//! // Simulate a small stockpile.
//! let mut stock = std::collections::BTreeMap::from([
//!     (Mat::IronOre, 3u32),
//!     (Mat::Coal, 2),
//! ]);
//!
//! assert!(steel_sword.can_craft(|k| *stock.get(k).unwrap_or(&0)));
//!
//! let snap = stock.clone();
//! let result = steel_sword.try_craft(
//!     |k| *snap.get(k).unwrap_or(&0),
//!     |k, n| *stock.entry(*k).or_insert(0) -= n,
//! );
//! assert_eq!(result, Some(100u32));
//! assert_eq!(stock[&Mat::IronOre], 1); // 3 - 2 = 1
//! assert_eq!(stock[&Mat::Coal],    1); // 2 - 1 = 1
//! ```
//!
//! ## Design
//!
//! Ingredients are stored sorted by key (`K: Ord`) for deterministic
//! iteration and hashing. Duplicate keys in the constructor are merged (their
//! counts summed). A recipe with no ingredients always succeeds (a "free"
//! recipe). [`Recipe`] implements [`DetHash`](crate::world_hash::DetHash),
//! folding the ingredient list and output into the replay checksum.

use crate::world_hash::{DetHash, Fnv1a};

/// A single ingredient requirement: `count` units of item type `key`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ingredient<K> {
    /// The item type key.
    pub key: K,
    /// Required quantity (≥ 1 is meaningful; 0-count ingredients are no-ops).
    pub count: u32,
}

/// A crafting recipe: a sorted ingredient list and an output value.
#[derive(Clone, Debug)]
pub struct Recipe<K: Ord + Clone, O: Clone> {
    name: String,
    /// Sorted by `key` with duplicate keys merged (counts summed).
    ingredients: Vec<Ingredient<K>>,
    output: O,
}

impl<K: Ord + Clone, O: Clone + PartialEq> Recipe<K, O> {
    /// Create a recipe from an (unsorted, possibly duplicate-key) ingredient
    /// list. Ingredients are sorted by key and duplicates are merged.
    pub fn new(name: impl Into<String>, mut ingredients: Vec<Ingredient<K>>, output: O) -> Self {
        // Sort then merge duplicate keys.
        ingredients.sort_by(|a, b| a.key.cmp(&b.key));
        let mut merged: Vec<Ingredient<K>> = Vec::new();
        for ing in ingredients {
            if let Some(last) = merged.last_mut() {
                if last.key == ing.key {
                    last.count = last.count.saturating_add(ing.count);
                    continue;
                }
            }
            merged.push(ing);
        }
        // Drop zero-count ingredients (they are trivially satisfied).
        merged.retain(|i| i.count > 0);
        Recipe {
            name: name.into(),
            ingredients: merged,
            output,
        }
    }

    /// The recipe's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The output produced when the recipe is crafted.
    pub fn output(&self) -> &O {
        &self.output
    }

    /// The normalised ingredient list (sorted, merged, positive counts only).
    pub fn ingredients(&self) -> &[Ingredient<K>] {
        &self.ingredients
    }

    /// The number of distinct ingredient types.
    pub fn ingredient_count(&self) -> usize {
        self.ingredients.len()
    }

    /// `true` if `available(k)` returns at least the required count for every
    /// ingredient. A recipe with no ingredients always returns `true`.
    pub fn can_craft<F>(&self, available: F) -> bool
    where
        F: Fn(&K) -> u32,
    {
        self.ingredients
            .iter()
            .all(|i| available(&i.key) >= i.count)
    }

    /// Attempt to craft: if `can_craft` succeeds, call `consume(k, n)` for
    /// each ingredient and return `Some(output.clone())`. If any ingredient is
    /// insufficient, return `None` without calling `consume` at all.
    pub fn try_craft<F, G>(&self, available: F, mut consume: G) -> Option<O>
    where
        F: Fn(&K) -> u32,
        G: FnMut(&K, u32),
    {
        if !self.can_craft(&available) {
            return None;
        }
        for i in &self.ingredients {
            consume(&i.key, i.count);
        }
        Some(self.output.clone())
    }
}

impl<K: Ord + Clone + DetHash, O: Clone + PartialEq + DetHash> DetHash for Recipe<K, O> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_str(&self.name);
        hasher.write_u32(self.ingredients.len() as u32);
        for i in &self.ingredients {
            i.key.det_hash(hasher);
            hasher.write_u32(i.count);
        }
        self.output.det_hash(hasher);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;
    use std::collections::BTreeMap;

    fn stock(pairs: &[(u32, u32)]) -> BTreeMap<u32, u32> {
        pairs.iter().copied().collect()
    }

    // Use u32 output to satisfy the DetHash bound without needing &str: DetHash.
    fn recipe_ab() -> Recipe<u32, u32> {
        Recipe::new(
            "AB",
            vec![
                Ingredient { key: 1, count: 2 },
                Ingredient { key: 2, count: 1 },
            ],
            99u32,
        )
    }

    fn avail(s: &BTreeMap<u32, u32>, k: &u32) -> u32 {
        *s.get(k).unwrap_or(&0)
    }

    #[test]
    fn test_can_craft_sufficient() {
        let r = recipe_ab();
        let s = stock(&[(1, 3), (2, 1)]);
        assert!(r.can_craft(|k| avail(&s, k)));
    }

    #[test]
    fn test_can_craft_insufficient() {
        let r = recipe_ab();
        let s = stock(&[(1, 1), (2, 1)]); // only 1 of key 1, need 2
        assert!(!r.can_craft(|k| avail(&s, k)));
    }

    #[test]
    fn test_try_craft_success_consumes() {
        let r = recipe_ab();
        let mut s = stock(&[(1, 3), (2, 2)]);
        // Snapshot for read closure — avoids overlapping borrows of s.
        let snap = s.clone();
        let out = r.try_craft(|k| avail(&snap, k), |k, n| *s.entry(*k).or_insert(0) -= n);
        assert_eq!(out, Some(99u32));
        assert_eq!(s[&1], 1, "2 consumed from 3");
        assert_eq!(s[&2], 1, "1 consumed from 2");
    }

    #[test]
    fn test_try_craft_failure_does_not_consume() {
        let r = recipe_ab();
        let s = stock(&[(1, 1), (2, 2)]);
        let mut consumed = false;
        let out = r.try_craft(|k| avail(&s, k), |_, _| consumed = true);
        assert_eq!(out, None, "should fail: only 1 of key 1");
        assert!(!consumed, "consume must not be called on failure");
    }

    #[test]
    fn test_no_ingredients_always_crafts() {
        let r: Recipe<u32, u32> = Recipe::new("free", vec![], 42);
        let empty: BTreeMap<u32, u32> = BTreeMap::new();
        assert!(r.can_craft(|k| avail(&empty, k)));
        let out = r.try_craft(|k| avail(&empty, k), |_, _| {});
        assert_eq!(out, Some(42));
    }

    #[test]
    fn test_duplicate_keys_merged() {
        let r = Recipe::new(
            "merged",
            vec![
                Ingredient {
                    key: 1u32,
                    count: 2,
                },
                Ingredient { key: 1, count: 3 },
            ],
            0u32,
        );
        assert_eq!(r.ingredient_count(), 1, "duplicate keys must be merged");
        assert_eq!(r.ingredients()[0].count, 5, "counts summed");
    }

    #[test]
    fn test_ingredients_sorted_by_key() {
        let r: Recipe<u32, u32> = Recipe::new(
            "sort",
            vec![
                Ingredient { key: 5, count: 1 },
                Ingredient { key: 2, count: 1 },
                Ingredient { key: 8, count: 1 },
            ],
            0u32,
        );
        let keys: Vec<u32> = r.ingredients().iter().map(|i| i.key).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "ingredients must be in key order");
    }

    #[test]
    fn test_exact_amount_can_craft() {
        let r = recipe_ab();
        let s = stock(&[(1, 2), (2, 1)]); // exactly the required amounts
        assert!(r.can_craft(|k| *s.get(k).unwrap_or(&0)));
    }

    #[test]
    fn test_zero_count_ingredient_ignored() {
        let r: Recipe<u32, u32> = Recipe::new("zero", vec![Ingredient { key: 99, count: 0 }], 0u32);
        assert_eq!(r.ingredient_count(), 0, "zero-count ingredient dropped");
        let empty: BTreeMap<u32, u32> = BTreeMap::new();
        assert!(r.can_craft(|k| *empty.get(k).unwrap_or(&0)));
    }

    #[test]
    fn test_det_hash_canonical_and_sensitive() {
        let a = recipe_ab();
        let b = recipe_ab();
        assert_eq!(hash_state(&a), hash_state(&b), "same recipe, same hash");

        let c: Recipe<u32, u32> = Recipe::new(
            "AB",
            vec![
                Ingredient { key: 1, count: 3 }, // different count
                Ingredient { key: 2, count: 1 },
            ],
            99u32,
        );
        assert_ne!(
            hash_state(&a),
            hash_state(&c),
            "different count → different hash"
        );
    }
}
