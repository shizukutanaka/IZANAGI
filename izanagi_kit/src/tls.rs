//! TLS 1.x record layer (RFC 5246 §6.2 / RFC 8446 §5.1): each record is
//! `content_type u8 ‖ legacy_version u16 ‖ length u16 ‖ fragment`.
//! `parse_records` walks the whole stream; `Record::name()` labels the
//! type (`change-cipher-spec`, `alert`, `handshake`, `application-data`).
//! `handshake` splits a handshake fragment into `(type, body)` messages.
//!
//! This is a *codec*, not a cipher — records are walked, not decrypted.
//!
//! ```
//! use izanagi_kit::tls;
//! let mut f = vec![22, 3, 3, 0, 5, 1, 0, 0, 1, 0]; // handshake, ClientHello
//! f.extend_from_slice(&[23, 3, 3, 0, 1, 0xAA]);   // app data
//! let r = tls::records(&f).unwrap();
//! assert_eq!(r.len(), 2);
//! assert_eq!(r[0].name(), "handshake");
//! assert_eq!(r[1].name(), "application-data");
//! ```

/// Content-type byte → name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    /// `20` CCS, `21` Alert, `22` Handshake, `23` AppData.
    pub ty: u8,
    /// `0x0301` TLS1.0, `0x0303` TLS1.2, `0x0304` TLS1.3.
    pub version: u16,
    /// Fragment bytes (max 16384 per RFC).
    pub fragment: Vec<u8>,
}

impl Record {
    /// Human-readable content-type name.
    pub fn name(&self) -> &'static str {
        match self.ty {
            20 => "change-cipher-spec",
            21 => "alert",
            22 => "handshake",
            23 => "application-data",
            _ => "unknown",
        }
    }
}

/// A handshake sub-message inside a Handshake record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Handshake {
    /// `1` ClientHello, `2` ServerHello, `4` NewSessionTicket,
    /// `11` Certificate, `20` Finished, …
    pub ty: u8,
    /// Handshake body bytes.
    pub body: Vec<u8>,
}

impl Handshake {
    /// Human-readable handshake-type name.
    pub fn name(&self) -> &'static str {
        match self.ty {
            0 => "hello-request",
            1 => "client-hello",
            2 => "server-hello",
            4 => "new-session-ticket",
            8 => "encrypted-extensions",
            11 => "certificate",
            12 => "server-key-exchange",
            13 => "certificate-request",
            14 => "server-hello-done",
            15 => "certificate-verify",
            16 => "client-key-exchange",
            20 => "finished",
            _ => "unknown",
        }
    }
}

/// Walk a byte stream as a sequence of TLS records. `None` on a
/// truncated record, a non-TLS content type, or a record length that
/// exceeds the protocol's 16384 cap.
pub fn records(d: &[u8]) -> Option<Vec<Record>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < d.len() {
        let ty = *d.get(i)?;
        match ty {
            20..=23 => {}
            _ => return None,
        }
        let version = ((*d.get(i + 1)? as u16) << 8) | *d.get(i + 2)? as u16;
        let len = ((*d.get(i + 3)? as usize) << 8) | *d.get(i + 4)? as usize;
        if len > 16384 {
            return None;
        }
        let start = i + 5;
        let end = start.checked_add(len)?;
        if end > d.len() {
            return None;
        }
        out.push(Record {
            ty,
            version,
            fragment: d[start..end].to_vec(),
        });
        i = end;
    }
    Some(out)
}

/// Emit a single record: `ty ‖ version ‖ len ‖ fragment`.
pub fn emit(ty: u8, version: u16, fragment: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(5 + fragment.len());
    out.push(ty);
    out.push((version >> 8) as u8);
    out.push(version as u8);
    out.push((fragment.len() >> 8) as u8);
    out.push(fragment.len() as u8);
    out.extend_from_slice(fragment);
    out
}

/// Split a Handshake record's fragment into `(type, body)` messages:
/// `type u8 ‖ len u24 ‖ body`.
pub fn handshake(d: &[u8]) -> Option<Vec<Handshake>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < d.len() {
        let ty = *d.get(i)?;
        let len = ((*d.get(i + 1)? as usize) << 16)
            | ((*d.get(i + 2)? as usize) << 8)
            | *d.get(i + 3)? as usize;
        let start = i + 4;
        let end = start.checked_add(len)?;
        if end > d.len() {
            return None;
        }
        out.push(Handshake {
            ty,
            body: d[start..end].to_vec(),
        });
        i = end;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client_hello_frag() -> Vec<u8> {
        // handshake type 1 (ClientHello), body of 5 bytes
        vec![1, 0, 0, 5, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE]
    }

    #[test]
    fn record_walk() {
        let mut f = emit(22, 0x0303, &client_hello_frag());
        f.extend_from_slice(&emit(23, 0x0303, &[1, 2, 3]));
        let r = records(&f).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].ty, 22);
        assert_eq!(r[0].version, 0x0303);
        assert_eq!(r[0].name(), "handshake");
        assert_eq!(r[1].name(), "application-data");
        let hs = handshake(&r[0].fragment).unwrap();
        assert_eq!(hs.len(), 1);
        assert_eq!(hs[0].ty, 1);
        assert_eq!(hs[0].name(), "client-hello");
        assert_eq!(hs[0].body, vec![0xAA, 0xBB, 0xCC, 0xDD, 0xEE]);
    }

    #[test]
    fn emit_roundtrip() {
        let frag = b"hello, world";
        let wire = emit(23, 0x0301, frag);
        let r = records(&wire).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].fragment, frag);
    }

    #[test]
    fn malformed_degrades() {
        assert!(records(&[]).unwrap().is_empty()); // empty stream = no records
        assert!(records(&[22]).is_none()); // truncated header
        assert!(records(&[99, 3, 3, 0, 0]).is_none()); // bad type
        assert!(records(&[22, 3, 3, 0x40, 0]).is_none()); // len > 16384
        assert!(records(&[22, 3, 3, 0, 5, 1]).is_none()); // truncated frag
        assert!(handshake(&[1, 0, 0]).is_none());
    }
}
