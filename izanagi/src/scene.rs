//! Scene graph — parent-child transforms.
//!
//! Each node has a local [`crate::Mat3`] and an optional parent. Global
//! transforms are derived on request. Scene is independent of ECS so
//! either can be used alone.

use crate::math::{Mat3, Vec2};

/// A handle to a scene node.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Node(u32);

struct NodeData {
    local: Mat3,
    parent: Option<Node>,
    alive: bool,
}

/// A scene of 2D transforms.
pub struct Scene {
    nodes: Vec<NodeData>,
}

impl Scene {
    /// Empty scene.
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Add a root node at identity.
    pub fn add(&mut self) -> Node {
        self.nodes.push(NodeData {
            local: Mat3::IDENTITY,
            parent: None,
            alive: true,
        });
        Node((self.nodes.len() - 1) as u32)
    }

    /// Add a child of `parent`. Inherits `parent`'s world transform.
    pub fn add_child(&mut self, parent: Node) -> Node {
        self.nodes.push(NodeData {
            local: Mat3::IDENTITY,
            parent: Some(parent),
            alive: true,
        });
        Node((self.nodes.len() - 1) as u32)
    }

    /// Remove a node. Its children become roots (they keep their current local).
    pub fn remove(&mut self, n: Node) {
        if let Some(d) = self.nodes.get_mut(n.0 as usize) {
            d.alive = false;
        }
        for other in self.nodes.iter_mut() {
            if other.parent == Some(n) {
                other.parent = None;
            }
        }
    }

    /// Set a node's local transform.
    pub fn set_local(&mut self, n: Node, t: Mat3) {
        if let Some(d) = self.nodes.get_mut(n.0 as usize) {
            d.local = t;
        }
    }

    /// Translate a node (convenience).
    pub fn translate(&mut self, n: Node, delta: Vec2) {
        if let Some(d) = self.nodes.get_mut(n.0 as usize) {
            d.local = Mat3::translation(delta) * d.local;
        }
    }

    /// Get the local transform.
    pub fn local(&self, n: Node) -> Mat3 {
        self.nodes
            .get(n.0 as usize)
            .map(|d| d.local)
            .unwrap_or(Mat3::IDENTITY)
    }

    /// Compute the world transform by walking to root.
    pub fn world(&self, n: Node) -> Mat3 {
        let mut cur = n;
        let mut acc = Mat3::IDENTITY;
        let mut guard = 0;
        loop {
            let Some(d) = self.nodes.get(cur.0 as usize) else {
                return acc;
            };
            if !d.alive {
                return acc;
            }
            acc = d.local * acc;
            match d.parent {
                Some(p) => cur = p,
                None => return acc,
            }
            guard += 1;
            if guard > 10_000 {
                return acc;
            } // cycle safety
        }
    }

    /// Number of living nodes.
    pub fn len(&self) -> usize {
        self.nodes.iter().filter(|d| d.alive).count()
    }

    /// No living nodes.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_child_world() {
        let mut s = Scene::new();
        let root = s.add();
        let child = s.add_child(root);
        s.set_local(root, Mat3::translation(Vec2::new(10.0, 0.0)));
        s.set_local(child, Mat3::translation(Vec2::new(5.0, 0.0)));
        let p = s.world(child).transform_point(Vec2::ZERO);
        assert!((p.x - 15.0).abs() < 1e-4);
    }

    #[test]
    fn translate_accumulates() {
        let mut s = Scene::new();
        let n = s.add();
        s.translate(n, Vec2::new(1.0, 0.0));
        s.translate(n, Vec2::new(2.0, 0.0));
        let p = s.local(n).transform_point(Vec2::ZERO);
        assert!((p.x - 3.0).abs() < 1e-4);
    }

    #[test]
    fn remove_orphans_children() {
        let mut s = Scene::new();
        let parent = s.add();
        let child = s.add_child(parent);
        s.remove(parent);
        assert_eq!(s.len(), 1);
        // Child's world should just be its local (no parent to walk through).
        let _ = s.world(child);
    }
}
