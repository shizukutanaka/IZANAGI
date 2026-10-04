//! qmake project file (`.pro`/`*.pri`) parser.
//!
//! Detects qmake projects by their characteristic assignments (`TEMPLATE`/
//! `TARGET`/`SOURCES`/`HEADERS`/`FORMS`/`RESOURCES`/`TRANSLATIONS`/`QT +=`/
//! `CONFIG`/`DEFINES`/`INCLUDEPATH`/`LIBS`/`DESTDIR`/`QMAKE_*`/`SUBDIRS`/
//! `PKGCONFIG`/`INSTALLS` …), `$$` variable references and scope conditions
//! (`win32:`/`unix:`/`!isEmpty()`/`contains()` …).
//!
//! ```
//! let b = b"TEMPLATE = app\nTARGET = myapp\nQT += core gui\nSOURCES += main.cpp\nHEADERS += main.h\n";
//! assert!(izanagi_kit::qmakepro::detect(b));
//! let c = izanagi_kit::qmakepro::Qmake::parse(b).unwrap();
//! assert!(c.keys >= 4);
//! ```

/// Parsed .pro summary.
#[derive(Debug, Clone)]
pub struct Qmake {
    /// Recognized assignment occurrences.
    pub keys: usize,
    /// Source/asset variables (`SOURCES`/`HEADERS`/`FORMS`/`RESOURCES`/`TRANSLATIONS`/`DISTFILES`/`OTHER_FILES`/`STATECHARTS`/`LEXSOURCES`/`YACCSOURCES`/`OBJECTIVE_SOURCES`/`CODECFORSRC`/`CODECFORTRC`/`RC_FILE`/`RC_ICONS`/`ICON`/`QMAKE_INFO_PLIST`/`DIST`/`INSTALLS`).
    pub source_keys: usize,
    /// Build variables (`TEMPLATE`/`TARGET`/`QT`/`CONFIG`/`DEFINES`/`INCLUDEPATH`/`DEPENDPATH`/`LIBS`/`DESTDIR`/`OBJECTS_DIR`/`MOC_DIR`/`RCC_DIR`/`UI_DIR`/`VERSION`/`SUBDIRS`/`PRE_TARGETDEPS`/`PKGCONFIG`/`QMAKE_*`/`REQUIRES`/`BUILD_PASS`/`EXCLUDE_FILES`/`HAVE*`).
    pub build_keys: usize,
    /// Function/scope calls (`include(`/`include()`/`!isEmpty(`/`isEmpty(`/`contains(`/`greaterThan(`/`lessThan(`/`equals(`/`message(`/`error(`/`warning(`/`system(`/`exists(`/`for(`/`else:`/`win32:`/`unix:`/`macx:`/`linux*:`/`debug:`/`release:`/`$$PWD`/`$$OUT_PWD`/`$$_PRO_FILE_`/`write_file(`/`files(`/`basename(`/`dirname(`/`clean_path(`/`shell_path(`/`quote(`/`join_path(`/`mkpath(`/`first(`/`last(`/`member(`/`num_add(`/`reverse(`/`replace(`/`split(`/`str_member(`/`take_first(`/`take_last(`/`unique(`/`sorted(`/`contains(`/`prompt(`/`log(`/`sprintf(`/`section(`/`cache(`/`load(`/`eval(`/`find(`/`enumerate(`/`export(`/`packagesExist(`/`prepareRecursiveTarget(`/`qtCompileTest(`/`qtHaveModule(`/`resolve_dependencies(`/`touch(`).
    pub scope_keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Source/asset variables.
const SOURCE_KEYS: &[&str] = &[
    "SOURCES",
    "HEADERS",
    "FORMS",
    "RESOURCES",
    "TRANSLATIONS",
    "DISTFILES",
    "OTHER_FILES",
    "STATECHARTS",
    "LEXSOURCES",
    "YACCSOURCES",
    "OBJECTIVE_SOURCES",
    "CODECFORSRC",
    "CODECFORTRC",
    "RC_FILE",
    "RC_ICONS",
    "ICON",
    "QMAKE_INFO_PLIST",
    "INSTALLS",
];

/// Build variables.
const BUILD_KEYS: &[&str] = &[
    "TEMPLATE",
    "TARGET",
    "QT",
    "CONFIG",
    "DEFINES",
    "INCLUDEPATH",
    "DEPENDPATH",
    "LIBS",
    "DESTDIR",
    "OBJECTS_DIR",
    "MOC_DIR",
    "RCC_DIR",
    "UI_DIR",
    "VERSION",
    "SUBDIRS",
    "PRE_TARGETDEPS",
    "PKGCONFIG",
    "QMAKE_CFLAGS",
    "QMAKE_CXXFLAGS",
    "QMAKE_LFLAGS",
    "QMAKE_CC",
    "QMAKE_CXX",
    "QMAKE_LINK",
    "QMAKE_LIBDIR",
    "QMAKE_OBJECTIVE_CFLAGS",
    "QMAKE_INCDIR",
    "QMAKE_CLEAN",
    "REQUIRES",
    "BUILD_PASS",
    "EXCLUDE_FILES",
];

/// Function/scope calls.
const SCOPE_KEYS: &[&str] = &[
    "include(",
    "!isEmpty(",
    "isEmpty(",
    "contains(",
    "greaterThan(",
    "lessThan(",
    "equals(",
    "message(",
    "error(",
    "warning(",
    "system(",
    "exists(",
    "for(",
    "else:",
    "win32:",
    "unix:",
    "macx:",
    "linux:",
    "debug:",
    "release:",
    "$$PWD",
    "$$OUT_PWD",
    "write_file(",
    "files(",
    "basename(",
    "dirname(",
    "clean_path(",
    "shell_path(",
    "quote(",
    "mkpath(",
    "first(",
    "last(",
    "member(",
    "num_add(",
    "reverse(",
    "replace(",
    "split(",
    "take_first(",
    "take_last(",
    "unique(",
    "sorted(",
    "prompt(",
    "log(",
    "sprintf(",
    "section(",
    "cache(",
    "load(",
    "eval(",
    "find(",
    "enumerate(",
    "export(",
    "packagesExist(",
    "qtCompileTest(",
    "qtHaveModule(",
    "touch(",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["TEMPLATE", "QT", "SOURCES", "qmake"];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "TEMPLATE",
    "TARGET",
    "SOURCES",
    "HEADERS",
    "QT",
    "CONFIG",
    "DEFINES",
    "INCLUDEPATH",
    "LIBS",
    "FORMS",
    "RESOURCES",
    "SUBDIRS",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(&format!("{k} ="))
        || t.contains(&format!("{k}="))
        || t.contains(&format!("{k} +="))
        || t.contains(&format!("{k}+="))
        || t.contains(&format!("{k} -="))
        || t.contains(&format!("{k}-="))
        || t.contains(&format!("{k} *="))
        || t.contains(&format!("{k}*="))
        || t.contains(&format!("{k} ~="))
        || t.contains(&format!("{k}~="))
}

/// Detect a qmake .pro/.pri file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Qmake {
    /// Count categories in a .pro file. Returns `None` when the input does
    /// not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            source_keys: 0,
            build_keys: 0,
            scope_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') {
                c.comments += 1;
            }
        }
        for k in SOURCE_KEYS {
            c.source_keys += t.matches(k).count();
        }
        for k in BUILD_KEYS {
            c.build_keys += t.matches(k).count();
        }
        for k in SCOPE_KEYS {
            c.scope_keys += t.matches(k).count();
        }
        c.keys = c.source_keys + c.build_keys + c.scope_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"# project\nTEMPLATE = app\nTARGET = myapp\nQT += core gui widgets\nCONFIG += c++17 warn_on\nDEFINES += APP_X\nINCLUDEPATH += include\nSOURCES += main.cpp util.cpp\nHEADERS += main.h\nFORMS += main.ui\nRESOURCES += res.qrc\nwin32:LIBS += -lws2_32\n!isEmpty(PREFIX):target.path = $$PREFIX/bin\nINSTALLS += target\n";
        assert!(detect(b));
        let c = Qmake::parse(b).unwrap();
        assert_eq!(c.comments, 1);
        assert!(c.source_keys >= 4);
        assert!(c.build_keys >= 5);
        assert!(c.scope_keys >= 3);
        assert!(c.keys >= 12);
    }

    #[test]
    fn rejects_random() {
        assert!(!detect(b"x = 1\nprint(x)\n"));
        assert!(Qmake::parse(b"a = b\n").is_none());
    }
}
