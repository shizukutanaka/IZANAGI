//! Sendmail `sendmail.cf` census.
//!
//! The file is a flat list of one-letter command lines:
//! `O <option>` (options), `K <name> <class>` (maps),
//! `M <mailer>` (delivery agents), `R <rules>` (rewrite rules),
//! `S <ruleset>` (ruleset headers), `D <macro>` (macros),
//! `C`/`F` (class definitions/files), `T` (trusted users),
//! `P` (priority classes), `V` (version), `H` (header defs),
//! `L` (local info), `E` (environment), `Q` (queue groups),
//! `X` (special maps, +cf), `cp.`/`dnl` m4 leftovers and `#` comments.
//!
//! ```rust
//! let s = "V10/Berkeley\nCwlocalhost\nO DaemonPortOptions=Port=smtp,Addr=127.0.0.1\nDsmtp:example.org\nS0\nR$+ $: $>0 $1\n";
//! let c = izanagi_kit::sendmail::Sendmail::parse(s.as_bytes()).unwrap();
//! assert_eq!(c.options, 1);
//! assert_eq!(c.rules, 1);
//! ```

/// sendmail.cf census.
#[derive(Debug, Clone)]
pub struct Sendmail {
    /// `O`/`O ` option lines.
    pub options: usize,
    /// `K` map declarations.
    pub maps: usize,
    /// `M` mailer definitions.
    pub mailers: usize,
    /// `R` rewrite rules.
    pub rules: usize,
    /// `S` ruleset headers.
    pub rulesets: usize,
    /// `D` macro definitions.
    pub macros: usize,
    /// `C`/`F` class lines.
    pub classes: usize,
    /// `T` trusted-user lines.
    pub trusted: usize,
    /// `H` header definitions.
    pub headers: usize,
    /// `V` version lines.
    pub versions: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Detect sendmail.cf content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut hits = 0usize;
    let mut v = false;
    for line in t.lines() {
        match line.as_bytes().first().copied().unwrap_or(0) {
            b'V' => v = true,
            b'R' => hits += 1,
            b'S' => {
                if line.len() <= 12 {
                    hits += 1;
                }
            }
            b'O' | b'K' | b'M' | b'D' => {
                let second_ok = line.as_bytes().get(1) == Some(&b' ')
                    || line
                        .as_bytes()
                        .get(1)
                        .is_some_and(|c| c.is_ascii_alphanumeric());
                if second_ok {
                    hits += 1;
                }
            }
            _ => {}
        }
    }
    (v && hits >= 2) || hits >= 4
}

impl Sendmail {
    /// Census a sendmail.cf buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            options: 0,
            maps: 0,
            mailers: 0,
            rules: 0,
            rulesets: 0,
            macros: 0,
            classes: 0,
            trusted: 0,
            headers: 0,
            versions: 0,
            comments: 0,
        };
        for line in t.lines() {
            if line.is_empty() {
                continue;
            }
            if line.starts_with('#') || line.starts_with("dnl") {
                c.comments += 1;
                continue;
            }
            let first = line.as_bytes()[0];
            match first {
                b'O' if line.as_bytes().get(1) == Some(&b' ') => c.options += 1,
                b'K' => c.maps += 1,
                b'M' => c.mailers += 1,
                b'R' => c.rules += 1,
                b'S' => c.rulesets += 1,
                b'D' => c.macros += 1,
                b'C' | b'F' => c.classes += 1,
                b'T' => c.trusted += 1,
                b'H' => c.headers += 1,
                b'V' => c.versions += 1,
                _ => {}
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_cf() {
        let b = b"V10/Berkeley\nO DaemonPortOptions=Port=smtp\nS0\nR$+ $:$1\nS1\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_cf() {
        let b = concat!(
            "# sendmail.cf\n",
            "V10/Berkeley\n",
            "Cwlocalhost\n",
            "Fw/etc/mail/local-host-names\n",
            "O DaemonPortOptions=Port=smtp,Addr=127.0.0.1\n",
            "O Timeout.connect=1m\n",
            "Dsmtp.example.org\n",
            "Kaccess hash -T<TMPF> /etc/mail/access\n",
            "Mlocal, P=/usr/bin/procmail, F=DFMmSPfhnu9, S=EnvFromL/HdrFromL\n",
            "Troot daemon\n",
            "H?P?Return-Path: <$g>\n",
            "S0\n",
            "R$+ $: $>Parse0 $1\n",
            "R$* $#error $@ 5.7.1 $: \"550 rejected\"\n",
            "S96\n",
            "R$+ $: $1 < @ $j >\n",
        );
        let c = Sendmail::parse(b.as_bytes()).unwrap();
        assert_eq!(c.versions, 1);
        assert_eq!(c.options, 2);
        assert_eq!(c.maps, 1);
        assert_eq!(c.mailers, 1);
        assert_eq!(c.rules, 3);
        assert_eq!(c.rulesets, 2);
        assert_eq!(c.macros, 1);
        assert_eq!(c.classes, 2);
        assert_eq!(c.trusted, 1);
        assert_eq!(c.headers, 1);
        assert_eq!(c.comments, 1);
    }
}
