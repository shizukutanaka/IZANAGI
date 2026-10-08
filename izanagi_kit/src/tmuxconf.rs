//! Parser for tmux configuration files (`tmux.conf` / `.tmux.conf`).
//!
//! Counts `set`/`set-option`/`setw`/`set-window-option`/`set-environment`
//! commands, `bind`/`bind-key`/`unbind`/`unbind-key` bindings, `source-file`,
//! `if-shell`/`%if`, hooks (`set-hook`, `run-shell`), plugin lines
//! (`@plugin`), and comments.
//!
//! ```
//! let b = b"set -g prefix C-a\nbind r source-file ~/.tmux.conf\nset -g @plugin 'tmux-plugins/tpm'\n";
//! assert!(izanagi_kit::tmuxconf::detect(b));
//! let c = izanagi_kit::tmuxconf::Tmuxconf::parse(b).unwrap();
//! assert_eq!(c.sets, 2);
//! assert_eq!(c.binds, 1);
//! assert_eq!(c.plugins, 1);
//! ```

/// Parsed tmux.conf summary.
#[derive(Debug, Clone)]
pub struct Tmuxconf {
    /// `set`/`set-option`/`set-option -a`/`set -g`/`set -s`/`set -u` commands.
    pub sets: usize,
    /// `setw`/`set-window-option` window options.
    pub setws: usize,
    /// `set-environment`/`setenv` lines.
    pub setenvs: usize,
    /// `bind`/`bind-key`/`bind-key -n`/`bind-key -r` bindings.
    pub binds: usize,
    /// `unbind`/`unbind-key` lines.
    pub unbinds: usize,
    /// `source-file`/`source` includes.
    pub sources: usize,
    /// `if-shell`/`%if`/`%elif`/`%endif` conditionals.
    pub conditionals: usize,
    /// `set-hook`/`run-shell`/`run`/`display-message`/`new-window`/`split-window`/`send-keys` commands.
    pub commands: usize,
    /// `set -g @plugin`/`set -g @name` user options.
    pub plugins: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const BIND_CMDS: &[&str] = &["bind", "bind-key", "unbind", "unbind-key"];
const SRC_CMDS: &[&str] = &["source", "source-file"];
const OTHER_CMDS: &[&str] = &[
    "set-hook",
    "run-shell",
    "run",
    "display-message",
    "display",
    "new-window",
    "neww",
    "split-window",
    "splitw",
    "send-keys",
    "send",
    "kill-pane",
    "select-pane",
    "resize-pane",
    "swap-pane",
    "rename-window",
    "move-window",
    "attach-session",
    "detach-client",
    "clock",
];

/// ほぼ tmux 固有の動詞(`send`/`display`/`run`/`source`/`bind`/`set` は
/// 他形式でも現れるため弱い証拠)。
const STRONG_CMDS: &[&str] = &[
    "bind-key",
    "unbind-key",
    "set-option",
    "set-window-option",
    "setw",
    "setenv",
    "set-environment",
    "set-hook",
    "run-shell",
    "send-keys",
    "new-window",
    "neww",
    "split-window",
    "splitw",
    "kill-pane",
    "select-pane",
    "resize-pane",
    "swap-pane",
    "rename-window",
    "move-window",
    "attach-session",
    "detach-client",
    "display-message",
    "display-panes",
    "if-shell",
    "clock",
    "clock-mode",
    "show-options",
    "show-option",
    "list-keys",
    "source-file",
];

/// Returns `true` when the bytes look like a tmux.conf.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    // `set`/`bind`/`send`/`source` 等の裸動詞だけでは他形式と区別が付かない。
    // tmux 固有動詞(bind-key/setw/send-keys/…)か `set -g …` 形のオプション
    // 代入を要求する。
    let mut strong = 0usize;
    let mut opt_set = 0usize;
    let mut weak = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        let mut it = tr.split_whitespace();
        let first = it.next().unwrap_or("");
        if STRONG_CMDS.contains(&first) {
            strong += 1;
            continue;
        }
        if first == "set" {
            // `set -g`/`set -s`/`set -u`/`set -a`/`set -F`/`set -e`/`set -o`
            // あるいは `set-option` 系のフラグ付き形だけを tmux 証拠とする。
            if it.next().is_some_and(|f| f.starts_with('-')) {
                opt_set += 1;
            }
            continue;
        }
        if BIND_CMDS.contains(&first) || SRC_CMDS.contains(&first) || OTHER_CMDS.contains(&first) {
            weak += 1;
        }
    }
    strong >= 1 || opt_set >= 2 || (opt_set >= 1 && weak >= 1)
}

impl Tmuxconf {
    /// Parses a tmux.conf, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            sets: 0,
            setws: 0,
            setenvs: 0,
            binds: 0,
            unbinds: 0,
            sources: 0,
            conditionals: 0,
            commands: 0,
            plugins: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if tr.starts_with('%') {
                c.conditionals += 1;
                continue;
            }
            let mut it = tr.split_whitespace();
            let cmd = it.next().unwrap_or("");
            match cmd {
                "set" | "set-option" => {
                    c.sets += 1;
                    if tr.contains("@plugin") || tr.contains(" @") {
                        c.plugins += 1;
                    }
                }
                "setw" | "set-window-option" => c.setws += 1,
                "setenv" | "set-environment" => c.setenvs += 1,
                "bind" | "bind-key" => c.binds += 1,
                "unbind" | "unbind-key" => c.unbinds += 1,
                "source" | "source-file" => c.sources += 1,
                "if-shell" | "if" => c.conditionals += 1,
                _ => {
                    if OTHER_CMDS.contains(&cmd) {
                        c.commands += 1;
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

    const CONF: &[u8] = b"# conf\nset -g prefix C-a\nset -g mouse on\nsetw -g mode-keys vi\nbind r source-file ~/.tmux.conf\nbind -n M-l select-pane -R\nunbind C-b\nset -g @plugin 'tmux-plugins/tpm'\nrun-shell ~/.tmux/plugins/tpm/tpm\n";

    #[test]
    fn parses_tmuxconf() {
        let c = Tmuxconf::parse(CONF).unwrap();
        assert_eq!(c.sets, 3);
        assert_eq!(c.setws, 1);
        assert_eq!(c.binds, 2);
        assert_eq!(c.unbinds, 1);
        assert_eq!(c.plugins, 1);
        assert_eq!(c.commands, 1);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn rejects_non_tmuxconf() {
        assert!(!detect(b"key=value\nfoo bar"));
        assert!(Tmuxconf::parse(b"x").is_none());
    }
}
