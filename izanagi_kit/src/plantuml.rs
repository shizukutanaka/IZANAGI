//! PlantUML text diagram description language (`.puml` / `.plantuml`).
//!
//! PlantUML files are delimited by `@startuml`/`@enduml` markers (plus
//! `@startsalt`, `@startmindmap`, `@startgantt`, …), declare participants
//! with `participant`/`actor`, arrows with `->`/`-->`, and OO types with
//! `class`/`interface`/`abstract`/`enum`.
//!
//! ```
//! let b = concat!(
//!     "@startuml\n",
//!     "participant Alice\n",
//!     "participant Bob\n",
//!     "Alice -> Bob : hello\n",
//!     "@enduml\n"
//! ).as_bytes();
//! assert!(izanagi_kit::plantuml::detect(b));
//! let c = izanagi_kit::plantuml::Plantuml::parse(b).unwrap();
//! assert_eq!(c.blocks, 1);
//! assert_eq!(c.arrows, 1);
//! ```

/// Parsed PlantUML summary.
#[derive(Debug, Clone)]
pub struct Plantuml {
    /// `@start*` blocks.
    pub blocks: usize,
    /// `->`, `-->`, `->>`, `<-`, `<..`, etc. arrow operators.
    pub arrows: usize,
    /// `participant`/`actor`/`boundary`/`control`/`entity`/`database`/`collections`/`queue`/`create` declarations.
    pub participants: usize,
    /// `class`/`interface`/`abstract`/`enum`/`annotation`/`package`/`namespace`/`object`/`component`/`node`/`folder`/`cloud`/`frame`/`rectangle`/`artifact`/`card`/`entity`/`usecase`/`circle`/`diamond` declarations.
    pub types: usize,
    /// `note` declarations.
    pub notes: usize,
    /// `'` comment lines plus `!` directives (`!include`, `!define`, `!pragma`, `!theme`, …).
    pub comments_directives: usize,
    /// `start`/`end`/`stop`/`if`/`else`/`while`/`repeat`/`fork`/`split`/`group`/`alt`/`opt`/`loop`/`par`/`break`/`critical`/`activate`/`deactivate`/`destroy`/`box`/`ref`/`auto`/`hide`/`show`/`skin`/`title`/`legend`/`caption`/`header`/`footer`/`scale`/`left to right direction`/`top to bottom direction`/`together`/`newpage`/`==` divider keywords.
    pub keywords: usize,
}

const TYPE_KW: &[&str] = &[
    "class",
    "interface",
    "abstract",
    "enum",
    "annotation",
    "package",
    "namespace",
    "object",
    "component",
    "node",
    "folder",
    "cloud",
    "frame",
    "rectangle",
    "artifact",
    "card",
    "usecase",
    "circle",
    "diamond",
    "file",
    "storage",
    "agent",
];
const FLOW_KW: &[&str] = &[
    "start",
    "end",
    "stop",
    "if",
    "else",
    "elseif",
    "while",
    "repeat",
    "fork",
    "split",
    "group",
    "alt",
    "opt",
    "loop",
    "par",
    "break",
    "critical",
    "activate",
    "deactivate",
    "destroy",
    "box",
    "ref",
    "auto",
    "hide",
    "show",
    "skin",
    "title",
    "legend",
    "caption",
    "header",
    "footer",
    "scale",
    "left to right direction",
    "top to bottom direction",
    "together",
    "newpage",
    "==",
    "partition",
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

/// Whether the buffer looks like PlantUML.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let has_marker = t.contains("@start") && t.contains("@end");
    has_marker || (t.contains("skinparam") && (t.contains("->") || t.contains("class ")))
}

impl Plantuml {
    /// Parses a PlantUML summary.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            blocks: 0,
            arrows: 0,
            participants: 0,
            types: 0,
            notes: 0,
            comments_directives: 0,
            keywords: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('@') {
                if tr.starts_with("@start") {
                    c.blocks += 1;
                }
                continue;
            }
            if tr.starts_with('\'') || tr.starts_with('!') {
                c.comments_directives += 1;
                continue;
            }
            // Longer operators contain shorter ones (`->` inside `->>`,
            // `..` inside `..>`, `<-` inside `<--`); subtract inner overlaps.
            let mut arr = 0usize;
            for pat in [
                "-[hidden]->",
                "->>",
                "->",
                "..>",
                "..",
                "-->",
                "<--",
                "<-",
                "<|..",
                "<|--",
                "*--",
                "o--",
                "+--",
                "#--",
                "x--",
                "}--",
            ] {
                arr += count(tr, pat);
            }
            c.arrows += arr.saturating_sub(
                count(tr, "->>") + count(tr, "..>") + count(tr, "<--") + count(tr, "-[hidden]->"),
            );
            let low = tr.to_ascii_lowercase();
            if [
                "participant ",
                "actor ",
                "boundary ",
                "control ",
                "entity ",
                "database ",
                "collections ",
                "queue ",
                "create ",
            ]
            .iter()
            .any(|k| low.starts_with(k))
            {
                c.participants += 1;
            }
            for kw in TYPE_KW {
                if low.starts_with(kw) {
                    c.types += 1;
                    break;
                }
            }
            if low.starts_with("note") {
                c.notes += 1;
            }
            for kw in FLOW_KW {
                if low == *kw || low.starts_with(&format!("{kw} ")) {
                    c.keywords += 1;
                    break;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sequence() {
        let b = concat!(
            "@startuml\n",
            "participant Alice\n",
            "participant Bob\n",
            "Alice -> Bob : hello\n",
            "@enduml\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Plantuml::parse(b).unwrap();
        assert_eq!(c.blocks, 1);
        assert_eq!(c.participants, 2);
        assert_eq!(c.arrows, 1);
    }

    #[test]
    fn parses_class() {
        let b = concat!(
            "@startuml\n",
            "class Foo {\n",
            "  +bar\n",
            "}\n",
            "Foo <|-- Bar\n",
            "note right of Foo : hi\n",
            "@enduml\n"
        )
        .as_bytes();
        assert!(detect(b));
        let c = Plantuml::parse(b).unwrap();
        assert_eq!(c.blocks, 1);
        assert_eq!(c.types, 1);
        assert_eq!(c.arrows, 1);
        assert_eq!(c.notes, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"int x = 0;"));
        assert!(Plantuml::parse(b"x").is_none());
    }
}
