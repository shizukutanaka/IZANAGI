//! Godot `project.godot` engine configuration parser.
//!
//! INI-style file: `; Engine configuration file.` comment, `[section]`
//! headers, `key=value` entries. Counts sections, entries, autoloads,
//! input actions and reads `config/name` + `config_version` +
//! `run/main_scene`.
//!
//! ```
//! use izanagi_kit::godot::Godot;
//! let src = b"; Engine configuration file.\n\n[application]\nconfig/name=\"Demo\"\nrun/main_scene=\"res://main.tscn\"\n\n[autoload]\nGlobal=\"*res://global.gd\"\n\n[rendering]\n";
//! assert!(izanagi_kit::godot::detect(src));
//! let g = Godot::parse(src).unwrap();
//! assert_eq!(g.name, "Demo");
//! assert_eq!(g.main_scene, "res://main.tscn");
//! assert_eq!(g.autoloads, 1);
//! ```

/// Parsed census of a `project.godot` file.
#[derive(Debug, Clone)]
pub struct Godot {
    /// `config/name` value.
    pub name: String,
    /// `config_version` (3-5 depending on engine).
    pub config_version: u32,
    /// `run/main_scene` value.
    pub main_scene: String,
    /// `[section]` headers.
    pub sections: usize,
    /// `key=value` entries.
    pub entries: usize,
    /// `[autoload]` section entries.
    pub autoloads: usize,
    /// `[input]` section action entries.
    pub input_actions: usize,
    /// Distinct sections seen.
    pub distinct_sections: usize,
    /// `;`/`#` comment lines.
    pub comments: usize,
}

/// Returns `true` when `b` looks like a `project.godot` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = core::str::from_utf8(b).unwrap_or("");
    t.contains("Engine configuration file")
        || (t.contains("[application]") && t.contains("config/name"))
        || (t.contains("config_version") && !t.contains("[gd_") && t.contains('='))
}

impl Godot {
    /// Parses a `project.godot`; `None` without any `[section]` or `config/` key.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        let mut g = Self {
            name: String::new(),
            config_version: 0,
            main_scene: String::new(),
            sections: 0,
            entries: 0,
            autoloads: 0,
            input_actions: 0,
            distinct_sections: 0,
            comments: 0,
        };
        let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut cur = "";
        for line in t.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with(';') || l.starts_with('#') {
                g.comments += 1;
                continue;
            }
            if l.starts_with('[') && l.contains(']') {
                g.sections += 1;
                let name = l[1..l.find(']').unwrap_or(1)].to_string();
                cur = match name.as_str() {
                    "autoload" => "autoload",
                    "input" => "input",
                    _ => "",
                };
                seen.insert(name);
                continue;
            }
            if let Some(eq) = l.find('=') {
                g.entries += 1;
                let key = l[..eq].trim();
                let val = l[eq + 1..].trim().trim_matches('"');
                match key {
                    "config/name" => g.name = val.to_string(),
                    "config_version" => g.config_version = val.parse().unwrap_or(0),
                    "run/main_scene" => g.main_scene = val.to_string(),
                    _ => {}
                }
                match cur {
                    "autoload" => g.autoloads += 1,
                    "input" => g.input_actions += 1,
                    _ => {}
                }
            }
        }
        g.distinct_sections = seen.len();
        (g.sections > 0 || !g.name.is_empty() || g.config_version > 0).then_some(g)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_config_file() {
        assert!(detect(b"; Engine configuration file.\n[application]\n"));
        assert!(!detect(b"[note]\nkey=1\n"));
    }

    #[test]
    fn counts_sections_entries() {
        let src = b"[application]\nconfig/name=\"X\"\n\n[display]\nwindow/size/viewport_width=640\nwindow/size/viewport_height=480\n";
        let g = Godot::parse(src).unwrap();
        assert_eq!(g.sections, 2);
        assert_eq!(g.entries, 3);
        assert_eq!(g.name, "X");
    }

    #[test]
    fn parse_none_on_random() {
        assert!(Godot::parse(b"nothing").is_none());
    }
}
