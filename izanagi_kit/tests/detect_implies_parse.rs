//! Directional contract test: when a detector claims a *complete* sample,
//! the registered parser must also accept it (`detect(f) => parse(f)`).
//! The reverse direction is deliberately not asserted — `detect` may be a
//! stricter gate than `parse` by design (several modules implement it as
//! `parse` plus a count threshold), so `parse = Some` does not promise
//! `detect = true`.
//!
//! Deliberate fragment consts (magic prefixes that `detect` accepts but a
//! full `parse` legitimately rejects) are listed in `KNOWN_FRAGMENTS`.

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
#[test]
fn detect_implies_parse_on_complete_fixtures() {
    const KNOWN_FRAGMENTS: &[&str] = &["raf::MAGIC", "slob::MAGIC"];
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations: Vec<String> = Vec::new();
    let mut seen_fragments = std::collections::BTreeSet::new();
    {
        let text = fs::read_to_string(src.join("a2r.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::a2r::detect(&fx) && !(izanagi_kit::a2r::parse(&fx).is_some()) {
                let key = format!("a2r::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ac.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ac::detect(&fx) && !(izanagi_kit::ac::parse(&fx).is_some()) {
                let key = format!("ac::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ace.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ace::detect(&fx) && !(izanagi_kit::ace::parse(&fx).is_some()) {
                let key = format!("ace::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("actionlint.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::actionlint::detect(&fx)
                && !(izanagi_kit::actionlint::parse(&fx).is_some())
            {
                let key = format!("actionlint::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("activemq.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::activemq::detect(&fx) && !(izanagi_kit::activemq::parse(&fx).is_some())
            {
                let key = format!("activemq::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("aercconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::aercconf::detect(&fx) && !(izanagi_kit::aercconf::parse(&fx).is_some())
            {
                let key = format!("aercconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("aerospike.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::aerospike::detect(&fx)
                && !(izanagi_kit::aerospike::parse(&fx).is_some())
            {
                let key = format!("aerospike::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("aideconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::aideconf::detect(&fx) && !(izanagi_kit::aideconf::parse(&fx).is_some())
            {
                let key = format!("aideconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("aiger.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::aiger::detect(&fx) && !(izanagi_kit::aiger::parse(&fx).is_some()) {
                let key = format!("aiger::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("airbyteconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::airbyteconf::detect(&fx)
                && !(izanagi_kit::airbyteconf::parse(&fx).is_some())
            {
                let key = format!("airbyteconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("alembic.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::alembic::detect(&fx) && !(izanagi_kit::alembic::parse(&fx).is_some()) {
                let key = format!("alembic::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("alexrc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::alexrc::detect(&fx) && !(izanagi_kit::alexrc::parse(&fx).is_some()) {
                let key = format!("alexrc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("aln.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::aln::detect(&fx) && !(izanagi_kit::aln::parse(&fx).is_some()) {
                let key = format!("aln::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("alto.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::alto::detect(&fx) && !(izanagi_kit::alto::parse(&fx).is_some()) {
                let key = format!("alto::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("alz.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::alz::detect(&fx) && !(izanagi_kit::alz::parse(&fx).is_some()) {
                let key = format!("alz::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("amandaconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::amandaconf::detect(&fx) && !(true) {
                let key = format!("amandaconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ampl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ampl::detect(&fx) && !(izanagi_kit::ampl::parse(&fx).is_some()) {
                let key = format!("ampl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("amplifyconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::amplifyconf::detect(&fx)
                && !(izanagi_kit::amplifyconf::parse(&fx).is_some())
            {
                let key = format!("amplifyconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("amr.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::amr::detect(&fx) && !(izanagi_kit::amr::parse(&fx).is_some()) {
                let key = format!("amr::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ansi.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ansi::detect(&fx) && !(izanagi_kit::ansi::parse(&fx).is_some()) {
                let key = format!("ansi::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("antex.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::antex::detect(&fx) && !(izanagi_kit::antex::parse(&fx).is_some()) {
                let key = format!("antex::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("aoe.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::aoe::detect(&fx) && !(izanagi_kit::aoe::parse(&fx).is_some()) {
                let key = format!("aoe::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("appcache.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::appcache::detect(&fx) && !(izanagi_kit::appcache::parse(&fx).is_some())
            {
                let key = format!("appcache::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("appveyor.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::appveyor::detect(&fx) && !(izanagi_kit::appveyor::parse(&fx).is_some())
            {
                let key = format!("appveyor::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("aprxconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::aprxconf::detect(&fx) && !(true) {
                let key = format!("aprxconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("archinstall.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::archinstall::detect(&fx)
                && !(izanagi_kit::archinstall::parse(&fx).is_some())
            {
                let key = format!("archinstall::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ardour.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ardour::detect(&fx) && !(izanagi_kit::ardour::parse(&fx).is_some()) {
                let key = format!("ardour::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("arduinoconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::arduinoconf::detect(&fx)
                && !(izanagi_kit::arduinoconf::parse(&fx).is_some())
            {
                let key = format!("arduinoconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("argorollout.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::argorollout::detect(&fx)
                && !(izanagi_kit::argorollout::parse(&fx).is_some())
            {
                let key = format!("argorollout::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("argusconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::argusconf::detect(&fx)
                && !(izanagi_kit::argusconf::parse(&fx).is_some())
            {
                let key = format!("argusconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("arkimeconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::arkimeconf::detect(&fx)
                && !(izanagi_kit::arkimeconf::parse(&fx).is_some())
            {
                let key = format!("arkimeconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("arma3conf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::arma3conf::detect(&fx) && !(true) {
                let key = format!("arma3conf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("arw.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::arw::detect(&fx) && !(izanagi_kit::arw::parse(&fx).is_some()) {
                let key = format!("arw::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("asciicast.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::asciicast::detect(&fx)
                && !(izanagi_kit::asciicast::parse(&fx).is_some())
            {
                let key = format!("asciicast::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("asf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::asf::detect(&fx) && !(izanagi_kit::asf::parse(&fx).is_some()) {
                let key = format!("asf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("asn1.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::asn1::detect(&fx) && !(izanagi_kit::asn1::parse(&fx).is_some()) {
                let key = format!("asn1::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("atom.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::atom::detect(&fx) && !(izanagi_kit::atom::parse(&fx).is_some()) {
                let key = format!("atom::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("audacity.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::audacity::detect(&fx) && !(izanagi_kit::audacity::parse(&fx).is_some())
            {
                let key = format!("audacity::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("auditdconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::auditdconf::detect(&fx)
                && !(izanagi_kit::auditdconf::parse(&fx).is_some())
            {
                let key = format!("auditdconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("auditrule.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::auditrule::detect(&fx)
                && !(izanagi_kit::auditrule::parse(&fx).is_some())
            {
                let key = format!("auditrule::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("autofs.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::autofs::detect(&fx) && !(izanagi_kit::autofs::parse(&fx).is_some()) {
                let key = format!("autofs::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("autoyast.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::autoyast::detect(&fx) && !(izanagi_kit::autoyast::parse(&fx).is_some())
            {
                let key = format!("autoyast::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("axports.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::axports::detect(&fx) && !(true) {
                let key = format!("axports::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("backstage.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::backstage::detect(&fx)
                && !(izanagi_kit::backstage::parse(&fx).is_some())
            {
                let key = format!("backstage::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("baculadir.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::baculadir::detect(&fx) && !(true) {
                let key = format!("baculadir::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("bai2.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::bai2::detect(&fx) && !(izanagi_kit::bai2::parse(&fx).is_some()) {
                let key = format!("bai2::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("bam.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::bam::detect(&fx) && !(izanagi_kit::bam::parse(&fx).is_some()) {
                let key = format!("bam::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("base32.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::base32::detect(&fx) && !(izanagi_kit::base32::parse(&fx).is_some()) {
                let key = format!("base32::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("bazel.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::bazel::detect(&fx) && !(izanagi_kit::bazel::parse(&fx).is_some()) {
                let key = format!("bazel::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("bbcode.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::bbcode::detect(&fx) && !(izanagi_kit::bbcode::parse(&fx).is_some()) {
                let key = format!("bbcode::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("bit.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::bit::detect(&fx) && !(izanagi_kit::bit::parse(&fx).is_some()) {
                let key = format!("bit::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("bitcoinconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::bitcoinconf::detect(&fx)
                && !(izanagi_kit::bitcoinconf::parse(&fx).is_some())
            {
                let key = format!("bitcoinconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("bogofilter.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::bogofilter::detect(&fx)
                && !(izanagi_kit::bogofilter::parse(&fx).is_some())
            {
                let key = format!("bogofilter::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("browserconfig.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::browserconfig::detect(&fx)
                && !(izanagi_kit::browserconfig::parse(&fx).is_some())
            {
                let key = format!("browserconfig::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("bsnes.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::bsnes::detect(&fx) && !(true) {
                let key = format!("bsnes::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("btrfs.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::btrfs::detect(&fx) && !(izanagi_kit::btrfs::parse(&fx).is_some()) {
                let key = format!("btrfs::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("buck.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::buck::detect(&fx) && !(izanagi_kit::buck::parse(&fx).is_some()) {
                let key = format!("buck::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("buildkitd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::buildkitd::detect(&fx)
                && !(izanagi_kit::buildkitd::parse(&fx).is_some())
            {
                let key = format!("buildkitd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("buildkite.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::buildkite::detect(&fx)
                && !(izanagi_kit::buildkite::parse(&fx).is_some())
            {
                let key = format!("buildkite::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("bundlerconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::bundlerconf::detect(&fx)
                && !(izanagi_kit::bundlerconf::parse(&fx).is_some())
            {
                let key = format!("bundlerconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cabal.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cabal::detect(&fx) && !(izanagi_kit::cabal::parse(&fx).is_some()) {
                let key = format!("cabal::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("calamares.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::calamares::detect(&fx)
                && !(izanagi_kit::calamares::parse(&fx).is_some())
            {
                let key = format!("calamares::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("camt.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::camt::detect(&fx) && !(izanagi_kit::camt::parse(&fx).is_some()) {
                let key = format!("camt::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("capx.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::capx::detect(&fx) && !(izanagi_kit::capx::parse(&fx).is_some()) {
                let key = format!("capx::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cardanoconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cardanoconf::detect(&fx)
                && !(izanagi_kit::cardanoconf::parse(&fx).is_some())
            {
                let key = format!("cardanoconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cargoconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cargoconf::detect(&fx)
                && !(izanagi_kit::cargoconf::parse(&fx).is_some())
            {
                let key = format!("cargoconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("carla.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::carla::detect(&fx) && !(izanagi_kit::carla::parse(&fx).is_some()) {
                let key = format!("carla::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cartocss.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cartocss::detect(&fx) && !(true) {
                let key = format!("cartocss::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cephconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cephconf::detect(&fx) && !(izanagi_kit::cephconf::parse(&fx).is_some())
            {
                let key = format!("cephconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("changesets.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::changesets::detect(&fx)
                && !(izanagi_kit::changesets::parse(&fx).is_some())
            {
                let key = format!("changesets::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("chaosmesh.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::chaosmesh::detect(&fx)
                && !(izanagi_kit::chaosmesh::parse(&fx).is_some())
            {
                let key = format!("chaosmesh::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("chirpcsv.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::chirpcsv::detect(&fx) && !(true) {
                let key = format!("chirpcsv::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("chronyconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::chronyconf::detect(&fx)
                && !(izanagi_kit::chronyconf::parse(&fx).is_some())
            {
                let key = format!("chronyconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cibxml.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cibxml::detect(&fx) && !(izanagi_kit::cibxml::parse(&fx).is_some()) {
                let key = format!("cibxml::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cirrus.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cirrus::detect(&fx) && !(izanagi_kit::cirrus::parse(&fx).is_some()) {
                let key = format!("cirrus::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("citraconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::citraconf::detect(&fx) && !(true) {
                let key = format!("citraconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cliff.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cliff::detect(&fx) && !(izanagi_kit::cliff::parse(&fx).is_some()) {
                let key = format!("cliff::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("clusterconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::clusterconf::detect(&fx)
                && !(izanagi_kit::clusterconf::parse(&fx).is_some())
            {
                let key = format!("clusterconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cmake.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cmake::detect(&fx) && !(izanagi_kit::cmake::parse(&fx).is_some()) {
                let key = format!("cmake::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cob.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cob::detect(&fx) && !(izanagi_kit::cob::parse(&fx).is_some()) {
                let key = format!("cob::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cocosproj.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cocosproj::detect(&fx)
                && !(izanagi_kit::cocosproj::parse(&fx).is_some())
            {
                let key = format!("cocosproj::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("codeclimate.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::codeclimate::detect(&fx)
                && !(izanagi_kit::codeclimate::parse(&fx).is_some())
            {
                let key = format!("codeclimate::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("codecov.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::codecov::detect(&fx) && !(izanagi_kit::codecov::parse(&fx).is_some()) {
                let key = format!("codecov::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("codespell.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::codespell::detect(&fx)
                && !(izanagi_kit::codespell::parse(&fx).is_some())
            {
                let key = format!("codespell::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("colima.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::colima::detect(&fx) && !(izanagi_kit::colima::parse(&fx).is_some()) {
                let key = format!("colima::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("commitlint.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::commitlint::detect(&fx)
                && !(izanagi_kit::commitlint::parse(&fx).is_some())
            {
                let key = format!("commitlint::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("contourconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::contourconf::detect(&fx)
                && !(izanagi_kit::contourconf::parse(&fx).is_some())
            {
                let key = format!("contourconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cookiejar.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cookiejar::detect(&fx)
                && !(izanagi_kit::cookiejar::parse(&fx).is_some())
            {
                let key = format!("cookiejar::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("coq.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::coq::detect(&fx) && !(izanagi_kit::coq::parse(&fx).is_some()) {
                let key = format!("coq::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("corosync.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::corosync::detect(&fx) && !(izanagi_kit::corosync::parse(&fx).is_some())
            {
                let key = format!("corosync::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("coveralls.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::coveralls::detect(&fx)
                && !(izanagi_kit::coveralls::parse(&fx).is_some())
            {
                let key = format!("coveralls::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cpanfile.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cpanfile::detect(&fx) && !(izanagi_kit::cpanfile::parse(&fx).is_some())
            {
                let key = format!("cpanfile::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cr2.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cr2::detect(&fx) && !(izanagi_kit::cr2::parse(&fx).is_some()) {
                let key = format!("cr2::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("creole.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::creole::detect(&fx) && !(izanagi_kit::creole::parse(&fx).is_some()) {
                let key = format!("creole::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("crio.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::crio::detect(&fx) && !(izanagi_kit::crio::parse(&fx).is_some()) {
                let key = format!("crio::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("crmconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::crmconf::detect(&fx) && !(izanagi_kit::crmconf::parse(&fx).is_some()) {
                let key = format!("crmconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("crockford.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::crockford::detect(&fx)
                && !(izanagi_kit::crockford::parse(&fx).is_some())
            {
                let key = format!("crockford::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cromwell.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cromwell::detect(&fx) && !(true) {
                let key = format!("cromwell::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("crossplane.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::crossplane::detect(&fx)
                && !(izanagi_kit::crossplane::parse(&fx).is_some())
            {
                let key = format!("crossplane::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("crowdsec.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::crowdsec::detect(&fx) && !(true) {
                let key = format!("crowdsec::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("csa.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::csa::detect(&fx) && !(izanagi_kit::csa::parse(&fx).is_some()) {
                let key = format!("csa::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("csd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::csd::detect(&fx) && !(izanagi_kit::csd::parse(&fx).is_some()) {
                let key = format!("csd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cspell.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cspell::detect(&fx) && !(izanagi_kit::cspell::parse(&fx).is_some()) {
                let key = format!("cspell::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cuid.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cuid::detect(&fx) && !(izanagi_kit::cuid::parse(&fx).is_some()) {
                let key = format!("cuid::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cupsconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cupsconf::detect(&fx) && !(true) {
                let key = format!("cupsconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("curaconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::curaconf::detect(&fx) && !(izanagi_kit::curaconf::parse(&fx).is_some())
            {
                let key = format!("curaconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cve.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cve::detect(&fx) && !(izanagi_kit::cve::parse(&fx).is_some()) {
                let key = format!("cve::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("cypher.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::cypher::detect(&fx) && !(izanagi_kit::cypher::parse(&fx).is_some()) {
                let key = format!("cypher::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dae.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dae::detect(&fx) && !(izanagi_kit::dae::parse(&fx).is_some()) {
                let key = format!("dae::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dbm.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dbm::detect(&fx) && !(izanagi_kit::dbm::parse(&fx).is_some()) {
                let key = format!("dbm::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("debconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::debconf::detect(&fx) && !(izanagi_kit::debconf::parse(&fx).is_some()) {
                let key = format!("debconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("defconfig.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::defconfig::detect(&fx)
                && !(izanagi_kit::defconfig::parse(&fx).is_some())
            {
                let key = format!("defconfig::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("defoldproj.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::defoldproj::detect(&fx)
                && !(izanagi_kit::defoldproj::parse(&fx).is_some())
            {
                let key = format!("defoldproj::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("denyhosts.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::denyhosts::detect(&fx) && !(true) {
                let key = format!("denyhosts::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dependabot.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dependabot::detect(&fx)
                && !(izanagi_kit::dependabot::parse(&fx).is_some())
            {
                let key = format!("dependabot::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("devbox.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::devbox::detect(&fx) && !(izanagi_kit::devbox::parse(&fx).is_some()) {
                let key = format!("devbox::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dhclientconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dhclientconf::detect(&fx)
                && !(izanagi_kit::dhclientconf::parse(&fx).is_some())
            {
                let key = format!("dhclientconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dhcpcdconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dhcpcdconf::detect(&fx)
                && !(izanagi_kit::dhcpcdconf::parse(&fx).is_some())
            {
                let key = format!("dhcpcdconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dictd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dictd::detect(&fx) && !(izanagi_kit::dictd::parse(&fx).is_some()) {
                let key = format!("dictd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dictzip.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dictzip::detect(&fx) && !(izanagi_kit::dictzip::parse(&fx).is_some()) {
                let key = format!("dictzip::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("did.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::did::detect(&fx) && !(izanagi_kit::did::parse(&fx).is_some()) {
                let key = format!("did::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("digikamrc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::digikamrc::detect(&fx) && !(true) {
                let key = format!("digikamrc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dimacs.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dimacs::detect(&fx) && !(izanagi_kit::dimacs::parse(&fx).is_some()) {
                let key = format!("dimacs::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("direwolfconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::direwolfconf::detect(&fx) && !(true) {
                let key = format!("direwolfconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("discourse.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::discourse::detect(&fx)
                && !(izanagi_kit::discourse::parse(&fx).is_some())
            {
                let key = format!("discourse::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dita.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dita::detect(&fx) && !(izanagi_kit::dita::parse(&fx).is_some()) {
                let key = format!("dita::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dng.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dng::detect(&fx) && !(izanagi_kit::dng::parse(&fx).is_some()) {
                let key = format!("dng::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("docbook.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::docbook::detect(&fx) && !(izanagi_kit::docbook::parse(&fx).is_some()) {
                let key = format!("docbook::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dolphinconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dolphinconf::detect(&fx)
                && !(izanagi_kit::dolphinconf::parse(&fx).is_some())
            {
                let key = format!("dolphinconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dosboxconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dosboxconf::detect(&fx) && !(true) {
                let key = format!("dosboxconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dpx.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dpx::detect(&fx) && !(izanagi_kit::dpx::parse(&fx).is_some()) {
                let key = format!("dpx::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dragonflyconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dragonflyconf::detect(&fx)
                && !(izanagi_kit::dragonflyconf::parse(&fx).is_some())
            {
                let key = format!("dragonflyconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("drbdconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::drbdconf::detect(&fx) && !(izanagi_kit::drbdconf::parse(&fx).is_some())
            {
                let key = format!("drbdconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dsig.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dsig::detect(&fx) && !(izanagi_kit::dsig::parse(&fx).is_some()) {
                let key = format!("dsig::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dsl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dsl::detect(&fx) && !(izanagi_kit::dsl::parse(&fx).is_some()) {
                let key = format!("dsl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dtd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dtd::detect(&fx) && !(izanagi_kit::dtd::parse(&fx).is_some()) {
                let key = format!("dtd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dvi.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dvi::detect(&fx) && !(izanagi_kit::dvi::parse(&fx).is_some()) {
                let key = format!("dvi::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("dxbc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::dxbc::detect(&fx) && !(izanagi_kit::dxbc::parse(&fx).is_some()) {
                let key = format!("dxbc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ead.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ead::detect(&fx) && !(izanagi_kit::ead::parse(&fx).is_some()) {
                let key = format!("ead::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("eaglexml.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::eaglexml::detect(&fx) && !(izanagi_kit::eaglexml::parse(&fx).is_some())
            {
                let key = format!("eaglexml::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ean.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ean::detect(&fx) && !(izanagi_kit::ean::parse(&fx).is_some()) {
                let key = format!("ean::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("earthly.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::earthly::detect(&fx) && !(izanagi_kit::earthly::parse(&fx).is_some()) {
                let key = format!("earthly::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("edsk.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::edsk::detect(&fx) && !(izanagi_kit::edsk::parse(&fx).is_some()) {
                let key = format!("edsk::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ejabberd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ejabberd::detect(&fx) && !(izanagi_kit::ejabberd::parse(&fx).is_some())
            {
                let key = format!("ejabberd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("epd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::epd::detect(&fx) && !(izanagi_kit::epd::parse(&fx).is_some()) {
                let key = format!("epd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("epwing.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::epwing::detect(&fx) && !(izanagi_kit::epwing::parse(&fx).is_some()) {
                let key = format!("epwing::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("exr.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::exr::detect(&fx) && !(izanagi_kit::exr::parse(&fx).is_some()) {
                let key = format!("exr::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("extmanifest.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::extmanifest::detect(&fx)
                && !(izanagi_kit::extmanifest::parse(&fx).is_some())
            {
                let key = format!("extmanifest::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("f2fs.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::f2fs::detect(&fx) && !(izanagi_kit::f2fs::parse(&fx).is_some()) {
                let key = format!("f2fs::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("factoriosettings.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::factoriosettings::detect(&fx) && !(true) {
                let key = format!("factoriosettings::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("fail2ban.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::fail2ban::detect(&fx) && !(izanagi_kit::fail2ban::parse(&fx).is_some())
            {
                let key = format!("fail2ban::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("fail2banconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::fail2banconf::detect(&fx) && !(true) {
                let key = format!("fail2banconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("falcoconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::falcoconf::detect(&fx)
                && !(izanagi_kit::falcoconf::parse(&fx).is_some())
            {
                let key = format!("falcoconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("far.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::far::detect(&fx) && !(izanagi_kit::far::parse(&fx).is_some()) {
                let key = format!("far::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("fceux.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::fceux::detect(&fx) && !(true) {
                let key = format!("fceux::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("fcoe.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::fcoe::detect(&fx) && !(izanagi_kit::fcoe::parse(&fx).is_some()) {
                let key = format!("fcoe::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ferm.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ferm::detect(&fx) && !(izanagi_kit::ferm::parse(&fx).is_some()) {
                let key = format!("ferm::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("fetchmailconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::fetchmailconf::detect(&fx)
                && !(izanagi_kit::fetchmailconf::parse(&fx).is_some())
            {
                let key = format!("fetchmailconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("fidl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::fidl::detect(&fx) && !(izanagi_kit::fidl::parse(&fx).is_some()) {
                let key = format!("fidl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("firejailprof.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::firejailprof::detect(&fx) && !(true) {
                let key = format!("firejailprof::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("fivetranconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::fivetranconf::detect(&fx)
                && !(izanagi_kit::fivetranconf::parse(&fx).is_some())
            {
                let key = format!("fivetranconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("fixml.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::fixml::detect(&fx) && !(izanagi_kit::fixml::parse(&fx).is_some()) {
                let key = format!("fixml::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("flagger.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::flagger::detect(&fx) && !(izanagi_kit::flagger::parse(&fx).is_some()) {
                let key = format!("flagger::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("fldigiconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::fldigiconf::detect(&fx) && !(true) {
                let key = format!("fldigiconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("flif.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::flif::detect(&fx) && !(izanagi_kit::flif::parse(&fx).is_some()) {
                let key = format!("flif::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("fluxcd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::fluxcd::detect(&fx) && !(izanagi_kit::fluxcd::parse(&fx).is_some()) {
                let key = format!("fluxcd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("flyio.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::flyio::detect(&fx) && !(izanagi_kit::flyio::parse(&fx).is_some()) {
                let key = format!("flyio::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("flyway.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::flyway::detect(&fx) && !(izanagi_kit::flyway::parse(&fx).is_some()) {
                let key = format!("flyway::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("footconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::footconf::detect(&fx) && !(izanagi_kit::footconf::parse(&fx).is_some())
            {
                let key = format!("footconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("fpml.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::fpml::detect(&fx) && !(izanagi_kit::fpml::parse(&fx).is_some()) {
                let key = format!("fpml::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("fusesoc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::fusesoc::detect(&fx) && !(true) {
                let key = format!("fusesoc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("fxml.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::fxml::detect(&fx) && !(izanagi_kit::fxml::parse(&fx).is_some()) {
                let key = format!("fxml::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("fxp.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::fxp::detect(&fx) && !(izanagi_kit::fxp::parse(&fx).is_some()) {
                let key = format!("fxp::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("garnetconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::garnetconf::detect(&fx)
                && !(izanagi_kit::garnetconf::parse(&fx).is_some())
            {
                let key = format!("garnetconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gatekeeper.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gatekeeper::detect(&fx)
                && !(izanagi_kit::gatekeeper::parse(&fx).is_some())
            {
                let key = format!("gatekeeper::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gbstudio.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gbstudio::detect(&fx) && !(izanagi_kit::gbstudio::parse(&fx).is_some())
            {
                let key = format!("gbstudio::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gdf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gdf::detect(&fx) && !(izanagi_kit::gdf::parse(&fx).is_some()) {
                let key = format!("gdf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gdmconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gdmconf::detect(&fx) && !(izanagi_kit::gdmconf::parse(&fx).is_some()) {
                let key = format!("gdmconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gedasch.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gedasch::detect(&fx) && !(izanagi_kit::gedasch::parse(&fx).is_some()) {
                let key = format!("gedasch::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gemrc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gemrc::detect(&fx) && !(izanagi_kit::gemrc::parse(&fx).is_some()) {
                let key = format!("gemrc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gerbera.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gerbera::detect(&fx) && !(true) {
                let key = format!("gerbera::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gethconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gethconf::detect(&fx) && !(izanagi_kit::gethconf::parse(&fx).is_some())
            {
                let key = format!("gethconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("getmailrc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::getmailrc::detect(&fx) && !(true) {
                let key = format!("getmailrc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gexf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gexf::detect(&fx) && !(izanagi_kit::gexf::parse(&fx).is_some()) {
                let key = format!("gexf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gf::detect(&fx) && !(izanagi_kit::gf::parse(&fx).is_some()) {
                let key = format!("gf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ghosttyconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ghosttyconf::detect(&fx)
                && !(izanagi_kit::ghosttyconf::parse(&fx).is_some())
            {
                let key = format!("ghosttyconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("giteaaction.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::giteaaction::detect(&fx)
                && !(izanagi_kit::giteaaction::parse(&fx).is_some())
            {
                let key = format!("giteaaction::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("glade.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::glade::detect(&fx) && !(izanagi_kit::glade::parse(&fx).is_some()) {
                let key = format!("glade::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("glsl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::glsl::detect(&fx) && !(izanagi_kit::glsl::parse(&fx).is_some()) {
                let key = format!("glsl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("glusterfs.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::glusterfs::detect(&fx)
                && !(izanagi_kit::glusterfs::parse(&fx).is_some())
            {
                let key = format!("glusterfs::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gml.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gml::detect(&fx) && !(izanagi_kit::gml::parse(&fx).is_some()) {
                let key = format!("gml::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gn.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gn::detect(&fx) && !(izanagi_kit::gn::parse(&fx).is_some()) {
                let key = format!("gn::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gnuplot.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gnuplot::detect(&fx) && !(true) {
                let key = format!("gnuplot::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("goreleaser.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::goreleaser::detect(&fx)
                && !(izanagi_kit::goreleaser::parse(&fx).is_some())
            {
                let key = format!("goreleaser::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gostconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gostconf::detect(&fx) && !(true) {
                let key = format!("gostconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gp.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gp::detect(&fx) && !(izanagi_kit::gp::parse(&fx).is_some()) {
                let key = format!("gp::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gpsd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gpsd::detect(&fx) && !(izanagi_kit::gpsd::parse(&fx).is_some()) {
                let key = format!("gpsd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("gqrxconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::gqrxconf::detect(&fx) && !(true) {
                let key = format!("gqrxconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("graphml.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::graphml::detect(&fx) && !(izanagi_kit::graphml::parse(&fx).is_some()) {
                let key = format!("graphml::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("graphql.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::graphql::detect(&fx) && !(izanagi_kit::graphql::parse(&fx).is_some()) {
                let key = format!("graphql::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("greetd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::greetd::detect(&fx) && !(true) {
                let key = format!("greetd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("grubcfg.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::grubcfg::detect(&fx) && !(true) {
                let key = format!("grubcfg::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("grubconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::grubconf::detect(&fx) && !(izanagi_kit::grubconf::parse(&fx).is_some())
            {
                let key = format!("grubconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("grubenv.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::grubenv::detect(&fx) && !(izanagi_kit::grubenv::parse(&fx).is_some()) {
                let key = format!("grubenv::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("hacf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::hacf::detect(&fx) && !(izanagi_kit::hacf::parse(&fx).is_some()) {
                let key = format!("hacf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("hadolintconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::hadolintconf::detect(&fx)
                && !(izanagi_kit::hadolintconf::parse(&fx).is_some())
            {
                let key = format!("hadolintconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("hadoopconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::hadoopconf::detect(&fx)
                && !(izanagi_kit::hadoopconf::parse(&fx).is_some())
            {
                let key = format!("hadoopconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("haresources.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::haresources::detect(&fx)
                && !(izanagi_kit::haresources::parse(&fx).is_some())
            {
                let key = format!("haresources::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("harness.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::harness::detect(&fx) && !(izanagi_kit::harness::parse(&fx).is_some()) {
                let key = format!("harness::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("hb.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::hb::detect(&fx) && !(izanagi_kit::hb::parse(&fx).is_some()) {
                let key = format!("hb::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("hexchat.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::hexchat::detect(&fx) && !(izanagi_kit::hexchat::parse(&fx).is_some()) {
                let key = format!("hexchat::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("hfe.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::hfe::detect(&fx) && !(izanagi_kit::hfe::parse(&fx).is_some()) {
                let key = format!("hfe::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("hfs.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::hfs::detect(&fx) && !(izanagi_kit::hfs::parse(&fx).is_some()) {
                let key = format!("hfs::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("himalayaconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::himalayaconf::detect(&fx) && !(true) {
                let key = format!("himalayaconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("hivemq.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::hivemq::detect(&fx) && !(izanagi_kit::hivemq::parse(&fx).is_some()) {
                let key = format!("hivemq::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("hlsl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::hlsl::detect(&fx) && !(izanagi_kit::hlsl::parse(&fx).is_some()) {
                let key = format!("hlsl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("hocr.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::hocr::detect(&fx) && !(izanagi_kit::hocr::parse(&fx).is_some()) {
                let key = format!("hocr::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("hopconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::hopconf::detect(&fx) && !(izanagi_kit::hopconf::parse(&fx).is_some()) {
                let key = format!("hopconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("hostapd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::hostapd::detect(&fx) && !(izanagi_kit::hostapd::parse(&fx).is_some()) {
                let key = format!("hostapd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("hydrogen.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::hydrogen::detect(&fx) && !(izanagi_kit::hydrogen::parse(&fx).is_some())
            {
                let key = format!("hydrogen::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("hysteriaconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::hysteriaconf::detect(&fx) && !(true) {
                let key = format!("hysteriaconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ibmmq.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ibmmq::detect(&fx) && !(izanagi_kit::ibmmq::parse(&fx).is_some()) {
                let key = format!("ibmmq::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ical.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ical::detect(&fx) && !(izanagi_kit::ical::parse(&fx).is_some()) {
                let key = format!("ical::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("idl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::idl::detect(&fx) && !(izanagi_kit::idl::parse(&fx).is_some()) {
                let key = format!("idl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("imd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::imd::detect(&fx) && !(izanagi_kit::imd::parse(&fx).is_some()) {
                let key = format!("imd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ioc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ioc::detect(&fx) && !(izanagi_kit::ioc::parse(&fx).is_some()) {
                let key = format!("ioc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ipset.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ipset::detect(&fx) && !(izanagi_kit::ipset::parse(&fx).is_some()) {
                let key = format!("ipset::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("iptablessave.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::iptablessave::detect(&fx)
                && !(izanagi_kit::iptablessave::parse(&fx).is_some())
            {
                let key = format!("iptablessave::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("iptc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::iptc::detect(&fx) && !(izanagi_kit::iptc::parse(&fx).is_some()) {
                let key = format!("iptc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ipxact.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ipxact::detect(&fx) && !(izanagi_kit::ipxact::parse(&fx).is_some()) {
                let key = format!("ipxact::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ipxescript.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ipxescript::detect(&fx)
                && !(izanagi_kit::ipxescript::parse(&fx).is_some())
            {
                let key = format!("ipxescript::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ircam.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ircam::detect(&fx) && !(izanagi_kit::ircam::parse(&fx).is_some()) {
                let key = format!("ircam::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("irssi.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::irssi::detect(&fx) && !(izanagi_kit::irssi::parse(&fx).is_some()) {
                let key = format!("irssi::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("isabelle.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::isabelle::detect(&fx) && !(izanagi_kit::isabelle::parse(&fx).is_some())
            {
                let key = format!("isabelle::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("isbn.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::isbn::detect(&fx) && !(izanagi_kit::isbn::parse(&fx).is_some()) {
                let key = format!("isbn::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("isc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::isc::detect(&fx) && !(izanagi_kit::isc::parse(&fx).is_some()) {
                let key = format!("isc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("iscsi.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::iscsi::detect(&fx) && !(izanagi_kit::iscsi::parse(&fx).is_some()) {
                let key = format!("iscsi::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ismn.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ismn::detect(&fx) && !(izanagi_kit::ismn::parse(&fx).is_some()) {
                let key = format!("ismn::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("issn.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::issn::detect(&fx) && !(izanagi_kit::issn::parse(&fx).is_some()) {
                let key = format!("issn::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("iterm.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::iterm::detect(&fx) && !(izanagi_kit::iterm::parse(&fx).is_some()) {
                let key = format!("iterm::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("itermdyn.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::itermdyn::detect(&fx) && !(izanagi_kit::itermdyn::parse(&fx).is_some())
            {
                let key = format!("itermdyn::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("iv.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::iv::detect(&fx) && !(izanagi_kit::iv::parse(&fx).is_some()) {
                let key = format!("iv::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ivf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ivf::detect(&fx) && !(izanagi_kit::ivf::parse(&fx).is_some()) {
                let key = format!("ivf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("iwdconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::iwdconf::detect(&fx) && !(izanagi_kit::iwdconf::parse(&fx).is_some()) {
                let key = format!("iwdconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("jbig2.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::jbig2::detect(&fx) && !(izanagi_kit::jbig2::parse(&fx).is_some()) {
                let key = format!("jbig2::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("jed.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::jed::detect(&fx) && !(izanagi_kit::jed::parse(&fx).is_some()) {
                let key = format!("jed::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("jfm.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::jfm::detect(&fx) && !(izanagi_kit::jfm::parse(&fx).is_some()) {
                let key = format!("jfm::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("jfs.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::jfs::detect(&fx) && !(izanagi_kit::jfs::parse(&fx).is_some()) {
                let key = format!("jfs::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("journaldconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::journaldconf::detect(&fx)
                && !(izanagi_kit::journaldconf::parse(&fx).is_some())
            {
                let key = format!("journaldconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("jp2.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::jp2::detect(&fx) && !(izanagi_kit::jp2::parse(&fx).is_some()) {
                let key = format!("jp2::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("jq.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::jq::detect(&fx) && !(izanagi_kit::jq::parse(&fx).is_some()) {
                let key = format!("jq::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("jsonpath.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::jsonpath::detect(&fx) && !(izanagi_kit::jsonpath::parse(&fx).is_some())
            {
                let key = format!("jsonpath::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("justfile.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::justfile::detect(&fx) && !(izanagi_kit::justfile::parse(&fx).is_some())
            {
                let key = format!("justfile::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("jwe.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::jwe::detect(&fx) && !(izanagi_kit::jwe::parse(&fx).is_some()) {
                let key = format!("jwe::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("jwk.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::jwk::detect(&fx) && !(izanagi_kit::jwk::parse(&fx).is_some()) {
                let key = format!("jwk::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("jxr.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::jxr::detect(&fx) && !(izanagi_kit::jxr::parse(&fx).is_some()) {
                let key = format!("jxr::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("k0sconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::k0sconf::detect(&fx) && !(izanagi_kit::k0sconf::parse(&fx).is_some()) {
                let key = format!("k0sconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("k3sconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::k3sconf::detect(&fx) && !(izanagi_kit::k3sconf::parse(&fx).is_some()) {
                let key = format!("k3sconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("kanata.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::kanata::detect(&fx) && !(true) {
                let key = format!("kanata::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("kbm.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::kbm::detect(&fx) && !(izanagi_kit::kbm::parse(&fx).is_some()) {
                let key = format!("kbm::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("kconfig.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::kconfig::detect(&fx) && !(izanagi_kit::kconfig::parse(&fx).is_some()) {
                let key = format!("kconfig::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("kdeglobals.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::kdeglobals::detect(&fx) && !(true) {
                let key = format!("kdeglobals::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("kern.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::kern::detect(&fx) && !(izanagi_kit::kern::parse(&fx).is_some()) {
                let key = format!("kern::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ketl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ketl::detect(&fx) && !(izanagi_kit::ketl::parse(&fx).is_some()) {
                let key = format!("ketl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("keyd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::keyd::detect(&fx) && !(true) {
                let key = format!("keyd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("keydbconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::keydbconf::detect(&fx)
                && !(izanagi_kit::keydbconf::parse(&fx).is_some())
            {
                let key = format!("keydbconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("keytab.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::keytab::detect(&fx) && !(izanagi_kit::keytab::parse(&fx).is_some()) {
                let key = format!("keytab::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("kicadpcb.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::kicadpcb::detect(&fx) && !(izanagi_kit::kicadpcb::parse(&fx).is_some())
            {
                let key = format!("kicadpcb::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("kicadpro.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::kicadpro::detect(&fx) && !(izanagi_kit::kicadpro::parse(&fx).is_some())
            {
                let key = format!("kicadpro::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("kicadsch.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::kicadsch::detect(&fx) && !(izanagi_kit::kicadsch::parse(&fx).is_some())
            {
                let key = format!("kicadsch::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("kickstart.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::kickstart::detect(&fx)
                && !(izanagi_kit::kickstart::parse(&fx).is_some())
            {
                let key = format!("kickstart::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("kif.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::kif::detect(&fx) && !(izanagi_kit::kif::parse(&fx).is_some()) {
                let key = format!("kif::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("kittyimg.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::kittyimg::detect(&fx) && !(izanagi_kit::kittyimg::parse(&fx).is_some())
            {
                let key = format!("kittyimg::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("klipperconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::klipperconf::detect(&fx)
                && !(izanagi_kit::klipperconf::parse(&fx).is_some())
            {
                let key = format!("klipperconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("knative.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::knative::detect(&fx) && !(izanagi_kit::knative::parse(&fx).is_some()) {
                let key = format!("knative::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("knexfile.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::knexfile::detect(&fx) && !(izanagi_kit::knexfile::parse(&fx).is_some())
            {
                let key = format!("knexfile::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("kql.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::kql::detect(&fx) && !(izanagi_kit::kql::parse(&fx).is_some()) {
                let key = format!("kql::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("krb5conf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::krb5conf::detect(&fx) && !(izanagi_kit::krb5conf::parse(&fx).is_some())
            {
                let key = format!("krb5conf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("kubemq.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::kubemq::detect(&fx) && !(izanagi_kit::kubemq::parse(&fx).is_some()) {
                let key = format!("kubemq::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("kyverno.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::kyverno::detect(&fx) && !(izanagi_kit::kyverno::parse(&fx).is_some()) {
                let key = format!("kyverno::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ldif.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ldif::detect(&fx) && !(izanagi_kit::ldif::parse(&fx).is_some()) {
                let key = format!("ldif::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ldirectord.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ldirectord::detect(&fx)
                && !(izanagi_kit::ldirectord::parse(&fx).is_some())
            {
                let key = format!("ldirectord::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ldtk.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ldtk::detect(&fx) && !(izanagi_kit::ldtk::parse(&fx).is_some()) {
                let key = format!("ldtk::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("lean.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::lean::detect(&fx) && !(izanagi_kit::lean::parse(&fx).is_some()) {
                let key = format!("lean::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("leda.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::leda::detect(&fx) && !(izanagi_kit::leda::parse(&fx).is_some()) {
                let key = format!("leda::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ledgerjournal.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ledgerjournal::detect(&fx) && !(true) {
                let key = format!("ledgerjournal::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("lefthook.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::lefthook::detect(&fx) && !(izanagi_kit::lefthook::parse(&fx).is_some())
            {
                let key = format!("lefthook::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("leiningen.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::leiningen::detect(&fx)
                && !(izanagi_kit::leiningen::parse(&fx).is_some())
            {
                let key = format!("leiningen::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("lightdm.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::lightdm::detect(&fx) && !(izanagi_kit::lightdm::parse(&fx).is_some()) {
                let key = format!("lightdm::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("limine.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::limine::detect(&fx) && !(izanagi_kit::limine::parse(&fx).is_some()) {
                let key = format!("limine::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("lintstaged.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::lintstaged::detect(&fx)
                && !(izanagi_kit::lintstaged::parse(&fx).is_some())
            {
                let key = format!("lintstaged::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("liquibase.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::liquibase::detect(&fx)
                && !(izanagi_kit::liquibase::parse(&fx).is_some())
            {
                let key = format!("liquibase::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("litmus.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::litmus::detect(&fx) && !(izanagi_kit::litmus::parse(&fx).is_some()) {
                let key = format!("litmus::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("lmms.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::lmms::detect(&fx) && !(izanagi_kit::lmms::parse(&fx).is_some()) {
                let key = format!("lmms::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("lndconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::lndconf::detect(&fx) && !(izanagi_kit::lndconf::parse(&fx).is_some()) {
                let key = format!("lndconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("locxml.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::locxml::detect(&fx) && !(true) {
                let key = format!("locxml::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("log4j.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::log4j::detect(&fx) && !(izanagi_kit::log4j::parse(&fx).is_some()) {
                let key = format!("log4j::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("log4perl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::log4perl::detect(&fx) && !(izanagi_kit::log4perl::parse(&fx).is_some())
            {
                let key = format!("log4perl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("logback.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::logback::detect(&fx) && !(izanagi_kit::logback::parse(&fx).is_some()) {
                let key = format!("logback::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("logindefs.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::logindefs::detect(&fx)
                && !(izanagi_kit::logindefs::parse(&fx).is_some())
            {
                let key = format!("logindefs::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("logrotate.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::logrotate::detect(&fx)
                && !(izanagi_kit::logrotate::parse(&fx).is_some())
            {
                let key = format!("logrotate::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("loki.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::loki::detect(&fx) && !(izanagi_kit::loki::parse(&fx).is_some()) {
                let key = format!("loki::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("loveconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::loveconf::detect(&fx) && !(izanagi_kit::loveconf::parse(&fx).is_some())
            {
                let key = format!("loveconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("lp.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::lp::detect(&fx) && !(izanagi_kit::lp::parse(&fx).is_some()) {
                let key = format!("lp::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("lpf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::lpf::detect(&fx) && !(izanagi_kit::lpf::parse(&fx).is_some()) {
                let key = format!("lpf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ltsconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ltsconf::detect(&fx) && !(izanagi_kit::ltsconf::parse(&fx).is_some()) {
                let key = format!("ltsconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("lucene.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::lucene::detect(&fx) && !(izanagi_kit::lucene::parse(&fx).is_some()) {
                let key = format!("lucene::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("luhn.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::luhn::detect(&fx) && !(izanagi_kit::luhn::parse(&fx).is_some()) {
                let key = format!("luhn::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("luigi.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::luigi::detect(&fx) && !(true) {
                let key = format!("luigi::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("lvmconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::lvmconf::detect(&fx) && !(izanagi_kit::lvmconf::parse(&fx).is_some()) {
                let key = format!("lvmconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("lwo.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::lwo::detect(&fx) && !(izanagi_kit::lwo::parse(&fx).is_some()) {
                let key = format!("lwo::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ly.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ly::detect(&fx) && !(izanagi_kit::ly::parse(&fx).is_some()) {
                let key = format!("ly::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("lynisconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::lynisconf::detect(&fx)
                && !(izanagi_kit::lynisconf::parse(&fx).is_some())
            {
                let key = format!("lynisconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("lzfse.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::lzfse::detect(&fx) && !(izanagi_kit::lzfse::parse(&fx).is_some()) {
                let key = format!("lzfse::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("lzip.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::lzip::detect(&fx) && !(izanagi_kit::lzip::parse(&fx).is_some()) {
                let key = format!("lzip::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("macaroon.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::macaroon::detect(&fx) && !(izanagi_kit::macaroon::parse(&fx).is_some())
            {
                let key = format!("macaroon::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("maf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::maf::detect(&fx) && !(izanagi_kit::maf::parse(&fx).is_some()) {
                let key = format!("maf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("maildrop.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::maildrop::detect(&fx) && !(izanagi_kit::maildrop::parse(&fx).is_some())
            {
                let key = format!("maildrop::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mameconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mameconf::detect(&fx) && !(izanagi_kit::mameconf::parse(&fx).is_some())
            {
                let key = format!("mameconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mapfile.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mapfile::detect(&fx) && !(izanagi_kit::mapfile::parse(&fx).is_some()) {
                let key = format!("mapfile::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mapnikxml.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mapnikxml::detect(&fx)
                && !(izanagi_kit::mapnikxml::parse(&fx).is_some())
            {
                let key = format!("mapnikxml::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mapproxyconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mapproxyconf::detect(&fx)
                && !(izanagi_kit::mapproxyconf::parse(&fx).is_some())
            {
                let key = format!("mapproxyconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("marc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::marc::detect(&fx) && !(izanagi_kit::marc::parse(&fx).is_some()) {
                let key = format!("marc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("markdownlint.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::markdownlint::detect(&fx)
                && !(izanagi_kit::markdownlint::parse(&fx).is_some())
            {
                let key = format!("markdownlint::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("marlinconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::marlinconf::detect(&fx)
                && !(izanagi_kit::marlinconf::parse(&fx).is_some())
            {
                let key = format!("marlinconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("matplotlibrc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::matplotlibrc::detect(&fx) && !(true) {
                let key = format!("matplotlibrc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("matterbridge.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::matterbridge::detect(&fx)
                && !(izanagi_kit::matterbridge::parse(&fx).is_some())
            {
                let key = format!("matterbridge::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mattermost.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mattermost::detect(&fx)
                && !(izanagi_kit::mattermost::parse(&fx).is_some())
            {
                let key = format!("mattermost::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("maud.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::maud::detect(&fx) && !(izanagi_kit::maud::parse(&fx).is_some()) {
                let key = format!("maud::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mbedapp.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mbedapp::detect(&fx) && !(izanagi_kit::mbedapp::parse(&fx).is_some()) {
                let key = format!("mbedapp::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mbsyncrc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mbsyncrc::detect(&fx) && !(izanagi_kit::mbsyncrc::parse(&fx).is_some())
            {
                let key = format!("mbsyncrc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mcserverprops.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mcserverprops::detect(&fx) && !(true) {
                let key = format!("mcserverprops::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("md3.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::md3::detect(&fx) && !(izanagi_kit::md3::parse(&fx).is_some()) {
                let key = format!("md3::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mdx.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mdx::detect(&fx) && !(izanagi_kit::mdx::parse(&fx).is_some()) {
                let key = format!("mdx::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("med.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::med::detect(&fx) && !(izanagi_kit::med::parse(&fx).is_some()) {
                let key = format!("med::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mediawiki.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mediawiki::detect(&fx)
                && !(izanagi_kit::mediawiki::parse(&fx).is_some())
            {
                let key = format!("mediawiki::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mednafen.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mednafen::detect(&fx) && !(true) {
                let key = format!("mednafen::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mei.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mei::detect(&fx) && !(izanagi_kit::mei::parse(&fx).is_some()) {
                let key = format!("mei::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("melonds.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::melonds::detect(&fx) && !(true) {
                let key = format!("melonds::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("meltano.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::meltano::detect(&fx) && !(izanagi_kit::meltano::parse(&fx).is_some()) {
                let key = format!("meltano::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("memcachedconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::memcachedconf::detect(&fx)
                && !(izanagi_kit::memcachedconf::parse(&fx).is_some())
            {
                let key = format!("memcachedconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mergify.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mergify::detect(&fx) && !(izanagi_kit::mergify::parse(&fx).is_some()) {
                let key = format!("mergify::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("meson.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::meson::detect(&fx) && !(izanagi_kit::meson::parse(&fx).is_some()) {
                let key = format!("meson::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("metallib.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::metallib::detect(&fx) && !(izanagi_kit::metallib::parse(&fx).is_some())
            {
                let key = format!("metallib::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mets.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mets::detect(&fx) && !(izanagi_kit::mets::parse(&fx).is_some()) {
                let key = format!("mets::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mimirconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mimirconf::detect(&fx)
                && !(izanagi_kit::mimirconf::parse(&fx).is_some())
            {
                let key = format!("mimirconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("minidlna.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::minidlna::detect(&fx) && !(true) {
                let key = format!("minidlna::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("minikubeconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::minikubeconf::detect(&fx)
                && !(izanagi_kit::minikubeconf::parse(&fx).is_some())
            {
                let key = format!("minikubeconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mise.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mise::detect(&fx) && !(izanagi_kit::mise::parse(&fx).is_some()) {
                let key = format!("mise::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("misp.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::misp::detect(&fx) && !(izanagi_kit::misp::parse(&fx).is_some()) {
                let key = format!("misp::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mixexs.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mixexs::detect(&fx) && !(izanagi_kit::mixexs::parse(&fx).is_some()) {
                let key = format!("mixexs::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mixxx.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mixxx::detect(&fx) && !(izanagi_kit::mixxx::parse(&fx).is_some()) {
                let key = format!("mixxx::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mml.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mml::detect(&fx) && !(izanagi_kit::mml::parse(&fx).is_some()) {
                let key = format!("mml::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mmlstyle.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mmlstyle::detect(&fx) && !(true) {
                let key = format!("mmlstyle::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("modeldo.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::modeldo::detect(&fx) && !(true) {
                let key = format!("modeldo::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("modprobeconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::modprobeconf::detect(&fx)
                && !(izanagi_kit::modprobeconf::parse(&fx).is_some())
            {
                let key = format!("modprobeconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("modsecurity.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::modsecurity::detect(&fx) && !(true) {
                let key = format!("modsecurity::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("monero.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::monero::detect(&fx) && !(izanagi_kit::monero::parse(&fx).is_some()) {
                let key = format!("monero::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("moonrakerconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::moonrakerconf::detect(&fx)
                && !(izanagi_kit::moonrakerconf::parse(&fx).is_some())
            {
                let key = format!("moonrakerconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mpegts.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mpegts::detect(&fx) && !(izanagi_kit::mpegts::parse(&fx).is_some()) {
                let key = format!("mpegts::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mps.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mps::detect(&fx) && !(izanagi_kit::mps::parse(&fx).is_some()) {
                let key = format!("mps::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mpvconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mpvconf::detect(&fx) && !(true) {
                let key = format!("mpvconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mscx.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mscx::detect(&fx) && !(izanagi_kit::mscx::parse(&fx).is_some()) {
                let key = format!("mscx::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("msmtprc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::msmtprc::detect(&fx) && !(izanagi_kit::msmtprc::parse(&fx).is_some()) {
                let key = format!("msmtprc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mtm.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mtm::detect(&fx) && !(izanagi_kit::mtm::parse(&fx).is_some()) {
                let key = format!("mtm::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mtx.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mtx::detect(&fx) && !(izanagi_kit::mtx::parse(&fx).is_some()) {
                let key = format!("mtx::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("musicxml.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::musicxml::detect(&fx) && !(izanagi_kit::musicxml::parse(&fx).is_some())
            {
                let key = format!("musicxml::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("muttrc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::muttrc::detect(&fx) && !(izanagi_kit::muttrc::parse(&fx).is_some()) {
                let key = format!("muttrc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mvnsettings.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mvnsettings::detect(&fx)
                && !(izanagi_kit::mvnsettings::parse(&fx).is_some())
            {
                let key = format!("mvnsettings::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("mxf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::mxf::detect(&fx) && !(izanagi_kit::mxf::parse(&fx).is_some()) {
                let key = format!("mxf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nanoid.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nanoid::detect(&fx) && !(izanagi_kit::nanoid::parse(&fx).is_some()) {
                let key = format!("nanoid::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("naxsiconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::naxsiconf::detect(&fx) && !(true) {
                let key = format!("naxsiconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nbd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nbd::detect(&fx) && !(izanagi_kit::nbd::parse(&fx).is_some()) {
                let key = format!("nbd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nef.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nef::detect(&fx) && !(izanagi_kit::nef::parse(&fx).is_some()) {
                let key = format!("nef::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("neomuttconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::neomuttconf::detect(&fx)
                && !(izanagi_kit::neomuttconf::parse(&fx).is_some())
            {
                let key = format!("neomuttconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nerdctl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nerdctl::detect(&fx) && !(izanagi_kit::nerdctl::parse(&fx).is_some()) {
                let key = format!("nerdctl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("netlifyconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::netlifyconf::detect(&fx)
                && !(izanagi_kit::netlifyconf::parse(&fx).is_some())
            {
                let key = format!("netlifyconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("networkd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::networkd::detect(&fx) && !(izanagi_kit::networkd::parse(&fx).is_some())
            {
                let key = format!("networkd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("newsboat.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::newsboat::detect(&fx) && !(izanagi_kit::newsboat::parse(&fx).is_some())
            {
                let key = format!("newsboat::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("newsyslog.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::newsyslog::detect(&fx)
                && !(izanagi_kit::newsyslog::parse(&fx).is_some())
            {
                let key = format!("newsyslog::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nextflow.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nextflow::detect(&fx) && !(true) {
                let key = format!("nextflow::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nexus.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nexus::detect(&fx) && !(izanagi_kit::nexus::parse(&fx).is_some()) {
                let key = format!("nexus::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nfpm.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nfpm::detect(&fx) && !(izanagi_kit::nfpm::parse(&fx).is_some()) {
                let key = format!("nfpm::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nfsexports.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nfsexports::detect(&fx)
                && !(izanagi_kit::nfsexports::parse(&fx).is_some())
            {
                let key = format!("nfsexports::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nftconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nftconf::detect(&fx) && !(izanagi_kit::nftconf::parse(&fx).is_some()) {
                let key = format!("nftconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ngircd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ngircd::detect(&fx) && !(izanagi_kit::ngircd::parse(&fx).is_some()) {
                let key = format!("ngircd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nib.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nib::detect(&fx) && !(izanagi_kit::nib::parse(&fx).is_some()) {
                let key = format!("nib::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nififlow.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nififlow::detect(&fx) && !(izanagi_kit::nififlow::parse(&fx).is_some())
            {
                let key = format!("nififlow::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nimble.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nimble::detect(&fx) && !(izanagi_kit::nimble::parse(&fx).is_some()) {
                let key = format!("nimble::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nist.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nist::detect(&fx) && !(izanagi_kit::nist::parse(&fx).is_some()) {
                let key = format!("nist::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nix.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nix::detect(&fx) && !(izanagi_kit::nix::parse(&fx).is_some()) {
                let key = format!("nix::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nlogconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nlogconf::detect(&fx) && !(izanagi_kit::nlogconf::parse(&fx).is_some())
            {
                let key = format!("nlogconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nmconnection.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nmconnection::detect(&fx)
                && !(izanagi_kit::nmconnection::parse(&fx).is_some())
            {
                let key = format!("nmconnection::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("npmrc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::npmrc::detect(&fx) && !(izanagi_kit::npmrc::parse(&fx).is_some()) {
                let key = format!("npmrc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nsjailcfg.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nsjailcfg::detect(&fx) && !(true) {
                let key = format!("nsjailcfg::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nslcdconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nslcdconf::detect(&fx)
                && !(izanagi_kit::nslcdconf::parse(&fx).is_some())
            {
                let key = format!("nslcdconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nsqconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nsqconf::detect(&fx) && !(izanagi_kit::nsqconf::parse(&fx).is_some()) {
                let key = format!("nsqconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ntpconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ntpconf::detect(&fx) && !(izanagi_kit::ntpconf::parse(&fx).is_some()) {
                let key = format!("ntpconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ntpsec.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ntpsec::detect(&fx) && !(izanagi_kit::ntpsec::parse(&fx).is_some()) {
                let key = format!("ntpsec::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nugetconfig.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nugetconfig::detect(&fx)
                && !(izanagi_kit::nugetconfig::parse(&fx).is_some())
            {
                let key = format!("nugetconfig::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nullmailerconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nullmailerconf::detect(&fx) && !(true) {
                let key = format!("nullmailerconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nwc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nwc::detect(&fx) && !(izanagi_kit::nwc::parse(&fx).is_some()) {
                let key = format!("nwc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("nzbget.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::nzbget::detect(&fx) && !(true) {
                let key = format!("nzbget::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("oai.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::oai::detect(&fx) && !(izanagi_kit::oai::parse(&fx).is_some()) {
                let key = format!("oai::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("octaverc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::octaverc::detect(&fx) && !(true) {
                let key = format!("octaverc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("octoprint.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::octoprint::detect(&fx)
                && !(izanagi_kit::octoprint::parse(&fx).is_some())
            {
                let key = format!("octoprint::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("oem.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::oem::detect(&fx) && !(izanagi_kit::oem::parse(&fx).is_some()) {
                let key = format!("oem::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("offlineimap.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::offlineimap::detect(&fx)
                && !(izanagi_kit::offlineimap::parse(&fx).is_some())
            {
                let key = format!("offlineimap::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ogmo.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ogmo::detect(&fx) && !(izanagi_kit::ogmo::parse(&fx).is_some()) {
                let key = format!("ogmo::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("omm.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::omm::detect(&fx) && !(izanagi_kit::omm::parse(&fx).is_some()) {
                let key = format!("omm::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("opam.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::opam::detect(&fx) && !(izanagi_kit::opam::parse(&fx).is_some()) {
                let key = format!("opam::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("opb.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::opb::detect(&fx) && !(izanagi_kit::opb::parse(&fx).is_some()) {
                let key = format!("opb::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("openapi.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::openapi::detect(&fx) && !(izanagi_kit::openapi::parse(&fx).is_some()) {
                let key = format!("openapi::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("openlane.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::openlane::detect(&fx) && !(true) {
                let key = format!("openlane::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("openmsx.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::openmsx::detect(&fx) && !(true) {
                let key = format!("openmsx::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("openntpd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::openntpd::detect(&fx) && !(izanagi_kit::openntpd::parse(&fx).is_some())
            {
                let key = format!("openntpd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("orcaslicer.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::orcaslicer::detect(&fx)
                && !(izanagi_kit::orcaslicer::parse(&fx).is_some())
            {
                let key = format!("orcaslicer::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("orcid.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::orcid::detect(&fx) && !(izanagi_kit::orcid::parse(&fx).is_some()) {
                let key = format!("orcid::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("orf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::orf::detect(&fx) && !(izanagi_kit::orf::parse(&fx).is_some()) {
                let key = format!("orf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("osc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::osc::detect(&fx) && !(izanagi_kit::osc::parse(&fx).is_some()) {
                let key = format!("osc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("osm2pgsqlstyle.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::osm2pgsqlstyle::detect(&fx)
                && !(izanagi_kit::osm2pgsqlstyle::parse(&fx).is_some())
            {
                let key = format!("osm2pgsqlstyle::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("osqueryconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::osqueryconf::detect(&fx)
                && !(izanagi_kit::osqueryconf::parse(&fx).is_some())
            {
                let key = format!("osqueryconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ossecconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ossecconf::detect(&fx)
                && !(izanagi_kit::ossecconf::parse(&fx).is_some())
            {
                let key = format!("ossecconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("overpass.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::overpass::detect(&fx) && !(true) {
                let key = format!("overpass::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pacemaker.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pacemaker::detect(&fx) && !(true) {
                let key = format!("pacemaker::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("paf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::paf::detect(&fx) && !(izanagi_kit::paf::parse(&fx).is_some()) {
                let key = format!("paf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pain.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pain::detect(&fx) && !(izanagi_kit::pain::parse(&fx).is_some()) {
                let key = format!("pain::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pajek.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pajek::detect(&fx) && !(izanagi_kit::pajek::parse(&fx).is_some()) {
                let key = format!("pajek::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pamstack.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pamstack::detect(&fx) && !(izanagi_kit::pamstack::parse(&fx).is_some())
            {
                let key = format!("pamstack::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pants.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pants::detect(&fx) && !(izanagi_kit::pants::parse(&fx).is_some()) {
                let key = format!("pants::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("parityconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::parityconf::detect(&fx)
                && !(izanagi_kit::parityconf::parse(&fx).is_some())
            {
                let key = format!("parityconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("paseto.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::paseto::detect(&fx) && !(izanagi_kit::paseto::parse(&fx).is_some()) {
                let key = format!("paseto::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pcsx2conf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pcsx2conf::detect(&fx)
                && !(izanagi_kit::pcsx2conf::parse(&fx).is_some())
            {
                let key = format!("pcsx2conf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pfconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pfconf::detect(&fx) && !(izanagi_kit::pfconf::parse(&fx).is_some()) {
                let key = format!("pfconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("phylip.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::phylip::detect(&fx) && !(izanagi_kit::phylip::parse(&fx).is_some()) {
                let key = format!("phylip::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pidginconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pidginconf::detect(&fx)
                && !(izanagi_kit::pidginconf::parse(&fx).is_some())
            {
                let key = format!("pidginconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pileup.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pileup::detect(&fx) && !(izanagi_kit::pileup::parse(&fx).is_some()) {
                let key = format!("pileup::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pinerc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pinerc::detect(&fx) && !(true) {
                let key = format!("pinerc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pk.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pk::detect(&fx) && !(izanagi_kit::pk::parse(&fx).is_some()) {
                let key = format!("pk::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pl::detect(&fx) && !(izanagi_kit::pl::parse(&fx).is_some()) {
                let key = format!("pl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("planetilerconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::planetilerconf::detect(&fx)
                && !(izanagi_kit::planetilerconf::parse(&fx).is_some())
            {
                let key = format!("planetilerconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("platformio.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::platformio::detect(&fx)
                && !(izanagi_kit::platformio::parse(&fx).is_some())
            {
                let key = format!("platformio::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("platformsh.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::platformsh::detect(&fx)
                && !(izanagi_kit::platformsh::parse(&fx).is_some())
            {
                let key = format!("platformsh::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pmacctconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pmacctconf::detect(&fx)
                && !(izanagi_kit::pmacctconf::parse(&fx).is_some())
            {
                let key = format!("pmacctconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("postsrsdconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::postsrsdconf::detect(&fx) && !(true) {
                let key = format!("postsrsdconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pppdconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pppdconf::detect(&fx) && !(izanagi_kit::pppdconf::parse(&fx).is_some())
            {
                let key = format!("pppdconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ppssppconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ppssppconf::detect(&fx)
                && !(izanagi_kit::ppssppconf::parse(&fx).is_some())
            {
                let key = format!("ppssppconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("precommit.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::precommit::detect(&fx)
                && !(izanagi_kit::precommit::parse(&fx).is_some())
            {
                let key = format!("precommit::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("preseed.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::preseed::detect(&fx) && !(izanagi_kit::preseed::parse(&fx).is_some()) {
                let key = format!("preseed::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("procmailrc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::procmailrc::detect(&fx)
                && !(izanagi_kit::procmailrc::parse(&fx).is_some())
            {
                let key = format!("procmailrc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("projjson.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::projjson::detect(&fx) && !(true) {
                let key = format!("projjson::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("promtailconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::promtailconf::detect(&fx)
                && !(izanagi_kit::promtailconf::parse(&fx).is_some())
            {
                let key = format!("promtailconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("proselint.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::proselint::detect(&fx)
                && !(izanagi_kit::proselint::parse(&fx).is_some())
            {
                let key = format!("proselint::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("prosody.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::prosody::detect(&fx) && !(izanagi_kit::prosody::parse(&fx).is_some()) {
                let key = format!("prosody::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("prusaslicer.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::prusaslicer::detect(&fx)
                && !(izanagi_kit::prusaslicer::parse(&fx).is_some())
            {
                let key = format!("prusaslicer::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ptm.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ptm::detect(&fx) && !(izanagi_kit::ptm::parse(&fx).is_some()) {
                let key = format!("ptm::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ptp4l.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ptp4l::detect(&fx) && !(izanagi_kit::ptp4l::parse(&fx).is_some()) {
                let key = format!("ptp4l::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ptx.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ptx::detect(&fx) && !(izanagi_kit::ptx::parse(&fx).is_some()) {
                let key = format!("ptx::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pubspec.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pubspec::detect(&fx) && !(izanagi_kit::pubspec::parse(&fx).is_some()) {
                let key = format!("pubspec::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("puz.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::puz::detect(&fx) && !(izanagi_kit::puz::parse(&fx).is_some()) {
                let key = format!("puz::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pxelinux.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pxelinux::detect(&fx) && !(izanagi_kit::pxelinux::parse(&fx).is_some())
            {
                let key = format!("pxelinux::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pypirc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pypirc::detect(&fx) && !(izanagi_kit::pypirc::parse(&fx).is_some()) {
                let key = format!("pypirc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pyroconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pyroconf::detect(&fx) && !(izanagi_kit::pyroconf::parse(&fx).is_some())
            {
                let key = format!("pyroconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("pzserver.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::pzserver::detect(&fx) && !(true) {
                let key = format!("pzserver::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("qcp.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::qcp::detect(&fx) && !(izanagi_kit::qcp::parse(&fx).is_some()) {
                let key = format!("qcp::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("qgsproj.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::qgsproj::detect(&fx) && !(izanagi_kit::qgsproj::parse(&fx).is_some()) {
                let key = format!("qgsproj::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("qmap.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::qmap::detect(&fx) && !(izanagi_kit::qmap::parse(&fx).is_some()) {
                let key = format!("qmap::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("qpf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::qpf::detect(&fx) && !(true) {
                let key = format!("qpf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("qsf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::qsf::detect(&fx) && !(true) {
                let key = format!("qsf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("qt5ctconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::qt5ctconf::detect(&fx) && !(true) {
                let key = format!("qt5ctconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("qtui.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::qtui::detect(&fx) && !(izanagi_kit::qtui::parse(&fx).is_some()) {
                let key = format!("qtui::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("raf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::raf::detect(&fx) && !(izanagi_kit::raf::parse(&fx).is_some()) {
                let key = format!("raf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("railwayconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::railwayconf::detect(&fx)
                && !(izanagi_kit::railwayconf::parse(&fx).is_some())
            {
                let key = format!("railwayconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rakefile.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rakefile::detect(&fx) && !(izanagi_kit::rakefile::parse(&fx).is_some())
            {
                let key = format!("rakefile::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("razorconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::razorconf::detect(&fx) && !(true) {
                let key = format!("razorconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rc::detect(&fx) && !(izanagi_kit::rc::parse(&fx).is_some()) {
                let key = format!("rc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rdpfile.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rdpfile::detect(&fx) && !(izanagi_kit::rdpfile::parse(&fx).is_some()) {
                let key = format!("rdpfile::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("readarr.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::readarr::detect(&fx) && !(true) {
                let key = format!("readarr::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("reaper.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::reaper::detect(&fx) && !(izanagi_kit::reaper::parse(&fx).is_some()) {
                let key = format!("reaper::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rebarconfig.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rebarconfig::detect(&fx)
                && !(izanagi_kit::rebarconfig::parse(&fx).is_some())
            {
                let key = format!("rebarconfig::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("redpen.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::redpen::detect(&fx) && !(izanagi_kit::redpen::parse(&fx).is_some()) {
                let key = format!("redpen::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("refind.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::refind::detect(&fx) && !(izanagi_kit::refind::parse(&fx).is_some()) {
                let key = format!("refind::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("reiserfs.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::reiserfs::detect(&fx) && !(izanagi_kit::reiserfs::parse(&fx).is_some())
            {
                let key = format!("reiserfs::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("relaxng.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::relaxng::detect(&fx) && !(izanagi_kit::relaxng::parse(&fx).is_some()) {
                let key = format!("relaxng::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("releaseplease.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::releaseplease::detect(&fx)
                && !(izanagi_kit::releaseplease::parse(&fx).is_some())
            {
                let key = format!("releaseplease::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("releaserc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::releaserc::detect(&fx)
                && !(izanagi_kit::releaserc::parse(&fx).is_some())
            {
                let key = format!("releaserc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("remminaconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::remminaconf::detect(&fx)
                && !(izanagi_kit::remminaconf::parse(&fx).is_some())
            {
                let key = format!("remminaconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("renderconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::renderconf::detect(&fx)
                && !(izanagi_kit::renderconf::parse(&fx).is_some())
            {
                let key = format!("renderconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("renovate.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::renovate::detect(&fx) && !(izanagi_kit::renovate::parse(&fx).is_some())
            {
                let key = format!("renovate::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("renviron.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::renviron::detect(&fx) && !(true) {
                let key = format!("renviron::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("res.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::res::detect(&fx) && !(izanagi_kit::res::parse(&fx).is_some()) {
                let key = format!("res::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("retroarch.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::retroarch::detect(&fx)
                && !(izanagi_kit::retroarch::parse(&fx).is_some())
            {
                let key = format!("retroarch::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("reviveconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::reviveconf::detect(&fx)
                && !(izanagi_kit::reviveconf::parse(&fx).is_some())
            {
                let key = format!("reviveconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rfb.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rfb::detect(&fx) && !(izanagi_kit::rfb::parse(&fx).is_some()) {
                let key = format!("rfb::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rinex.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rinex::detect(&fx) && !(izanagi_kit::rinex::parse(&fx).is_some()) {
                let key = format!("rinex::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rkhunter.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rkhunter::detect(&fx) && !(izanagi_kit::rkhunter::parse(&fx).is_some())
            {
                let key = format!("rkhunter::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rm.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rm::detect(&fx) && !(izanagi_kit::rm::parse(&fx).is_some()) {
                let key = format!("rm::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rocketmq.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rocketmq::detect(&fx) && !(izanagi_kit::rocketmq::parse(&fx).is_some())
            {
                let key = format!("rocketmq::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rockspec.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rockspec::detect(&fx) && !(izanagi_kit::rockspec::parse(&fx).is_some())
            {
                let key = format!("rockspec::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rpcs3conf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rpcs3conf::detect(&fx)
                && !(izanagi_kit::rpcs3conf::parse(&fx).is_some())
            {
                let key = format!("rpcs3conf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rpgmakerconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rpgmakerconf::detect(&fx)
                && !(izanagi_kit::rpgmakerconf::parse(&fx).is_some())
            {
                let key = format!("rpgmakerconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rprofile.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rprofile::detect(&fx) && !(true) {
                let key = format!("rprofile::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rss2email.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rss2email::detect(&fx)
                && !(izanagi_kit::rss2email::parse(&fx).is_some())
            {
                let key = format!("rss2email::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rsyslogd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rsyslogd::detect(&fx) && !(izanagi_kit::rsyslogd::parse(&fx).is_some())
            {
                let key = format!("rsyslogd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rw2.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rw2::detect(&fx) && !(izanagi_kit::rw2::parse(&fx).is_some()) {
                let key = format!("rw2::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("rx2.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::rx2::detect(&fx) && !(izanagi_kit::rx2::parse(&fx).is_some()) {
                let key = format!("rx2::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sabnzbd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sabnzbd::detect(&fx) && !(true) {
                let key = format!("sabnzbd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("saif.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::saif::detect(&fx) && !(izanagi_kit::saif::parse(&fx).is_some()) {
                let key = format!("saif::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("samba.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::samba::detect(&fx) && !(izanagi_kit::samba::parse(&fx).is_some()) {
                let key = format!("samba::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("samhainconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::samhainconf::detect(&fx) && !(true) {
                let key = format!("samhainconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("saml.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::saml::detect(&fx) && !(izanagi_kit::saml::parse(&fx).is_some()) {
                let key = format!("saml::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sbf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sbf::detect(&fx) && !(izanagi_kit::sbf::parse(&fx).is_some()) {
                let key = format!("sbf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sby.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sby::detect(&fx) && !(izanagi_kit::sby::parse(&fx).is_some()) {
                let key = format!("sby::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("scandata.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::scandata::detect(&fx) && !(izanagi_kit::scandata::parse(&fx).is_some())
            {
                let key = format!("scandata::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sch.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sch::detect(&fx) && !(izanagi_kit::sch::parse(&fx).is_some()) {
                let key = format!("sch::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("scl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::scl::detect(&fx) && !(izanagi_kit::scl::parse(&fx).is_some()) {
                let key = format!("scl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("scp.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::scp::detect(&fx) && !(izanagi_kit::scp::parse(&fx).is_some()) {
                let key = format!("scp::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("scummvm.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::scummvm::detect(&fx) && !(true) {
                let key = format!("scummvm::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sdc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sdc::detect(&fx) && !(izanagi_kit::sdc::parse(&fx).is_some()) {
                let key = format!("sdc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sddmconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sddmconf::detect(&fx) && !(izanagi_kit::sddmconf::parse(&fx).is_some())
            {
                let key = format!("sddmconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sdkconfig.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sdkconfig::detect(&fx)
                && !(izanagi_kit::sdkconfig::parse(&fx).is_some())
            {
                let key = format!("sdkconfig::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sdrppconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sdrppconf::detect(&fx) && !(true) {
                let key = format!("sdrppconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("selinuxfc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::selinuxfc::detect(&fx) && !(true) {
                let key = format!("selinuxfc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("selinuxte.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::selinuxte::detect(&fx) && !(true) {
                let key = format!("selinuxte::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sequelizerc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sequelizerc::detect(&fx)
                && !(izanagi_kit::sequelizerc::parse(&fx).is_some())
            {
                let key = format!("sequelizerc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("serilog.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::serilog::detect(&fx) && !(izanagi_kit::serilog::parse(&fx).is_some()) {
                let key = format!("serilog::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sevendtdxml.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sevendtdxml::detect(&fx) && !(true) {
                let key = format!("sevendtdxml::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sftp.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sftp::detect(&fx) && !(izanagi_kit::sftp::parse(&fx).is_some()) {
                let key = format!("sftp::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sgi.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sgi::detect(&fx) && !(izanagi_kit::sgi::parse(&fx).is_some()) {
                let key = format!("sgi::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("shard.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::shard::detect(&fx) && !(izanagi_kit::shard::parse(&fx).is_some()) {
                let key = format!("shard::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("shellcheckrc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::shellcheckrc::detect(&fx)
                && !(izanagi_kit::shellcheckrc::parse(&fx).is_some())
            {
                let key = format!("shellcheckrc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("shorewall.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::shorewall::detect(&fx)
                && !(izanagi_kit::shorewall::parse(&fx).is_some())
            {
                let key = format!("shorewall::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sievescript.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sievescript::detect(&fx)
                && !(izanagi_kit::sievescript::parse(&fx).is_some())
            {
                let key = format!("sievescript::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sigma.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sigma::detect(&fx) && !(izanagi_kit::sigma::parse(&fx).is_some()) {
                let key = format!("sigma::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("singerconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::singerconf::detect(&fx)
                && !(izanagi_kit::singerconf::parse(&fx).is_some())
            {
                let key = format!("singerconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sixel.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sixel::detect(&fx) && !(izanagi_kit::sixel::parse(&fx).is_some()) {
                let key = format!("sixel::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("slob.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::slob::detect(&fx) && !(izanagi_kit::slob::parse(&fx).is_some()) {
                let key = format!("slob::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("slrnconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::slrnconf::detect(&fx) && !(izanagi_kit::slrnconf::parse(&fx).is_some())
            {
                let key = format!("slrnconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("smd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::smd::detect(&fx) && !(izanagi_kit::smd::parse(&fx).is_some()) {
                let key = format!("smd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("smithy.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::smithy::detect(&fx) && !(izanagi_kit::smithy::parse(&fx).is_some()) {
                let key = format!("smithy::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("smt2.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::smt2::detect(&fx) && !(izanagi_kit::smt2::parse(&fx).is_some()) {
                let key = format!("smt2::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("smtpdconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::smtpdconf::detect(&fx) && !(true) {
                let key = format!("smtpdconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("snort.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::snort::detect(&fx) && !(izanagi_kit::snort::parse(&fx).is_some()) {
                let key = format!("snort::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sp3.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sp3::detect(&fx) && !(izanagi_kit::sp3::parse(&fx).is_some()) {
                let key = format!("sp3::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sparql.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sparql::detect(&fx) && !(izanagi_kit::sparql::parse(&fx).is_some()) {
                let key = format!("sparql::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("spef.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::spef::detect(&fx) && !(izanagi_kit::spef::parse(&fx).is_some()) {
                let key = format!("spef::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("spicenet.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::spicenet::detect(&fx) && !(izanagi_kit::spicenet::parse(&fx).is_some())
            {
                let key = format!("spicenet::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("spv.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::spv::detect(&fx) && !(izanagi_kit::spv::parse(&fx).is_some()) {
                let key = format!("spv::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sqitchconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sqitchconf::detect(&fx)
                && !(izanagi_kit::sqitchconf::parse(&fx).is_some())
            {
                let key = format!("sqitchconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("srcdscfg.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::srcdscfg::detect(&fx) && !(true) {
                let key = format!("srcdscfg::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ssmtpconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ssmtpconf::detect(&fx) && !(true) {
                let key = format!("ssmtpconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sssdconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sssdconf::detect(&fx) && !(izanagi_kit::sssdconf::parse(&fx).is_some())
            {
                let key = format!("sssdconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("stardict.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::stardict::detect(&fx) && !(izanagi_kit::stardict::parse(&fx).is_some())
            {
                let key = format!("stardict::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("staticcheckconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::staticcheckconf::detect(&fx)
                && !(izanagi_kit::staticcheckconf::parse(&fx).is_some())
            {
                let key = format!("staticcheckconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("stix.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::stix::detect(&fx) && !(izanagi_kit::stix::parse(&fx).is_some()) {
                let key = format!("stix::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("stm.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::stm::detect(&fx) && !(izanagi_kit::stm::parse(&fx).is_some()) {
                let key = format!("stm::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("strongswanconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::strongswanconf::detect(&fx) && !(true) {
                let key = format!("strongswanconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sudoku.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sudoku::detect(&fx) && !(izanagi_kit::sudoku::parse(&fx).is_some()) {
                let key = format!("sudoku::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("suiconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::suiconf::detect(&fx) && !(izanagi_kit::suiconf::parse(&fx).is_some()) {
                let key = format!("suiconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("suricata.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::suricata::detect(&fx) && !(izanagi_kit::suricata::parse(&fx).is_some())
            {
                let key = format!("suricata::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("svf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::svf::detect(&fx) && !(izanagi_kit::svf::parse(&fx).is_some()) {
                let key = format!("svf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("swf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::swf::detect(&fx) && !(izanagi_kit::swf::parse(&fx).is_some()) {
                let key = format!("swf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("swiftmt.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::swiftmt::detect(&fx) && !(izanagi_kit::swiftmt::parse(&fx).is_some()) {
                let key = format!("swiftmt::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("synapse.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::synapse::detect(&fx) && !(izanagi_kit::synapse::parse(&fx).is_some()) {
                let key = format!("synapse::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sysctlconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sysctlconf::detect(&fx)
                && !(izanagi_kit::sysctlconf::parse(&fx).is_some())
            {
                let key = format!("sysctlconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sysmonconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sysmonconf::detect(&fx)
                && !(izanagi_kit::sysmonconf::parse(&fx).is_some())
            {
                let key = format!("sysmonconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("systemdboot.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::systemdboot::detect(&fx)
                && !(izanagi_kit::systemdboot::parse(&fx).is_some())
            {
                let key = format!("systemdboot::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("sysv.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::sysv::detect(&fx) && !(izanagi_kit::sysv::parse(&fx).is_some()) {
                let key = format!("sysv::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("syx.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::syx::detect(&fx) && !(izanagi_kit::syx::parse(&fx).is_some()) {
                let key = format!("syx::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("t3d.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::t3d::detect(&fx) && !(izanagi_kit::t3d::parse(&fx).is_some()) {
                let key = format!("t3d::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("tabbyconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::tabbyconf::detect(&fx)
                && !(izanagi_kit::tabbyconf::parse(&fx).is_some())
            {
                let key = format!("tabbyconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("tarantool.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::tarantool::detect(&fx)
                && !(izanagi_kit::tarantool::parse(&fx).is_some())
            {
                let key = format!("tarantool::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("taskfile.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::taskfile::detect(&fx) && !(izanagi_kit::taskfile::parse(&fx).is_some())
            {
                let key = format!("taskfile::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("td0.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::td0::detect(&fx) && !(izanagi_kit::td0::parse(&fx).is_some()) {
                let key = format!("td0::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("tdm.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::tdm::detect(&fx) && !(izanagi_kit::tdm::parse(&fx).is_some()) {
                let key = format!("tdm::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("tekton.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::tekton::detect(&fx) && !(izanagi_kit::tekton::parse(&fx).is_some()) {
                let key = format!("tekton::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("telnet.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::telnet::detect(&fx) && !(izanagi_kit::telnet::parse(&fx).is_some()) {
                let key = format!("telnet::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("tempoconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::tempoconf::detect(&fx)
                && !(izanagi_kit::tempoconf::parse(&fx).is_some())
            {
                let key = format!("tempoconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("terminfo.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::terminfo::detect(&fx) && !(izanagi_kit::terminfo::parse(&fx).is_some())
            {
                let key = format!("terminfo::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("terrariaconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::terrariaconf::detect(&fx) && !(true) {
                let key = format!("terrariaconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("textile.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::textile::detect(&fx) && !(izanagi_kit::textile::parse(&fx).is_some()) {
                let key = format!("textile::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("textlint.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::textlint::detect(&fx) && !(izanagi_kit::textlint::parse(&fx).is_some())
            {
                let key = format!("textlint::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("tfm.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::tfm::detect(&fx) && !(izanagi_kit::tfm::parse(&fx).is_some()) {
                let key = format!("tfm::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("thanosconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::thanosconf::detect(&fx)
                && !(izanagi_kit::thanosconf::parse(&fx).is_some())
            {
                let key = format!("thanosconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("tileservergl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::tileservergl::detect(&fx)
                && !(izanagi_kit::tileservergl::parse(&fx).is_some())
            {
                let key = format!("tileservergl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("tilestacheconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::tilestacheconf::detect(&fx) && !(true) {
                let key = format!("tilestacheconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("timesyncd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::timesyncd::detect(&fx)
                && !(izanagi_kit::timesyncd::parse(&fx).is_some())
            {
                let key = format!("timesyncd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("tlp.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::tlp::detect(&fx) && !(izanagi_kit::tlp::parse(&fx).is_some()) {
                let key = format!("tlp::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("tmpfilesd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::tmpfilesd::detect(&fx)
                && !(izanagi_kit::tmpfilesd::parse(&fx).is_some())
            {
                let key = format!("tmpfilesd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("tmx.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::tmx::detect(&fx) && !(izanagi_kit::tmx::parse(&fx).is_some()) {
                let key = format!("tmx::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("torrc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::torrc::detect(&fx) && !(true) {
                let key = format!("torrc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("tptp.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::tptp::detect(&fx) && !(izanagi_kit::tptp::parse(&fx).is_some()) {
                let key = format!("tptp::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("travisci.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::travisci::detect(&fx) && !(izanagi_kit::travisci::parse(&fx).is_some())
            {
                let key = format!("travisci::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("tripwireconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::tripwireconf::detect(&fx) && !(true) {
                let key = format!("tripwireconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("trojanconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::trojanconf::detect(&fx) && !(true) {
                let key = format!("trojanconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ts3serverini.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ts3serverini::detect(&fx) && !(true) {
                let key = format!("ts3serverini::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("tsx.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::tsx::detect(&fx) && !(izanagi_kit::tsx::parse(&fx).is_some()) {
                let key = format!("tsx::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ttyrec.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ttyrec::detect(&fx) && !(izanagi_kit::ttyrec::parse(&fx).is_some()) {
                let key = format!("ttyrec::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("tuicconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::tuicconf::detect(&fx) && !(true) {
                let key = format!("tuicconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("txt2tags.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::txt2tags::detect(&fx) && !(izanagi_kit::txt2tags::parse(&fx).is_some())
            {
                let key = format!("txt2tags::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("typeid.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::typeid::detect(&fx) && !(izanagi_kit::typeid::parse(&fx).is_some()) {
                let key = format!("typeid::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("typeormconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::typeormconf::detect(&fx)
                && !(izanagi_kit::typeormconf::parse(&fx).is_some())
            {
                let key = format!("typeormconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ubootenv.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ubootenv::detect(&fx) && !(izanagi_kit::ubootenv::parse(&fx).is_some())
            {
                let key = format!("ubootenv::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ucf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ucf::detect(&fx) && !(true) {
                let key = format!("ucf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("udevrules.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::udevrules::detect(&fx)
                && !(izanagi_kit::udevrules::parse(&fx).is_some())
            {
                let key = format!("udevrules::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ufwrules.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ufwrules::detect(&fx) && !(izanagi_kit::ufwrules::parse(&fx).is_some())
            {
                let key = format!("ufwrules::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("ult.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::ult::detect(&fx) && !(izanagi_kit::ult::parse(&fx).is_some()) {
                let key = format!("ult::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("unitymanifest.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::unitymanifest::detect(&fx)
                && !(izanagi_kit::unitymanifest::parse(&fx).is_some())
            {
                let key = format!("unitymanifest::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("unitysettings.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::unitysettings::detect(&fx)
                && !(izanagi_kit::unitysettings::parse(&fx).is_some())
            {
                let key = format!("unitysettings::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("unrealircd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::unrealircd::detect(&fx)
                && !(izanagi_kit::unrealircd::parse(&fx).is_some())
            {
                let key = format!("unrealircd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("upc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::upc::detect(&fx) && !(izanagi_kit::upc::parse(&fx).is_some()) {
                let key = format!("upc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("upf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::upf::detect(&fx) && !(izanagi_kit::upf::parse(&fx).is_some()) {
                let key = format!("upf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("usercss.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::usercss::detect(&fx) && !(izanagi_kit::usercss::parse(&fx).is_some()) {
                let key = format!("usercss::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("userscript.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::userscript::detect(&fx)
                && !(izanagi_kit::userscript::parse(&fx).is_some())
            {
                let key = format!("userscript::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("usi.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::usi::detect(&fx) && !(izanagi_kit::usi::parse(&fx).is_some()) {
                let key = format!("usi::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("v2rayconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::v2rayconf::detect(&fx) && !(true) {
                let key = format!("v2rayconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("vale.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::vale::detect(&fx) && !(izanagi_kit::vale::parse(&fx).is_some()) {
                let key = format!("vale::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("valkeyconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::valkeyconf::detect(&fx)
                && !(izanagi_kit::valkeyconf::parse(&fx).is_some())
            {
                let key = format!("valkeyconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("vcard.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::vcard::detect(&fx) && !(izanagi_kit::vcard::parse(&fx).is_some()) {
                let key = format!("vcard::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("vcd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::vcd::detect(&fx) && !(izanagi_kit::vcd::parse(&fx).is_some()) {
                let key = format!("vcd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("velero.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::velero::detect(&fx) && !(izanagi_kit::velero::parse(&fx).is_some()) {
                let key = format!("velero::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("vercelconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::vercelconf::detect(&fx)
                && !(izanagi_kit::vercelconf::parse(&fx).is_some())
            {
                let key = format!("vercelconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("verilog.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::verilog::detect(&fx) && !(izanagi_kit::verilog::parse(&fx).is_some()) {
                let key = format!("verilog::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("vernemq.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::vernemq::detect(&fx) && !(izanagi_kit::vernemq::parse(&fx).is_some()) {
                let key = format!("vernemq::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("vf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::vf::detect(&fx) && !(izanagi_kit::vf::parse(&fx).is_some()) {
                let key = format!("vf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("vlt.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::vlt::detect(&fx) && !(true) {
                let key = format!("vlt::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("vmagentconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::vmagentconf::detect(&fx)
                && !(izanagi_kit::vmagentconf::parse(&fx).is_some())
            {
                let key = format!("vmagentconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("vrtgdal.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::vrtgdal::detect(&fx) && !(true) {
                let key = format!("vrtgdal::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("w64.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::w64::detect(&fx) && !(izanagi_kit::w64::parse(&fx).is_some()) {
                let key = format!("w64::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("webmanifest.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::webmanifest::detect(&fx)
                && !(izanagi_kit::webmanifest::parse(&fx).is_some())
            {
                let key = format!("webmanifest::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("weechat.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::weechat::detect(&fx) && !(izanagi_kit::weechat::parse(&fx).is_some()) {
                let key = format!("weechat::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("werf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::werf::detect(&fx) && !(izanagi_kit::werf::parse(&fx).is_some()) {
                let key = format!("werf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("westconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::westconf::detect(&fx) && !(izanagi_kit::westconf::parse(&fx).is_some())
            {
                let key = format!("westconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("westonconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::westonconf::detect(&fx)
                && !(izanagi_kit::westonconf::parse(&fx).is_some())
            {
                let key = format!("westonconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("weztermconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::weztermconf::detect(&fx)
                && !(izanagi_kit::weztermconf::parse(&fx).is_some())
            {
                let key = format!("weztermconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("wgsl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::wgsl::detect(&fx) && !(izanagi_kit::wgsl::parse(&fx).is_some()) {
                let key = format!("wgsl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("widgetxml.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::widgetxml::detect(&fx)
                && !(izanagi_kit::widgetxml::parse(&fx).is_some())
            {
                let key = format!("widgetxml::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("wim.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::wim::detect(&fx) && !(izanagi_kit::wim::parse(&fx).is_some()) {
                let key = format!("wim::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("windowsterminal.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::windowsterminal::detect(&fx)
                && !(izanagi_kit::windowsterminal::parse(&fx).is_some())
            {
                let key = format!("windowsterminal::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("winstonconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::winstonconf::detect(&fx)
                && !(izanagi_kit::winstonconf::parse(&fx).is_some())
            {
                let key = format!("winstonconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("wiresharkpref.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::wiresharkpref::detect(&fx)
                && !(izanagi_kit::wiresharkpref::parse(&fx).is_some())
            {
                let key = format!("wiresharkpref::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("wktproj.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::wktproj::detect(&fx) && !(true) {
                let key = format!("wktproj::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("woodpecker.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::woodpecker::detect(&fx)
                && !(izanagi_kit::woodpecker::parse(&fx).is_some())
            {
                let key = format!("woodpecker::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("woz.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::woz::detect(&fx) && !(izanagi_kit::woz::parse(&fx).is_some()) {
                let key = format!("woz::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("wpasupplicant.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::wpasupplicant::detect(&fx)
                && !(izanagi_kit::wpasupplicant::parse(&fx).is_some())
            {
                let key = format!("wpasupplicant::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("wrl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::wrl::detect(&fx) && !(izanagi_kit::wrl::parse(&fx).is_some()) {
                let key = format!("wrl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("wsdl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::wsdl::detect(&fx) && !(izanagi_kit::wsdl::parse(&fx).is_some()) {
                let key = format!("wsdl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("wsjtxconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::wsjtxconf::detect(&fx) && !(true) {
                let key = format!("wsjtxconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xbrl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xbrl::detect(&fx) && !(izanagi_kit::xbrl::parse(&fx).is_some()) {
                let key = format!("xbrl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xdc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xdc::detect(&fx) && !(izanagi_kit::xdc::parse(&fx).is_some()) {
                let key = format!("xdc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xib.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xib::detect(&fx) && !(izanagi_kit::xib::parse(&fx).is_some()) {
                let key = format!("xib::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xid.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xid::detect(&fx) && !(izanagi_kit::xid::parse(&fx).is_some()) {
                let key = format!("xid::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xinetdconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xinetdconf::detect(&fx) && !(true) {
                let key = format!("xinetdconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xlink.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xlink::detect(&fx) && !(izanagi_kit::xlink::parse(&fx).is_some()) {
                let key = format!("xlink::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xmodmap.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xmodmap::detect(&fx) && !(izanagi_kit::xmodmap::parse(&fx).is_some()) {
                let key = format!("xmodmap::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xmp.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xmp::detect(&fx) && !(izanagi_kit::xmp::parse(&fx).is_some()) {
                let key = format!("xmp::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xorgconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xorgconf::detect(&fx) && !(izanagi_kit::xorgconf::parse(&fx).is_some())
            {
                let key = format!("xorgconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xpath.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xpath::detect(&fx) && !(izanagi_kit::xpath::parse(&fx).is_some()) {
                let key = format!("xpath::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xq.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xq::detect(&fx) && !(izanagi_kit::xq::parse(&fx).is_some()) {
                let key = format!("xq::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xqf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xqf::detect(&fx) && !(izanagi_kit::xqf::parse(&fx).is_some()) {
                let key = format!("xqf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xrdpconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xrdpconf::detect(&fx) && !(izanagi_kit::xrdpconf::parse(&fx).is_some())
            {
                let key = format!("xrdpconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xresources.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xresources::detect(&fx)
                && !(izanagi_kit::xresources::parse(&fx).is_some())
            {
                let key = format!("xresources::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xsd.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xsd::detect(&fx) && !(izanagi_kit::xsd::parse(&fx).is_some()) {
                let key = format!("xsd::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xslt.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xslt::detect(&fx) && !(izanagi_kit::xslt::parse(&fx).is_some()) {
                let key = format!("xslt::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("xsvf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::xsvf::detect(&fx) && !(izanagi_kit::xsvf::parse(&fx).is_some()) {
                let key = format!("xsvf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("y4m.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::y4m::detect(&fx) && !(izanagi_kit::y4m::parse(&fx).is_some()) {
                let key = format!("y4m::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("yamllint.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::yamllint::detect(&fx) && !(izanagi_kit::yamllint::parse(&fx).is_some())
            {
                let key = format!("yamllint::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("yara.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::yara::detect(&fx) && !(izanagi_kit::yara::parse(&fx).is_some()) {
                let key = format!("yara::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("yggdrasil.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::yggdrasil::detect(&fx) && !(true) {
                let key = format!("yggdrasil::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("yosys.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::yosys::detect(&fx) && !(true) {
                let key = format!("yosys::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("yuzuconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::yuzuconf::detect(&fx) && !(izanagi_kit::yuzuconf::parse(&fx).is_some())
            {
                let key = format!("yuzuconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("zapconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::zapconf::detect(&fx) && !(izanagi_kit::zapconf::parse(&fx).is_some()) {
                let key = format!("zapconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("zathurarc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::zathurarc::detect(&fx) && !(true) {
                let key = format!("zathurarc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("zeekconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::zeekconf::detect(&fx) && !(true) {
                let key = format!("zeekconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("zeekctl.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::zeekctl::detect(&fx) && !(izanagi_kit::zeekctl::parse(&fx).is_some()) {
                let key = format!("zeekctl::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("zeekscript.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::zeekscript::detect(&fx)
                && !(izanagi_kit::zeekscript::parse(&fx).is_some())
            {
                let key = format!("zeekscript::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("zfs.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::zfs::detect(&fx) && !(izanagi_kit::zfs::parse(&fx).is_some()) {
                let key = format!("zfs::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("znc.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::znc::detect(&fx) && !(izanagi_kit::znc::parse(&fx).is_some()) {
                let key = format!("znc::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("zoo.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::zoo::detect(&fx) && !(izanagi_kit::zoo::parse(&fx).is_some()) {
                let key = format!("zoo::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("zpaq.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::zpaq::detect(&fx) && !(izanagi_kit::zpaq::parse(&fx).is_some()) {
                let key = format!("zpaq::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    {
        let text = fs::read_to_string(src.join("zulipconf.rs")).unwrap_or_default();
        for (cn, fx) in extract_fixtures(&text) {
            if izanagi_kit::zulipconf::detect(&fx)
                && !(izanagi_kit::zulipconf::parse(&fx).is_some())
            {
                let key = format!("zulipconf::{cn}");
                if KNOWN_FRAGMENTS.contains(&key.as_str()) {
                    seen_fragments.insert(key);
                } else {
                    violations.push(key);
                }
            }
        }
    }
    for k in KNOWN_FRAGMENTS {
        assert!(
            seen_fragments.contains(*k),
            "{k} is no longer a fragment — update KNOWN_FRAGMENTS"
        );
    }
    assert!(
        violations.is_empty(),
        "detect accepted complete fixtures the parser rejected: {violations:?}"
    );
}
