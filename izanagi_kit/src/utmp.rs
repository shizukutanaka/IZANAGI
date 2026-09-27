//! Linux `utmp`/`wtmp` binary login records (utmp(5), glibc `struct utmp`,
//! 384 bytes each, little-endian assumed).
//!
//! ```
//! use izanagi_kit::utmp::{parse, RECORD};
//!
//! let mut d = vec![0u8; RECORD];
//! d[0..4].copy_from_slice(&7i32.to_le_bytes());      // USER_PROCESS
//! d[44..47].copy_from_slice(b"ume");                 // user
//! let u = parse(&d).unwrap();
//! assert_eq!(u.entries.len(), 1);
//! assert_eq!(u.entries[0].user, "ume");
//! ```

/// Size of one `struct utmp` on glibc Linux (x86-64 / generic).
pub const RECORD: usize = 384;

/// One utmp/wtmp record (selected fields).
#[derive(Debug, Clone)]
pub struct Entry {
    /// Record type (0=EMPTY, 7=USER_PROCESS, 8=DEAD_PROCESS, ...).
    pub entry_type: i32,
    /// Process id.
    pub pid: i32,
    /// Device line (`tty1`, `pts/0`, ...), NUL-trimmed.
    pub line: String,
    /// Inittab id, NUL-trimmed.
    pub id: String,
    /// User name, NUL-trimmed.
    pub user: String,
    /// Remote host, NUL-trimmed.
    pub host: String,
    /// Session id.
    pub session: i32,
    /// `tv_sec` seconds.
    pub sec: i32,
    /// `tv_usec` microseconds.
    pub usec: i32,
    /// `ut_addr_v6` raw four words.
    pub addr_v6: [u32; 4],
}

/// A whole utmp/wtmp file.
#[derive(Debug, Clone)]
pub struct Utmp {
    /// Records in file order.
    pub entries: Vec<Entry>,
}

fn le32(d: &[u8], at: usize) -> Option<i32> {
    let s = d.get(at..at + 4)?;
    Some(i32::from(s[0]) | i32::from(s[1]) << 8 | i32::from(s[2]) << 16 | i32::from(s[3]) << 24)
}

fn le32u(d: &[u8], at: usize) -> Option<u32> {
    le32(d, at).map(|v| v as u32)
}

fn text(d: &[u8]) -> String {
    let end = d.iter().position(|&b| b == 0).unwrap_or(d.len());
    String::from_utf8_lossy(&d[..end]).into_owned()
}

/// Parse `len % 384 == 0` records.
pub fn parse(data: &[u8]) -> Option<Utmp> {
    if data.len() % RECORD != 0 {
        return None;
    }
    let mut entries = Vec::new();
    for rec in data.chunks_exact(RECORD) {
        let mut addr_v6 = [0u32; 4];
        for (i, slot) in addr_v6.iter_mut().enumerate() {
            *slot = le32u(rec, 348 + i * 4)?;
        }
        entries.push(Entry {
            entry_type: le32(rec, 0)?,
            pid: le32(rec, 4)?,
            line: text(&rec[8..40]),
            id: text(&rec[40..44]),
            user: text(&rec[44..76]),
            host: text(&rec[76..332]),
            session: le32(rec, 336)?,
            sec: le32(rec, 340)?,
            usec: le32(rec, 344)?,
            addr_v6,
        });
    }
    Some(Utmp { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let mut d = vec![0u8; RECORD * 2];
        d[0..4].copy_from_slice(&7i32.to_le_bytes());
        d[4..8].copy_from_slice(&1234i32.to_le_bytes());
        d[8..12].copy_from_slice(b"tty1");
        d[44..48].copy_from_slice(b"root");
        d[340..344].copy_from_slice(&1700000000i32.to_le_bytes());
        let u = parse(&d).unwrap();
        assert_eq!(u.entries[0].entry_type, 7);
        assert_eq!(u.entries[0].pid, 1234);
        assert_eq!(u.entries[0].line, "tty1");
        assert_eq!(u.entries[0].sec, 1700000000);
        assert_eq!(u.entries[1].entry_type, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 100]).is_none());
    }
}
