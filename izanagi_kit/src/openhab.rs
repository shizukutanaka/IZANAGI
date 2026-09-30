//! openHAB `.items`/`.things`/`.rules`/`.sitemap` census.
//!
//! Items: `<Type> <name> "label" <icon> (groups) ["tags"] {channel="..."}`
//! with types Switch/Dimmer/Color/Contact/DateTime/Number/Rollershutter/
//! String/Group/Image/Location/Player/Call/Rollershutter. Things:
//! `Thing <binding>:<type>:<id> "label" @ "loc" [ params ]` / `Bridge`/
//! `bridge` / `Channels { ... }`. Rules: `rule "name" when ... then ... end`
//! with `Item X changed`/`received command`/`System started`/`Time cron`/
//! `Channel triggered` triggers. Sitemaps: `sitemap <name> label="" { Frame/
//! Text/Switch/Slider/Setpoint/Selection/Group/Chart/Video/Image/Webview/
//! Mapview/Default/Colorpicker elements }`.
//!
//! ```rust
//! let i = concat!(
//!     "Switch Living_Light \"Living\" <light> (gLiving) [\"Switchable\"] {channel=\"hue:1\"}\n",
//!     "Number Temp \"T [%.1f]\" <temperature>\n",
//!     "Group gLiving \"Living\"\n",
//! );
//! let c = izanagi_kit::openhab::Openhab::parse(i.as_bytes()).unwrap();
//! assert_eq!(c.items, 3);
//! ```

const ITEMS: &[&str] = &[
    "Switch",
    "Dimmer",
    "Color",
    "Contact",
    "DateTime",
    "Number",
    "Rollershutter",
    "String",
    "Group",
    "Image",
    "Location",
    "Player",
    "Call",
    "Number:",
    "String:",
    "Switch:",
    "Dimmer:",
    "Contact:",
    "DateTime:",
    "Color:",
    "Rollershutter:",
    "Location:",
    "Group:",
];

const SITEMAP: &[&str] = &[
    "Frame",
    "Text",
    "Switch",
    "Slider",
    "Setpoint",
    "Selection",
    "Group",
    "Chart",
    "Video",
    "Image",
    "Webview",
    "Mapview",
    "Default",
    "Colorpicker",
    "List",
    "Buttongrid",
    "sitemap",
];

/// openHAB configuration census.
#[derive(Debug, Clone)]
pub struct Openhab {
    /// Item definitions (`<Type> <name> ...`).
    pub items: usize,
    /// `Thing`/`Bridge`/`bridge`/`Channels`/`Thing ... {` declarations.
    pub things: usize,
    /// `{channel=`/`channel="`/`{ga=`/`{alexa=`/`{homekit=`/`{expire=`/`{autoupdate=`/`{mqtt=`/`{http=`/`{modbus=`/`{knx=`/`{ihc=` bindings + metadata.
    pub bindings: usize,
    /// `rule "..."`/`when`/`then`/`end`/`triggeringItem`/`System started`/`Time cron`/`Item ... changed`/`received command`/`Channel ... triggered` rule constructs.
    pub rules: usize,
    /// Sitemap elements (`Frame`/`Text`/`Switch`/`Group`/`Selection`/... at statement start inside sitemap syntax).
    pub sitemap_elems: usize,
    /// Group `(group)`/`["tag"]` memberships + `<icon>` icon refs.
    pub refs: usize,
    /// `val`/`var`/`sendCommand`/`postUpdate`/`createTimer`/`triggeringItem`/`import`/`rule `/`when`/`then`/`end` occurrences in rules files.
    pub scripting: usize,
}

/// Whether the buffer looks like an openHAB config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    ITEMS.iter().any(|k| t.contains(&format!("{k} ")))
        || t.contains("Thing ")
        || t.contains("rule \"")
        || t.contains("sitemap ")
        || t.contains("Bridge ")
        || t.contains("{channel=")
}

impl Openhab {
    /// Parse openHAB config text into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            items: 0,
            things: 0,
            bindings: 0,
            rules: 0,
            sitemap_elems: 0,
            refs: 0,
            scripting: 0,
        };
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with("//") || s.starts_with('#') || s == "}" {
                continue;
            }
            let head = s.split_whitespace().next().unwrap_or("");
            if ITEMS.contains(&head) {
                c.items += 1;
                if s.contains('(') {
                    c.refs += 1;
                }
                if s.contains('<') {
                    c.refs += 1;
                }
                if s.contains('{') {
                    c.bindings += 1;
                }
                continue;
            }
            if head == "Thing"
                || head == "Bridge"
                || head == "bridge"
                || head == "Channels"
                || s == "Thing"
                || s.starts_with("Thing ")
            {
                c.things += 1;
                continue;
            }
            if head == "sitemap" || head == "Sitemap" {
                c.sitemap_elems += 1;
                continue;
            }
            if SITEMAP.contains(&head)
                && (s.contains('{')
                    || s.contains("label=")
                    || s.contains("item=")
                    || s.contains("icon=")
                    || s.contains("mappings="))
            {
                c.sitemap_elems += 1;
                continue;
            }
            if head == "rule" || s.starts_with("rule \"") {
                c.rules += 1;
                continue;
            }
            if head == "when"
                || head == "then"
                || head == "end"
                || s.starts_with("Item ") && (s.contains("changed") || s.contains("received"))
                || s.starts_with("Time ")
                || s.starts_with("System ")
                || s.starts_with("Channel ")
                || s.starts_with("Thing ")
                || s.contains("triggeringItem")
            {
                c.rules += 1;
                continue;
            }
            if head == "val"
                || head == "var"
                || s.contains("sendCommand")
                || s.contains("postUpdate")
                || s.contains("createTimer")
                || head == "import"
                || s.contains("now.")
            {
                c.scripting += 1;
                continue;
            }
            if s.contains("{channel=")
                || s.contains("channel=\"")
                || s.contains("{ga=")
                || s.contains("{alexa=")
                || s.contains("{homekit=")
                || s.contains("{expire=")
                || s.contains("{autoupdate=")
                || s.contains("{mqtt=")
                || s.contains("{http=")
                || s.contains("{modbus=")
                || s.contains("{knx=")
                || s.contains("{ihc=")
            {
                c.bindings += 1;
                continue;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_items() {
        let b = concat!(
            "Switch Living_Light \"Living\" <light> (gLiving) [\"Switchable\"] {channel=\"hue:1\"}\n",
            "Number Temp \"T\" <temperature> (gTemp)\n",
            "Group gLiving \"Living\"\n",
            "Dimmer D1 \"D\" {channel=\"zwave:2\"}\n",
            "Contact Door \"Door\" <contact>\n",
        );
        let c = Openhab::parse(b.as_bytes()).unwrap();
        assert_eq!(c.items, 5);
        assert_eq!(c.bindings, 2);
        assert_eq!(c.refs, 5);
    }

    #[test]
    fn parses_rules() {
        let b = concat!(
            "rule \"Night\"\n",
            "when\n",
            "    Item Sun changed\n",
            "then\n",
            "    Living_Light.sendCommand(OFF)\n",
            "end\n",
        );
        let c = Openhab::parse(b.as_bytes()).unwrap();
        assert_eq!(c.rules, 5);
        assert_eq!(c.scripting, 1);
    }

    #[test]
    fn rejects_other() {
        assert!(Openhab::parse(b"foo = 1").is_none());
    }
}
