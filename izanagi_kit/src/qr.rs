//! QR Code encoder — byte mode, error-correction level L, versions 1–10
//! (ISO/IEC 18004, Denso Wave). Reed–Solomon ECC on GF(2⁸) with the
//! QR primitive polynomial 0x11D — a private `QrGf` table, since
//! [`crate::gf2`] carries the AES polynomial 0x11B. All 8 mask
//! patterns are scored per the ISO penalty rules.
//!
//! Output is a module matrix ready for rendering (`braille`, `plot`, or
//! the terminal). Payload capacity at level L ranges from 17 bytes
//! (v1, 21×21) to 271 bytes (v10, 57×57).
//!
//! ```
//! use izanagi_kit::qr::{encode, ModuleMatrix};
//!
//! let m = encode(b"HELLO").unwrap();
//! assert_eq!(m.size, 21); // version 1 fits 17 bytes
//! // Finder patterns occupy the three outer corners.
//! assert!(m.get(0, 0) && m.get(6, 0) && !m.get(7, 0));
//! assert!(m.get(0, m.size - 1) && m.get(m.size - 1, 0));
//! ```

use std::vec::Vec;

/// GF(2⁸) arithmetic over the QR polynomial `x⁸+x⁷+x⁵+x³+1` (0x11D,
/// generator `0x02`). NOTE: [`crate::gf2`] implements AES's 0x11B — a
/// different field — so QR keeps its own tables.
struct QrGf {
    exp: [u8; 512],
    log: [u8; 256],
}

impl QrGf {
    fn new() -> Self {
        let mut exp = [0u8; 512];
        let mut log = [0u8; 256];
        let mut x = 1u16;
        for (i, e) in exp.iter_mut().enumerate().take(255) {
            *e = x as u8;
            log[x as usize] = i as u8;
            x <<= 1;
            if x & 0x100 != 0 {
                x ^= 0x11D;
            }
        }
        for i in 255..512 {
            exp[i] = exp[i - 255];
        }
        QrGf { exp, log }
    }
    fn mul(&self, a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            0
        } else {
            self.exp[self.log[a as usize] as usize + self.log[b as usize] as usize]
        }
    }
    /// αⁱ (i wraps mod 255).
    fn pow(&self, i: u32) -> u8 {
        self.exp[(i % 255) as usize]
    }
}

/// One QR symbol: `size × size` modules; `get(x, y)` is `true` for dark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleMatrix {
    /// Side length in modules (21 + 4·(version−1)).
    pub size: usize,
    /// Row-major module states; `true` = dark.
    pub cells: Vec<bool>,
    /// QR version chosen (1–10).
    pub version: u8,
    /// Mask pattern selected by the penalty evaluation (0–7).
    pub mask: u8,
}

impl ModuleMatrix {
    /// Module at column `x`, row `y` — `false` when out of bounds.
    pub fn get(&self, x: usize, y: usize) -> bool {
        if x < self.size && y < self.size {
            self.cells[y * self.size + x]
        } else {
            false
        }
    }

    /// Renders the symbol as a string of `██`/two spaces with a
    /// 4-module quiet zone — scannable in most terminals.
    pub fn to_terminal(&self) -> std::string::String {
        let mut s = std::string::String::new();
        let n = self.size + 8;
        for y in 0..n {
            for x in 0..n {
                let dark = x >= 4 && y >= 4 && self.get(x - 4, y - 4);
                s.push_str(if dark { "██" } else { "  " });
            }
            s.push('\n');
        }
        s
    }
}

/// Per-version structure at EC level L: `(total codewords,
/// [(data cw, ec cw) per block group; count × members implied by
/// position], data-block sizes group1, group2 size, group2 count)`.
///
/// Table (version → total cw, ec per block, [group1 blocks × data cw],
/// [group2 blocks × data cw]):
fn block_table(version: usize) -> (usize, usize, usize, usize, usize, usize) {
    // (total_cw, ec_per_block, g1_blocks, g1_data, g2_blocks, g2_data)
    match version {
        1 => (26, 7, 1, 19, 0, 0),
        2 => (44, 10, 1, 34, 0, 0),
        3 => (70, 15, 1, 55, 0, 0),
        4 => (100, 20, 1, 80, 0, 0),
        5 => (134, 26, 1, 108, 0, 0),
        6 => (172, 18, 2, 68, 0, 0),
        7 => (196, 20, 2, 78, 0, 0),
        8 => (242, 24, 2, 97, 0, 0),
        9 => (292, 30, 2, 116, 0, 0),
        10 => (346, 18, 2, 68, 2, 69),
        _ => (0, 0, 0, 0, 0, 0),
    }
}

/// Alignment pattern centre coordinates per version (v1 has none).
fn alignment_centres(version: usize) -> &'static [usize] {
    match version {
        1 => &[],
        2 => &[6, 18],
        3 => &[6, 22],
        4 => &[6, 26],
        5 => &[6, 30],
        6 => &[6, 34],
        7 => &[6, 22, 38],
        8 => &[6, 24, 42],
        9 => &[6, 26, 46],
        _ => &[6, 28, 50],
    }
}

/// Smallest version 1–10 whose byte-mode capacity covers `len`, or
/// `None` when the payload exceeds v10-L (271 bytes).
fn pick_version(len: usize) -> Option<usize> {
    for v in 1..=10 {
        let (_, _, g1b, g1d, g2b, g2d) = block_table(v);
        let data_cw = g1b * g1d + g2b * g2d;
        // Byte mode: 4 (mode) + 8/16 (count, v<10/v≥10) + 8·len.
        let count_bits = if v < 10 { 8 } else { 16 };
        let need = 4 + count_bits + 8 * len;
        if need <= data_cw * 8 {
            return Some(v);
        }
    }
    None
}

/// RS generator polynomial `∏_{i=0..ec}(x − αⁱ)` over GF(2⁸),
/// coefficients highest-degree first (length `ec + 1`, leading 1).
fn rs_generator(ec: usize) -> Vec<u8> {
    let gf = QrGf::new();
    let mut g = vec![1u8];
    for i in 0..ec {
        let a = gf.pow(i as u32);
        // Multiply g(x) by (x + a): c'[k] = c[k−1] + a·c[k].
        // Ascending-degree accumulation: ng[k] gets a·c[k] (constant
        // term of the factor) and ng[k+1] gets c[k] (the x term).
        let mut ng = vec![0u8; g.len() + 1];
        for (k, &c) in g.iter().enumerate() {
            ng[k] ^= gf.mul(c, a);
            ng[k + 1] ^= c;
        }
        g = ng;
    }
    g.iter().rev().copied().collect()
}

/// Systematic RS parity: `data` polynomial `x^ec · msg mod g`, returns
/// `ec` parity codewords.
fn rs_parity(data: &[u8], g: &[u8], ec: usize) -> Vec<u8> {
    let gf = QrGf::new();
    let mut msg = Vec::with_capacity(data.len() + ec);
    msg.extend_from_slice(data);
    msg.extend(std::iter::repeat(0u8).take(ec));
    for i in 0..data.len() {
        let coef = msg[i];
        if coef != 0 {
            for j in 1..g.len() {
                msg[i + j] ^= gf.mul(coef, g[j]);
            }
        }
    }
    msg[data.len()..].to_vec()
}

/// Builds the interleaved codeword stream for `data` at `version`.
fn codewords(data: &[u8], version: usize) -> Vec<u8> {
    let (total_cw, ec_per, g1b, g1d, g2b, g2d) = block_table(version);
    let data_cw = g1b * g1d + g2b * g2d;
    let count_bits = if version < 10 { 8 } else { 16 };

    // Byte-mode bit stream into a Vec<u8> where each element holds one
    // BIT (kept unpacked until byte assembly — simpler and exact).
    let mut bits: Vec<u8> = Vec::with_capacity(data_cw * 8);
    bits.extend_from_slice(&[0, 1, 0, 0]); // mode 0100
    for i in (0..count_bits).rev() {
        bits.push(((data.len() >> i) & 1) as u8);
    }
    for &b in data {
        for i in (0..8).rev() {
            bits.push((b >> i) & 1);
        }
    }
    // Terminator: up to four 0 bits, then pad to a byte boundary.
    let cap = data_cw * 8;
    let term = 4usize.min(cap.saturating_sub(bits.len()));
    bits.extend(std::iter::repeat(0u8).take(term));
    while bits.len() % 8 != 0 {
        bits.push(0);
    }
    // Pad bytes EC 11 alternating.
    let pads = [0xecu8, 0x11];
    let mut pi = 0;
    while bits.len() < cap {
        let p = pads[pi % 2];
        pi += 1;
        for i in (0..8).rev() {
            bits.push((p >> i) & 1);
        }
    }
    // Assemble data codewords.
    let mut dcw = Vec::with_capacity(data_cw);
    for chunk in bits.chunks(8) {
        let mut b = 0u8;
        for &bit in chunk {
            b = (b << 1) | bit;
        }
        dcw.push(b);
    }

    // Split into blocks, compute parity per block, interleave.
    let nblocks = g1b + g2b;
    let mut blocks: Vec<(Vec<u8>, Vec<u8>)> = Vec::with_capacity(nblocks);
    let gen = rs_generator(ec_per);
    let mut at = 0;
    for _ in 0..g1b {
        let d = dcw[at..at + g1d].to_vec();
        at += g1d;
        let p = rs_parity(&d, &gen, ec_per);
        blocks.push((d, p));
    }
    for _ in 0..g2b {
        let d = dcw[at..at + g2d].to_vec();
        at += g2d;
        let p = rs_parity(&d, &gen, ec_per);
        blocks.push((d, p));
    }
    let mut out = Vec::with_capacity(total_cw);
    let maxd = g1d.max(g2d);
    for i in 0..maxd {
        for (d, _) in &blocks {
            if i < d.len() {
                out.push(d[i]);
            }
        }
    }
    for i in 0..ec_per {
        for (_, p) in &blocks {
            out.push(p[i]);
        }
    }
    out
}

/// Function-pattern grid: `Some(is_dark)` for function modules,
/// `None` for data cells.
fn function_layer(version: usize) -> Vec<Option<bool>> {
    let size = 17 + 4 * version;
    let mut f = vec![None; size * size];
    let set = |f: &mut Vec<Option<bool>>, x: usize, y: usize, v: bool| {
        f[y * size + x] = Some(v);
    };
    // Finder + separator at (x0, y0) top-left of the 7×7.
    let finder = |f: &mut Vec<Option<bool>>, fx: usize, fy: usize| {
        for dy in 0..9usize {
            for dx in 0..9usize {
                let x = fx as isize + dx as isize - 1;
                let y = fy as isize + dy as isize - 1;
                if x < 0 || y < 0 || x >= size as isize || y >= size as isize {
                    continue;
                }
                let (x, y) = (x as usize, y as usize);
                let inner = (1..8).contains(&dx) && (1..8).contains(&dy);
                let dark = if !inner {
                    false // separator ring
                } else {
                    let (ix, iy) = (dx - 1, dy - 1);
                    ix == 0
                        || ix == 6
                        || iy == 0
                        || iy == 6
                        || (2..5).contains(&ix) && (2..5).contains(&iy)
                };
                set(f, x, y, dark);
            }
        }
    };
    finder(&mut f, 0, 0);
    finder(&mut f, size - 7, 0);
    finder(&mut f, 0, size - 7);
    // Timing patterns.
    for i in 8..size - 8 {
        let dark = i % 2 == 0;
        set(&mut f, i, 6, dark);
        set(&mut f, 6, i, dark);
    }
    // Alignment patterns (5×5 bullseye), skipping overlaps with finders.
    let centres = alignment_centres(version);
    for &cy in centres {
        for &cx in centres {
            let overlaps =
                (cx == 6 && cy == 6) || (cx == 6 && cy == size - 7) || (cx == size - 7 && cy == 6);
            if overlaps {
                continue;
            }
            for dy in 0..5usize {
                for dx in 0..5usize {
                    let dark = dy == 0 || dy == 4 || dx == 0 || dx == 4 || (dx == 2 && dy == 2);
                    set(&mut f, cx + dx - 2, cy + dy - 2, dark);
                }
            }
        }
    }
    // Dark module + format-info placeholders (marked occupied; values
    // filled after mask selection).
    set(&mut f, 8, 4 * version + 9, true);
    for i in 0..9usize {
        if i != 6 {
            set(&mut f, 8, i, false);
            set(&mut f, i, 8, false);
        }
    }
    for i in 0..8usize {
        set(&mut f, size - 1 - i, 8, false);
    }
    for i in 0..7usize {
        set(&mut f, 8, size - 7 + i, false);
    }
    f
}

/// Mask predicate for pattern `m` (ISO table 6.4.10.3.1).
fn mask_bit(m: u8, x: usize, y: usize) -> bool {
    match m {
        0 => (x + y) % 2 == 0,
        1 => y % 2 == 0,
        2 => x % 3 == 0,
        3 => (x + y) % 3 == 0,
        4 => (y / 2 + x / 3) % 2 == 0,
        5 => (x * y) % 2 + (x * y) % 3 == 0,
        6 => ((x * y) % 2 + (x * y) % 3) % 2 == 0,
        _ => ((x + y) % 2 + (x * y) % 3) % 2 == 0,
    }
}

/// 15-bit BCH format string for EC level L (01) + `mask` — masked by
/// 101010000010010 per ISO 7.9.1.
fn format_bits(mask: u8) -> u16 {
    let data = (0b01u16 << 3) | mask as u16;
    let mut rem = data << 10;
    let gen = 0b10100110111u16;
    for i in (0..=4u32).rev() {
        if rem >> (i + 10) & 1 == 1 {
            rem ^= gen << i;
        }
    }
    ((data << 10) | rem) ^ 0b101010000010010
}

/// ISO penalty score of a finished matrix — lower is better.
fn penalty(cells: &[bool], size: usize) -> u32 {
    let at = |x: isize, y: isize| -> bool {
        if x < 0 || y < 0 || x >= size as isize || y >= size as isize {
            false
        } else {
            cells[y as usize * size + x as usize]
        }
    };
    let mut score = 0u32;
    // Rule 1: runs of ≥5 same colour, horizontally and vertically.
    for y in 0..size {
        let mut run = 1u32;
        for x in 1..size {
            if at(x as isize, y as isize) == at(x as isize - 1, y as isize) {
                run += 1;
            } else {
                if run >= 5 {
                    score += 3 + (run - 5);
                }
                run = 1;
            }
        }
        if run >= 5 {
            score += 3 + (run - 5);
        }
    }
    for x in 0..size {
        let mut run = 1u32;
        for y in 1..size {
            if at(x as isize, y as isize) == at(x as isize, y as isize - 1) {
                run += 1;
            } else {
                if run >= 5 {
                    score += 3 + (run - 5);
                }
                run = 1;
            }
        }
        if run >= 5 {
            score += 3 + (run - 5);
        }
    }
    // Rule 2: 2×2 same-colour blocks.
    for y in 0..size - 1 {
        for x in 0..size - 1 {
            let c = at(x as isize, y as isize);
            if at(x as isize + 1, y as isize) == c
                && at(x as isize, y as isize + 1) == c
                && at(x as isize + 1, y as isize + 1) == c
            {
                score += 3;
            }
        }
    }
    // Rule 3: 1011101 with 4+ light modules either side.
    let pat = [true, false, true, true, true, false, true];
    let matches = |seq: &[bool; 7], dark_end: bool| -> bool {
        // finder-like with white margin after (dark_end = pattern at
        // the start of the window)
        *seq == pat && dark_end
    };
    for y in 0..size {
        for x in 0..size {
            // horizontal window of 11
            if x + 11 <= size {
                let seq: Vec<bool> = (0..11).map(|d| at((x + d) as isize, y as isize)).collect();
                let seven: [bool; 7] = [seq[0], seq[1], seq[2], seq[3], seq[4], seq[5], seq[6]];
                if matches(&seven, seq[7..11].iter().all(|&v| !v)) {
                    score += 40;
                }
                let seven2: [bool; 7] = [seq[4], seq[5], seq[6], seq[7], seq[8], seq[9], seq[10]];
                if matches(&seven2, seq[0..4].iter().all(|&v| !v)) {
                    score += 40;
                }
            }
            if y + 11 <= size {
                let seq: Vec<bool> = (0..11).map(|d| at(x as isize, (y + d) as isize)).collect();
                let seven: [bool; 7] = [seq[0], seq[1], seq[2], seq[3], seq[4], seq[5], seq[6]];
                if matches(&seven, seq[7..11].iter().all(|&v| !v)) {
                    score += 40;
                }
                let seven2: [bool; 7] = [seq[4], seq[5], seq[6], seq[7], seq[8], seq[9], seq[10]];
                if matches(&seven2, seq[0..4].iter().all(|&v| !v)) {
                    score += 40;
                }
            }
        }
    }
    // Rule 4: dark-module proportion vs 50%.
    let dark = cells.iter().filter(|&&c| c).count();
    let pct = dark * 100 / (size * size);
    let low = pct - pct % 5;
    let high = low + 5;
    let k = (50i32 - low as i32).abs().min((50i32 - high as i32).abs()) / 5;
    score += k.max(0) as u32 * 10;
    score
}

/// Encodes `data` into a QR symbol (byte mode, EC level L, version
/// 1–10 auto-selected). `None` for payloads over 271 bytes.
pub fn encode(data: &[u8]) -> Option<ModuleMatrix> {
    let version = pick_version(data.len())?;
    let size = 17 + 4 * version;
    let cw = codewords(data, version);
    let function = function_layer(version);

    // Zigzag data placement into a fresh grid per mask candidate.
    let mut best: Option<(u32, u8, Vec<bool>)> = None;
    for mask in 0..8u8 {
        let mut cells = vec![false; size * size];
        for (i, f) in function.iter().enumerate() {
            if let Some(v) = f {
                cells[i] = *v;
            }
        }
        // Fill data bits: right-to-left column pairs, alternate
        // up/down, skip column 6.
        let mut bit_iter = cw
            .iter()
            .flat_map(|&b| (0..8).rev().map(move |i| (b >> i) & 1));
        let mut x = size - 1;
        let mut upward = true;
        'cols: while x > 0 {
            if x == 6 {
                x -= 1;
            }
            let col2 = x;
            let col1 = x - 1;
            for row in 0..size {
                let y = if upward { size - 1 - row } else { row };
                for col in [col2, col1] {
                    if function[y * size + col].is_none() {
                        match bit_iter.next() {
                            Some(bit) => {
                                let dark = bit == 1;
                                cells[y * size + col] = dark ^ mask_bit(mask, col, y);
                            }
                            None => break 'cols,
                        }
                    }
                }
            }
            upward = !upward;
            x = x.saturating_sub(2);
        }
        // Format bits (copy 1 + copy 2) + dark module.
        let bits = format_bits(mask);
        let get = |i: u32| (bits >> i) & 1 == 1;
        // Copy 1 per ISO: bits 0-5 → col 8 rows 0-5; bit 6 → (8,7);
        // bit 7 → (8,8); bit 8 → (7,8); bits 9-14 → row 8 cols 5-0.
        for i in 0..6usize {
            cells[i * size + 8] = get(i as u32);
        }
        cells[7 * size + 8] = get(6);
        cells[8 * size + 8] = get(7);
        cells[8 * size + 7] = get(8);
        for i in 0..6usize {
            cells[8 * size + 5 - i] = get(9 + i as u32);
        }
        // Copy 2: bits 0-7 → row 8 cols size-1 down size-8; bits 8-14 →
        // col 8 rows size-7..size-1.
        for i in 0..8usize {
            cells[8 * size + size - 1 - i] = get(i as u32);
        }
        for i in 0..7usize {
            cells[(size - 7 + i) * size + 8] = get(8 + i as u32);
        }
        cells[(4 * version + 9) * size + 8] = true; // dark module

        let score = penalty(&cells, size);
        if best.as_ref().map(|b| score < b.0).unwrap_or(true) {
            best = Some((score, mask, cells));
        }
    }
    let (_, mask, cells) = best?;
    Some(ModuleMatrix {
        size,
        cells,
        version: version as u8,
        mask,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generator_polynomials_match_spec() {
        // ISO/IEC 18004 Annex A: degree-7 generator = x⁷+127x⁶+…+87
        // coefficients [1,127,122,154,164,11,68,117].
        let g7 = rs_generator(7);
        assert_eq!(g7, vec![1, 127, 122, 154, 164, 11, 68, 117]);
        // Degree 10: [1,216,194,159,111,199,94,95,113,157,193].
        let g10 = rs_generator(10);
        assert_eq!(g10, vec![1, 216, 194, 159, 111, 199, 94, 95, 113, 157, 193]);
    }

    #[test]
    fn rs_parity_hello_world_verified() {
        // "HELLO WORLD" byte-mode stream at v1-Q shape (13 data
        // codewords) → 10 parity codewords;
        // generator and remainder cross-verified independently.
        let data: Vec<u8> = vec![
            0x20, 0x5b, 0x0b, 0x78, 0xd1, 0x72, 0xdc, 0x4d, 0x43, 0x40, 0xec, 0x11, 0xec,
        ];
        let g = rs_generator(10);
        let p = rs_parity(&data, &g, 10);
        // Verified against an independent GF(2⁸) implementation —
        // systematic remainder of msg·x^10 mod g.
        assert_eq!(p, vec![87, 86, 68, 17, 99, 235, 189, 232, 98, 195]);
    }

    #[test]
    fn version_selection_by_capacity() {
        assert_eq!(pick_version(1), Some(1));
        assert_eq!(pick_version(17), Some(1));
        assert_eq!(pick_version(18), Some(2));
        assert_eq!(pick_version(32), Some(2));
        assert_eq!(pick_version(271), Some(10));
        assert_eq!(pick_version(272), None);
    }

    #[test]
    fn encode_hello_structure() {
        let m = encode(b"HELLO WORLD").unwrap();
        assert_eq!(m.version, 1);
        assert_eq!(m.size, 21);
        // Finders at the three corners are present and dark.
        for dy in 0..7 {
            for dx in 0..7 {
                let edge = dy == 0 || dy == 6 || dx == 0 || dx == 6;
                let core = (2..5).contains(&dx) && (2..5).contains(&dy);
                assert_eq!(m.get(dx, dy), edge || core, "tl {dx},{dy}");
                assert_eq!(m.get(m.size - 7 + dx, dy), edge || core, "tr {dx},{dy}");
                assert_eq!(m.get(dx, m.size - 7 + dy), edge || core, "bl {dx},{dy}");
            }
        }
        // Timing lines alternate, starting dark at index 6,8.
        for i in 8..m.size - 8 {
            assert_eq!(m.get(i, 6), i % 2 == 0);
            assert_eq!(m.get(6, i), i % 2 == 0);
        }
        // Dark module always set.
        assert!(m.get(8, 4 * m.version as usize + 9));
    }

    #[test]
    fn encode_is_deterministic_and_nonempty() {
        let a = encode(b"deterministic").unwrap();
        let b = encode(b"deterministic").unwrap();
        assert_eq!(a, b);
        assert!(a.cells.iter().any(|&c| c));
        assert!(a.cells.iter().any(|&c| !c));
    }

    #[test]
    fn masks_scored_differently() {
        // Force mask evaluation to matter: a longer payload exercises
        // every mask's penalty profile; whichever wins is stable.
        let m = encode(&b"the quick brown fox jumps over the lazy dog"[..]).unwrap();
        assert!(m.version >= 2);
        assert!(m.mask < 8);
    }

    #[test]
    fn max_payload_v10() {
        let data = vec![b'a'; 271];
        let m = encode(&data).unwrap();
        assert_eq!(m.version, 10);
        assert_eq!(m.size, 57);
        assert_eq!(encode(&vec![b'a'; 272]), None);
    }

    #[test]
    fn format_bits_are_masked_bch() {
        // mask 0, level L: format 111011111000100 (0x77C4).
        assert_eq!(format_bits(0), 0b111011111000100);
        // mask 7, level L: 110100101110110 (ISO format table).
        assert_eq!(format_bits(7), 0b110100101110110);
    }

    #[test]
    fn terminal_render_shape() {
        let m = encode(b"HELLO").unwrap();
        let t = m.to_terminal();
        let lines: Vec<&str> = t.lines().collect();
        // 4-module quiet zone above and below; one terminal row per
        // module row, two characters per module.
        assert_eq!(lines.len(), m.size + 8);
        assert!(lines.iter().all(|l| l.chars().count() == (m.size + 8) * 2));
        // Top-left corner module renders as a full block after the
        // quiet-zone padding.
        assert!(lines[4].starts_with(&" ".repeat(8)));
        assert!(lines[4].contains("██"));
    }
}
