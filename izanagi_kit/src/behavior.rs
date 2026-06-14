//! Hierarchical behavior trees for game AI (G8 in `STRENGTHS_WEAKNESSES.md`).
//!
//! A behavior tree is a decision-making structure built from composite nodes
//! (Sequence, Selector), decorator nodes (Invert, Repeat, Succeed, Fail), and
//! leaf nodes (Action, Condition). Unlike a flat FSM it expresses priority,
//! fallback, and iteration hierarchically without enumerating every transition.
//!
//! ## Design choices
//!
//! - **Data-oriented leaves**: leaf nodes store a caller-chosen identifier `A`
//!   (e.g. an action enum). Logic is supplied as closures at evaluation time,
//!   keeping the tree serializable, `Clone`-able, and `DetHash`-able.
//! - **Zero allocation during evaluation**: `evaluate` recurses on the stack;
//!   no intermediate `Vec` or `Box` is allocated per call.
//! - **No float, no OS clock, no `HashMap`** — replay-safe.
//!
//! ## Example
//! ```
//! use izanagi_kit::behavior::{BehaviorNode, BehaviorStatus, BehaviorTree};
//!
//! #[derive(Clone, Copy, PartialEq, Debug)]
//! enum Act { SeeEnemy, Attack, Flee, Idle }
//!
//! let tree = BehaviorTree::new(
//!     BehaviorNode::selector(vec![
//!         BehaviorNode::sequence(vec![
//!             BehaviorNode::condition(Act::SeeEnemy),
//!             BehaviorNode::action(Act::Attack),
//!         ]),
//!         BehaviorNode::action(Act::Idle),
//!     ])
//! );
//!
//! let mut hp: i32 = 50;
//! let status = tree.evaluate(
//!     &mut hp,
//!     |ctx, act| match act {
//!         Act::Attack => { *ctx -= 10; BehaviorStatus::Success }
//!         Act::Flee   => BehaviorStatus::Success,
//!         Act::Idle   => BehaviorStatus::Running,
//!         _           => BehaviorStatus::Failure,
//!     },
//!     |_ctx, act| match act {
//!         Act::SeeEnemy => true,
//!         _ => false,
//!     },
//! );
//! assert!(matches!(status, BehaviorStatus::Success));
//! assert_eq!(hp, 40);
//! ```

use crate::world_hash::{DetHash, Fnv1a};

/// Outcome of evaluating a single behavior tree node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BehaviorStatus {
    /// The node completed its task successfully.
    Success,
    /// The node could not complete its task.
    Failure,
    /// The node is still in progress (e.g. a multi-tick action).
    Running,
}

impl DetHash for BehaviorStatus {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u8(match self {
            BehaviorStatus::Success => 0,
            BehaviorStatus::Failure => 1,
            BehaviorStatus::Running => 2,
        });
    }
}

/// Internal representation of a behavior tree node.
enum NodeKind<A> {
    /// Run children left-to-right; succeed when all succeed, fail/run on first non-success.
    Sequence(Vec<BehaviorNode<A>>),
    /// Run children left-to-right; succeed on first success, fail when all fail.
    Selector(Vec<BehaviorNode<A>>),
    /// Flip Success ↔ Failure; pass Running through unchanged.
    Invert(Box<BehaviorNode<A>>),
    /// Repeat child up to `times`; stop early on Failure or Running.
    Repeat {
        node: Box<BehaviorNode<A>>,
        times: u32,
    },
    /// Always return Success after running child (absorbs Failure).
    Succeed(Box<BehaviorNode<A>>),
    /// Always return Failure after running child (absorbs Success).
    Fail(Box<BehaviorNode<A>>),
    /// Leaf: call the caller-supplied action callback with identifier `A`.
    Action(A),
    /// Leaf: call the caller-supplied condition callback; maps `true` → Success.
    Condition(A),
}

/// A behavior tree node. Parameterized over `A`, the action/condition
/// identifier type (typically a small enum).
///
/// Build trees with the factory methods ([`sequence`], [`selector`], [`action`],
/// etc.), then evaluate them by passing logic callbacks to [`evaluate`].
///
/// [`sequence`]: BehaviorNode::sequence
/// [`selector`]: BehaviorNode::selector
/// [`action`]: BehaviorNode::action
/// [`evaluate`]: BehaviorNode::evaluate
pub struct BehaviorNode<A> {
    kind: NodeKind<A>,
}

impl<A: Clone> BehaviorNode<A> {
    // ── Composite nodes ──────────────────────────────────────────────────

    /// Run children in order. Returns the first non-Success result; succeeds
    /// only when every child succeeds. An empty sequence always succeeds.
    pub fn sequence(children: Vec<Self>) -> Self {
        Self {
            kind: NodeKind::Sequence(children),
        }
    }

    /// Run children in order. Returns the first Success; fails only when every
    /// child fails. An empty selector always fails.
    pub fn selector(children: Vec<Self>) -> Self {
        Self {
            kind: NodeKind::Selector(children),
        }
    }

    // ── Decorator nodes ──────────────────────────────────────────────────

    /// Invert the child's result: Success → Failure, Failure → Success.
    /// Running is passed through unchanged.
    pub fn invert(child: Self) -> Self {
        Self {
            kind: NodeKind::Invert(Box::new(child)),
        }
    }

    /// Run `child` up to `times` iterations. Stops on the first Failure or
    /// Running result, returning that status. Returns Success after all
    /// iterations complete. `times = 0` returns Success immediately without
    /// running the child.
    pub fn repeat(child: Self, times: u32) -> Self {
        Self {
            kind: NodeKind::Repeat {
                node: Box::new(child),
                times,
            },
        }
    }

    /// Run `child` then return Success regardless of its outcome.
    /// Useful to make an optional step in a Sequence non-blocking.
    pub fn succeed(child: Self) -> Self {
        Self {
            kind: NodeKind::Succeed(Box::new(child)),
        }
    }

    /// Run `child` then return Failure regardless of its outcome.
    pub fn fail(child: Self) -> Self {
        Self {
            kind: NodeKind::Fail(Box::new(child)),
        }
    }

    // ── Leaf nodes ───────────────────────────────────────────────────────

    /// Action leaf: call `action(ctx, id)` and return whatever it returns.
    pub fn action(id: A) -> Self {
        Self {
            kind: NodeKind::Action(id),
        }
    }

    /// Condition leaf: call `condition(ctx, id)` and return Success / Failure.
    pub fn condition(id: A) -> Self {
        Self {
            kind: NodeKind::Condition(id),
        }
    }

    // ── Evaluation ───────────────────────────────────────────────────────

    /// Evaluate this node against a mutable context `ctx`.
    ///
    /// - `action`: called for `Action` leaves; receives `(&mut ctx, &id)`.
    /// - `condition`: called for `Condition` leaves; receives `(&ctx, &id)`;
    ///   returns `true` for Success.
    ///
    /// Evaluation is fully deterministic and allocates no heap memory.
    pub fn evaluate<C, F, G>(&self, ctx: &mut C, action: &F, condition: &G) -> BehaviorStatus
    where
        F: Fn(&mut C, &A) -> BehaviorStatus,
        G: Fn(&C, &A) -> bool,
    {
        eval_node(self, ctx, action, condition)
    }

    // ── Structural queries ────────────────────────────────────────────────

    /// Total number of nodes in this subtree (self + descendants).
    pub fn node_count(&self) -> usize {
        match &self.kind {
            NodeKind::Sequence(cs) | NodeKind::Selector(cs) => {
                1 + cs.iter().map(|c| c.node_count()).sum::<usize>()
            }
            NodeKind::Invert(c)
            | NodeKind::Repeat { node: c, .. }
            | NodeKind::Succeed(c)
            | NodeKind::Fail(c) => 1 + c.node_count(),
            NodeKind::Action(_) | NodeKind::Condition(_) => 1,
        }
    }

    /// Depth of the deepest path from this node to a leaf (a single leaf = 1).
    pub fn depth(&self) -> usize {
        match &self.kind {
            NodeKind::Sequence(cs) | NodeKind::Selector(cs) => {
                1 + cs.iter().map(|c| c.depth()).max().unwrap_or(0)
            }
            NodeKind::Invert(c)
            | NodeKind::Repeat { node: c, .. }
            | NodeKind::Succeed(c)
            | NodeKind::Fail(c) => 1 + c.depth(),
            NodeKind::Action(_) | NodeKind::Condition(_) => 1,
        }
    }
}

impl<A: Clone> Clone for BehaviorNode<A> {
    fn clone(&self) -> Self {
        let kind = match &self.kind {
            NodeKind::Sequence(cs) => NodeKind::Sequence(cs.clone()),
            NodeKind::Selector(cs) => NodeKind::Selector(cs.clone()),
            NodeKind::Invert(c) => NodeKind::Invert(c.clone()),
            NodeKind::Repeat { node, times } => NodeKind::Repeat {
                node: node.clone(),
                times: *times,
            },
            NodeKind::Succeed(c) => NodeKind::Succeed(c.clone()),
            NodeKind::Fail(c) => NodeKind::Fail(c.clone()),
            NodeKind::Action(a) => NodeKind::Action(a.clone()),
            NodeKind::Condition(a) => NodeKind::Condition(a.clone()),
        };
        Self { kind }
    }
}

impl<A: DetHash> DetHash for BehaviorNode<A> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        match &self.kind {
            NodeKind::Sequence(cs) => {
                hasher.write_u8(0);
                hasher.write_u32(cs.len() as u32);
                for c in cs {
                    c.det_hash(hasher);
                }
            }
            NodeKind::Selector(cs) => {
                hasher.write_u8(1);
                hasher.write_u32(cs.len() as u32);
                for c in cs {
                    c.det_hash(hasher);
                }
            }
            NodeKind::Invert(c) => {
                hasher.write_u8(2);
                c.det_hash(hasher);
            }
            NodeKind::Repeat { node, times } => {
                hasher.write_u8(3);
                hasher.write_u32(*times);
                node.det_hash(hasher);
            }
            NodeKind::Succeed(c) => {
                hasher.write_u8(4);
                c.det_hash(hasher);
            }
            NodeKind::Fail(c) => {
                hasher.write_u8(5);
                c.det_hash(hasher);
            }
            NodeKind::Action(a) => {
                hasher.write_u8(6);
                a.det_hash(hasher);
            }
            NodeKind::Condition(a) => {
                hasher.write_u8(7);
                a.det_hash(hasher);
            }
        }
    }
}

/// Recursive evaluation helper (avoids storing closures on the stack frame of
/// methods taking `self` by reference).
fn eval_node<A, C, F, G>(
    node: &BehaviorNode<A>,
    ctx: &mut C,
    action: &F,
    condition: &G,
) -> BehaviorStatus
where
    F: Fn(&mut C, &A) -> BehaviorStatus,
    G: Fn(&C, &A) -> bool,
{
    match &node.kind {
        NodeKind::Sequence(children) => {
            for child in children {
                match eval_node(child, ctx, action, condition) {
                    BehaviorStatus::Success => continue,
                    other => return other,
                }
            }
            BehaviorStatus::Success
        }
        NodeKind::Selector(children) => {
            for child in children {
                match eval_node(child, ctx, action, condition) {
                    BehaviorStatus::Failure => continue,
                    other => return other,
                }
            }
            BehaviorStatus::Failure
        }
        NodeKind::Invert(child) => match eval_node(child, ctx, action, condition) {
            BehaviorStatus::Success => BehaviorStatus::Failure,
            BehaviorStatus::Failure => BehaviorStatus::Success,
            BehaviorStatus::Running => BehaviorStatus::Running,
        },
        NodeKind::Repeat { node: child, times } => {
            for _ in 0..*times {
                match eval_node(child, ctx, action, condition) {
                    BehaviorStatus::Success => continue,
                    other => return other,
                }
            }
            BehaviorStatus::Success
        }
        NodeKind::Succeed(child) => {
            eval_node(child, ctx, action, condition);
            BehaviorStatus::Success
        }
        NodeKind::Fail(child) => {
            eval_node(child, ctx, action, condition);
            BehaviorStatus::Failure
        }
        NodeKind::Action(id) => action(ctx, id),
        NodeKind::Condition(id) => {
            if condition(ctx, id) {
                BehaviorStatus::Success
            } else {
                BehaviorStatus::Failure
            }
        }
    }
}

/// A complete behavior tree: a named root node plus convenience evaluation.
///
/// ```
/// use izanagi_kit::behavior::{BehaviorNode, BehaviorStatus, BehaviorTree};
///
/// let tree = BehaviorTree::new(BehaviorNode::<u8>::action(1));
/// let status = tree.evaluate(&mut (), |_, _| BehaviorStatus::Success, |_, _| false);
/// assert_eq!(status, BehaviorStatus::Success);
/// ```
#[derive(Clone)]
pub struct BehaviorTree<A> {
    root: BehaviorNode<A>,
}

impl<A: Clone> BehaviorTree<A> {
    /// Wrap a root node into a tree.
    pub fn new(root: BehaviorNode<A>) -> Self {
        Self { root }
    }

    /// Access the root node.
    #[inline]
    pub fn root(&self) -> &BehaviorNode<A> {
        &self.root
    }

    /// Evaluate the tree. Equivalent to `self.root().evaluate(ctx, action, condition)`.
    pub fn evaluate<C, F, G>(&self, ctx: &mut C, action: F, condition: G) -> BehaviorStatus
    where
        F: Fn(&mut C, &A) -> BehaviorStatus,
        G: Fn(&C, &A) -> bool,
    {
        eval_node(&self.root, ctx, &action, &condition)
    }

    /// Total node count in the tree.
    #[inline]
    pub fn node_count(&self) -> usize {
        self.root.node_count()
    }

    /// Depth of the deepest path in the tree.
    #[inline]
    pub fn depth(&self) -> usize {
        self.root.depth()
    }
}

impl<A: DetHash> DetHash for BehaviorTree<A> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.root.det_hash(hasher);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    /// Tiny action set used across tests.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Act {
        Ok,
        Fail,
        Run,
        Cond(bool),
    }

    impl DetHash for Act {
        fn det_hash(&self, hasher: &mut Fnv1a) {
            match self {
                Act::Ok => hasher.write_u8(0),
                Act::Fail => hasher.write_u8(1),
                Act::Run => hasher.write_u8(2),
                Act::Cond(b) => {
                    hasher.write_u8(3);
                    hasher.write_u8(*b as u8);
                }
            }
        }
    }

    fn act(ctx: &mut Vec<Act>, id: &Act) -> BehaviorStatus {
        ctx.push(*id);
        match id {
            Act::Ok => BehaviorStatus::Success,
            Act::Fail => BehaviorStatus::Failure,
            Act::Run => BehaviorStatus::Running,
            Act::Cond(_) => BehaviorStatus::Success,
        }
    }

    fn cond(_ctx: &Vec<Act>, id: &Act) -> bool {
        matches!(id, Act::Cond(true))
    }

    // ── Sequence ──────────────────────────────────────────────────────────

    #[test]
    fn test_sequence_all_success() {
        let node = BehaviorNode::sequence(vec![
            BehaviorNode::action(Act::Ok),
            BehaviorNode::action(Act::Ok),
        ]);
        let mut log = vec![];
        assert_eq!(node.evaluate(&mut log, &act, &cond), BehaviorStatus::Success);
        assert_eq!(log, [Act::Ok, Act::Ok]);
    }

    #[test]
    fn test_sequence_stops_on_failure() {
        let node = BehaviorNode::sequence(vec![
            BehaviorNode::action(Act::Ok),
            BehaviorNode::action(Act::Fail),
            BehaviorNode::action(Act::Ok),
        ]);
        let mut log = vec![];
        assert_eq!(
            node.evaluate(&mut log, &act, &cond),
            BehaviorStatus::Failure
        );
        assert_eq!(log.len(), 2, "third node must not run");
    }

    #[test]
    fn test_sequence_stops_on_running() {
        let node = BehaviorNode::sequence(vec![
            BehaviorNode::action(Act::Run),
            BehaviorNode::action(Act::Ok),
        ]);
        let mut log = vec![];
        assert_eq!(
            node.evaluate(&mut log, &act, &cond),
            BehaviorStatus::Running
        );
        assert_eq!(log.len(), 1, "second node must not run");
    }

    #[test]
    fn test_sequence_empty_succeeds() {
        let node: BehaviorNode<Act> = BehaviorNode::sequence(vec![]);
        let mut log = vec![];
        assert_eq!(node.evaluate(&mut log, &act, &cond), BehaviorStatus::Success);
        assert!(log.is_empty());
    }

    // ── Selector ──────────────────────────────────────────────────────────

    #[test]
    fn test_selector_stops_on_success() {
        let node = BehaviorNode::selector(vec![
            BehaviorNode::action(Act::Fail),
            BehaviorNode::action(Act::Ok),
            BehaviorNode::action(Act::Ok),
        ]);
        let mut log = vec![];
        assert_eq!(node.evaluate(&mut log, &act, &cond), BehaviorStatus::Success);
        assert_eq!(log.len(), 2, "third node must not run");
    }

    #[test]
    fn test_selector_all_fail_returns_failure() {
        let node = BehaviorNode::selector(vec![
            BehaviorNode::action(Act::Fail),
            BehaviorNode::action(Act::Fail),
        ]);
        let mut log = vec![];
        assert_eq!(
            node.evaluate(&mut log, &act, &cond),
            BehaviorStatus::Failure
        );
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn test_selector_empty_fails() {
        let node: BehaviorNode<Act> = BehaviorNode::selector(vec![]);
        let mut log = vec![];
        assert_eq!(
            node.evaluate(&mut log, &act, &cond),
            BehaviorStatus::Failure
        );
    }

    #[test]
    fn test_selector_stops_on_running() {
        let node = BehaviorNode::selector(vec![
            BehaviorNode::action(Act::Fail),
            BehaviorNode::action(Act::Run),
            BehaviorNode::action(Act::Ok),
        ]);
        let mut log = vec![];
        assert_eq!(
            node.evaluate(&mut log, &act, &cond),
            BehaviorStatus::Running
        );
        assert_eq!(log.len(), 2, "third node must not run");
    }

    // ── Invert ───────────────────────────────────────────────────────────

    #[test]
    fn test_invert_success_to_failure() {
        let node = BehaviorNode::invert(BehaviorNode::action(Act::Ok));
        let mut log = vec![];
        assert_eq!(
            node.evaluate(&mut log, &act, &cond),
            BehaviorStatus::Failure
        );
    }

    #[test]
    fn test_invert_failure_to_success() {
        let node = BehaviorNode::invert(BehaviorNode::action(Act::Fail));
        let mut log = vec![];
        assert_eq!(node.evaluate(&mut log, &act, &cond), BehaviorStatus::Success);
    }

    #[test]
    fn test_invert_passes_running() {
        let node = BehaviorNode::invert(BehaviorNode::action(Act::Run));
        let mut log = vec![];
        assert_eq!(
            node.evaluate(&mut log, &act, &cond),
            BehaviorStatus::Running
        );
    }

    // ── Repeat ───────────────────────────────────────────────────────────

    #[test]
    fn test_repeat_zero_times_no_run() {
        let node = BehaviorNode::repeat(BehaviorNode::action(Act::Ok), 0);
        let mut log = vec![];
        assert_eq!(node.evaluate(&mut log, &act, &cond), BehaviorStatus::Success);
        assert!(log.is_empty(), "zero repetitions must not run child");
    }

    #[test]
    fn test_repeat_runs_n_times_on_success() {
        let node = BehaviorNode::repeat(BehaviorNode::action(Act::Ok), 3);
        let mut log = vec![];
        assert_eq!(node.evaluate(&mut log, &act, &cond), BehaviorStatus::Success);
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn test_repeat_stops_on_failure() {
        let node = BehaviorNode::repeat(BehaviorNode::action(Act::Fail), 5);
        let mut log = vec![];
        assert_eq!(
            node.evaluate(&mut log, &act, &cond),
            BehaviorStatus::Failure
        );
        assert_eq!(log.len(), 1, "must stop after first failure");
    }

    #[test]
    fn test_repeat_stops_on_running() {
        let node = BehaviorNode::repeat(BehaviorNode::action(Act::Run), 10);
        let mut log = vec![];
        assert_eq!(
            node.evaluate(&mut log, &act, &cond),
            BehaviorStatus::Running
        );
        assert_eq!(log.len(), 1, "must stop after first Running");
    }

    // ── Succeed / Fail decorators ─────────────────────────────────────────

    #[test]
    fn test_succeed_absorbs_failure() {
        let node = BehaviorNode::succeed(BehaviorNode::action(Act::Fail));
        let mut log = vec![];
        assert_eq!(node.evaluate(&mut log, &act, &cond), BehaviorStatus::Success);
        assert_eq!(log.len(), 1, "child must still run");
    }

    #[test]
    fn test_fail_absorbs_success() {
        let node = BehaviorNode::fail(BehaviorNode::action(Act::Ok));
        let mut log = vec![];
        assert_eq!(
            node.evaluate(&mut log, &act, &cond),
            BehaviorStatus::Failure
        );
        assert_eq!(log.len(), 1, "child must still run");
    }

    // ── Condition leaf ───────────────────────────────────────────────────

    #[test]
    fn test_condition_true_is_success() {
        let node = BehaviorNode::condition(Act::Cond(true));
        let mut log = vec![];
        assert_eq!(node.evaluate(&mut log, &act, &cond), BehaviorStatus::Success);
    }

    #[test]
    fn test_condition_false_is_failure() {
        let node = BehaviorNode::condition(Act::Cond(false));
        let mut log = vec![];
        assert_eq!(
            node.evaluate(&mut log, &act, &cond),
            BehaviorStatus::Failure
        );
    }

    // ── node_count / depth ────────────────────────────────────────────────

    #[test]
    fn test_node_count_leaf_is_one() {
        assert_eq!(BehaviorNode::action(Act::Ok).node_count(), 1);
        assert_eq!(BehaviorNode::condition(Act::Cond(true)).node_count(), 1);
    }

    #[test]
    fn test_node_count_sequence_two_children() {
        let node = BehaviorNode::sequence(vec![
            BehaviorNode::action(Act::Ok),
            BehaviorNode::action(Act::Fail),
        ]);
        assert_eq!(node.node_count(), 3); // sequence + 2 leaves
    }

    #[test]
    fn test_depth_leaf_is_one() {
        assert_eq!(BehaviorNode::action(Act::Ok).depth(), 1);
    }

    #[test]
    fn test_depth_nested_sequence_selector() {
        // selector → sequence → action  (depth 3)
        let node = BehaviorNode::selector(vec![BehaviorNode::sequence(vec![
            BehaviorNode::action(Act::Ok),
        ])]);
        assert_eq!(node.depth(), 3);
    }

    // ── BehaviorTree ─────────────────────────────────────────────────────

    #[test]
    fn test_tree_evaluate_delegates_to_root() {
        let tree = BehaviorTree::new(BehaviorNode::action(Act::Ok));
        let mut log = vec![];
        assert_eq!(
            tree.evaluate(&mut log, act, cond),
            BehaviorStatus::Success
        );
    }

    #[test]
    fn test_tree_node_count_and_depth() {
        let tree = BehaviorTree::new(BehaviorNode::sequence(vec![
            BehaviorNode::action(Act::Ok),
            BehaviorNode::action(Act::Fail),
        ]));
        assert_eq!(tree.node_count(), 3);
        assert_eq!(tree.depth(), 2);
    }

    // ── DetHash ──────────────────────────────────────────────────────────

    #[test]
    fn test_det_hash_same_tree_same_hash() {
        let make = || {
            BehaviorNode::selector(vec![
                BehaviorNode::action(Act::Ok),
                BehaviorNode::condition(Act::Cond(true)),
            ])
        };
        assert_eq!(hash_state(&make()), hash_state(&make()));
    }

    #[test]
    fn test_det_hash_action_vs_condition_differ() {
        let a = BehaviorNode::action(Act::Ok);
        let c = BehaviorNode::condition(Act::Ok);
        assert_ne!(
            hash_state(&a),
            hash_state(&c),
            "action and condition nodes must produce different hashes"
        );
    }

    #[test]
    fn test_det_hash_sequence_vs_selector_differ() {
        let seq = BehaviorNode::sequence(vec![BehaviorNode::action(Act::Ok)]);
        let sel = BehaviorNode::selector(vec![BehaviorNode::action(Act::Ok)]);
        assert_ne!(hash_state(&seq), hash_state(&sel));
    }

    #[test]
    fn test_det_hash_repeat_count_affects_hash() {
        let r3 = BehaviorNode::repeat(BehaviorNode::action(Act::Ok), 3);
        let r5 = BehaviorNode::repeat(BehaviorNode::action(Act::Ok), 5);
        assert_ne!(hash_state(&r3), hash_state(&r5));
    }

    // ── Roguelike integration scenario ───────────────────────────────────

    #[test]
    fn test_enemy_ai_scenario() {
        // Enemy: if HP low → flee; else if see player → attack; else patrol.
        #[derive(Clone, Copy, Debug, PartialEq)]
        enum EAct {
            LowHp,
            SeePlayer,
            Flee,
            Attack,
            Patrol,
        }
        impl DetHash for EAct {
            fn det_hash(&self, h: &mut Fnv1a) {
                h.write_u8(*self as u8);
            }
        }

        struct GameCtx {
            hp: i32,
            player_visible: bool,
            action_taken: Option<EAct>,
        }

        let tree = BehaviorTree::new(BehaviorNode::selector(vec![
            BehaviorNode::sequence(vec![
                BehaviorNode::condition(EAct::LowHp),
                BehaviorNode::action(EAct::Flee),
            ]),
            BehaviorNode::sequence(vec![
                BehaviorNode::condition(EAct::SeePlayer),
                BehaviorNode::action(EAct::Attack),
            ]),
            BehaviorNode::action(EAct::Patrol),
        ]));

        // Scenario A: high HP, player visible → attack.
        let mut ctx = GameCtx {
            hp: 50,
            player_visible: true,
            action_taken: None,
        };
        tree.evaluate(
            &mut ctx,
            |c, id| {
                c.action_taken = Some(*id);
                BehaviorStatus::Success
            },
            |c, id| match id {
                EAct::LowHp => c.hp < 20,
                EAct::SeePlayer => c.player_visible,
                _ => false,
            },
        );
        assert_eq!(ctx.action_taken, Some(EAct::Attack));

        // Scenario B: low HP → flee.
        ctx.hp = 5;
        ctx.action_taken = None;
        tree.evaluate(
            &mut ctx,
            |c, id| {
                c.action_taken = Some(*id);
                BehaviorStatus::Success
            },
            |c, id| match id {
                EAct::LowHp => c.hp < 20,
                EAct::SeePlayer => c.player_visible,
                _ => false,
            },
        );
        assert_eq!(ctx.action_taken, Some(EAct::Flee));

        // Scenario C: high HP, player not visible → patrol.
        ctx.hp = 50;
        ctx.player_visible = false;
        ctx.action_taken = None;
        tree.evaluate(
            &mut ctx,
            |c, id| {
                c.action_taken = Some(*id);
                BehaviorStatus::Success
            },
            |c, id| match id {
                EAct::LowHp => c.hp < 20,
                EAct::SeePlayer => c.player_visible,
                _ => false,
            },
        );
        assert_eq!(ctx.action_taken, Some(EAct::Patrol));
    }
}
