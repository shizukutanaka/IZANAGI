//! `.bashrc` / `.bash_profile` / `.profile` census.
//!
//! `export VAR=val`, `alias x=cmd`, `shopt -s/-u`
//! (`histappend`, `checkwinsize`, `cmdhist`, `globstar`,
//! `cdspell`, `dirspell`, `autocd`, `checkhash`,
//! `expand_aliases`, `extglob`, `nocaseglob`, `nullglob`,
//! `progcomp`, `promptvars`, `sourcepath`),
//! `PS1=`/`PS2=`/`PROMPT_COMMAND=`/`HISTFILE`/`HISTSIZE`/
//! `HISTFILESIZE`/`HISTCONTROL`/`HISTIGNORE` assignments,
//! `source`/`.`, `function name() {`/`name() {`,
//! `[[ -f ]]`, `bind`, `complete`, `eval "$(...)"`,
//! `ulimit`, `umask`, `set -o`/`set +o`, `readonly`,
//! `declare`/`local`/`typeset`, `if`/`then`/`fi`,
//! `for`/`in`/`done`, `case`/`esac`, `trap`,
//! `command_not_found_handle`, `unalias`.
//!
//! ```rust
//! let z = "export EDITOR=vim\nalias ll='ls -la'\nshopt -s histappend\n";
//! let c = izanagi_kit::bashrc::Bashrc::parse(z.as_bytes()).unwrap();
//! assert_eq!(c.exports, 1);
//! ```

use crate::textutil::strip_bom;
/// bashrc census.
#[derive(Debug, Clone)]
pub struct Bashrc {
    /// `export` statements.
    pub exports: usize,
    /// `alias` statements.
    pub aliases: usize,
    /// `source`/`.` statements.
    pub sources: usize,
    /// `shopt` statements.
    pub shops: usize,
    /// `function name()`/`name() {`/`function name` definitions.
    pub functions: usize,
    /// `bind` statements.
    pub binds: usize,
    /// Other recognised statements (`eval`/`typeset`/`local`/`readonly`/`declare`/`if`/`then`/`elif`/`else`/`fi`/`for`/`in`/`do`/`done`/`while`/`until`/`case`/`esac`/`select`/`return`/`echo`/`printf`/`test`/`ulimit`/`umask`/`set`/`cd`/`trap`/`complete`/`compgen`/`compopt`/`command`/`builtin`/`type`/`hash`/`rehash`/`unalias`/`unfunction`/`stty`/`tty`/`history`/`fc`/`hup`/`disown`/`jobs`/`bg`/`fg`/`wait`/`times`/`exit`/`logout`/`suspend`/`clear`/`reset`/`tput`/`dircolors`/`eval`/`PROMPT_COMMAND`/`PS1`/`PS2`/`PS3`/`PS4`/`HIST*`).
    pub named: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const OTHER_HEADS: &[&str] = &[
    "eval",
    "typeset",
    "local",
    "readonly",
    "declare",
    "integer",
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
    "trap",
    "complete",
    "compgen",
    "compopt",
    "command",
    "builtin",
    "type",
    "hash",
    "rehash",
    "unalias",
    "unfunction",
    "stty",
    "tty",
    "history",
    "fc",
    "hup",
    "disown",
    "jobs",
    "bg",
    "fg",
    "wait",
    "times",
    "exit",
    "logout",
    "suspend",
    "clear",
    "reset",
    "tput",
    "dircolors",
    "[[",
    "[",
    "{",
    "}",
    "shift",
    "unset",
    "let",
    "exec",
];

/// Detect bashrc content.
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
        // `source <path>` may sit mid-line after `&&`/`;` (e.g. the common
        // `[ -n "$PS1" ] && source ~/.bash_profile` delegation idiom).
        let sources_path = {
            let toks: Vec<&str> = s
                .split(|c: char| c.is_whitespace() || c == ';' || c == '&')
                .filter(|t| !t.is_empty())
                .collect();
            toks.iter()
                .position(|t| *t == "source" || *t == ".")
                .is_some_and(|i| {
                    toks.get(i + 1)
                        .is_some_and(|n| n.starts_with(['~', '/', '.', '$']))
                })
        };
        if head == "export" || head == "alias" || head == "shopt" || head == "source" {
            hits += 1;
        }
        if s.starts_with("PS1=") || s.contains("$PS1") {
            hits += 1;
        }
        if s.contains("() {") {
            hits += 1;
        }
        if sources_path && head != "source" {
            hits += 1;
        }
    }
    hits >= 2
}

impl Bashrc {
    /// Census a bashrc buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            exports: 0,
            aliases: 0,
            sources: 0,
            shops: 0,
            functions: 0,
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
                "export" => c.exports += 1,
                "alias" => c.aliases += 1,
                "source" | "." => c.sources += 1,
                "shopt" => c.shops += 1,
                "bind" => c.binds += 1,
                "function" => c.functions += 1,
                _ => {
                    if s.contains("() {") || s.ends_with("()") {
                        c.functions += 1;
                    } else {
                        let assigned = s.contains('=') && {
                            let lhs = s.split('=').next().unwrap_or("").trim();
                            !lhs.is_empty()
                                && lhs.bytes().all(|x| {
                                    x.is_ascii_uppercase() || x == b'_' || x.is_ascii_digit()
                                })
                        };
                        if assigned || OTHER_HEADS.contains(&head) {
                            c.named += 1;
                        }
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
    fn detects_bashrc() {
        let b = b"export EDITOR=vim\nalias ll='ls -la'\n";
        assert!(detect(b));
    }

    #[test]
    fn detects_delegation_line() {
        // Real-world one-liner .bashrc that delegates to .bash_profile
        // behind an interactive-shell guard.
        assert!(detect(b"[ -n \"$PS1\" ] && source ~/.bash_profile;\n"));
        // Dot-builtin source counts too.
        assert!(detect(
            b"export EDITOR=vi\n[ -z \"$PS1\" ] && return\n. ~/.aliases\n"
        ));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_bashrc() {
        let b = concat!(
            "# bashrc\n",
            "export EDITOR=vim\n",
            "export VISUAL=vim\n",
            "export PATH=$HOME/bin:$PATH\n",
            "export LANG=en_US.UTF-8\n",
            "export PAGER=less\n",
            "PS1='[\\u@\\h \\W]\\$ '\n",
            "PS2='> '\n",
            "HISTCONTROL=ignoreboth\n",
            "HISTSIZE=10000\n",
            "HISTFILESIZE=20000\n",
            "HISTIGNORE='ls:cd:exit'\n",
            "PROMPT_COMMAND='history -a'\n",
            "alias ll='ls -la'\n",
            "alias la='ls -A'\n",
            "alias g=git\n",
            "alias gs='git status'\n",
            "alias ..='cd ..'\n",
            "alias grep='grep --color=auto'\n",
            "shopt -s histappend\n",
            "shopt -s checkwinsize\n",
            "shopt -s cmdhist\n",
            "shopt -s globstar\n",
            "shopt -s autocd\n",
            "shopt -u mailwarn\n",
            "source ~/.bash_aliases\n",
            "source /etc/bash_completion\n",
            ". ~/.cargo/env\n",
            "bind 'set completion-ignore-case on'\n",
            "bind 'set show-all-if-ambiguous on'\n",
            "bind '\"\\e[A\": history-search-backward'\n",
            "mkcd() { mkdir -p \"$1\" && cd \"$1\"; }\n",
            "function gc() { git commit -m \"$1\"; }\n",
            "function extract {\n",
            "  tar -xf \"$1\"\n",
            "}\n",
            "if [[ -f ~/.fzf.bash ]]; then\n",
            "  source ~/.fzf.bash\n",
            "fi\n",
            "eval \"$(direnv hook bash)\"\n",
            "ulimit -n 4096\n",
            "umask 022\n",
        );
        let c = Bashrc::parse(b.as_bytes()).unwrap();
        assert_eq!(c.exports, 5);
        assert_eq!(c.aliases, 6);
        assert_eq!(c.sources, 4);
        assert_eq!(c.shops, 6);
        assert_eq!(c.functions, 3);
        assert_eq!(c.binds, 3);
        assert!(c.named >= 12);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
