//! `.zshrc` / `.zprofile` census.
//!
//! `export VAR=val`, `setopt X`/`unsetopt X` (`AUTO_CD`,
//! `CORRECT`, `HIST_IGNORE_DUPS`, `SHARE_HISTORY`,
//! `INC_APPEND_HISTORY`, `EXTENDED_HISTORY`, `AUTO_PUSHD`,
//! `PUSHD_IGNORE_DUPS`, `COMPLETE_IN_WORD`, `ALWAYS_TO_END`,
//! `PROMPT_SUBST`, `INTERACTIVE_COMMENTS`, `NO_BEEP`,
//! `AUTOLIST`, `MENU_COMPLETE`, `GLOB_DOTS`,
//! `EXTENDED_GLOB`, `RC_QUOTES`, `VI`/`EMACS`),
//! `alias x=cmd`/`alias -g X=cmd`, `autoload -Uz compinit`
//! and calls, `zstyle ':comp*'`, `bindkey`,
//! `function name() {`/`name() {`, `source`,
//! `plugins=(…)`, `ZSH_THEME=`, `PROMPT=`,
//! `HISTFILE`/`HISTSIZE`/`SAVEHIST` assignments,
//! `typeset`/`local`/`readonly`, `eval "$(cmd init zsh)"`,
//! `add-zsh-hook`, `precmd`, `compdef`, `compadd`,
//! `if`/`[[`/`then`/`fi`, `for`/`in`/`done`, `case`/`esac`,
//! `fpath+=`/`fpath=(…)`.
//!
//! ```rust
//! let z = "export EDITOR=vim\nsetopt SHARE_HISTORY\nalias g=git\n";
//! let c = izanagi_kit::zshrc::Zshrc::parse(z.as_bytes()).unwrap();
//! assert_eq!(c.exports, 1);
//! assert_eq!(c.setopts, 1);
//! ```

/// zshrc census.
#[derive(Debug, Clone)]
pub struct Zshrc {
    /// `export` statements.
    pub exports: usize,
    /// `setopt`/`unsetopt` statements.
    pub setopts: usize,
    /// `alias` statements.
    pub aliases: usize,
    /// `source`/`.` statements.
    pub sources: usize,
    /// `zstyle` statements.
    pub zstyles: usize,
    /// `function name()`/`name() {`/`function name` definitions.
    pub functions: usize,
    /// `autoload` statements.
    pub autoloads: usize,
    /// Other recognised statements (`bindkey`/`compinit`/`compdef`/`add-zsh-hook`/`eval`/`typeset`/`local`/`readonly`/`declare`/`if`/`then`/`elif`/`else`/`fi`/`for`/`in`/`do`/`done`/`while`/`until`/`case`/`esac`/`return`/`echo`/`printf`/`test`/`ulimit`/`umask`/`set`/`cd`/`path`/`fpath`/`precmd`/`preexec`/`chpwd`/`vcs_info`/`promptinit`/`colors`/`setprompt`/`stty`/`ttyctl`/`rehash`/`hash`/`unhash`/`fc`/`history`/`key`/`whereis`/`which`/`whence`/`command`/`builtin`/`noglob`/`setopt`/`limit`/`sched`/`zmodload`/`zle`).
    pub named: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const OTHER_HEADS: &[&str] = &[
    "bindkey",
    "compinit",
    "compdef",
    "compadd",
    "add-zsh-hook",
    "eval",
    "typeset",
    "local",
    "readonly",
    "declare",
    "integer",
    "float",
    "if",
    "then",
    "elif",
    "else",
    "fi",
    "for",
    "in",
    "do",
    "done",
    "while",
    "until",
    "repeat",
    "case",
    "esac",
    "select",
    "return",
    "echo",
    "print",
    "printf",
    "test",
    "ulimit",
    "umask",
    "set",
    "cd",
    "path",
    "fpath",
    "precmd",
    "preexec",
    "chpwd",
    "vcs_info",
    "promptinit",
    "colors",
    "setprompt",
    "stty",
    "ttyctl",
    "rehash",
    "hash",
    "unhash",
    "fc",
    "history",
    "key",
    "whereis",
    "which",
    "whence",
    "command",
    "builtin",
    "noglob",
    "limit",
    "sched",
    "zmodload",
    "zle",
    "[[",
    "[",
    "{",
    "}",
    "trap",
    "exec",
    "exit",
    "emulate",
    "zsh",
    "chgrp",
    "chmod",
    "chown",
    "stat",
    "sysread",
    "syswrite",
];
fn strip_bom(t: &str) -> &str {
    t.strip_prefix('\u{feff}').unwrap_or(t)
}

/// Detect zshrc content.
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
            || head == "setopt"
            || head == "unsetopt"
            || head == "alias"
            || head == "zstyle"
            || head == "bindkey"
            || s.starts_with("plugins=(")
            || s.starts_with("ZSH_THEME")
            || s.contains("() {")
        {
            hits += 1;
        }
    }
    hits >= 2
}

impl Zshrc {
    /// Census a zshrc buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            exports: 0,
            setopts: 0,
            aliases: 0,
            sources: 0,
            zstyles: 0,
            functions: 0,
            autoloads: 0,
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
                "export" => c.exports += 1,
                "setopt" | "unsetopt" => c.setopts += 1,
                "alias" => c.aliases += 1,
                "source" | "." => c.sources += 1,
                "zstyle" => c.zstyles += 1,
                "autoload" => c.autoloads += 1,
                "function" => c.functions += 1,
                _ => {
                    if s.contains("() {") || s.ends_with("()") {
                        c.functions += 1;
                    } else if OTHER_HEADS.contains(&head) {
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
    fn detects_zshrc() {
        let b = b"export EDITOR=vim\nsetopt SHARE_HISTORY\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_zshrc() {
        let b = concat!(
            "# zshrc\n",
            "export EDITOR=nvim\n",
            "export PATH=$HOME/bin:$PATH\n",
            "export LANG=ja_JP.UTF-8\n",
            "export ZSH=$HOME/.oh-my-zsh\n",
            "ZSH_THEME=\"robbyrussell\"\n",
            "plugins=(git zsh-autosuggestions zsh-syntax-highlighting)\n",
            "setopt SHARE_HISTORY\n",
            "setopt HIST_IGNORE_DUPS\n",
            "setopt AUTO_CD\n",
            "setopt CORRECT\n",
            "setopt EXTENDED_GLOB\n",
            "unsetopt BEEP\n",
            "HISTFILE=~/.zsh_history\n",
            "HISTSIZE=100000\n",
            "SAVEHIST=100000\n",
            "alias g=git\n",
            "alias gs='git status'\n",
            "alias -g L='| less'\n",
            "alias -g G='| grep'\n",
            "source $ZSH/oh-my-zsh.sh\n",
            "source ~/.zshrc.local\n",
            ". ~/.cargo/env\n",
            "zstyle ':completion:*' menu select\n",
            "zstyle ':completion:*' matcher-list 'm:{a-z}={A-Z}'\n",
            "autoload -Uz compinit && compinit\n",
            "autoload -Uz vcs_info\n",
            "function mkcd() { mkdir -p \"$1\" && cd \"$1\"; }\n",
            "function gc() { git commit -m \"$1\"; }\n",
            "gup() { git pull --rebase; }\n",
            "bindkey -v\n",
            "bindkey '^R' history-incremental-search-backward\n",
            "bindkey '^P' up-history\n",
            "add-zsh-hook precmd vcs_info\n",
            "eval \"$(starship init zsh)\"\n",
            "typeset -gU path fpath\n",
            "fpath+=~/.zsh/completions\n",
            "if [[ -f ~/.fzf.zsh ]]; then\n",
            "  source ~/.fzf.zsh\n",
            "fi\n",
        );
        let c = Zshrc::parse(b.as_bytes()).unwrap();
        assert_eq!(c.exports, 4);
        assert_eq!(c.setopts, 6);
        assert_eq!(c.aliases, 4);
        assert_eq!(c.sources, 4);
        assert_eq!(c.zstyles, 2);
        assert_eq!(c.functions, 3);
        assert_eq!(c.autoloads, 2);
        assert!(c.named >= 5);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
