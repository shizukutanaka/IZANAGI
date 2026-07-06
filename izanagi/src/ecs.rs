//! Entity-component-system.
//!
//! Sparse storage, generational entity handles. No `unsafe`. No allocations
//! in the hot path when using [`World::for_each`] / [`World::for_each_mut`].
//!
//! ## Patterns
//!
//! ```
//! use izanagi::{World, Entity};
//!
//! let mut w = World::new();
//!
//! // Spawn + attach.
//! let e = w.spawn();
//! w.insert(e, 42_u32);
//! w.insert(e, "hello");
//!
//! // Zero-alloc iteration.
//! w.for_each::<u32>(|_entity, n| println!("{n}"));
//!
//! // Two-component iteration (entities with BOTH T and U).
//! w.for_each2::<u32, &str>(|_e, n, s| println!("{n} {s}"));
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// A handle to an entity. Invalidated after [`World::despawn`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Entity {
    pub(crate) index: u32,
    pub(crate) gen: u32,
}

impl Entity {
    /// Raw index. Unstable — may be reused after despawn.
    pub fn index(self) -> u32 {
        self.index
    }
    /// Generation. Incremented on despawn; used to detect stale handles.
    pub fn gen(self) -> u32 {
        self.gen
    }
}

// ─────────────────────────────────────────────────────────────────
// Internal column abstraction
// ─────────────────────────────────────────────────────────────────

trait Column: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn remove(&mut self, index: u32);
    fn contains(&self, index: u32) -> bool;
}

struct TypedColumn<T: 'static> {
    data: HashMap<u32, T>,
}

impl<T: 'static> Column for TypedColumn<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn remove(&mut self, index: u32) {
        self.data.remove(&index);
    }
    fn contains(&self, index: u32) -> bool {
        self.data.contains_key(&index)
    }
}

fn typed<T: 'static>(col: &dyn Column) -> Option<&TypedColumn<T>> {
    col.as_any().downcast_ref()
}
fn typed_mut<T: 'static>(col: &mut dyn Column) -> Option<&mut TypedColumn<T>> {
    col.as_any_mut().downcast_mut()
}

// ─────────────────────────────────────────────────────────────────
// World
// ─────────────────────────────────────────────────────────────────

/// The ECS world. Owns entities and all their components.
pub struct World {
    generations: Vec<u32>,
    free: Vec<u32>,
    columns: HashMap<TypeId, Box<dyn Column>>,
    alive: u64,
}

impl World {
    /// Create an empty world.
    pub fn new() -> Self {
        Self {
            generations: Vec::new(),
            free: Vec::new(),
            columns: HashMap::new(),
            alive: 0,
        }
    }

    // ── Entity lifecycle ─────────────────────────────────────────

    /// Spawn a new entity with no components.
    pub fn spawn(&mut self) -> Entity {
        self.alive += 1;
        if let Some(index) = self.free.pop() {
            Entity {
                index,
                gen: self.generations[index as usize],
            }
        } else {
            let index = self.generations.len() as u32;
            self.generations.push(0);
            Entity { index, gen: 0 }
        }
    }

    /// Despawn an entity, removing all its components.
    /// Returns `false` if the handle was already invalid.
    pub fn despawn(&mut self, e: Entity) -> bool {
        if !self.alive(e) {
            return false;
        }
        for col in self.columns.values_mut() {
            col.remove(e.index);
        }
        self.generations[e.index as usize] = self.generations[e.index as usize].wrapping_add(1);
        self.free.push(e.index);
        self.alive -= 1;
        true
    }

    /// Is this handle still valid?
    pub fn alive(&self, e: Entity) -> bool {
        (e.index as usize) < self.generations.len() && self.generations[e.index as usize] == e.gen
    }

    /// Number of living entities.
    pub fn len(&self) -> u64 {
        self.alive
    }

    /// True if no entities exist.
    pub fn is_empty(&self) -> bool {
        self.alive == 0
    }

    /// Despawn every entity and remove every component column.
    /// Generations roll forward so previously-issued handles stay invalid.
    pub fn clear(&mut self) {
        for g in self.generations.iter_mut() {
            *g = g.wrapping_add(1);
        }
        self.free.clear();
        self.columns.clear();
        self.alive = 0;
    }

    // ── Component access ─────────────────────────────────────────

    /// Attach (or replace) a component of type `T` on `e`.
    pub fn insert<T: 'static>(&mut self, e: Entity, value: T) {
        if !self.alive(e) {
            return;
        }
        let col = self.columns.entry(TypeId::of::<T>()).or_insert_with(|| {
            Box::new(TypedColumn::<T> {
                data: HashMap::new(),
            })
        });
        typed_mut::<T>(col.as_mut())
            .unwrap()
            .data
            .insert(e.index, value);
    }

    /// Remove component `T` from `e`, returning the value.
    pub fn remove<T: 'static>(&mut self, e: Entity) -> Option<T> {
        let col = self.columns.get_mut(&TypeId::of::<T>())?;
        typed_mut::<T>(col.as_mut())?.data.remove(&e.index)
    }

    /// Borrow component `T` on `e`.
    pub fn get<T: 'static>(&self, e: Entity) -> Option<&T> {
        let col = self.columns.get(&TypeId::of::<T>())?;
        typed::<T>(col.as_ref())?.data.get(&e.index)
    }

    /// Mutably borrow component `T` on `e`.
    pub fn get_mut<T: 'static>(&mut self, e: Entity) -> Option<&mut T> {
        let col = self.columns.get_mut(&TypeId::of::<T>())?;
        typed_mut::<T>(col.as_mut())?.data.get_mut(&e.index)
    }

    // ── Queries — allocating ─────────────────────────────────────

    /// Collect all `(Entity, &T)` pairs. Allocates a `Vec`.
    ///
    /// For hot loops, prefer [`World::for_each`].
    pub fn query<T: 'static>(&self) -> Vec<(Entity, &T)> {
        let Some(col) = self.columns.get(&TypeId::of::<T>()) else {
            return Vec::new();
        };
        let Some(col) = typed::<T>(col.as_ref()) else {
            return Vec::new();
        };
        col.data
            .iter()
            .filter_map(|(&index, v)| {
                let gen = *self.generations.get(index as usize)?;
                Some((Entity { index, gen }, v))
            })
            .collect()
    }

    /// Collect entities that have BOTH `T` and `U`. Allocates a `Vec`.
    ///
    /// For hot loops, prefer [`World::for_each2`].
    pub fn query2<T: 'static, U: 'static>(&self) -> Vec<(Entity, &T, &U)> {
        let Some(tc) = self.columns.get(&TypeId::of::<T>()) else {
            return Vec::new();
        };
        let Some(uc) = self.columns.get(&TypeId::of::<U>()) else {
            return Vec::new();
        };
        let Some(tc) = typed::<T>(tc.as_ref()) else {
            return Vec::new();
        };
        let Some(uc) = typed::<U>(uc.as_ref()) else {
            return Vec::new();
        };
        tc.data
            .iter()
            .filter_map(|(&index, t)| {
                let gen = *self.generations.get(index as usize)?;
                let u = uc.data.get(&index)?;
                Some((Entity { index, gen }, t, u))
            })
            .collect()
    }

    // ── Queries — zero-alloc ──────────────────────────────────────

    /// Iterate every `(Entity, &T)` without allocating.
    ///
    /// ```
    /// use izanagi::World;
    /// let mut w = World::new();
    /// let e = w.spawn();
    /// w.insert(e, 99_i32);
    /// w.for_each::<i32>(|_, n| assert_eq!(*n, 99));
    /// ```
    pub fn for_each<T: 'static>(&self, mut f: impl FnMut(Entity, &T)) {
        let Some(col) = self.columns.get(&TypeId::of::<T>()) else {
            return;
        };
        let Some(col) = typed::<T>(col.as_ref()) else {
            return;
        };
        for (&index, v) in &col.data {
            if let Some(&gen) = self.generations.get(index as usize) {
                f(Entity { index, gen }, v);
            }
        }
    }

    /// Iterate every `(Entity, &mut T)` without allocating.
    pub fn for_each_mut<T: 'static>(&mut self, mut f: impl FnMut(Entity, &mut T)) {
        let gens = &self.generations;
        let Some(col) = self.columns.get_mut(&TypeId::of::<T>()) else {
            return;
        };
        let Some(col) = typed_mut::<T>(col.as_mut()) else {
            return;
        };
        for (&index, v) in col.data.iter_mut() {
            if let Some(&gen) = gens.get(index as usize) {
                f(Entity { index, gen }, v);
            }
        }
    }

    /// Iterate entities that have BOTH `T` and `U`, without allocating.
    ///
    /// Both borrows are shared. For mixed mutability patterns, query
    /// one component type and call [`World::get`] on the other inside
    /// the loop.
    pub fn for_each2<T: 'static, U: 'static>(&self, mut f: impl FnMut(Entity, &T, &U)) {
        let Some(tc) = self.columns.get(&TypeId::of::<T>()) else {
            return;
        };
        let Some(uc) = self.columns.get(&TypeId::of::<U>()) else {
            return;
        };
        let Some(tc) = typed::<T>(tc.as_ref()) else {
            return;
        };
        let Some(uc) = typed::<U>(uc.as_ref()) else {
            return;
        };
        for (&index, t) in &tc.data {
            let Some(&gen) = self.generations.get(index as usize) else {
                continue;
            };
            let Some(u) = uc.data.get(&index) else {
                continue;
            };
            f(Entity { index, gen }, t, u);
        }
    }

    /// Iterate every `(Entity, &T, &U, &V)` triple.
    pub fn query3<T: 'static, U: 'static, V: 'static>(&self) -> Vec<(Entity, &T, &U, &V)> {
        let Some(tc) = self.columns.get(&TypeId::of::<T>()) else {
            return Vec::new();
        };
        let Some(uc) = self.columns.get(&TypeId::of::<U>()) else {
            return Vec::new();
        };
        let Some(vc) = self.columns.get(&TypeId::of::<V>()) else {
            return Vec::new();
        };
        let Some(tc) = typed::<T>(tc.as_ref()) else {
            return Vec::new();
        };
        let Some(uc) = typed::<U>(uc.as_ref()) else {
            return Vec::new();
        };
        let Some(vc) = typed::<V>(vc.as_ref()) else {
            return Vec::new();
        };
        tc.data
            .iter()
            .filter_map(|(&index, t)| {
                let gen = *self.generations.get(index as usize)?;
                let u = uc.data.get(&index)?;
                let v = vc.data.get(&index)?;
                Some((Entity { index, gen }, t, u, v))
            })
            .collect()
    }

    /// Allocation-free three-component iteration.
    pub fn for_each3<T: 'static, U: 'static, V: 'static>(
        &self,
        mut f: impl FnMut(Entity, &T, &U, &V),
    ) {
        let Some(tc) = self.columns.get(&TypeId::of::<T>()) else {
            return;
        };
        let Some(uc) = self.columns.get(&TypeId::of::<U>()) else {
            return;
        };
        let Some(vc) = self.columns.get(&TypeId::of::<V>()) else {
            return;
        };
        let Some(tc) = typed::<T>(tc.as_ref()) else {
            return;
        };
        let Some(uc) = typed::<U>(uc.as_ref()) else {
            return;
        };
        let Some(vc) = typed::<V>(vc.as_ref()) else {
            return;
        };
        for (&index, t) in &tc.data {
            let Some(&gen) = self.generations.get(index as usize) else {
                continue;
            };
            let Some(u) = uc.data.get(&index) else {
                continue;
            };
            let Some(v) = vc.data.get(&index) else {
                continue;
            };
            f(Entity { index, gen }, t, u, v);
        }
    }

    // ── Utilities ────────────────────────────────────────────────

    /// Count entities with a component of type `T`.
    pub fn count<T: 'static>(&self) -> usize {
        self.columns
            .get(&TypeId::of::<T>())
            .and_then(|c| typed::<T>(c.as_ref()))
            .map(|c| c.data.len())
            .unwrap_or(0)
    }

    /// Despawn all entities that satisfy a predicate on component `T`.
    /// Returns number despawned.
    pub fn despawn_if<T: 'static>(&mut self, mut pred: impl FnMut(&T) -> bool) -> u32 {
        let Some(col) = self.columns.get(&TypeId::of::<T>()) else {
            return 0;
        };
        let Some(col) = typed::<T>(col.as_ref()) else {
            return 0;
        };
        let to_kill: Vec<Entity> = col
            .data
            .iter()
            .filter_map(|(&index, v)| {
                if pred(v) {
                    let gen = *self.generations.get(index as usize)?;
                    Some(Entity { index, gen })
                } else {
                    None
                }
            })
            .collect();
        let n = to_kill.len() as u32;
        for e in to_kill {
            self.despawn(e);
        }
        n
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Hp(u32);
    #[derive(Debug, PartialEq)]
    struct Pos(f32, f32);
    #[derive(Debug, PartialEq)]
    struct Tag(&'static str);

    #[test]
    fn spawn_and_count() {
        let mut w = World::new();
        assert!(w.is_empty());
        w.spawn();
        w.spawn();
        assert_eq!(w.len(), 2);
    }

    #[test]
    fn insert_get() {
        let mut w = World::new();
        let e = w.spawn();
        w.insert(e, Hp(100));
        assert_eq!(w.get::<Hp>(e), Some(&Hp(100)));
    }

    #[test]
    fn despawn_invalidates() {
        let mut w = World::new();
        let e = w.spawn();
        w.insert(e, Hp(50));
        assert!(w.despawn(e));
        assert!(!w.alive(e));
        assert_eq!(w.get::<Hp>(e), None);
    }

    #[test]
    fn despawn_recycles_index() {
        let mut w = World::new();
        let a = w.spawn();
        w.despawn(a);
        let b = w.spawn();
        assert_eq!(a.index, b.index);
        assert_ne!(a.gen, b.gen);
    }

    #[test]
    fn remove_returns_value() {
        let mut w = World::new();
        let e = w.spawn();
        w.insert(e, Pos(1.0, 2.0));
        assert_eq!(w.remove::<Pos>(e), Some(Pos(1.0, 2.0)));
        assert_eq!(w.get::<Pos>(e), None);
    }

    #[test]
    fn query_all() {
        let mut w = World::new();
        for i in 0..10u32 {
            let e = w.spawn();
            w.insert(e, Hp(i));
        }
        assert_eq!(w.query::<Hp>().len(), 10);
    }

    #[test]
    fn insert_on_dead_is_noop() {
        let mut w = World::new();
        let e = w.spawn();
        w.despawn(e);
        w.insert(e, Hp(1));
        assert_eq!(w.get::<Hp>(e), None);
    }

    #[test]
    fn for_each_zero_alloc_iterates_all() {
        let mut w = World::new();
        for i in 0..5u32 {
            let e = w.spawn();
            w.insert(e, Hp(i * 10));
        }
        let mut sum = 0u32;
        w.for_each::<Hp>(|_, h| sum += h.0);
        assert_eq!(sum, 100); // 0+10+20+30+40
    }

    #[test]
    fn for_each_mut_modifies_in_place() {
        let mut w = World::new();
        for _ in 0..3 {
            let e = w.spawn();
            w.insert(e, Hp(10));
        }
        w.for_each_mut::<Hp>(|_, h| h.0 += 5);
        let mut sum = 0u32;
        w.for_each::<Hp>(|_, h| sum += h.0);
        assert_eq!(sum, 45);
    }

    #[test]
    fn for_each2_yields_only_intersection() {
        let mut w = World::new();
        let a = w.spawn();
        w.insert(a, Hp(1));
        w.insert(a, Pos(0.0, 0.0));
        let b = w.spawn();
        w.insert(b, Hp(2)); // no Pos
        let c = w.spawn();
        w.insert(c, Pos(1.0, 1.0)); // no Hp

        let mut count = 0;
        w.for_each2::<Hp, Pos>(|_, _, _| count += 1);
        assert_eq!(count, 1); // only `a`
    }

    #[test]
    fn clear_invalidates_handles() {
        let mut w = World::new();
        let e = w.spawn();
        w.insert(e, Hp(10));
        assert_eq!(w.len(), 1);
        w.clear();
        assert_eq!(w.len(), 0);
        assert!(!w.alive(e));
        assert_eq!(w.get::<Hp>(e), None);
    }

    #[test]
    fn query3_intersects_three_columns() {
        let mut w = World::new();
        let a = w.spawn();
        let b = w.spawn();
        let c = w.spawn();
        w.insert(a, Hp(1));
        w.insert(a, Pos(0.0, 0.0));
        w.insert(a, Tag("alpha"));
        w.insert(b, Hp(2));
        w.insert(b, Pos(1.0, 0.0));
        // b lacks Tag
        w.insert(c, Hp(3));
        w.insert(c, Tag("gamma"));
        // c lacks Pos
        let result = w.query3::<Hp, Pos, Tag>();
        assert_eq!(result.len(), 1);
        let mut count = 0;
        w.for_each3::<Hp, Pos, Tag>(|_, _, _, _| count += 1);
        assert_eq!(count, 1);
    }

    #[test]
    fn query2_matches_for_each2() {
        let mut w = World::new();
        for i in 0..5u32 {
            let e = w.spawn();
            w.insert(e, Hp(i));
            if i % 2 == 0 {
                w.insert(e, Tag("even"));
            }
        }
        let v = w.query2::<Hp, Tag>();
        let mut count = 0;
        w.for_each2::<Hp, Tag>(|_, _, _| count += 1);
        assert_eq!(v.len(), count);
        assert_eq!(v.len(), 3); // 0,2,4
    }

    #[test]
    fn count_per_component() {
        let mut w = World::new();
        for i in 0..7u32 {
            let e = w.spawn();
            w.insert(e, Hp(i));
        }
        assert_eq!(w.count::<Hp>(), 7);
        assert_eq!(w.count::<Pos>(), 0);
    }

    #[test]
    fn despawn_if_removes_matching() {
        let mut w = World::new();
        for i in 0..10u32 {
            let e = w.spawn();
            w.insert(e, Hp(i));
        }
        let removed = w.despawn_if::<Hp>(|h| h.0 >= 5);
        assert_eq!(removed, 5);
        assert_eq!(w.count::<Hp>(), 5);
    }
}
