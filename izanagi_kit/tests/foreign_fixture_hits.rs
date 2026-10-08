//! Cross-contamination ratchet: each detector's count of *foreign* fixture
//! hits (fixtures belonging to other modules) must stay at or below the
//! recorded ceiling.
//!
//! Fixtures live in `src/<module>.rs` as `const NAME: &[u8] = b"..."` test
//! constants. A detector that fires on another format's fixture is a
//! false-positive risk, so the ceiling only ever moves down: a fix lowers
//! the count, and the recorded number is updated to match. A new detector
//! has no entry and gets the DEFAULT ceiling — add a line when a
//! legitimate generic format needs more room.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

fn unescape_byte_string(body: &str) -> Vec<u8> {
    let bytes = body.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        i += 1;
        match bytes.get(i).copied() {
            Some(b'n') => out.push(b'\n'),
            Some(b'r') => out.push(b'\r'),
            Some(b't') => out.push(b'\t'),
            Some(b'0') => out.push(0),
            Some(b'\\') => out.push(b'\\'),
            Some(b'\'') => out.push(b'\''),
            Some(b'"') => out.push(b'"'),
            Some(b'x') => {
                let h = bytes.get(i + 1..i + 3).unwrap_or(&[]);
                out.push(
                    u8::from_str_radix(std::str::from_utf8(h).unwrap_or("0"), 16).unwrap_or(0),
                );
                i += 2;
            }
            Some(b'u') => {
                let mut j = i + 1;
                let mut val = String::new();
                while bytes.get(j).is_some_and(|&c| c != b'}') {
                    val.push(bytes[j] as char);
                    j += 1;
                }
                let cp = u32::from_str_radix(&val, 16).unwrap_or(0);
                if let Some(c) = char::from_u32(cp) {
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                }
                i = j;
            }
            _ => {}
        }
        i += 1;
    }
    out
}

fn extract_fixtures(text: &str) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(p) = rest.find("b\"") {
        let head = &rest[..p];
        let mut line_start = head.len();
        for _ in 0..4 {
            line_start = head[..line_start].rfind('\n').map_or(0, |x| x + 1);
            if head[line_start..].contains("const ") || line_start == 0 {
                break;
            }
        }
        let decl = &head[line_start..];
        if !(decl.contains("const ") && decl.contains(": &[u8") && decl.trim_end().ends_with('=')) {
            rest = &rest[p + 2..];
            continue;
        }
        let name = decl
            .split("const ")
            .nth(1)
            .and_then(|s| s.split(':').next())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let body = &rest[p + 2..];
        let mut end = None;
        let mut j = 0;
        let bb = body.as_bytes();
        while j < bb.len() {
            if bb[j] == b'\\' {
                j += 2;
                continue;
            }
            if bb[j] == b'"' {
                end = Some(j);
                break;
            }
            j += 1;
        }
        let Some(end) = end else { break };
        out.push((name, unescape_byte_string(&body[..end])));
        rest = &body[end + 1..];
    }
    out
}

/// Per-detector ceiling for foreign-fixture hits. The numbers are a
/// snapshot of the measured matrix — they may only be tightened.
const CEILINGS: &[(&str, usize)] = &[
    ("mml", 477),
    ("haresources", 404),
    ("requirements", 247),
    ("lucene", 212),
    ("creole", 161),
    ("mediawiki", 150),
    ("dockerignore", 123),
    ("dotenv", 92),
    ("gitignore", 91),
    ("justfile", 82),
    ("xpath", 82),
    ("unbound", 71),
    ("sudoers", 70),
    ("openssl", 66),
    ("pppdconf", 65),
    ("txt2tags", 59),
    ("gitconfig", 45),
    ("pgpass", 40),
    ("memcachedconf", 37),
    ("rsyslogd", 34),
    ("autofs", 31),
    ("crockford", 30),
    ("inputrc", 28),
    ("kubemq", 27),
    ("mpd", 26),
    ("base32", 23),
    ("nanoid", 19),
    ("gradle", 14),
    ("edn", 10),
    ("openhab", 9),
    ("pkl", 9),
    ("appdaemon", 8),
    ("kittyconf", 8),
    ("nsswitch", 8),
    ("tnsnames", 8),
    ("zeekscript", 8),
    ("appengine", 7),
    ("appveyor", 7),
    ("babelrc", 7),
    ("bunfig", 7),
    ("cmus", 7),
    ("colima", 7),
    ("dhcpcdconf", 7),
    ("dovecot", 7),
    ("grubconf", 7),
    ("junos", 7),
    ("ledgerjournal", 7),
    ("mvnsettings", 7),
    ("pptpd", 7),
    ("sendmail", 7),
    ("ubootenv", 7),
    ("vimrc", 7),
];

/// Ceiling applied to any detector without a CEILINGS entry.
const DEFAULT_CEILING: usize = 6;

#[test]
fn foreign_hits_stay_below_ceilings() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut fixtures: Vec<(String, Vec<u8>)> = Vec::new();
    for e in fs::read_dir(&src).unwrap() {
        let p = e.unwrap().path();
        if p.extension() != Some(OsStr::new("rs")) {
            continue;
        }
        let module = p.file_stem().unwrap().to_string_lossy().into_owned();
        let text = fs::read_to_string(&p).unwrap();
        for (_name, data) in extract_fixtures(&text) {
            fixtures.push((module.clone(), data));
        }
    }
    let mut hits: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (module, data) in &fixtures {
        for &(dname, dfn) in izanagi_kit::DETECTORS {
            if dname == module.as_str() {
                continue;
            }
            if dfn(data) {
                *hits.entry(dname).or_default() += 1;
            }
        }
    }
    let ceilings: BTreeMap<&str, usize> = CEILINGS.iter().copied().collect();
    let mut over: Vec<String> = Vec::new();
    for (name, n) in &hits {
        let cap = ceilings.get(name).copied().unwrap_or(DEFAULT_CEILING);
        if *n > cap {
            over.push(format!("{name}: {n} > {cap}"));
        }
    }
    assert!(
        over.is_empty(),
        "detectors exceeding foreign-fixture ceilings: {over:?}"
    );
}
