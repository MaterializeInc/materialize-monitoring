// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Compliance tab: what the gateway removed from log lines before storing
//! them, and whether it looked at every line.
//!
//! Two checks run on the gateway's log path. The card-number check,
//! `stage.luhn`, runs on every line. The secret filter runs as three
//! `loki.secretfilter` instances: `ingest` scans the full tier, `ingestBulk`
//! the `DEBUG` and `TRACE` tier, and `shadow`, in shadow mode only, scans a
//! copy of every line and stores what it would have redacted.
//!
//! # A redaction is rare, and its first one is the one that matters
//!
//! Alloy creates a redaction counter at its first redaction, so the series
//! starts at 1 and a rate never draws that first one. A credential logged once
//! is the common case. So the verdict and the tables count over the selected
//! range in the form the alert rules use, which counts a series created inside
//! the range by its whole value; `infra-alloy.yaml` has the details. The graphs
//! say *when*, for redactions that repeat.
//!
//! # Log axes
//!
//! Several rules match strings that aren't credentials, and one random 13- to
//! 19-digit number in ten passes the card-number checksum. So a busy install
//! carries a steady rate of redactions that mean nothing, and the reading that
//! matters is a small rate appearing beside it. The graphs that split
//! redactions by rule or by source are on a log axis, which draws the small rate
//! as clearly as the large one. The linear ones are listed, with reasons, in the
//! tests.
//!
//! # The store
//!
//! The rows that read Loki start collapsed, because they read stored lines
//! rather than counters. Counting lines that carry a placeholder reads every
//! stored line in range, so that row is also hidden past
//! [`volume_guard::THRESHOLD`], with a twin that says why. The shadow copies and
//! the partly scanned lines carry a label of their own, which Loki finds
//! without reading anything else.
//!
//! None of the Loki queries take the Alloy pickers: the lines come from every
//! workload, and the collectors are only one of them. They split by `job` and
//! `unit`, like the card-number panels, because a node's journal lines carry
//! no namespace or container.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::BigValueTextMode;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel, Stat};
use mzmon_lib::grafana::threshold;

use super::logs::LINES_PER_MINUTE;
use super::overview::LINES_PER_SECOND;
use super::theme;
use crate::grafana::queries::Queries;
use crate::grafana::{field_override, transform, volume_guard};

const SHADE: &str = theme::COMPLIANCE.shade;

/// Unit for a count over the selected range: exact, with thousands separators,
/// since a compliance reading wants 4,971 rather than `5 K`.
const COUNT: &str = "locale";

/// Unit for the secret filter's per-second redaction rates.
const REDACTIONS_PER_SECOND: &str = "suffix:redactions/s";

/// The y axis base for the graphs that watch for a small rate beside a steady
/// one. See the module docs.
const LOG_BASE: f64 = 10.0;

/// Decimals for a rate on a [`LOG_BASE`] axis.
///
/// Grafana guesses one decimal count for every tick from the axis's linear
/// range, so the ticks below 1 would read `0`. Three covers one redaction per
/// `$__rate_interval` over a week's range.
const RATE_DECIMALS: f64 = 3.0;

/// The guarded row's title. Its stand-in carries the same title plus why it's
/// empty, since Grafana flags two rows with one title.
const STORED_TITLE: &str = "Stored Lines with a Placeholder";

/// What the twin of the guarded row says.
const STORED_NOTE: &str = "**These panels are hidden for ranges longer than 7 days.**\n\n\
     Finding the lines that carry a placeholder means reading every stored line in the range, \
     because Loki indexes labels, not line content.\n\n\
     Shorten the time range to 7 days or less and they come back. To search a longer time range, \
     run `{job=~\".+\", secretfilter!=\"shadow\"} |= \"<redacted:\"` in [Explore](/explore). \
     The counts above cover any time range.";

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        verdict(q),
        redactions(q),
        cards(q),
        coverage(q),
        shadow(q),
        stored(q),
        stored_hidden(),
        shadow_copies(q),
        partly_scanned(q),
    ]
}

fn verdict(q: &Queries) -> Row {
    Row::new("Summary").hide_header().grid(
        AutoGrid::new(5)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("compliance-scanned", scanned(q))
            .panel("compliance-redacted", redacted(q))
            .panel("compliance-cards", card_lines(q))
            .panel("compliance-timed-out", timed_out(q))
            .panel("compliance-shadow", shadow_matches(q)),
    )
}

fn redactions(q: &Queries) -> Row {
    Row::new("Redactions").grid(
        AutoGrid::new(3)
            .panel("compliance-redacted-by-rule", redacted_by_rule(q))
            .panel("compliance-redacted-by-origin", redacted_by_origin(q))
            .panel("compliance-redacted-table", redacted_table(q)),
    )
}

fn cards(q: &Queries) -> Row {
    Row::new("Card Numbers").grid(
        AutoGrid::new(2)
            .panel("compliance-cards-by-source", cards_by_source(q))
            .panel("compliance-cards-table", cards_table(q)),
    )
}

fn coverage(q: &Queries) -> Row {
    Row::new("Filter Coverage").grid(
        AutoGrid::new(3)
            .panel("compliance-coverage", lines_scanned(q))
            .panel("compliance-scan-time", scan_time(q))
            .panel("compliance-timeouts", timeouts(q)),
    )
}

fn shadow(q: &Queries) -> Row {
    Row::new("Shadow Mode").grid(
        AutoGrid::new(3)
            .panel("compliance-shadow-by-rule", shadow_by_rule(q))
            .panel("compliance-shadow-by-origin", shadow_by_origin(q))
            .panel("compliance-shadow-table", shadow_table(q)),
    )
}

fn stored(q: &Queries) -> Row {
    Row::new(STORED_TITLE)
        .collapsed()
        .only_within(volume_guard::THRESHOLD)
        .grid(
            AutoGrid::new(1)
                .row_height(RowHeight::Tall)
                .panel("compliance-stored-rate", stored_rate(q))
                .panel("compliance-stored-feed", stored_feed(q)),
        )
}

/// The guarded row's stand-in past [`volume_guard::THRESHOLD`].
///
/// Collapsed and titled after the row it replaces, unlike the logs tabs' shared
/// stand-in, so the tab's layout is the same at every range and the note shows
/// only to a reader who expands it.
fn stored_hidden() -> Row {
    Row::new(format!("{STORED_TITLE} (Hidden Past 7 Days)"))
        .collapsed()
        .only_beyond(volume_guard::THRESHOLD)
        .grid(
            AutoGrid::new(1).row_height(RowHeight::Short).panel(
                "compliance-stored-hidden-note",
                Panel::text("Stored Lines (skipped)", STORED_NOTE)
                    .description("Why the stored-line panels aren't drawn over long ranges.")
                    .build(0),
            ),
        )
}

fn shadow_copies(q: &Queries) -> Row {
    Row::new("Shadow Copies in the Store").collapsed().grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("compliance-shadow-copies-rate", shadow_copies_rate(q))
            .panel("compliance-shadow-copies-feed", shadow_copies_feed(q)),
    )
}

fn partly_scanned(q: &Queries) -> Row {
    Row::new("Partly Scanned Lines in the Store")
        .collapsed()
        .grid(
            AutoGrid::new(1)
                .row_height(RowHeight::Tall)
                .panel("compliance-partly-scanned-rate", partly_scanned_rate(q))
                .panel("compliance-partly-scanned-feed", partly_scanned_feed(q)),
        )
}

// --- empty states --------------------------------------------------------------

/// The secret filter and the card-number check run on the gateway only, so a
/// role picker set to the agents empties every metric panel here.
fn no_gateway() -> NoValue {
    NoValue::Custom(
        "No data from the gateway. Check that the role picker includes alloy-gateway, and that \
         the gateway is running."
            .to_string(),
    )
}

fn none_in_range(what: &str) -> NoValue {
    NoValue::Custom(format!("No {what} in this time range."))
}

// --- summary ---------------------------------------------------------------------

/// A count over the selected range, where anything above zero is a finding.
fn finding(title: &str) -> Panel<Stat> {
    Panel::stat(title)
        .color_background()
        .unit(COUNT)
        .decimals(0.0)
        .min(0.0)
}

/// One tile per ingest instance, in the tab's shade rather than health colours:
/// 0% is the expected reading while the filter is `off` or in shadow mode.
fn scanned(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Lines Scanned (Select Time Range)")
        .query(
            q.get("infra.alloy.compliance.scanned")
                .legend("{{component_id}}"),
        )
        .text_mode(BigValueTextMode::ValueAndName)
        .shade(SHADE)
        .unit("percentunit")
        .decimals(1.0)
        .min(0.0)
        .max(1.0)
        .no_value(no_gateway())
        .build(0)
}

fn redacted(q: &Queries) -> dashboardv2::PanelKind {
    finding("Credentials Redacted (Select Time Range)")
        .query(q.get("infra.alloy.compliance.redacted").legend("redacted"))
        .thresholds(threshold::errors(1.0, 10.0).build())
        .no_value(no_gateway())
        .build(0)
}

/// Lines rather than numbers, since that is what the stage's counter counts.
fn card_lines(q: &Queries) -> dashboardv2::PanelKind {
    finding("Card Numbers Replaced (Select Time Range)")
        .query(q.get("infra.alloy.compliance.cards").legend("lines"))
        .thresholds(threshold::errors(1.0, 100.0).build())
        .no_value(no_gateway())
        .build(0)
}

fn timed_out(q: &Queries) -> dashboardv2::PanelKind {
    finding("Lines Timed Out (Select Time Range)")
        .query(q.get("infra.alloy.compliance.timed_out").legend("lines"))
        .thresholds(threshold::errors(1.0, 100.0).build())
        .no_value(no_gateway())
        .build(0)
}

/// In the tab's shade: a match in shadow mode is something to read, not a
/// fault.
fn shadow_matches(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Shadow Matches (Select Time Range)")
        .query(q.get("infra.alloy.compliance.shadow").legend("matches"))
        .shade(SHADE)
        .unit(COUNT)
        .decimals(0.0)
        .min(0.0)
        .no_value(no_gateway())
        .build(0)
}

// --- redactions ------------------------------------------------------------------

fn redacted_by_rule(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Redactions by Rule")
        .query(
            q.get("infra.alloy.compliance.redacted.by_rule")
                .legend("{{rule}}"),
        )
        .unit(REDACTIONS_PER_SECOND)
        .log_scale(LOG_BASE)
        .decimals(RATE_DECIMALS)
        .no_value(none_in_range("redactions"))
        .build(0)
}

fn redacted_by_origin(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Redactions by Origin")
        .query(
            q.get("infra.alloy.compliance.redacted.by_origin")
                .legend("{{origin}}"),
        )
        .unit(REDACTIONS_PER_SECOND)
        .log_scale(LOG_BASE)
        .decimals(RATE_DECIMALS)
        .no_value(none_in_range("redactions"))
        .build(0)
}

fn redacted_table(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Redactions by Rule and Origin (Select Time Range)")
        .query(
            q.get("infra.alloy.compliance.redacted.table")
                .table_format(),
        )
        .transformations(vec![
            transform::organize_full(
                &["Time"],
                &["rule", "origin", "Value"],
                &[
                    ("rule", "Rule"),
                    ("origin", "Origin"),
                    ("Value", "Redactions"),
                ],
            ),
            transform::sort_by("Redactions", true),
        ])
        .unit(COUNT)
        .decimals(0.0)
        .no_value(none_in_range("redactions"))
        .build(0)
}

/// A dash in a column's empty cells, where Grafana would otherwise put the
/// panel's empty-state text.
fn blank_when_missing(field: &str) -> dashboardv2::FieldConfigSourceOverridesItem {
    field_override::by_name(field)
        .property("noValue", serde_json::Value::String("—".to_string()))
        .build()
}

// --- card numbers ----------------------------------------------------------------

/// By `job` and `unit`: a container stream's `job` is
/// `<namespace>/<container>`, and a journal line's names the journal, with the
/// systemd unit beside it.
fn cards_by_source(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Card Numbers by Source")
        .query(
            q.get("infra.alloy.compliance.cards.by_source")
                .legend("{{job}} {{unit}}"),
        )
        .unit(LINES_PER_SECOND)
        .log_scale(LOG_BASE)
        .decimals(RATE_DECIMALS)
        .no_value(none_in_range("card numbers"))
        .build(0)
}

fn cards_table(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Card Numbers by Source (Select Time Range)")
        .query(q.get("infra.alloy.compliance.cards.table").table_format())
        .transformations(vec![
            transform::organize_full(
                &["Time"],
                &["job", "unit", "Value"],
                &[("job", "Source"), ("unit", "Unit"), ("Value", "Lines")],
            ),
            transform::sort_by("Lines", true),
        ])
        .unit(COUNT)
        .decimals(0.0)
        .no_value(none_in_range("card numbers"))
        // Only a journal line has a unit.
        .overrides(vec![blank_when_missing("Unit")])
        .build(0)
}

// --- filter coverage -------------------------------------------------------------

fn lines_scanned(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Lines Scanned and Bypassed")
        .query(q.legended(
            "infra.alloy.compliance.coverage",
            &["{{component_id}} scanned", "{{component_id}} bypassed"],
        ))
        .unit(LINES_PER_SECOND)
        .log_scale(LOG_BASE)
        .decimals(RATE_DECIMALS)
        .no_value(no_gateway())
        .build(0)
}

fn scan_time(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Scan Time per Line (p99)")
        .query(
            q.get("infra.alloy.compliance.scan_time")
                .legend("{{component_id}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(no_gateway())
        .build(0)
}

fn timeouts(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Lines Timed Out and Dropped")
        .query(q.legended(
            "infra.alloy.compliance.timeouts",
            &["{{component_id}} timed out", "{{component_id}} dropped"],
        ))
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(no_gateway())
        .build(0)
}

// --- shadow mode -----------------------------------------------------------------

fn not_in_shadow_mode() -> NoValue {
    NoValue::Custom(
        "No shadow matches in this time range. Outside shadow mode the shadow filter scans nothing."
            .to_string(),
    )
}

fn shadow_by_rule(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Shadow Matches by Rule")
        .query(
            q.get("infra.alloy.compliance.shadow.by_rule")
                .legend("{{rule}}"),
        )
        .unit(REDACTIONS_PER_SECOND)
        .log_scale(LOG_BASE)
        .decimals(RATE_DECIMALS)
        .no_value(not_in_shadow_mode())
        .build(0)
}

fn shadow_by_origin(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Shadow Matches by Origin")
        .query(
            q.get("infra.alloy.compliance.shadow.by_origin")
                .legend("{{origin}}"),
        )
        .unit(REDACTIONS_PER_SECOND)
        .log_scale(LOG_BASE)
        .decimals(RATE_DECIMALS)
        .no_value(not_in_shadow_mode())
        .build(0)
}

fn shadow_table(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Shadow Matches by Rule and Origin (Select Time Range)")
        .query(q.get("infra.alloy.compliance.shadow.table").table_format())
        .transformations(vec![
            transform::organize_full(
                &["Time"],
                &["rule", "origin", "Value"],
                &[("rule", "Rule"), ("origin", "Origin"), ("Value", "Matches")],
            ),
            transform::sort_by("Matches", true),
        ])
        .unit(COUNT)
        .decimals(0.0)
        .no_value(not_in_shadow_mode())
        .build(0)
}

// --- the store -------------------------------------------------------------------

fn stored_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Stored Lines with a Placeholder by Source")
        .query(
            q.logs("infra.alloy.compliance.stored.rate")
                .legend("{{job}} {{unit}}"),
        )
        .unit(LINES_PER_MINUTE)
        .log_scale(LOG_BASE)
        .decimals(RATE_DECIMALS)
        .no_value(none_in_range("stored lines with a placeholder"))
        .build(0)
}

fn stored_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Stored Lines with a Placeholder")
        .query(q.logs("infra.alloy.compliance.stored.stream"))
        .no_value(none_in_range("stored lines with a placeholder"))
        .build(0)
}

fn shadow_copies_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Shadow Copies by Source")
        .query(
            q.logs("infra.alloy.compliance.shadow_copies.rate")
                .legend("{{job}} {{unit}}"),
        )
        .unit(LINES_PER_MINUTE)
        .log_scale(LOG_BASE)
        .decimals(RATE_DECIMALS)
        .no_value(not_in_shadow_mode())
        .build(0)
}

fn shadow_copies_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Shadow Copies")
        .query(q.logs("infra.alloy.compliance.shadow_copies.stream"))
        .no_value(not_in_shadow_mode())
        .build(0)
}

fn partly_scanned_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Partly Scanned Lines by Source")
        .query(
            q.logs("infra.alloy.compliance.partly_scanned.rate")
                .legend("{{job}} {{unit}}"),
        )
        .unit(LINES_PER_MINUTE)
        .min(0.0)
        .no_value(none_in_range("partly scanned lines"))
        .build(0)
}

fn partly_scanned_feed(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("Partly Scanned Lines")
        .query(q.logs("infra.alloy.compliance.partly_scanned.stream"))
        .no_value(none_in_range("partly scanned lines"))
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_queries;
    use mzmon_lib::grafana::layout::Layout;
    use serde_json::Value;

    fn elements() -> serde_json::Map<String, Value> {
        let q = &test_queries();
        let assembled = Layout::rows(rows(q)).assemble().expect("assemble");
        assert!(q.failures().is_empty(), "{:?}", q.failures());
        let json = serde_json::to_value(&assembled.elements).expect("serialize");
        json.as_object().expect("elements").clone()
    }

    fn exprs(panel: &Value) -> Vec<String> {
        panel
            .pointer("/spec/data/spec/queries")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|q| q.pointer("/spec/query/spec/expr").and_then(Value::as_str))
            .map(str::to_string)
            .collect()
    }

    fn group(panel: &Value) -> Option<&str> {
        panel
            .pointer("/spec/data/spec/queries/0/spec/query/group")
            .and_then(Value::as_str)
    }

    /// Timeseries on a linear axis, and why each is not on [`LOG_BASE`].
    const LINEAR: &[(&str, &str)] = &[
        (
            "compliance-scan-time",
            "a percentile per instance, where a climb toward the timeout is the signal",
        ),
        (
            "compliance-timeouts",
            "zero is the healthy reading, and a log axis can't draw it",
        ),
        (
            "compliance-partly-scanned-rate",
            "zero is the healthy reading, and a log axis can't draw it",
        ),
    ];

    #[test]
    fn graphs_watching_for_a_small_rate_use_a_log_axis() {
        let elements = elements();
        for (name, panel) in &elements {
            if panel
                .pointer("/spec/vizConfig/group")
                .and_then(Value::as_str)
                != Some("timeseries")
            {
                continue;
            }
            let scale = panel
                .pointer("/spec/vizConfig/spec/fieldConfig/defaults/custom/scaleDistribution/type")
                .and_then(Value::as_str);
            let linear = LINEAR.iter().any(|(l, _)| l == name);
            assert_eq!(
                scale == Some("log"),
                !linear,
                "{name}: list it in LINEAR with a reason, or put it on a log axis"
            );
        }
        for (name, _) in LINEAR {
            assert!(elements.contains_key(*name), "{name} is not on the tab");
        }
    }

    #[test]
    fn rows_that_read_the_store_start_collapsed() {
        // A Loki panel on an expanded row runs as soon as the tab opens.
        let q = &test_queries();
        let mut checked = 0;
        for row in rows(q) {
            let assembled = Layout::rows(vec![row]).assemble().expect("assemble");
            let elements = serde_json::to_string(&assembled.elements).expect("serialize");
            if !elements.contains(r#""group":"loki""#) {
                continue;
            }
            checked += 1;
            let layout = serde_json::to_string(&assembled.layout).expect("serialize");
            assert!(
                layout.contains(r#""collapse":true"#),
                "a row reading Loki is expanded: {layout}"
            );
        }
        assert_eq!(checked, 3, "the store rows moved");
    }

    #[test]
    fn range_counts_count_a_series_created_in_the_range() {
        // The counters start at 1, so a plain increase() reads a credential
        // logged once as zero.
        let elements = elements();
        for name in [
            "compliance-redacted",
            "compliance-cards",
            "compliance-shadow",
            "compliance-redacted-table",
            "compliance-cards-table",
            "compliance-shadow-table",
        ] {
            for expr in exprs(&elements[name]) {
                assert!(
                    expr.contains("unless") && expr.contains("offset $__range"),
                    "{name}: {expr}"
                );
                assert!(expr.contains("last_over_time"), "{name}: {expr}");
            }
        }
    }

    #[test]
    fn the_card_counter_is_read_by_the_streams_labels() {
        // The counter carries the labels of the stream it counted, and the
        // gateway's scrape keeps them over its own, so a collector picker on
        // `namespace` or `pod` would match the workload's and drop the series.
        let elements = elements();
        for name in [
            "compliance-cards",
            "compliance-cards-by-source",
            "compliance-cards-table",
        ] {
            for expr in exprs(&elements[name]) {
                assert!(!expr.contains("$alloy"), "{name}: {expr}");
                assert!(
                    expr.contains(r#"component_id="loki.process.inputPrefilter""#),
                    "{name}: {expr}"
                );
            }
        }
        for name in ["compliance-cards-by-source", "compliance-cards-table"] {
            for expr in exprs(&elements[name]) {
                assert!(expr.contains("by (job, unit)"), "{name}: {expr}");
            }
        }
    }

    #[test]
    fn the_store_wide_search_is_guarded_and_its_twin_has_no_query() {
        let elements = elements();
        let note = &elements["compliance-stored-hidden-note"];
        assert!(exprs(note).is_empty(), "the note runs a query");
        for name in ["compliance-stored-rate", "compliance-stored-feed"] {
            assert_eq!(group(&elements[name]), Some("loki"), "{name}");
        }
        assert!(STORED_NOTE.contains("7 days"), "{STORED_NOTE}");
        assert!(STORED_NOTE.contains("/explore"), "{STORED_NOTE}");
        let layout = Layout::rows(vec![stored(&test_queries()), stored_hidden()])
            .assemble()
            .expect("assemble");
        // Grafana flags two rows with one title, so the stand-in's differs.
        let json = serde_json::to_string(&layout.layout).expect("serialize");
        assert_eq!(json.matches(STORED_TITLE).count(), 2, "{json}");
        assert_eq!(
            json.matches(&format!(r#""{STORED_TITLE}""#)).count(),
            1,
            "{json}"
        );
    }
}
