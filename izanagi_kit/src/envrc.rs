//! direnv `.envrc` census.
//!
//! `export VAR=val`, `use nix|flake|asdf|rbenv|node|python|
//! guix|vim|julien|mise|rtx`, `layout python|node|ruby|go|
//! perl|php|julia|anaconda|pyenv`, `dotenv`/`dotenv_if_exists`,
//! `watch_file`/`watch`, `source_env`/`source_up`/
//! `source_up_if_exists`/`source_env_if_exists`,
//! `load_prefix`, `PATH_add`/`path_add`/`MANPATH_add`,
//! `env_vars_required`/`require`/`strict_env`/`unstrict_env`,
//! `user_rel_path`/`expand_path`/`direnv_load`/`find_up`/`has`/
//! `on_git_branch`/`direnv_apply_dump`/`PATH_rm`,
//! `rvm`, `use_guix`, shell statements.
//!
//! ```rust
//! let e = "export FOO=1\nuse flake\nPATH_add bin\n";
//! let c = izanagi_kit::envrc::Envrc::parse(e.as_bytes()).unwrap();
//! assert_eq!(c.exports, 1);
//! assert_eq!(c.uses, 1);
//! ```

/// .envrc census.
#[derive(Debug, Clone)]
pub struct Envrc {
    /// `export` statements.
    pub exports: usize,
    /// `use` statements.
    pub uses: usize,
    /// `layout` statements.
    pub layouts: usize,
    /// `source*`/`load_prefix`/`dotenv*` statements.
    pub sources: usize,
    /// `watch*`/`on_git_branch` statements.
    pub watches: usize,
    /// Helper statements (`PATH_add`/`MANPATH_add`/`env_vars_required`/`strict_env`/`unstrict_env`/`require`/`user_rel_path`/`expand_path`/`direnv_load`/`find_up`/`has`/`rvm`/`PATH_rm`/`log_status`/`log_error`/`direnv_version`/`semver_search`/`use_vim`).
    pub helpers: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const SOURCE_HEADS: &[&str] = &[
    "source_env",
    "source_up",
    "source_up_if_exists",
    "source_env_if_exists",
    "source_url",
    "load_prefix",
    "dotenv",
    "dotenv_if_exists",
];

const WATCH_HEADS: &[&str] = &["watch_file", "watch", "on_git_branch", "watch_dir"];

const HELPER_HEADS: &[&str] = &[
    "PATH_add",
    "path_add",
    "MANPATH_add",
    "PATH_rm",
    "env_vars_required",
    "strict_env",
    "unstrict_env",
    "require",
    "user_rel_path",
    "expand_path",
    "direnv_load",
    "direnv_apply_dump",
    "find_up",
    "has",
    "rvm",
    "log_status",
    "log_error",
    "direnv_version",
    "semver_search",
    "use_vim",
    "use_guix",
    "nix",
    "guix",
    "direnv_layout_dir",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detect .envrc content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let t = strip_bom(t);
    let mut hits = 0usize;
    for line in t.lines() {
        let s = line.trim();
        let head = s.split(' ').next().unwrap_or("");
        if head == "export"
            || head == "use"
            || head == "layout"
            || SOURCE_HEADS.contains(&head)
            || WATCH_HEADS.contains(&head)
            || HELPER_HEADS.contains(&head)
        {
            hits += 1;
        }
    }
    hits >= 2
}

impl Envrc {
    /// Census an .envrc buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            exports: 0,
            uses: 0,
            layouts: 0,
            sources: 0,
            watches: 0,
            helpers: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let head = s.split([' ', '\t']).next().unwrap_or("");
            match head {
                "export" => c.exports += 1,
                "use" => c.uses += 1,
                "layout" => c.layouts += 1,
                _ => {
                    if SOURCE_HEADS.contains(&head) {
                        c.sources += 1;
                    } else if WATCH_HEADS.contains(&head) {
                        c.watches += 1;
                    } else if HELPER_HEADS.contains(&head) {
                        c.helpers += 1;
                    }
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
    fn detects_envrc() {
        let b = b"export FOO=1\nuse flake\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_envrc() {
        let b = concat!(
            "# envrc\n",
            "export PROJECT_ROOT=$PWD\n",
            "export DATABASE_URL=postgres://localhost/dev\n",
            "export LOG_LEVEL=debug\n",
            "export AWS_PROFILE=dev\n",
            "use nix\n",
            "use flake\n",
            "use asdf\n",
            "use node 20\n",
            "layout python\n",
            "layout ruby\n",
            "dotenv .env.local\n",
            "dotenv_if_exists .env.optional\n",
            "source_env .env.secrets\n",
            "source_up\n",
            "source_up_if_exists .envrc.parent\n",
            "watch_file flake.nix\n",
            "watch_file pyproject.toml\n",
            "watch .env\n",
            "PATH_add bin\n",
            "PATH_add scripts\n",
            "MANPATH_add man\n",
            "PATH_rm /usr/old/bin\n",
            "env_vars_required DATABASE_URL\n",
            "strict_env\n",
            "require curl\n",
            "has jq && echo ok\n",
            "on_git_branch main\n",
        );
        let c = Envrc::parse(b.as_bytes()).unwrap();
        assert_eq!(c.exports, 4);
        assert_eq!(c.uses, 4);
        assert_eq!(c.layouts, 2);
        assert_eq!(c.sources, 5);
        assert_eq!(c.watches, 4);
        assert!(c.helpers >= 8);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
