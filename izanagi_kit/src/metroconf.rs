//! Census of a Metro `metro.config.js` (React Native bundler config).
//!
//! `module.exports = {…}` / `mergeConfig(getDefaultConfig(__dirname), {…})`
//! with `resolver` (`assetExts`/`sourceExts`/`resolverMainFields`/
//! `resolveRequest`/`blockList`/`extraNodeModules`/`platforms`/
//! `unstable_enablePackageExports`/`unstable_conditionNames`), `transformer`
//! (`babelTransformerPath`/`minifierPath`/`minifierConfig`/`assetPlugins`/
//! `getTransformOptions`/`unstable_allowRequireContext`), `serializer`
//! (`getModulesRunBeforeMainModule`/`createModuleIdFactory`/`getRunModuleStatement`/
//! `processModuleFilter`), `server` (`port`/`enhanceMiddleware`/`unstable_serverRoot`),
//! `symbolicator`, `watchFolders`, `maxWorkers`, `cacheStores`, `reporter`,
//! `watcher` (`watchman`/`healthCheck`), `resetCache`.
//!
//! ```rust
//! let m = izanagi_kit::metroconf::MetroConf::parse(
//!     b"const {getDefaultConfig, mergeConfig} = require('@react-native/metro-config');\nmodule.exports = mergeConfig(getDefaultConfig(__dirname), { resolver: { assetExts: ['bin'] } });",
//! ).unwrap();
//! assert!(m.sections >= 1);
//! ```
#![forbid(unsafe_code)]

/// metro.config.js census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetroConf {
    /// Section objects: `resolver`/`transformer`/`serializer`/`server`/
    /// `symbolicator`/`watcher`/`project`/`reporter`.
    pub sections: usize,
    /// `key:` keys total.
    pub keys: usize,
    /// `mergeConfig(`/`getDefaultConfig(` sites.
    pub helpers: usize,
    /// Function-valued options (`X:` followed by `(`/`function`/`=>`).
    pub functions: usize,
    /// `watchFolders` entries / array items in list-valued keys.
    pub list_items: usize,
    /// `require(`/`import` lines.
    pub requires: usize,
}

/// Section keys counted in `sections`.
const SECTION_KEYS: &[&str] = &[
    "resolver",
    "transformer",
    "serializer",
    "server",
    "symbolicator",
    "watcher",
    "project",
    "reporter",
];

/// True if `b` looks like a metro config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("module.exports") || t.contains("export default"))
        && (t.contains("resolver")
            || t.contains("transformer")
            || t.contains("metro")
            || t.contains("assetExts")
            || t.contains("sourceExts"))
}

impl MetroConf {
    /// Count the census fields; `None` unless [`detect`] holds.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            keys: 0,
            helpers: 0,
            functions: 0,
            list_items: 0,
            requires: 0,
        };
        c.helpers = t.matches("getDefaultConfig").count() + t.matches("mergeConfig").count();
        for raw in t.lines() {
            let l = raw.trim();
            if l.is_empty() || l.starts_with("//") || l.starts_with('*') {
                continue;
            }
            if l.starts_with("require(") || l.contains("= require(") || l.starts_with("import ") {
                c.requires += 1;
            }
            for seg in l.split(',') {
                let s = seg.trim();
                if let Some(colon) = s.find(':') {
                    let key = s[..colon]
                        .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .next()
                        .unwrap_or("");
                    if key.is_empty()
                        || !key
                            .chars()
                            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                    {
                        continue;
                    }
                    c.keys += 1;
                    if SECTION_KEYS.contains(&key) {
                        c.sections += 1;
                    }
                    let rest = s[colon + 1..].trim_start();
                    if rest.starts_with('(') || rest.starts_with("function") || rest.contains("=>")
                    {
                        c.functions += 1;
                    }
                } else if s.starts_with('\'') || s.starts_with('"') {
                    // bare array item
                    c.list_items += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] =
        b"const {getDefaultConfig, mergeConfig} = require('@react-native/metro-config');
const path = require('path');

const config = {
  projectRoot: path.resolve(__dirname),
  watchFolders: [path.resolve(__dirname, '../shared')],
  resolver: {
    assetExts: ['png', 'jpg', 'bin'],
    sourceExts: ['js', 'jsx', 'ts', 'tsx', 'json'],
    resolverMainFields: ['react-native', 'browser', 'main'],
    resolveRequest: (context, moduleName, platform) => null,
  },
  transformer: {
    babelTransformerPath: require.resolve('react-native-svg-transformer'),
    getTransformOptions: async () => ({ transform: { inlineRequires: true } }),
  },
  server: {
    port: 8081,
  },
  maxWorkers: 4,
};

module.exports = mergeConfig(getDefaultConfig(__dirname), config);";

    #[test]
    fn detects_config() {
        assert!(detect(SAMPLE));
        assert!(!detect(b"const x = 1;"));
    }

    #[test]
    fn parses() {
        let c = MetroConf::parse(SAMPLE).unwrap();
        assert!(c.sections >= 3);
        assert!(c.helpers >= 2);
        assert!(c.keys >= 8);
        assert!(c.functions >= 2);
        assert!(c.requires >= 2);
        assert!(c.list_items >= 5);
    }

    #[test]
    fn rejects() {
        assert!(MetroConf::parse(b"").is_none());
        assert!(MetroConf::parse(b"module.exports = {};").is_none());
    }
}
