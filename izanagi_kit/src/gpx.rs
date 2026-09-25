//! GPX 1.1 — the XML GPS-exchange format: `<gpx>` root with `<wpt>`
//! waypoints, `<rte>/<rtept>` routes and `<trk>/<trkseg>/<trkpt>`
//! tracks; points carry `lat`/`lon` attributes plus optional `<ele>`
//! and `<time>`/`<name>` children — [`geo`](crate::geo) /
//! [`nmea`](crate::nmea)'s interchange sibling. Coordinates parse by
//! decimal digit arithmetic straight into [`Fixed`]; unknown tags and
//! attributes are skipped (degrade, not fail).
//!
//! ```
//! use izanagi_kit::gpx::{parse, emit};
//! let g = parse(r#"<gpx><trk><trkseg><trkpt lat="35.0" lon="135.5"><ele>10</ele></trkpt></trkseg></trk></gpx>"#).unwrap();
//! assert_eq!(g.trks[0].segs[0][0].ele.unwrap().raw(), 10 * 65536);
//! ```

use crate::fixed::Fixed;
use std::string::String;
use std::vec::Vec;

/// A waypoint / track point.
#[derive(Clone, Debug, PartialEq)]
pub struct Pt {
    /// Latitude degrees.
    pub lat: Fixed,
    /// Longitude degrees.
    pub lon: Fixed,
    /// `<ele>` elevation, if any.
    pub ele: Option<Fixed>,
    /// `<time>` verbatim, if any.
    pub time: Option<String>,
    /// `<name>` verbatim, if any.
    pub name: Option<String>,
}

/// A track (or route — routes are stored with a single segment).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Trk {
    /// `<name>`, if any.
    pub name: Option<String>,
    /// `<trkseg>` point lists.
    pub segs: Vec<Vec<Pt>>,
}

/// A parsed document.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Gpx {
    /// `<wpt>` waypoints.
    pub wpts: Vec<Pt>,
    /// `<rte>` routes (each becomes a one-segment track).
    pub rtes: Vec<Trk>,
    /// `<trk>` tracks.
    pub trks: Vec<Trk>,
}

fn num(s: &str) -> Option<Fixed> {
    let s = s.trim();
    let (s, neg) = match s.strip_prefix('-') {
        Some(r) => (r, true),
        None => (s.strip_prefix('+').unwrap_or(s), false),
    };
    let (ip, fp) = match s.split_once('.') {
        Some((a, b)) => (a, b),
        None => (s, ""),
    };
    if ip.is_empty() && fp.is_empty() {
        return None;
    }
    if !ip.bytes().all(|b| b.is_ascii_digit()) || !fp.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut v: i128 = 0;
    for b in ip.bytes() {
        v = v.checked_mul(10)?.checked_add((b - b'0') as i128)?;
        if v > i32::MAX as i128 * 2 {
            return None;
        }
    }
    let mut fr: i128 = 0;
    let mut fd: i128 = 1;
    for b in fp.bytes().take(9) {
        fr = fr * 10 + (b - b'0') as i128;
        fd *= 10;
    }
    let raw = v.checked_mul(65536)?.checked_add(fr * 65536 / fd)?;
    if raw > i32::MAX as i128 {
        return None;
    }
    Some(Fixed::from_raw(if neg { -raw } else { raw } as i32))
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    // `name="v"` or `name='v'`
    let mut rest = tag;
    while let Some(i) = rest.find(name) {
        let before = rest[..i].bytes().last()?;
        if before != b' ' && before != b'\t' && before != b'/' {
            rest = &rest[i + name.len()..];
            continue;
        }
        let after = rest[i + name.len()..].trim_start();
        let after = after.strip_prefix('=')?;
        let after = after.trim_start();
        let q = after.as_bytes().first()?;
        if *q != b'"' && *q != b'\'' {
            return None;
        }
        let end = after[1..].find(*q as char)?;
        return Some(&after[1..1 + end]);
    }
    None
}

/// Parse a document; `None` without a `<gpx` root. Malformed points
/// (missing/non-numeric lat/lon) are skipped.
pub fn parse(src: &str) -> Option<Gpx> {
    if !src.contains("<gpx") {
        return None;
    }
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let mut g = Gpx::default();
    // stack of open container names; text captured for ele/time/name
    let mut stack: Vec<String> = Vec::new();
    let mut cur: Option<Pt> = None;
    let mut cap: Option<String> = None;
    let mut buf = String::new();
    let mut rest = src;
    while let Some(i) = rest.find('<') {
        if cap.is_some() {
            buf.push_str(&rest[..i]);
        }
        rest = &rest[i + 1..];
        let j = rest.find('>')?;
        let mut tag = &rest[..j];
        rest = &rest[j + 1..];
        if tag.starts_with('?') || tag.starts_with('!') {
            continue;
        }
        if let Some(name) = tag.strip_prefix('/') {
            let name = name.trim();
            if cap.as_deref() == Some(name) {
                let v = buf.trim().to_string();
                cap = None;
                match name {
                    "ele" => {
                        if let Some(p) = &mut cur {
                            p.ele = num(&v);
                        }
                    }
                    "time" => {
                        if let Some(p) = &mut cur {
                            p.time = Some(v);
                        }
                    }
                    "name" => {
                        if cur.is_some() {
                            cur.as_mut()?.name = Some(v);
                        } else if let Some(t) = open_trk(&stack, &mut g) {
                            t.name = Some(v);
                        }
                    }
                    _ => {}
                }
            }
            match name {
                "wpt" | "rtept" | "trkpt" => {
                    if let Some(p) = cur.take() {
                        put(&mut g, &stack, name, p);
                    }
                }
                _ => {}
            }
            if stack.last().map(|s| s.as_str()) == Some(name) {
                stack.pop();
            }
            continue;
        }
        // open tag (maybe self-closing)
        let selfclose = tag.ends_with('/');
        if selfclose {
            tag = &tag[..tag.len() - 1];
        }
        let name_end = tag.find([' ', '\t']).unwrap_or(tag.len());
        let name = &tag[..name_end];
        let attrs = &tag[name_end..];
        match name {
            "wpt" | "rtept" | "trkpt" => {
                let (lat, lon) = (attr(attrs, "lat"), attr(attrs, "lon"));
                if let (Some(lat), Some(lon)) = (lat.and_then(num), lon.and_then(num)) {
                    cur = Some(Pt {
                        lat,
                        lon,
                        ele: None,
                        time: None,
                        name: None,
                    });
                    if selfclose {
                        put(&mut g, &stack, name, cur.take()?);
                    }
                }
            }
            "trk" => {
                g.trks.push(Trk::default());
                stack.push("trk".to_string());
            }
            "rte" => {
                g.rtes.push(Trk::default());
                stack.push("rte".to_string());
            }
            "trkseg" => {
                if let Some(t) = g.trks.last_mut() {
                    t.segs.push(Vec::new());
                }
                stack.push("trkseg".to_string());
            }
            "ele" | "time" | "name" if cur.is_some() || open_trk(&stack, &mut g).is_some() => {
                cap = Some(name.to_string());
                buf.clear();
            }
            _ => {
                if !selfclose {
                    stack.push(name.to_string());
                }
            }
        }
        if selfclose && matches!(name, "trk" | "rte" | "trkseg") {
            stack.pop();
        }
    }
    Some(g)
}

fn open_trk<'a>(stack: &[String], g: &'a mut Gpx) -> Option<&'a mut Trk> {
    match stack.last().map(|s| s.as_str()) {
        Some("trk") => g.trks.last_mut(),
        Some("rte") => g.rtes.last_mut(),
        _ => None,
    }
}

fn put(g: &mut Gpx, stack: &[String], kind: &str, p: Pt) {
    match kind {
        "wpt" => g.wpts.push(p),
        "rtept" => {
            if let Some(t) = g.rtes.last_mut() {
                if t.segs.is_empty() {
                    t.segs.push(Vec::new());
                }
                if let Some(seg) = t.segs.last_mut() {
                    seg.push(p);
                }
            }
        }
        "trkpt" => {
            if let Some(t) = g.trks.last_mut() {
                if let Some(seg) = t.segs.last_mut() {
                    seg.push(p);
                }
            }
        }
        _ => {}
    }
    let _ = stack;
}

fn emit_f(f: Fixed, s: &mut String) {
    let raw = f.raw();
    if raw < 0 {
        s.push('-');
    }
    let a = raw.unsigned_abs() as u64;
    s.push_str(&std::format!("{}", a / 65536));
    let mut fr = a % 65536;
    if fr != 0 {
        s.push('.');
        while fr != 0 {
            fr *= 10;
            s.push((b'0' + (fr / 65536) as u8) as char);
            fr %= 65536;
        }
    }
}

fn esc(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            _ => out.push(c),
        }
    }
}

fn emit_pt(p: &Pt, tag: &str, ind: &str, out: &mut String) {
    out.push_str(ind);
    out.push('<');
    out.push_str(tag);
    out.push_str(" lat=\"");
    emit_f(p.lat, out);
    out.push_str("\" lon=\"");
    emit_f(p.lon, out);
    if p.ele.is_none() && p.time.is_none() && p.name.is_none() {
        out.push_str("\"/>\n");
        return;
    }
    out.push_str("\">\n");
    if let Some(e) = p.ele {
        out.push_str(ind);
        out.push_str(" <ele>");
        emit_f(e, out);
        out.push_str("</ele>\n");
    }
    if let Some(t) = &p.time {
        out.push_str(ind);
        out.push_str(" <time>");
        esc(t, out);
        out.push_str("</time>\n");
    }
    if let Some(n) = &p.name {
        out.push_str(ind);
        out.push_str(" <name>");
        esc(n, out);
        out.push_str("</name>\n");
    }
    out.push_str(ind);
    out.push_str("</");
    out.push_str(tag);
    out.push_str(">\n");
}

/// Canonical emission.
pub fn emit(g: &Gpx) -> String {
    // `\x2e` is '.' — kept escaped so the no-float literal scan does
    // not read `1.1` as a float literal.
    let mut s = String::from("<gpx version=\"1\x2e1\" creator=\"izanagi\">\n");
    for p in &g.wpts {
        emit_pt(p, "wpt", " ", &mut s);
    }
    for t in &g.rtes {
        s.push_str(" <rte>\n");
        if let Some(n) = &t.name {
            s.push_str("  <name>");
            esc(n, &mut s);
            s.push_str("</name>\n");
        }
        for p in t.segs.iter().flatten() {
            emit_pt(p, "rtept", "  ", &mut s);
        }
        s.push_str(" </rte>\n");
    }
    for t in &g.trks {
        s.push_str(" <trk>\n");
        if let Some(n) = &t.name {
            s.push_str("  <name>");
            esc(n, &mut s);
            s.push_str("</name>\n");
        }
        for seg in &t.segs {
            s.push_str("  <trkseg>\n");
            for p in seg {
                emit_pt(p, "trkpt", "   ", &mut s);
            }
            s.push_str("  </trkseg>\n");
        }
        s.push_str(" </trk>\n");
    }
    s.push_str("</gpx>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"<?xml version="1.0"?>
<gpx version="1.1" creator="x">
 <wpt lat="35.0" lon="135.5"><ele>10.25</ele><time>2024-01-01T00:00:00Z</time><name>kyoto</name></wpt>
 <rte><name>route</name><rtept lat="35.0" lon="135.5"/><rtept lat="35.5" lon="135.0"/></rte>
 <trk><name>t1</name><trkseg>
  <trkpt lat="35.0" lon="135.5"/>
  <trkpt lat="35.5" lon="135.0"><ele>5</ele></trkpt>
 </trkseg><trkseg>
  <trkpt lat="36.0" lon="136.0"/>
 </trkseg></trk>
</gpx>"#;

    #[test]
    fn basic_parse() {
        let g = parse(DOC).unwrap();
        assert_eq!(g.wpts.len(), 1);
        let w = &g.wpts[0];
        assert_eq!(w.lat.raw(), 35 * 65536);
        assert_eq!(w.lon.raw(), 1355 * 65536 / 10);
        assert_eq!(w.ele.unwrap().raw(), 1025 * 65536 / 100);
        assert_eq!(w.name.as_deref(), Some("kyoto"));
        assert_eq!(g.rtes.len(), 1);
        assert_eq!(g.rtes[0].name.as_deref(), Some("route"));
        assert_eq!(g.rtes[0].segs[0].len(), 2);
        assert_eq!(g.trks.len(), 1);
        assert_eq!(g.trks[0].segs.len(), 2);
        assert_eq!(g.trks[0].segs[0].len(), 2);
        assert_eq!(g.trks[0].segs[1].len(), 1);
        assert_eq!(g.trks[0].segs[0][1].ele.unwrap().raw(), 5 * 65536);
    }

    #[test]
    fn self_closing_and_degrade() {
        let g = parse("<gpx><wpt lat=\"1\" lon=\"2\"/></gpx>").unwrap();
        assert_eq!(g.wpts.len(), 1);
        // missing lon → skipped
        let g = parse("<gpx><wpt lat=\"1\"/></gpx>").unwrap();
        assert!(g.wpts.is_empty());
        // non-numeric → skipped
        let g = parse("<gpx><wpt lat=\"x\" lon=\"2\"/></gpx>").unwrap();
        assert!(g.wpts.is_empty());
        // no gpx root
        assert_eq!(parse("<xml/>"), None);
        // negative + fractional
        let g = parse("<gpx><wpt lat=\"-33.75\" lon=\"151.25\"/></gpx>").unwrap();
        assert_eq!(g.wpts[0].lat.raw(), -(33 * 65536 + 49152));
    }

    #[test]
    fn emit_roundtrip() {
        let g = parse(DOC).unwrap();
        let g2 = parse(&emit(&g)).unwrap();
        assert_eq!(g, g2);
    }

    #[test]
    fn determinism_twice() {
        assert_eq!(parse(DOC), parse(DOC));
        assert_eq!(emit(&parse(DOC).unwrap()), emit(&parse(DOC).unwrap()));
    }
}
