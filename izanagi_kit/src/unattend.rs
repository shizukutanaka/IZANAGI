//! Windows `unattend.xml`/`autounattend.xml` answer-file census.
//!
//! Unattended-setup answer files are XML under `<unattend>` with
//! `<settings pass="windowsPE|offlineServicing|generalize|
//! specialize|auditSystem|auditUser|oobeSystem">` blocks containing
//! `<component name="Microsoft-Windows-…">`, `<CreatePartition>`,
//! `<ImageInstall>`/`<OSImage>`/`<InstallFrom>`/`<InstallTo>`,
//! `<UserData>`/`<ProductKey>`/`<AcceptEula>`,
//! `<LocalAccount>`/`<AdministratorPassword>`,
//! `<oobeSystem>`/`<OOBE>`/`<SkipMachineOOBE>` settings.
//!
//! ```rust
//! let c = izanagi_kit::unattend::Unattend::parse(b"<unattend>\n<settings pass=\"oobeSystem\">\n<component name=\"Microsoft-Windows-Setup\">\n</component>\n</settings>\n</unattend>\n").unwrap();
//! assert_eq!(c.passes, 1);
//! ```

/// `unattend.xml`/`autounattend.xml` census.
#[derive(Debug, Clone)]
pub struct Unattend {
    /// `<settings pass="…">` blocks.
    pub passes: usize,
    /// `<component …>` blocks.
    pub components: usize,
    /// `<key>value</key>`/`<tag …/>` leaf elements.
    pub entries: usize,
    /// `<!-- -->` comments.
    pub comments: usize,
}

fn strip_comments(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let mut rest = t;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start + 4..].find("-->") {
            Some(end) => rest = &rest[start + 4 + end + 3..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Whether the buffer looks like an answer file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let t = strip_comments(t);
    (t.contains("<unattend") || t.contains("<autounattend"))
        && (t.contains("<settings") || t.contains("Microsoft-Windows-") || t.contains("<component"))
}

impl Unattend {
    /// Parse an answer file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = strip_comments(std::str::from_utf8(b).ok()?);
        let mut c = Self {
            passes: 0,
            components: 0,
            entries: 0,
            comments: 0,
        };
        c.comments = t.matches("<!--").count();
        for l in t.lines() {
            let s = l.trim();
            if s.contains("<settings") {
                c.passes += 1;
                continue;
            }
            if s.contains("<component") {
                c.components += 1;
                continue;
            }
            if let Some(rest) = s.strip_prefix('<') {
                if rest.starts_with('!') || rest.starts_with('?') || rest.starts_with('/') {
                    continue;
                }
                if rest.contains('>') {
                    let head = rest.split('>').next().unwrap_or("");
                    let name = head.split(' ').next().unwrap_or("").trim_end_matches('/');
                    if name.is_empty()
                        || name == "unattend"
                        || name == "autounattend"
                        || !name
                            .bytes()
                            .all(|x| x.is_ascii_alphanumeric() || x == b'_' || x == b'-')
                    {
                        continue;
                    }
                    c.entries += 1;
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
    fn parses_unattend() {
        let b = concat!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n",
            "<unattend xmlns=\"urn:schemas-microsoft-com:unattend\">\n",
            "<settings pass=\"windowsPE\">\n",
            "<component name=\"Microsoft-Windows-Setup\">\n",
            "<ImageInstall>\n",
            "<OSImage>\n",
            "<InstallTo>\n",
            "<DiskID>0</DiskID>\n",
            "<PartitionID>1</PartitionID>\n",
            "</InstallTo>\n",
            "</OSImage>\n",
            "</ImageInstall>\n",
            "<UserData>\n",
            "<ProductKey>\n",
            "<Key>XXXXX-XXXXX</Key>\n",
            "</ProductKey>\n",
            "<AcceptEula>true</AcceptEula>\n",
            "</UserData>\n",
            "</component>\n",
            "</settings>\n",
            "<settings pass=\"specialize\">\n",
            "<component name=\"Microsoft-Windows-Deployment\">\n",
            "<RunSynchronousCommand/>\n",
            "</component>\n",
            "</settings>\n",
            "</unattend>\n",
        );
        let c = Unattend::parse(b.as_bytes()).unwrap();
        assert_eq!(c.passes, 2);
        assert_eq!(c.components, 2);
        assert_eq!(c.entries, 10);
    }

    #[test]
    fn rejects_other() {
        assert!(Unattend::parse(b"<root></root>").is_none());
    }

    #[test]
    fn xml_comments_are_stripped() {
        let t = strip_comments(
            "<a><!-- hidden
<config -->x</a>",
        );
        assert_eq!(t, "<a>x</a>");
        let u = strip_comments("<a><!-- unterminated");
        assert_eq!(u, "<a>");
    }
}
