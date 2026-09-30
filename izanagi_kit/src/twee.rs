//! Twee 3 interactive-fiction source parser (Twine/Tweego).
//!
//! Passages are `:: Name [tags] {json}` headers; story metadata lives in
//! `:: StoryTitle`, `:: StoryData`, `:: StoryStylesheet`, `:: StoryScript`.
//! Body text supports `[[links]]`, macros `<<...>>` and comments `/%...%/`?
//! (comments are `/% ... %/` — counted as `%/` lines too).
//!
//! ```
//! use izanagi_kit::twee::Twee;
//! let src = b":: StoryTitle\nDemo\n\n:: StoryData\n{\n\"ifid\": \"ABC\"\n}\n\n:: Start [start-tag]\nHello [[Next]].\n\n:: Next\nEnd.\n";
//! assert!(izanagi_kit::twee::detect(src));
//! let t = Twee::parse(src).unwrap();
//! assert_eq!(t.title, "Demo");
//! assert_eq!(t.passages, 4);
//! assert_eq!(t.links, 1);
//! ```

/// Parsed census of a Twee source file.
#[derive(Debug, Clone)]
pub struct Twee {
    /// `:: StoryTitle` passage body.
    pub title: String,
    /// `:: StoryData` present.
    pub story_data: bool,
    /// `:: StoryStylesheet` / `:: StoryScript` counts.
    pub story_blocks: usize,
    /// All `:: ` header lines (incl. title/data/style/script).
    pub passages: usize,
    /// Passage headers carrying `[tags]`.
    pub tagged: usize,
    /// `[[links]]` inside passage bodies.
    pub links: usize,
    /// `<<macro>>` occurrences.
    pub macros: usize,
    /// `/% ... %/` comment lines.
    pub comments: usize,
    /// Non-empty body lines.
    pub body_lines: usize,
}

/// Returns `true` when `b` looks like Twee source.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    t.lines()
        .any(|l| l.starts_with(":: StoryTitle") || l.starts_with(":: StoryData"))
        || t.lines().filter(|l| l.starts_with(":: ")).count() >= 3
}

impl Twee {
    /// Parses a Twee file; `None` without any `:: ` header.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        let mut w = Self {
            title: String::new(),
            story_data: false,
            story_blocks: 0,
            passages: 0,
            tagged: 0,
            links: 0,
            macros: 0,
            comments: 0,
            body_lines: 0,
        };
        let mut in_title = false;
        let mut any = false;
        for line in t.lines() {
            if let Some(rest) = line.strip_prefix(":: ") {
                any = true;
                w.passages += 1;
                in_title = rest.starts_with("StoryTitle");
                if rest.starts_with("StoryData") {
                    w.story_data = true;
                }
                if rest.starts_with("StoryStylesheet") || rest.starts_with("StoryScript") {
                    w.story_blocks += 1;
                }
                if rest.contains('[') && rest.contains(']') {
                    w.tagged += 1;
                }
                continue;
            }
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with("/%") || l.ends_with("%/") {
                w.comments += 1;
                continue;
            }
            if in_title && w.title.is_empty() && !l.starts_with('{') {
                w.title = l.to_string();
            }
            w.links += l.matches("[[").count();
            w.macros += l.matches("<<").count();
            w.body_lines += 1;
        }
        any.then_some(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_twee() {
        assert!(detect(b":: StoryTitle\nX\n"));
        assert!(!detect(b"just text\n"));
    }

    #[test]
    fn counts_passages_links() {
        let src = b":: A [tag1]\ngo [[B]]\n\n:: B\n<<set $x to 1>>\n\n:: C\nend\n";
        let w = Twee::parse(src).unwrap();
        assert_eq!(w.passages, 3);
        assert_eq!(w.tagged, 1);
        assert_eq!(w.links, 1);
        assert_eq!(w.macros, 1);
    }
}
