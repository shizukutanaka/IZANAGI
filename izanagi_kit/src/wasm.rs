//! WebAssembly binary module container: the `\0asm` magic + version
//! header, then the LEB128-sized section stream (`custom`, `type`,
//! `import`, `function`, `table`, `memory`, `global`, `export`,
//! `start`, `element`, `code`, `data`). Each section is exposed as
//! `(id, payload offset, size)` — the payload itself is left to the
//! caller; `wasm` is a *structural* walk, not an interpreter.
//!
//! `varint` provides the LEB128 length decode.
//!
//! ```
//! use izanagi_kit::wasm;
//! let mut m = b"\0asm".to_vec();
//! m.extend_from_slice(&[1, 0, 0, 0]); // version 1
//! m.extend_from_slice(&[1, 4, 1, 0x60, 0, 0]); // type section: 1 func ()->()
//! let w = wasm::parse(&m).unwrap();
//! assert_eq!(w.version, 1);
//! assert_eq!(w.sections.len(), 1);
//! assert_eq!(w.sections[0].id, 1);
//! assert_eq!(w.sections[0].name(), "type");
//! ```

/// One WebAssembly section header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    /// Section ID (0=custom, 1=type, 2=import, 3=function, 4=table,
    /// 5=memory, 6=global, 7=export, 8=start, 9=element, 10=code,
    /// 11=data, 12=data-count).
    pub id: u8,
    /// File offset of the section payload (after the id+size header).
    pub offset: usize,
    /// Payload size in bytes.
    pub size: usize,
}

impl Section {
    /// Canonical name for the section ID.
    pub fn name(&self) -> &'static str {
        match self.id {
            0 => "custom",
            1 => "type",
            2 => "import",
            3 => "function",
            4 => "table",
            5 => "memory",
            6 => "global",
            7 => "export",
            8 => "start",
            9 => "element",
            10 => "code",
            11 => "data",
            12 => "data-count",
            _ => "unknown",
        }
    }
}

/// A parsed WebAssembly module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wasm {
    /// Module version (currently always 1).
    pub version: u32,
    /// All sections in file order.
    pub sections: Vec<Section>,
}

/// Decode a LEB128 u32 at `at`; returns `(value, consumed)`.
fn leb(d: &[u8], at: usize) -> Option<(u32, usize)> {
    let mut v: u32 = 0;
    let mut shift = 0u32;
    let mut n = 0usize;
    loop {
        let b = *d.get(at + n)?;
        v |= ((b & 0x7F) as u32).checked_shl(shift).unwrap_or(0);
        n += 1;
        if b & 0x80 == 0 {
            return Some((v, n));
        }
        shift += 7;
        if shift > 28 {
            return None; // u32 LEB128 can be at most 5 bytes
        }
    }
}

/// Parse a WebAssembly binary module: magic + version + section table.
/// `None` on bad magic/version, a section running past EOF, or a
/// malformed LEB128 length.
pub fn parse(d: &[u8]) -> Option<Wasm> {
    if d.get(..4)? != b"\0asm" {
        return None;
    }
    let version =
        (d[4] as u32) | ((d[5] as u32) << 8) | ((d[6] as u32) << 16) | ((d[7] as u32) << 24);
    if version != 1 {
        return None;
    }
    let mut at = 8usize;
    let mut sections = Vec::new();
    while at < d.len() {
        let id = *d.get(at)?;
        if id > 12 {
            return None; // IDs above 12 are not part of MVP wasm
        }
        let (size, used) = leb(d, at + 1)?;
        let offset = at + 1 + used;
        let size = size as usize;
        let end = offset.checked_add(size)?;
        if end > d.len() {
            return None;
        }
        sections.push(Section { id, offset, size });
        at = end;
    }
    Some(Wasm { version, sections })
}

/// The first section with `id`, or `None`.
pub fn section(w: &Wasm, id: u8) -> Option<&Section> {
    w.sections.iter().find(|s| s.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal() -> Vec<u8> {
        let mut m = b"\0asm".to_vec();
        m.extend_from_slice(&[1, 0, 0, 0]);
        // type section: 1 entry, func () -> ()
        m.extend_from_slice(&[1, 4, 1, 0x60, 0, 0]);
        // function section: 1 entry = type index 0
        m.extend_from_slice(&[3, 2, 1, 0]);
        // code section: 1 entry of 4 bytes (locals=0, body=end)
        m.extend_from_slice(&[10, 4, 1, 2, 0, 0x0B]);
        m
    }

    #[test]
    fn module_walk() {
        let w = parse(&minimal()).unwrap();
        assert_eq!(w.version, 1);
        assert_eq!(w.sections.len(), 3);
        assert_eq!(w.sections[0].id, 1);
        assert_eq!(w.sections[0].size, 4);
        assert_eq!(w.sections[0].name(), "type");
        assert_eq!(w.sections[1].name(), "function");
        assert_eq!(w.sections[2].name(), "code");
        assert_eq!(section(&w, 7), None);
        assert_eq!(section(&w, 10).unwrap().id, 10);
    }

    #[test]
    fn custom_and_all_ids() {
        let mut m = b"\0asm".to_vec();
        m.extend_from_slice(&[1, 0, 0, 0]);
        for id in 0..=12u8 {
            m.push(id);
            m.push(0); // empty payload
        }
        let w = parse(&m).unwrap();
        assert_eq!(w.sections.len(), 13);
        assert_eq!(w.sections[12].name(), "data-count");
        assert_eq!(w.sections[0].name(), "custom");
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"\0asx").is_none()); // bad magic
        assert!(parse(b"\0asm\x02\0\0\0").is_none()); // version 2
        let mut bad = minimal();
        bad.truncate(12); // cut inside section payload
        assert!(parse(&bad).is_none());
        let mut bad2 = b"\0asm".to_vec();
        bad2.extend_from_slice(&[1, 0, 0, 0]);
        bad2.push(13); // invalid section id
        assert!(parse(&bad2).is_none());
        // LEB128 overflow (6 continuation bytes)
        let mut bad3 = b"\0asm".to_vec();
        bad3.extend_from_slice(&[1, 0, 0, 0]);
        bad3.push(0);
        bad3.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        assert!(parse(&bad3).is_none());
    }
}
