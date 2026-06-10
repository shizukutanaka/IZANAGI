//! Content -> ECS instantiation.
//!
//! Closes the pipeline: a validated [`Content`] bundle becomes a live world of
//! entities with `Position` and `Render` components, stored in the kit's
//! sparse-set ECS. Iteration order is canonical (sorted) so a loaded level
//! hashes deterministically — consistent with the replay goal.

use crate::content::{Color, Content};
use crate::entity::{Entity, EntityAllocator};
use crate::sparse_set::SparseSet;
use crate::world_hash::{DetHash, Fnv1a};

/// Numeric stats loaded from the prefab template onto a spawned entity.
///
/// Entries are stored in insertion (BTreeMap alphabetical) order so the
/// component hashes deterministically. Use `get` for O(n) key lookup or
/// iterate with `iter` for canonical enumeration.
#[derive(Clone, Debug, Default)]
pub struct Stats {
    entries: Vec<(String, i32)>,
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            entries: Vec::new(),
        }
    }

    /// Look up `key`, returning `Some(value)` if present.
    pub fn get(&self, key: &str) -> Option<i32> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| *v)
    }

    /// Iterate `(key, value)` pairs in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, i32)> + '_ {
        self.entries.iter().map(|(k, v)| (k.as_str(), *v))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Set `key` to `value`. Updates an existing entry in-place or appends a
    /// new one. Use this to apply runtime buffs or overrides without removing
    /// and re-inserting the whole `Stats` component.
    pub fn set(&mut self, key: &str, value: i32) {
        if let Some(e) = self.entries.iter_mut().find(|(k, _)| k == key) {
            e.1 = value;
        } else {
            self.entries.push((key.to_string(), value));
        }
    }
}

impl DetHash for Stats {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.entries.len() as u32);
        for (k, v) in &self.entries {
            for b in k.as_bytes() {
                hasher.write_u32(*b as u32);
            }
            hasher.write_i32(*v);
        }
    }
}

/// Grid position of an instantiated entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub x: u32,
    pub y: u32,
}

/// Terminal appearance of an instantiated entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Render {
    pub glyph: char,
    pub color: Color,
}

impl crate::world_hash::DetHash for Position {
    #[inline]
    fn det_hash(&self, hasher: &mut crate::world_hash::Fnv1a) {
        hasher.write_u32(self.x);
        hasher.write_u32(self.y);
    }
}

impl crate::world_hash::DetHash for Render {
    #[inline]
    fn det_hash(&self, hasher: &mut crate::world_hash::Fnv1a) {
        crate::world_hash::DetHash::det_hash(&self.glyph, hasher);
        crate::world_hash::DetHash::det_hash(&self.color, hasher);
    }
}

/// A populated world for one level.
pub struct LoadedLevel {
    pub alloc: EntityAllocator,
    pub positions: SparseSet<Position>,
    pub renders: SparseSet<Render>,
    /// Numeric stats from the prefab template. Only populated for entities
    /// whose prefab defines at least one stat; absent entities have no entry.
    pub stats: SparseSet<Stats>,
    pub entities: Vec<Entity>,
}

impl LoadedLevel {
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Return the first entity at grid position `(x, y)`, or `None` if no
    /// entity occupies that cell.
    ///
    /// Iterates the `positions` sparse-set in insertion order; returns the
    /// first match (there is normally at most one entity per cell, but the
    /// method makes no uniqueness assumption).
    pub fn find_entity_at(&self, x: u32, y: u32) -> Option<Entity> {
        self.entities
            .iter()
            .copied()
            .find(|&e| self.positions.get(e) == Some(&Position { x, y }))
    }
}

/// Instantiates the named level's spawns into a fresh world. Expects content
/// that already passed validation; still fails loudly on an unknown prefab or
/// missing level rather than silently dropping entities.
pub fn load_level(content: &Content, level_name: &str) -> Result<LoadedLevel, String> {
    let level = content
        .level(level_name)
        .ok_or_else(|| format!("no level named '{level_name}'"))?;

    let mut world = LoadedLevel {
        alloc: EntityAllocator::new(),
        positions: SparseSet::new(),
        renders: SparseSet::new(),
        stats: SparseSet::new(),
        entities: Vec::new(),
    };

    for spawn in &level.spawns {
        let prefab = content
            .prefab(&spawn.prefab)
            .ok_or_else(|| format!("spawn references undefined prefab '{}'", spawn.prefab))?;
        let e = world.alloc.allocate();
        world.positions.insert(
            e,
            Position {
                x: spawn.x,
                y: spawn.y,
            },
        );
        world.renders.insert(
            e,
            Render {
                glyph: prefab.glyph,
                color: prefab.color,
            },
        );
        if !prefab.stats.is_empty() {
            // BTreeMap iterates in alphabetical key order — deterministic.
            let entries = prefab.stats.iter().map(|(k, v)| (k.clone(), *v)).collect();
            world.stats.insert(e, Stats { entries });
        }
        world.entities.push(e);
    }

    Ok(world)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn loaded() -> LoadedLevel {
        let src = "\
prefab goblin
  glyph g
  color #f85149
prefab rat
  glyph r
  color #d29922
level cave 5x3
  row #####
  row #r.g#
  row #####
  spawn goblin 3 1
  spawn rat 1 1
";
        let (c, d) = parse(src);
        assert!(d.iter().all(|x| !x.is_error()));
        load_level(&c, "cave").unwrap()
    }

    #[test]
    fn test_load_count_matches_spawns() {
        assert_eq!(loaded().entity_count(), 2);
    }

    #[test]
    fn test_load_positions_and_renders() {
        let w = loaded();
        // Entities are allocated in spawn order: [0]=goblin, [1]=rat.
        let g = w.entities[0];
        assert_eq!(w.positions.get(g), Some(&Position { x: 3, y: 1 }));
        assert_eq!(w.renders.get(g).unwrap().glyph, 'g');
        let r = w.entities[1];
        assert_eq!(w.positions.get(r), Some(&Position { x: 1, y: 1 }));
        assert_eq!(w.renders.get(r).unwrap().glyph, 'r');
    }

    #[test]
    fn test_load_missing_level_errs() {
        let (c, _) = parse("prefab g\n  glyph g\n");
        assert!(load_level(&c, "nope").is_err());
    }

    #[test]
    fn test_stats_loaded_from_prefab() {
        let src = "\
prefab goblin
  glyph g
  color #f85149
  stat hp 10
  stat atk 3
level cave 3x1
  row ###
  spawn goblin 1 0
";
        let (c, d) = parse(src);
        assert!(d.iter().all(|x| !x.is_error()), "parse diags: {d:?}");
        let w = load_level(&c, "cave").unwrap();
        let e = w.entities[0];
        let s = w.stats.get(e).expect("goblin should have Stats");
        assert_eq!(s.get("hp"), Some(10));
        assert_eq!(s.get("atk"), Some(3));
        assert_eq!(s.get("missing"), None);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_stats_absent_for_no_stat_prefab() {
        let w = loaded();
        // goblin and rat have no stats in the fixture, so stats sparse set is empty.
        for &e in &w.entities {
            assert!(w.stats.get(e).is_none());
        }
    }

    #[test]
    fn test_stats_iter_alphabetical_order() {
        let src = "\
prefab orc
  glyph o
  stat zap 1
  stat atk 5
  stat hp 20
level room 1x1
  row #
  spawn orc 0 0
";
        let (c, _) = parse(src);
        let w = load_level(&c, "room").unwrap();
        let e = w.entities[0];
        let s = w.stats.get(e).unwrap();
        let keys: Vec<&str> = s.iter().map(|(k, _)| k).collect();
        // BTreeMap → alphabetical: atk, hp, zap
        assert_eq!(keys, vec!["atk", "hp", "zap"]);
    }

    #[test]
    fn test_stats_set_inserts_new_key() {
        let mut s = Stats::new();
        s.set("hp", 10);
        assert_eq!(s.get("hp"), Some(10));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_stats_set_updates_existing_key() {
        let mut s = Stats::new();
        s.set("hp", 10);
        s.set("hp", 25);
        assert_eq!(s.get("hp"), Some(25));
        assert_eq!(s.len(), 1, "no duplicate entry created");
    }

    #[test]
    fn test_stats_set_preserves_other_keys() {
        let mut s = Stats::new();
        s.set("hp", 10);
        s.set("atk", 3);
        s.set("hp", 20);
        assert_eq!(s.get("hp"), Some(20));
        assert_eq!(s.get("atk"), Some(3));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_find_entity_at_returns_entity() {
        let w = loaded();
        // entities[0] = goblin at (3, 1); entities[1] = rat at (1, 1)
        let g = w.find_entity_at(3, 1);
        assert_eq!(g, Some(w.entities[0]));
    }

    #[test]
    fn test_find_entity_at_absent_returns_none() {
        let w = loaded();
        assert_eq!(w.find_entity_at(0, 0), None);
        assert_eq!(w.find_entity_at(2, 2), None);
    }

    #[test]
    fn test_find_entity_at_second_entity() {
        let w = loaded();
        let r = w.find_entity_at(1, 1);
        assert_eq!(r, Some(w.entities[1]));
    }
}
