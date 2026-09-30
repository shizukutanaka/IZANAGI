//! Nushell `config.nu` / `env.nu` census.
//!
//! `$env.* = v` (`$env.PATH`, `$env.config = { ... }`,
//! `$env.config.show_banner`, `$env.ENV_CONVERSIONS`),
//! `let x = `/`mut x`, `def name [args] { }`/`def-env`/
//! `export def`, `alias x = `, `use`/`export use`/`module`/
//! `source`/`source-env`/`overlay use`/`hide`, `export env`,
//! `$env.config.keybindings`, `$env.config.hooks`,
//! `$env.config.menus`, `ls`/`cd` pipeline bodies ignored.
//!
//! ```rust
//! let n = "$env.config = { show_banner: false }\ndef ll [path?: string] { ls $path }\nalias g = git\n";
//! let c = izanagi_kit::nuconf::Nuconf::parse(n.as_bytes()).unwrap();
//! assert_eq!(c.defs, 1);
//! ```

/// config.nu census.
#[derive(Debug, Clone)]
pub struct Nuconf {
    /// `$env.*`/`$env.config.*` assignments.
    pub envvars: usize,
    /// `def`/`def-env`/`export def` definitions.
    pub defs: usize,
    /// `let`/`mut` assignments.
    pub lets: usize,
    /// `alias`/`export alias` statements.
    pub aliases: usize,
    /// `use`/`export use`/`module`/`overlay use` statements.
    pub uses: usize,
    /// `source`/`source-env` statements.
    pub sources: usize,
    /// Other recognised statements (`export`/`hide`/`ls`/`cd`/`echo`/`print`/`match`/`if`/`else`/`for`/`loop`/`while`/`do`/`try`/`catch`/`return`/`error make`/`assert`/`describe`/`sleep`/`timeit`/`version`/`help`/`config`/`keybindings`/`register`).
    pub named: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const OTHER_HEADS: &[&str] = &[
    "export",
    "hide",
    "ls",
    "cd",
    "echo",
    "print",
    "match",
    "if",
    "else",
    "for",
    "loop",
    "while",
    "do",
    "try",
    "catch",
    "return",
    "error",
    "assert",
    "describe",
    "sleep",
    "timeit",
    "version",
    "help",
    "config",
    "keybindings",
    "register",
    "export-env",
    "save",
    "open",
    "break",
    "continue",
    "null",
    "const",
];

/// Detect config.nu content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut hits = 0usize;
    for line in t.lines() {
        let s = line.trim();
        if s.starts_with("$env")
            || s.starts_with("def ")
            || s.starts_with("alias ")
            || s.starts_with("export def")
        {
            hits += 1;
        }
    }
    hits >= 2
}

impl Nuconf {
    /// Census a config.nu buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            envvars: 0,
            defs: 0,
            lets: 0,
            aliases: 0,
            uses: 0,
            sources: 0,
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
            if s.starts_with("$env") {
                c.envvars += 1;
                continue;
            }
            if s.starts_with("def ") || s.starts_with("def-env") || s.starts_with("export def") {
                c.defs += 1;
                continue;
            }
            if s.starts_with("alias ") || s.starts_with("export alias") {
                c.aliases += 1;
                continue;
            }
            if s.starts_with("let ") || s.starts_with("mut ") || s.starts_with("const ") {
                c.lets += 1;
                continue;
            }
            if s.starts_with("use ")
                || s.starts_with("export use")
                || s.starts_with("module ")
                || s.starts_with("overlay use")
            {
                c.uses += 1;
                continue;
            }
            if s.starts_with("source ") || s.starts_with("source-env") {
                c.sources += 1;
                continue;
            }
            let head = s.split(' ').next().unwrap_or("");
            if OTHER_HEADS.contains(&head) {
                c.named += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_nu() {
        let b = b"$env.PATH = []\ndef x [] {}\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_nu() {
        let b = concat!(
            "# config.nu\n",
            "$env.config = {\n",
            "  show_banner: false\n",
            "  edit_mode: vi\n",
            "  buffer_editor: \"nvim\"\n",
            "  history: {\n",
            "    max_size: 100000\n",
            "    sync_on_enter: true\n",
            "  }\n",
            "  completions: {\n",
            "    case_sensitive: false\n",
            "    quick: true\n",
            "  }\n",
            "  keybindings: [\n",
            "    {\n",
            "      name: fuzzy_history\n",
            "      modifier: control\n",
            "      keycode: char_r\n",
            "      mode: [emacs, vi_normal, vi_insert]\n",
            "    }\n",
            "  ]\n",
            "}\n",
            "$env.PATH = ($env.PATH | prepend '~/.local/bin')\n",
            "$env.EDITOR = \"nvim\"\n",
            "$env.VISUAL = \"nvim\"\n",
            "let carapace_completer = {|spans|\n",
            "  carapace $spans.0 nushell $spans\n",
            "}\n",
            "mut cache_dir = '~/.cache'\n",
            "const config_dir = '~/.config'\n",
            "def ll [path?: string] { ls $path }\n",
            "def gco [branch: string] { git checkout $branch }\n",
            "def-env set-title [title: string] { }\n",
            "export def mkcd [dir: string] { mkdir $dir; cd $dir }\n",
            "alias g = git\n",
            "alias gst = git status\n",
            "export alias l = ls -la\n",
            "use std/dirs\n",
            "use std/util 'path add'\n",
            "module tools\n",
            "overlay use zoxide.nu\n",
            "source ~/.config/nushell/local.nu\n",
            "source-env ~/.config/nushell/secrets.nu\n",
            "hide starship\n",
            "match $env.OS {\n",
            "  'macos' => {}\n",
            "  _ => {}\n",
            "}\n",
        );
        let c = Nuconf::parse(b.as_bytes()).unwrap();
        assert_eq!(c.envvars, 4);
        assert_eq!(c.defs, 4);
        assert_eq!(c.lets, 3);
        assert_eq!(c.aliases, 3);
        assert_eq!(c.uses, 4);
        assert_eq!(c.sources, 2);
        assert!(c.named >= 1);
        assert_eq!(c.comments, 1);
    }
}
