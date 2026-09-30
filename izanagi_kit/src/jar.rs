//! Java `.jar` / `.war` / `.apk`-adjacent archive: a ZIP that must
//! contain `META-INF/MANIFEST.MF`. Manifest continuation lines (a line
//! starting with a space extends the previous value per RFC 5322-style
//! folding) are unfolded before `Key: value` pairs are read.
//!
//! ```
//! let mut zw = izanagi_kit::zip::ZipWriter::new();
//! zw.add_stored(
//!     "META-INF/MANIFEST.MF",
//!     b"Manifest-Version: 1\x2e0\r\nMain-Class: com.example.Main\r\nImplementation-Version: 2\x2e5\r\n\r\n",
//! );
//! zw.add_stored("com/example/Main.class", b"\xca\xfe\xba\xbe");
//! zw.add_stored("com/example/Util.class", b"\xca\xfe\xba\xbe");
//! let j = izanagi_kit::jar::parse(&zw.finish()).unwrap();
//! assert_eq!(j.main_class.as_deref(), Some("com.example.Main"));
//! assert_eq!(j.class_files, 2);
//! ```

use std::string::String;
use std::vec::Vec;

/// A detected JAR package.
#[derive(Clone, Debug)]
pub struct Jar {
    /// Entry names in the archive.
    pub entries: Vec<String>,
    /// `*.class` member count (excluding `module-info.class` is NOT
    /// applied — it is a class file too).
    pub class_files: usize,
    /// `Main-Class` manifest attribute.
    pub main_class: Option<String>,
    /// `Implementation-Version` manifest attribute.
    pub implementation_version: Option<String>,
    /// `Automatic-Module-Name` manifest attribute (JPMS on the module
    /// path for non-modular jars).
    pub automatic_module: Option<String>,
    /// `module-info.class` member is present (modular jar).
    pub modular: bool,
    /// `META-INF/*.SF` signature file is present.
    pub signed: bool,
    /// Multi-release jar (`META-INF/versions/` members present).
    pub multi_release: bool,
}

fn manifest_get(body: &[u8], key: &str) -> Option<String> {
    // Unfold continuation lines: a LF/CRLF followed by a space extends
    // the previous line (java.util.jar.Manifest semantics).
    let mut flat: Vec<u8> = Vec::with_capacity(body.len());
    let mut i = 0usize;
    while i < body.len() {
        let b = body[i];
        if b == b'\n' {
            if body.get(i + 1) == Some(&b' ') {
                i += 2; // swallow newline + the continuation's leading space
                continue;
            }
            flat.push(b);
            i += 1;
        } else if b == b'\r' {
            i += 1;
        } else {
            flat.push(b);
            i += 1;
        }
    }
    let kb = key.as_bytes();
    let mut pos = 0usize;
    while pos < flat.len() {
        let eol = flat[pos..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|e| pos + e)
            .unwrap_or(flat.len());
        let line = &flat[pos..eol];
        if line.len() > kb.len() + 1 && line[..kb.len()] == *kb && line[kb.len()] == b':' {
            let mut v = &line[kb.len() + 1..];
            while !v.is_empty() && v[0] == b' ' {
                v = &v[1..];
            }
            if let Ok(s) = std::str::from_utf8(v) {
                return Some(String::from(s.trim_end()));
            }
        }
        pos = eol + 1;
    }
    None
}

/// Parse a `.jar`: ZIP members must include `META-INF/MANIFEST.MF`.
/// Returns `None` otherwise.
pub fn parse(d: &[u8]) -> Option<Jar> {
    let entries = crate::zip::list(d)?;
    let mut names: Vec<String> = Vec::new();
    let mut manifest = false;
    let mut class_files = 0usize;
    let mut modular = false;
    let mut signed = false;
    let mut multi_release = false;
    for e in &entries {
        let bare = e.name.trim_start_matches('/');
        let upper = bare.to_uppercase();
        if upper == "META-INF/MANIFEST.MF" {
            manifest = true;
        }
        if bare.ends_with(".class") {
            class_files += 1;
            if bare == "module-info.class" || bare.ends_with("/module-info.class") {
                modular = true;
            }
        }
        if upper.starts_with("META-INF/") && upper.ends_with(".SF") {
            signed = true;
        }
        if bare.starts_with("META-INF/versions/") {
            multi_release = true;
        }
        names.push(e.name.clone());
    }
    if !manifest {
        return None;
    }
    let body = crate::zip::extract(d, "META-INF/MANIFEST.MF")?;
    Some(Jar {
        entries: names,
        class_files,
        main_class: manifest_get(&body, "Main-Class"),
        implementation_version: manifest_get(&body, "Implementation-Version"),
        automatic_module: manifest_get(&body, "Automatic-Module-Name"),
        modular,
        signed,
        multi_release,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored(
            "META-INF/MANIFEST.MF",
            b"Manifest-Version: 1\x2e0\r\nMain-Class: com.\r\n example.Main\r\nAutomatic-Module-Name: demo\r\n\r\n",
        );
        zw.add_stored("com/example/Main.class", b"\xca\xfe\xba\xbe");
        zw.add_stored("module-info.class", b"\xca\xfe\xba\xbe");
        zw.add_stored("META-INF/DEMO.SF", b"Signature-Version: 1\x2e0");
        zw.add_stored("META-INF/versions/9/x.class", b"\xca\xfe\xba\xbe");
        zw.finish()
    }

    #[test]
    fn manifest_fields() {
        let j = parse(&fixture()).unwrap();
        assert_eq!(j.main_class.as_deref(), Some("com.example.Main"));
        assert_eq!(j.automatic_module.as_deref(), Some("demo"));
        assert_eq!(j.class_files, 3);
        assert!(j.modular && j.signed && j.multi_release);
    }

    #[test]
    fn rejects_plain_zip() {
        assert!(parse(b"").is_none());
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored("x.class", b"\xca\xfe\xba\xbe");
        assert!(parse(&zw.finish()).is_none());
        let mut zw2 = crate::zip::ZipWriter::new();
        zw2.add_stored("META-INF/MANIFEST.MF", b"Manifest-Version: 1\x2e0\n\n");
        assert!(parse(&zw2.finish()).is_some());
    }
}
