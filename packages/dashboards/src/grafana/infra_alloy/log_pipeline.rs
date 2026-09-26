// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Log Pipeline tab: a log line's path from a node to the store.
//!
//! Rows follow the line. The agents read it, a delivery hop sends it to the
//! gateway, the gateway's processing may discard it, and a second delivery hop
//! sends it to Loki. The two delivery hops share one row because they are the
//! same component, `loki.write`, and read the same way; the destination label
//! says which hop a series is.
//!
//! Gateway Ingest comes last although it sits mid-path. It is the receiving end
//! of the first hop, and it is read after Delivery has said that hop is failing,
//! to learn whether the gateway is the one refusing.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::{BigValueGraphMode, BigValueTextMode};
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::overview::{self, LINES_PER_SECOND, not_collected};
use super::theme;
use crate::grafana::queries::Queries;

const SHADE: &str = theme::LOG_PIPELINE.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![flow(q), agents(q), delivery(q), processing(q), ingest(q)]
}

fn flow(q: &Queries) -> Row {
    Row::new("Flow").hide_header().grid(
        AutoGrid::new(4)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("logs-flow-read", lines_read(q))
            .panel("logs-flow-stored", lines_stored(q))
            .panel("logs-flow-lost", overview::log_lines_lost(q))
            .panel("logs-flow-propagation", propagation(q)),
    )
}

fn agents(q: &Queries) -> Row {
    Row::new("Agents").grid(
        AutoGrid::new(3)
            .panel("logs-agents-files", files(q))
            .panel("logs-agents-file-lines", file_lines(q))
            .panel("logs-agents-journal-lines", journal_lines(q)),
    )
}

fn delivery(q: &Queries) -> Row {
    Row::new("Delivery").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("logs-delivery-requests", requests(q))
            .panel("logs-delivery-dropped", dropped(q))
            .panel("logs-delivery-retries", retries(q))
            .panel("logs-delivery-latency", request_latency(q)),
    )
}

fn processing(q: &Queries) -> Row {
    Row::new("Processing").grid(
        AutoGrid::new(2)
            .panel("logs-processing-guards", guard_drops(q))
            .panel("logs-processing-truncated", truncated(q)),
    )
}

fn ingest(q: &Queries) -> Row {
    Row::new("Gateway Ingest").grid(
        AutoGrid::new(2)
            .panel("logs-ingest-pushes", pushes_received(q))
            .panel("logs-ingest-otlp", otlp_received(q)),
    )
}

fn lines_read(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Lines Read")
        .query(q.get("infra.alloy.log_pipeline.read").legend("lines/s"))
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(NoValue::Custom(
            "No agent is reporting what it reads".to_string(),
        ))
        .build(0)
}

fn lines_stored(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Lines Delivered to Store")
        .query(q.get("infra.alloy.log_pipeline.stored").legend("lines/s"))
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

/// One tile per role, named, since the gateway's figure includes the agent hop
/// and the pair is what says which hop the delay is in.
fn propagation(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Time to Delivery (p99)")
        .query(
            q.get("infra.alloy.log_pipeline.propagation")
                .legend("{{app}}"),
        )
        .graph_mode(BigValueGraphMode::Area)
        .text_mode(BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn files(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Files Tailed")
        .query(q.get("infra.alloy.log_pipeline.files").legend("{{pod}}"))
        .unit("short")
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn file_lines(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Container Log Lines Read")
        .query(
            q.get("infra.alloy.log_pipeline.file_lines")
                .legend("{{pod}}"),
        )
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn journal_lines(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Journal Lines Read")
        .query(
            q.get("infra.alloy.log_pipeline.journal_lines")
                .legend("{{pod}}"),
        )
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn requests(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Delivery Requests by Status")
        .query(
            q.get("infra.alloy.log_pipeline.requests")
                .legend("{{host}} {{status_code}}"),
        )
        .unit("reqps")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn dropped(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Lines Dropped at Delivery")
        .query(
            q.get("infra.alloy.log_pipeline.dropped")
                .legend("{{app}} {{reason}}"),
        )
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(NoValue::Custom(
            "Nothing was dropped at delivery".to_string(),
        ))
        .build(0)
}

fn retries(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Delivery Retries")
        .query(
            q.get("infra.alloy.log_pipeline.retries")
                .legend("{{app}} to {{host}}"),
        )
        .unit("suffix:batches/s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn request_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Delivery Latency (p99)")
        .query(
            q.get("infra.alloy.log_pipeline.request_latency")
                .legend("{{host}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(not_collected())
        .build(0)
}

fn guard_drops(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Lines Discarded by Guards")
        .query(
            q.get("infra.alloy.log_pipeline.guard_drops")
                .legend("{{app}} {{reason}}"),
        )
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(NoValue::Custom(
            "No guard discarded anything in this range".to_string(),
        ))
        .build(0)
}

fn truncated(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Truncated Lines")
        .query(
            q.get("infra.alloy.log_pipeline.truncated")
                .legend("{{pod}}"),
        )
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn pushes_received(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pushes Received by Status")
        .query(
            q.get("infra.alloy.log_pipeline.pushes_received")
                .legend("{{status_code}}"),
        )
        .unit("reqps")
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn otlp_received(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("OTLP Log Records Received")
        .query(q.legended(
            "infra.alloy.log_pipeline.otlp_received",
            &["accepted", "refused"],
        ))
        .unit("suffix:records/s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "Nothing has sent logs to the gateway over OTLP".to_string(),
        ))
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_queries;
    use std::collections::BTreeSet;

    /// Reasons a pipeline guard records, which are losses.
    ///
    /// `ratelimit_drop_stage` is Alloy's built-in reason for a `stage.limit`
    /// with `drop = true`, so it is named nowhere in the pipelines themselves.
    const GUARD_REASONS: &[&str] = &[
        "backlog > 2hr",
        "processed time > 2hr old",
        "too large",
        "ratelimit_drop_stage",
    ];

    /// Every `drop_counter_reason` the rendered pipelines set.
    fn pipeline_reasons() -> BTreeSet<String> {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../charts/materialize-monitoring/pre-rendered/pipelines");
        let mut out = BTreeSet::new();
        for entry in std::fs::read_dir(&dir).expect("pipelines directory") {
            let path = entry.expect("entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("alloy") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read pipeline");
            for line in text.lines() {
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                if key.trim() == "drop_counter_reason" {
                    out.insert(value.trim().trim_matches('"').to_string());
                }
            }
        }
        assert!(
            !out.is_empty(),
            "found no drop reasons in {}",
            dir.display()
        );
        out
    }

    /// The deliberate reasons the guard query excludes, read off its matcher.
    fn excluded_reasons(q: &Queries) -> BTreeSet<String> {
        let panel = guard_drops(q);
        let expr = panel.spec.data.spec.queries[0]
            .spec
            .query
            .spec
            .as_ref()
            .expect("spec")["expr"]
            .as_str()
            .expect("expr")
            .to_string();
        let start = expr.find(r#"reason!~""#).expect("an exclusion matcher") + r#"reason!~""#.len();
        let end = start + expr[start..].find('"').expect("closing quote");
        expr[start..end].split('|').map(str::to_string).collect()
    }

    #[test]
    fn the_tab_assembles_with_every_panel_placed() {
        let q = &test_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 15);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn every_pipeline_drop_reason_is_classified() {
        // A new drop stage has to be either a deliberate drop, excluded from the
        // guard panel, or a guard whose losses it draws. Leaving it unclassified
        // would still draw it, but its meaning belongs in the registry prose.
        let q = &test_queries();
        let excluded = excluded_reasons(q);
        for reason in pipeline_reasons() {
            assert!(
                excluded.contains(&reason) || GUARD_REASONS.contains(&reason.as_str()),
                "drop reason {reason:?} is neither excluded as deliberate nor a known guard"
            );
        }
    }

    #[test]
    fn every_excluded_reason_still_exists() {
        // An exclusion for a reason no pipeline sets any more is dead weight,
        // and worse: a later stage reusing the string would be hidden by it.
        let q = &test_queries();
        let reasons = pipeline_reasons();
        for reason in excluded_reasons(q) {
            assert!(
                reasons.contains(&reason),
                "{reason:?} is excluded but unused"
            );
        }
    }

    #[test]
    fn the_status_panel_counts_requests_not_buckets() {
        // Summing `_bucket` series counts each request once per bucket, which
        // is the mistake in the dashboards this one replaces.
        let q = &test_queries();
        let panel = requests(q);
        let expr = panel.spec.data.spec.queries[0]
            .spec
            .query
            .spec
            .as_ref()
            .expect("spec")["expr"]
            .as_str()
            .expect("expr")
            .to_string();
        assert!(
            expr.contains("loki_write_request_duration_seconds_count"),
            "{expr}"
        );
        assert!(!expr.contains("_bucket"), "{expr}");
    }
}
