//! Kerberos `keytab` — `0x05 0x02` (v2) or `0x05 0x01` header then
//! `[u32be len][entry]` records: principal components, name type,
//! timestamp, key version, enctype and key bytes.
//!
//! ```
//! let mut d = vec![0x05, 0x02];
//! let mut e = vec![0, 0, 0, 28];
//! e.extend_from_slice(&[0, 1, 0, 3, b'a', b'b', b'c', 0, 4, b'r', b'e', b'a', b'l',
//!     0, 0, 0, 2, 0, 0, 0, 0, 1, 0, 18, 0, 2, 0, 0]);
//! d.append(&mut e);
//! let k = izanagi_kit::keytab::parse(&d).unwrap();
//! assert_eq!(k.version, 2);
//! assert_eq!(k.entries, 1);
//! assert!(izanagi_kit::keytab::detect(&d));
//! ```

/// Census of a Kerberos keytab.
#[derive(Debug, Clone)]
pub struct Keytab {
    /// Format version (1 or 2).
    pub version: u8,
    /// Successfully walked entries.
    pub entries: usize,
    /// Total bytes consumed by entries.
    pub entry_bytes: usize,
    /// Max principal component count seen.
    pub max_components: usize,
    /// Distinct enctypes seen (count).
    pub enctypes: usize,
    /// Truncated/garbage trailing bytes.
    pub trailing: usize,
}

fn u16be(b: &[u8]) -> u32 {
    (b[0] as u32) << 8 | b[1] as u32
}

fn u32be(b: &[u8]) -> u32 {
    (b[0] as u32) << 24 | (b[1] as u32) << 16 | (b[2] as u32) << 8 | b[3] as u32
}

/// Detects a keytab: `0x05 0x02`/`0x05 0x01` magic.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() >= 2 && b[0] == 0x05 && (b[1] == 0x02 || b[1] == 0x01)
}

/// Walks keytab entries; `None` on bad magic or a truncated first entry.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Keytab> {
    if !detect(b) {
        return None;
    }
    let mut k = Keytab {
        version: b[1],
        entries: 0,
        entry_bytes: 0,
        max_components: 0,
        enctypes: 0,
        trailing: 0,
    };
    let mut seen = Vec::new();
    let mut i = 2usize;
    while i + 4 <= b.len() {
        let len = u32be(&b[i..i + 4]) as usize;
        if len == 0 {
            i += 4;
            continue;
        }
        let end = i + 4 + len;
        if end > b.len() || len < 10 {
            k.trailing = b.len() - i;
            break;
        }
        let e = &b[i + 4..end];
        // entry: u16 components + strings + u32 name_type + u32 ts +
        // u8 kvno + u16 enctype + u16 keylen + key
        let comps = u16be(e);
        let mut j = 2usize;
        let mut ok = comps > 0 && comps <= 32;
        for _ in 0..comps.saturating_add(1) {
            // components then realm string (v2: realm first? both counted)
            if j + 2 > e.len() {
                ok = false;
                break;
            }
            let sl = u16be(&e[j..j + 2]) as usize;
            j += 2 + sl;
        }
        // after realm+component strings: u32 name_type + u32 ts + u8 kvno
        // + u16 enctype + u16 keylen + key bytes
        if ok && j + 13 <= e.len() {
            let enctype = u16be(&e[j + 9..j + 11]);
            let keylen = u16be(&e[j + 11..j + 13]) as usize;
            if j + 13 + keylen <= e.len() {
                if !seen.contains(&enctype) {
                    seen.push(enctype);
                }
                k.entries += 1;
                k.entry_bytes += len;
                if comps as usize > k.max_components {
                    k.max_components = comps as usize;
                }
            }
        }
        i = end;
    }
    k.enctypes = seen.len();
    (k.entries > 0).then_some(k)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(comps: &[&[u8]], realm: &[u8], enctype: u32, key: &[u8]) -> Vec<u8> {
        let mut e = Vec::new();
        e.extend_from_slice(&(comps.len() as u16).to_be_bytes());
        e.extend_from_slice(&(realm.len() as u16).to_be_bytes());
        e.extend_from_slice(realm);
        for c in comps {
            e.extend_from_slice(&(c.len() as u16).to_be_bytes());
            e.extend_from_slice(c);
        }
        e.extend_from_slice(&2u32.to_be_bytes()); // name type
        e.extend_from_slice(&0u32.to_be_bytes()); // timestamp
        e.push(1); // kvno
        e.extend_from_slice(&(enctype as u16).to_be_bytes());
        e.extend_from_slice(&(key.len() as u16).to_be_bytes());
        e.extend_from_slice(key);
        let mut out = (e.len() as u32).to_be_bytes().to_vec();
        out.append(&mut e);
        out
    }

    #[test]
    fn parses() {
        let mut d = vec![0x05, 0x02];
        d.append(&mut entry(&[b"a"], b"REALM", 18, &[0, 1]));
        d.append(&mut entry(&[b"x", b"y"], b"R2", 17, &[9]));
        let k = parse(&d).unwrap();
        assert_eq!(k.version, 2);
        assert_eq!(k.entries, 2);
        assert_eq!(k.max_components, 2);
        assert_eq!(k.enctypes, 2);
        assert_eq!(k.trailing, 0);
    }

    #[test]
    fn trailing() {
        let mut d = vec![0x05, 0x01];
        d.append(&mut entry(&[b"a"], b"R", 3, &[0]));
        d.extend_from_slice(&[0, 0, 0, 2, 0, 1]); // truncated next entry
        let k = parse(&d).unwrap();
        assert_eq!(k.entries, 1);
        assert_eq!(k.trailing, 6);
    }

    #[test]
    fn detect_works() {
        assert!(detect(&[0x05, 0x02]));
        assert!(detect(&[0x05, 0x01, 0]));
        assert!(!detect(&[0x05, 0x03]));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0x05, 0x02, 0, 0, 0, 9]).is_none());
    }
}
