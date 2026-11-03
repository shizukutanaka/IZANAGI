//! `.vimrc` / `init.vim` census.
//!
//! Vimscript statements: `set opt`/`set noopt`/`set opt=val`,
//! `let g:x = v`/`let b:`/`let s:`/`let @x`, `map`/`nmap`/
//! `imap`/`vmap`/`xmap`/`cmap`/`omap`/`tmap` + `noremap`
//! variants (`nnoremap`, `inoremap`, `vnoremap`, `xnoremap`,
//! `cnoremap`, `onoremap`, `tnoremap`, `noremap`),
//! `autocmd`/`au`, `command[!]`, `function[!]`, `call`,
//! `colorscheme`, `syntax`/`filetype`/`highlight`/`hi`,
//! `Plug`/`plugin`, `if`/`endif`, `silent`/`execute`,
//! `source`, `packadd`, `augroup`, `mapleader`.
//!
//! ```rust
//! let v = "set number\nset tabstop=4\nnnoremap <leader>w :w<CR>\nlet g:mapleader = \" \"\n";
//! let c = izanagi_kit::vimrc::Vimrc::parse(v.as_bytes()).unwrap();
//! assert_eq!(c.sets, 2);
//! assert_eq!(c.maps, 1);
//! ```

use crate::textutil::strip_bom;
/// vimrc census.
#[derive(Debug, Clone)]
pub struct Vimrc {
    /// `set` statements.
    pub sets: usize,
    /// `*map`/`*noremap` statements.
    pub maps: usize,
    /// `let` statements.
    pub lets: usize,
    /// `autocmd`/`au` statements.
    pub autocmds: usize,
    /// Other recognised statements (`function`/`command`/`call`/`colorscheme`/`syntax`/`filetype`/`highlight`/`Plug`/`source`/`if`/`augroup`/`packadd`/`execute`/`silent`).
    pub named: usize,
    /// `"` comment lines.
    pub comments: usize,
}

const MAP_HEADS: &[&str] = &[
    "map", "nmap", "imap", "vmap", "xmap", "cmap", "omap", "tmap", "noremap", "nnoremap",
    "inoremap", "vnoremap", "xnoremap", "cnoremap", "onoremap", "tnoremap", "unmap", "nunmap",
    "map!", "noremap!", "lmap", "smap",
];

const OTHER_HEADS: &[&str] = &[
    "function",
    "function!",
    "endfunction",
    "command",
    "command!",
    "call",
    "colorscheme",
    "syntax",
    "filetype",
    "highlight",
    "hi",
    "Plug",
    "plug",
    "plugin",
    "source",
    "if",
    "endif",
    "else",
    "elseif",
    "while",
    "endwhile",
    "for",
    "endfor",
    "try",
    "catch",
    "finally",
    "endtry",
    "return",
    "augroup",
    "packadd",
    "execute",
    "silent",
    "normal",
    "echom",
    "echo",
    "unlet",
    "cd",
    "lcd",
    "cwd",
    "doautocmd",
    "runtime",
    "finish",
];

/// `true` when every argument of a `set` line is a valid Vim option
/// argument: `opt`, `noopt`, `invopt`, `opt?` or `opt` joined to a
/// value by `=`/`+=`/`-=`/`^=`. tmux `set -g`, slrn
/// `set username "joe"` and ipxe `set base <url>` do not qualify.
fn set_args_vim_shaped(line: &str) -> bool {
    let mut args = line.split_whitespace().skip(1).peekable();
    if args.peek().is_none() {
        return false;
    }
    args.all(|a| {
        let a = a
            .strip_prefix("no")
            .or_else(|| a.strip_prefix("inv"))
            .unwrap_or(a);
        // `opt+=v`/`opt-=v`/`opt^=v`: strip the operator's sign char
        let head = a.split(['=', '?']).next().unwrap_or("");
        let name = head.trim_end_matches(['+', '-', '^']);
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            && name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
    })
}

/// Detect vimrc content.
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
        if (head == "set" && set_args_vim_shaped(s)) || head == "let" || MAP_HEADS.contains(&head) {
            hits += 1;
        }
    }
    hits >= 2
}

impl Vimrc {
    /// Census a vimrc buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        let mut c = Self {
            sets: 0,
            maps: 0,
            lets: 0,
            autocmds: 0,
            named: 0,
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
            if s.starts_with("set ")
                || s == "set"
                || s.starts_with("setlocal ")
                || s.starts_with("setglobal ")
            {
                c.sets += 1;
                continue;
            }
            if s.starts_with("let ") || s.starts_with("unlet ") {
                c.lets += 1;
                continue;
            }
            if s.starts_with("autocmd ")
                || s.starts_with("au ")
                || s.starts_with("autocmd!")
                || s == "autocmd"
            {
                c.autocmds += 1;
                continue;
            }
            let head = s.split([' ', '\t', '!']).next().unwrap_or("");
            if MAP_HEADS.contains(&head) {
                c.maps += 1;
                continue;
            }
            if OTHER_HEADS.contains(&head) || s.starts_with("Plug ") {
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
    fn detects_vimrc() {
        let b = b"set number\nlet g:x = 1\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
        // tmux/slrn/ipxe `set` args are not Vim option arguments
        assert!(!detect(
            b"set -g prefix C-a\nset -g mouse on\nsetw -g mode-keys vi\n"
        ));
        assert!(!detect(
            b"set username \"joe\"\nset hostname \"example.com\"\n"
        ));
        assert!(!detect(b"set base http://x/img\nset boot-url http://x\n"));
        assert!(!detect(
            b"set dotenv-load\nset shell := [\"bash\", \"-c\"]\n"
        ));
        // `opt+=v` / `noopt` / `invopt` are still Vim
        assert!(detect(b"set dir+=/tmp\nset invnumber\nset nonumber\n"));
    }

    #[test]
    fn parses_vimrc() {
        let b = concat!(
            "\" vimrc\n",
            "set nocompatible\n",
            "set number\n",
            "set tabstop=4 shiftwidth=4\n",
            "set expandtab\n",
            "set clipboard=unnamed\n",
            "setlocal wrap\n",
            "syntax on\n",
            "filetype plugin indent on\n",
            "colorscheme desert\n",
            "let g:mapleader = \" \"\n",
            "let g:netrw_banner = 0\n",
            "nnoremap <leader>w :w<CR>\n",
            "inoremap jk <Esc>\n",
            "vnoremap < <gv\n",
            "noremap ; :\n",
            "autocmd BufWritePre * :%s/\\s\\+$//e\n",
            "au FileType rust setlocal ts=4\n",
            "command! W w\n",
            "function! Trim()\n",
            "  call setline(1, 'x')\n",
            "endfunction\n",
            "call plug#begin()\n",
            "Plug 'tpope/vim-sensible'\n",
            "Plug 'junegunn/fzf'\n",
            "call plug#end()\n",
            "if has('nvim')\n",
            "endif\n",
            "source ~/.vim/local.vim\n",
        );
        let c = Vimrc::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sets, 6);
        assert_eq!(c.maps, 4);
        assert_eq!(c.lets, 2);
        assert_eq!(c.autocmds, 2);
        assert!(c.named >= 13);
        assert_eq!(c.comments, 1);
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
