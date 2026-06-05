//! Content -> ECS instantiation.
//!
//! Closes the pipeline: a validated [`Content`] bundle becomes a live world of
//! entities with `Position` and `Render` components, stored in the kit's
//! sparse-set ECS. Iteration order is canonical (sorted) so a loaded level
//! hashes deterministically — consistent with the replay goal.

use crate::content::{Color, Content};
use crate::entity::{Entity, EntityAllocator};
use crate::sparse_set::SparseSet;

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

/// A populated world for one level.
pub struct LoadedLevel {
    pub alloc: EntityAllocator,
    pub positions: SparseSet<Position>,
    pub renders: SparseSet<Render>,
    pub entities: Vec<Entity>,
}

impl LoadedLevel {
    pub fn entity_count(&self) -> usize {
        self.entities.len()
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
}
