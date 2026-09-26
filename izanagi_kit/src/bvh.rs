//! BVH — Biovision motion capture. A text file: `HIERARCHY` opens a
//! skeleton of `ROOT`/`JOINT` nodes each carrying `CHANNELS n` (6 for
//! a root — position + rotation, 3 for joints); `MOTION` opens the
//! frame block with `Frames: n` and `Frame Time: t`. Floats are
//! avoided: `Frame Time` is parsed into integer milliunits.
//!
//! ```
//! use izanagi_kit::bvh::{parse, joints, frames, frame_time_ms};
//! let d = b"HIERARCHY\nROOT Hips\n{\n CHANNELS 6 a b c d e f\n}\nMOTION\nFrames: 2\nFrame Time: 0.033333\n0 0 0\n";
//! let b = parse(d).unwrap();
//! assert_eq!(b.root, b"Hips");
//! assert_eq!(frames(d), Some(2));
//! assert_eq!(frame_time_ms(d), Some(33));
//! ```

/// A parsed header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bvh {
    /// Root joint name.
    pub root: Vec<u8>,
    /// Total joints seen (`ROOT` + every `JOINT`).
    pub joints: u32,
}

fn lines(d: &[u8]) -> impl Iterator<Item = &[u8]> + '_ {
    d.split(|&c| c == b'\n')
}

fn words(l: &[u8]) -> impl Iterator<Item = &[u8]> + '_ {
    l.split(|&c| c == b' ' || c == b'\t')
        .filter(|w| !w.is_empty())
}

fn key_value<'l>(d: &'l [u8], key: &[u8]) -> Option<&'l [u8]> {
    lines(d).find_map(|l| {
        let l = l
            .iter()
            .position(|&c| c != b' ' && c != b'\t')
            .map_or(l, |p| &l[p..]);
        if l.get(..key.len())? == key {
            let rest = l.get(key.len()..)?;
            rest.iter()
                .position(|&c| c != b' ' && c != b'\t')
                .and_then(|p| {
                    let v = &rest[p..];
                    let end = v
                        .iter()
                        .position(|&c| c == b' ' || c == b'\t')
                        .unwrap_or(v.len());
                    (!v[..end].is_empty()).then_some(&v[..end])
                })
        } else {
            None
        }
    })
}

/// Parse the hierarchy head: requires `HIERARCHY` then a `ROOT` name.
/// Counts every `JOINT` line anywhere in the file.
pub fn parse(d: &[u8]) -> Option<Bvh> {
    let mut saw_hierarchy = false;
    let mut root = None;
    let mut joints = 0u32;
    for l in lines(d) {
        let mut w = words(l);
        match w.next() {
            Some(b"HIERARCHY") => saw_hierarchy = true,
            Some(b"ROOT") if root.is_none() => {
                root = Some(w.next()?.to_vec());
                joints += 1;
            }
            Some(b"JOINT") => joints += 1,
            Some(b"MOTION") => break,
            _ => {}
        }
    }
    if !saw_hierarchy {
        return None;
    }
    Some(Bvh {
        root: root?,
        joints,
    })
}

/// Names of `JOINT` lines (the root is `Bvh::root`).
pub fn joints(d: &[u8]) -> Vec<Vec<u8>> {
    lines(d)
        .filter_map(|l| {
            let mut w = words(l);
            (w.next()? == b"JOINT").then(|| w.next().unwrap_or(b"").to_vec())
        })
        .collect()
}

/// `Frames:` value in the `MOTION` block.
pub fn frames(d: &[u8]) -> Option<u32> {
    let v = key_value(d, b"Frames:")?;
    if v.is_empty() || !v.iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    v.iter().try_fold(0u32, |n, &c| {
        n.checked_mul(10)?.checked_add(u32::from(c - b'0'))
    })
}

/// `Frame Time:` as integer milliunits (`0.033333` → 33).
pub fn frame_time_ms(d: &[u8]) -> Option<u32> {
    let v = key_value(d, b"Frame Time:")?;
    let mut ms = 0u32;
    let mut frac = false;
    let mut digits = 0u32;
    let mut seen = false;
    for &c in v {
        match c {
            b'0'..=b'9' => {
                seen = true;
                if !frac {
                    ms = ms.checked_mul(10)?.checked_add(u32::from(c - b'0'))?;
                } else if digits < 3 {
                    ms = ms.checked_mul(10)?.checked_add(u32::from(c - b'0'))?;
                    digits += 1;
                }
            }
            b'.' if !frac => frac = true,
            _ => return None,
        }
    }
    if !seen {
        return None;
    }
    if frac {
        // int part was multiplied as whole number — fold: value was
        // parsed as `ms = int` then each of ≤3 frac digits appended.
        // e.g. "0.033333" → int 0, frac digits 0,3,3 → ms = 33.
        while digits < 3 {
            ms = ms.checked_mul(10)?;
            digits += 1;
        }
        Some(ms)
    } else {
        ms.checked_mul(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        b"HIERARCHY\nROOT Hips\n{\n\tCHANNELS 6 Xposition Yposition Zposition Zrotation Xrotation Yrotation\n\tJOINT Spine\n\t{\n\t\tCHANNELS 3 a b c\n\t}\n}\nMOTION\nFrames: 480\nFrame Time: 0.033333\n".to_vec()
    }

    #[test]
    fn parses_hierarchy() {
        let d = fixture();
        let b = parse(&d).unwrap();
        assert_eq!(b.root, b"Hips");
        assert_eq!(b.joints, 2);
        assert_eq!(joints(&d), vec![b"Spine".to_vec()]);
        assert_eq!(frames(&d), Some(480));
        assert_eq!(frame_time_ms(&d), Some(33));
    }

    #[test]
    fn frame_time_variants() {
        assert_eq!(frame_time_ms(b"MOTION\nFrame Time: 1.5\n"), Some(1500));
        assert_eq!(frame_time_ms(b"MOTION\nFrame Time: 2\n"), Some(2000));
        assert_eq!(frame_time_ms(b"nothing"), None);
        assert_eq!(frames(b"Frames: x\n"), None);
        assert_eq!(frames(b"Frames: 3\n"), Some(3));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"MOTION\nFrames: 1\n").is_none());
        // no ROOT name
        assert!(parse(b"HIERARCHY\nROOT\n").is_none());
    }
}
