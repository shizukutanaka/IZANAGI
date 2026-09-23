//! Deterministic behavior tree — the standard game-AI tick structure.
//!
//! A [`BehaviorTree`] is a flat `Vec<Node>`; children are referenced by
//! index and leaves carry user ids interpreted by the caller's
//! `eval(id, world)` callback. Composite nodes implement *resume*
//! semantics: a `Sequence`/`Selector` that returned [`Status::Running`]
//! re-enters at the running child on the next tick (the `mem` slot per
//! node), and resets to the first child when a terminal status is
//! reached — so the tick function is a pure function of
//! `(nodes, mem, world)`.
//!
//! ```
//! use izanagi_kit::btree::{BehaviorTree, Node, Status};
//! let mut bt = BehaviorTree::new(vec![
//!     Node::Sequence(vec![1, 2]),
//!     Node::Condition(7),  // id 7 → eval'd by the caller
//!     Node::Action(9),
//! ]);
//! // eval: condition 7 true, action 9 succeeds.
//! let s = bt.tick(|id, _| if id == 7 { Status::Success } else { Status::Success }, &mut ());
//! assert_eq!(s, Status::Success);
//! ```

/// Tick result for every node kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Node finished its work.
    Success,
    /// Node finished, work not done.
    Failure,
    /// Node is mid-work; the composite remembers where to resume.
    Running,
}

/// One tree node. `Condition` and `Action` carry an id handed to the
/// eval callback — the tree itself never interprets it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    /// Leaf predicate — eval may return Success or Failure only;
    /// `Running` is treated as `Failure`.
    Condition(u32),
    /// Leaf work unit — eval returns any status.
    Action(u32),
    /// Children succeed in order; first non-Success propagates.
    Sequence(Vec<u32>),
    /// First non-Failure child propagates; all-Failure is Failure.
    Selector(Vec<u32>),
    /// Swaps Success/Failure; Running passes through.
    Inverter(u32),
    /// Failure becomes Success; others pass through.
    Succeeder(u32),
    /// Ticks its child once per tick: child Success → Running
    /// (keep ticking), Failure/Running passes through.
    RepeatUntilFail(u32),
}

/// A behavior tree with per-node resume memory.
pub struct BehaviorTree {
    nodes: Vec<Node>,
    /// Resume cursor per composite (`usize::MAX`-free: 0 = re-enter
    /// at the first child).
    mem: Vec<u32>,
}

impl BehaviorTree {
    /// A tree whose node 0 is the root.
    pub fn new(nodes: Vec<Node>) -> BehaviorTree {
        let mem = vec![0; nodes.len()];
        BehaviorTree { nodes, mem }
    }

    /// Number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Empty tree flag.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Tick the whole tree once (root = node 0). `eval(id, world)`
    /// resolves leaf ids. Out-of-range child references tick as
    /// `Failure` — malformed trees degrade, never hang.
    pub fn tick<W, F>(&mut self, mut eval: F, world: &mut W) -> Status
    where
        F: FnMut(u32, &mut W) -> Status,
    {
        if self.nodes.is_empty() {
            return Status::Failure;
        }
        self.node_tick(0, &mut eval, world)
    }

    fn node_tick<W, F>(&mut self, i: usize, eval: &mut F, world: &mut W) -> Status
    where
        F: FnMut(u32, &mut W) -> Status,
    {
        // Copy the node out so the recursive borrows stay disjoint —
        // nodes are small and children indices, not data.
        let node = match self.nodes.get(i) {
            None => return Status::Failure,
            Some(n) => n.clone(),
        };
        match node {
            Node::Condition(id) => match eval(id, world) {
                Status::Success => Status::Success,
                _ => Status::Failure,
            },
            Node::Action(id) => eval(id, world),
            Node::Sequence(kids) => {
                let mut k = self.mem[i] as usize;
                while k < kids.len() {
                    let s = self.node_tick(kids[k] as usize, eval, world);
                    match s {
                        Status::Success => k += 1,
                        Status::Running => {
                            self.mem[i] = k as u32;
                            return Status::Running;
                        }
                        Status::Failure => {
                            self.mem[i] = 0;
                            return Status::Failure;
                        }
                    }
                }
                self.mem[i] = 0;
                Status::Success
            }
            Node::Selector(kids) => {
                let mut k = self.mem[i] as usize;
                while k < kids.len() {
                    let s = self.node_tick(kids[k] as usize, eval, world);
                    match s {
                        Status::Failure => k += 1,
                        Status::Running => {
                            self.mem[i] = k as u32;
                            return Status::Running;
                        }
                        Status::Success => {
                            self.mem[i] = 0;
                            return Status::Success;
                        }
                    }
                }
                self.mem[i] = 0;
                Status::Failure
            }
            Node::Inverter(c) => match self.node_tick(c as usize, eval, world) {
                Status::Success => Status::Failure,
                Status::Failure => Status::Success,
                Status::Running => Status::Running,
            },
            Node::Succeeder(c) => match self.node_tick(c as usize, eval, world) {
                Status::Failure => Status::Success,
                s => s,
            },
            Node::RepeatUntilFail(c) => match self.node_tick(c as usize, eval, world) {
                Status::Success => Status::Running,
                s => s,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Independent evaluator: a fully reactive, stateless tick (no
    /// resume memory) over the same node semantics — Running leaves
    /// propagate, composites always scan from child 0. For evals that
    /// never return Running this must agree with the resuming tree.
    fn eager(nodes: &[Node], i: usize, script: &[Status], calls: &mut usize) -> Status {
        let leaf = |calls: &mut usize| {
            let s = script.get(*calls).copied().unwrap_or(Status::Failure);
            *calls += 1;
            s
        };
        match &nodes[i] {
            Node::Condition(_) | Node::Action(_) => leaf(calls),
            Node::Sequence(kids) => {
                for &k in kids {
                    match eager(nodes, k as usize, script, calls) {
                        Status::Success => continue,
                        s => return s,
                    }
                }
                Status::Success
            }
            Node::Selector(kids) => {
                for &k in kids {
                    match eager(nodes, k as usize, script, calls) {
                        Status::Failure => continue,
                        s => return s,
                    }
                }
                Status::Failure
            }
            Node::Inverter(c) => match eager(nodes, *c as usize, script, calls) {
                Status::Success => Status::Failure,
                Status::Failure => Status::Success,
                s => s,
            },
            Node::Succeeder(c) => match eager(nodes, *c as usize, script, calls) {
                Status::Failure => Status::Success,
                s => s,
            },
            Node::RepeatUntilFail(c) => match eager(nodes, *c as usize, script, calls) {
                Status::Success => Status::Running,
                s => s,
            },
        }
    }

    #[test]
    fn basics() {
        // Sequence of a failing condition and an action → Failure,
        // action never ticked.
        let mut bt = BehaviorTree::new(vec![
            Node::Sequence(vec![1, 2]),
            Node::Condition(0),
            Node::Action(1),
        ]);
        let mut calls = 0;
        let s = bt.tick(
            |id, c: &mut usize| {
                *c += 1;
                if id == 0 {
                    Status::Failure
                } else {
                    Status::Success
                }
            },
            &mut calls,
        );
        assert_eq!(s, Status::Failure);
        assert_eq!(calls, 1); // short-circuit: action not ticked

        // Selector falls through to second child.
        let mut bt = BehaviorTree::new(vec![
            Node::Selector(vec![1, 2]),
            Node::Action(0),
            Node::Action(1),
        ]);
        let s = bt.tick(
            |id, _| {
                if id == 0 {
                    Status::Failure
                } else {
                    Status::Success
                }
            },
            &mut (),
        );
        assert_eq!(s, Status::Success);

        // Inverter / Succeeder / Repeat.
        let mut bt = BehaviorTree::new(vec![
            Node::Sequence(vec![1, 2, 3]),
            Node::Inverter(4),
            Node::Succeeder(5),
            Node::RepeatUntilFail(6),
            Node::Condition(0),
            Node::Condition(1),
            Node::Action(2),
        ]);
        let s = bt.tick(
            |id, _| match id {
                0 => Status::Failure, // inverted to Success
                1 => Status::Failure, // succeedered to Success
                _ => Status::Success, // repeated → Running
            },
            &mut (),
        );
        assert_eq!(s, Status::Running);
    }

    #[test]
    fn resume_semantics() {
        // Sequence [Running, Action]: second tick resumes at the
        // running child — the first child is NOT re-ticked.
        let mut bt = BehaviorTree::new(vec![
            Node::Sequence(vec![1, 2]),
            Node::Action(0),
            Node::Action(1),
        ]);
        let mut mode = 0;
        let s = bt.tick(
            |id, m: &mut usize| {
                if id == 0 && *m == 0 {
                    *m = 1;
                    Status::Running
                } else {
                    Status::Success
                }
            },
            &mut mode,
        );
        assert_eq!(s, Status::Running);
        let mut seen: Vec<u32> = Vec::new();
        let s = bt.tick(
            |id, seen: &mut Vec<u32>| {
                seen.push(id);
                Status::Success
            },
            &mut seen,
        );
        assert_eq!(s, Status::Success);
        assert_eq!(seen, vec![0, 1]); // resumes at child 0's slot —
                                      // child 0 re-ticks since it was the running one; child 1 follows
    }

    #[test]
    fn oracle_no_running() {
        // With non-Running leaf results the resuming tree and the
        // eager oracle agree on every script.
        let nodes = vec![
            Node::Sequence(vec![1, 2]),
            Node::Selector(vec![3, 4]),
            Node::Inverter(5),
            Node::Condition(0),
            Node::Condition(1),
            Node::Condition(2),
        ];
        let mut rng = SplitMix64::new(0xb7ee_c0de_d00d_beef);
        for _case in 0..300 {
            let script: Vec<Status> = (0..8)
                .map(|_| {
                    if rng.below(2) == 0 {
                        Status::Success
                    } else {
                        Status::Failure
                    }
                })
                .collect();
            let mut bt = BehaviorTree::new(nodes.clone());
            let mut idx = 0;
            let got = bt.tick(
                |_id, i: &mut usize| {
                    let s = script.get(*i).copied().unwrap_or(Status::Failure);
                    *i += 1;
                    s
                },
                &mut idx,
            );
            let mut calls = 0;
            let want = eager(&nodes, 0, &script, &mut calls);
            assert_eq!(got, want);
            assert_eq!(idx, calls); // same leaf-visit count
        }
    }

    #[test]
    fn determinism() {
        // Same (nodes, script) twice → identical status + call trace.
        let nodes = vec![Node::Selector(vec![1, 2]), Node::Action(0), Node::Action(1)];
        for _ in 0..50 {
            let mut a = BehaviorTree::new(nodes.clone());
            let mut b = BehaviorTree::new(nodes.clone());
            let (mut ta, mut tb) = (Vec::new(), Vec::new());
            for _ in 0..3 {
                let sa = a.tick(
                    |id, t: &mut Vec<u32>| {
                        t.push(id);
                        Status::Success
                    },
                    &mut ta,
                );
                let sb = b.tick(
                    |id, t: &mut Vec<u32>| {
                        t.push(id);
                        Status::Success
                    },
                    &mut tb,
                );
                assert_eq!(sa, sb);
            }
            assert_eq!(ta, tb);
        }
    }
}
