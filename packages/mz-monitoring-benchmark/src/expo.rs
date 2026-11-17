// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Prometheus text exposition (format 0.0.4): enough to read what Materialize
//! serves, and to write what `gen-load` serves.
//!
//! The fixture format reuses the sample syntax, with one change: the slot a
//! sample's timestamp would take holds the counter's rate instead. So the
//! trailing token is kept as a string here and each caller interprets it.

use std::fmt::Write as _;

use anyhow::{Result, bail};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Counter,
    Gauge,
    Histogram,
    Summary,
    Untyped,
}

impl Kind {
    pub fn parse(s: &str) -> Kind {
        match s {
            "counter" => Kind::Counter,
            "gauge" => Kind::Gauge,
            "histogram" => Kind::Histogram,
            "summary" => Kind::Summary,
            _ => Kind::Untyped,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Counter => "counter",
            Kind::Gauge => "gauge",
            Kind::Histogram => "histogram",
            Kind::Summary => "summary",
            Kind::Untyped => "untyped",
        }
    }
}

/// Whether a sample only ever goes up: a counter, or the bucket, count and sum
/// series of a histogram or summary.
///
/// A histogram's `_sum` can fall when it observes negative values. Nothing
/// Materialize exports does, and a fixture records a negative rate as zero.
pub fn counter_like(kind: Kind, family: &str, sample: &str) -> bool {
    let suffix = sample.strip_prefix(family).unwrap_or("");
    match kind {
        Kind::Counter => true,
        Kind::Histogram => matches!(suffix, "_bucket" | "_count" | "_sum"),
        Kind::Summary => matches!(suffix, "_count" | "_sum"),
        Kind::Gauge => false,
        Kind::Untyped => sample.ends_with("_total"),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub name: String,
    pub labels: Vec<(String, String)>,
    /// The value token, unparsed: fixtures write symbolic values here.
    pub value: String,
    /// The token after the value: a timestamp in exposition, a rate in a fixture.
    pub trailer: Option<String>,
}

impl Sample {
    pub fn label(&self, name: &str) -> Option<&str> {
        self.labels
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Family {
    pub name: String,
    pub kind: Kind,
    pub help: Option<String>,
    pub samples: Vec<Sample>,
}

/// A parsed exposition body: its leading `# key: value` comments, then families.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    pub meta: Vec<(String, String)>,
    pub families: Vec<Family>,
}

const SUFFIXES: &[&str] = &["_bucket", "_count", "_sum", "_total", "_created", "_info"];

/// Whether `sample` belongs to the family named `family`.
fn belongs(family: &str, sample: &str) -> bool {
    match sample.strip_prefix(family) {
        Some("") => true,
        Some(rest) => SUFFIXES.contains(&rest),
        None => false,
    }
}

pub fn parse(body: &str) -> Result<Document> {
    let mut doc = Document::default();
    let mut seen_family = false;

    for (n, line) in body.lines().enumerate() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        if let Some(comment) = line.strip_prefix('#') {
            let comment = comment.trim_start();
            if let Some(rest) = comment.strip_prefix("HELP ") {
                let (name, text) = rest.split_once(' ').unwrap_or((rest, ""));
                family_for(&mut doc.families, name).help = Some(text.to_string());
                seen_family = true;
            } else if let Some(rest) = comment.strip_prefix("TYPE ") {
                let (name, kind) = rest.split_once(' ').unwrap_or((rest, "untyped"));
                family_for(&mut doc.families, name).kind = Kind::parse(kind.trim());
                seen_family = true;
            } else if !seen_family && let Some((k, v)) = comment.split_once(": ") {
                doc.meta.push((k.trim().to_string(), v.trim().to_string()));
            }
            continue;
        }
        let sample = parse_sample(line).map_err(|e| anyhow::anyhow!("line {}: {e}", n + 1))?;
        seen_family = true;
        let fits = doc
            .families
            .last()
            .is_some_and(|f| belongs(&f.name, &sample.name));
        if !fits {
            doc.families.push(Family {
                name: sample.name.clone(),
                kind: Kind::Untyped,
                help: None,
                samples: Vec::new(),
            });
        }
        doc.families
            .last_mut()
            .expect("pushed above")
            .samples
            .push(sample);
    }
    Ok(doc)
}

/// The family `name`, continuing the last one when it is the same family —
/// `# HELP` and `# TYPE` arrive as two lines for one family.
fn family_for<'a>(families: &'a mut Vec<Family>, name: &str) -> &'a mut Family {
    let continues = families
        .last()
        .is_some_and(|f| f.name == name && f.samples.is_empty());
    if !continues {
        families.push(Family {
            name: name.to_string(),
            kind: Kind::Untyped,
            help: None,
            samples: Vec::new(),
        });
    }
    families.last_mut().expect("pushed above")
}

pub fn parse_sample(line: &str) -> Result<Sample> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b':')
    {
        i += 1;
    }
    if i == 0 {
        bail!("expected a metric name in {line:?}");
    }
    let name = line[..i].to_string();
    let mut labels = Vec::new();
    if bytes.get(i) == Some(&b'{') {
        i += 1;
        loop {
            while bytes.get(i) == Some(&b' ') {
                i += 1;
            }
            if bytes.get(i) == Some(&b'}') {
                i += 1;
                break;
            }
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let key = line[start..i].to_string();
            if key.is_empty() || bytes.get(i) != Some(&b'=') || bytes.get(i + 1) != Some(&b'"') {
                bail!("malformed label set in {line:?}");
            }
            i += 2;
            let mut value = String::new();
            loop {
                match bytes.get(i) {
                    None => bail!("unterminated label value in {line:?}"),
                    Some(b'"') => {
                        i += 1;
                        break;
                    }
                    Some(b'\\') => {
                        match bytes.get(i + 1) {
                            Some(b'n') => value.push('\n'),
                            Some(b'\\') => value.push('\\'),
                            Some(b'"') => value.push('"'),
                            _ => bail!("bad escape in {line:?}"),
                        }
                        i += 2;
                    }
                    Some(_) => {
                        let ch = line[i..].chars().next().expect("in bounds");
                        value.push(ch);
                        i += ch.len_utf8();
                    }
                }
            }
            labels.push((key, value));
            match bytes.get(i) {
                Some(b',') => i += 1,
                Some(b'}') => {
                    i += 1;
                    break;
                }
                _ => bail!("expected , or }} in {line:?}"),
            }
        }
    }
    let mut rest = line[i..].split_whitespace();
    let Some(value) = rest.next() else {
        bail!("sample without a value: {line:?}");
    };
    Ok(Sample {
        name,
        labels,
        value: value.to_string(),
        trailer: rest.next().map(str::to_string),
    })
}

/// A float as exposition spells it.
pub fn write_f64(out: &mut String, v: f64) {
    if v.is_nan() {
        out.push_str("NaN");
    } else if v == f64::INFINITY {
        out.push_str("+Inf");
    } else if v == f64::NEG_INFINITY {
        out.push_str("-Inf");
    } else {
        let _ = write!(out, "{v}");
    }
}

pub fn parse_f64(s: &str) -> Option<f64> {
    match s {
        "+Inf" | "Inf" => Some(f64::INFINITY),
        "-Inf" => Some(f64::NEG_INFINITY),
        "NaN" => Some(f64::NAN),
        _ => s.parse().ok(),
    }
}

pub fn escape_label_value(out: &mut String, v: &str) {
    for ch in v.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = r#"# HELP mz_foo_total Foo.
# TYPE mz_foo_total counter
mz_foo_total{a="b",c="d \"q\" \\ e"} 12 1700000000000
# HELP mz_lat Latency.
# TYPE mz_lat histogram
mz_lat_bucket{le="0.1"} 1
mz_lat_bucket{le="+Inf"} 3
mz_lat_sum 0.5
mz_lat_count 3
untyped_thing 4
"#;

    #[test]
    fn parses_families_and_escapes() {
        let doc = parse(BODY).unwrap();
        assert_eq!(doc.families.len(), 3);
        let foo = &doc.families[0];
        assert_eq!(foo.kind, Kind::Counter);
        assert_eq!(foo.samples[0].label("c"), Some(r#"d "q" \ e"#));
        assert_eq!(foo.samples[0].trailer.as_deref(), Some("1700000000000"));
        let lat = &doc.families[1];
        assert_eq!(lat.kind, Kind::Histogram);
        assert_eq!(lat.samples.len(), 4);
        assert_eq!(doc.families[2].kind, Kind::Untyped);
    }

    #[test]
    fn reads_leading_metadata() {
        let doc = parse("# role: clusterd\n# path: /metrics\n# HELP x X.\nx 1\n").unwrap();
        assert_eq!(doc.meta[0], ("role".to_string(), "clusterd".to_string()));
        assert_eq!(doc.meta.len(), 2);
    }

    #[test]
    fn escape_round_trips() {
        let mut out = String::from("x{v=\"");
        escape_label_value(&mut out, "a\"b\\c\nd");
        out.push_str("\"} 1");
        assert_eq!(parse_sample(&out).unwrap().label("v"), Some("a\"b\\c\nd"));
    }

    #[test]
    fn counter_like_by_kind_and_suffix() {
        assert!(counter_like(Kind::Histogram, "mz_lat", "mz_lat_bucket"));
        assert!(!counter_like(Kind::Summary, "mz_q", "mz_q"));
        assert!(counter_like(Kind::Summary, "mz_q", "mz_q_count"));
        assert!(!counter_like(Kind::Gauge, "g", "g"));
    }
}
