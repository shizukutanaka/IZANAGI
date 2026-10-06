//! `axports` (AX.25 port table) 検出モジュール。
//!
//! `/etc/ax25/axports` は空白区切りの5カラム形式:
//! `portname  callsign  speed  paclen  window  description`。
//! コールサインは `N0CALL-9` 型(3–9文字、英数字とハイフン)。
//!
//! ```
//! let b = br#"# name callsign speed paclen window description
//! radio   N0CALL-1    9600    255     2       144.800 MHz (1200 bps)
//! uhf     N0CALL-3    19200   255     7       430.675 MHz (9600 bps)
//! "#;
//! let c = izanagi_kit::axports::parse(b);
//! assert!(izanagi_kit::axports::detect(b));
//! assert_eq!(c.ports, 2);
//! ```

fn is_callsign(s: &str) -> bool {
    let n = s.len();
    (3..=9).contains(&n)
        && s.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
        && s.chars().any(|c| c.is_ascii_uppercase())
        && s.chars().any(|c| c.is_ascii_digit())
}

fn is_port_line(t: &str) -> bool {
    let f: Vec<&str> = t.split_whitespace().collect();
    if f.len() < 5 {
        return false;
    }
    // port callsign speed paclen window [descr...]
    is_callsign(f[1])
        && f[2].chars().all(|c| c.is_ascii_digit())
        && f[3].chars().all(|c| c.is_ascii_digit())
        && f[4].chars().all(|c| c.is_ascii_digit())
        && !f[0].chars().next().is_some_and(|c| c.is_ascii_uppercase())
}

/// `b` が axports に見えるかを返す。
pub fn detect(b: &[u8]) -> bool {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut ports = 0usize;
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() || tr.starts_with('#') {
            continue;
        }
        if is_port_line(tr) {
            ports += 1;
        }
    }
    ports >= 2
}

/// axports の統計。
#[derive(Debug, Default, Clone)]
pub struct Axports {
    /// ポート定義行数。
    pub ports: usize,
    /// コメント行数。
    pub comments: usize,
}

/// `b` を axports として統計する。
pub fn parse(b: &[u8]) -> Axports {
    let t = std::str::from_utf8(b).unwrap_or("");
    let mut c = Axports::default();
    for l in t.lines() {
        let tr = l.trim();
        if tr.is_empty() {
            continue;
        }
        if tr.starts_with('#') {
            c.comments += 1;
            continue;
        }
        if is_port_line(tr) {
            c.ports += 1;
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = br#"radio   N0CALL-1    9600    255     2       144.800 MHz
uhf     N0CALL-3    19200   255     7       430.675 MHz
"#;
        assert!(detect(b));
        let c = parse(b);
        assert_eq!(c.ports, 2);
    }

    #[test]
    fn rejects_single_port() {
        assert!(!detect(
            b"radio   N0CALL-1    9600    255     2       desc\n"
        ));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!detect(b"foo N0CALL 9600 255 2\n"));
        assert!(!detect(b"a b c d e\nf g h i j\n"));
        assert!(!detect(b"host 192.168.0.1\nhost2 192.168.0.2\n"));
    }

    #[test]
    fn empty_and_binary() {
        assert!(!detect(b""));
        assert!(!detect(&[0xff, 0xfe, 0x00, 0x01, 0x90]));
        let c = parse(b"");
        assert_eq!(c.ports, 0);
    }
}
