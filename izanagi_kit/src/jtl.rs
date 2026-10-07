//! Apache JMeter JTL result census (CSV and `<testResults>` XML forms).
//!
//! CSV JTL files start with a header containing `timeStamp,elapsed,label` and
//! include `success` and `threadName` columns; XML JTL files use
//! `<testResults>` with `<sample>`/`<httpSample>` elements carrying
//! `s="true|false"` and `tn="thread"` attributes. `parse` counts samples,
//! failures, distinct labels and distinct thread names.
//!
//! ```rust
//! let csv = b"timeStamp,elapsed,label,responseCode,responseMessage,threadName,success\n\
//!   1000,5,home,200,OK,g1 1-1,true\n\
//!   2000,7,api,500,ERR,g1 1-2,false\n\
//!   3000,3,home,200,OK,g1 1-1,true\n";
//! let c = izanagi_kit::jtl::Jtl::parse(csv).unwrap();
//! assert_eq!(c.samples, 3);
//! assert_eq!(c.errors, 1);
//! ```

/// JTL result census.
#[derive(Debug, Clone)]
pub struct Jtl {
    /// Sample rows / elements.
    pub samples: usize,
    /// Failed samples (`success` = `false` or `s="false"`).
    pub errors: usize,
    /// Distinct `label` values.
    pub labels: usize,
    /// Distinct thread names (`threadName` / `tn`).
    pub threads: usize,
}

fn distinct_csv_col(lines: &[&str], idx: usize) -> usize {
    let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for l in lines {
        let v = l.split(',').nth(idx).unwrap_or("");
        if seen.insert(v) {}
    }
    seen.len()
}

/// Whether the buffer looks like a JMeter JTL file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    (t.contains("timeStamp") && t.contains("elapsed") && t.contains("success"))
        || (t.contains("<testResults") && (t.contains("<httpSample") || t.contains("<sample")))
}

impl Jtl {
    /// Parse a JTL file into census counts.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        if t.contains("<testResults") {
            let samples = t.matches("<httpSample").count()
                + t.matches("<sample ").count()
                + t.matches("<sample>").count()
                + t.matches("<sample/").count();
            let errors = t.matches("s=\"false\"").count();
            let mut labels: Vec<&str> = Vec::new();
            let mut threads: Vec<&str> = Vec::new();
            let mut rest = t;
            while let Some(i) = rest.find("lb=\"") {
                let v = rest[i + 4..].split('"').next().unwrap_or("");
                if !labels.contains(&v) {
                    labels.push(v);
                }
                rest = &rest[i + 4..];
            }
            let mut rest = t;
            while let Some(i) = rest.find("tn=\"") {
                let v = rest[i + 4..].split('"').next().unwrap_or("");
                if !threads.contains(&v) {
                    threads.push(v);
                }
                rest = &rest[i + 4..];
            }
            return Some(Self {
                samples,
                errors,
                labels: labels.len(),
                threads: threads.len(),
            });
        }
        let mut lines: Vec<&str> = t.lines().collect();
        if lines.first().is_some_and(|h| h.contains("timeStamp")) {
            lines.remove(0);
        }
        let rows: Vec<&str> = lines.into_iter().filter(|l| !l.trim().is_empty()).collect();
        let mut errors = 0usize;
        for r in &rows {
            if r.split(',').any(|f| f.trim() == "false") {
                errors += 1;
            }
        }
        Some(Self {
            samples: rows.len(),
            errors,
            labels: distinct_csv_col(&rows, 2),
            threads: distinct_csv_col(&rows, 5),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_csv() {
        let b = b"timeStamp,elapsed,label,responseCode,responseMessage,threadName,success\n1,5,home,200,OK,g1 1-1,true\n2,7,api,500,ERR,g1 1-2,false\n3,3,home,200,OK,g1 1-1,true\n";
        let c = Jtl::parse(b).unwrap();
        assert_eq!(c.samples, 3);
        assert_eq!(c.errors, 1);
        assert_eq!(c.labels, 2);
        assert_eq!(c.threads, 2);
    }

    #[test]
    fn parses_xml() {
        let b = br#"<testResults version="1.2">
          <httpSample t="5" s="true" lb="home" tn="g1 1-1"/>
          <httpSample t="7" s="false" lb="api" tn="g1 1-2"/>
          <sample t="3" s="true" lb="home" tn="g1 1-1"/>
        </testResults>"#;
        let c = Jtl::parse(b).unwrap();
        assert_eq!(c.samples, 3);
        assert_eq!(c.errors, 1);
        assert_eq!(c.labels, 2);
        assert_eq!(c.threads, 2);
    }

    #[test]
    fn rejects_other() {
        assert!(Jtl::parse(b"a,b,c\n1,2,3\n").is_none());
    }
}
