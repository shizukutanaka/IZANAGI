//! Dune `dune`/`dune-project` census (OCaml).
//!
//! S-expression stanzas: `(lang dune 3.x)` (dune-project only),
//! `(name x)`, `(executable ...)`, `(executables ...)`,
//! `(library ...)`, `(public_name x)`, `(libraries ...)`,
//! `(modules ...)`, `(deps ...)`, `(preprocess (pps ...))`,
//! `(rule ...)`, `(target x)`, `(action (run ...))`,
//! `(env ...)`, `(ocamllex x)`, `(ocamlyacc x)`, `(menhir ...)`,
//! `(cinaps ...)`, `(install ...)`, `(dirs ...)`,
//! `(data_only_dirs ...)`, `(ignored_subdirs ...)`,
//! `(vendored_dirs ...)`, `(subdir x ...)`, `(copy_files ...)`,
//! `(alias ...)`, `(test ...)`, `(tests ...)`, `(toplevel ...)`,
//! `(documentation ...)`, `(foreign_stubs ...)`,
//! `(foreign_archives ...)`, `(include_subdirs ...)`,
//! `(package ...)`, `(authors ...)`, `(maintainers ...)`,
//! `(license ...)`, `(homepage ...)`, `(bug_reports ...)`,
//! `(source (uri ...))`, `(generate_opam_files)`, `(explicit_js_mode)`,
//! `(js_of_ocaml ...)`, `(melange.emit ...)`, `(cram ...)`,
//! `(mdx ...)`, `(coq.theory ...)`, `(coq.extraction ...)`,
//! `(warnings ...)`, `(flags ...)`, `(ocamlc_flags ...)`,
//! `(ocamlopt_flags ...)`, `(root_module ...)`, `(modules_without_implementation ...)`,
//! `(wrapped ...)`, `(optional)`, `(enabled_if ...)`,
//! `(only_names ...)`, `(modes ...)`, `(kind x)`, `(ppx_runtime_libraries ...)`.
//!
//! ```rust
//! let k = b"(executable\n (name main)\n (libraries unix threads)\n (modules main))\n(rule\n (target out)\n (action (run x)))\n";
//! assert!(izanagi_kit::dune::detect(k));
//! ```

/// dune file census.
#[derive(Debug, Clone)]
pub struct Dune {
    /// `(stanza` openers.
    pub stanzas: usize,
    /// `)` closers.
    pub closers: usize,
    /// recognised stanza names present.
    pub keys: usize,
    /// `;`/`#` comment lines.
    pub comments: usize,
}

const STANZAS: &[&str] = &[
    "lang dune",
    "executable",
    "executables",
    "library",
    "public_name",
    "libraries",
    "modules",
    "preprocess",
    "rule",
    "target",
    "action",
    "env",
    "ocamllex",
    "ocamlyacc",
    "menhir",
    "cinaps",
    "install",
    "dirs",
    "data_only_dirs",
    "ignored_subdirs",
    "vendored_dirs",
    "subdir",
    "copy_files",
    "alias",
    "test",
    "tests",
    "toplevel",
    "documentation",
    "foreign_stubs",
    "foreign_archives",
    "include_subdirs",
    "package",
    "authors",
    "maintainers",
    "license",
    "homepage",
    "bug_reports",
    "source",
    "generate_opam_files",
    "explicit_js_mode",
    "js_of_ocaml",
    "melange.emit",
    "cram",
    "mdx",
    "coq.theory",
    "coq.extraction",
    "warnings",
    "flags",
    "ocamlc_flags",
    "ocamlopt_flags",
    "root_module",
    "modules_without_implementation",
    "wrapped",
    "optional",
    "enabled_if",
    "only_names",
    "modes",
    "kind",
    "ppx_runtime_libraries",
    "name",
    "deps",
    "using",
    "opam_file_location",
    "implicit_transitive_deps",
    "dialect",
    "external_library_dependencies",
    "dune-project",
    "version",
    "subst",
    "formatting",
    "dune-workspace",
    "context",
    "profile",
    "workspace",
    "lock",
    "pin",
    "repository",
];

fn stanza_name(line: &str) -> Option<&str> {
    let s = line.trim_start();
    if s.is_empty() || s.starts_with(';') || s.starts_with('#') || s.starts_with(";;") {
        return None;
    }
    let rest = s.strip_prefix('(')?;
    // stanza name = first token after `(`
    let end = rest.find([' ', ')', '\t', '\n']).unwrap_or(rest.len());
    let name = &rest[..end];
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Detect a `dune`/`dune-project` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `(lang dune`/`(libraries`/`(modules`/`(executable`/
    // `(preprocess`/`(public_name` are dune-exclusive stanzas.
    let mut n = 0usize;
    for line in t.lines() {
        if let Some(s) = stanza_name(line) {
            if STANZAS.contains(&s) {
                n += 1;
            }
        }
    }
    n >= 3
}

impl Dune {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            stanzas: 0,
            closers: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with(';') || s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if let Some(st) = stanza_name(line) {
                c.stanzas += 1;
                if STANZAS.contains(&st) {
                    c.keys += 1;
                }
            }
            if s == ")" || s.starts_with("))") {
                c.closers += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"(executable\n (name main)\n (libraries unix threads)\n (modules main))\n(rule\n (target out)\n (action (run x)))\n";
        assert!(detect(b));
        let c = Dune::parse(b).unwrap();
        assert!(c.keys >= 5);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"(foo x)\n(bar y)\n"));
        assert!(!detect(
            b"; (executable (name x))\n; (libraries y)\n; (modules z)\n(a b)\n"
        ));
    }
}
