//! Kodi `advancedsettings.xml` census.
//!
//! Kodi advancedsettings.xml is nested XML under `<advancedsettings>`:
//! `<video>`/`<network>`/`<audio>`/`<videolibrary>`/`<musiclibrary>`/
//! `<pvr>`/`<edl>`/`<karaoke>`/`<samba>`/`<pathsubstitution>`/
//! `<database>`/`<tvshowmatching>`/`<episodename>`/`<fanart>`/
//! `<subtitleextensions>`/`<playlistasfolders>`/`<loglevel>`/
//! `<buffermode>`/`<cachemembuffersize>`/`<remotedelay>`/
//! `<cputempcommand>`/`<gputempcommand>` elements.
//!
//! ```rust
//! let c = izanagi_kit::kodiadv::Kodiadv::parse(b"<advancedsettings>\n<network>\n<buffermode>1</buffermode>\n</network>\n<loglevel>1</loglevel>\n</advancedsettings>").unwrap();
//! assert_eq!(c.entries, 2);
//! ```

/// Kodi `advancedsettings.xml` census.
#[derive(Debug, Clone)]
pub struct Kodiadv {
    /// Elements opening a nested block.
    pub sections: usize,
    /// `<key>value</key>`/self-closing leaf elements.
    pub entries: usize,
    /// `<!-- -->` comments.
    pub comments: usize,
}

const KEYS: &[&str] = &[
    "loglevel",
    "buffermode",
    "cachemembuffersize",
    "fanartheight",
    "imageres",
    "videolibrary",
    "musiclibrary",
    "pvr",
    "samba",
    "pathsubstitution",
    "tvshowmatching",
    "episodename",
    "remotedelay",
    "cputempcommand",
    "gputempcommand",
    "subtitleextensions",
    "playlistasfolders",
    "seeksteps",
];

/// Whether the buffer looks like an advancedsettings.xml.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<advancedsettings") && KEYS.iter().filter(|k| t.contains(**k)).count() >= 2
}

impl Kodiadv {
    /// Parse an advancedsettings.xml into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            entries: 0,
            comments: 0,
        };
        c.comments = t.matches("<!--").count();
        for l in t.lines() {
            let s = l.trim();
            if let Some(rest) = s.strip_prefix('<') {
                if rest.starts_with('!') || rest.starts_with('?') || rest.starts_with('/') {
                    continue;
                }
                if let Some(gt) = rest.find('>') {
                    let head = &rest[..gt];
                    let name = head.split(' ').next().unwrap_or("");
                    if name.is_empty()
                        || name == "advancedsettings"
                        || !name.bytes().all(|x| x.is_ascii_alphanumeric() || x == b'_')
                    {
                        continue;
                    }
                    let val = &rest[gt + 1..];
                    if rest.ends_with("/>") || val.contains("</") {
                        c.entries += 1;
                    } else {
                        c.sections += 1;
                    }
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
    fn parses_advancedsettings() {
        let b = concat!(
            "<advancedsettings version=\"1.0\">\n",
            "  <video>\n",
            "    <excludefromscan>extras</excludefromscan>\n",
            "  </video>\n",
            "  <network>\n",
            "    <buffermode>1</buffermode>\n",
            "    <cachemembuffersize>20971520</cachemembuffersize>\n",
            "  </network>\n",
            "  <loglevel>1</loglevel>\n",
            "  <videolibrary>\n",
            "    <cleanonupdate>true</cleanonupdate>\n",
            "  </videolibrary>\n",
            "  <samba>\n",
            "    <minversion>2</minversion>\n",
            "  </samba>\n",
            "</advancedsettings>\n",
        );
        let c = Kodiadv::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.entries, 6);
    }

    #[test]
    fn rejects_other() {
        assert!(Kodiadv::parse(b"<advancedsettings></advancedsettings>").is_none());
    }
}
