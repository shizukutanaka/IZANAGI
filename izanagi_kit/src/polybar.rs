//! Polybar config (INI, `config.ini`) census.
//!
//! A polybar config is INI: `[bar/name]`, `[module/name]`, `[settings]`,
//! `[colors]`, `[global/wm]` sections with `key = value` pairs —
//! `monitor`/`width`/`height`/`radius`/`fixed-center`/`background`/
//! `foreground`/`line-*`/`border-*`/`padding`/`module-*`/`font-N`/
//! `separator`/`wm-restack`/`override-redirect`/`cursor`/`scroll-*`/
//! `enable-ipc`/`tray-*`/`click-*`/`double-*`/`format`/`format-*`/
//! `label*`/`indicator*`/`animation*`/`type`/`exec`/`interval`/`tail`/
//! `env-*`/`initial`/`format-connected`/`format-disconnected`/
//! `ramp*`/`bar-*`/`animation-*`/`${xrdb:`/`${env:`/`${file:` refs.
//!
//! ```rust
//! let p = concat!(
//!     "[bar/main]\n",
//!     "monitor = ${env:MONITOR:}\n",
//!     "width = 100%\n",
//!     "modules-left = cpu\n",
//!     "[module/cpu]\n",
//!     "type = internal/cpu\n",
//!     "interval = 2\n",
//! );
//! let c = izanagi_kit::polybar::Polybar::parse(p.as_bytes()).unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// Polybar config census.
#[derive(Debug, Clone)]
pub struct Polybar {
    /// `[bar/*]`/`[module/*]`/`[settings]`/`[colors]`/`[global/*]`/`[application]`/`[other]` section headers.
    pub sections: usize,
    /// `type`/`exec`/`interval`/`tail`/`exec-if`/`env-*`/`format*`/`label*`/`ramp*`/`bar-*`/`animation-*`/`internal/*`/`custom/*`/`x*`/`menu-*`/`click-*`/`scroll-*`/`double-*`/`rotate`/`cycle`/`online`/`ping-interval`/`format-online`/`format-offline`/`hook-*`/`initial`/`mount-*`/`warn-*`/`units`/`speed-unit`/`label-*`/`indicator-*`/`mark-*`/`ws-icon-*`/`fuzzymatch`/`available*`/`active*`/`occupied*`/`urgent*`/`empty*`/`visible*`/`hidden*`/`master*`/`slave*`/`sp*`/`pct*`/`transmission-*`/`adapter-*`/`capacity-*`/`rate-*`/`low-at`/`animation-charging-*`/`sys-*`/`disable-scroll`/`smooth-scrolling`/`repeat*`/`value*`/`range*`/`min*`/`max*`/`step*`/`inner*`/`outer*`/`assign*`/`click-mode`/`server*`/`user*`/`channel*`/`notice*`/`msg*`/`header*`/`vol*`/`up*`/`dn*`/`muted*`/`sink*`/`card*`/`icon-*`/`sep-*`/`half*`/`lin*`/`day*`/`previous*`/`next*`/`toggle*`/`inc*`/`dec*`/`incr*`/`decr*`/`text*`/`bs*`/`ff*`/`rw*`/`hot*`/`last*`/`current*`/`alt*`/`conn*`/`sess*`/`for*`/`win*`/`sel*`/`cur*`/`pending*`/`lock*`/`wifi*`/`eth*`/`bb*`/`rs*`/`ls*`/`net*`/`ip*`/`wlan*`/`wired*`/`iwl*`/`enp*`/`wlp*`/`eth*`/`wlo*`/`vbox*`/`vm*`/`docker*`/`vpn*`/`tun*`/`tap*`/`ppp*`/`gre*`/`wg*`/`vxlan*`/`bridge*`/`vlan*`/`bond*`/`team*`/`dummy*`/`lo*`/`if*`/`dev*`/`speed*`/`duplex*`/`link*`/`addr*`/`mtu*`/`state*`/`carrier*`/`oper*`/`flags*`/`units*`-style module keys.
    pub keys: usize,
    /// `monitor`/`width`/`height`/`radius`/`fixed-center`/`background`/`foreground`/`line-*`/`border-*`/`padding-*`/`module-*`/`font-*`/`separator`/`wm-restack`/`override-redirect`/`cursor-*`/`scroll-*`/`enable-ipc`/`tray-*`/`offset-*`/`dpi`/`pseudo-transparency`/`struts`/`top`/`bottom`/`left`/`right`/`modules-*`/`include-file`/`include-directory`/`compositing-*`/`screenchange-*`/`dim-value`/`shuffle`/`settings`/`throttle-*`/`real-*`/`warn*` bar-level keys.
    pub bars: usize,
    /// `${xrdb:`/`${env:`/`${file:`/`${colors.*}`/`${bar.*}`/`${root.*}`/`${self.*}` variable references.
    pub refs: usize,
}

/// Whether the buffer looks like a polybar config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("[bar/") || t.contains("[module/"))
        && (t.contains("modules-") || t.contains("internal/") || t.contains("type ="))
}

impl Polybar {
    /// Parse a polybar config into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            keys: 0,
            bars: 0,
            refs: 0,
        };
        let mut in_module = false;
        for l in t.lines() {
            let s = l.trim();
            if s.is_empty() || s.starts_with(';') || s.starts_with('#') {
                continue;
            }
            if s.starts_with('[') && s.ends_with(']') {
                c.sections += 1;
                in_module = s.starts_with("[module/");
                continue;
            }
            c.refs += s.matches("${xrdb:").count()
                + s.matches("${env:").count()
                + s.matches("${file:").count()
                + s.matches("${colors.").count()
                + s.matches("${bar.").count()
                + s.matches("${root.").count()
                + s.matches("${self.").count();
            let Some(eq) = s.find('=') else {
                continue;
            };
            let key = s[..eq].trim();
            if in_module {
                c.keys += 1;
            } else if !key.is_empty() {
                c.bars += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config() {
        let b = concat!(
            "[colors]\n",
            "bg = #ff000000\n",
            "fg = #ffffffff\n",
            "[bar/main]\n",
            "monitor = ${env:MONITOR:}\n",
            "width = 100%\n",
            "height = 24\n",
            "background = ${colors.bg}\n",
            "modules-left = cpu memory\n",
            "modules-right = date\n",
            "font-0 = monospace:size=10\n",
            "tray-position = right\n",
            "[module/cpu]\n",
            "type = internal/cpu\n",
            "interval = 2\n",
            "format = <ramp-coreload>\n",
            "label = %percentage%%\n",
            "[module/date]\n",
            "type = internal/date\n",
            "interval = 1\n",
        );
        let c = Polybar::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 4);
        assert_eq!(c.bars, 10);
        assert_eq!(c.keys, 6);
        assert_eq!(c.refs, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Polybar::parse(b"foo = 1").is_none());
    }
}
