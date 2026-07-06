//! Entity parent/child relationship tracking.
//!
//! `Relations` maintains a forest of parent→children mappings. Each entity
//! (`Entity` generational handle) can have at most one parent and any number
//! of children. The common use cases are:
//!
//! - Equipped items parented to their wielder.
//! - Projectiles parented to their firer for ownership tracking.
//! - Skeletal segments or mounted riders.
//!
//! ## Invariants
//! - `attach(child, parent)` removes any existing parent of `child` first.
//! - Cycle detection: attaching would be a no-op if `parent` is already a
//!   descendant of `child` (prevents cycles); the call returns `false`.
//! - Removing an entity removes it from its parent's children list and
//!   optionally detaches or re-parents its children.
//!
//! `DetHash` folds (entity, parent) pairs in canonical entity-index order so
//! the hash is insertion-order-independent.

use std::collections::VecDeque;

use crate::{
    entity::Entity,
    world_hash::{DetHash, Fnv1a},
};

/// Parent/child relationship store for a set of entities.
#[derive(Clone, Debug, Default)]
pub struct Relations {
    /// Maps child → parent.
    parents: Vec<(Entity, Entity)>,
    /// Maps parent → children (one entry per child).
    children: Vec<(Entity, Entity)>,
}

impl Relations {
    pub fn new() -> Self {
        Relations::default()
    }

    /// Attach `child` to `parent`. Returns `false` if attaching would create
    /// a cycle (parent is already a descendant of child); returns `true` on
    /// success. Any existing parent of `child` is removed first.
    pub fn attach(&mut self, child: Entity, parent: Entity) -> bool {
        if child == parent {
            return false;
        }
        // Cycle check: is parent already a descendant of child?
        if self.is_ancestor(child, parent) {
            return false;
        }
        // Detach existing parent if any.
        self.detach(child);
        self.parents.push((child, parent));
        self.children.push((parent, child));
        true
    }

    /// Remove the parent relationship of `child`. No-op if `child` has no parent.
    pub fn detach(&mut self, child: Entity) {
        if let Some(pos) = self.parents.iter().position(|(c, _)| *c == child) {
            let (_, parent) = self.parents.swap_remove(pos);
            if let Some(cpos) = self
                .children
                .iter()
                .position(|(p, c)| *p == parent && *c == child)
            {
                self.children.swap_remove(cpos);
            }
        }
    }

    /// Remove all relationships involving `entity` (as parent or child).
    /// Children of `entity` become root entities (their parent entry is removed).
    pub fn remove_entity(&mut self, entity: Entity) {
        // Remove all children of entity (collect first to avoid borrowing
        // `self.children` while `detach` mutates it).
        for child in self.children_of(entity) {
            self.detach(child);
        }
        // Also detach entity from its own parent.
        self.detach(entity);
    }

    /// Get the parent of `entity`, or `None` if it is a root.
    pub fn parent_of(&self, entity: Entity) -> Option<Entity> {
        self.parents
            .iter()
            .find_map(|(c, p)| if *c == entity { Some(*p) } else { None })
    }

    /// Iterate the direct children of `entity` in insertion order. Private
    /// helper backing `children_of` / `child_count` / `child_at_index` so the
    /// `self.children` filter-by-parent scan lives in one place.
    fn children_iter(&self, entity: Entity) -> impl Iterator<Item = Entity> + '_ {
        self.children
            .iter()
            .filter(move |(p, _)| *p == entity)
            .map(|(_, c)| *c)
    }

    /// Get all direct children of `entity` as a `Vec`.
    pub fn children_of(&self, entity: Entity) -> Vec<Entity> {
        self.children_iter(entity).collect()
    }

    /// Number of direct children of `entity`. Avoids the `Vec` allocation of
    /// `children_of` when only the count is needed.
    pub fn child_count(&self, entity: Entity) -> usize {
        self.children_iter(entity).count()
    }

    /// Whether `entity` has at least one direct child. Shorthand for
    /// `child_count > 0` — avoids spelling out the comparison at every call site
    /// that only needs a boolean ("is this a leaf?").
    #[inline]
    pub fn has_children(&self, entity: Entity) -> bool {
        self.children_iter(entity).next().is_some()
    }

    /// Return the `idx`-th direct child of `entity` in insertion order, or
    /// `None` if `entity` has fewer than `idx + 1` children.
    ///
    /// Avoids the full `Vec` allocation of [`children_of`](Self::children_of)
    /// when only a single child is needed (e.g. "next in sequence" patterns or
    /// index-based child selection for AI).
    pub fn child_at_index(&self, entity: Entity, idx: usize) -> Option<Entity> {
        self.children_iter(entity).nth(idx)
    }

    /// All entities in the subtree rooted at `entity`, **excluding** `entity`
    /// itself, in breadth-first order (children before grandchildren).
    ///
    /// Useful for "kill entity and all carried/mounted children" patterns.
    pub fn descendants_of(&self, entity: Entity) -> Vec<Entity> {
        let mut result = self.children_of(entity);
        let mut i = 0;
        while i < result.len() {
            let e = result[i];
            result.extend(self.children_of(e));
            i += 1;
        }
        result
    }

    /// Walk up the tree and return the root ancestor of `entity`. If `entity`
    /// has no parent it is its own root.
    pub fn root_of(&self, entity: Entity) -> Entity {
        let mut current = entity;
        while let Some(p) = self.parent_of(current) {
            current = p;
        }
        current
    }

    /// True if `ancestor` is an ancestor of `descendant` (direct or indirect).
    pub fn is_ancestor(&self, ancestor: Entity, descendant: Entity) -> bool {
        let mut current = descendant;
        loop {
            match self.parent_of(current) {
                None => return false,
                Some(p) if p == ancestor => return true,
                Some(p) => current = p,
            }
        }
    }

    /// Depth of `entity` in the tree (root = 0).
    pub fn depth(&self, entity: Entity) -> usize {
        let mut d = 0;
        let mut current = entity;
        while let Some(p) = self.parent_of(current) {
            d += 1;
            current = p;
        }
        d
    }

    /// True if `entity` has no parent.
    pub fn is_root(&self, entity: Entity) -> bool {
        self.parent_of(entity).is_none()
    }

    /// True if `entity` has no children.
    pub fn is_leaf(&self, entity: Entity) -> bool {
        !self.children.iter().any(|(p, _)| *p == entity)
    }

    /// Total nodes in the subtree rooted at `entity`, including `entity`
    /// itself. A leaf returns `1`; an entity with two children returns `3`.
    /// Useful for container-capacity budgets and UI size estimates.
    #[inline]
    pub fn subtree_size(&self, entity: Entity) -> usize {
        1 + self.descendants_of(entity).len()
    }

    /// Number of (child, parent) relationships.
    pub fn len(&self) -> usize {
        self.parents.len()
    }

    /// True if no relationships are stored.
    pub fn is_empty(&self) -> bool {
        self.parents.is_empty()
    }

    /// Remove all relationships.
    pub fn clear(&mut self) {
        self.parents.clear();
        self.children.clear();
    }

    /// Detach every direct child of `parent`, making them root entities, and
    /// return them. Returns an empty `Vec` if `parent` has no children.
    ///
    /// Useful for "drop all carried items on death" patterns where the caller
    /// needs the list of newly-freed entities to process further.
    pub fn detach_all_children(&mut self, parent: Entity) -> Vec<Entity> {
        let children = self.children_of(parent);
        for &child in &children {
            self.detach(child);
        }
        children
    }

    /// Walk up the parent chain and return the full path from `entity` to the
    /// root, **inclusive** of both endpoints.
    ///
    /// The returned `Vec` is ordered `[entity, parent, grandparent, …, root]`.
    /// If `entity` has no parent the result is `[entity]`.
    pub fn path_to_root(&self, entity: Entity) -> Vec<Entity> {
        let mut path = vec![entity];
        let mut current = entity;
        while let Some(p) = self.parent_of(current) {
            path.push(p);
            current = p;
        }
        path
    }

    /// Lowest common ancestor of `e1` and `e2`, or `None` if they share no
    /// ancestor (including if both are roots in different trees).
    ///
    /// Uses the `path_to_root` of `e1` as a lookup set and walks `e2`'s chain
    /// until a common node is found.
    pub fn find_common_ancestor(&self, e1: Entity, e2: Entity) -> Option<Entity> {
        let chain1 = self.path_to_root(e1);
        let mut current = e2;
        loop {
            if chain1.contains(&current) {
                return Some(current);
            }
            match self.parent_of(current) {
                None => return None,
                Some(p) => current = p,
            }
        }
    }

    /// All direct siblings of `entity` — entities that share the same parent,
    /// **excluding** `entity` itself. Returns an empty `Vec` if `entity` is a
    /// root (no parent) or is the only child of its parent.
    ///
    /// Useful for item-group queries ("other items in the same container") and
    /// companion grouping ("adjacent squad members").
    pub fn siblings_of(&self, entity: Entity) -> Vec<Entity> {
        match self.parent_of(entity) {
            None => Vec::new(),
            Some(parent) => self
                .children_of(parent)
                .into_iter()
                .filter(|&e| e != entity)
                .collect(),
        }
    }

    /// Move all children of `old_parent` to `new_parent`. Returns `true` if at
    /// least one child was reparented. A no-op (returns `false`) when
    /// `old_parent == new_parent` or `old_parent` has no children.
    pub fn reparent_all(&mut self, old_parent: Entity, new_parent: Entity) -> bool {
        if old_parent == new_parent {
            return false;
        }
        let children = self.detach_all_children(old_parent);
        if children.is_empty() {
            return false;
        }
        for child in children {
            self.attach(child, new_parent);
        }
        true
    }

    /// Iterate `(child, parent)` pairs in unspecified order.
    pub fn iter(&self) -> impl Iterator<Item = (Entity, Entity)> + '_ {
        self.parents.iter().copied()
    }

    /// All entities that are parents of at least one child but are themselves
    /// root entities (no parent of their own), in ascending entity-index order.
    ///
    /// Useful as starting points for [`propagate`](Self::propagate) when the
    /// caller also manages entities that have no relationships at all.
    pub fn root_entities(&self) -> Vec<Entity> {
        // Collect all entities that appear as a parent.
        let mut roots: Vec<Entity> = self
            .children
            .iter()
            .map(|(p, _)| *p)
            .filter(|p| !self.parents.iter().any(|(c, _)| c == p))
            .collect();
        roots.sort_unstable_by_key(|e| e.index());
        roots.dedup_by_key(|e| e.index());
        roots
    }

    /// Visit every `(parent, child)` edge in topological BFS order — parents
    /// always before their children — calling `f(parent, child)` for each edge
    /// (W6 in `STRENGTHS_WEAKNESSES.md`).
    ///
    /// Within a given depth level children are visited in ascending entity-index
    /// order, making the traversal fully deterministic regardless of insertion
    /// history.
    ///
    /// Use this to propagate world transforms without coupling `Relations` to
    /// any specific transform type:
    ///
    /// ```rust
    /// # use izanagi_kit::{relations::Relations, entity::EntityAllocator,
    /// #                    sparse_set::SparseSet};
    /// # let mut alloc = EntityAllocator::new();
    /// # let parent = alloc.allocate();
    /// # let child  = alloc.allocate();
    /// # let mut r = Relations::new();
    /// # r.attach(child, parent);
    /// // local positions (relative to parent)
    /// let mut local: SparseSet<(i32, i32)> = SparseSet::new();
    /// // world positions (absolute, updated each frame)
    /// let mut world: SparseSet<(i32, i32)> = SparseSet::new();
    /// // seed world positions for roots with their local position
    /// for root in r.root_entities() {
    ///     let lp = local.get(root).copied().unwrap_or((0, 0));
    ///     world.insert(root, lp);
    /// }
    /// r.propagate(|par, chi| {
    ///     let (px, py) = world.get(par).copied().unwrap_or((0, 0));
    ///     let (lx, ly) = local.get(chi).copied().unwrap_or((0, 0));
    ///     world.insert(chi, (px + lx, py + ly));
    /// });
    /// ```
    pub fn propagate<F>(&self, mut f: F)
    where
        F: FnMut(Entity, Entity),
    {
        let roots = self.root_entities();
        let mut queue: VecDeque<Entity> = roots.into_iter().collect();

        while let Some(parent) = queue.pop_front() {
            let mut children = self.children_of(parent);
            children.sort_unstable_by_key(|e| e.index());
            for child in children {
                f(parent, child);
                queue.push_back(child);
            }
        }
    }
}

impl DetHash for Relations {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        // Sort by child entity index for canonical order.
        let mut pairs: Vec<(Entity, Entity)> = self.parents.clone();
        pairs.sort_by_key(|(c, _)| c.index());
        hasher.write_u32(pairs.len() as u32);
        for (child, parent) in pairs {
            child.det_hash(hasher);
            parent.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{entity::EntityAllocator, world_hash::hash_state};

    fn entities(n: usize) -> Vec<Entity> {
        let mut alloc = EntityAllocator::new();
        (0..n).map(|_| alloc.allocate()).collect()
    }

    #[test]
    fn test_attach_and_parent_of() {
        let e = entities(2);
        let mut r = Relations::new();
        assert!(r.attach(e[0], e[1]));
        assert_eq!(r.parent_of(e[0]), Some(e[1]));
        assert_eq!(r.parent_of(e[1]), None);
    }

    #[test]
    fn test_children_of() {
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[0], e[2]);
        r.attach(e[1], e[2]);
        let mut ch = r.children_of(e[2]);
        ch.sort_by_key(|e| e.index());
        assert_eq!(ch.len(), 2);
    }

    #[test]
    fn test_detach_removes_relationship() {
        let e = entities(2);
        let mut r = Relations::new();
        r.attach(e[0], e[1]);
        r.detach(e[0]);
        assert_eq!(r.parent_of(e[0]), None);
        assert!(r.children_of(e[1]).is_empty());
    }

    #[test]
    fn test_re_attach_removes_old_parent() {
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[0], e[1]);
        r.attach(e[0], e[2]); // re-parent to e[2]
        assert_eq!(r.parent_of(e[0]), Some(e[2]));
        assert!(r.children_of(e[1]).is_empty());
        assert_eq!(r.children_of(e[2]), vec![e[0]]);
    }

    #[test]
    fn test_cycle_prevented_direct() {
        let e = entities(2);
        let mut r = Relations::new();
        r.attach(e[0], e[1]); // e[0] -> e[1]
        assert!(!r.attach(e[1], e[0])); // would create cycle
        assert_eq!(r.parent_of(e[1]), None);
    }

    #[test]
    fn test_cycle_prevented_indirect() {
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[0], e[1]); // e[0]->e[1]
        r.attach(e[1], e[2]); // e[1]->e[2]
        assert!(!r.attach(e[2], e[0])); // e[2]->e[0] would cycle via e[1]
    }

    #[test]
    fn test_self_attach_returns_false() {
        let e = entities(1);
        let mut r = Relations::new();
        assert!(!r.attach(e[0], e[0]));
    }

    #[test]
    fn test_is_ancestor() {
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[0], e[1]);
        r.attach(e[1], e[2]);
        assert!(r.is_ancestor(e[2], e[0])); // e[2] is grandparent of e[0]
        assert!(!r.is_ancestor(e[0], e[2]));
    }

    #[test]
    fn test_depth() {
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[0], e[1]);
        r.attach(e[1], e[2]);
        assert_eq!(r.depth(e[2]), 0);
        assert_eq!(r.depth(e[1]), 1);
        assert_eq!(r.depth(e[0]), 2);
    }

    #[test]
    fn test_remove_entity_detaches_children() {
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[0], e[1]); // e[0]->e[1]
        r.attach(e[1], e[2]); // e[1]->e[2]
        r.remove_entity(e[1]);
        assert_eq!(r.parent_of(e[0]), None); // e[0] becomes root
        assert_eq!(r.parent_of(e[1]), None);
        assert!(r.children_of(e[2]).is_empty());
    }

    #[test]
    fn test_is_root_and_is_leaf() {
        let e = entities(2);
        let mut r = Relations::new();
        assert!(r.is_root(e[0]));
        assert!(r.is_leaf(e[0]));
        r.attach(e[0], e[1]);
        assert!(!r.is_root(e[0]));
        assert!(r.is_leaf(e[0]));
        assert!(r.is_root(e[1]));
        assert!(!r.is_leaf(e[1]));
    }

    #[test]
    fn test_len_and_clear() {
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[0], e[1]);
        r.attach(e[1], e[2]);
        assert_eq!(r.len(), 2);
        r.clear();
        assert_eq!(r.len(), 0);
        assert!(r.is_empty());
    }

    #[test]
    fn test_det_hash_same_relations_same_hash() {
        let e = entities(2);
        let mut a = Relations::new();
        let mut b = Relations::new();
        a.attach(e[0], e[1]);
        b.attach(e[0], e[1]);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_descendants_of_empty_for_leaf() {
        let e = entities(1);
        let r = Relations::new();
        assert!(r.descendants_of(e[0]).is_empty());
    }

    #[test]
    fn test_descendants_of_multi_level() {
        // e[3] → e[2] → e[1] (e[1] is root), e[0] also child of e[2]
        let e = entities(4);
        let mut r = Relations::new();
        r.attach(e[2], e[1]);
        r.attach(e[3], e[2]);
        r.attach(e[0], e[2]);
        // descendants of e[1]: e[2], then e[3] and e[0]
        let mut desc = r.descendants_of(e[1]);
        desc.sort_by_key(|x| x.index());
        assert!(desc.contains(&e[2]));
        assert!(desc.contains(&e[3]));
        assert!(desc.contains(&e[0]));
        assert_eq!(desc.len(), 3);
    }

    #[test]
    fn test_root_of_returns_self_when_no_parent() {
        let e = entities(1);
        let r = Relations::new();
        assert_eq!(r.root_of(e[0]), e[0]);
    }

    #[test]
    fn test_root_of_walks_to_root() {
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[0], e[1]); // e[0] → e[1]
        r.attach(e[1], e[2]); // e[1] → e[2]
        assert_eq!(r.root_of(e[0]), e[2]);
        assert_eq!(r.root_of(e[1]), e[2]);
        assert_eq!(r.root_of(e[2]), e[2]);
    }

    #[test]
    fn test_det_hash_different_relations_different_hash() {
        let e = entities(3);
        let mut a = Relations::new();
        let mut b = Relations::new();
        a.attach(e[0], e[1]);
        b.attach(e[0], e[2]);
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_detach_all_children_returns_and_removes() {
        let e = entities(4);
        let mut r = Relations::new();
        r.attach(e[0], e[3]);
        r.attach(e[1], e[3]);
        r.attach(e[2], e[3]);
        let mut freed = r.detach_all_children(e[3]);
        freed.sort_by_key(|x| x.index());
        assert_eq!(freed.len(), 3);
        // All children are now roots.
        for &child in e.iter().take(3) {
            assert!(r.is_root(child));
        }
        assert!(r.children_of(e[3]).is_empty());
    }

    #[test]
    fn test_detach_all_children_empty_parent_returns_empty() {
        let e = entities(1);
        let mut r = Relations::new();
        let freed = r.detach_all_children(e[0]);
        assert!(freed.is_empty());
    }

    #[test]
    fn test_detach_all_children_does_not_remove_parent_of_entity() {
        // e[0] is a child of e[1]; e[1] is a child of e[2].
        // detach_all_children(e[1]) should free e[0] but leave e[1]→e[2].
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[0], e[1]);
        r.attach(e[1], e[2]);
        r.detach_all_children(e[1]);
        assert!(r.is_root(e[0]));
        assert_eq!(r.parent_of(e[1]), Some(e[2])); // e[1] still has its parent
    }

    #[test]
    fn test_path_to_root_root_entity_returns_self() {
        let e = entities(1);
        let r = Relations::new();
        assert_eq!(r.path_to_root(e[0]), vec![e[0]]);
    }

    #[test]
    fn test_path_to_root_three_levels() {
        // e[0] → e[1] → e[2]  (e[2] is root)
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[0], e[1]);
        r.attach(e[1], e[2]);
        assert_eq!(r.path_to_root(e[0]), vec![e[0], e[1], e[2]]);
        assert_eq!(r.path_to_root(e[2]), vec![e[2]]);
    }

    #[test]
    fn test_find_common_ancestor_same_entity_is_itself() {
        let e = entities(2);
        let mut r = Relations::new();
        r.attach(e[0], e[1]);
        // e[1] and e[1] share e[1] as ancestor
        assert_eq!(r.find_common_ancestor(e[1], e[1]), Some(e[1]));
    }

    #[test]
    fn test_find_common_ancestor_direct_parent() {
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[0], e[2]);
        r.attach(e[1], e[2]);
        // siblings — common ancestor is e[2]
        assert_eq!(r.find_common_ancestor(e[0], e[1]), Some(e[2]));
    }

    #[test]
    fn test_find_common_ancestor_none_for_disjoint_trees() {
        let e = entities(4);
        let mut r = Relations::new();
        r.attach(e[0], e[1]); // tree 1: e[0]→e[1]
        r.attach(e[2], e[3]); // tree 2: e[2]→e[3]
        assert_eq!(r.find_common_ancestor(e[0], e[2]), None);
    }

    #[test]
    fn test_find_common_ancestor_ancestor_is_one_of_inputs() {
        // e[0]→e[1]→e[2]; LCA of e[0] and e[1] is e[1]
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[0], e[1]);
        r.attach(e[1], e[2]);
        assert_eq!(r.find_common_ancestor(e[0], e[1]), Some(e[1]));
    }

    #[test]
    fn test_siblings_of_returns_other_children() {
        // e[0], e[1], e[2] all parented to e[3]
        let e = entities(4);
        let mut r = Relations::new();
        r.attach(e[0], e[3]);
        r.attach(e[1], e[3]);
        r.attach(e[2], e[3]);
        let mut sibs = r.siblings_of(e[0]);
        sibs.sort_by_key(|x| x.index());
        assert_eq!(sibs, vec![e[1], e[2]]);
        assert!(!sibs.contains(&e[0]));
    }

    #[test]
    fn test_siblings_of_only_child_returns_empty() {
        let e = entities(2);
        let mut r = Relations::new();
        r.attach(e[0], e[1]); // e[0] is the only child of e[1]
        assert!(r.siblings_of(e[0]).is_empty());
    }

    #[test]
    fn test_siblings_of_root_returns_empty() {
        let e = entities(2);
        let mut r = Relations::new();
        r.attach(e[0], e[1]); // e[1] is a root
        assert!(r.siblings_of(e[1]).is_empty());
    }

    #[test]
    fn test_reparent_all_moves_children() {
        let e = entities(5);
        let mut r = Relations::new();
        r.attach(e[0], e[3]); // e[0], e[1], e[2] are children of e[3]
        r.attach(e[1], e[3]);
        r.attach(e[2], e[3]);
        assert!(r.reparent_all(e[3], e[4]));
        // All children should now be under e[4].
        let children = r.children_of(e[4]);
        assert_eq!(children.len(), 3);
        for &child in &[e[0], e[1], e[2]] {
            assert_eq!(r.parent_of(child), Some(e[4]));
        }
        assert!(r.children_of(e[3]).is_empty());
    }

    #[test]
    fn test_reparent_all_same_parent_returns_false() {
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[0], e[2]);
        assert!(!r.reparent_all(e[2], e[2]));
    }

    #[test]
    fn test_reparent_all_no_children_returns_false() {
        let e = entities(3);
        let mut r = Relations::new();
        // e[0] has no children
        assert!(!r.reparent_all(e[0], e[1]));
    }

    #[test]
    fn test_subtree_size_leaf_is_one() {
        let entities = entities(3);
        let mut r = Relations::new();
        r.attach(entities[1], entities[0]); // 0 is parent of 1
        r.attach(entities[2], entities[0]); // 0 is parent of 2
        assert_eq!(r.subtree_size(entities[1]), 1, "leaf node");
    }

    #[test]
    fn test_subtree_size_with_children() {
        let entities = entities(3);
        let mut r = Relations::new();
        r.attach(entities[1], entities[0]);
        r.attach(entities[2], entities[0]);
        assert_eq!(r.subtree_size(entities[0]), 3, "self + 2 children");
    }

    #[test]
    fn test_subtree_size_chain() {
        let entities = entities(4);
        let mut r = Relations::new();
        r.attach(entities[1], entities[0]);
        r.attach(entities[2], entities[1]);
        r.attach(entities[3], entities[2]);
        assert_eq!(
            r.subtree_size(entities[0]),
            4,
            "self + 3 descendants in a chain"
        );
    }

    #[test]
    fn test_child_count_no_children() {
        let e = entities(2);
        let mut r = Relations::new();
        r.attach(e[0], e[1]);
        assert_eq!(r.child_count(e[0]), 0);
    }

    #[test]
    fn test_child_count_multiple_children() {
        let e = entities(4);
        let mut r = Relations::new();
        r.attach(e[1], e[0]);
        r.attach(e[2], e[0]);
        r.attach(e[3], e[0]);
        assert_eq!(r.child_count(e[0]), 3);
    }

    #[test]
    fn test_child_count_matches_children_of_len() {
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[1], e[0]);
        r.attach(e[2], e[0]);
        assert_eq!(r.child_count(e[0]), r.children_of(e[0]).len());
    }

    #[test]
    fn test_child_at_index_first_child() {
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[1], e[0]);
        r.attach(e[2], e[0]);
        assert_eq!(r.child_at_index(e[0], 0), Some(e[1]));
    }

    #[test]
    fn test_child_at_index_out_of_bounds_is_none() {
        let e = entities(2);
        let mut r = Relations::new();
        r.attach(e[1], e[0]);
        assert_eq!(r.child_at_index(e[0], 1), None);
    }

    #[test]
    fn test_child_at_index_no_children_is_none() {
        let e = entities(1);
        let r = Relations::new();
        assert_eq!(r.child_at_index(e[0], 0), None);
    }

    #[test]
    fn test_has_children_true_when_children_present() {
        let e = entities(2);
        let mut r = Relations::new();
        r.attach(e[1], e[0]);
        assert!(r.has_children(e[0]));
    }

    #[test]
    fn test_has_children_false_for_leaf() {
        let e = entities(2);
        let mut r = Relations::new();
        r.attach(e[1], e[0]);
        assert!(!r.has_children(e[1]));
    }

    #[test]
    fn test_has_children_false_after_detach() {
        let e = entities(2);
        let mut r = Relations::new();
        r.attach(e[1], e[0]);
        r.detach(e[1]);
        assert!(!r.has_children(e[0]));
    }

    // --- root_entities / propagate (W6) ---

    #[test]
    fn test_root_entities_empty_relations() {
        let r = Relations::new();
        assert!(r.root_entities().is_empty());
    }

    #[test]
    fn test_root_entities_single_tree() {
        // e[0] → parent of e[1] and e[2]; e[0] has no parent.
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[1], e[0]);
        r.attach(e[2], e[0]);
        let roots = r.root_entities();
        assert_eq!(roots, vec![e[0]], "only e[0] is a root");
    }

    #[test]
    fn test_root_entities_two_independent_trees() {
        let e = entities(4);
        let mut r = Relations::new();
        // Tree A: e[0] → e[1]
        r.attach(e[1], e[0]);
        // Tree B: e[2] → e[3]
        r.attach(e[3], e[2]);
        let roots = r.root_entities();
        assert_eq!(roots.len(), 2);
        assert!(roots.contains(&e[0]));
        assert!(roots.contains(&e[2]));
        // Must be sorted by entity index.
        let idx: Vec<u32> = roots.iter().map(|e| e.index()).collect();
        assert!(idx.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn test_propagate_empty_no_calls() {
        let r = Relations::new();
        let mut calls = 0u32;
        r.propagate(|_, _| calls += 1);
        assert_eq!(calls, 0);
    }

    #[test]
    fn test_propagate_single_edge_calls_once() {
        let e = entities(2);
        let mut r = Relations::new();
        r.attach(e[1], e[0]); // e[1] is child of e[0]
        let mut log: Vec<(Entity, Entity)> = Vec::new();
        r.propagate(|parent, child| log.push((parent, child)));
        assert_eq!(log, vec![(e[0], e[1])]);
    }

    #[test]
    fn test_propagate_parent_visited_before_child() {
        // Grandparent → parent → child (depth-3 chain)
        let e = entities(3);
        let mut r = Relations::new();
        r.attach(e[1], e[0]); // e[1] parent = e[0]
        r.attach(e[2], e[1]); // e[2] parent = e[1]
        let mut log: Vec<(Entity, Entity)> = Vec::new();
        r.propagate(|parent, child| log.push((parent, child)));
        assert_eq!(
            log,
            vec![(e[0], e[1]), (e[1], e[2])],
            "topological order: grandparent edge before parent edge"
        );
    }

    #[test]
    fn test_propagate_children_in_index_order() {
        // e[0] has three children; they must appear in ascending entity-index order.
        let e = entities(4);
        let mut r = Relations::new();
        // Insert in reverse index order to confirm sorting.
        r.attach(e[3], e[0]);
        r.attach(e[1], e[0]);
        r.attach(e[2], e[0]);
        let mut children_visited: Vec<Entity> = Vec::new();
        r.propagate(|_, child| children_visited.push(child));
        assert_eq!(
            children_visited,
            vec![e[1], e[2], e[3]],
            "children must be visited in ascending entity-index order"
        );
    }

    #[test]
    fn test_propagate_world_transform_accumulation() {
        // Two-level hierarchy: root at (10, 0), child at local (0, 5) → world (10, 5).
        // Uses SparseSet-style maps to verify the pattern from the method doc.
        let e = entities(2);
        let mut r = Relations::new();
        r.attach(e[1], e[0]); // e[1] child of e[0]

        let mut local: std::collections::HashMap<u32, (i32, i32)> =
            std::collections::HashMap::new();
        let mut world: std::collections::HashMap<u32, (i32, i32)> =
            std::collections::HashMap::new();

        local.insert(e[0].index(), (10, 0));
        local.insert(e[1].index(), (0, 5));

        // Seed roots.
        for root in r.root_entities() {
            let lp = *local.get(&root.index()).unwrap_or(&(0, 0));
            world.insert(root.index(), lp);
        }
        // Propagate: add parent world pos to child local pos.
        r.propagate(|parent, child| {
            let (px, py) = *world.get(&parent.index()).unwrap_or(&(0, 0));
            let (lx, ly) = *local.get(&child.index()).unwrap_or(&(0, 0));
            world.insert(child.index(), (px + lx, py + ly));
        });

        assert_eq!(world[&e[0].index()], (10, 0), "root world pos unchanged");
        assert_eq!(
            world[&e[1].index()],
            (10, 5),
            "child world pos = parent + local"
        );
    }
}
