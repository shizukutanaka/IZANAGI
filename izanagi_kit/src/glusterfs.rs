//! GlusterFS volume file (`*.vol`) パーサ。
//!
//! `volume <name>` / `type <translator>` / `option <k> <v>` / `subvolumes` / `end-volume`
//! の5構文を計数する。
//!
//! ```
//! use izanagi_kit::glusterfs;
//! let vol = b"volume dist\n  type cluster/distribute\n  option auth.ip-allow 192.168.*\n  subvolumes brick1 brick2\nend-volume\n";
//! assert!(glusterfs::detect(vol));
//! let c = glusterfs::parse(vol).unwrap();
//! assert_eq!(c.volumes, 1);
//! assert_eq!(c.end_volumes, 1);
//! ```

/// 判定結果の計数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// `volume <name>` ブロック数。
    pub volumes: usize,
    /// `end-volume` 数。
    pub end_volumes: usize,
    /// `type <translator>` 行数。
    pub types: usize,
    /// `option <k> <v>` 行数。
    pub options: usize,
    /// `subvolumes <...>` 行数。
    pub subvolumes: usize,
}

/// GlusterFS volfile らしさを簡易判定する。
pub fn detect(b: &[u8]) -> bool {
    let c = parse(b);
    match c {
        Some(c) => c.volumes >= 1 && c.end_volumes >= 1 && (c.types + c.options) >= 2,
        None => false,
    }
}

/// 行種別を計数する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = std::str::from_utf8(b).ok()?;
    let mut c = Counts {
        volumes: 0,
        end_volumes: 0,
        types: 0,
        options: 0,
        subvolumes: 0,
    };
    for l in s.lines() {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let word = t.split_whitespace().next().unwrap_or("");
        match word {
            "volume" => c.volumes += 1,
            "end-volume" => c.end_volumes += 1,
            "type" => c.types += 1,
            "option" => c.options += 1,
            "subvolumes" => c.subvolumes += 1,
            _ => {}
        }
    }
    (c.volumes > 0 || c.end_volumes > 0).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"volume brick1\n  type storage/posix\n  option directory /data/brick1\nend-volume\nvolume dist\n  type cluster/distribute\n  subvolumes brick1\n  option auth.ip-allow 192.168.*\nend-volume\n";

    #[test]
    fn detects_volfile() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.volumes, 2);
        assert_eq!(c.end_volumes, 2);
        assert_eq!(c.types, 2);
        assert_eq!(c.options, 2);
        assert_eq!(c.subvolumes, 1);
    }

    #[test]
    fn rejects_other() {
        let t = b"volume: not this\nend-volume nope\n";
        assert!(!detect(t));
    }
}
