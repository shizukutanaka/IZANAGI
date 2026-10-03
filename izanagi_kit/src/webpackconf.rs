//! Census of a webpack `webpack.config.js` (or `webpackfile.js`).
//!
//! `module.exports = {…}` / `module.exports = (env) => ({…})` / `export default
//! {…}` config objects. Key fields: `entry`, `output`, `mode`,
//! `devtool`, `resolve`, `optimization`, `devServer`, `plugins`,
//! `watch`, `target`, `stats`, `experiments`, `externals`, `cache`,
//! `performance`, `context`, `bail`, `dependencies`, `infrastructureLogging`,
//! `loader`, `name`, `node`, `parallelism`, `profile`, `recordsPath`,
//! `snapshot`. `module.rules` entries carry `test:`/`use:`/`loader:`.
//!
//! ```rust
//! let w = izanagi_kit::webpackconf::WebpackConf::parse(
//!     b"module.exports = { mode: 'development', entry: './src/index.js', output: { filename: 'bundle.js' } };",
//! ).unwrap();
//! assert_eq!(w.mode, 1);
//! ```
#![forbid(unsafe_code)]

/// webpack.config.js census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebpackConf {
    /// `mode:` entries.
    pub mode: usize,
    /// `module.rules` / `module: { rules: [` entries (`test:`).
    pub rules: usize,
    /// `plugins:` array items / `new XxxPlugin()` call sites.
    pub plugins: usize,
    /// Top-level and other config keys (`key:`).
    pub keys: usize,
    /// `devtool:` values.
    pub devtool: usize,
    /// `env`/argv function configs (`=>`/`function (env…)`).
    pub function_configs: usize,
}

/// Well-known webpack config keys.
const KEYS: &[&str] = &[
    "entry",
    "output",
    "mode",
    "devtool",
    "resolve",
    "optimization",
    "devServer",
    "plugins",
    "watch",
    "target",
    "stats",
    "experiments",
    "externals",
    "cache",
    "performance",
    "context",
    "bail",
    "dependencies",
    "infrastructureLogging",
    "loader",
    "name",
    "node",
    "parallelism",
    "profile",
    "recordsPath",
    "snapshot",
    "module",
];

/// True if `b` looks like a webpack config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let exports = t.contains("module.exports") || t.contains("export default");
    let webpackish = t.contains("devtool")
        || (t.contains("mode")
            && (t.contains("output") || t.contains("entry") || t.contains("=>")))
        || t.contains("webpack");
    exports && webpackish
}

impl WebpackConf {
    /// Count the census fields; `None` unless [`detect`] holds.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            mode: 0,
            rules: 0,
            plugins: 0,
            keys: 0,
            devtool: 0,
            function_configs: 0,
        };
        if t.contains("=>") || t.contains("function (env") || t.contains("function(env") {
            c.function_configs += 1;
        }
        let mut in_rules = false;
        let mut in_plugins = false;
        for raw in t.lines() {
            let l = raw.trim();
            if l.is_empty() {
                continue;
            }
            if l.contains("rules") && (l.contains('[') || l.ends_with(':')) {
                in_rules = true;
            }
            if l.contains("plugins") && l.contains('[') {
                in_plugins = true;
            }
            if l.starts_with(']') {
                in_rules = false;
                in_plugins = false;
            }
            // `new XxxPlugin(` / `new webpack.` sites.
            if l.starts_with("new ") {
                c.plugins += 1;
            }
            for (i, seg) in l.split(',').enumerate() {
                let s = seg.trim();
                if let Some(colon) = s.find(':') {
                    let key = s[..colon]
                        .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .next()
                        .unwrap_or("");
                    if !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    {
                        c.keys += 1;
                        if key == "mode" {
                            c.mode += 1;
                        }
                        if key == "devtool" {
                            c.devtool += 1;
                        }
                        if (in_rules && key == "test") || (i == 0 && s.starts_with("test:")) {
                            c.rules += 1;
                        }
                    }
                }
            }
            if in_plugins && (l.starts_with("new ") || l.contains("Plugin(")) {
                continue;
            }
        }
        let _ = KEYS;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"const path = require('path');
module.exports = {
  mode: 'production',
  entry: './src/index.js',
  output: { filename: '[name].js', path: path.resolve(__dirname, 'dist') },
  devtool: 'source-map',
  module: {
    rules: [
      { test: /\\.js$/, use: 'babel-loader' },
      { test: /\\.css$/, use: ['style-loader', 'css-loader'] },
    ],
  },
  plugins: [
    new HtmlWebpackPlugin({ title: 'app' }),
    new webpack.HotModuleReplacementPlugin(),
  ],
  devServer: { port: 8080 },
};";

    #[test]
    fn detects_config() {
        assert!(detect(SAMPLE));
        assert!(!detect(b"const x = 1;"));
    }

    #[test]
    fn parses() {
        let c = WebpackConf::parse(SAMPLE).unwrap();
        assert_eq!(c.mode, 1);
        assert_eq!(c.devtool, 1);
        assert!(c.rules >= 2);
        assert!(c.plugins >= 2);
        assert!(c.keys >= 8);
    }

    #[test]
    fn function_config_detected() {
        assert_eq!(
            WebpackConf::parse(b"module.exports = (env) => ({ mode: env.production ? 'production' : 'development' });")
                .unwrap()
                .function_configs,
            1
        );
    }

    #[test]
    fn rejects() {
        assert!(WebpackConf::parse(b"").is_none());
        assert!(WebpackConf::parse(b"hello").is_none());
    }
}
