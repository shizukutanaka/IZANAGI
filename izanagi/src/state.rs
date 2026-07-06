//! Game state stack.
//!
//! A pushdown automaton for things like menu / play / pause. States are
//! whatever you want — usually an enum. The stack keeps history so "pause"
//! can return to "play" by popping.
//!
//! ```
//! use izanagi::state::States;
//!
//! #[derive(Copy, Clone, Debug, PartialEq, Eq)]
//! enum S { Menu, Play, Pause }
//!
//! let mut s = States::new(S::Menu);
//! s.push(S::Play);
//! s.push(S::Pause);
//! assert_eq!(*s.current(), S::Pause);
//! s.pop();
//! assert_eq!(*s.current(), S::Play);
//! ```

/// A stack of states.
pub struct States<S: Clone> {
    stack: Vec<S>,
    transitioned_this_frame: bool,
}

impl<S: Clone> States<S> {
    /// New stack with one initial state.
    pub fn new(initial: S) -> Self {
        Self {
            stack: vec![initial],
            transitioned_this_frame: true,
        }
    }

    /// The current (top) state.
    pub fn current(&self) -> &S {
        // Stack is never empty — `new` enforces it.
        self.stack.last().expect("state stack empty")
    }

    /// Replace the top state.
    pub fn replace(&mut self, s: S) {
        if let Some(top) = self.stack.last_mut() {
            *top = s;
        }
        self.transitioned_this_frame = true;
    }

    /// Push a new state on top.
    pub fn push(&mut self, s: S) {
        self.stack.push(s);
        self.transitioned_this_frame = true;
    }

    /// Pop the top state. Refuses to empty the stack.
    pub fn pop(&mut self) -> bool {
        if self.stack.len() <= 1 {
            return false;
        }
        self.stack.pop();
        self.transitioned_this_frame = true;
        true
    }

    /// Replace the entire stack with a new initial state.
    pub fn reset(&mut self, s: S) {
        self.stack.clear();
        self.stack.push(s);
        self.transitioned_this_frame = true;
    }

    /// True for one frame after any transition. Useful for one-shot setup.
    pub fn just_transitioned(&self) -> bool {
        self.transitioned_this_frame
    }

    /// Stack depth.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Call this at the end of each frame to clear the transition flag.
    pub fn end_frame(&mut self) {
        self.transitioned_this_frame = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum S {
        A,
        B,
        C,
    }

    #[test]
    fn push_pop() {
        let mut s = States::new(S::A);
        s.push(S::B);
        s.push(S::C);
        assert_eq!(*s.current(), S::C);
        assert_eq!(s.depth(), 3);
        assert!(s.pop());
        assert_eq!(*s.current(), S::B);
    }

    #[test]
    fn pop_refuses_to_empty() {
        let mut s = States::new(S::A);
        assert!(!s.pop());
        assert_eq!(*s.current(), S::A);
    }

    #[test]
    fn just_transitioned_flag() {
        let mut s = States::new(S::A);
        assert!(s.just_transitioned());
        s.end_frame();
        assert!(!s.just_transitioned());
        s.push(S::B);
        assert!(s.just_transitioned());
    }

    #[test]
    fn replace_swaps_top() {
        let mut s = States::new(S::A);
        s.push(S::B);
        s.replace(S::C);
        assert_eq!(*s.current(), S::C);
        assert_eq!(s.depth(), 2);
    }

    #[test]
    fn reset_clears_history() {
        let mut s = States::new(S::A);
        s.push(S::B);
        s.push(S::C);
        s.reset(S::A);
        assert_eq!(s.depth(), 1);
        assert_eq!(*s.current(), S::A);
    }
}
