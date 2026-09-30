//! Node-RED `flows.json` census.
//!
//! A flows file is a JSON array of node objects: `{"id", "type", "name",
//! "x", "y", "z", "wires"}` — types like `tab`, `subflow`, `group`,
//! `inject`, `debug`, `function`, `switch`, `change`, `delay`, `split`,
//! `join`, `template`, `http in`/`http request`/`http response`, `mqtt in`/
//! `mqtt out`/`mqtt-broker`, `link in`/`link out`/`link call`, `websocket`,
//! `tcp`/`udp`/`serial`, `exec`, `file`, `watch`, `csv`/`json`/`xml`/`yaml`/
//! `html`, `rbe`, `trigger`, `batch`, `sort`, `range`, `complete`, `catch`,
//! `status`, `comment`, `global-config`, `env`, `conf-*`.
//!
//! ```rust
//! let j = concat!(
//!     "[\n",
//!     "  {\"id\":\"1\",\"type\":\"tab\",\"label\":\"f\"},\n",
//!     "  {\"id\":\"2\",\"type\":\"inject\",\"z\":\"1\",\"wires\":[[\"3\"]]},\n",
//!     "  {\"id\":\"3\",\"type\":\"debug\",\"z\":\"1\"}\n",
//!     "]\n",
//! );
//! let c = izanagi_kit::nodered::Nodered::parse(j.as_bytes()).unwrap();
//! assert_eq!(c.nodes, 3);
//! ```

const TYPES: &[&str] = &[
    "tab",
    "subflow",
    "group",
    "inject",
    "debug",
    "function",
    "switch",
    "change",
    "delay",
    "split",
    "join",
    "template",
    "http in",
    "http request",
    "http response",
    "http-proxy",
    "mqtt in",
    "mqtt out",
    "mqtt-broker",
    "link in",
    "link out",
    "link call",
    "websocket",
    "websocket-listener",
    "websocket-client",
    "tcp in",
    "tcp out",
    "tcp request",
    "udp in",
    "udp out",
    "serial in",
    "serial out",
    "serial request",
    "exec",
    "daemon",
    "file",
    "file in",
    "watch",
    "csv",
    "json",
    "xml",
    "yaml",
    "html",
    "rbe",
    "trigger",
    "batch",
    "sort",
    "range",
    "complete",
    "catch",
    "status",
    "comment",
    "global-config",
    "env",
    "function-exec",
    "tls-config",
    "http-config",
    "ui_base",
    "junction",
    "pause",
    "loop",
    "while",
    "counter",
    "switch-node",
    "random",
    "interval",
    "poll-state",
    "current-state",
    "events: state",
    "server-state-changed",
    "ha-",
    "call-service",
    "api",
    "webhook",
    "cronplus",
    "bigtimer",
    "schedex",
    "timer",
    "timeframerange",
    "ui_button",
    "ui_switch",
    "ui_slider",
    "ui_numeric",
    "ui_text",
    "ui_text_input",
    "ui_date_picker",
    "ui_dropdown",
    "ui_colour_picker",
    "ui_form",
    "ui_gauge",
    "ui_chart",
    "ui_audio",
    "ui_toast",
    "ui_ui_control",
    "ui_template",
    "ui_link",
    "ui_group",
    "ui_tab",
    "ui_spacer",
    "site",
    "email",
    "e-mail",
    "redis",
    "sqlite",
    "mysql",
    "postgres",
    "mongodb",
    "influxdb",
    "influxdb in",
    "influxdb out",
    "influxdb batch",
    "pushbullet",
    "pushover",
    "telegram",
    "twilio",
    "xmpp",
    "feedparser",
    "rss",
    "twitter",
    "irc",
    "slack",
    "discord",
    "alexa",
    "home-assistant",
    "node-red-contrib-",
    "ping",
    "tail",
    "exec-node",
    "fs-ops",
    "moment",
    "humanizer",
    "base64",
    "msg-resend",
    "msg-speed",
    "smooth",
    "aggregate",
    "deduplicate",
    "statemachine",
    "statistics",
    "average",
    "min",
    "max",
    "sum",
    "median",
    "string",
    "array",
    "object",
    "buffer",
    "parser",
    "split",
    "reverse",
    "truncate",
    "wordwrap",
];

/// Node-RED flows census.
#[derive(Debug, Clone)]
pub struct Nodered {
    /// Node objects (`"id"` occurrences).
    pub nodes: usize,
    /// `tab`/`subflow`/`group` container nodes.
    pub containers: usize,
    /// `inject`/`debug`/`function`/`switch`/`change`/`trigger`/`delay` nodes.
    pub core: usize,
    /// `http`/`mqtt`/`tcp`/`udp`/`websocket`/`serial`/`link` I/O nodes.
    pub io: usize,
    /// `ui_*`/`site` dashboard nodes.
    pub ui: usize,
    /// `file`/`exec`/`tail`/`watch`/`fs`/`csv`/`xml`/`yaml`/`html`/`json`/`template`/`base64`/`moment` data nodes.
    pub data: usize,
    /// `"wires"` wiring arrays.
    pub wires: usize,
    /// `"env"`/`"config"`/`"credentials"`/`"outputs"`/`"props"` keys.
    pub misc: usize,
}

/// Whether the buffer looks like a flows.json file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("\"wires\"") && t.contains("\"type\""))
        || t.contains("\"type\": \"tab\"")
        || t.contains("\"type\":\"tab\"")
}

impl Nodered {
    /// Parse flows.json into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            nodes: 0,
            containers: 0,
            core: 0,
            io: 0,
            ui: 0,
            data: 0,
            wires: 0,
            misc: 0,
        };
        c.wires = t.matches("\"wires\"").count();
        c.misc = t.matches("\"env\"").count()
            + t.matches("\"config\"").count()
            + t.matches("\"credentials\"").count()
            + t.matches("\"outputs\"").count()
            + t.matches("\"props\"").count();
        c.nodes = t.matches("\"id\"").count();
        // Count node types across the whole text.
        for seg in t.split("\"type\"").skip(1) {
            let after = seg.trim_start_matches([':', ' ']);
            let ty = after
                .strip_prefix('"')
                .and_then(|s| s.split('"').next())
                .unwrap_or("");
            if ["tab", "subflow", "group"].contains(&ty) {
                c.containers += 1;
            } else if [
                "inject",
                "debug",
                "function",
                "switch",
                "change",
                "trigger",
                "delay",
                "junction",
                "catch",
                "status",
                "comment",
                "complete",
                "link call",
                "link in",
                "link out",
            ]
            .contains(&ty)
            {
                c.core += 1;
            } else if ty.starts_with("http")
                || ty.starts_with("mqtt")
                || ty.starts_with("tcp")
                || ty.starts_with("udp")
                || ty.starts_with("websocket")
                || ty.starts_with("serial")
                || ty.starts_with("webhook")
                || ty == "server-state-changed"
            {
                c.io += 1;
            } else if ty.starts_with("ui_") || ty == "site" {
                c.ui += 1;
            } else if [
                "file",
                "file in",
                "watch",
                "csv",
                "xml",
                "yaml",
                "html",
                "json",
                "template",
                "exec",
                "tail",
                "fs-ops",
                "moment",
                "base64",
                "rbe",
                "split",
                "join",
                "sort",
                "range",
                "batch",
                "counter",
                "string",
                "array",
                "object",
                "buffer",
                "parser",
                "smooth",
                "aggregate",
                "deduplicate",
                "statistics",
                "average",
                "msg-speed",
                "msg-resend",
                "pause",
                "loop",
                "statemachine",
            ]
            .contains(&ty)
                || TYPES.contains(&ty)
            {
                c.data += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flows() {
        let b = concat!(
            "[\n",
            "  {\"id\":\"t1\",\"type\":\"tab\",\"label\":\"flow\"},\n",
            "  {\"id\":\"g1\",\"type\":\"group\",\"z\":\"t1\",\"name\":\"grp\",\"nodes\":[\"n1\"]},\n",
            "  {\"id\":\"n1\",\"type\":\"inject\",\"z\":\"t1\",\"wires\":[[\"n2\"]]},\n",
            "  {\"id\":\"n2\",\"type\":\"function\",\"z\":\"t1\",\"func\":\"return msg;\",\"wires\":[[\"n3\"]]},\n",
            "  {\"id\":\"n3\",\"type\":\"debug\",\"z\":\"t1\"},\n",
            "  {\"id\":\"n4\",\"type\":\"mqtt in\",\"z\":\"t1\",\"broker\":\"b1\",\"wires\":[[\"n5\"]]},\n",
            "  {\"id\":\"n5\",\"type\":\"http request\",\"z\":\"t1\",\"wires\":[[\"n6\"]]},\n",
            "  {\"id\":\"n6\",\"type\":\"ui_gauge\",\"z\":\"t1\",\"group\":\"g1\"},\n",
            "  {\"id\":\"n7\",\"type\":\"csv\",\"z\":\"t1\",\"wires\":[]},\n",
            "  {\"id\":\"b1\",\"type\":\"mqtt-broker\",\"broker\":\"127.0.0.1\"}\n",
            "]\n",
        );
        let c = Nodered::parse(b.as_bytes()).unwrap();
        assert_eq!(c.nodes, 10);
        assert_eq!(c.containers, 2);
        assert_eq!(c.core, 3);
        assert_eq!(c.io, 3);
        assert_eq!(c.ui, 1);
        assert_eq!(c.data, 1);
        assert_eq!(c.wires, 5);
    }

    #[test]
    fn rejects_other() {
        assert!(Nodered::parse(b"{}").is_none());
    }
}
