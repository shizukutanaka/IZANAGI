//! Census of a seccomp-bpf profile (Docker/Podman JSON).
//!
//! `{"defaultAction":"SCMP_ACT_ALLOW","architectures":[…],
//! "archMap":[…],"syscalls":[{"names":[…],"action":"SCMP_ACT_ALLOW",
//! "args":[…],"includes":{…},"excludes":{…}}]}`. Counts syscall rule
//! blocks, syscall names, action strings, architectures and `args`
//! conditions.
//!
//! ```rust
//! let c = izanagi_kit::seccomp::Seccomp::parse(
//!     b"{\"defaultAction\":\"SCMP_ACT_ERRNO\",\"architectures\":[\"SCMP_ARCH_X86_64\"],\
//!       \"syscalls\":[{\"names\":[\"open\",\"close\"],\"action\":\"SCMP_ACT_ALLOW\"}]}",
//! ).unwrap();
//! assert_eq!(c.syscall_rules, 1);
//! assert_eq!(c.syscall_names, 2);
//! ```
#![forbid(unsafe_code)]

use crate::textutil::strip_bom;
/// seccomp profile census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seccomp {
    /// `{"names":[…],…}` rule objects inside `"syscalls"`.
    pub syscall_rules: usize,
    /// Total quoted names inside `"names"` arrays.
    pub syscall_names: usize,
    /// `SCMP_ACT_*` action strings.
    pub actions: usize,
    /// `SCMP_ARCH_*` architecture strings.
    pub arches: usize,
    /// `args` condition entries.
    pub args: usize,
}

/// True if `b` looks like a seccomp profile.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    t.trim_start().starts_with('{')
        && (t.contains("SCMP_ACT_") || t.contains("SCMP_ARCH_"))
        && t.contains("\"syscalls\"")
}

/// Count `"name"` occurrences (quoted `"…"` items) inside a `[…]` slice.
fn count_names(arr: &str) -> usize {
    arr.matches('"').count() / 2
}

impl Seccomp {
    /// Parse a seccomp profile into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        if !t.trim_start().starts_with('{') {
            return None;
        }
        let actions = t.matches("SCMP_ACT_").count();
        let arches = t.matches("SCMP_ARCH_").count();
        let args = t.matches("\"args\"").count();
        let mut syscall_rules = 0usize;
        let mut syscall_names = 0usize;
        // Each syscall rule is `{ "names": [ ... ], ... }` inside the
        // `"syscalls"` array. Find `"names"` keys and their `[` arrays.
        let mut rest = t;
        while let Some(p) = rest.find("\"names\"") {
            rest = &rest[p + 7..];
            let Some(open) = rest.find('[') else {
                break;
            };
            let Some(close) = rest[open..].find(']') else {
                break;
            };
            syscall_rules += 1;
            syscall_names += count_names(&rest[open + 1..open + close]);
            rest = &rest[open + close..];
        }
        if actions == 0 || syscall_rules == 0 {
            return None;
        }
        Some(Self {
            syscall_rules,
            syscall_names,
            actions,
            arches,
            args,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> &'static str {
        concat!(
            "{\"defaultAction\":\"SCMP_ACT_ERRNO\",",
            "\"architectures\":[\"SCMP_ARCH_X86_64\",\"SCMP_ARCH_AARCH64\"],",
            "\"syscalls\":[",
            "{\"names\":[\"open\",\"openat\",\"close\"],\"action\":\"SCMP_ACT_ALLOW\"},",
            "{\"names\":[\"ptrace\"],\"action\":\"SCMP_ACT_ERRNO\",\"args\":[{\"index\":0}]}]}",
        )
    }

    #[test]
    fn detects_and_parses() {
        let b = sample();
        assert!(detect(b.as_bytes()));
        let c = Seccomp::parse(b.as_bytes()).unwrap();
        assert_eq!(c.syscall_rules, 2);
        assert_eq!(c.syscall_names, 4);
        assert_eq!(c.actions, 3);
        assert_eq!(c.arches, 2);
        assert_eq!(c.args, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"{\"rules\":[]}"));
        assert!(Seccomp::parse(b"[]").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
