//! `.ideavimrc` census.
//!
//! Vim-like syntax plus IdeaVim extensions: `set opt`
//! (`set surround`/`set highlightedyank`/`set NERDTree`/
//! `set easyemotion` plugin enables), `let g:ideavim*`,
//! `map`/`nmap`/`imap`/`vmap`/`xmap`/`nnoremap`/… +
//! `<Action>(IdeaVimAction)` RHS references, `sethandler`
//! (IdeaVim keyboard handler), `source`, `command`, `function`.
//!
//! ```rust
//! let i = "set number\nnnoremap <leader>f <Action>(GotoFile)\nsethandler <C-S> n-v-i a:vim\n";
//! let c = izanagi_kit::ideavim::Ideavim::parse(i.as_bytes()).unwrap();
//! assert_eq!(c.actions, 1);
//! ```

/// ideavimrc census.
#[derive(Debug, Clone)]
pub struct Ideavim {
    /// `set` statements.
    pub sets: usize,
    /// `*map`/`*noremap` statements.
    pub maps: usize,
    /// `let` statements.
    pub lets: usize,
    /// `sethandler` statements.
    pub handlers: usize,
    /// `<Action>(` references.
    pub actions: usize,
    /// `"` comment lines.
    pub comments: usize,
}

const MAP_HEADS: &[&str] = &[
    "map", "nmap", "imap", "vmap", "xmap", "cmap", "omap", "tmap", "noremap", "nnoremap",
    "inoremap", "vnoremap", "xnoremap", "cnoremap", "onoremap", "tnoremap",
];

/// Detect ideavimrc content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    t.contains("<Action>(")
        || t.contains("ideavim")
        || (t.contains("sethandler") && t.contains("vim"))
        || (t.contains("set ") && t.contains("<Action>"))
}

impl Ideavim {
    /// Census an ideavimrc buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sets: 0,
            maps: 0,
            lets: 0,
            handlers: 0,
            actions: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('"') {
                c.comments += 1;
                continue;
            }
            if s.contains("<Action>") {
                c.actions += 1;
            }
            if s.starts_with("sethandler") {
                c.handlers += 1;
                continue;
            }
            if s.starts_with("set ") || s.starts_with("setlocal ") || s.starts_with("setglobal ") {
                c.sets += 1;
                continue;
            }
            if s.starts_with("let ") {
                c.lets += 1;
                continue;
            }
            let head = s.split(' ').next().unwrap_or("");
            if MAP_HEADS.contains(&head) {
                c.maps += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_ideavim() {
        let b = b"nnoremap <leader>f <Action>(GotoFile)\nset number\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_ideavim() {
        let b = concat!(
            "\" ideavimrc\n",
            "set number\n",
            "set relativenumber\n",
            "set scrolloff=8\n",
            "set surround\n",
            "set highlightedyank\n",
            "set ideajoin\n",
            "let g:ideavim_sneak = 1\n",
            "let mapleader = \" \"\n",
            "nnoremap <leader>f <Action>(GotoFile)\n",
            "nnoremap <leader>r <Action>(RenameElement)\n",
            "nnoremap gd <Action>(GotoDeclaration)\n",
            "nnoremap gi <Action>(GotoImplementation)\n",
            "inoremap jk <Esc>\n",
            "vnoremap <leader>c <Action>(CommentByLineComment)\n",
            "map <leader>a <Action>(SelectIn)\n",
            "sethandler <C-a> a:vim\n",
            "sethandler <C-x> a:vim\n",
            "sethandler <C-S> n-v-i a:ide\n",
        );
        let c = Ideavim::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sets, 6);
        assert_eq!(c.maps, 7);
        assert_eq!(c.lets, 2);
        assert_eq!(c.handlers, 3);
        assert_eq!(c.actions, 6);
        assert_eq!(c.comments, 1);
    }
}
