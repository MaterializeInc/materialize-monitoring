// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Just enough Prometheus text-exposition parsing to read a counter off a
//! component's own `/metrics`.
//!
//! Deliberately not a full parser. The suite reads a handful of self-monitoring
//! counters to answer "did this component do the thing at all", and pulling in a
//! parser crate for that is more dependency than the job needs.

/// Sum every sample of `name`, across all label sets.
///
/// Returns `None` when the family is absent — which is a different answer from
/// `Some(0.0)`. Absent means the component never registered the counter (wrong
/// component, wrong port, wrong version); zero means it registered it and the
/// thing has not happened. Collapsing the two turns "I am talking to the wrong
/// pod" into "ingestion has not started yet", and the suite then retries a
/// mistake until the deadline.
pub fn sum_samples(body: &str, name: &str) -> Option<f64> {
    let mut total = None;

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(rest) = line.strip_prefix(name) else {
            continue;
        };
        // Guard against a longer family sharing this prefix: after the name
        // comes either a label set or whitespace, never another name character.
        if !matches!(rest.chars().next(), Some('{') | Some(' ') | Some('\t')) {
            continue;
        }
        // A sample is `name[{labels}] value [timestamp]`. Skipping the label set
        // wholesale rather than splitting on whitespace: a label *value* may
        // contain spaces, and an optional trailing timestamp means the last
        // field is not reliably the value either.
        let rest = match rest.strip_prefix('{') {
            Some(labelled) => match labelled.find('}') {
                Some(end) => &labelled[end + 1..],
                None => continue,
            },
            None => rest,
        };
        let Some(value) = rest.split_whitespace().next() else {
            continue;
        };
        if let Ok(value) = value.parse::<f64>() {
            *total.get_or_insert(0.0) += value;
        }
    }

    total
}

#[cfg(test)]
mod tests {
    use super::sum_samples;

    #[test]
    fn sums_across_label_sets() {
        let body = "\
# HELP loki_ingester_streams_created_total Total streams created.
# TYPE loki_ingester_streams_created_total counter
loki_ingester_streams_created_total{tenant=\"loki\"} 59
loki_ingester_streams_created_total{tenant=\"audit\"} 3
";
        assert_eq!(
            sum_samples(body, "loki_ingester_streams_created_total"),
            Some(62.0)
        );
    }

    #[test]
    fn reads_an_unlabelled_sample() {
        assert_eq!(sum_samples("some_counter 7", "some_counter"), Some(7.0));
    }

    #[test]
    fn absent_is_none_not_zero() {
        assert_eq!(sum_samples("other_counter 1", "some_counter"), None);
    }

    /// A prefix match would report `2` here, and the caller would conclude the
    /// component is healthy off a metric it never exported.
    #[test]
    fn does_not_match_a_longer_family_sharing_the_prefix() {
        let body = "some_counter_bytes 2";
        assert_eq!(sum_samples(body, "some_counter"), None);
    }
}
