//! Emacs `init.el` / `.emacs` / `.spacemacs` census.
//!
//! S-expression statements: `(setq x v)`/`(setq-default)`,
//! `(require 'x)`, `(use-package x ...)`,
//! `(add-hook 'x-hook 'fn)`/`(remove-hook)`,
//! `(global-set-key (kbd "..") 'fn)`/`(define-key ...)`/
//! `(bind-key ...)`, `(custom-set-variables ...)`/
//! `(custom-set-faces ...)`, `(load-file "...")`/`(load ...)`,
//! `(defun name (args) ...)`, `(defvar/defcustom/defmacro)`,
//! `(package-initialize)`, `(add-to-list 'load-path ...)`,
//! `(dotspacemacs/init|user-init|user-config|layers)` for
//! Spacemacs, `;;`/`;;;` comments.
//!
//! ```rust
//! let e = "(setq inhibit-startup-screen t)\n(require 'use-package)\n(use-package magit :ensure t)\n";
//! let c = izanagi_kit::emacs::Emacs::parse(e.as_bytes()).unwrap();
//! assert_eq!(c.usepackages, 1);
//! ```

/// init.el census.
#[derive(Debug, Clone)]
pub struct Emacs {
    /// `(setq*` / `(setf` forms.
    pub setqs: usize,
    /// `(use-package` forms.
    pub usepackages: usize,
    /// `(require` forms.
    pub requires: usize,
    /// `(add-hook`/`(remove-hook` forms.
    pub hooks: usize,
    /// Key binding forms (`global-set-key`/`define-key`/`bind-key`/`general-define-key`).
    pub keys: usize,
    /// `(defun`/`(defvar`/`(defcustom`/`(defmacro`/`(defconst`/`(defalias`/`(cl-defun` forms.
    pub defuns: usize,
    /// Other recognised forms (`custom-set-*`/`load*`/`add-to-list`/`package-*`/`dotspacemacs-*`/`provide`/`eval-*`/`with-*`/`let*`/`if`/`progn`/`lambda`/`autoload`/`set-face-*`/`modify-*`).
    pub named: usize,
    /// `;` comment lines.
    pub comments: usize,
}

const SETQ_HEADS: &[&str] = &["(setq", "(setq-default", "(setq-local", "(setf", "(setq!"];

const HOOK_HEADS: &[&str] = &["(add-hook", "(remove-hook", "(add-transient-hook"];

const KEY_HEADS: &[&str] = &[
    "(global-set-key",
    "(define-key",
    "(bind-key",
    "(bind-keys",
    "(general-define-key",
    "(keymap-set",
    "(keymap-global-set",
    "(local-set-key",
    "(global-unset-key",
];

const DEF_HEADS: &[&str] = &[
    "(defun",
    "(defmacro",
    "(defvar",
    "(defcustom",
    "(defconst",
    "(defalias",
    "(defsubst",
    "(defadvice",
    "(defhydra",
    "(defmethod",
    "(cl-defun",
    "(cl-defmacro",
];

const OTHER_HEADS: &[&str] = &[
    "(custom-set-variables",
    "(custom-set-faces",
    "(custom-theme-set-faces",
    "(custom-theme-set-variables",
    "(load-file",
    "(load-library",
    "(load-theme",
    "(load",
    "(require-theme",
    "(add-to-list",
    "(package-initialize",
    "(package-install",
    "(package-refresh-contents",
    "(package-menu-execute",
    "(use-package-ensure-elpa",
    "(dotspacemacs",
    "(provide",
    "(eval-when-compile",
    "(eval-after-load",
    "(with-eval-after-load",
    "(with-library",
    "(let",
    "(let*",
    "(if",
    "(when",
    "(unless",
    "(progn",
    "(lambda",
    "(autoload",
    "(set-face-attribute",
    "(modify-frame-parameters",
    "(menu-bar-mode",
    "(tool-bar-mode",
    "(scroll-bar-mode",
    "(show-paren-mode",
    "(electric-pair-mode",
    "(global-auto-revert-mode",
    "(delete-selection-mode",
    "(recentf-mode",
    "(savehist-mode",
    "(save-place-mode",
    "(winner-mode",
    "(which-key-mode",
    "(global-company-mode",
    "(ivy-mode",
    "(helm-mode",
    "(counsel-mode",
    "(projectile-mode",
    "(flycheck-mode",
    "(yas-global-mode",
    "(org-babel-do-load-languages",
    "(server-start",
    "(prefer-coding-system",
    "(set-language-environment",
    "(set-default-coding-systems",
    "(set-terminal-coding-system",
    "(set-keyboard-coding-system",
    "(modify-syntax-entry",
    "(put",
    "(fset",
    "(global-display-line-numbers-mode",
    "(display-time-mode",
    "(column-number-mode",
    "(size-indication-mode",
    "(blink-cursor-mode",
    "(transient-mark-mode",
];

/// Detect init.el content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut hits = 0usize;
    for p in [
        "(setq",
        "(require",
        "(use-package",
        "(add-hook",
        "(defun",
        "(custom-set",
        "(dotspacemacs",
    ] {
        if t.contains(p) {
            hits += 1;
        }
    }
    hits >= 2
}

impl Emacs {
    /// Census an init.el buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            setqs: 0,
            usepackages: 0,
            requires: 0,
            hooks: 0,
            keys: 0,
            defuns: 0,
            named: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with(';') {
                c.comments += 1;
                continue;
            }
            if s.starts_with("(use-package") {
                c.usepackages += 1;
                continue;
            }
            if s.starts_with("(require") {
                c.requires += 1;
                continue;
            }
            if SETQ_HEADS.iter().any(|h| s.starts_with(h)) {
                c.setqs += 1;
                continue;
            }
            if HOOK_HEADS.iter().any(|h| s.starts_with(h)) {
                c.hooks += 1;
                continue;
            }
            if KEY_HEADS.iter().any(|h| s.starts_with(h)) {
                c.keys += 1;
                continue;
            }
            if DEF_HEADS.iter().any(|h| s.starts_with(h)) {
                c.defuns += 1;
                continue;
            }
            if OTHER_HEADS.iter().any(|h| s.starts_with(h)) {
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
    fn detects_init() {
        let b = b"(setq inhibit-startup-screen t)\n(require 'use-package)\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_init() {
        let b = concat!(
            ";;; init.el\n",
            "(setq inhibit-startup-screen t)\n",
            "(setq ring-bell-function 'ignore)\n",
            "(setq-default indent-tabs-mode nil)\n",
            "(setq tab-width 4)\n",
            "(require 'package)\n",
            "(require 'use-package)\n",
            "(package-initialize)\n",
            "(use-package magit :ensure t :bind ((\"C-x g\" . magit-status)))\n",
            "(use-package company :ensure t :hook (after-init . global-company-mode))\n",
            "(use-package ivy :ensure t :config (ivy-mode 1))\n",
            "(add-hook 'prog-mode-hook 'display-line-numbers-mode)\n",
            "(add-hook 'text-mode-hook 'flyspell-mode)\n",
            "(global-set-key (kbd \"C-c c\") 'compile)\n",
            "(define-key global-map (kbd \"C-x C-b\") 'ibuffer)\n",
            "(custom-set-variables '(custom-enabled-themes '(wombat)))\n",
            "(custom-set-faces '(default ((t (:height 120)))))\n",
            "(add-to-list 'load-path \"~/.emacs.d/lisp\")\n",
            "(load-file \"~/.emacs.d/local.el\")\n",
            "(defun my-cleanup ()\n",
            "  (interactive)\n",
            "  (delete-trailing-whitespace))\n",
            "(defvar my-cache-dir \"~/.cache\")\n",
            "(menu-bar-mode -1)\n",
            "(tool-bar-mode -1)\n",
            "(show-paren-mode 1)\n",
            "(recentf-mode 1)\n",
            "(server-start)\n",
        );
        let c = Emacs::parse(b.as_bytes()).unwrap();
        assert_eq!(c.setqs, 4);
        assert_eq!(c.usepackages, 3);
        assert_eq!(c.requires, 2);
        assert_eq!(c.hooks, 2);
        assert_eq!(c.keys, 2);
        assert_eq!(c.defuns, 2);
        assert!(c.named >= 10);
        assert_eq!(c.comments, 1);
    }
}
