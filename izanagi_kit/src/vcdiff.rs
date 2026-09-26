//! VCDIFF delta decoding (RFC 3284): the binary differencing format used
//! by xdelta, SVN's delta layer, and the Web's `delta` content-encoding
//! experiments. A delta file is a 4-byte magic `0xD6 0xC3 0xC4 0x00`, a
//! header indicator, then windows of `ADD`/`RUN`/`COPY` instructions
//! coded through a fixed 256-entry code table and a near/same address
//! cache. [`apply`] reconstructs the target given the source bytes and
//! the delta; [`emit_add`] produces a trivial all-`ADD` delta (the
//! encoder's degenerate but valid case). Secondary compression
//! (`VCD_DECOMPRESS`) and application-defined code tables
//! (`VCD_CODETABLE`) are rejected.
//!
//! ```
//! use izanagi_kit::vcdiff::{apply, emit_add};
//! let src = b"the quick brown fox";
//! let delta = emit_add(b"the quick red fox");
//! assert_eq!(apply(src, &delta).unwrap(), b"the quick red fox");
//! ```

use std::vec::Vec;

const S_NEAR: usize = 4;
const S_SAME: usize = 3;

// op kinds in a code-table entry
const NOOP: u8 = 0;
const ADD: u8 = 1;
const RUN: u8 = 2;
const COPY: u8 = 3;

/// RFC 3284 §5.6 default code table, generated from the compact
/// depiction: (inst1, size1, mode1, inst2, size2, mode2) where size 0
/// means the real size follows as a varint in the instruction stream.
fn code(code: u8) -> (u8, u8, u8, u8, u8, u8) {
    let c = code as usize;
    if c == 0 {
        return (RUN, 0, 0, NOOP, 0, 0);
    }
    if c <= 18 {
        // ADD size 0(explicit),1..17
        return (ADD, (c - 1) as u8, 0, NOOP, 0, 0);
    }
    if c <= 162 {
        // COPY mode m in 0..8, sizes 0(explicit),4..18, 16 per mode
        let m = (c - 19) / 16;
        let s = (c - 19) % 16;
        let size = if s == 0 { 0 } else { (s + 3) as u8 };
        return (COPY, size, m as u8, NOOP, 0, 0);
    }
    if c <= 234 {
        // ADD size 1..4 then COPY mode 0..5 size 4..6, 12 per mode
        let m = (c - 163) / 12;
        let a = (c - 163) % 12;
        return (ADD, (a / 3 + 1) as u8, 0, COPY, (a % 3 + 4) as u8, m as u8);
    }
    if c <= 246 {
        // ADD size 1..4 then COPY mode 6..8 size 4, 4 per mode
        let m = (c - 235) / 4 + 6;
        let a = (c - 235) % 4;
        return (ADD, (a + 1) as u8, 0, COPY, 4, m as u8);
    }
    // 247..255: COPY mode 0..8 size 4 then ADD size 1
    (COPY, 4, (c - 247) as u8, ADD, 1, 0)
}

/// RFC 3284 §2 big-endian base-128 varint (MSB-first continuation bits —
/// the opposite order from [`crate::varint`]'s LEB128).
fn varint(d: &[u8], i: &mut usize) -> Option<u64> {
    let mut v = 0u64;
    loop {
        let b = *d.get(*i)?;
        *i += 1;
        v = v.checked_mul(128)?.checked_add((b & 0x7F) as u64)?;
        if b & 0x80 == 0 {
            return Some(v);
        }
    }
}

fn enc_varint(mut v: u64, out: &mut Vec<u8>) {
    let mut tmp = [0u8; 10];
    let mut n = 0;
    loop {
        tmp[n] = (v % 128) as u8;
        v /= 128;
        n += 1;
        if v == 0 {
            break;
        }
    }
    while n > 0 {
        n -= 1;
        out.push(tmp[n] | if n == 0 { 0 } else { 0x80 });
    }
}

struct Cache {
    near: [u64; S_NEAR],
    next_slot: usize,
    same: [u64; S_SAME * 256],
}

impl Cache {
    fn new() -> Self {
        Cache {
            near: [0; S_NEAR],
            next_slot: 0,
            same: [0; S_SAME * 256],
        }
    }
    fn update(&mut self, addr: u64) {
        self.near[self.next_slot] = addr;
        self.next_slot = (self.next_slot + 1) % S_NEAR;
        self.same[(addr % (S_SAME * 256) as u64) as usize] = addr;
    }
    /// RFC 3284 §5.3 `addr_decode`: `here` is the current position in the
    /// merged `source ++ target-so-far` address space.
    fn addr(&mut self, d: &[u8], i: &mut usize, here: u64, mode: u8) -> Option<u64> {
        let addr = match mode {
            0 => varint(d, i)?,                    // VCD_SELF
            1 => here.checked_sub(varint(d, i)?)?, // VCD_HERE
            m if (2..2 + S_NEAR as u8).contains(&m) => {
                self.near[(m - 2) as usize].checked_add(varint(d, i)?)?
            }
            m => {
                let m = (m as usize).checked_sub(2 + S_NEAR)?;
                if m >= S_SAME {
                    return None;
                }
                let b = *d.get(*i)? as usize;
                *i += 1;
                self.same[m * 256 + b]
            }
        };
        self.update(addr);
        Some(addr)
    }
}

/// Apply an RFC 3284 delta to `src`, producing the target. `None` on a
/// bad magic, a flagged feature this decoder doesn't implement
/// (secondary compression, custom code table), truncated sections, or an
/// instruction stream that doesn't produce exactly the declared target
/// window size.
pub fn apply(src: &[u8], delta: &[u8]) -> Option<Vec<u8>> {
    if delta.get(..4)? != [0xD6, 0xC3, 0xC4, 0x00] {
        return None;
    }
    let mut i = 4;
    let hdr = *delta.get(i)?;
    i += 1;
    if hdr & 0x03 != 0 {
        return None; // VCD_DECOMPRESS / VCD_CODETABLE unsupported
    }
    if hdr & !0x03 != 0 {
        return None; // reserved bits must be zero
    }
    let mut out = Vec::new();
    while i < delta.len() {
        let win = *delta.get(i)?;
        i += 1;
        let (src_kind, s_at, s_len): (usize, usize, usize) = match win & 0x03 {
            0 => (0, 0, 0),
            1 | 2 => {
                let len = varint(delta, &mut i)? as usize;
                let pos = varint(delta, &mut i)? as usize;
                ((win & 0x03) as usize, pos, len)
            }
            _ => return None, // both source and target flagged
        };
        let _delta_len = varint(delta, &mut i)?;
        let t_len = varint(delta, &mut i)? as usize;
        if *delta.get(i)? != 0 {
            return None; // Delta_Indicator: compressed sections unsupported
        }
        i += 1;
        let d_len = varint(delta, &mut i)? as usize;
        let i_len = varint(delta, &mut i)? as usize;
        let a_len = varint(delta, &mut i)? as usize;
        let data = delta.get(i..i.checked_add(d_len)?)?;
        i += d_len;
        let inst = delta.get(i..i.checked_add(i_len)?)?;
        i += i_len;
        let addr = delta.get(i..i.checked_add(a_len)?)?;
        i += a_len;

        run_window(
            &Win {
                src,
                kind: src_kind,
                at: s_at,
                len: s_len,
                t_len,
                data,
                inst,
                addr,
            },
            &mut out,
        )?;
    }
    Some(out)
}

/// One decoded window's inputs: the source-window selector
/// (`kind` = VCD_SOURCE/VCD_TARGET/0 → slice `src[at..at+len]` or
/// `out[at..at+len]` or empty), the declared target length, and the
/// three code sections.
struct Win<'a> {
    src: &'a [u8],
    kind: usize,
    at: usize,
    len: usize,
    t_len: usize,
    data: &'a [u8],
    inst: &'a [u8],
    addr: &'a [u8],
}

/// Decode one window: `sw` is the address space's source prefix (from
/// the file's source for VCD_SOURCE, from `out` so far for VCD_TARGET,
/// empty when unflagged), and `here` starts at `sw.len()`.
fn run_window(w: &Win<'_>, out: &mut Vec<u8>) -> Option<()> {
    // materialize the source window (owned — it may alias `out` when
    // VCD_TARGET is set, and `out` keeps growing under it)
    let sw: Vec<u8> = match w.kind {
        0 => Vec::new(),
        1 => w.src.get(w.at..w.at.checked_add(w.len)?)?.to_vec(),
        _ => out.get(w.at..w.at.checked_add(w.len)?)?.to_vec(),
    };
    let base = out.len();
    let mut cache = Cache::new();
    let mut di = 0usize; // data cursor
    let mut ii = 0usize; // instruction cursor
    let mut ai = 0usize; // address cursor
    let mut made = 0usize;
    while ii < w.inst.len() && made < w.t_len {
        let (i1, s1, m1, i2, s2, m2) = code(*w.inst.get(ii)?);
        ii += 1;
        for &(op, sz, mode) in [(i1, s1, m1), (i2, s2, m2)].iter() {
            if op == NOOP {
                continue;
            }
            let size = if sz == 0 {
                varint(w.inst, &mut ii)? as usize
            } else {
                sz as usize
            };
            if made + size > w.t_len {
                return None;
            }
            match op {
                ADD => {
                    let lit = w.data.get(di..di.checked_add(size)?)?;
                    out.extend_from_slice(lit);
                    di += size;
                }
                RUN => {
                    let b = *w.data.get(di)?;
                    di += 1;
                    out.resize(out.len() + size, b);
                }
                COPY => {
                    let here = (sw.len() + made) as u64;
                    let a = cache.addr(w.addr, &mut ai, here, mode)? as usize;
                    // byte-wise so a COPY may overlap bytes it just emitted
                    copy_from(&sw, out, base, a, size)?;
                }
                _ => return None,
            }
            made += size;
        }
    }
    if made != w.t_len || ii != w.inst.len() {
        return None;
    }
    Some(())
}

fn copy_from(sw: &[u8], out: &mut Vec<u8>, base: usize, a: usize, size: usize) -> Option<()> {
    for k in 0..size {
        let src_idx = a.checked_add(k)?;
        let b = if src_idx < sw.len() {
            *sw.get(src_idx)?
        } else {
            *out.get(base + src_idx - sw.len())?
        };
        out.push(b);
    }
    Some(())
}

/// Emit a valid delta that produces `target` regardless of source — a
/// single unflagged window of `ADD` instructions. Useful for tests and
/// for "the whole file changed" patches.
pub fn emit_add(target: &[u8]) -> Vec<u8> {
    let mut inst = Vec::new();
    let mut rest = target;
    while !rest.is_empty() {
        let n = rest.len().min(17);
        inst.push((n + 1) as u8); // code 2..18: ADD n
        rest = &rest[n..];
    }
    let body_len = 1
        + vn_size(target.len() as u64)
        + 1
        + vn_size(target.len() as u64)
        + vn_size(inst.len() as u64)
        + vn_size(0)
        + target.len()
        + inst.len();
    let mut d = Vec::new();
    d.extend_from_slice(&[0xD6, 0xC3, 0xC4, 0x00]);
    d.push(0); // Hdr_Indicator
    d.push(0); // Win_Indicator: no source window
    enc_varint(body_len as u64, &mut d);
    enc_varint(target.len() as u64, &mut d);
    d.push(0); // Delta_Indicator
    enc_varint(target.len() as u64, &mut d);
    enc_varint(inst.len() as u64, &mut d);
    enc_varint(0, &mut d);
    d.extend_from_slice(target);
    d.extend_from_slice(&inst);
    d
}

fn vn_size(mut v: u64) -> usize {
    let mut n = 1;
    while v >= 128 {
        v /= 128;
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window_with(
        src_flag: u8,
        spos: usize,
        slen: usize,
        tlen: usize,
        data: &[u8],
        inst: &[u8],
        addr: &[u8],
    ) -> Vec<u8> {
        let mut d = vec![0xD6, 0xC3, 0xC4, 0x00, 0x00];
        d.push(src_flag);
        if src_flag != 0 {
            enc_varint(slen as u64, &mut d);
            enc_varint(spos as u64, &mut d);
        }
        let body = vn_size(tlen as u64)
            + 1
            + vn_size(data.len() as u64)
            + vn_size(inst.len() as u64)
            + vn_size(addr.len() as u64)
            + data.len()
            + inst.len()
            + addr.len();
        enc_varint(body as u64, &mut d);
        enc_varint(tlen as u64, &mut d);
        d.push(0);
        enc_varint(data.len() as u64, &mut d);
        enc_varint(inst.len() as u64, &mut d);
        enc_varint(addr.len() as u64, &mut d);
        d.extend_from_slice(data);
        d.extend_from_slice(inst);
        d.extend_from_slice(addr);
        d
    }

    #[test]
    fn add_only_delta() {
        let d = emit_add(b"hello world");
        assert_eq!(apply(b"", &d).unwrap(), b"hello world");
        assert_eq!(apply(b"ignored source", &d).unwrap(), b"hello world");
    }

    #[test]
    fn copy_from_source() {
        // src = "abcdefgh"; window: COPY 4 @0 (VCD_SELF), ADD "XY", COPY 4 @0
        let mut inst = Vec::new();
        inst.push(19); // COPY mode 0 explicit size
        inst.push(4); // size=4
        let mut addr = Vec::new();
        enc_varint(0, &mut addr); // addr 0
        inst.push(3); // ADD 2
        inst.push(19);
        inst.push(4);
        enc_varint(0, &mut addr); // second COPY addr 0 (SELF)
        let d = window_with(1, 0, 8, 10, b"XY", &inst, &addr);
        assert_eq!(apply(b"abcdefgh", &d).unwrap(), b"abcdXYabcd");
    }

    #[test]
    fn run_and_here_mode() {
        // RUN 3 'Z' then COPY 3 with VCD_HERE (addr = here - 3 → re-copies the run)
        let mut inst = vec![0, 3]; // RUN explicit size 3
        inst.push(35); // COPY mode 1 explicit size
        inst.push(3);
        let mut addr = Vec::new();
        enc_varint(3, &mut addr); // here(=3 after run) - addr => addr 0
        let d = window_with(0, 0, 0, 6, b"Z", &inst, &addr);
        assert_eq!(apply(b"", &d).unwrap(), b"ZZZZZZ");
    }

    #[test]
    fn overlapping_copy() {
        // classic "copy a byte run via self-overlap": ADD 'a', COPY 7 @0
        // (HERE mode): after ADD, here=1, addr = 1-1 = 0 → copies the 'a'
        // forward 7 times through the target window.
        let mut inst = vec![2]; // ADD 1
        inst.push(35);
        inst.push(7);
        let mut addr = Vec::new();
        enc_varint(1, &mut addr); // here(1) - 1 = 0
        let d = window_with(0, 0, 0, 8, b"a", &inst, &addr);
        assert_eq!(apply(b"", &d).unwrap(), b"aaaaaaaa");
    }

    #[test]
    fn target_flagged_window() {
        // window1: ADD "abcd"; window2 (VCD_TARGET): COPY 4 from out[0..4]
        let mut d = emit_add(b"abcd");
        // second window: source = target file pos 0 len 4, COPY 4 @0
        let inst = [19u8, 4]; // COPY mode 0 explicit size 4
        let mut addr = Vec::new();
        enc_varint(0, &mut addr);
        let w2 = window_with(2, 0, 4, 4, b"", &inst, &addr);
        d.extend_from_slice(&w2[5..]); // drop magic + Hdr_Indicator
        assert_eq!(apply(b"", &d).unwrap(), b"abcdabcd");
    }

    #[test]
    fn bad_inputs() {
        assert!(apply(b"", b"").is_none());
        assert!(apply(b"", &[0xD6, 0xC3, 0xC4, 0x01, 0]).is_none()); // bad version
        assert!(apply(b"", &[0xD6, 0xC3, 0xC4, 0x00, 0x01]).is_none()); // VCD_DECOMPRESS
        assert!(apply(b"", &[0xD6, 0xC3, 0xC4, 0x00, 0x02]).is_none()); // VCD_CODETABLE
        let mut d = emit_add(b"xy");
        d[5] = 3; // both window flags
        assert!(apply(b"", &d).is_none());
        let mut d = emit_add(b"xy");
        d.truncate(d.len() - 1); // truncated instruction section
        assert!(apply(b"", &d).is_none());
    }

    #[test]
    fn code_table_shape() {
        assert_eq!(code(0), (RUN, 0, 0, NOOP, 0, 0));
        assert_eq!(code(18), (ADD, 17, 0, NOOP, 0, 0));
        assert_eq!(code(19), (COPY, 0, 0, NOOP, 0, 0));
        assert_eq!(code(34), (COPY, 18, 0, NOOP, 0, 0));
        assert_eq!(code(162), (COPY, 18, 8, NOOP, 0, 0));
        assert_eq!(code(163), (ADD, 1, 0, COPY, 4, 0));
        assert_eq!(code(174), (ADD, 4, 0, COPY, 6, 0));
        assert_eq!(code(235), (ADD, 1, 0, COPY, 4, 6));
        assert_eq!(code(246), (ADD, 4, 0, COPY, 4, 8));
        assert_eq!(code(247), (COPY, 4, 0, ADD, 1, 0));
        assert_eq!(code(255), (COPY, 4, 8, ADD, 1, 0));
    }
}
