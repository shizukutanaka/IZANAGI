//! Asset loading.
//!
//! In-memory store with optional file-system backing. No global state.

use std::collections::HashMap;
use std::path::PathBuf;

/// Simple asset handle.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Handle(u64);

/// Asset loader and cache.
pub struct Assets {
    root: PathBuf,
    bytes: HashMap<Handle, Vec<u8>>,
    names: HashMap<String, Handle>,
    next: u64,
}

impl Assets {
    /// New loader rooted at the current working directory.
    pub fn new() -> Self {
        Self {
            root: PathBuf::from("."),
            bytes: HashMap::new(),
            names: HashMap::new(),
            next: 1,
        }
    }

    /// Set the directory assets are resolved relative to.
    pub fn set_root(&mut self, root: impl Into<PathBuf>) {
        self.root = root.into();
    }

    /// Load bytes from a file, caching the result.
    ///
    /// Returns `None` if the file cannot be read.
    pub fn load(&mut self, name: &str) -> Option<Handle> {
        if let Some(&h) = self.names.get(name) {
            return Some(h);
        }
        let path = self.root.join(name);
        let bytes = std::fs::read(&path).ok()?;
        let h = Handle(self.next);
        self.next += 1;
        self.bytes.insert(h, bytes);
        self.names.insert(name.to_string(), h);
        Some(h)
    }

    /// Insert raw bytes under a name. Useful for tests and embedded assets.
    pub fn insert(&mut self, name: &str, bytes: Vec<u8>) -> Handle {
        let h = Handle(self.next);
        self.next += 1;
        self.bytes.insert(h, bytes);
        self.names.insert(name.to_string(), h);
        h
    }

    /// Borrow cached bytes for a handle.
    pub fn get(&self, h: Handle) -> Option<&[u8]> {
        self.bytes.get(&h).map(|v| v.as_slice())
    }

    /// Number of cached assets.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// True if nothing is cached.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl Default for Assets {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let mut a = Assets::new();
        let h = a.insert("sword.png", vec![1, 2, 3]);
        assert_eq!(a.get(h), Some(&[1, 2, 3][..]));
    }

    #[test]
    fn load_missing_returns_none() {
        let mut a = Assets::new();
        assert!(a.load("nonexistent-file-zzz.bin").is_none());
    }

    #[test]
    fn duplicate_insert_overwrites_name_mapping() {
        let mut a = Assets::new();
        let h1 = a.insert("x", vec![1]);
        let h2 = a.insert("x", vec![2]);
        assert_ne!(h1, h2);
        assert_eq!(a.len(), 2);
    }
}
