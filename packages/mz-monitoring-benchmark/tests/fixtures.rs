// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Checks on the committed fixtures, so a regenerated set cannot land with an
//! identifier in it or with a shape `gen-load` cannot serve.

use std::collections::HashSet;

use mz_monitoring_benchmark::expo;
use mz_monitoring_benchmark::fixture::{EntityKind, Part};
use mz_monitoring_benchmark::model::{Model, Sizing};
use mz_monitoring_benchmark::role::{Role, load_fixtures};
use mz_monitoring_benchmark::scenario::MetricScenario;
use mz_monitoring_benchmark::seed::Seed;

/// What the anonymizer must have removed, checked against the literal text of
/// every label value. Placeholders are skipped; they are the point.
#[test]
fn committed_fixtures_carry_no_identifiers() {
    let patterns = [
        (r"\b\d{1,3}(?:\.\d{1,3}){3}\b", "an IPv4 address"),
        (
            r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
            "a UUID",
        ),
        (r"^u\d+$", "a raw user ID"),
        (r"@", "an email address"),
        (
            r"\.svc\b|\.cluster\.local|\.internal\b|\.com\b|\.dev\b",
            "a host name",
        ),
        (r"\bmz[0-9a-z]{10}\b", "an environment resource ID"),
        (
            r"\b(?:aws|gcp|azure)\b|us-east|us-west|eu-west|europe-|asia-",
            "a cloud region",
        ),
    ]
    .map(|(re, what)| (regex::Regex::new(re).unwrap(), what));

    for role in Role::ALL {
        for f in load_fixtures(role, None).unwrap() {
            for fam in &f.families {
                for s in &fam.series {
                    for (k, t) in &s.labels {
                        for part in &t.parts {
                            let Part::Lit(lit) = part else { continue };
                            for (re, what) in &patterns {
                                assert!(
                                    !re.is_match(lit),
                                    "{role} {} {} {k}={lit:?} looks like {what}",
                                    f.path,
                                    s.name
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The example from the design conversation, at full size: every body parses,
/// and no series is emitted twice.
#[test]
fn every_role_renders_valid_unique_series_at_scale() {
    for role in Role::ALL {
        let fixtures = load_fixtures(role, None).unwrap();
        let names = |k: EntityKind| {
            fixtures
                .iter()
                .flat_map(|f| &f.families)
                .flat_map(|f| &f.series)
                .flat_map(|s| s.labels.iter().flat_map(|(_, t)| t.placeholders()))
                .any(|p| p.kind == k)
        };
        let sizing = Sizing {
            shards: names(EntityKind::Shard).then_some(400),
            collections: names(EntityKind::Collection).then_some(3000),
            clusters: names(EntityKind::Cluster).then_some(6),
            replicas_per_cluster: names(EntityKind::Replica).then_some(2),
        };
        let model = Model::new(
            role,
            &fixtures,
            &sizing,
            if names(EntityKind::Collection) {
                MetricScenario::parse("dist=pulse180,collection-grow=200").unwrap()
            } else {
                MetricScenario::parse("dist=pulse180").unwrap()
            },
            Seed::new(&format!("bench-{role}-0")),
            0.1,
            1_800_000_000.0,
        )
        .unwrap_or_else(|e| panic!("{role}: {e:#}"));
        let paths: Vec<String> = model.paths().map(str::to_string).collect();
        for path in paths {
            let body = model.render(&path, 1_800_000_090.0).unwrap();
            let doc = expo::parse(&body).unwrap_or_else(|e| panic!("{role} {path}: {e:#}"));
            let mut seen = HashSet::new();
            for s in doc.families.iter().flat_map(|f| &f.samples) {
                assert!(
                    seen.insert((s.name.clone(), s.labels.clone())),
                    "{role} {path} emits {} {:?} twice",
                    s.name,
                    s.labels
                );
                assert!(
                    expo::parse_f64(&s.value).is_some(),
                    "{role} {path} {} has value {:?}",
                    s.name,
                    s.value
                );
            }
        }
    }
}

/// A restarted pod with the same seed serves the same identities.
#[test]
fn same_seed_same_scrape_shape() {
    let render = || {
        let fixtures = load_fixtures(Role::Clusterd, None).unwrap();
        let m = Model::new(
            Role::Clusterd,
            &fixtures,
            &Sizing {
                shards: Some(50),
                ..Sizing::default()
            },
            MetricScenario::default(),
            Seed::new("bench-clusterd-3"),
            0.1,
            1_800_000_000.0,
        )
        .unwrap();
        let body = m.render("/metrics", 1_800_000_030.0).unwrap();
        body.lines()
            .filter(|l| !l.starts_with('#'))
            .map(|l| l.rsplit_once(' ').unwrap().0.to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(render(), render());
}
