//! EasyEffects/PulseEffects preset (`*.json`) parser.
//!
//! Detects EasyEffects presets by their `"input"`/`"output"` device blocks
//! containing `"plugins"` maps of `"bassenhancer#0"`/`"compressor#0"`/
//! `"convolver#0"`/`"crystalizer#0"`/`"deesser#0"`/`"echocanceller#0"`/
//! `"exciter#0"`/`"filter#0"`/`"gate#0"`/`"limiter#0"`/`"loudness#0"`/
//! `"multibandcompressor#0"`/`"multibandgate#0"`/`"pitch#0"`/`"reverb#0"`/
//! `"rnnoise#0"`/`"speex#0"`/`"stereotools#0"`/`"equalizer#0"`/
//! `"autogain#0"`/`"bassloudness#0"`/`"crossfeed#0"`/`"delay#0"`/
//! `"maximizer#0"`/`"pipe.#*"-style plugin ids plus their parameter keys
//! (`"input-gain"`/`"output-gain"`/`"threshold"`/`"ratio"`/`"attack"`/
//! `"release"`/`"makeup"`/`"knee"`/`"mix"`/`"bypass"`/`"input-device"`/
//! `"output-device"`/`"sampling-rate"`/`"block-size"`/`"ir-width"`/
//! `"model-name"`/`"level"`/`"depth"`/`"harmonics"`/`"speed"`/`"blend"`/
//! `"retro"`/`"room"`/`"pre-delay"`/`"decay"`/`"size"`/`"damping"`/
//! `"spread"`/`"reduction"`/`"hz"`/`"band#*"-style band params).
//!
//! ```
//! let b = b"{\n  \"output\": {\n    \"blocklist\": [],\n    \"plugins_order\": [\"compressor#0\", \"limiter#0\"],\n    \"compressor#0\": {\n      \"attack\": 20.0, \"ratio\": 4.0, \"threshold\": -18.0\n    }\n  }\n}\n";
//! assert!(izanagi_kit::easyeffects::detect(b));
//! let c = izanagi_kit::easyeffects::Eepreset::parse(b).unwrap();
//! assert!(c.keys >= 5);
//! ```

/// Parsed EasyEffects preset summary.
#[derive(Debug, Clone)]
pub struct Eepreset {
    /// Recognized key occurrences.
    pub keys: usize,
    /// Top-level/device keys (`"input"`/`"output"`/`"plugins"`/`"plugins_order"`/`"blocklist"`/`"plugin"`/`"bypass"`/`"input-device"`/`"output-device"`).
    pub top_keys: usize,
    /// Plugin ids (`"bassenhancer#"`/`"compressor#"`/`"convolver#"`/`"crystalizer#"`/`"deesser#"`/`"echocanceller#"`/`"exciter#"`/`"filter#"`/`"gate#"`/`"limiter#"`/`"loudness#"`/`"multibandcompressor#"`/`"multibandgate#"`/`"pitch#"`/`"reverb#"`/`"rnnoise#"`/`"speex#"`/`"stereotools#"`/`"equalizer#"`/`"autogain#"`/`"bassloudness#"`/`"crossfeed#"`/`"delay#"`/`"maximizer#"`/`"lv2#"`/`"plugin#"`).
    pub plugin_keys: usize,
    /// Parameter keys (`"input-gain"`/`"output-gain"`/`"threshold"`/`"ratio"`/`"attack"`/`"release"`/`"makeup"`/`"knee"`/`"sidechain"`/`"dry"`/`"wet"`/`"stereo"`/`"preamp"`/`"lookahead"`/`"mode"`/`"slope"`/`"frequency"`/`"gain"`/`"q-factor"`/`"oversample"`).
    pub param_keys: usize,
    /// `#`/`//` comment lines.
    pub comments: usize,
}

/// Top/device keys.
const TOP_KEYS: &[&str] = &[
    "\"input\"",
    "\"output\"",
    "\"plugins\"",
    "\"plugins_order\"",
    "\"blocklist\"",
    "\"plugin\"",
    "\"bypass\"",
    "\"input-device\"",
    "\"output-device\"",
];

/// Plugin ids.
const PLUGIN_KEYS: &[&str] = &[
    "\"bassenhancer#",
    "\"compressor#",
    "\"convolver#",
    "\"crystalizer#",
    "\"deesser#",
    "\"echocanceller#",
    "\"exciter#",
    "\"filter#",
    "\"gate#",
    "\"limiter#",
    "\"loudness#",
    "\"multibandcompressor#",
    "\"multibandgate#",
    "\"pitch#",
    "\"reverb#",
    "\"rnnoise#",
    "\"speex#",
    "\"stereotools#",
    "\"equalizer#",
    "\"autogain#",
    "\"bassloudness#",
    "\"crossfeed#",
    "\"delay#",
    "\"maximizer#",
    "\"lv2#",
];

/// Parameter keys.
const PARAM_KEYS: &[&str] = &[
    "\"input-gain\"",
    "\"output-gain\"",
    "\"threshold\"",
    "\"ratio\"",
    "\"attack\"",
    "\"release\"",
    "\"makeup\"",
    "\"knee\"",
    "\"mix\"",
    "\"sampling-rate\"",
    "\"block-size\"",
    "\"ir-width\"",
    "\"model-name\"",
    "\"level\"",
    "\"depth\"",
    "\"harmonics\"",
    "\"speed\"",
    "\"blend\"",
    "\"retro\"",
    "\"room\"",
    "\"pre-delay\"",
    "\"decay\"",
    "\"size\"",
    "\"damping\"",
    "\"spread\"",
    "\"reduction\"",
    "\"sidechain\"",
    "\"dry\"",
    "\"wet\"",
    "\"post-gain\"",
    "\"pre-gain\"",
    "\"split\"",
    "\"solo\"",
    "\"mute\"",
    "\"stereo\"",
    "\"link\"",
    "\"linked\"",
    "\"left\"",
    "\"right\"",
    "\"low\"",
    "\"mid\"",
    "\"high\"",
    "\"frequency\"",
    "\"q\"",
    "\"width\"",
    "\"amount\"",
    "\"saturation\"",
    "\"drive\"",
    "\"preamp\"",
    "\"floor\"",
    "\"gate\"",
    "\"limiter\"",
    "\"lookahead\"",
    "\"hold\"",
    "\"range\"",
    "\"oversample\"",
    "\"attack-rate\"",
    "\"release-rate\"",
    "\"window\"",
    "\"autogain\"",
    "\"filters\"",
    "\"q-factor\"",
    "\"mode\"",
    "\"slope\"",
    "\"cut\"",
    "\"gain\"",
    "\"resonance\"",
    "\"crossover\"",
];

/// Anchor tokens.
const ANCHORS: &[&str] = &["#0", "\"plugins\"", "\"output\""];

/// Distinctive keys for detection.
const ALL: &[&str] = &[
    "\"plugins_order\"",
    "\"compressor#",
    "\"limiter#",
    "\"equalizer#",
    "\"reverb#",
    "\"output\"",
    "\"blocklist\"",
];

fn key_present(t: &str, k: &str) -> bool {
    t.contains(k)
}

/// Detect an EasyEffects preset.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let hits = ALL.iter().filter(|k| key_present(t, k)).count();
    let anchor = ANCHORS.iter().any(|a| t.contains(a));
    hits >= 2 && anchor
}

impl Eepreset {
    /// Count categories in a preset. Returns `None` when the input does
    /// not look like one.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            keys: 0,
            top_keys: 0,
            plugin_keys: 0,
            param_keys: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.starts_with('#') || tr.starts_with("//") {
                c.comments += 1;
            }
        }
        for k in TOP_KEYS {
            c.top_keys += t.matches(k).count();
        }
        for k in PLUGIN_KEYS {
            c.plugin_keys += t.matches(k).count();
        }
        for k in PARAM_KEYS {
            c.param_keys += t.matches(k).count();
        }
        c.keys = c.top_keys + c.plugin_keys + c.param_keys;
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_counts() {
        let b = b"{\n  \"output\": {\n    \"blocklist\": [],\n    \"plugins_order\": [\"compressor#0\", \"limiter#0\", \"equalizer#0\"],\n    \"compressor#0\": {\n      \"attack\": 20.0, \"ratio\": 4.0, \"threshold\": -18.0,\n      \"makeup\": 6.0, \"sidechain\": { \"type\": \"Feedforward\" }\n    },\n    \"limiter#0\": {\n      \"attack\": 5.0, \"release\": 100.0, \"threshold\": -3.0,\n      \"stereo-link\": 100.0, \"oversampling\": \"Half X4\"\n    },\n    \"equalizer#0\": {\n      \"mode\": \"IIR\", \"num-bands\": 8,\n      \"band0\": { \"frequency\": 31.5, \"gain\": 0.0 }\n    }\n  }\n}\n";
        assert!(detect(b));
        let c = Eepreset::parse(b).unwrap();
        assert!(c.top_keys >= 2);
        assert!(c.plugin_keys >= 3);
        assert!(c.param_keys >= 5);
        assert!(c.keys >= 10);
    }

    #[test]
    fn rejects_json() {
        assert!(!detect(b"{\"a\": 1, \"b\": 2}"));
        assert!(Eepreset::parse(b"a = b\n").is_none());
    }
}
