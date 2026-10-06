//! moonrepo `moon.yml`/`.moon/*.yml` workspace census.
//!
//! Project files (`moon.yml`) declare `id:`/`project:`/`language:`/
//! `type:`/`tasks:`/`fileGroups:`/`dependsOn:`; workspace files
//! (`.moon/workspace.yml`, `toolchain.yml`, `tasks.yml`) add
//! `projects:`/`vcs:`/`runner:`/`toolchain:`/`generator:`.
//!
//! ```rust
//! let k = b"id: web\nlanguage: typescript\ntasks:\n  build:\n    command: vite build\n  test:\n    command: vitest\n";
//! assert!(izanagi_kit::moonrepo::detect(k));
//! ```

/// moonrepo config census.
#[derive(Debug, Clone)]
pub struct Moonrepo {
    /// `tasks:` entries.
    pub tasks: usize,
    /// `fileGroups:` entries.
    pub file_groups: usize,
    /// Recognised top-level keys present.
    pub sections: usize,
    /// `key:`/`key: value` lines.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const TOP_KEYS: &[&str] = &[
    "id",
    "project",
    "language",
    "type",
    "stack",
    "tasks",
    "fileGroups",
    "dependsOn",
    "deps",
    "env",
    "tags",
    "project",
    "projects",
    "vcs",
    "runner",
    "toolchain",
    "generator",
    "workspace",
    "experiments",
    "codeowners",
    "constraints",
    "docker",
    "hasher",
    "notifier",
    "implicitDeps",
    "implicitInputs",
];

/// Strong keys that, combined with `tasks:`, identify moon.
const MOON_HINTS: &[&str] = &[
    "language",
    "type",
    "stack",
    "fileGroups",
    "dependsOn",
    "vcs",
    "runner",
    "toolchain",
    "projects",
    "codeowners",
    "generator",
];

fn top_key(line: &str) -> Option<&str> {
    if line.starts_with(' ') || line.starts_with('\t') || line.starts_with('-') {
        return None;
    }
    let s = line.trim_end();
    if s.starts_with('#') {
        return None;
    }
    match s.find(':') {
        Some(i) => Some(s[..i].trim()),
        None => None,
    }
}

/// Detect moonrepo config content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut has_tasks = false;
    let mut hints = 0usize;
    for line in t.lines() {
        if let Some(k) = top_key(line) {
            if k == "tasks" {
                has_tasks = true;
            } else if MOON_HINTS.contains(&k) {
                hints += 1;
            }
        }
    }
    // `tasks:` plus a moon-specific sibling, or several moon keys.
    (has_tasks && hints >= 1) || hints >= 3
}

impl Moonrepo {
    /// Census a moonrepo buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            tasks: 0,
            file_groups: 0,
            sections: 0,
            settings: 0,
            comments: 0,
        };
        let mut section = "";
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(k) = top_key(line) {
                section = k;
                if TOP_KEYS.contains(&k) {
                    c.sections += 1;
                }
                c.settings += 1;
                continue;
            }
            if s.starts_with('-') && s.contains(':') {
                continue;
            }
            if s.contains(':') {
                // Two-space-indented child of `tasks:`/`fileGroups:`.
                if section == "tasks" && line.starts_with("  ") && !line.starts_with("   ") {
                    c.tasks += 1;
                } else if section == "fileGroups"
                    && line.starts_with("  ")
                    && !line.starts_with("   ")
                {
                    c.file_groups += 1;
                }
                c.settings += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_project() {
        let b = b"id: web\nlanguage: typescript\ntasks:\n  build:\n    command: vite build\n  test:\n    command: vitest\n";
        assert!(detect(b));
        let c = Moonrepo::parse(b).unwrap();
        assert_eq!(c.tasks, 2);
    }

    #[test]
    fn detects_workspace() {
        let b = b"projects:\n  - 'apps/*'\nvcs:\n  manager: git\nrunner:\n  implicitDeps: []\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other_yaml() {
        // Taskfile uses `tasks:` too but has `version:`/`cmds:` not moon keys.
        assert!(!detect(b"tasks:\n  build:\n    cmds:\n      - echo hi\n"));
        assert!(!detect(b"key: value\nlist:\n- a\n"));
    }
}
