//! Fluent Bit `fluent-bit.conf` census.
//!
//! Fluent Bit config is INI-ish: `[SERVICE]`/`[INPUT]`/`[FILTER]`/
//! `[OUTPUT]`/`[PARSER]`/`[MULTILINE_PARSER]`/`[CUSTOM]` sections,
//! indented `Key  Value` directives (`Name`, `Match`, `Tag`,
//! `Alias`, `Parser`, `Format`, `Flush`, `Log_Level`, `Daemon`,
//! `Listen`, `Port`, `Path`, `DB`, `Host`, `Exclude`, `Regex`).
//!
//! ```rust
//! let k = b"[SERVICE]\n    Flush        5\n    Log_Level    info\n[INPUT]\n    Name         tail\n    Path         /var/log/x.log\n[OUTPUT]\n    Name         stdout\n    Match        *\n";
//! assert!(izanagi_kit::fluentbit::detect(k));
//! ```

/// Fluent Bit config census.
#[derive(Debug, Clone)]
pub struct Fluentbit {
    /// `[X]` section headers.
    pub sections: usize,
    /// `Key  Value` directive lines.
    pub settings: usize,
    /// recognised sections/directives present.
    pub keys: usize,
    /// `#` comment lines.
    pub comments: usize,
}

const SECTIONS: &[&str] = &[
    "SERVICE",
    "INPUT",
    "FILTER",
    "OUTPUT",
    "PARSER",
    "MULTILINE_PARSER",
    "CUSTOM",
];

const STRONG: &[&str] = &[
    "Name",
    "Match",
    "Tag",
    "Match_Regex",
    "Key_Name",
    "Reserve_Data",
    "Preserve_Key",
    "Reserve_Entity",
    "Parser",
    "Parser_Firstline",
    "Parser_N",
    "Types",
    "Decoder_Key",
    "Decoder_Field",
    "Multiline",
    "Multiline.Parser",
    "Multiline.key_content",
    "Read_From_Head",
    "Skip_Long_Lines",
    "Skip_Empty_Lines",
    "Refresh_Interval",
    "Mem_Buf_Limit",
    "storage.type",
    "storage.backlog.mem_limit",
];

const WEAK: &[&str] = &[
    "Alias",
    "Format",
    "Flush",
    "Log_Level",
    "Daemon",
    "DNS.mode",
    "HTTP_Server",
    "HTTP_Listen",
    "HTTP_Port",
    "Listen",
    "Port",
    "Path",
    "DB",
    "DB.locking",
    "Host",
    "Exclude",
    "Regex",
    "Ignore_Older",
    "Rotate_Wait",
    "Parsers_File",
    "Plugins_File",
    "Streams_File",
    "Workers",
    "Coro_Stack_Size",
    "Metrics",
];

fn section(line: &str) -> Option<&str> {
    let s = line.trim();
    let inner = s.strip_prefix('[')?.strip_suffix(']')?;
    let inner = inner.trim();
    if inner.is_empty() {
        None
    } else {
        Some(inner)
    }
}

fn directive(line: &str) -> Option<&str> {
    if !(line.starts_with(' ') || line.starts_with('\t')) {
        return None;
    }
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') {
        return None;
    }
    let k = s.split([' ', '\t']).next()?;
    if k.is_empty() {
        None
    } else {
        Some(k)
    }
}

/// Detect a Fluent Bit `fluent-bit.conf`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // INPUT/OUTPUT/FILTER pipeline sections plus `Name`/`Match`
    // directives are fluent-bit specific.
    let mut sections = 0usize;
    let mut strong = 0usize;
    let mut weak = 0usize;
    for line in t.lines() {
        if let Some(h) = section(line) {
            if SECTIONS.contains(&h) {
                sections += 1;
            }
            continue;
        }
        if let Some(k) = directive(line) {
            if STRONG.contains(&k) {
                strong += 1;
            } else if WEAK.contains(&k) {
                weak += 1;
            }
        }
    }
    sections >= 2 && strong >= 1 && strong + weak >= 2
}

impl Fluentbit {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            keys: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if section(line).is_some() {
                c.sections += 1;
                continue;
            }
            if let Some(k) = directive(line) {
                c.settings += 1;
                if STRONG.contains(&k) || WEAK.contains(&k) {
                    c.keys += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"[SERVICE]\n    Flush        5\n    Log_Level    info\n[INPUT]\n    Name         tail\n    Path         /var/log/x.log\n[OUTPUT]\n    Name         stdout\n    Match        *\n";
        assert!(detect(b));
        let c = Fluentbit::parse(b).unwrap();
        assert_eq!(c.sections, 3);
        assert!(c.keys >= 4);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"[server]\nport = 53\n[client]\nx = 1\n"));
        assert!(!detect(b"# [INPUT]\n# [OUTPUT]\n[SERVICE]\n"));
    }
}
