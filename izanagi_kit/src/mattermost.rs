//! Mattermost `config.json` — `"*Settings": { … }` PascalCase セクション
//! を持つ JSON 設定。
//!
//! ```
//! let cfg = b"{\n    \"ServiceSettings\": {\n        \"SiteURL\": \"\",\n        \"ListenAddress\": \":8065\"\n    },\n    \"TeamSettings\": {\n        \"SiteName\": \"Mattermost\"\n    },\n    \"SqlSettings\": {\n        \"DriverName\": \"postgres\",\n        \"DataSource\": \"mmuser:pw@tcp(x:3306)/mm\"\n    }\n}\n";
//! assert!(izanagi_kit::mattermost::detect(cfg));
//! let c = izanagi_kit::mattermost::parse(cfg).unwrap();
//! assert_eq!(c.known_sections, 3);
//! ```

/// Mattermost config.json の既知セクション名。
const KNOWN_SECTIONS: &[&str] = &[
    "ServiceSettings",
    "TeamSettings",
    "SqlSettings",
    "EmailSettings",
    "LogSettings",
    "PasswordSettings",
    "FileSettings",
    "RateLimitSettings",
    "PrivacySettings",
    "SupportSettings",
    "AnnouncementSettings",
    "ThemeSettings",
    "GitLabSettings",
    "GoogleSettings",
    "Office365Settings",
    "LdapSettings",
    "ComplianceSettings",
    "LocalizationSettings",
    "SamlSettings",
    "NativeAppSettings",
    "ClusterSettings",
    "MetricsSettings",
    "AnalyticsSettings",
    "ClientRequirements",
    "PluginSettings",
    "ImageProxySettings",
    "BleveSettings",
    "ExperimentalSettings",
    "DisplaySettings",
    "GuestAccountsSettings",
    "MessageExportSettings",
    "WebrtcSettings",
    "ElasticsearchSettings",
    "DataRetentionSettings",
    "JobSettings",
    "CloudSettings",
    "PushNotificationSettings",
    "FeatureFlags",
];

/// 走査結果の集計。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 物理行数(空行除く)。
    pub lines: usize,
    /// `"*Settings":`/既知セクションの `"Name":` 数。
    pub sections: usize,
    /// 既知名のセクション数。
    pub known_sections: usize,
    /// `"key":` エントリ数。
    pub entries: usize,
    /// `true`/`false` 値のエントリ数。
    pub bool_values: usize,
}

/// `"key"` トークンを取る (文字列リテラル先頭部のみ単純走査)。
fn first_quoted(s: &str) -> Option<&str> {
    let start = s.find('"')? + 1;
    let end = s[start..].find('"')? + start;
    Some(&s[start..end])
}

/// Mattermost config.json らしい構造かを判定する。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Some(c) = parse(b) else {
        return false;
    };
    c.known_sections >= 2 && c.entries >= 6
}

/// 行を走査して集計する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    if !b.is_ascii() {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut c = Counts {
        lines: 0,
        sections: 0,
        known_sections: 0,
        entries: 0,
        bool_values: 0,
    };
    let mut depth: i64 = 0;
    for raw in s.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        c.lines += 1;
        if let Some((key_tok, rest)) = t.split_once(':') {
            if let Some(key) = first_quoted(key_tok) {
                let key_ok = !key.is_empty()
                    && key
                        .bytes()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'.');
                if key_ok {
                    c.entries += 1;
                    // PascalCase `*Settings`/既知セクション = 深度1のオブジェクトキー。
                    if depth <= 1 && KNOWN_SECTIONS.contains(&key) {
                        c.sections += 1;
                        c.known_sections += 1;
                    }
                    let v = rest.trim_start();
                    if v.starts_with("true") || v.starts_with("false") {
                        c.bool_values += 1;
                    }
                }
            }
        }
        for ch in t.bytes() {
            match ch {
                b'{' | b'[' => depth += 1,
                b'}' | b']' => depth -= 1,
                _ => {}
            }
        }
    }
    if c.entries == 0 {
        return None;
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"{\n    \"ServiceSettings\": {\n        \"SiteURL\": \"\",\n        \"ListenAddress\": \":8065\",\n        \"EnableBotAccountCreation\": true\n    },\n    \"TeamSettings\": {\n        \"SiteName\": \"Mattermost\",\n        \"MaxUsersPerTeam\": 50\n    },\n    \"SqlSettings\": {\n        \"DriverName\": \"postgres\",\n        \"DataSource\": \"mmuser:pw@tcp(x:3306)/mm\",\n        \"MaxIdleConns\": 20\n    },\n    \"EmailSettings\": {\n        \"SendEmailNotifications\": true\n    }\n}\n";

    #[test]
    fn detects_mattermost() {
        assert!(detect(SAMPLE));
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.known_sections, 4);
        assert_eq!(c.entries, 13);
        assert_eq!(c.bool_values, 2);
    }

    #[test]
    fn rejects_other_json() {
        assert!(!detect(b"{\"name\": \"x\", \"version\": \"1\"}\n"));
        assert!(!detect(b"{\n  \"a\": {\n    \"b\": 1\n  }\n}\n"));
    }
}
