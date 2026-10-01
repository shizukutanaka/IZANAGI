//! Go `pprof` text profile parser.
//!
//! Recognises the legacy text form emitted by `go tool pprof -raw` /
//! `-traces` (heap, goroutine, threadcreate, contention, allocs …) and the
//! folded/collapsed stack form (`func1;func2 N`) consumed by flamegraph
//! tooling, counting sample lines, `#`-comment frames, mapped-library rows
//! and folded stacks.
//!
//! ```
//! let b = concat!(
//!     "heap profile:2: 4096 [4: 8192] @ heap/1048576\n",
//!     "1: 2048 [2: 4096] @ 0x40e3f4 0x41a2dd 0x4503a7\n",
//!     "#  0x40e3f3    runtime.mallocgc+0xfb\n",
//!     "#  0x41a2dc    main.alloc+0x4c\n",
//!     "1: 2048 [2: 4096] @ 0x40e3f4 0x4503a7\n",
//!     "#  0x40e3f3    runtime.mallocgc+0xfb\n",
//!     "#  0x4503a6    main.main+0x12\n"
//! ).as_bytes();
//! assert!(izanagi_kit::pprof::detect(b));
//! let c = izanagi_kit::pprof::Pprof::parse(b).unwrap();
//! assert_eq!(c.samples, 2);
//! assert_eq!(c.frames, 4);
//! ```

/// Parsed pprof text profile summary.
#[derive(Debug, Clone)]
pub struct Pprof {
    /// Profile kind header (`heap profile:` …), empty when folded form.
    pub header: String,
    /// `N: M [A: B] @ 0x…` sample lines.
    pub samples: usize,
    /// `#\t0xADDR sym+off` frame lines.
    pub frames: usize,
    /// Mapped-library rows (`buildID`/`0x…-0x…` address ranges).
    pub mapped: usize,
    /// Folded `func;func;… COUNT` stack lines.
    pub folded: usize,
}

const HEADERS: &[&str] = &[
    "heap profile:",
    "allocs profile:",
    "goroutine profile:",
    "threadcreate profile:",
    "contentions profile:",
    "contention profile:",
    "block profile:",
    "mutex profile:",
    "profile:",
    "samples profile:",
];

fn is_sample(tr: &str) -> bool {
    // `1: 2048 [2: 4096] @ 0x…`
    let Some(c1) = tr.find(": ") else {
        return false;
    };
    if tr[..c1].chars().any(|c| !c.is_ascii_digit()) {
        return false;
    }
    let rest = &tr[c1 + 2..];
    rest.chars().next().is_some_and(|c| c.is_ascii_digit())
        && rest.contains('[')
        && rest.contains(']')
        && rest.contains('@')
}

fn is_frame(tr: &str) -> bool {
    tr.starts_with('#') && tr.contains("0x")
}

fn is_folded(tr: &str) -> bool {
    // `main;work;parse 42` — semicolon-separated frames + trailing count.
    if tr.starts_with('#') || tr.starts_with("0x") || !tr.contains(';') {
        return false;
    }
    let Some(sp) = tr.rfind(' ') else {
        return false;
    };
    let tail = &tr[sp + 1..];
    !tail.is_empty()
        && tail.chars().all(|c| c.is_ascii_digit())
        && tr[..sp].chars().all(|c| c != ' ' && c != '\t' || c == ';')
}

fn is_mapped(tr: &str) -> bool {
    // `0x400000-0x4a0000 /bin/app` or `buildID:` rows.
    (tr.starts_with("0x") && tr.contains('-') && tr.contains('/'))
        || tr.starts_with("buildID:")
        || tr.starts_with("MAPPED_LIBRARIES:")
}

/// Whether the buffer looks like a pprof text profile.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    if b.starts_with(b"\x1f\x8b") {
        return false;
    }
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let mut samples = 0usize;
    let mut folded = 0usize;
    for l in t.lines().take(8192) {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if HEADERS.iter().any(|h| tr.starts_with(h)) {
            return true;
        }
        if is_sample(tr) {
            samples += 1;
        }
        if is_folded(tr) {
            folded += 1;
        }
    }
    samples >= 1 || folded >= 2
}

impl Pprof {
    /// Parses a pprof text profile summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            header: String::new(),
            samples: 0,
            frames: 0,
            mapped: 0,
            folded: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if c.header.is_empty() && HEADERS.iter().any(|h| tr.starts_with(h)) {
                c.header = format!("{}:", tr.split(':').next().unwrap_or(tr));
                continue;
            }
            if is_sample(tr) {
                c.samples += 1;
            } else if is_frame(tr) {
                c.frames += 1;
            } else if is_mapped(tr) {
                c.mapped += 1;
            } else if is_folded(tr) {
                c.folded += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_heap_profile() {
        let b = br#"heap profile:2: 4096 [4: 8192] @ heap/1048576
1: 2048 [2: 4096] @ 0x40e3f4 0x41a2dd 0x4503a7
#  0x40e3f3    runtime.mallocgc+0xfb
#  0x41a2dc    main.alloc+0x4c
1: 2048 [2: 4096] @ 0x40e3f4 0x4503a7
#  0x40e3f3    runtime.mallocgc+0xfb
#  0x4503a6    main.main+0x12

MAPPED_LIBRARIES:
"#;
        assert!(detect(b));
        let c = Pprof::parse(b).unwrap();
        assert_eq!(c.header, "heap profile:");
        assert_eq!(c.samples, 2);
        assert_eq!(c.frames, 4);
        assert_eq!(c.mapped, 1);
        assert_eq!(c.folded, 0);
    }

    #[test]
    fn detects_folded() {
        let b = br#"main;work;parse 42
main;work;render 17
main;idle 3
"#;
        assert!(detect(b));
        let c = Pprof::parse(b).unwrap();
        assert_eq!(c.folded, 3);
        assert_eq!(c.samples, 0);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"func main() {\n}\n"));
        assert!(!detect(b"{\"traceEvents\": []}"));
        assert!(Pprof::parse(b"not a profile").is_none());
    }
}
