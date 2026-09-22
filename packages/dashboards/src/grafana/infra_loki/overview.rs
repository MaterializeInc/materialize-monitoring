// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Overview tab: is the log store working, and is this dashboard even
//! looking at it.
//!
//! Two questions, in that order, and the second is not rhetorical. Every panel on
//! the other four tabs reads a metric that arrives by being scraped, so a
//! collection failure and a healthy quiet render identically. The scrape row is
//! here to tell them apart, and it earns its place at the top: the fault it
//! catches is the one that had been live on every mTLS install until the
//! plaintext-exporter split landed.
//!
//! The canary leads because it is the only signal here that is end-to-end. Every
//! other panel measures one stage and infers the rest; the canary writes a line
//! and reads it back, so it is wrong only when the store really is.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::BigValueGraphMode;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::theme;
use crate::grafana::queries::Queries;
use crate::grafana::transform;

const SHADE: &str = theme::OVERVIEW.shade;

/// Grafana has no unit for log lines, so these are its custom-suffix form.
pub(super) const LINES_PER_SECOND: &str = "suffix:lines/s";

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![verdict(q), collection(q), requests(q), canary(q)]
}

/// The five numbers worth reading before anything else.
///
/// Header hidden: this row is the dashboard's answer, not a section of it.
/// Half height, because these are read at a glance and at full height they push
/// the scrape row below the fold — which is the one thing that has to be seen
/// before any of the rest is believed.
fn verdict(q: &Queries) -> Row {
    Row::new("End to End").hide_header().grid(
        AutoGrid::new(5)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("overview-canary-missing", canary_missing(q))
            .panel("overview-canary-latency", canary_latency_stat(q))
            .panel("overview-ingest", ingest(q))
            .panel("overview-client-error-rate", client_error_rate(q))
            .panel("overview-error-rate", error_rate(q)),
    )
}

fn collection(q: &Queries) -> Row {
    Row::new("Collection Health").grid(
        AutoGrid::new(2)
            .panel("overview-scrape", scrape_health(q))
            .panel("overview-restarts", restarts(q)),
    )
}

fn requests(q: &Queries) -> Row {
    Row::new("Requests").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("overview-latency-by-route", latency_by_route(q))
            .panel("overview-failures", failures(q)),
    )
}

fn canary(q: &Queries) -> Row {
    Row::new("Canary").grid(
        AutoGrid::new(2)
            .panel("overview-canary-roundtrip", canary_roundtrip(q))
            .panel("overview-canary-losses", canary_losses(q)),
    )
}

/// The headline. Zero is the only healthy reading, so it is not shaded with the
/// tab colour — a health number gets health colours.
fn canary_missing(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Canary Lines Lost")
        .query(q.get("infra.loki.health.canary.missing").legend("lost/s"))
        .graph_mode(BigValueGraphMode::Area)
        .unit(LINES_PER_SECOND)
        .min(0.0)
        // Absent is a different finding from zero, and the two must not read the
        // same. Zero means the round trip is clean; absent means the canary is
        // not being collected and this panel knows nothing.
        .no_value(NoValue::Custom(
            "Canary not collected — check Collection Health below".to_string(),
        ))
        .build(0)
}

fn canary_latency_stat(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Canary Round Trip (p99)")
        .query(q.legended("infra.loki.health.canary.latency", &["p50", "p99"]))
        .reduce_fields("/p99/")
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "Canary not collected — check Collection Health below".to_string(),
        ))
        .build(0)
}

fn ingest(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Lines Ingested")
        .query(q.get("infra.loki.write.lines").legend("lines/s"))
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(NoValue::Custom(
            "Nothing is reaching the distributor".to_string(),
        ))
        .build(0)
}

/// Not a health number the way 5xx is, which is why it carries the tab shade
/// rather than sitting beside 5xx as a second alarm.
///
/// A little above zero is the resting state — a query for a label that no longer
/// exists, a client cancelling a slow request. What is worth reading is a
/// *step*: a rejected write is a 4xx from the client's side, so this moves
/// before the discard counters on the Writes tab do, and it moves when the
/// caller is at fault rather than the store.
fn client_error_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("4xx Rate")
        .query(q.get("infra.loki.health.client_errors").legend("4xx share"))
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit("percentunit")
        .min(0.0)
        .no_value(NoValue::Custom("No requests in this range".to_string()))
        .build(0)
}

fn error_rate(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("5xx Rate")
        .query(
            q.get("infra.loki.health.request_errors")
                .legend("5xx share"),
        )
        .graph_mode(BigValueGraphMode::Area)
        .unit("percentunit")
        .min(0.0)
        // Zero divided by zero when nothing is being asked of the store, which
        // is a real and healthy state on a quiet cluster.
        .no_value(NoValue::Custom("No requests in this range".to_string()))
        .build(0)
}

/// The panel that says whether the rest of the dashboard can be believed.
///
/// `instant`, from the registry: this is a current fact, and evaluated over a
/// range each target repeats once per scrape.
fn scrape_health(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Scrape Health")
        // `table_format`, then drop `Time` — which together are the whole of
        // what this panel was missing. Prometheus returns one *frame* per
        // series, and a Table handed several frames renders a frame **picker**
        // rather than a table: eleven one-row tables behind a dropdown, each
        // keyed by its scrape timestamp. The first frame renders correctly,
        // which is what makes it easy to ship.
        .query(q.get("infra.loki.health.up").table_format())
        .transformations(vec![transform::organize(
            &["Time"],
            &["container", "service", "Value"],
        )])
        .unit("short")
        .no_value(NoValue::Custom(
            "No Loki targets found at all — check the namespace picker".to_string(),
        ))
        .build(0)
}

fn restarts(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Container Restarts")
        .query(q.get("infra.loki.health.restarts").legend("{{pod}}"))
        .unit("short")
        .no_value(NoValue::Custom(
            "No Loki container has restarted in this range".to_string(),
        ))
        .build(0)
}

fn latency_by_route(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Request Latency by Route (p99)")
        .query(
            q.get("infra.loki.health.request_latency")
                .legend("{{route}}"),
        )
        .unit("s")
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn failures(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Failures Without a Status Code")
        .query(
            q.get("infra.loki.health.request_failures")
                .legend("{{container}}"),
        )
        .unit("reqps")
        .min(0.0)
        .no_value(NoValue::Custom(
            "No requests failed without an HTTP status.".to_string(),
        ))
        .build(0)
}

fn canary_roundtrip(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Canary Round Trip")
        .query(q.legended("infra.loki.health.canary.latency", &["p50", "p99"]))
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom("Canary not collected".to_string()))
        .build(0)
}

/// Both loss counters on one panel, because they answer different questions
/// about the same thing and the pair is the reading.
///
/// Missing entries means a line never arrived. Spot-check misses mean a line
/// arrived, was readable, and later stopped being — which is retention or
/// storage rather than ingestion. Apart, each is a number; together they say
/// which half of the store to look at.
fn canary_losses(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Canary Losses")
        .query(q.legended(
            "infra.loki.health.canary.losses",
            &["missing on write", "missing on re-read"],
        ))
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(NoValue::Custom(
            "No canary losses in this range".to_string(),
        ))
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_queries;

    #[test]
    fn the_tab_assembles_with_every_panel_placed() {
        let q = &test_queries();
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows(q))
            .assemble()
            .expect("assemble");
        assert_eq!(assembled.elements.len(), 11);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn the_scrape_panel_renders_as_one_table() {
        // Without `table_format` Prometheus returns a frame per series and the
        // Table becomes a dropdown of eleven one-row tables, each keyed by its
        // scrape timestamp. The first renders correctly, so this is invisible
        // until someone looks for the target that is actually down.
        let q = &test_queries();
        let panel = scrape_health(q);
        let json = serde_json::to_string(&panel).expect("serialize");
        assert!(json.contains(r#""format":"table""#), "{json}");
        assert!(json.contains("organize"), "{json}");
        assert!(json.contains(r#""Time":true"#), "{json}");
    }

    #[test]
    fn the_scrape_panel_is_an_instant_query() {
        // A range query repeats each target once per scrape and the table
        // becomes unreadable. The registry marks it instant; this asserts the
        // dashboard did not lose that on the way through.
        let q = &test_queries();
        let panel = scrape_health(q);
        let json = serde_json::to_string(&panel).expect("serialize");
        assert!(json.contains(r#""instant":true"#), "{json}");
    }

    #[test]
    fn the_canary_stats_say_so_when_the_canary_is_absent() {
        // The whole point of the mTLS fix: a canary that is not collected has to
        // read differently from a canary reporting no losses, or the dashboard
        // shows green for the one failure it exists to catch.
        let q = &test_queries();
        for panel in [canary_missing(q), canary_latency_stat(q)] {
            let json = serde_json::to_string(&panel).expect("serialize");
            assert!(json.contains("not collected"), "{json}");
        }
    }
}
