//! Census of a `rollup.config.js` / `rollup.config.mjs`.
//!
//! `export default {…}` (or array of configs) with `input`, `output`
//! (`file:`/`dir:`/`format:`/`name:`/`globals:`/`sourcemap:`), `plugins:`
//! (`resolve()`/`commonjs()`/`babel()`/`terser()`/`replace()`/`json()`/
//! `typescript()`/`nodeResolve()`/…), `external`, `treeshake`, `watch`,
//! `onwarn`, `context`, `preserveModules`, `manualChunks`.
//!
//! ```rust
//! let r = izanagi_kit::rollup::Rollup::parse(
//!     b"export default { input: 'src/main.js', output: { file: 'dist/bundle.js', format: 'es' } };",
//! ).unwrap();
//! assert_eq!(r.inputs, 1);
//! ```
#![forbid(unsafe_code)]

/// rollup.config.js census.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rollup {
    /// `input:` entries.
    pub inputs: usize,
    /// `output:` blocks.
    pub outputs: usize,
    /// `format:` values (es/cjs/umd/iife/amd/system).
    pub formats: usize,
    /// Plugin call sites in `plugins: [` (`name(` entries).
    pub plugins: usize,
    /// `external:` entries.
    pub external: usize,
    /// Other `key:` keys.
    pub keys: usize,
    /// Array-of-configs entries (top-level `[`).
    pub configs: usize,
}

/// True if `b` looks like a rollup config.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("export default") || t.contains("module.exports"))
        && (t.contains("input") || t.contains("output"))
        && (t.contains("format")
            || t.contains("plugins")
            || t.contains("external")
            || t.contains("rollup"))
}

impl Rollup {
    /// Count the census fields; `None` unless [`detect`] holds.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            inputs: 0,
            outputs: 0,
            formats: 0,
            plugins: 0,
            external: 0,
            keys: 0,
            configs: 0,
        };
        let mut in_plugins = false;
        let mut in_output = false;
        for raw in t.lines() {
            let l = raw.trim();
            if l.is_empty() {
                continue;
            }
            if l.starts_with("export default [") || l == "[" && c.configs == 0 {
                // will count configs below via `input:` occurrences
            }
            if l.contains("plugins") && l.contains('[') {
                in_plugins = true;
            }
            if l.contains("output") && (l.contains('{') || l.contains('[')) {
                in_output = true;
                c.outputs += 1;
            }
            if l.starts_with(']') {
                in_plugins = false;
                in_output = false;
            }
            if in_plugins {
                // plugin factory call `xxx(`
                for part in l.split(',') {
                    let s = part.trim();
                    if let Some(p) = s.find('(') {
                        let name = &s[..p];
                        if !name.is_empty()
                            && name
                                .chars()
                                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.')
                            && name
                                .chars()
                                .next()
                                .is_some_and(|ch| ch.is_ascii_lowercase())
                        {
                            c.plugins += 1;
                        }
                    }
                }
                continue;
            }
            for seg in l.split(',') {
                let s = seg.trim();
                if let Some(colon) = s.find(':') {
                    let key = s[..colon]
                        .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .next()
                        .unwrap_or("");
                    if !key.is_empty()
                        && key
                            .chars()
                            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                    {
                        c.keys += 1;
                        match key {
                            "input" => {
                                c.inputs += 1;
                                c.configs += 1;
                            }
                            "external" => c.external += 1,
                            _ => {}
                        }
                    }
                }
            }
            if in_output {
                c.formats += l.matches("format:").count();
                if l.starts_with('{') {
                    continue;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"import resolve from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';
import terser from '@rollup/plugin-terser';

export default {
  input: 'src/main.js',
  output: {
    file: 'dist/bundle.js',
    format: 'es',
    sourcemap: true,
  },
  external: ['react', 'react-dom'],
  plugins: [
    resolve(),
    commonjs(),
    terser(),
  ],
  treeshake: true,
};";

    #[test]
    fn detects_config() {
        assert!(detect(SAMPLE));
        assert!(!detect(b"export default 42;"));
    }

    #[test]
    fn parses() {
        let c = Rollup::parse(SAMPLE).unwrap();
        assert_eq!(c.inputs, 1);
        assert_eq!(c.formats, 1);
        assert!(c.plugins >= 3);
        assert_eq!(c.external, 1);
        assert!(c.keys >= 5);
    }

    #[test]
    fn array_of_configs() {
        let c = Rollup::parse(
            b"export default [\n{ input: 'a.js', output: { format: 'es' } },\n{ input: 'b.js', output: { format: 'cjs' } },\n];",
        )
        .unwrap();
        assert_eq!(c.inputs, 2);
        assert_eq!(c.formats, 2);
    }

    #[test]
    fn rejects() {
        assert!(Rollup::parse(b"").is_none());
        assert!(Rollup::parse(b"no config here").is_none());
    }
}
