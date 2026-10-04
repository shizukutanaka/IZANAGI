//! autoconf input (`configure.ac`/`configure.in`) parser.
//!
//! Detects autoconf files by their characteristic `AC_*`/`AM_*`/`LT_*`/
//! `PKG_*`/`AX_*` macro calls (`AC_INIT`/`AM_INIT_AUTOMAKE`/
//! `AC_PROG_CC`/`AC_CHECK_HEADERS`/`AC_CHECK_FUNCS`/`AC_CHECK_LIB`/
//! `AC_ARG_ENABLE`/`AC_ARG_WITH`/`AC_SUBST`/`AC_DEFINE`/`AC_CONFIG_FILES`/
//! `AC_OUTPUT`/`PKG_CHECK_MODULES`/`LT_INIT`/`AM_CONDITIONAL` …) and
//! counts occurrences by category.
//!
//! ```
//! let b = b"AC_INIT([app], [1.0])\nAM_INIT_AUTOMAKE\nAC_PROG_CC\nAC_CONFIG_HEADERS([config.h])\nAC_CONFIG_FILES([Makefile])\nAC_OUTPUT\n";
//! assert!(izanagi_kit::configureac::detect(b));
//! let c = izanagi_kit::configureac::Confac::parse(b).unwrap();
//! assert!(c.keys >= 5);
//! ```

/// Parsed configure.ac summary.
#[derive(Debug, Clone)]
pub struct Confac {
    /// Recognized macro occurrences.
    pub keys: usize,
    /// Setup macros (`AC_INIT`/`AC_PREREQ`/`AC_CONFIG_SRCDIR`/`AC_CONFIG_AUX_DIR`/`AC_CONFIG_MACRO_DIR`/`AC_CONFIG_HEADERS`/`AC_CONFIG_FILES`/`AC_CONFIG_LINKS`/`AC_CONFIG_COMMANDS`/`AC_CONFIG_SUBDIRS`/`AC_OUTPUT`/`AC_REVISION`/`AC_COPYRIGHT`/`AC_PRESERVE_HELP_ORDER`).
    pub setup_keys: usize,
    /// Program/compiler macros (`AC_PROG_CC`/`AC_PROG_CXX`/`AC_PROG_CPP`/`AC_PROG_INSTALL`/`AC_PROG_MAKE_SET`/`AC_PROG_RANLIB`/`AC_PROG_LIBTOOL`/`LT_INIT`/`AM_PROG_AR`/`AC_PROG_LEX`/`AC_PROG_YACC`/`AC_PROG_AWK`/`AC_PROG_SED`/`AC_PROG_GREP`/`AC_PROG_EGREP`/`AC_PROG_FGREP`/`AC_PROG_LN_S`/`AC_PROG_MKDIR_P`/`AC_PROG_FGREP`/`AM_PATH_*`/`PKG_PROG_PKG_CONFIG`/`AX_*`/`AC_CANONICAL_*`/`AC_PROG_*` family).
    pub prog_keys: usize,
    /// Check macros (`AC_CHECK_HEADERS`/`AC_CHECK_HEADER`/`AC_CHECK_FUNCS`/`AC_CHECK_FUNC`/`AC_CHECK_LIB`/`AC_CHECK_TYPES`/`AC_CHECK_TYPE`/`AC_CHECK_DECLS`/`AC_CHECK_DECL`/`AC_CHECK_MEMBER`/`AC_CHECK_MEMBERS`/`AC_CHECK_SIZEOF`/`AC_CHECK_PROG`/`AC_CHECK_PROGS`/`AC_CHECK_TOOL`/`AC_CHECK_TOOLS`/`AC_PATH_PROG`/`AC_PATH_PROGS`/`AC_SEARCH_LIBS`/`AC_FUNC_*`/`AC_HEADER_*`/`AC_TYPE_*`/`AC_COMPILE_IFELSE`/`AC_LINK_IFELSE`/`AC_RUN_IFELSE`/`AC_PREPROC_IFELSE`/`AC_TRY_*`/`AC_LANG*`/`PKG_CHECK_MODULES`/`PKG_CHECK_*`).
    pub check_keys: usize,
    /// Output macros (`AC_SUBST`/`AC_DEFINE`/`AC_DEFINE_UNQUOTED`/`AC_ARG_ENABLE`/`AC_ARG_WITH`/`AC_ARG_VAR`/`AC_MSG_*`/`AM_INIT_AUTOMAKE`/`AM_CONDITIONAL`/`AM_SILENT_RULES`/`AM_GNU_GETTEXT`/`AM_MAINTAINER_MODE`/`PKG_CHECK_MODULES`/`AS_*`/`AH_*`/`m4_*`/`dnl`).
    pub out_keys: usize,
    /// `dnl`/`#` comment lines.
    pub comments: usize,
}

/// Setup macros.
const SETUP_KEYS: &[&str] = &[
    "AC_INIT",
    "AC_PREREQ",
    "AC_CONFIG_SRCDIR",
    "AC_CONFIG_AUX_DIR",
    "AC_CONFIG_MACRO_DIR",
    "AC_CONFIG_HEADERS",
    "AC_CONFIG_FILES",
    "AC_CONFIG_LINKS",
    "AC_CONFIG_COMMANDS",
    "AC_CONFIG_SUBDIRS",
    "AC_OUTPUT",
    "AC_REVISION",
    "AC_COPYRIGHT",
];

/// Program/compiler macros.
const PROG_KEYS: &[&str] = &[
    "AC_PROG_CC",
    "AC_PROG_CXX",
    "AC_PROG_CPP",
    "AC_PROG_INSTALL",
    "AC_PROG_MAKE_SET",
    "AC_PROG_RANLIB",
    "AC_PROG_LIBTOOL",
    "LT_INIT",
    "AM_PROG_AR",
    "AC_PROG_LEX",
    "AC_PROG_YACC",
    "AC_PROG_AWK",
    "AC_PROG_SED",
    "AC_PROG_GREP",
    "AC_PROG_EGREP",
    "AC_PROG_FGREP",
    "AC_PROG_LN_S",
    "AC_PROG_MKDIR_P",
    "AM_PATH_",
    "PKG_PROG_PKG_CONFIG",
    "AC_CANONICAL_",
];

/// Check macros.
const CHECK_KEYS: &[&str] = &[
    "AC_CHECK_HEADERS",
    "AC_CHECK_HEADER",
    "AC_CHECK_FUNCS",
    "AC_CHECK_FUNC",
    "AC_CHECK_LIB",
    "AC_CHECK_TYPES",
    "AC_CHECK_TYPE",
    "AC_CHECK_DECLS",
    "AC_CHECK_DECL",
    "AC_CHECK_MEMBER",
    "AC_CHECK_MEMBERS",
    "AC_CHECK_SIZEOF",
    "AC_CHECK_PROG",
    "AC_CHECK_PROGS",
    "AC_CHECK_TOOL",
    "AC_CHECK_TOOLS",
    "AC_PATH_PROG",
    "AC_PATH_PROGS",
    "AC_SEARCH_LIBS",
    "AC_FUNC_",
    "AC_HEADER_",
    "AC_TYPE_",
    "AC_COMPILE_IFELSE",
    "AC_LINK_IFELSE",
    "AC_RUN_IFELSE",
    "AC_PREPROC_IFELSE",
    "AC_TRY_",
    "AC_LANG",
    "PKG_CHECK_MODULES",
];

/// Output/automake macros.
const OUT_KEYS: &[&str] = &[
    "AC_SUBST",
    "AC_DEFINE",
    "AC_DEFINE_UNQUOTED",
    "AC_ARG_ENABLE",
    "AC_ARG_WITH",
    "AC_ARG_VAR",
    "AC_MSG_",
    "AM_INIT_AUTOMAKE",
    "AM_CONDITIONAL",
    "AM_SILENT_RULES",
    "AM_GNU_GETTEXT",
    "AM_MAINTAINER_MODE",
    "AS_",
    "AH_",
    "m4_",
    "dnl",
];

/// Distinctive macros for detection.
const ALL: &[&str] = &[
    "AC_INIT",
    "AM_INIT_AUTOMAKE",
    "AC_CONFIG_SRCDIR",
    "AC_CONFIG_HEADERS",
    "AC_CONFIG_FILES",
    "AC_PROG_CC",
    "AC_CHECK_HEADERS",
    "AC_CHECK_FUNCS",
    "AC_CHECK_LIB",
    "AC_ARG_ENABLE",
    "AC_SUBST",
    "AC_OUTPUT",
    "LT_INIT",
    "PKG_CHECK_MODULES",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a configure.ac file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    hits >= 2
}

impl Confac {
    /// Count macro categories in a configure.ac. Returns `None` when the
    /// input does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            setup_keys: 0,
            prog_keys: 0,
            check_keys: 0,
            out_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with("dnl") || tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in SETUP_KEYS {
            c.setup_keys += t.matches(k).count();
        }
        for k in PROG_KEYS {
            c.prog_keys += t.matches(k).count();
        }
        for k in CHECK_KEYS {
            c.check_keys += t.matches(k).count();
        }
        for k in OUT_KEYS {
            c.out_keys += t.matches(k).count();
        }
        c.keys = c.setup_keys + c.prog_keys + c.check_keys + c.out_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"dnl configure\nAC_INIT([app], [1.0], [bugs@x])\nAC_CONFIG_SRCDIR([src/main.c])\nAC_CONFIG_HEADERS([config.h])\nAM_INIT_AUTOMAKE([foreign])\nAC_PROG_CC\nAC_PROG_INSTALL\nAC_CHECK_HEADERS([stdlib.h string.h])\nAC_CHECK_FUNCS([memset])\nAC_CHECK_LIB([z], [deflate])\nAC_ARG_ENABLE([debug], [AS_HELP_STRING([--enable-debug], [debug])])\nPKG_CHECK_MODULES([ZLIB], [zlib])\nAM_CONDITIONAL([DEBUG], [test x$debug = xyes])\nAC_CONFIG_FILES([Makefile src/Makefile])\nAC_OUTPUT\n";
        assert!(detect(b));
        let c = Confac::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.setup_keys >= 3);
        assert!(c.prog_keys >= 2);
        assert!(c.check_keys >= 3);
        assert!(c.out_keys >= 3);
        assert!(c.keys >= 11);
    }

    #[test]
    fn rejects_random_sh() {
        assert!(!detect(b"#!/bin/sh\necho hi\n"));
        assert!(Confac::parse(b"a = b\n").is_none());
    }
}
