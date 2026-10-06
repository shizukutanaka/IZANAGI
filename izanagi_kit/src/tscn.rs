//! Godot `.tscn` scene / `.tres` resource text format parser.
//!
//! `[gd_scene load_steps=N format=N uid="..."]` / `[gd_resource ...]`
//! headers, `[node name type parent]`, `[ext_resource]`, `[sub_resource]`,
//! `[resource]`, `[connection]`, `[editable]` blocks and `key=value` props.
//!
//! ```
//! use izanagi_kit::tscn::Tscn;
//! let src = b"[gd_scene load_steps=2 format=3 uid=\"uid://abc\"]\n\n[ext_resource type=\"Script\" path=\"res://a.gd\" id=\"1\"]\n\n[node name=\"Root\" type=\"Node2D\"]\n\n[node name=\"Child\" type=\"Sprite2D\" parent=\".\"]\n";
//! assert!(izanagi_kit::tscn::detect(src));
//! let s = Tscn::parse(src).unwrap();
//! assert_eq!(s.format, 3);
//! assert_eq!(s.nodes, 2);
//! assert_eq!(s.ext_resources, 1);
//! ```

/// Parsed census of a `.tscn`/`.tres` file.
#[derive(Debug, Clone)]
pub struct Tscn {
    /// `gd_scene` (scene) vs `gd_resource` (resource) document kind.
    pub kind: String,
    /// `format=` attribute.
    pub format: u32,
    /// `load_steps=` attribute.
    pub load_steps: u32,
    /// `uid="..."` attribute.
    pub uid: String,
    /// `[node ...]` blocks.
    pub nodes: usize,
    /// `[ext_resource ...]` blocks.
    pub ext_resources: usize,
    /// `[sub_resource ...]` blocks.
    pub sub_resources: usize,
    /// `[resource]` blocks.
    pub resources: usize,
    /// `[connection ...]` signal connections.
    pub connections: usize,
    /// `[editable ...]` blocks.
    pub editables: usize,
    /// `key=value` property lines.
    pub properties: usize,
    /// `;`/`#` comment lines.
    pub comments: usize,
}

fn attr<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let pat = format!("{}=\"", name);
    let i = line.find(&pat)?;
    let s = &line[i + pat.len()..];
    let end = s.find('"')?;
    Some(&s[..end])
}

fn attr_int(line: &str, name: &str) -> u32 {
    let pat = format!("{}=", name);
    let Some(i) = line.find(&pat) else { return 0 };
    let s = &line[i + pat.len()..];
    let digits: usize = s.bytes().take_while(u8::is_ascii_digit).count();
    s[..digits].parse().unwrap_or(0)
}

/// Returns `true` when `b` looks like a `.tscn`/`.tres` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    // BOM(U+FEFF)は trim_start が空白と見なさないため先に剥がす。
    let t = t.strip_prefix('\u{feff}').unwrap_or(t);
    let tt = t.trim_start();
    tt.starts_with("[gd_scene") || tt.starts_with("[gd_resource")
}

impl Tscn {
    /// Parses a `.tscn`/`.tres`; `None` without a `gd_scene`/`gd_resource` header.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        let t = t.strip_prefix('\u{feff}').unwrap_or(t);
        let tt = t.trim_start();
        let kind = if tt.starts_with("[gd_scene") {
            "gd_scene"
        } else if tt.starts_with("[gd_resource") {
            "gd_resource"
        } else {
            return None;
        };
        let head = t.lines().next().unwrap_or("");
        let mut s = Self {
            kind: kind.to_string(),
            format: attr_int(head, "format"),
            load_steps: attr_int(head, "load_steps"),
            uid: attr(head, "uid").unwrap_or("").to_string(),
            nodes: 0,
            ext_resources: 0,
            sub_resources: 0,
            resources: 0,
            connections: 0,
            editables: 0,
            properties: 0,
            comments: 0,
        };
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with(';') || l.starts_with('#') {
                s.comments += 1;
                continue;
            }
            if l.starts_with('[') {
                if l.starts_with("[node") {
                    s.nodes += 1;
                } else if l.starts_with("[ext_resource") {
                    s.ext_resources += 1;
                } else if l.starts_with("[sub_resource") {
                    s.sub_resources += 1;
                } else if l.starts_with("[resource") {
                    s.resources += 1;
                } else if l.starts_with("[connection") {
                    s.connections += 1;
                } else if l.starts_with("[editable") {
                    s.editables += 1;
                }
                continue;
            }
            if l.contains('=') {
                s.properties += 1;
            }
        }
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_utf8_bom() {
        assert!(detect(b"\xef\xbb\xbf[gd_scene format=3]\n"));
        let s = Tscn::parse(b"\xef\xbb\xbf[gd_scene format=3]\n").unwrap();
        assert_eq!(s.kind, "gd_scene");
    }

    #[test]
    fn detects_scene() {
        assert!(detect(b"[gd_scene format=3]\n"));
        assert!(detect(b"[gd_resource type=\"Resource\" format=3]\n"));
        assert!(!detect(b"[application]\n"));
    }

    #[test]
    fn counts_blocks() {
        let src = b"[gd_scene format=3]\n\n[sub_resource type=\"CircleShape2D\" id=\"1\"]\n\n[node name=\"A\" type=\"CharacterBody2D\"]\nscript = ExtResource(\"1\")\n\n[connection signal=\"hit\" from=\"A\" to=\"A\" method=\"_on_hit\"]\n";
        let s = Tscn::parse(src).unwrap();
        assert_eq!(s.sub_resources, 1);
        assert_eq!(s.nodes, 1);
        assert_eq!(s.connections, 1);
        assert_eq!(s.properties, 1);
    }
}
