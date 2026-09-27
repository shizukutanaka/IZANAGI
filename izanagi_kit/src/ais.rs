//! AIS — NMEA-0183 `!AIVDM`/`!AIVDO` sentences with 6-bit payload.
//!
//! `!AIVDM,total,frag,seq,chan,payload,pad*CS` — checksum is XOR over
//! characters between `!`/`$` and `*`; the payload is ASCII-armored
//! 6-bit nibbles (`0-9`, `:;<=>?@`, `A-W`, `` ` ``, `a-w`).
//!
//! ```
//! use izanagi_kit::ais::parse;
//!
//! // "1:" payload → message type 1 (position report)
//! let a = parse(b"!AIVDM,1,1,,B,15:,0*1B").unwrap();
//! assert_eq!(a.msg_type(), Some(1));
//! ```

/// A parsed AIS sentence.
#[derive(Clone, Debug)]
pub struct Ais {
    /// Talker + formatter (`AIVDM`/`AIVDO`).
    pub kind: String,
    /// Total fragment count.
    pub total: u8,
    /// Fragment index.
    pub fragment: u8,
    /// Channel letter.
    pub channel: Option<String>,
    /// ASCII-armored payload.
    pub payload: String,
    /// Pad bits (0-5) at the payload tail.
    pub pad: u8,
}

impl Ais {
    /// First 6 bits of the payload = AIS message type.
    pub fn msg_type(&self) -> Option<u8> {
        let b = self.payload.as_bytes().first()?;
        sixbit(*b)
    }

    /// `bits` bits starting at bit `at` (MSB-first packing).
    pub fn field(&self, at: usize, bits: usize) -> Option<u64> {
        if bits > 64 {
            return None;
        }
        let mut v: u64 = 0;
        for i in 0..bits {
            let bitpos = at + i;
            let byte = *self.payload.as_bytes().get(bitpos / 6)?;
            let six = sixbit(byte)?;
            let bit = (six >> (5 - (bitpos % 6))) & 1;
            v = (v << 1) | bit as u64;
        }
        Some(v)
    }
}

/// 6-bit armoring value of one ASCII byte.
pub fn sixbit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'W' => Some(b - 48),
        b'`'..=b'w' => Some(b - 56),
        _ => None,
    }
}

fn xorsum(d: &str) -> Option<bool> {
    // verify `*hh` XOR over bytes between `!`/`$` and `*`
    let star = d.rfind('*')?;
    let body = d.get(1..star)?;
    let mut x: u8 = 0;
    for &b in body.as_bytes() {
        x ^= b;
    }
    let hex = d.get(star + 1..star + 3)?;
    Some(u8::from_str_radix(hex, 16).ok()? == x)
}

/// Parse one AIS sentence. `None` on wrong marker, bad checksum, wrong
/// field count, or a payload byte outside the 6-bit alphabet.
pub fn parse(line: &[u8]) -> Option<Ais> {
    let text = std::str::from_utf8(line).ok()?.trim();
    if !(text.starts_with("!AI") || text.starts_with("$AI")) {
        return None;
    }
    if xorsum(text) != Some(true) {
        return None;
    }
    let star = text.find('*')?;
    let body = &text[1..star];
    let mut f = body.split(',');
    let kind = f.next()?.to_string();
    if kind != "AIVDM" && kind != "AIVDO" {
        return None;
    }
    let total: u8 = f.next()?.parse().ok()?;
    let fragment: u8 = f.next()?.parse().ok()?;
    let _seq = f.next()?; // sequential message id, usually empty
    let channel = match f.next()? {
        "" => None,
        c => Some(c.to_string()),
    };
    let payload = f.next()?.to_string();
    for &b in payload.as_bytes() {
        sixbit(b)?;
    }
    let pad: u8 = f.next()?.parse().ok()?;
    if pad > 5 {
        return None;
    }
    Some(Ais {
        kind,
        total,
        fragment,
        channel,
        payload,
        pad,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // build a sentence with a real checksum
    fn sentence(payload: &str, pad: u8) -> String {
        let body = format!("AIVDM,1,1,,B,{payload},{pad}");
        let mut x: u8 = 0;
        for &b in body.as_bytes() {
            x ^= b;
        }
        format!("!{body}*{x:02X}")
    }

    #[test]
    fn parses() {
        let s = sentence("15MvqT=", 0);
        let a = parse(s.as_bytes()).unwrap();
        assert_eq!(a.kind, "AIVDM");
        assert_eq!(a.channel.as_deref(), Some("B"));
        assert_eq!(a.msg_type(), Some(1));
        assert_eq!(a.field(0, 6), Some(1)); // message type field
        assert_eq!(a.field(6, 2), Some(0)); // repeat indicator
        assert_eq!(a.field(64, 65), None); // >64 bits rejected
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"!AIVDM,1,1,,B,15:,0*00").is_none()); // bad checksum
        let s = sentence("15~", 0);
        assert!(parse(s.as_bytes()).is_none()); // `~` outside alphabet
        let s2 = sentence("15", 6);
        assert!(parse(s2.as_bytes()).is_none()); // pad > 5
    }

    #[test]
    fn armor_alphabet() {
        assert_eq!(sixbit(b'0'), Some(0));
        assert_eq!(sixbit(b'W'), Some(39));
        assert_eq!(sixbit(b'`'), Some(40));
        assert_eq!(sixbit(b'w'), Some(63));
        assert_eq!(sixbit(b'x'), None);
        assert_eq!(sixbit(b'~'), None);
    }
}
