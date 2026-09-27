//! Minimal reader for Garmin TCX (Training Center XML): `<Activities>`
//! → `<Activity Sport="...">` → `<Lap>` → `<Track><Trackpoint>` with
//! `<Time>`, `<Position><LatitudeDegrees>/<LongitudeDegrees>`,
//! `<AltitudeMeters>`, `<HeartRateBpm><Value>`. Self-contained tag
//! scanning; positions stay verbatim strings.
//!
//! ```
//! use izanagi_kit::tcx::parse;
//!
//! let t = parse(
//!     b"<TrainingCenterDatabase><Activities><Activity Sport=\"Running\">\
//!        <Lap><Track><Trackpoint><Time>2020-01-01T00:00:00Z</Time>\
//!        <Position><LatitudeDegrees>35.1</LatitudeDegrees>\
//!        <LongitudeDegrees>139.7</LongitudeDegrees></Position>\
//!        <HeartRateBpm><Value>150</Value></HeartRateBpm></Trackpoint>\
//!        </Track></Lap></Activity></Activities></TrainingCenterDatabase>",
//! )
//! .unwrap();
//! assert_eq!(t.activities.len(), 1);
//! assert_eq!(t.activities[0].sport.as_deref(), Some("Running"));
//! assert_eq!(t.activities[0].trackpoints.len(), 1);
//! ```

/// One `<Trackpoint>`.
#[derive(Debug, Default)]
pub struct Trackpoint {
    /// `<Time>` text.
    pub time: Option<String>,
    /// `LatitudeDegrees` verbatim.
    pub lat: Option<String>,
    /// `LongitudeDegrees` verbatim.
    pub lon: Option<String>,
    /// `AltitudeMeters` verbatim.
    pub alt: Option<String>,
    /// `<HeartRateBpm><Value>` as an integer.
    pub bpm: Option<u32>,
}

/// One `<Activity>`.
#[derive(Debug)]
pub struct Activity {
    /// `Sport` attribute (`Running`, `Biking`, ...).
    pub sport: Option<String>,
    /// Trackpoints from all `<Track>`/`<Lap>` children, in order.
    pub trackpoints: Vec<Trackpoint>,
}

/// A parsed TCX document.
#[derive(Debug)]
pub struct Tcx {
    /// `<Activity>` elements in order.
    pub activities: Vec<Activity>,
}

fn text_between<'a>(src: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let a = src.find(open)? + open.len();
    let b = src[a..].find(close)? + a;
    Some(&src[a..b])
}

fn attr(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let at = tag.find(&pat)? + pat.len();
    let end = tag[at..].find('"')? + at;
    Some(tag[at..end].to_string())
}

fn trackpoint(src: &str) -> Trackpoint {
    let time = text_between(src, "<Time>", "</Time>").map(str::to_string);
    let (lat, lon) = match text_between(src, "<Position>", "</Position>") {
        Some(pos) => (
            text_between(pos, "<LatitudeDegrees>", "</LatitudeDegrees>")
                .map(|s| s.trim().to_string()),
            text_between(pos, "<LongitudeDegrees>", "</LongitudeDegrees>")
                .map(|s| s.trim().to_string()),
        ),
        None => (None, None),
    };
    let alt =
        text_between(src, "<AltitudeMeters>", "</AltitudeMeters>").map(|s| s.trim().to_string());
    // the `<HeartRateBpm>` open tag carries `runSensorSpeed`; `<Value>` holds the bpm
    let bpm = text_between(src, "<HeartRateBpm", "</HeartRateBpm>")
        .and_then(|hr| text_between(hr, "<Value>", "</Value>"))
        .and_then(|v| v.trim().parse().ok());
    Trackpoint {
        time,
        lat,
        lon,
        alt,
        bpm,
    }
}

/// Parse a TCX document. `None` without `<TrainingCenterDatabase>` /
/// `<Activities>` roots.
pub fn parse(data: &[u8]) -> Option<Tcx> {
    let src = std::str::from_utf8(data).ok()?;
    if !src.contains("<TrainingCenterDatabase") && !src.contains("<Activities") {
        return None;
    }
    let mut activities = Vec::new();
    let mut rest = src;
    while let Some(a) = rest.find("<Activity") {
        // `<Activity` vs `<Activities` boundary
        if rest.as_bytes().get(a + 9) == Some(&b'i') {
            rest = &rest[a + 9..];
            continue;
        }
        let tag_end = rest[a..].find('>')? + a;
        let sport = attr(&rest[a..tag_end + 1], "Sport");
        let body = match rest[tag_end + 1..].find("</Activity>") {
            Some(e) => &rest[tag_end + 1..tag_end + 1 + e],
            None => break,
        };
        let mut tps = Vec::new();
        let mut inner = body;
        while let Some(t) = inner.find("<Trackpoint>") {
            let end = match inner[t..].find("</Trackpoint>") {
                Some(e) => t + e,
                None => break,
            };
            tps.push(trackpoint(&inner[t + 12..end]));
            inner = &inner[end + "</Trackpoint>".len()..];
        }
        activities.push(Activity {
            sport,
            trackpoints: tps,
        });
        rest = &rest[tag_end + 1 + body.len() + "</Activity>".len()..];
    }
    Some(Tcx { activities })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"<TrainingCenterDatabase><Activities>\
<Activity Sport=\"Biking\"><Lap><Track>\
<Trackpoint><Time>t0</Time><Position><LatitudeDegrees>35.0</LatitudeDegrees>\
<LongitudeDegrees>139.0</LongitudeDegrees></Position>\
<AltitudeMeters>42</AltitudeMeters>\
<HeartRateBpm runSensorSpeed=\"x\"><Value>120</Value></HeartRateBpm></Trackpoint>\
<Trackpoint><Time>t1</Time></Trackpoint>\
</Track></Lap></Activity></Activities></TrainingCenterDatabase>";

    #[test]
    fn parses() {
        let t = parse(DOC).unwrap();
        assert_eq!(t.activities.len(), 1);
        let a = &t.activities[0];
        assert_eq!(a.sport.as_deref(), Some("Biking"));
        assert_eq!(a.trackpoints.len(), 2);
        let tp = &a.trackpoints[0];
        assert_eq!(tp.time.as_deref(), Some("t0"));
        assert_eq!(tp.lat.as_deref(), Some("35.0"));
        assert_eq!(tp.lon.as_deref(), Some("139.0"));
        assert_eq!(tp.alt.as_deref(), Some("42"));
        assert_eq!(tp.bpm, Some(120));
        assert_eq!(a.trackpoints[1].lat, None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<gpx/>").is_none());
        assert!(parse(&[0xFF]).is_none());
        // bare <Activity> with no close — tolerated (loop just ends)
        let t = parse(b"<TrainingCenterDatabase><Activity Sport=\"x\">").unwrap();
        assert_eq!(t.activities.len(), 0);
    }
}
