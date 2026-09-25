//! Semantic Versioning 2.0.0 (semver.org) — parse, compare, and
//! range-match version strings. Mod/save/tooling protocols name
//! semver as their compatibility currency, and the ordering rules
//! (numeric < alphanumeric, longer list > prefix-equal list, build
//! metadata ignored) are exactly the kind that silent half-parsers
//! get wrong — the spec's own 11-version chain is pinned below.
//!
//! ```
//! use izanagi_kit::semver::{satisfies_caret, SemVer};
//!
//! let v = SemVer::parse(b"1.4.2-alpha.1+build.7").unwrap();
//! assert_eq!((v.major, v.minor, v.patch), (1, 4, 2));
//! assert!(v < SemVer::parse(b"1.4.2").unwrap());
//! let rel = SemVer::parse(b"1.4.9").unwrap();
//! assert!(satisfies_caret(&rel, &SemVer::parse(b"1.0.0").unwrap()));
//! ```

/// A pre-release identifier: numeric ids sort before alphanumeric
/// ones and numerically among themselves (SemVer §11.4.2–3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pre {
    /// All-digit identifier, e.g. `alpha.1`'s `1`.
    Num(u64),
    /// Alphanumeric identifier (may contain digits, sorts ASCII).
    Str(Vec<u8>),
}

impl Ord for Pre {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        use core::cmp::Ordering::*;
        match (self, other) {
            (Pre::Num(a), Pre::Num(b)) => a.cmp(b),
            (Pre::Num(_), Pre::Str(_)) => Less,
            (Pre::Str(_), Pre::Num(_)) => Greater,
            (Pre::Str(a), Pre::Str(b)) => a.cmp(b),
        }
    }
}
impl PartialOrd for Pre {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// `MAJOR.MINOR.PATCH[-prerelease][+build]`. `build` is retained for
/// round-tripping but ignored by `Ord`, per spec §10.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemVer {
    /// Incompatible-API counter.
    pub major: u64,
    /// Backward-compatible feature counter.
    pub minor: u64,
    /// Backward-compatible fix counter.
    pub patch: u64,
    /// Dot-separated pre-release identifiers (empty = release).
    pub pre: Vec<Pre>,
    /// Dot-separated build-metadata identifiers (ordering-ignored).
    pub build: Vec<Vec<u8>>,
}

fn digits(s: &[u8]) -> Option<u64> {
    if s.is_empty() || (s.len() > 1 && s[0] == b'0') {
        return None; // spec: no leading zeros
    }
    let mut n = 0u64;
    for &c in s {
        if !c.is_ascii_digit() {
            return None;
        }
        n = n.checked_mul(10)?.checked_add((c - b'0') as u64)?;
    }
    Some(n)
}

fn is_ident_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'-'
}

fn parse_ident_list(s: &[u8], numeric_rules: bool) -> Option<Vec<Pre>> {
    let mut out = Vec::new();
    for part in s.split(|&c| c == b'.') {
        if part.is_empty() || !part.iter().all(|&c| is_ident_char(c)) {
            return None;
        }
        if numeric_rules && part.iter().all(|&c| c.is_ascii_digit()) {
            out.push(Pre::Num(digits(part)?));
        } else {
            out.push(Pre::Str(part.to_vec()));
        }
    }
    Some(out)
}

impl SemVer {
    /// Strict SemVer 2.0.0 parse. Returns `None` on any spec
    /// violation (missing component, empty identifier, leading zero
    /// in a numeric field, bad charset).
    pub fn parse(s: &[u8]) -> Option<SemVer> {
        let (core_pre, build_part) = match s.iter().position(|&c| c == b'+') {
            Some(i) => (&s[..i], Some(&s[i + 1..])),
            None => (s, None),
        };
        let (core, pre_part) = match core_pre.iter().position(|&c| c == b'-') {
            Some(i) => (&core_pre[..i], Some(&core_pre[i + 1..])),
            None => (core_pre, None),
        };
        let mut it = core.split(|&c| c == b'.');
        let major = digits(it.next()?)?;
        let minor = digits(it.next()?)?;
        let patch = digits(it.next()?)?;
        if it.next().is_some() {
            return None;
        }
        let pre = match pre_part {
            Some(p) => parse_ident_list(p, true)?,
            None => Vec::new(),
        };
        let mut build = Vec::new();
        if let Some(b) = build_part {
            for part in b.split(|&c| c == b'.') {
                // Build ids get the same charset/nonempty checks but
                // may carry leading zeros.
                if part.is_empty() || !part.iter().all(|&c| is_ident_char(c)) {
                    return None;
                }
                build.push(part.to_vec());
            }
        }
        Some(SemVer {
            major,
            minor,
            patch,
            pre,
            build,
        })
    }

    /// `true` when any pre-release identifiers are present.
    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        use core::cmp::Ordering::*;
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Equal,
                (true, false) => Greater, // release > prerelease
                (false, true) => Less,
                (false, false) => self.pre.cmp(&other.pre),
            })
    }
}
impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// npm caret range `^req`: the highest release compatible with
/// `req`'s declared compatibility — `[req, next-breaking)` where
/// next-breaking is major+1 (or minor+1 / patch+1 for 0.x left-pad
/// rules).
pub fn satisfies_caret(v: &SemVer, req: &SemVer) -> bool {
    if req.is_prerelease() {
        return *v == *req; // caret on prerelease pins exact
    }
    if v.is_prerelease() || v < req {
        return false;
    }
    if req.major > 0 {
        v.major == req.major
    } else if req.minor > 0 {
        v.major == 0 && v.minor == req.minor
    } else {
        *v == *req
    }
}

/// npm tilde range `~req`: `[req, minor+1)` — patch-level changes.
pub fn satisfies_tilde(v: &SemVer, req: &SemVer) -> bool {
    if req.is_prerelease() {
        return *v == *req;
    }
    !v.is_prerelease() && *v >= *req && v.major == req.major && v.minor == req.minor
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cmp::Ordering::*;

    fn p(s: &str) -> SemVer {
        SemVer::parse(s.as_bytes()).unwrap()
    }

    #[test]
    fn parse_roundtrip() {
        let v = p("12.3.4-rc.2+ci.99");
        assert_eq!((v.major, v.minor, v.patch), (12, 3, 4));
        assert_eq!(v.pre, vec![Pre::Str(b"rc".to_vec()), Pre::Num(2)]);
        assert_eq!(v.build, vec![b"ci".to_vec(), b"99".to_vec()]);
        assert!(v.is_prerelease());
        assert!(!p("1.0.0").is_prerelease());
    }

    #[test]
    fn parse_rejects_spec_violations() {
        for s in [
            "",
            "1",
            "1.2",
            "1.2.3.4",
            "1.02.3",
            "01.2.3",
            "1.2.x",
            "v1.2.3",
            "1.2.3-",
            "1.2.3-a..b",
            "1.2.3-a_b",
            "1.2.3+",
            "1.2.3+a..b",
            "1.2.3-01",
            "1.2.3-é",
        ] {
            assert!(SemVer::parse(s.as_bytes()).is_none(), "{s}");
        }
        for s in ["0.0.0", "1.2.3-0", "1.2.3-a.b.c+1.2.3", "1.2.3+build"] {
            assert!(SemVer::parse(s.as_bytes()).is_some(), "{s}");
        }
    }

    #[test]
    fn spec_ordering_chain() {
        // The semver.org §11 example chain.
        let chain = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];
        for w in chain.windows(2) {
            assert_eq!(p(w[0]).cmp(&p(w[1])), Less, "{} < {}", w[0], w[1]);
            assert_eq!(p(w[1]).cmp(&p(w[0])), Greater);
        }
        // Numeric < alphanumeric; build metadata ignored.
        assert_eq!(p("1.0.0-2").cmp(&p("1.0.0-a")), Less);
        assert_eq!(p("1.2.3+aaa").cmp(&p("1.2.3+zzz")), Equal);
        assert_ne!(p("1.2.3+aaa"), p("1.2.3+zzz")); // build differs → not Eq
        assert_eq!(p("1.2.3").cmp(&p("1.2.3+zzz")), Equal);
    }

    #[test]
    fn caret_and_tilde() {
        let v = |s: &str| p(s);
        assert!(satisfies_caret(&v("1.4.9"), &v("1.2.0")));
        assert!(!satisfies_caret(&v("2.0.0"), &v("1.2.0")));
        assert!(!satisfies_caret(&v("1.1.9"), &v("1.2.0")));
        // 0.x rules: ^0.2.x pins minor; ^0.0.x pins exact.
        assert!(satisfies_caret(&v("0.2.9"), &v("0.2.1")));
        assert!(!satisfies_caret(&v("0.3.0"), &v("0.2.1")));
        assert!(satisfies_caret(&v("0.0.3"), &v("0.0.3")));
        assert!(!satisfies_caret(&v("0.0.4"), &v("0.0.3")));
        // Prereleases don't satisfy release ranges.
        assert!(!satisfies_caret(&v("1.5.0-rc.1"), &v("1.2.0")));
        assert!(satisfies_tilde(&v("1.2.9"), &v("1.2.0")));
        assert!(!satisfies_tilde(&v("1.3.0"), &v("1.2.0")));
        assert!(satisfies_tilde(&v("2.2.0"), &v("2.2.0")));
    }

    #[test]
    fn deterministic_twice() {
        assert_eq!(SemVer::parse(b"9.9.9-x.1+y"), SemVer::parse(b"9.9.9-x.1+y"));
    }
}
