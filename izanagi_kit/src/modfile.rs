//! ProTracker `.mod` modules — the tracker music format born on the
//! Amiga and still circulating in demoscene/chiptune archives. Layout:
//! 20-byte song title, then 31 sample headers of 30 bytes (name 22,
//! length/loop points as big-endian *words*), song length, restart
//! byte, the 128-entry pattern order table, a 4-byte channel magic
//! (`M.K.`/`M!K!`/`FLT4`/`FLT8`/`NCHN`/`NNCH`), then pattern data —
//! `64 rows × channels × 4 bytes` per pattern — then raw signed-8-bit
//! sample data.
//!
//! The original Ultimate-Tracker-era format carried only 15 samples
//! and *no* magic word; [`parse`] detects both layouts (no recognized
//! signature at offset 1080 → 15-sample form at offset 600).
//!
//! ```
//! use izanagi_kit::modfile::{parse, PERIODS};
//!
//! // minimal M.K. module: 1 pattern of silence, 1 sample of 4 bytes.
//! let mut d = [0u8; 20].to_vec(); // title
//! let mut smp = [0u8; 30];
//! smp[22..24].copy_from_slice(&2u16.to_be_bytes()); // 2 words = 4 bytes
//! smp[25] = 64; // volume
//! for _ in 0..31 {
//!     d.extend_from_slice(&smp);
//! }
//! d.push(1); // song length
//! d.push(127); // restart
//! d.extend_from_slice(&[0u8; 128]); // order table
//! d.extend_from_slice(b"M.K.");
//! d.extend_from_slice(&[0u8; 1024]); // 1 pattern x 64 rows x 4ch x 4B
//! d.extend_from_slice(&[1, 2, 3, 4]); // sample data
//! let m = parse(&d).unwrap();
//! assert_eq!(m.channels, 4);
//! assert_eq!(m.samples[0].len_bytes, 4);
//! assert_eq!(m.sample_data(&d, 0), Some(&[1, 2, 3, 4][..]));
//! assert_eq!(PERIODS[0], 856);
//! ```

use std::string::String;
use std::vec::Vec;

/// ProTracker finetune-0 period table, C-1 through B-3. A period's
/// playback frequency on a PAL Amiga is `7093789.2 / (2 * period)`
/// Hz — see [`amiga_hz_x100`].
pub const PERIODS: [u16; 36] = [
    856, 808, 762, 720, 678, 640, 604, 570, 538, 508, 480, 453, 428, 404, 381, 360, 339, 320, 302,
    285, 269, 254, 240, 226, 214, 202, 190, 180, 170, 160, 151, 143, 135, 127, 120, 113,
];

/// PAL Amiga clock in hundredths of Hz (`709378920` — avoids floats
/// per the crate's determinism rules). Frequency of a period is
/// `AMIGA_CLOCK_X100 / (2 * period)` hundredths of Hz.
pub const AMIGA_CLOCK_X100: u64 = 709_378_920;

/// Playback frequency of a tracker `period` in hundredths of Hz
/// (period 0 → 0).
pub fn amiga_hz_x100(period: u16) -> u32 {
    if period == 0 {
        return 0;
    }
    (AMIGA_CLOCK_X100 / (u64::from(period) * 2)) as u32
}

/// One 30-byte sample header (data follows the patterns).
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    /// 22-byte name, NUL/space padding trimmed.
    pub name: String,
    /// Sample length in *bytes* (the wire stores words).
    pub len_bytes: usize,
    /// Finetune nibble decoded as −8…7 (values 8–15 are negative).
    pub finetune: i8,
    /// Default volume 0–64.
    pub volume: u8,
    /// Loop start, bytes.
    pub loop_start: usize,
    /// Loop length, bytes (2 means no loop on the wire).
    pub loop_len: usize,
}

/// One cell of pattern data (4 bytes on the wire).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    /// 12-bit period; index [`PERIODS`] or [`amiga_hz_x100`].
    pub period: u16,
    /// Sample number 1–31 (0 = no change).
    pub sample: u8,
    /// Effect command nibble (`0xC` = set volume, `0xF` = speed…).
    pub effect: u8,
    /// Effect parameter byte.
    pub param: u8,
}

/// A parsed module.
#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    /// Song title (20 bytes, trimmed).
    pub title: String,
    /// 31 (or 15, pre-signature files) sample headers.
    pub samples: Vec<Sample>,
    /// Song length in pattern-order entries.
    pub song_len: usize,
    /// Restart position byte (historically abused; often 127).
    pub restart: u8,
    /// Pattern order table: `song_len` leading entries are live.
    pub order: Vec<u8>,
    /// Channel count derived from the signature.
    pub channels: usize,
    /// All patterns, `64 × channels` events each, row-major then
    /// channel.
    pub patterns: Vec<Vec<Event>>,
    /// File offset where the concatenated sample data begins.
    pub sample_data_at: usize,
}

fn u16be(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) << 8 | u16::from(*d.get(at + 1)?))
}

fn name_str(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b {
        if c == 0 {
            break;
        }
        if c.is_ascii() {
            s.push(c as char);
        }
    }
    s.trim_end().to_string()
}

/// Channel count declared by the 4-byte signature at offset 1080 —
/// `None` when no recognized signature is present (15-sample form).
fn channels_of(sig: &[u8]) -> Option<usize> {
    match sig {
        b"M.K." | b"M!K!" | b"FLT4" | b"4CHN" => Some(4),
        b"FLT8" => Some(8),
        b"6CHN" | b"8CHN" => Some(usize::from(sig[0] - b'0')),
        _ => {
            // NNCH: two digits + "CH" (e.g. 12CH, 32CH).
            if sig[0].is_ascii_digit() && sig[1].is_ascii_digit() && &sig[2..] == b"CH" {
                Some(usize::from(sig[0] - b'0') * 10 + usize::from(sig[1] - b'0'))
            } else if sig[0].is_ascii_digit() && &sig[1..] == b"CHN" {
                Some(usize::from(sig[0] - b'0'))
            } else {
                None
            }
        }
    }
}

fn decode_event(b: &[u8]) -> Event {
    Event {
        period: u16::from(b[0] & 0x0f) << 8 | u16::from(b[1]),
        sample: (b[0] & 0xf0) | (b[2] >> 4),
        effect: b[2] & 0x0f,
        param: b[3],
    }
}

/// Parse a `.mod` file. `None` on truncation, a pattern count or
/// sample length that overruns the file, or a `song_len` of 0.
/// The `order` table is clamped to the live `song_len` prefix when
/// pattern indices appear only in padding.
pub fn parse(d: &[u8]) -> Option<Module> {
    if d.len() < 600 {
        return None;
    }
    // Try the 31-sample signature at offset 1080 first.
    let (n_samples, channels, base) = match d.get(1080..1084).and_then(channels_of) {
        Some(ch) => (31usize, ch, 1084usize),
        None => (15, 4, 600),
    };
    let header_len = 20 + n_samples * 30;
    if d.len() < header_len + 130 {
        return None;
    }
    let title = name_str(d.get(..20)?);
    let mut samples = Vec::with_capacity(n_samples);
    for i in 0..n_samples {
        let at = 20 + i * 30;
        let h = d.get(at..at + 30)?;
        let finetune_raw = h[24] & 0x0f;
        samples.push(Sample {
            name: name_str(&h[..22]),
            len_bytes: usize::from(u16be(h, 22)?) * 2,
            finetune: if finetune_raw > 7 {
                finetune_raw as i8 - 16
            } else {
                finetune_raw as i8
            },
            volume: h[25].min(64),
            loop_start: usize::from(u16be(h, 26)?) * 2,
            loop_len: usize::from(u16be(h, 28)?) * 2,
        });
    }
    let pos = header_len;
    let song_len = usize::from(d[pos]);
    let restart = d[pos + 1];
    if song_len == 0 || song_len > 128 {
        return None;
    }
    let table = d.get(pos + 2..pos + 130)?;
    let mut order = Vec::with_capacity(128);
    order.extend_from_slice(table);
    // Pattern count: highest order entry among the live positions.
    let used = &order[..song_len];
    let n_patterns = used.iter().copied().max().map(|m| usize::from(m) + 1)?;
    let pat_bytes = n_patterns
        .checked_mul(64)?
        .checked_mul(channels)?
        .checked_mul(4)?;
    let pat_at = base;
    if pat_at.checked_add(pat_bytes)? > d.len() {
        return None;
    }
    let mut patterns = Vec::with_capacity(n_patterns);
    for p in 0..n_patterns {
        let start = pat_at + p * 64 * channels * 4;
        let mut cells = Vec::with_capacity(64 * channels);
        for i in 0..64 * channels {
            cells.push(decode_event(&d[start + i * 4..start + i * 4 + 4]));
        }
        patterns.push(cells);
    }
    Some(Module {
        title,
        samples,
        song_len,
        restart,
        order,
        channels,
        patterns,
        sample_data_at: pat_at + pat_bytes,
    })
}

impl Module {
    /// The events of pattern `p` (64 rows × `channels`).
    pub fn pattern(&self, p: usize) -> Option<&[Event]> {
        self.patterns.get(p).map(Vec::as_slice)
    }

    /// The `channels`-wide event row at `row` of pattern `p`.
    pub fn row(&self, p: usize, row: usize) -> Option<&[Event]> {
        if row >= 64 {
            return None;
        }
        let pat = self.pattern(p)?;
        pat.get(row * self.channels..row * self.channels + self.channels)
    }

    /// The PCM bytes of sample `i` (signed 8-bit) inside the module
    /// image `d`.
    pub fn sample_data<'a>(&self, d: &'a [u8], i: usize) -> Option<&'a [u8]> {
        let s = self.samples.get(i)?;
        let mut at = self.sample_data_at;
        for prev in &self.samples[..i] {
            at = at.checked_add(prev.len_bytes)?;
        }
        d.get(at..at.checked_add(s.len_bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module(sig: &[u8], order: &[u8], song_len: u8) -> Vec<u8> {
        let mut d = vec![0u8; 20];
        d[..9].copy_from_slice(b"test song");
        for _ in 0..31 {
            let mut s = [0u8; 30];
            s[..7].copy_from_slice(b"lead v1");
            s[22..24].copy_from_slice(&2u16.to_be_bytes());
            s[24] = 12; // finetune -4
            s[25] = 40;
            s[26..28].copy_from_slice(&1u16.to_be_bytes());
            s[28..30].copy_from_slice(&2u16.to_be_bytes());
            d.extend_from_slice(&s);
        }
        d.push(song_len);
        d.push(127);
        let mut t = [0u8; 128];
        t[..order.len()].copy_from_slice(order);
        d.extend_from_slice(&t);
        d.extend_from_slice(sig);
        let ch = channels_of(sig).unwrap_or(4);
        let npat = usize::from(*order.iter().max().unwrap_or(&0)) + 1;
        d.extend_from_slice(&vec![0u8; npat * 64 * ch * 4]);
        d
    }

    #[test]
    fn mk_module_reads_events_and_samples() {
        let mut d = module(b"M.K.", &[0, 1], 2);
        // write an event into pattern 1, row 0, ch 0: period 214, sample 5, effect F, param 0x20
        let ev = 1084 + 64 * 4 * 4; // pattern 1 starts after pattern 0
        d[ev] = 0x00; // period hi nibble 0, sample hi nibble 0
        d[ev + 1] = 214;
        d[ev + 2] = 0x5f; // sample lo nibble 5, effect 0xF
        d[ev + 3] = 0x20;
        d.extend_from_slice(&[7u8; 31 * 4]);
        let m = parse(&d).unwrap();
        assert_eq!(m.title, "test song");
        assert_eq!(m.channels, 4);
        assert_eq!(m.song_len, 2);
        assert_eq!(m.patterns.len(), 2);
        let e = m.row(1, 0).unwrap()[0];
        assert_eq!((e.period, e.sample, e.effect, e.param), (214, 5, 15, 0x20));
        assert_eq!(m.samples[0].finetune, -4);
        assert_eq!(m.sample_data(&d, 0), Some(&[7, 7, 7, 7][..]));
        assert_eq!(m.sample_data(&d, 30), Some(&[7, 7, 7, 7][..]));
        assert_eq!(m.sample_data(&d, 31), None);
        assert_eq!(amiga_hz_x100(428), 828713);
        assert_eq!(amiga_hz_x100(0), 0);
        assert_eq!(PERIODS[PERIODS.len() - 1], 113);
    }

    #[test]
    fn signatures_select_channels() {
        for (sig, ch) in [
            (&b"M.K."[..], 4usize),
            (&b"M!K!"[..], 4),
            (&b"FLT8"[..], 8),
            (&b"6CHN"[..], 6),
            (&b"12CH"[..], 12),
            (&b"32CH"[..], 32),
        ] {
            let d = module(sig, &[0], 1);
            assert_eq!(parse(&d).unwrap().channels, ch, "{:?}", sig);
        }
    }

    #[test]
    fn legacy_15_sample_form() {
        let mut d = vec![0u8; 20];
        d[..4].copy_from_slice(b"old!");
        for _ in 0..15 {
            let mut s = [0u8; 30];
            s[22..24].copy_from_slice(&1u16.to_be_bytes());
            d.extend_from_slice(&s);
        }
        d.push(1);
        d.push(0);
        d.extend_from_slice(&[0u8; 128]);
        // no signature — pattern data starts at 600
        d.extend_from_slice(&[0u8; 64 * 4 * 4]);
        d.extend_from_slice(&[0u8; 28]);
        d.extend_from_slice(&[9, 9]);
        let m = parse(&d).unwrap();
        assert_eq!(m.channels, 4);
        assert_eq!(m.samples.len(), 15);
        assert_eq!(m.title, "old!");
        assert_eq!(m.sample_data(&d, 14), Some(&[9, 9][..]));
    }

    #[test]
    fn malformed() {
        assert!(parse(b"").is_none());
        // song_len 0
        let d = module(b"M.K.", &[0], 0);
        assert!(parse(&d).is_none());
        // pattern data truncated
        let mut d = module(b"M.K.", &[3], 1);
        d.truncate(1084 + 10);
        assert!(parse(&d).is_none());
    }
}
