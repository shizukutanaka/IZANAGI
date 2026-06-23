//! Branching dialogue trees — NPC conversations with choices.
//!
//! [`fsm`](crate::fsm) and [`hfsm`](crate::hfsm) model *general* state machines
//! for AI, and [`quest`](crate::quest) tracks task completion, but neither
//! captured the specific shape of a **conversation**: a graph of nodes where
//! each node shows a line of text and offers the player a set of labelled
//! choices, and each choice navigates to another node (or ends the talk).
//! [`Dialogue`] is that primitive — a static node graph plus a runtime cursor.
//!
//! ```
//! use izanagi_kit::dialogue::{Dialogue, DialogueNode};
//!
//! // Node 0 is the greeting; choices jump to nodes 1 and 2.
//! let nodes = vec![
//!     DialogueNode::new("Well met, traveller. Need something?")
//!         .with_choice("Ask about the cave", 1)
//!         .with_choice("Just passing through", 2),
//!     DialogueNode::new("The cave? Goblins took it. Mind yourself.")
//!         .with_choice("Thanks", 2),
//!     DialogueNode::new("Safe travels."), // no choices → terminal
//! ];
//! let mut talk = Dialogue::new(nodes, 0);
//!
//! assert_eq!(talk.current_text(), Some("Well met, traveller. Need something?"));
//! assert_eq!(talk.choice_count(), 2);
//!
//! talk.choose(0);                       // "Ask about the cave"
//! assert!(talk.current_text().unwrap().contains("Goblins"));
//!
//! talk.choose(0);                       // "Thanks" → node 2
//! assert!(talk.is_at_terminal());       // node 2 has no choices
//! talk.end();
//! assert!(talk.is_ended());
//! ```
//!
//! ## Design
//!
//! Nodes live in a `Vec<DialogueNode>` indexed by `usize`; a choice's target is
//! an index into that vector. The runtime state is a single `Option<usize>`
//! cursor (`None` once the conversation has ended), so navigation is pure and
//! contains no randomness. A node with no choices is a **terminal** node: the
//! conversation displays its text and then ends on the next [`end`](Dialogue::end).
//! Out-of-range choice indices and targets are rejected without changing state,
//! so a malformed tree can never panic. [`Dialogue`] implements
//! [`DetHash`](crate::world_hash::DetHash), folding the cursor and the node
//! graph into the replay checksum.

use crate::world_hash::{DetHash, Fnv1a};

/// A single labelled choice that navigates to another node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    label: String,
    target: usize,
}

impl Choice {
    /// Create a choice with display `label` that jumps to node index `target`.
    pub fn new(label: impl Into<String>, target: usize) -> Self {
        Choice {
            label: label.into(),
            target,
        }
    }

    /// The choice's display label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The node index this choice navigates to.
    pub fn target(&self) -> usize {
        self.target
    }
}

/// A single dialogue node: a line of text plus zero or more choices.
/// A node with no choices is a terminal (the conversation ends after it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialogueNode {
    text: String,
    choices: Vec<Choice>,
}

impl DialogueNode {
    /// Create a node showing `text` with no choices yet.
    pub fn new(text: impl Into<String>) -> Self {
        DialogueNode {
            text: text.into(),
            choices: Vec::new(),
        }
    }

    /// Add a choice (builder style) navigating to node index `target`.
    pub fn with_choice(mut self, label: impl Into<String>, target: usize) -> Self {
        self.choices.push(Choice::new(label, target));
        self
    }

    /// The node's text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The node's choices.
    pub fn choices(&self) -> &[Choice] {
        &self.choices
    }

    /// `true` if this node has no choices (a terminal node).
    pub fn is_terminal(&self) -> bool {
        self.choices.is_empty()
    }
}

/// A branching conversation: a static node graph with a runtime cursor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dialogue {
    nodes: Vec<DialogueNode>,
    current: Option<usize>,
    start: usize,
}

impl Dialogue {
    /// Create a dialogue from `nodes`, beginning at node index `start`. If
    /// `start` is out of range the conversation begins already ended.
    pub fn new(nodes: Vec<DialogueNode>, start: usize) -> Self {
        let current = if start < nodes.len() {
            Some(start)
        } else {
            None
        };
        Dialogue {
            nodes,
            current,
            start,
        }
    }

    /// The number of nodes in the tree.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Borrow node `i`, or `None` if out of range.
    pub fn node(&self, i: usize) -> Option<&DialogueNode> {
        self.nodes.get(i)
    }

    /// The index of the current node, or `None` if the conversation has ended.
    pub fn current_index(&self) -> Option<usize> {
        self.current
    }

    /// The current node, or `None` if the conversation has ended.
    pub fn current_node(&self) -> Option<&DialogueNode> {
        self.current.and_then(|i| self.nodes.get(i))
    }

    /// The text of the current node, or `None` if the conversation has ended.
    pub fn current_text(&self) -> Option<&str> {
        self.current_node().map(|n| n.text())
    }

    /// The choices available at the current node (empty if ended or terminal).
    pub fn choices(&self) -> &[Choice] {
        match self.current_node() {
            Some(n) => n.choices(),
            None => &[],
        }
    }

    /// The number of choices at the current node.
    pub fn choice_count(&self) -> usize {
        self.choices().len()
    }

    /// `true` while the conversation is on a node (not yet ended).
    pub fn is_active(&self) -> bool {
        self.current.is_some()
    }

    /// `true` once the conversation has ended.
    pub fn is_ended(&self) -> bool {
        self.current.is_none()
    }

    /// `true` if the current node is a terminal node (active but no choices).
    pub fn is_at_terminal(&self) -> bool {
        self.current_node().map(|n| n.is_terminal()).unwrap_or(false)
    }

    /// Take choice `i` at the current node, navigating to its target node.
    /// Returns `true` if a valid choice was followed; returns `false` (without
    /// changing state) if the conversation has ended, `i` is out of range, or
    /// the choice's target is out of range.
    pub fn choose(&mut self, i: usize) -> bool {
        let Some(node) = self.current_node() else {
            return false;
        };
        let Some(choice) = node.choices.get(i) else {
            return false;
        };
        let target = choice.target;
        if target < self.nodes.len() {
            self.current = Some(target);
            true
        } else {
            false
        }
    }

    /// Jump directly to node `node` (for scripted/forced dialogue). Returns
    /// `true` if the index is valid; `false` (no change) otherwise.
    pub fn goto(&mut self, node: usize) -> bool {
        if node < self.nodes.len() {
            self.current = Some(node);
            true
        } else {
            false
        }
    }

    /// End the conversation now (sets the cursor to `None`).
    pub fn end(&mut self) {
        self.current = None;
    }

    /// Restart the conversation at its original start node. If the start index
    /// was invalid the conversation remains ended.
    pub fn reset(&mut self) {
        self.current = if self.start < self.nodes.len() {
            Some(self.start)
        } else {
            None
        };
    }
}

impl DetHash for Choice {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_str(&self.label);
        hasher.write_u32(self.target as u32);
    }
}

impl DetHash for DialogueNode {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_str(&self.text);
        hasher.write_u32(self.choices.len() as u32);
        for c in &self.choices {
            c.det_hash(hasher);
        }
    }
}

impl DetHash for Dialogue {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        // Cursor: a sentinel for "ended" plus the index when active.
        match self.current {
            Some(i) => {
                hasher.write_u8(1);
                hasher.write_u32(i as u32);
            }
            None => hasher.write_u8(0),
        }
        hasher.write_u32(self.start as u32);
        hasher.write_u32(self.nodes.len() as u32);
        for n in &self.nodes {
            n.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    fn sample() -> Dialogue {
        let nodes = vec![
            DialogueNode::new("greeting")
                .with_choice("ask", 1)
                .with_choice("leave", 2),
            DialogueNode::new("info").with_choice("thanks", 2),
            DialogueNode::new("farewell"), // terminal
        ];
        Dialogue::new(nodes, 0)
    }

    #[test]
    fn test_starts_at_start_node() {
        let d = sample();
        assert!(d.is_active());
        assert_eq!(d.current_index(), Some(0));
        assert_eq!(d.current_text(), Some("greeting"));
        assert_eq!(d.choice_count(), 2);
    }

    #[test]
    fn test_choose_navigates() {
        let mut d = sample();
        assert!(d.choose(0)); // ask → node 1
        assert_eq!(d.current_text(), Some("info"));
        assert!(d.choose(0)); // thanks → node 2
        assert_eq!(d.current_text(), Some("farewell"));
    }

    #[test]
    fn test_terminal_node() {
        let mut d = sample();
        d.choose(1); // leave → node 2
        assert!(d.is_at_terminal());
        assert_eq!(d.choice_count(), 0);
        assert!(!d.choose(0), "no choices to take at a terminal node");
    }

    #[test]
    fn test_end_and_ended_state() {
        let mut d = sample();
        d.end();
        assert!(d.is_ended());
        assert!(!d.is_active());
        assert_eq!(d.current_text(), None);
        assert_eq!(d.choices().len(), 0);
        assert!(!d.choose(0), "cannot choose after end");
    }

    #[test]
    fn test_choose_out_of_range_is_rejected() {
        let mut d = sample();
        assert!(!d.choose(99), "out-of-range choice rejected");
        assert_eq!(d.current_index(), Some(0), "state unchanged");
    }

    #[test]
    fn test_choice_with_invalid_target_rejected() {
        let nodes = vec![DialogueNode::new("a").with_choice("bad", 999)];
        let mut d = Dialogue::new(nodes, 0);
        assert!(!d.choose(0), "choice to nonexistent node rejected");
        assert_eq!(d.current_index(), Some(0), "state unchanged");
    }

    #[test]
    fn test_invalid_start_begins_ended() {
        let d = Dialogue::new(vec![DialogueNode::new("a")], 5);
        assert!(d.is_ended());
    }

    #[test]
    fn test_goto() {
        let mut d = sample();
        assert!(d.goto(2));
        assert_eq!(d.current_text(), Some("farewell"));
        assert!(!d.goto(99), "out-of-range goto rejected");
        assert_eq!(d.current_index(), Some(2), "unchanged on bad goto");
    }

    #[test]
    fn test_reset() {
        let mut d = sample();
        d.choose(0);
        d.end();
        d.reset();
        assert_eq!(d.current_index(), Some(0));
        assert_eq!(d.current_text(), Some("greeting"));
    }

    #[test]
    fn test_node_accessors() {
        let d = sample();
        assert_eq!(d.node_count(), 3);
        assert_eq!(d.node(1).unwrap().text(), "info");
        assert!(d.node(2).unwrap().is_terminal());
        assert!(d.node(99).is_none());
        let c = &d.node(0).unwrap().choices()[0];
        assert_eq!(c.label(), "ask");
        assert_eq!(c.target(), 1);
    }

    #[test]
    fn test_det_hash_canonical_and_cursor_sensitive() {
        let a = sample();
        let b = sample();
        assert_eq!(hash_state(&a), hash_state(&b), "same dialogue, same hash");

        let mut c = sample();
        c.choose(0); // cursor moved to node 1
        assert_ne!(hash_state(&a), hash_state(&c), "cursor change → different hash");

        let mut d = sample();
        d.end();
        assert_ne!(hash_state(&a), hash_state(&d), "ended state hashes differently");
    }

    #[test]
    fn test_det_hash_structure_sensitive() {
        let a = sample();
        // Same cursor, different text content.
        let nodes = vec![
            DialogueNode::new("GREETING") // changed
                .with_choice("ask", 1)
                .with_choice("leave", 2),
            DialogueNode::new("info").with_choice("thanks", 2),
            DialogueNode::new("farewell"),
        ];
        let e = Dialogue::new(nodes, 0);
        assert_ne!(hash_state(&a), hash_state(&e), "node text change → different hash");
    }
}
