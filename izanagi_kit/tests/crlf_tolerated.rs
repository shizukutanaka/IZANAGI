//! CRLF tolerance: every *text* fixture (one that decodes as UTF-8)
//! must still be detected by its own detector after every `\n` is
//! rewritten to `\r\n`. A config file saved on Windows must not
//! silently stop being recognized. Binary fixtures — signatures and
//! magics containing non-UTF-8 bytes — are skipped by the UTF-8 gate,
//! since CR/LF bytes inside a binary header are payload, not newlines.

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

fn crlf(d: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(d.len() + d.len() / 4);
    let mut prev = 0u8;
    for &b in d {
        if b == b'\n' && prev != b'\r' {
            out.push(b'\r');
        }
        out.push(b);
        prev = b;
    }
    out
}

#[test]
fn text_fixtures_survive_crlf() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let detectors: BTreeMap<&str, &izanagi_kit::DetectorFn> = izanagi_kit::DETECTORS
        .iter()
        .map(|(n, d)| (*n, d))
        .collect();
    let mut misses: Vec<String> = Vec::new();
    let mut total = 0usize;
    for entry in fs::read_dir(&src).unwrap() {
        let path = entry.unwrap().path();
        if path.extension() != Some(OsStr::new("rs")) {
            continue;
        }
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();
        let Some(detect) = detectors.get(name.as_str()) else {
            continue;
        };
        let text = fs::read_to_string(&path).unwrap_or_default();
        for (cname, fixture) in extract_fixtures(&text) {
            // text-only: binary magics legitimately break under CRLF
            let text_fixture = std::str::from_utf8(&fixture).is_ok();
            if !text_fixture || !detect(&fixture) {
                continue;
            }
            let conv = crlf(&fixture);
            if conv == fixture {
                continue;
            }
            total += 1;
            if !detect(&conv) {
                misses.push(format!("{name}::{cname}"));
            }
        }
    }
    assert!(
        misses.is_empty(),
        "text fixtures no longer detected under CRLF ({total} checked): {misses:?}"
    );
}
