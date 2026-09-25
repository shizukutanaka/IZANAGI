//! Standard MIDI File (SMF) parser — `MThd`/`MTrk` containers, delta
//! varints, running status, meta and SysEx events, and note on/off
//! pairing. The media-family sibling of [`crate::png`]/[`crate::gif`]
//! for score data.
//!
//! Parsing is total: any malformed header, truncated event, bad
//! varint, or missing end-of-track degrades to `None`. Absolute ticks
//! are accumulated during the walk (deltas stay relative on the wire).
//!
//! ```
//! use izanagi_kit::midi::{parse, EvKind};
//!
//! // format-0, PPQN 96, one track: note-on 60/96, note-off at +96, EOT.
//! let smf: &[u8] = &[
//!     b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 1, 0, 96,
//!     b'M', b'T', b'r', b'k', 0, 0, 0, 12,
//!     0x00, 0x90, 60, 96, 0x60, 0x80, 60, 0, // delta, ev, args
//!     0x00, 0xff, 0x2f, 0x00, // end of track
//! ];
//! let m = parse(smf).unwrap();
//! assert_eq!(m.division, 96);
//! assert_eq!(m.tracks[0].events.len(), 3);
//! assert!(matches!(m.tracks[0].events[0].kind, EvKind::NoteOn { note: 60, .. }));
//! ```

use std::vec::Vec;

/// A parsed event at an absolute tick.
pub struct Ev {
    /// Absolute tick from track start.
    pub tick: u32,
    /// Event payload.
    pub kind: EvKind,
}

/// MIDI event kinds (channel voice + meta + sysex).
#[derive(Clone, PartialEq, Debug)]
pub enum EvKind {
    /// Note on (`vel == 0` is still `NoteOn` on the wire — see
    /// [`notes`] for the off-folded view).
    NoteOn {
        /// channel 0–15
        ch: u8,
        /// note number 0–127
        note: u8,
        /// velocity 0–127
        vel: u8,
    },
    /// Note off.
    NoteOff {
        /// channel
        ch: u8,
        /// note
        note: u8,
        /// release velocity
        vel: u8,
    },
    /// Polyphonic key pressure.
    Aftertouch {
        /// channel
        ch: u8,
        /// note
        note: u8,
        /// pressure
        vel: u8,
    },
    /// Control change.
    Cc {
        /// channel
        ch: u8,
        /// controller number
        cc: u8,
        /// value
        val: u8,
    },
    /// Program change.
    Program {
        /// channel
        ch: u8,
        /// program
        pg: u8,
    },
    /// Channel pressure.
    Pressure {
        /// channel
        ch: u8,
        /// pressure
        val: u8,
    },
    /// Pitch bend, 14-bit centered at 8192.
    PitchBend {
        /// channel
        ch: u8,
        /// raw 14-bit value
        val: u16,
    },
    /// Meta event (`type`, payload).
    Meta {
        /// meta type byte
        ty: u8,
        /// payload bytes
        data: Vec<u8>,
    },
    /// SysEx payload (without the `F0`/`F7` lead byte).
    Sysex(Vec<u8>),
}

/// One `MTrk` track.
pub struct Track {
    /// Events in wire order, absolute ticks.
    pub events: Vec<Ev>,
}

/// A parsed SMF file.
pub struct Midi {
    /// SMF format 0/1/2.
    pub format: u16,
    /// Division: ticks per quarter note (PPQN) when < 0x8000; the
    /// SMPTE rate word is kept verbatim otherwise.
    pub division: u16,
    /// Tracks.
    pub tracks: Vec<Track>,
}

fn be16(b: &[u8]) -> u16 {
    ((b[0] as u16) << 8) | b[1] as u16
}
fn be32(b: &[u8]) -> u32 {
    ((b[0] as u32) << 24) | ((b[1] as u32) << 16) | ((b[2] as u32) << 8) | b[3] as u32
}

fn varint(data: &[u8], at: &mut usize) -> Option<u32> {
    let mut v = 0u32;
    for _ in 0..4 {
        if *at >= data.len() {
            return None;
        }
        let b = data[*at];
        *at += 1;
        v = (v << 7) | (b & 0x7f) as u32;
        if b & 0x80 == 0 {
            return Some(v);
        }
    }
    None // 5th byte would exceed 28 bits
}

fn parse_track(data: &[u8]) -> Option<Track> {
    let mut events = Vec::new();
    let mut at = 0usize;
    let mut tick = 0u32;
    let mut status = 0u8; // running status
    let mut saw_eot = false;
    while at < data.len() {
        tick = tick.checked_add(varint(data, &mut at)?)?;
        if at >= data.len() {
            return None;
        }
        let b = data[at];
        if b & 0x80 != 0 {
            status = b;
            at += 1;
        } else if status == 0 {
            return None; // data byte without running status
        }
        let need = |n: usize, at: usize| -> Option<()> {
            if at + n > data.len() {
                None
            } else {
                Some(())
            }
        };
        let kind = match status {
            0xf0 | 0xf7 => {
                if data[at] & 0x80 != 0 {
                    // status byte already consumed; sysex len next
                    let len = varint(data, &mut at)? as usize;
                    need(len, at)?;
                    let payload = data[at..at + len].to_vec();
                    at += len;
                    EvKind::Sysex(payload)
                } else {
                    return None;
                }
            }
            0xff => {
                if at >= data.len() {
                    return None;
                }
                let ty = data[at];
                at += 1;
                let len = varint(data, &mut at)? as usize;
                need(len, at)?;
                let payload = data[at..at + len].to_vec();
                at += len;
                if ty == 0x2f {
                    saw_eot = true;
                }
                EvKind::Meta { ty, data: payload }
            }
            s if (0x80..0xf0).contains(&s) => {
                let ch = s & 0x0f;
                let two = matches!(s >> 4, 0x8 | 0x9 | 0xa | 0xb | 0xe);
                if two {
                    need(2, at)?;
                    let (a, c) = (data[at], data[at + 1]);
                    at += 2;
                    match s >> 4 {
                        0x8 => EvKind::NoteOff {
                            ch,
                            note: a,
                            vel: c,
                        },
                        0x9 => EvKind::NoteOn {
                            ch,
                            note: a,
                            vel: c,
                        },
                        0xa => EvKind::Aftertouch {
                            ch,
                            note: a,
                            vel: c,
                        },
                        0xb => EvKind::Cc { ch, cc: a, val: c },
                        _ => EvKind::PitchBend {
                            ch,
                            val: ((c as u16) << 7) | a as u16,
                        },
                    }
                } else {
                    need(1, at)?;
                    let a = data[at];
                    at += 1;
                    match s >> 4 {
                        0xc => EvKind::Program { ch, pg: a },
                        _ => EvKind::Pressure { ch, val: a },
                    }
                }
            }
            _ => return None, // realtime bytes can't appear in files
        };
        events.push(Ev { tick, kind });
        if saw_eot {
            break;
        }
    }
    if !saw_eot {
        return None;
    }
    Some(Track { events })
}

/// Parse an SMF stream. `None` on bad magic, truncated chunks, or a
/// track without end-of-track.
pub fn parse(data: &[u8]) -> Option<Midi> {
    if data.len() < 14 || data[..4] != *b"MThd" {
        return None;
    }
    let hlen = be32(&data[4..]) as usize;
    if hlen < 6 || 8 + hlen > data.len() {
        return None;
    }
    let h = &data[8..8 + hlen];
    let format = be16(&h[0..]);
    let ntracks = be16(&h[2..]) as usize;
    let division = be16(&h[4..]);
    if format > 2 || ntracks == 0 {
        return None;
    }
    let mut at = 8 + hlen;
    let mut tracks = Vec::with_capacity(ntracks);
    for _ in 0..ntracks {
        if at + 8 > data.len() || data[at..at + 4] != *b"MTrk" {
            return None;
        }
        let tlen = be32(&data[at + 4..]) as usize;
        let body = at + 8;
        if body + tlen > data.len() {
            return None;
        }
        tracks.push(parse_track(&data[body..body + tlen])?);
        at = body + tlen;
    }
    Some(Midi {
        format,
        division,
        tracks,
    })
}

/// Fold a track into sounding notes `(start_tick, duration, ch, note,
/// vel)`: each `NoteOn` with `vel > 0` pairs with the next matching
/// `NoteOff`/`NoteOn(vel=0)` on the same channel+note. Unclosed notes
/// are dropped — a malformed score degrades, never panics.
pub fn notes(track: &Track) -> Vec<(u32, u32, u8, u8, u8)> {
    let mut open: Vec<(u32, u8, u8, u8)> = Vec::new(); // (start, ch, note, vel)
    let mut out = Vec::new();
    for ev in &track.events {
        match ev.kind {
            EvKind::NoteOn { ch, note, vel } if vel > 0 => {
                open.push((ev.tick, ch, note, vel));
            }
            EvKind::NoteOff { ch, note, .. } | EvKind::NoteOn { ch, note, vel: 0 } => {
                let (ch, note) = (ch, note);
                if let Some(i) = open.iter().position(|&(_, c, n, _)| c == ch && n == note) {
                    let (start, _, _, vel) = open.remove(i);
                    out.push((start, ev.tick - start, ch, note, vel));
                }
            }
            _ => {}
        }
    }
    out
}

/// Tempo map: `(tick, µs per quarter)` from every `0x51` meta event.
pub fn tempo_map(track: &Track) -> Vec<(u32, u32)> {
    track
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EvKind::Meta { ty: 0x51, data } if data.len() == 3 => Some((
                e.tick,
                ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | data[2] as u32,
            )),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn smf(track_body: &[u8], division: u16) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"MThd");
        v.extend_from_slice(&[0, 0, 0, 6, 0, 0, 0, 1]);
        v.extend_from_slice(&[(division >> 8) as u8, division as u8]);
        v.extend_from_slice(b"MTrk");
        let tl = track_body.len() as u32;
        v.extend_from_slice(&[
            (tl >> 24) as u8,
            (tl >> 16) as u8,
            (tl >> 8) as u8,
            tl as u8,
        ]);
        v.extend_from_slice(track_body);
        v
    }

    #[test]
    fn simple_score() {
        let m = parse(&smf(
            &[
                0x00, 0x90, 60, 96, // note on
                0x60, 0x80, 60, 0, // +96 note off
                0x00, 0xff, 0x2f, 0x00, // end of track
            ],
            96,
        ))
        .unwrap();
        assert_eq!(m.format, 0);
        assert_eq!(m.division, 96);
        assert_eq!(m.tracks[0].events.len(), 3);
        let ns = notes(&m.tracks[0]);
        assert_eq!(ns, [(0, 96, 0, 60, 96)]);
    }

    #[test]
    fn running_status_and_eot() {
        let m = parse(&smf(
            &[
                0x00, 0x90, 60, 96, 0x10, 64, 96, // running status: no 0x90
                0x10, 60, 0, // vel-0 acts as note off for 60
                0x00, 64, 0, // off for 64
                0x00, 0xff, 0x2f, 0x00,
            ],
            480,
        ))
        .unwrap();
        assert_eq!(m.tracks[0].events.len(), 5);
        let ns = notes(&m.tracks[0]);
        assert_eq!(ns.len(), 2);
        assert_eq!(ns[0], (0, 32, 0, 60, 96));
        assert_eq!(ns[1], (16, 16, 0, 64, 96));
    }

    #[test]
    fn tempo_meta() {
        // 120 BPM = 500000 µs per quarter.
        let m = parse(&smf(
            &[
                0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20, 0x00, 0xff, 0x2f, 0x00,
            ],
            96,
        ))
        .unwrap();
        assert_eq!(tempo_map(&m.tracks[0]), [(0, 500_000)]);
    }

    #[test]
    fn varint_boundaries() {
        // delta 0x7f → one byte; 0x80 → two bytes 0x81 0x00.
        let m = parse(&smf(
            &[
                0x00, 0x90, 60, 96, 0x81, 0x00, 0x80, 60, 0, // delta = 128
                0x00, 0xff, 0x2f, 0x00,
            ],
            96,
        ))
        .unwrap();
        assert_eq!(notes(&m.tracks[0])[0].1, 128);
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(b"").is_none());
        assert!(parse(b"MThd\x00\x00\x00\x06\x00\x00\x00\x00\x00\x60").is_none());
        // Missing end-of-track.
        assert!(parse(&smf(&[0x00, 0x90, 60, 96], 96)).is_none());
        // Bad varint (5 bytes).
        assert!(parse(&smf(&[0x81, 0x81, 0x81, 0x81, 0x81, 0x90, 60, 96], 96)).is_none());
    }
}
