//! Excalidraw JSON scene format (`.excalidraw`).
//!
//! Excalidraw scenes are JSON documents with `"type":"excalidraw"`,
//! `"version"`, `"elements"` array whose items carry `"type"` of
//! rectangle/ellipse/diamond/line/arrow/text/image/frame/freedraw, plus
//! `"appState"` and `"files"` sections.
//!
//! ```
//! let b = br#"{"type":"excalidraw","version":2,"elements":[{"id":"a","type":"rectangle","x":0,"y":0},{"id":"b","type":"arrow"},{"id":"c","type":"text","text":"hi"}],"appState":{},"files":{}}"#;
//! assert!(izanagi_kit::excalidraw::detect(b));
//! let c = izanagi_kit::excalidraw::Excalidraw::parse(b).unwrap();
//! assert_eq!(c.elements, 3);
//! ```

/// Parsed Excalidraw scene summary.
#[derive(Debug, Clone)]
pub struct Excalidraw {
    /// Element count (`"type":"..."` inside `elements` array).
    pub elements: usize,
    /// Distinct element types encountered.
    pub element_types: usize,
    /// `appState` key count.
    pub app_state_keys: usize,
    /// `files` entry count.
    pub files: usize,
    /// `isDeleted` true count.
    pub deleted: usize,
    /// `boundElements` / `containerId` link occurrences.
    pub links: usize,
}

const ELEMENT_TYPES: &[&str] = &[
    "rectangle",
    "ellipse",
    "diamond",
    "line",
    "arrow",
    "text",
    "image",
    "frame",
    "freedraw",
    "embeddable",
    "iframe",
    "magicframe",
];

fn count(t: &str, pat: &str) -> usize {
    if pat.is_empty() {
        return 0;
    }
    let mut n = 0;
    let mut off = 0;
    while off + pat.len() <= t.len() {
        match t[off..].find(pat) {
            Some(p) => {
                n += 1;
                off += p + pat.len();
            }
            None => break,
        }
    }
    n
}

fn bracket_seg<'a>(t: &'a str, start_key: &str) -> Option<&'a str> {
    let s = t.find(start_key)? + start_key.len();
    let s = &t[s..];
    let open = s.find('[')?;
    let mut depth = 0usize;
    let mut end = open;
    for (i, ch) in s.char_indices().skip(open) {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    Some(&s[open + 1..end])
}

/// Whether the buffer looks like an Excalidraw JSON scene.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains(r#""type":"excalidraw""#)
        || (t.contains(r#""version""#)
            && t.contains(r#""elements""#)
            && ELEMENT_TYPES
                .iter()
                .any(|e| t.contains(&format!(r#""type":"{e}""#))))
}

impl Excalidraw {
    /// Parses an Excalidraw scene summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            elements: 0,
            element_types: 0,
            app_state_keys: 0,
            files: 0,
            deleted: 0,
            links: 0,
        };
        let elements_seg = bracket_seg(t, r#""elements":"#).unwrap_or(t);
        // `boundElements` arrays also carry `"type":` references — subtract them.
        let mut off = 0usize;
        let mut bound_total = 0usize;
        while let Some(p) = elements_seg[off..].find(r#""boundElements":["#) {
            let open = off + p + 15;
            let mut depth = 0usize;
            let mut close = elements_seg.len();
            for (i, ch) in elements_seg.char_indices().skip(open) {
                match ch {
                    '[' => depth += 1,
                    ']' => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            close = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            for e in ELEMENT_TYPES {
                bound_total += count(&elements_seg[open..close], &format!(r#""type":"{e}""#));
            }
            off = close;
        }
        for e in ELEMENT_TYPES {
            let pat = format!(r#""type":"{e}""#);
            let n = count(elements_seg, &pat);
            if n > 0 {
                c.element_types += 1;
                c.elements += n;
            }
        }
        c.elements = c.elements.saturating_sub(bound_total);
        c.deleted = count(elements_seg, r#""isDeleted":true"#);
        c.links =
            count(elements_seg, r#""boundElements""#) + count(elements_seg, r#""containerId""#);
        if let Some(p) = t.find(r#""appState":"#) {
            let seg = &t[p..];
            let keys_end = seg.find('}').unwrap_or(seg.len());
            let inner = &seg[seg.find('{').unwrap_or(0)..keys_end];
            c.app_state_keys = count(inner, "\":");
        }
        if let Some(p) = t.find(r#""files":"#) {
            let seg = &t[p..];
            let end = seg.find('}').unwrap_or(seg.len());
            c.files = count(&seg[..end], r#""id":""#);
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scene() {
        let b = br##"{"type":"excalidraw","version":2,"elements":[{"id":"a","type":"rectangle","x":0,"y":0},{"id":"b","type":"arrow","boundElements":[{"type":"text"}]},{"id":"c","type":"text","text":"hi"},{"id":"d","type":"rectangle","isDeleted":true}],"appState":{"viewBackgroundColor":"#fff","gridSize":20},"files":{"f1":{"id":"f1","mimeType":"image/png"}}}"##;
        assert!(detect(b));
        let c = Excalidraw::parse(b).unwrap();
        assert_eq!(c.elements, 4);
        assert_eq!(c.element_types, 3);
        assert_eq!(c.deleted, 1);
        assert_eq!(c.app_state_keys, 2);
        assert_eq!(c.files, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(br#"{"a":1}"#));
        assert!(Excalidraw::parse(b"x").is_none());
    }
}
