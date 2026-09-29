//! HL7 v2 message (`*.hl7`) segment census.
//!
//! Pipe-delimited segments: `MSH|^~\&|` header plus EVN/PID/PV1/ORC/
//! OBR/OBX/NTE/MSA/DG1/IN1/AL1/NK1/PR1/SCH/RXE lines. Counts fields
//! (`|`), components (`^`), repeats (`~`) and subcomponents (`&`).
//!
//! ```
//! let s = b"MSH|^~\\&|SEND|FAC|RECV|FAC2|20200101||ADT^A01|MSG1|P|2\nEVN|A01|20200101\nPID|1||12345||DOE^JOHN\n";
//! assert!(izanagi_kit::hl7::detect(s));
//! let h = izanagi_kit::hl7::Hl7::parse(s).unwrap();
//! assert_eq!(h.segments, 3);
//! assert_eq!(h.msh, 1);
//! assert_eq!(h.evn, 1);
//! assert_eq!(h.pid, 1);
//! assert_eq!(h.message_types, 1);
//! ```

/// Parsed census of an HL7 v2 message.
#[derive(Debug, Clone)]
pub struct Hl7 {
    /// Pipe-delimited segment lines.
    pub segments: usize,
    /// `MSH` segments.
    pub msh: usize,
    /// `EVN` segments.
    pub evn: usize,
    /// `PID` segments.
    pub pid: usize,
    /// `PV1` segments.
    pub pv1: usize,
    /// `ORC` segments.
    pub orc: usize,
    /// `OBR` segments.
    pub obr: usize,
    /// `OBX` segments.
    pub obx: usize,
    /// `NTE` segments.
    pub nte: usize,
    /// `MSA` segments.
    pub msa: usize,
    /// `DG1` segments.
    pub dg1: usize,
    /// `IN1` segments.
    pub in1: usize,
    /// `AL1` segments.
    pub al1: usize,
    /// `NK1` segments.
    pub nk1: usize,
    /// `PR1` segments.
    pub pr1: usize,
    /// `SCH` segments.
    pub sch: usize,
    /// `RXE`/`RXD` segments.
    pub rxe: usize,
    /// `MSH-9` message types (`ADT^A01` shapes).
    pub message_types: usize,
    /// `|` field separators.
    pub fields: usize,
    /// `^` components.
    pub components: usize,
    /// `~` repeats.
    pub repeats: usize,
    /// `&` subcomponents.
    pub subcomponents: usize,
    /// Segments with an unrecognized 3-char id.
    pub other_segments: usize,
}

const KNOWN: &[&str] = &[
    "MSH", "EVN", "PID", "PV1", "ORC", "OBR", "OBX", "NTE", "MSA", "DG1", "IN1", "AL1", "NK1",
    "PR1", "SCH", "RXE", "RXD", "CTI", "SPC", "TQ1", "TXA", "FT1", "GT1", "PD1", "QRD", "QRF",
    "ERR", "BHS", "BTS", "FHS", "FTS", "BLG", "IAM", "ACC", "UB1", "UB2", "DSC", "DSP", "MRG",
];

/// Reports whether `b` looks like an HL7 v2 message.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    t.starts_with("MSH|") && (t.contains("|^~\\&|") || t.contains("MSH|^"))
}

impl Hl7 {
    /// Parses `b` as an HL7 message, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut h = Hl7 {
            segments: 0,
            msh: 0,
            evn: 0,
            pid: 0,
            pv1: 0,
            orc: 0,
            obr: 0,
            obx: 0,
            nte: 0,
            msa: 0,
            dg1: 0,
            in1: 0,
            al1: 0,
            nk1: 0,
            pr1: 0,
            sch: 0,
            rxe: 0,
            message_types: 0,
            fields: 0,
            components: 0,
            repeats: 0,
            subcomponents: 0,
            other_segments: 0,
        };
        for l in t.lines() {
            let l = l.trim_end_matches(['\n', '\r']);
            if l.len() < 4 || !l.contains('|') {
                continue;
            }
            h.segments += 1;
            h.fields += l.matches('|').count();
            h.components += l.matches('^').count();
            h.repeats += l.matches('~').count();
            h.subcomponents += l.matches('&').count();
            let id = &l[..3];
            if !KNOWN.contains(&id) {
                h.other_segments += 1;
            }
            match id {
                "MSH" => {
                    h.msh += 1;
                    if l.split('|').nth(8).is_some_and(|f| f.contains('^')) {
                        h.message_types += 1;
                    }
                }
                "EVN" => h.evn += 1,
                "PID" => h.pid += 1,
                "PV1" => h.pv1 += 1,
                "ORC" => h.orc += 1,
                "OBR" => h.obr += 1,
                "OBX" => h.obx += 1,
                "NTE" => h.nte += 1,
                "MSA" => h.msa += 1,
                "DG1" => h.dg1 += 1,
                "IN1" => h.in1 += 1,
                "AL1" => h.al1 += 1,
                "NK1" => h.nk1 += 1,
                "PR1" => h.pr1 += 1,
                "SCH" => h.sch += 1,
                "RXE" | "RXD" => h.rxe += 1,
                _ => {}
            }
        }
        Some(h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] = b"MSH|^~\\&|SEND|FAC|RECV|FAC2|20200101||ADT^A01|MSG1|P|2\nEVN|A01|20200101\nPID|1||12345||DOE^JOHN\n";

    #[test]
    fn parses_hl7() {
        assert!(detect(S));
        let h = Hl7::parse(S).unwrap();
        assert_eq!(h.segments, 3);
        assert_eq!(h.msh, 1);
        assert_eq!(h.evn, 1);
        assert_eq!(h.pid, 1);
        assert_eq!(h.message_types, 1);
    }

    #[test]
    fn rejects_non_hl7() {
        assert!(!detect(b"PID|1||x"));
        assert!(Hl7::parse(b"").is_none());
    }
}
