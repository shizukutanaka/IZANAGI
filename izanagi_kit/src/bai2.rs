//! BAI2 cash-management balance/transaction report (`01` file header, `02`
//! group, `03` account, `16` transaction detail, `88` continuation, `49`/`98`/
//! `99` trailers). Each record is a comma-separated line ending in `/`.
//!
//! ```
//! let d = b"01,SNDCORP,RCVBANK,260926,0100,001,80,1,2/\n02,GRP1,SNDCORP,260926,0100,,USD,1/\n03,12345678,USD,010,500000,,,015,120000/\n16,475,15000,CK,0000123456,,,VENDOR INV 77/\n88,SECOND LINE/\n49,500000,1/\n98,500000,1,1/\n99,500000,1,1/\n";
//! let r = izanagi_kit::bai2::parse(d).unwrap();
//! assert_eq!(r.sender, "SNDCORP");
//! assert_eq!(r.receiver, "RCVBANK");
//! assert_eq!(r.groups, 1);
//! assert_eq!(r.transactions, 1);
//! assert_eq!(r.continuations, 1);
//! assert!(r.complete);
//! assert!(izanagi_kit::bai2::detect(d));
//! ```

/// A parsed BAI2 report.
#[derive(Debug)]
pub struct Bai2 {
    /// Originator/sender id from the `01` record.
    pub sender: String,
    /// Ultimate receiver id from the `01` record.
    pub receiver: String,
    /// `02` group-header count.
    pub groups: u32,
    /// `03` account-identifier count.
    pub accounts: u32,
    /// `16` transaction-detail count.
    pub transactions: u32,
    /// `88` continuation-record count.
    pub continuations: u32,
    /// `49`/`98`/`99` trailer record count.
    pub trailers: u32,
    /// Whether a `99` file trailer was present.
    pub complete: bool,
    /// Control total from the `99` record (raw, implied-decimal units).
    pub control_total: Option<String>,
}

fn fields(line: &str) -> Vec<&str> {
    line.trim_end_matches('/').split(',').collect()
}

/// First line `01,` plus a `99,` file trailer identifies BAI2.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    // UTF-8 BOM(U+FEFF)は trim 系が空白と見なさないため先に剥がす。
    let s = s.strip_prefix('\u{feff}').unwrap_or(s);
    let first = match s.lines().find(|l| !l.trim().is_empty()) {
        Some(l) => l.trim(),
        None => return false,
    };
    first.starts_with("01,") && s.lines().any(|l| l.trim_start().starts_with("99,"))
}

/// Parses the report; `None` without an `01` file header.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Bai2> {
    let s = core::str::from_utf8(b).ok()?;
    let s = s.strip_prefix('\u{feff}').unwrap_or(s);
    let mut r = Bai2 {
        sender: String::new(),
        receiver: String::new(),
        groups: 0,
        accounts: 0,
        transactions: 0,
        continuations: 0,
        trailers: 0,
        complete: false,
        control_total: None,
    };
    let mut saw01 = false;
    for line in s.lines() {
        let line = line.trim();
        let f = fields(line);
        match f.first().copied() {
            Some("01") => {
                saw01 = true;
                r.sender = f.get(1).copied().unwrap_or("").to_string();
                r.receiver = f.get(2).copied().unwrap_or("").to_string();
            }
            Some("02") => r.groups += 1,
            Some("03") => r.accounts += 1,
            Some("16") => r.transactions += 1,
            Some("88") => r.continuations += 1,
            Some("49") | Some("98") | Some("99") => {
                r.trailers += 1;
                if f[0] == "99" {
                    r.complete = true;
                    r.control_total = f.get(1).map(|s| (*s).to_string());
                }
            }
            _ => {}
        }
    }
    saw01.then_some(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_utf8_bom() {
        let mut v = b"\xef\xbb\xbf".to_vec();
        v.extend_from_slice(FIXTURE);
        assert!(detect(&v));
        assert!(parse(&v).is_some());
    }

    const FIXTURE: &[u8] = b"01,SNDCORP,RCVBANK,260926,0100,001,80,1,2/\n02,GRP1,SNDCORP,260926,0100,,USD,1/\n03,12345678,USD,010,500000,,,015,120000/\n16,475,15000,CK,0000123456,,,VENDOR INV 77/\n88,SECOND LINE/\n49,500000,1/\n98,500000,1,1/\n99,500000,1,1/\n";

    #[test]
    fn detect_works() {
        assert!(detect(FIXTURE));
        assert!(!detect(b"01,only-header/"));
        assert!(!detect(b"account,report"));
    }

    #[test]
    fn parses() {
        let r = parse(FIXTURE).unwrap();
        assert_eq!(r.sender, "SNDCORP");
        assert_eq!(r.receiver, "RCVBANK");
        assert_eq!((r.groups, r.accounts, r.transactions), (1, 1, 1));
        assert_eq!(r.continuations, 1);
        assert_eq!(r.trailers, 3);
        assert!(r.complete);
        assert_eq!(r.control_total.as_deref(), Some("500000"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"99,0,0,0/").is_none());
    }
}
