//! Key-to-action mapping — deterministic input translation.
//!
//! Raw key events (characters, arrow codes) are non-deterministic OS output;
//! game actions ("move north", "wait", "open inventory") are deterministic
//! sim inputs. `KeyMap<K, A>` bridges the two: it translates a key `K` into
//! an action `A` via a lookup table, returning `None` for unmapped keys.
//!
//! Keys are generic (`K: Eq + Clone`) so callers can use `char`, a backend-
//! specific keycode enum, or any other type. Actions are generic (`A: Clone`)
//! so no domain logic lives here.
//!
//! The map is intentionally simple: one key → zero or one action. Chords and
//! sequences belong in a dedicated input FSM; this is just the single-key
//! lookup layer. Because it is a pure function (no state), it is replay-safe
//! by construction — mapping the same key twice yields the same action.

/// A configurable mapping from keys of type `K` to actions of type `A`.
#[derive(Clone, Debug, Default)]
pub struct KeyMap<K, A> {
    bindings: Vec<(K, A)>,
}

impl<K: Eq + Clone, A: Clone> KeyMap<K, A> {
    pub fn new() -> Self {
        KeyMap {
            bindings: Vec::new(),
        }
    }

    /// Bind `key` to `action`. If `key` is already bound, the old binding is
    /// replaced (last-write wins so callers can override defaults).
    pub fn bind(&mut self, key: K, action: A) {
        if let Some((_, a)) = self.bindings.iter_mut().find(|(k, _)| *k == key) {
            *a = action;
        } else {
            self.bindings.push((key, action));
        }
    }

    /// Remove the binding for `key`. No-op if absent.
    pub fn unbind(&mut self, key: &K) {
        self.bindings.retain(|(k, _)| k != key);
    }

    /// Translate `key` to its action, or `None` if not bound.
    pub fn get(&self, key: &K) -> Option<&A> {
        self.bindings.iter().find(|(k, _)| k == key).map(|(_, a)| a)
    }

    /// Whether `key` has a binding.
    pub fn is_bound(&self, key: &K) -> bool {
        self.bindings.iter().any(|(k, _)| k == key)
    }

    /// Number of active bindings.
    #[inline]
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Translate a slice of raw key events into a `Vec` of actions, discarding
    /// unmapped keys. This is the typical per-tick call site: drain the OS
    /// event buffer, map it, then push the results into a [`CmdQueue`](crate::CmdQueue).
    pub fn translate_all(&self, keys: &[K]) -> Vec<A> {
        keys.iter().filter_map(|k| self.get(k).cloned()).collect()
    }

    /// Remove all bindings. Useful for resetting to a clean state before
    /// rebuilding a key layout at runtime.
    pub fn clear(&mut self) {
        self.bindings.clear();
    }

    /// Iterate all bindings in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &A)> {
        self.bindings.iter().map(|(k, a)| (k, a))
    }
}

impl<K: Eq + Clone, A: Eq + Clone> KeyMap<K, A> {
    /// `true` if at least one key is currently bound to `action`.
    /// Allocation-free — does not collect; use `get_keys_for_action` when
    /// you also need the key list.
    pub fn contains_action(&self, action: &A) -> bool {
        self.bindings.iter().any(|(_, a)| a == action)
    }

    /// Bind every key in `keys` to `action` in a single call.
    /// Existing bindings for any key in `keys` are replaced.
    pub fn bind_multiple(&mut self, keys: &[K], action: A)
    where
        A: Clone,
    {
        for key in keys {
            self.bind(key.clone(), action.clone());
        }
    }

    /// All keys currently bound to `action`, in insertion order.
    /// Returns an empty `Vec` if none are bound.
    pub fn get_keys_for_action(&self, action: &A) -> Vec<&K> {
        self.bindings
            .iter()
            .filter_map(|(k, a)| if a == action { Some(k) } else { None })
            .collect()
    }

    /// Atomically exchange the actions at `key1` and `key2`.
    ///
    /// If only one key is bound the bound action moves to the unbound key and
    /// the formerly-bound key becomes unbound. If neither is bound this is a
    /// no-op.
    pub fn swap_bindings(&mut self, key1: &K, key2: &K) {
        let a1 = self
            .bindings
            .iter()
            .find(|(k, _)| k == key1)
            .map(|(_, a)| a.clone());
        let a2 = self
            .bindings
            .iter()
            .find(|(k, _)| k == key2)
            .map(|(_, a)| a.clone());
        // Remove both first (order matters for uniqueness).
        self.bindings.retain(|(k, _)| k != key1 && k != key2);
        if let Some(a) = a2 {
            self.bindings.push((key1.clone(), a));
        }
        if let Some(a) = a1 {
            self.bindings.push((key2.clone(), a));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Simple roguelike action enum for testing.
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Action {
        MoveNorth,
        MoveSouth,
        MoveEast,
        MoveWest,
        Wait,
        Quit,
    }

    fn default_map() -> KeyMap<char, Action> {
        let mut m = KeyMap::new();
        m.bind('k', Action::MoveNorth);
        m.bind('j', Action::MoveSouth);
        m.bind('l', Action::MoveEast);
        m.bind('h', Action::MoveWest);
        m.bind('.', Action::Wait);
        m.bind('q', Action::Quit);
        m
    }

    #[test]
    fn test_get_returns_correct_action() {
        let m = default_map();
        assert_eq!(m.get(&'k'), Some(&Action::MoveNorth));
        assert_eq!(m.get(&'j'), Some(&Action::MoveSouth));
    }

    #[test]
    fn test_get_unbound_key_returns_none() {
        let m = default_map();
        assert_eq!(m.get(&'x'), None);
    }

    #[test]
    fn test_bind_replaces_existing() {
        let mut m: KeyMap<char, Action> = KeyMap::new();
        m.bind('k', Action::MoveNorth);
        m.bind('k', Action::MoveEast); // rebind
        assert_eq!(m.get(&'k'), Some(&Action::MoveEast));
        assert_eq!(m.len(), 1); // still one entry
    }

    #[test]
    fn test_unbind_removes_key() {
        let mut m = default_map();
        m.unbind(&'q');
        assert!(!m.is_bound(&'q'));
        assert_eq!(m.get(&'q'), None);
    }

    #[test]
    fn test_unbind_absent_is_noop() {
        let mut m = default_map();
        let before = m.len();
        m.unbind(&'z');
        assert_eq!(m.len(), before);
    }

    #[test]
    fn test_is_bound() {
        let m = default_map();
        assert!(m.is_bound(&'k'));
        assert!(!m.is_bound(&'z'));
    }

    #[test]
    fn test_translate_all_maps_and_discards_unmapped() {
        let m = default_map();
        let keys = vec!['k', 'z', 'j', 'x']; // 'z' and 'x' unmapped
        let actions = m.translate_all(&keys);
        assert_eq!(actions, [Action::MoveNorth, Action::MoveSouth]);
    }

    #[test]
    fn test_translate_all_empty_input() {
        let m = default_map();
        assert!(m.translate_all(&[]).is_empty());
    }

    #[test]
    fn test_translate_all_no_matches() {
        let m = default_map();
        let actions = m.translate_all(&['x', 'y', 'z']);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_translate_is_deterministic() {
        let m = default_map();
        let keys = vec!['k', 'j', 'l', 'h', '.'];
        assert_eq!(m.translate_all(&keys), m.translate_all(&keys));
    }

    #[test]
    fn test_len_and_is_empty() {
        let mut m: KeyMap<char, Action> = KeyMap::new();
        assert!(m.is_empty());
        m.bind('k', Action::MoveNorth);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn test_iter_returns_all_bindings() {
        let m = default_map();
        let count = m.iter().count();
        assert_eq!(count, m.len());
    }

    #[test]
    fn test_new_is_default_empty() {
        let m: KeyMap<char, Action> = KeyMap::new();
        assert!(m.is_empty());
    }

    #[test]
    fn test_clear_removes_all_bindings() {
        let mut m = default_map();
        m.clear();
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
        assert_eq!(m.get(&'k'), None);
    }

    #[test]
    fn test_clear_then_rebind() {
        let mut m = default_map();
        m.clear();
        m.bind('x', Action::Quit);
        assert_eq!(m.len(), 1);
        assert_eq!(m.get(&'x'), Some(&Action::Quit));
    }

    #[test]
    fn test_get_keys_for_action_finds_bound_keys() {
        let m = default_map();
        let keys = m.get_keys_for_action(&Action::MoveNorth);
        assert_eq!(keys, vec![&'k']);
    }

    #[test]
    fn test_get_keys_for_action_multiple_keys_same_action() {
        let mut m: KeyMap<char, Action> = KeyMap::new();
        m.bind('k', Action::MoveNorth);
        m.bind('w', Action::MoveNorth); // second binding for same action
        let mut keys = m.get_keys_for_action(&Action::MoveNorth);
        keys.sort();
        assert_eq!(keys, vec![&'k', &'w']);
    }

    #[test]
    fn test_get_keys_for_action_unbound_returns_empty() {
        let m = default_map();
        assert!(!m.get_keys_for_action(&Action::MoveNorth).is_empty());
        let empty: KeyMap<char, Action> = KeyMap::new();
        assert!(empty.get_keys_for_action(&Action::Quit).is_empty());
    }

    #[test]
    fn test_swap_bindings_exchanges_actions() {
        let mut m = default_map();
        m.swap_bindings(&'k', &'j'); // swap MoveNorth ↔ MoveSouth
        assert_eq!(m.get(&'k'), Some(&Action::MoveSouth));
        assert_eq!(m.get(&'j'), Some(&Action::MoveNorth));
    }

    #[test]
    fn test_swap_bindings_one_unbound_moves_action() {
        let mut m: KeyMap<char, Action> = KeyMap::new();
        m.bind('a', Action::Wait);
        m.swap_bindings(&'a', &'b'); // 'b' is unbound
        assert_eq!(m.get(&'a'), None);
        assert_eq!(m.get(&'b'), Some(&Action::Wait));
    }

    #[test]
    fn test_swap_bindings_both_unbound_is_noop() {
        let mut m: KeyMap<char, Action> = KeyMap::new();
        m.swap_bindings(&'x', &'y'); // both unbound — no panic
        assert!(m.is_empty());
    }

    #[test]
    fn test_contains_action_bound() {
        let m = default_map();
        assert!(m.contains_action(&Action::MoveNorth));
        assert!(m.contains_action(&Action::Quit));
    }

    #[test]
    fn test_contains_action_unbound() {
        let m: KeyMap<char, Action> = KeyMap::new();
        assert!(!m.contains_action(&Action::MoveNorth));
    }

    #[test]
    fn test_bind_multiple_binds_all_keys() {
        let mut m: KeyMap<char, Action> = KeyMap::new();
        m.bind_multiple(&['w', 'k', '8'], Action::MoveNorth);
        assert_eq!(m.get(&'w'), Some(&Action::MoveNorth));
        assert_eq!(m.get(&'k'), Some(&Action::MoveNorth));
        assert_eq!(m.get(&'8'), Some(&Action::MoveNorth));
    }

    #[test]
    fn test_bind_multiple_replaces_existing() {
        let mut m: KeyMap<char, Action> = KeyMap::new();
        m.bind('k', Action::MoveEast);
        m.bind_multiple(&['k'], Action::MoveNorth);
        assert_eq!(m.get(&'k'), Some(&Action::MoveNorth));
        assert_eq!(m.len(), 1); // still one entry (replaced)
    }
}
