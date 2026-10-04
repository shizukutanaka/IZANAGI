//! automake input (`Makefile.am`) parser.
//!
//! Detects automake files by their characteristic primaries (`bin_PROGRAMS`/
//! `lib_LTLIBRARIES`/`noinst_LIBRARIES`/`include_HEADERS`/`noinst_HEADERS`/
//! `dist_*`/`nodist_*`/`*_SOURCES`/`_LDADD`/`_LIBADD`/`AM_*FLAGS`/
//! `SUBDIRS`/`EXTRA_DIST`/`TESTS`/`man_MANS`/`pkgdata_DATA`/`AUTOMAKE_OPTIONS`
//! …) and counts occurrences by category.
//!
//! ```
//! let b = b"bin_PROGRAMS = app\napp_SOURCES = main.c util.c\napp_LDADD = libx.a\nAM_CPPFLAGS = -Iinclude\n";
//! assert!(izanagi_kit::amfile::detect(b));
//! let c = izanagi_kit::amfile::Amfile::parse(b).unwrap();
//! assert!(c.keys >= 3);
//! ```

/// Parsed Makefile.am summary.
#[derive(Debug, Clone)]
pub struct Amfile {
    /// Recognized primary/variable occurrences.
    pub keys: usize,
    /// Product primaries (`bin_PROGRAMS`/`sbin_PROGRAMS`/`noinst_PROGRAMS`/`check_PROGRAMS`/`lib_LTLIBRARIES`/`noinst_LTLIBRARIES`/`lib_LIBRARIES`/`noinst_LIBRARIES`/`check_LIBRARIES`/`python_PYTHON`/`noinst_PYTHON`/`java_JAVA`/`lisp_LISP`/`scripts_SCRIPTS`/`bin_SCRIPTS`/`sbin_SCRIPTS`/`noinst_SCRIPTS`/`info_TEXINFOS`/`man_MANS`/`dist_man_MANS`/`pkgdata_DATA`/`dist_pkgdata_DATA`/`aclocal_DATA`/`pkgconfig_DATA`/`doc_DATA`/`include_HEADERS`/`nobase_include_HEADERS`/`pkginclude_HEADERS`/`noinst_HEADERS`).
    pub primary_keys: usize,
    /// Per-target variables (`*_SOURCES`/`*_LDADD`/`*_LDFLAGS`/`*_LIBADD`/`*_CPPFLAGS`/`*_CFLAGS`/`*_CXXFLAGS`/`*_DEPENDENCIES`/`*_LINK`/`*_SHORTNAME`/`*_VPATH`/`*_EXTRA` pattern — counted via `_SOURCES`/`_LDADD`/`_LIBADD`/`_DEPENDENCIES` suffixes).
    pub target_keys: usize,
    /// Flags & file-list variables (`AM_CPPFLAGS`/`AM_CFLAGS`/`AM_CXXFLAGS`/`AM_LDFLAGS`/`AM_LFLAGS`/`AM_YFLAGS`/`AM_JAVACFLAGS`/`AM_VALAFLAGS`/`AUTOMAKE_OPTIONS`/`ACLOCAL_AMFLAGS`/`SUBDIRS`/`DIST_SUBDIRS`/`EXTRA_DIST`/`CLEANFILES`/`DISTCLEANFILES`/`MAINTAINERCLEANFILES`/`MOSTLYCLEANFILES`/`BUILT_SOURCES`/`TESTS`/`XFAIL_TESTS`/`LOG_COMPILER`/`TEST_EXTENSIONS`/`check_`/`noinst_`/`dist_`/`nodist_`/`include `).
    pub flag_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Product primaries.
const PRIMARY_KEYS: &[&str] = &[
    "bin_PROGRAMS",
    "sbin_PROGRAMS",
    "noinst_PROGRAMS",
    "check_PROGRAMS",
    "lib_LTLIBRARIES",
    "noinst_LTLIBRARIES",
    "lib_LIBRARIES",
    "noinst_LIBRARIES",
    "check_LIBRARIES",
    "python_PYTHON",
    "noinst_PYTHON",
    "java_JAVA",
    "lisp_LISP",
    "scripts_SCRIPTS",
    "bin_SCRIPTS",
    "sbin_SCRIPTS",
    "noinst_SCRIPTS",
    "info_TEXINFOS",
    "man_MANS",
    "dist_man_MANS",
    "pkgdata_DATA",
    "dist_pkgdata_DATA",
    "aclocal_DATA",
    "pkgconfig_DATA",
    "doc_DATA",
    "include_HEADERS",
    "nobase_include_HEADERS",
    "pkginclude_HEADERS",
    "noinst_HEADERS",
];

/// Per-target suffixes.
const TARGET_SUFFIXES: &[&str] = &[
    "_SOURCES",
    "_LDADD",
    "_LDFLAGS",
    "_LIBADD",
    "_CPPFLAGS",
    "_CFLAGS",
    "_CXXFLAGS",
    "_DEPENDENCIES",
    "_LINK",
    "_SHORTNAME",
    "_VPATH",
    "_EXTRA",
];

/// Flag/file-list variables.
const FLAG_KEYS: &[&str] = &[
    "AM_CPPFLAGS",
    "AM_CFLAGS",
    "AM_CXXFLAGS",
    "AM_LDFLAGS",
    "AM_LFLAGS",
    "AM_YFLAGS",
    "AM_JAVACFLAGS",
    "AM_VALAFLAGS",
    "AUTOMAKE_OPTIONS",
    "ACLOCAL_AMFLAGS",
    "SUBDIRS",
    "DIST_SUBDIRS",
    "EXTRA_DIST",
    "CLEANFILES",
    "DISTCLEANFILES",
    "MAINTAINERCLEANFILES",
    "MOSTLYCLEANFILES",
    "BUILT_SOURCES",
    "TESTS",
    "XFAIL_TESTS",
    "LOG_COMPILER",
    "TEST_EXTENSIONS",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["PROGRAMS", "SOURCES", "AM_", "SUBDIRS"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "bin_PROGRAMS",
    "noinst_PROGRAMS",
    "_SOURCES",
    "_LDADD",
    "AM_CPPFLAGS",
    "SUBDIRS",
    "EXTRA_DIST",
    "include_HEADERS",
    "noinst_HEADERS",
    "lib_LTLIBRARIES",
    "AUTOMAKE_OPTIONS",
    "man_MANS",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect a Makefile.am file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Amfile {
    /// Count categories in a Makefile.am. Returns `None` when the input
    /// does not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            primary_keys: 0,
            target_keys: 0,
            flag_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in PRIMARY_KEYS {
            c.primary_keys += t.matches(k).count();
        }
        for k in TARGET_SUFFIXES {
            c.target_keys += t.matches(k).count();
        }
        for k in FLAG_KEYS {
            c.flag_keys += t.matches(k).count();
        }
        c.keys = c.primary_keys + c.target_keys + c.flag_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"# makefile.am\nAUTOMAKE_OPTIONS = foreign\nSUBDIRS = src tests\nbin_PROGRAMS = app\napp_SOURCES = main.c util.c\napp_CPPFLAGS = -I$(top_srcdir)/include\napp_LDADD = $(top_builddir)/lib/libx.la\nAM_CPPFLAGS = -Wall\nnoinst_HEADERS = priv.h\nEXTRA_DIST = autogen.sh\nTESTS = test1.sh\n";
        assert!(detect(b));
        let c = Amfile::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.primary_keys >= 2);
        assert!(c.target_keys >= 3);
        assert!(c.flag_keys >= 3);
        assert!(c.keys >= 8);
    }

    #[test]
    fn rejects_makefile() {
        assert!(!detect(b"all: app\napp: main.o\n\t$(CC) -o app main.o\n"));
        assert!(Amfile::parse(b"a = b\n").is_none());
    }
}
