// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Ingest tab: what pushes into the gateway.
//!
//! The gateway serves three push listeners: log push on 3100, which the agents
//! use, remote write on 9090, and OTLP on 4317 and 4318. They share one server
//! configuration, one TLS setup and one NetworkPolicy, so they fail together
//! and are read together here, one row per listener.
//!
//! # Three vantage points, because no one of them sees everything
//!
//! * **The gateway's request counters** see every request that reached the HTTP
//!   layer, with its status. They are the primary signal.
//! * **The senders' own queues** see what the gateway cannot: a sender that
//!   never connects reaches no gateway counter at all. Remote-write senders
//!   this stack scrapes publish `prometheus_remote_storage_*` with the URL they
//!   write to, which is what the senders panel reads.
//! * **The gateway's own log** is the only record of a refused TLS handshake,
//!   which fails before any request exists. That is the Connections row.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::BigValueGraphMode;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::overview::{self, LINES_PER_SECOND, SAMPLES_PER_SECOND};
use super::theme;
use crate::grafana::queries::Queries;

const SHADE: &str = theme::INGEST.shade;

/// What a remote-write panel shows when nothing writes to the gateway.
fn no_remote_writers() -> NoValue {
    NoValue::Custom("Nothing is remote-writing to the gateway".to_string())
}

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![
        flow(q),
        log_push(q),
        remote_write(q),
        otlp(q),
        connections(q),
    ]
}

fn flow(q: &Queries) -> Row {
    Row::new("Flow").hide_header().grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Narrow)
            .row_height(RowHeight::Short)
            .panel("ingest-flow-lines", lines_received(q))
            .panel("ingest-flow-samples", samples_received(q))
            .panel("ingest-flow-refused", overview::pushes_refused(q)),
    )
}

fn log_push(q: &Queries) -> Row {
    Row::new("Log Push").grid(
        AutoGrid::new(2)
            .panel("ingest-log-requests", log_push_requests(q))
            .panel("ingest-log-latency", log_push_latency(q)),
    )
}

fn remote_write(q: &Queries) -> Row {
    Row::new("Remote Write").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("ingest-rw-requests", remote_write_requests(q))
            .panel("ingest-rw-samples", remote_write_samples(q))
            .panel("ingest-rw-latency", remote_write_latency(q))
            .panel("ingest-rw-senders", senders(q)),
    )
}

fn otlp(q: &Queries) -> Row {
    Row::new("OTLP").grid(
        AutoGrid::new(2)
            .panel("ingest-otlp-metrics", otlp_metrics(q))
            .panel("ingest-otlp-logs", otlp_logs(q)),
    )
}

fn connections(q: &Queries) -> Row {
    Row::new("Connections").grid(AutoGrid::new(1).panel("ingest-tls-rejections", tls_rejections(q)))
}

fn lines_received(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Log Lines Received")
        .query(q.get("infra.alloy.ingest.log_push.lines").legend("lines/s"))
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit(LINES_PER_SECOND)
        .min(0.0)
        .no_value(NoValue::Custom(
            "Nothing is pushing logs to the gateway".to_string(),
        ))
        .build(0)
}

fn samples_received(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Remote-Write Samples Received")
        .query(
            q.get("infra.alloy.ingest.remote_write.received")
                .legend("samples/s"),
        )
        .graph_mode(BigValueGraphMode::Area)
        .shade(SHADE)
        .unit(SAMPLES_PER_SECOND)
        .min(0.0)
        .no_value(no_remote_writers())
        .build(0)
}

fn log_push_requests(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Log Push Requests by Status")
        .query(
            q.get("infra.alloy.ingest.log_push.requests")
                .legend("{{route}} {{status_code}}"),
        )
        .unit("reqps")
        .min(0.0)
        .no_value(NoValue::Custom(
            "Nothing is pushing logs to the gateway".to_string(),
        ))
        .build(0)
}

fn log_push_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Log Push Latency (p99)")
        .query(q.get("infra.alloy.ingest.log_push.latency").legend("p99"))
        .unit("s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "Nothing is pushing logs to the gateway".to_string(),
        ))
        .build(0)
}

fn remote_write_requests(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Remote-Write Requests by Status")
        .query(
            q.get("infra.alloy.ingest.remote_write.requests")
                .legend("{{route}} {{status_code}}"),
        )
        .unit("reqps")
        .min(0.0)
        .no_value(no_remote_writers())
        .build(0)
}

fn remote_write_samples(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Remote-Write Samples")
        .query(q.legended(
            "infra.alloy.ingest.remote_write.samples",
            &["accepted", "dropped: invalid labels"],
        ))
        .unit(SAMPLES_PER_SECOND)
        .min(0.0)
        .no_value(no_remote_writers())
        .build(0)
}

fn remote_write_latency(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Remote-Write Latency (p99)")
        .query(
            q.get("infra.alloy.ingest.remote_write.latency")
                .legend("p99"),
        )
        .unit("s")
        .min(0.0)
        .no_value(no_remote_writers())
        .build(0)
}

/// The senders' side of the same traffic, read off their own queues.
fn senders(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Remote-Write Senders")
        .query(q.legended(
            "infra.alloy.ingest.remote_write.senders",
            &["{{job}} sent", "{{job}} failed", "{{job}} retried"],
        ))
        .unit(SAMPLES_PER_SECOND)
        .min(0.0)
        .no_value(NoValue::Custom(
            "No scraped sender is remote-writing to the gateway".to_string(),
        ))
        .build(0)
}

fn otlp_metrics(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("OTLP Metric Points")
        .query(q.legended("infra.alloy.ingest.otlp.metrics", &["accepted", "refused"]))
        .unit("suffix:points/s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "Nothing has sent metrics to the gateway over OTLP".to_string(),
        ))
        .build(0)
}

fn otlp_logs(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("OTLP Log Records")
        .query(q.legended("infra.alloy.ingest.otlp.logs", &["accepted", "refused"]))
        .unit("suffix:records/s")
        .min(0.0)
        .no_value(NoValue::Custom(
            "Nothing has sent logs to the gateway over OTLP".to_string(),
        ))
        .build(0)
}

fn tls_rejections(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Rejected TLS Handshakes")
        .query(
            q.logs("infra.alloy.ingest.tls_rejections")
                .legend("{{app}}"),
        )
        .unit("suffix:handshakes/min")
        .min(0.0)
        // Read from the gateway's own log, which travels through the gateway,
        // so an empty panel is only reassuring if that log is arriving.
        .no_value(NoValue::Custom(
            "No refused handshakes. If the Logs tab is empty too, the gateway's log is not arriving."
                .to_string(),
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
        assert_eq!(assembled.elements.len(), 12);
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    #[test]
    fn the_senders_panel_reads_the_senders_not_the_gateway() {
        // A sender that cannot connect reaches no gateway counter, so this
        // panel is only worth having if it reads the sender's own series.
        let q = &test_queries();
        let json = serde_json::to_string(&senders(q)).expect("serialize");
        assert!(json.contains("alloy-gateway"), "{json}");
        assert!(!json.contains("$alloyRole"), "{json}");
    }
}
