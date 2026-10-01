//! Parser for Grafana datasource/dashboard provisioning JSON/YAML.
//!
//! Counts `apiVersion`/`providers:`/`name`/`type`/`datasources:`/`deleteDatasources:`/
//! `foldersFilesFromPath`/`dashboards`/`panels`/`annotations`, common JSON keys
//! (`"type"`/`"url"`/`"access"`/`"basicAuth"`/`"jsonData"`/`"secureJsonData"`/
//! `"uid"`/`"title"`/`"targets"`/`"expr"`/`"panels"`/`"schemaVersion"`).
//!
//! ```
//! let b = b"apiVersion: 1\ndatasources:\n  - name: Prometheus\n    type: prometheus\n    url: http://prom\n";
//! assert!(izanagi_kit::grafana::detect(b));
//! let c = izanagi_kit::grafana::Grafana::parse(b).unwrap();
//! assert_eq!(c.datasources, 1);
//! ```

/// Parsed Grafana provisioning summary.
#[derive(Debug, Clone)]
pub struct Grafana {
    /// `apiVersion` declaration (YAML or JSON).
    pub api_version: usize,
    /// `providers:`/`datasources:`/`deleteDatasources:`/`foldersFilesFromPath`/`annotations`/`dashboards`/`alerting`/`contactPoints`/`policies`/`mutenings`/`templates` entries.
    pub providers: usize,
    /// `datasources:` `- name:` entries.
    pub datasources: usize,
    /// `type` values (`prometheus`/`loki`/`influxdb`/…).
    pub types: usize,
    /// `url`/`access`/`basicAuth`/`basicAuthUser`/`database`/`orgId`/`isDefault`/`version`/`editable`/`apiVersion` keys (datasource body).
    pub datasource_keys: usize,
    /// `jsonData`/`secureJsonData`/`custom`/`tls*`/`es*`/`prometheus*`/`keepCookies` keys.
    pub data_keys: usize,
    /// `uid`/`title`/`folder`/`folderUid`/`tags`/`timezone`/`schemaVersion`/`graphTooltip`/`style`/`templating`/`time`/`annotations`/`links`/`panels` dashboard keys.
    pub dashboard_keys: usize,
    /// `targets`/`expr`/`legendFormat`/`refId`/`datasource`/`format`/`instant`/`interval`/`maxDataPoints`/`alias`/`decimals`/`unit`/`gridPos`/`x`/`y`/`w`/`h`/`id`/`type`/`title`/`datasource`/`fieldConfig`/`options`/`targets`/`transformations`/`transparent`/`timeFrom`/`timeShift`/`repeat`/`datasource` panel/target keys.
    pub panel_keys: usize,
    /// `#`/`//` comment lines.
    pub comments: usize,
}

const PROVIDER_KEYS: &[&str] = &[
    "providers",
    "datasources",
    "deleteDatasources",
    "foldersFilesFromPath",
    "dashboards",
    "alerting",
    "contactPoints",
    "policies",
    "mutenings",
    "templates",
    "plugins",
    "notifiers",
];

const DS_KEYS: &[&str] = &[
    "url",
    "access",
    "basicAuth",
    "basicAuthUser",
    "database",
    "orgId",
    "isDefault",
    "editable",
    "version",
    "typeLogoUrl",
];

const DATA_KEYS: &[&str] = &[
    "jsonData",
    "secureJsonData",
    "custom",
    "tlsAuth",
    "tlsAuthWithCACert",
    "withCredentials",
    "secureJsonFields",
    "keepCookies",
    "esVersion",
    "index",
    "interval",
    "maxConcurrentShardRequests",
    "prometheusType",
    "prometheusVersion",
    "exemplarTraceIdDestinations",
];

const DASH_KEYS: &[&str] = &[
    "uid",
    "title",
    "folder",
    "folderUid",
    "tags",
    "timezone",
    "schemaVersion",
    "graphTooltip",
    "style",
    "templating",
    "time",
    "annotations",
    "links",
    "refresh",
    "editable",
    "fiscalYearStartMonth",
    "weekStart",
];

const PANEL_KEYS: &[&str] = &[
    "targets",
    "expr",
    "legendFormat",
    "refId",
    "datasource",
    "format",
    "instant",
    "interval",
    "maxDataPoints",
    "alias",
    "decimals",
    "unit",
    "gridPos",
    "fieldConfig",
    "options",
    "transformations",
    "transparent",
    "timeFrom",
    "timeShift",
    "repeat",
    "id",
];

fn key(l: &str) -> Option<&str> {
    let tr = l.trim().trim_start_matches('-').trim();
    let mut it = tr.splitn(2, ':');
    let k = it.next()?.trim().trim_matches('"').trim_matches('\'');
    it.next()?;
    Some(k)
}

/// Returns `true` when the bytes look like a Grafana provisioning file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("apiVersion:") && (t.contains("datasources:") || t.contains("providers:")))
        || t.contains("\"datasources\"")
        || t.contains("\"panels\"")
        || t.contains("\"schemaVersion\"")
        || (t.contains("\"type\"") && t.contains("\"gridPos\""))
}

impl Grafana {
    /// Parses a Grafana provisioning file, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            api_version: 0,
            providers: 0,
            datasources: 0,
            types: 0,
            datasource_keys: 0,
            data_keys: 0,
            dashboard_keys: 0,
            panel_keys: 0,
            comments: 0,
        };
        let mut in_ds = false;
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') || tr.starts_with("//") {
                c.comments += 1;
                continue;
            }
            if let Some(k) = key(l) {
                match k {
                    "apiVersion" => c.api_version += 1,
                    _ => {
                        if PROVIDER_KEYS.contains(&k) {
                            c.providers += 1;
                            in_ds = k == "datasources";
                        }
                        if DS_KEYS.contains(&k) {
                            c.datasource_keys += 1;
                        }
                        if DATA_KEYS.contains(&k) {
                            c.data_keys += 1;
                        }
                        if DASH_KEYS.contains(&k) {
                            c.dashboard_keys += 1;
                        }
                        if PANEL_KEYS.contains(&k) {
                            c.panel_keys += 1;
                        }
                        if k == "type" || k == "datasource" && in_ds {
                            c.types += 1;
                        }
                        if k == "name" && in_ds {
                            c.datasources += 1;
                        }
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

    const CONF: &[u8] = b"apiVersion: 1\ndatasources:\n  - name: Prometheus\n    type: prometheus\n    url: http://prom:9090\n    access: proxy\n    basicAuth: false\n    jsonData:\n      httpMethod: POST\n  - name: Loki\n    type: loki\n    url: http://loki:3100\nproviders:\n  - name: dashboards\n    folder: services\n";

    #[test]
    fn parses_grafana() {
        let c = Grafana::parse(CONF).unwrap();
        assert_eq!(c.api_version, 1);
        assert_eq!(c.providers, 2);
        assert_eq!(c.datasources, 2);
        assert_eq!(c.types, 2);
        assert!(c.datasource_keys >= 3);
        assert!(c.data_keys >= 1);
    }

    #[test]
    fn rejects_non_grafana() {
        assert!(!detect(b"version: 3\n"));
        assert!(Grafana::parse(b"x").is_none());
    }
}
