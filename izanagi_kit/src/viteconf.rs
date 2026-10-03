//! Census of a `vite.config.ts` / `vite.config.js`.
//!
//! `export default defineConfig({…})` or plain `export default {…}` with
//! `plugins:` (`react()`/`vue()`/`svelte()`/`vitePlugin*` calls), `resolve`
//! (`alias`), `build` (`outDir`/`sourcemap`/`target`/`lib`/`rollupOptions`/
//! `minify`/`cssCodeSplit`/`manifest`), `server` (`port`/`proxy`/`host`/
//! `strictPort`/`open`/`cors`/`https`), `preview`, `optimizeDeps`
//! (`include`/`exclude`/`entries`), `esbuild`, `css` (`modules`/`preprocessorOptions`),
//! `test` (Vitest: `environment`/`globals`/`coverage`), `define`,
//! `publicDir`, `base`, `root`, `envPrefix`, `logLevel`, `mode`,
//! `worker`, `ssr`, `assetsInclude`.
//!
//! ```rust
//! let v = izanagi_kit::viteconf::ViteConf::parse(
//!     b"import { defineConfig } from 'vite';\nexport default defineConfig({ base: '/app/', build: { outDir: 'dist' } });",
//! ).unwrap();
//! assert_eq!(v.define_config, 1);
//! ```
#![forbid(unsafe_code)]

/// vite.config census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViteConf {
    /// `defineConfig(` wrappers.
    pub define_config: usize,
    /// Plugin call sites in `plugins: [` (`name(`).
    pub plugins: usize,
    /// `build:`/`server:`/`resolve:`/`test:`/`css:`/`optimizeDeps:`/`ssr:`/`worker:`
    /// section keys.
    pub sections: usize,
    /// `key:` keys total.
    pub keys: usize,
    /// `proxy:` entries (`'/path':` or `"/path":`).
    pub proxies: usize,
    /// `alias` entries (`'name':` inside `resolve.alias` or `find:` items).
    pub aliases: usize,
}

/// Section keys counted in `sections`.
const SECTION_KEYS: &[&str] = &[
    "build",
    "server",
    "resolve",
    "test",
    "css",
    "optimizeDeps",
    "ssr",
    "worker",
    "esbuild",
    "preview",
];

/// True if `b` looks like a vite config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("defineConfig") || t.contains("export default"))
        && (t.contains("vite")
            || t.contains("plugins")
            || (t.contains("server") && t.contains("build")))
}

impl ViteConf {
    /// Count the census fields; `None` unless [`detect`] holds.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            define_config: 0,
            plugins: 0,
            sections: 0,
            keys: 0,
            proxies: 0,
            aliases: 0,
        };
        c.define_config = t
            .lines()
            .filter(|l| !l.trim_start().starts_with("import "))
            .map(|l| l.matches("defineConfig").count())
            .sum();
        let mut in_plugins = false;
        let mut in_alias = false;
        let mut in_proxy = false;
        for raw in t.lines() {
            let l = raw.trim();
            if l.is_empty() || l.starts_with("//") || l.starts_with("import ") {
                continue;
            }
            if l.contains("plugins") && l.contains('[') && !l.contains(']') {
                in_plugins = true;
            }
            if l.contains("alias") && (l.contains('{') || l.contains('[')) {
                in_alias = true;
            }
            if l.contains("proxy") && l.contains('{') {
                in_proxy = true;
            }
            if l.starts_with(']') || l.starts_with('}') {
                in_plugins = false;
                if l.starts_with('}') {
                    in_alias = false;
                    in_proxy = false;
                }
            }
            if in_plugins || (l.contains("plugins") && l.contains('[')) {
                for part in l.split(',') {
                    let s = part.trim();
                    if let Some(p) = s.find('(') {
                        let raw = &s[..p];
                        let name = raw.rsplit([':', '[']).next().unwrap_or(raw);
                        if !name.is_empty()
                            && name
                                .chars()
                                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.')
                            && name
                                .chars()
                                .next()
                                .is_some_and(|ch| ch.is_ascii_lowercase())
                            && !name.contains(':')
                        {
                            c.plugins += 1;
                        }
                    }
                }
                if !l.contains(':') {
                    continue;
                }
            }
            for seg in l.split(',') {
                let s = seg.trim();
                if let Some(colon) = s.find(':') {
                    if in_proxy && s.starts_with(['\'', '"']) {
                        c.proxies += 1;
                        continue;
                    }
                    if in_alias && s.starts_with(['\'', '"']) {
                        c.aliases += 1;
                        continue;
                    }
                    let key = s[..colon]
                        .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .next()
                        .unwrap_or("");
                    if key.is_empty() {
                        continue;
                    }
                    if key
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                    {
                        c.keys += 1;
                        if SECTION_KEYS.contains(&key) {
                            c.sections += 1;
                        }
                        if in_alias || key == "alias" || key == "find" {
                            c.aliases += 1;
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

    const SAMPLE: &[u8] = b"import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  base: '/app/',
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      '@components': path.resolve(__dirname, './src/components'),
    },
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
    target: 'es2020',
  },
  server: {
    port: 3000,
    proxy: {
      '/api': 'http://localhost:8080',
      '/ws': { target: 'ws://localhost:8080' },
    },
  },
});";

    #[test]
    fn detects_config() {
        assert!(detect(SAMPLE));
        assert!(!detect(b"const x = 1;"));
    }

    #[test]
    fn parses() {
        let c = ViteConf::parse(SAMPLE).unwrap();
        assert_eq!(c.define_config, 1);
        assert!(c.plugins >= 1);
        assert!(c.sections >= 3);
        assert!(c.keys >= 6);
        assert!(c.aliases >= 2);
    }

    #[test]
    fn rejects() {
        assert!(ViteConf::parse(b"").is_none());
        assert!(ViteConf::parse(b"export default {};").is_none());
    }
}
