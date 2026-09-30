//! AcrossLite `.puz` crossword — `"ACROSS&DOWN"` マジック、
//! チェックサム領域、幅/高さ/手がかり数、盤面グリッド(解答+状態)と文字列テーブル。
//!
//! ```
//! let mut d = vec![0u8; 2];
//! d.extend_from_slice(b"ACROSS&DOWN\x00");
//! d.resize(0x2c, 0);
//! d.extend_from_slice(&[3, 3]);           // width, height
//! d.extend_from_slice(&[4, 0]);           // nclues u16le
//! d.extend_from_slice(&[1, 0, 0, 0]);     // type, solution_state
//! d.extend_from_slice(b"CAT\x2eOG\x2eBI");  // 9 solution cells (mixed)
//! d.extend_from_slice(b"---------");      // state cells
//! d.extend_from_slice(b"Title\x00Author\x00(c)\x00clue1\x00clue2\x00clue3\x00clue4\x00");
//! let p = izanagi_kit::puz::parse(&d).unwrap();
//! assert_eq!(p.width, 3);
//! assert_eq!(p.clues, 4);
//! assert_eq!(p.title, "Title");
//! assert!(izanagi_kit::puz::detect(&d));
//! ```

/// A parsed `.puz` file census.
#[derive(Debug, Clone)]
pub struct Puz {
    /// Grid width (offset 0x2C).
    pub width: u8,
    /// Grid height (offset 0x2D).
    pub height: u8,
    /// Clue count (u16le at 0x2E).
    pub clues: u16,
    /// Puzzle type field (1 = normal).
    pub puzzle_type: u16,
    /// Solution letters in the grid.
    pub letters: usize,
    /// Black squares (`.` in the solution grid).
    pub blacks: usize,
    /// Title string (first NUL-terminated string after the grids).
    pub title: String,
    /// Author string.
    pub author: String,
    /// Extra region byte total (scrambled/locked areas ignored).
    pub extra_bytes: usize,
    /// Whether the file carries the optional extension sections.
    pub has_sections: bool,
}

fn zstr(b: &[u8], off: usize) -> (String, usize) {
    if off >= b.len() {
        return (String::new(), off);
    }
    let end = b[off..]
        .iter()
        .position(|&c| c == 0)
        .map(|i| off + i)
        .unwrap_or(b.len());
    (String::from_utf8_lossy(&b[off..end]).to_string(), end + 1)
}

/// Detects a `.puz` file: `"ACROSS&DOWN"` signature at offset 2.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.len() > 0x10 && b[2..13] == *b"ACROSS&DOWN"
}

/// Parses a `.puz` file; `None` without the signature or grids.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Puz> {
    if !detect(b) {
        return None;
    }
    if b.len() < 0x34 {
        return None;
    }
    let width = b[0x2c];
    let height = b[0x2d];
    let clues = u16::from_le_bytes([b[0x2e], b[0x2f]]);
    let ptype = u16::from_le_bytes([b[0x30], b[0x31]]);
    let cells = width as usize * height as usize;
    if cells == 0 || b.len() < 0x34 + 2 * cells {
        return None;
    }
    let sol = &b[0x34..0x34 + cells];
    let mut letters = 0usize;
    let mut blacks = 0usize;
    for &c in sol {
        if c == b'.' {
            blacks += 1;
        } else if c != 0 {
            letters += 1;
        }
    }
    let mut off = 0x34 + 2 * cells;
    let (title, o2) = zstr(b, off);
    off = o2;
    let (author, o3) = zstr(b, off);
    off = o3;
    let (_copyright, o4) = zstr(b, off);
    off = o4;
    let mut extra = 0usize;
    for _ in 0..clues {
        let (_, o) = zstr(b, off);
        if o <= off {
            break;
        }
        extra += o - off;
        off = o;
    }
    Some(Puz {
        width,
        height,
        clues,
        puzzle_type: ptype,
        letters,
        blacks,
        title,
        author,
        extra_bytes: extra + b.len().saturating_sub(off),
        has_sections: b.len() > off,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 2];
        d.extend_from_slice(b"ACROSS&DOWN\x00");
        d.resize(0x2c, 0);
        d.extend_from_slice(&[3, 3]);
        d.extend_from_slice(&[4, 0]);
        d.extend_from_slice(&[1, 0, 0, 0]);
        d.extend_from_slice(b"CAT.OG.BI");
        d.extend_from_slice(b"---------");
        d.extend_from_slice(
            b"MyPuzzle\x00JaneDoe\x00(c) 2024\x00clue-a\x00clue-b\x00clue-c\x00clue-d\x00",
        );
        d
    }

    #[test]
    fn parses() {
        let d = fixture();
        let p = parse(&d).unwrap();
        assert_eq!(p.width, 3);
        assert_eq!(p.height, 3);
        assert_eq!(p.clues, 4);
        assert_eq!(p.letters + p.blacks, 9);
        assert_eq!(p.title, "MyPuzzle");
        assert_eq!(p.author, "JaneDoe");
    }

    #[test]
    fn detect_works() {
        let d = fixture();
        assert!(detect(&d));
        assert!(!detect(b"ACROSS&DOWN\x00"));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"XXACROSS&DOWN\x00").is_none());
    }
}
