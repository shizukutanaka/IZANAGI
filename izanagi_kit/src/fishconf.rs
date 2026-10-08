//! fish `config.fish` census.
//!
//! `set -gx VAR v`/`set -g`/`set -l`/`set -U`/`set -x`,
//! `set_color`, `abbr -a x y`/`abbr`, `alias name='cmd'`/
//! `alias name cmd`, `function name`, `end`, `if`/`else`/
//! `elif`, `for x in ...`, `switch`/`case`, `source`,
//! `status`, `command`, `builtin`, `eval`, `bind`,
//! `funcsave`, `string`, `contains`, `test`/`[`,
//! `fish_vi_key_bindings`/`fish_default_key_bindings`,
//! `complete`, `path`, `fish_add_path`, `emit`,
//! `argparse`, `read`, `echo`, `printf`.
//!
//! ```rust
//! let f = "set -gx EDITOR vim\nabbr -a g git\nfunction fish_greeting\nend\n";
//! let c = izanagi_kit::fishconf::Fishconf::parse(f.as_bytes()).unwrap();
//! assert_eq!(c.sets, 1);
//! ```

use crate::textutil::strip_bom;
/// config.fish census.
#[derive(Debug, Clone)]
pub struct Fishconf {
    /// `set` statements.
    pub sets: usize,
    /// `abbr` statements.
    pub abbrevs: usize,
    /// `alias` statements.
    pub aliases: usize,
    /// `function` statements.
    pub functions: usize,
    /// `source` statements.
    pub sources: usize,
    /// `bind` statements.
    pub binds: usize,
    /// Other recognised statements (`if`/`else`/`for`/`switch`/`case`/`while`/`end`/`return`/`command`/`builtin`/`eval`/`echo`/`printf`/`test`/`contains`/`string`/`complete`/`path`/`fish_add_path`/`emit`/`argparse`/`read`/`status`/`set_color`/`funcsave`/`count`/`math`/`random`/`type`/`exec`/`disown`/`jobs`/`bg`/`fg`/`wait`/`block`/`breakpoint`/`realpath`/`vared`/`fish_*`).
    pub named: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const OTHER_HEADS: &[&str] = &[
    "if",
    "else",
    "elif",
    "for",
    "in",
    "switch",
    "case",
    "while",
    "end",
    "return",
    "break",
    "continue",
    "command",
    "builtin",
    "eval",
    "echo",
    "printf",
    "test",
    "contains",
    "string",
    "complete",
    "path",
    "fish_add_path",
    "emit",
    "argparse",
    "read",
    "status",
    "set_color",
    "funcsave",
    "count",
    "math",
    "random",
    "type",
    "exec",
    "disown",
    "jobs",
    "bg",
    "fg",
    "wait",
    "block",
    "breakpoint",
    "realpath",
    "vared",
    "fish_vi_key_bindings",
    "fish_default_key_bindings",
    "fish_config",
    "fish_update_completions",
    "begin",
    "and",
    "or",
    "not",
    "true",
    "false",
    "exit",
    "cd",
    "pushd",
    "popd",
    "dirs",
    "umask",
    "export",
    "shift",
];

/// Detect config.fish content.
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
        if head == "set" || head == "abbr" || head == "alias" || head == "function" {
            hits += 1;
        }
    }
    hits >= 2
}

impl Fishconf {
    /// Census a config.fish buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            sets: 0,
            abbrevs: 0,
            aliases: 0,
            functions: 0,
            sources: 0,
            binds: 0,
            named: 0,
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
                "set" => c.sets += 1,
                "abbr" => c.abbrevs += 1,
                "alias" => c.aliases += 1,
                "function" | "functions" => c.functions += 1,
                "source" | "." => c.sources += 1,
                "bind" => c.binds += 1,
                _ => {
                    if OTHER_HEADS.contains(&head) {
                        c.named += 1;
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
    fn detects_fish() {
        let b = b"set -gx EDITOR vim\nabbr -a g git\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_fish() {
        let b = concat!(
            "# config.fish\n",
            "set -gx EDITOR nvim\n",
            "set -gx VISUAL nvim\n",
            "set -g fish_greeting\n",
            "set -U fish_user_paths ~/.local/bin\n",
            "set -x GOPATH ~/go\n",
            "set_color red\n",
            "fish_vi_key_bindings\n",
            "abbr -a g git\n",
            "abbr -a gs 'git status'\n",
            "abbr -a --add ll 'ls -la'\n",
            "alias l='ls -la'\n",
            "alias vi nvim\n",
            "function fish_prompt\n",
            "  echo '> '\n",
            "end\n",
            "function mkcd\n",
            "  mkdir -p $argv; and cd $argv\n",
            "end\n",
            "function gc\n",
            "  git commit -m $argv\n",
            "end\n",
            "source ~/.config/fish/local.fish\n",
            "source /opt/asdf/asdf.fish\n",
            "bind \\cg 'git status; commandline -f repaint'\n",
            "bind --user \\e\\[1\\;3C forward-bigword\n",
            "if status is-interactive\n",
            "  starship init fish | source\n",
            "end\n",
            "for f in ~/.config/fish/conf.d/*.fish\n",
            "  source $f\n",
            "end\n",
            "switch $TERM\n",
            "  case 'xterm*'\n",
            "    set -gx TERM xterm-256color\n",
            "end\n",
            "eval (direnv hook fish)\n",
        );
        let c = Fishconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sets, 6);
        assert_eq!(c.abbrevs, 3);
        assert_eq!(c.aliases, 2);
        assert_eq!(c.functions, 3);
        assert_eq!(c.sources, 3);
        assert_eq!(c.binds, 2);
        assert!(c.named >= 12);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
