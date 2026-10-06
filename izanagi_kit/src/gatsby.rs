//! Gatsby `gatsby-config.*` census.
//!
//! `module.exports = { siteMetadata: { title, siteUrl, description,
//! author, twitterUsername, image }, plugins: [
//! 'gatsby-plugin-x' or { resolve: 'gatsby-plugin-x', options: {...} }
//! 'gatsby-source-filesystem', 'gatsby-transformer-*' (remark,
//! sharp, json, yaml, toml, csv, sqip, javascript-frontmatter),
//! 'gatsby-plugin-*' (image, sharp, manifest, sitemap, robots-txt,
//! mdx, react-helmet, offline, pwa, emition, styled-components,
//! sass, postcss, typescript, feed, disqus, netlify, netlify-cms,
//! vercel, segment, google-analytics, google-tagmanager, sentry,
//! sentry-io, algolia, lunr, favicon, coveralls, prune, subfont,
//! prefetching-google-fonts, remove-fingerprints, cli, schema,
//! page-creator, workers, graphql, node-api, data, browser-api,
//! adapter, no-sourcemaps, remove-serviceworker, fast-refresh,
//! script-loader, loadable-components, image-cdn, cloudinary,
//! contentful, shopify, wordpress, drupal, sanity, strapi, airtable,
//! firestore, cockpit, ghost, supabase, firebase) ] }`
//! `siteMetadata` + `gatsby-plugin-*`/`gatsby-source-*`/
//! `gatsby-transformer-*`/`resolve:` plugin entries are exclusive.
//!
//! ```rust
//! let k = b"module.exports = {\n  siteMetadata: { title: 'X', siteUrl: 'https://x' },\n  plugins: [\n    'gatsby-plugin-image',\n    { resolve: 'gatsby-source-filesystem', options: { name: 'x', path: 'src' } },\n    'gatsby-transformer-sharp',\n  ],\n};\n";
//! assert!(izanagi_kit::gatsby::detect(k));
//! ```

/// gatsby-config census.
#[derive(Debug, Clone)]
pub struct Gatsby {
    /// `key:`/`key =` config lines.
    pub settings: usize,
    /// `gatsby-*` plugin entries.
    pub plugins: usize,
    /// `//`/`/*` comment lines.
    pub comments: usize,
}

const MARKERS: &[&str] = &[
    "siteMetadata",
    "gatsby-plugin-",
    "gatsby-source-",
    "gatsby-transformer-",
    "gatsby-node",
    "gatsby-browser",
    "gatsby-ssr",
    "resolve:",
    "siteUrl",
    "twitterUsername",
    "gatsby-plugin-image",
    "gatsby-plugin-sharp",
    "gatsby-plugin-manifest",
    "gatsby-plugin-sitemap",
    "gatsby-plugin-mdx",
    "gatsby-plugin-offline",
    "gatsby-plugin-react-helmet",
    "gatsby-plugin-styled-components",
    "gatsby-plugin-sass",
    "gatsby-plugin-typescript",
    "gatsby-plugin-feed",
    "gatsby-source-filesystem",
    "gatsby-source-contentful",
    "gatsby-source-wordpress",
    "gatsby-source-shopify",
    "gatsby-source-drupal",
    "gatsby-source-sanity",
    "gatsby-source-strapi",
    "gatsby-source-airtable",
    "gatsby-source-firestore",
    "gatsby-source-graphql",
    "gatsby-transformer-remark",
    "gatsby-transformer-sharp",
    "gatsby-transformer-json",
    "gatsby-transformer-yaml",
    "gatsby-transformer-csv",
    "gatsby-transformer-sqip",
    "gatsby-plugin-image-cdn",
    "gatsby-plugin-favicon",
    "gatsby-plugin-page-creator",
    "gatsby-plugin-google-analytics",
    "gatsby-plugin-segment",
    "gatsby-plugin-sentry",
    "gatsby-plugin-algolia",
    "gatsby-plugin-lunr",
];

fn code(line: &str) -> bool {
    let s = line.trim();
    !s.is_empty()
        && !s.starts_with("//")
        && !s.starts_with('#')
        && !s.starts_with("/*")
        && !s.starts_with('*')
}

/// Detect a `gatsby-config.*` file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    // `siteMetadata` + `gatsby-plugin-*`/`gatsby-source-*`/
    // `gatsby-transformer-*` are gatsby-exclusive.
    let mut n = 0usize;
    for line in t.lines() {
        if code(line) && MARKERS.iter().any(|m| line.contains(m)) {
            n += 1;
        }
    }
    n >= 2
}

impl Gatsby {
    /// Census a buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            settings: 0,
            plugins: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("//") || s.starts_with("/*") || s.starts_with('*') {
                c.comments += 1;
                continue;
            }
            if s.contains(':') || s.contains('=') {
                c.settings += 1;
            }
            if s.contains("gatsby-") {
                c.plugins += 1;
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects() {
        let b = b"module.exports = {\n  siteMetadata: { title: 'X', siteUrl: 'https://x' },\n  plugins: [\n    'gatsby-plugin-image',\n    { resolve: 'gatsby-source-filesystem', options: { name: 'x', path: 'src' } },\n    'gatsby-transformer-sharp',\n  ],\n};\n";
        assert!(detect(b));
        let c = Gatsby::parse(b).unwrap();
        assert!(c.plugins >= 3);
    }

    #[test]
    fn rejects_others() {
        assert!(!detect(b"module.exports = { plugins: [] };\n"));
        assert!(!detect(
            b"// siteMetadata: {}\n// 'gatsby-plugin-x'\nplugins: []\n"
        ));
    }
}
