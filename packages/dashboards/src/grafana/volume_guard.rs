// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The time-range guard on log-volume rows, shared by every logs dashboard.
//!
//! # Why a guard at all
//!
//! Counting log *lines* means reading every one of them. Loki indexes labels,
//! not line counts, so a `rate()` or `count_over_time()` over a wide selection
//! decompresses the whole span. Measured against a representative cluster, one
//! such panel scans roughly:
//!
//! | Range | Scanned | Exec |
//! |---|---|---|
//! | 6h | 0.7 GB | 0.5s |
//! | 24h | 1.8 GB | 0.9s |
//! | 7d | 27 GB | 11s |
//! | 30d | 95 GB | 45s |
//!
//! Linear in the range, and a week is where it stops being a query and starts
//! being a cost. The log *feeds* on the same tabs are unaffected — they stop at
//! the first page of matches, so they answer in well under a second at any range.
//!
//! # Why a pair of rows
//!
//! Grafana's conditional rendering hides a row outright, and the space it leaves
//! says nothing: a reader cannot tell "too expensive to draw" from "this
//! dashboard is broken". So a guarded row is always paired with one carrying the
//! opposite condition and a text panel that explains the absence and says how to
//! get the panels back.
//!
//! [`Row::only_within`] and [`Row::only_beyond`] are exact complements at the
//! same threshold, so precisely one of the pair is on screen at any range.
//!
//! # Precedent
//!
//! This is the first conditional rendering in the repo. Applied to a whole *row*
//! rather than to individual panels, because the explanation belongs beside the
//! thing it replaces, and a row is the smallest unit that can carry both.

use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::Panel;

/// The range past which volume panels stop drawing themselves.
///
/// Chosen from the measurements in the module docs: a week is the last range
/// that answers in seconds. A guard rather than a limit — every picker still
/// narrows the cost, and scoping to one deployment's namespaces cuts a month from
/// 95 GB to 16 GB.
pub const THRESHOLD: &str = "7d";

/// The note shown in place of the volume panels.
const NOTE: &str = "**Volume panels are hidden for ranges longer than 7 days.**\n\n\
     Counting log lines means reading every one of them — Loki indexes labels, not counts — so \
     these panels scan the whole selection rather than an index. On a representative cluster that \
     is around 95 GB for a month against under 2 GB for a day.\n\n\
     Shorten the time range, or narrow the namespace and app pickers, and they come back. The \
     feeds below are unaffected at any range: they stop at the first page of matches. For counting \
     over longer spans, [Explore](/explore) is the better tool.";

/// The row shown in place of a volume row past [`THRESHOLD`].
///
/// `element` is the layout key for its text panel, which must be unique within
/// the dashboard — hence a parameter rather than a constant, since two tabs of
/// one dashboard may each guard a volume row.
pub fn hidden_row(element: &str) -> Row {
    Row::new("Volume")
        .only_beyond(THRESHOLD)
        .hide_header()
        .grid(
            AutoGrid::new(1).row_height(RowHeight::Short).panel(
                element,
                Panel::text("Volume", NOTE)
                    .description("Why the volume panels are not drawn over long ranges.")
                    .build(0),
            ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_note_says_how_to_get_the_panels_back() {
        // A hidden row that only says "hidden" leaves the reader stuck. The point
        // of the stand-in is the remedy, not the announcement.
        assert!(NOTE.contains("narrow"), "{NOTE}");
        assert!(NOTE.contains("time range"), "{NOTE}");
        assert!(NOTE.contains("/explore"), "{NOTE}");
        // And it must name the threshold it is enforcing.
        assert!(NOTE.contains("7 days"), "{NOTE}");
    }

    #[test]
    fn the_stand_in_carries_no_query() {
        // It exists to explain an absence. A query here would run the very cost
        // the guard is avoiding.
        let layout = mzmon_lib::grafana::layout::Layout::rows(vec![hidden_row("note")])
            .assemble()
            .expect("assemble");
        for (name, element) in &layout.elements {
            let mzmon_lib::grafana::generated::dashboardv2::Element::PanelKind(p) = element else {
                continue;
            };
            assert_eq!(p.spec.viz_config.group, "text", "{name}");
            assert!(p.spec.data.spec.queries.is_empty(), "{name} runs a query");
        }
    }
}
