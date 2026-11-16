//! Holds client-certificate tenancy to taking the tenant from the header alone.
//!
//! Under `pipeline.gateway.clientCertificateTenancy`, the header the
//! TLS-terminating proxy sets is the only thing that may choose a record's
//! tenant. Everything else a sender controls is a way to forge one:
//! attributes, the label hints the OTLP-to-Loki bridge obeys, resource
//! attributes that become `target_info` labels, and labels pushed to
//! `loki.source.api`. These tests run the tenancy components, the plain OTLP
//! log bridge, `localTenant`, and `inputProcessor` from the pipeline YAML on
//! `alloy run`, send each forgery, and check the tenant every record leaves
//! with, and that records without a usable header are dropped. The header's
//! two formats, an RFC 2253 subject and Envoy's `X-Forwarded-Client-Cert`,
//! each have their own pattern, and both refuse a header that names more than
//! one certificate.
//!
//! It skips when `alloy` is not on `PATH`, unless `MZMON_REQUIRE_ALLOY` is
//! set, as it is in the CI job that pins the binary.

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use common::alloy::{
    Alloy, alloy_available, free_port, http_with_headers, label_set, logfmt_fields,
};
use mzmon_lib::alloy::ast::{AttributeValue, Expression, ToBlock};
use mzmon_lib::alloy::pipeline::Pipeline;
use serde_json::{Value, json};

const HEADER: &str = "X-Amzn-Mtls-Clientcert-Subject";

/// Where `inputProcessor` puts a record with no other tenant.
const STATIC_TENANT: &str = "loki";

fn pipelines_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alloy-pipelines")
}

/// The named components from one pipeline file, rendered.
fn components(file: &str, wanted: &[(&str, &str)]) -> Vec<String> {
    let yaml = std::fs::read_to_string(pipelines_dir().join(file)).expect("read the pipeline");
    let pipeline = Pipeline::from_yaml_str(&yaml).expect("parse the pipeline");
    let blocks: Vec<_> = pipeline
        .blocks
        .iter()
        .map(|b| b.to_block().expect("to_block"))
        .collect();
    wanted
        .iter()
        .map(|(component, label)| {
            let mut block = blocks
                .iter()
                .find(|b| b.component == *component && b.label.as_deref() == Some(*label))
                .unwrap_or_else(|| panic!("{file} defines {component} {label:?}"))
                .clone();
            // The test's sinks stand in for the rest of the gateway.
            if *component == "loki.process" {
                block.attributes.insert(
                    "forward_to".to_string(),
                    AttributeValue::Array(vec![AttributeValue::Expression(
                        Expression::name_to_ref("loki.echo.out.receiver"),
                    )]),
                );
            }
            block.render().expect("render")
        })
        .collect()
}

/// The tenancy receiver forwards to `otelcol.processor.attributes "<processor>"`.
fn config(tenancy_port: u16, plain_port: u16, processor: &str) -> String {
    let mut parts = components(
        "gateway.yaml",
        &[
            ("otelcol.processor.attributes", "clientTenantSubject"),
            ("otelcol.processor.attributes", "clientTenantXfcc"),
            ("otelcol.processor.filter", "clientTenant"),
            ("otelcol.processor.transform", "clientTenant"),
            ("otelcol.exporter.loki", "clientTenant"),
            ("loki.relabel", "clientTenant"),
            ("otelcol.exporter.loki", "bridge"),
            ("loki.relabel", "bridge"),
            ("loki.process", "inputProcessor"),
        ],
    );
    parts.extend(components(
        "gateway-metrics.yaml",
        &[("otelcol.processor.transform", "localTenant")],
    ));
    let gateway = parts
        .join("\n")
        // Skip the prefilter and the secret filters, which never touch labels.
        .replace(
            "loki.process.inputPrefilter.receiver",
            "loki.process.inputProcessor.receiver",
        )
        .replace(
            "otelcol.processor.transform.inputMetricProcessor.input",
            "otelcol.exporter.prometheus.out.input",
        );
    format!(
        r#"otelcol.receiver.otlp "tenancy" {{
    http {{
        endpoint = "127.0.0.1:{tenancy_port}"
        include_metadata = true
    }}

    output {{
        logs = [otelcol.processor.attributes.{processor}.input]
        metrics = [otelcol.processor.attributes.{processor}.input]
    }}
}}

otelcol.receiver.otlp "plain" {{
    http {{
        endpoint = "127.0.0.1:{plain_port}"
    }}

    output {{
        logs = [otelcol.exporter.loki.bridge.input]
        metrics = [otelcol.processor.transform.localTenant.input]
    }}
}}

loki.source.api "in" {{
    http {{
        listen_address = "127.0.0.1"
        listen_port = LOKI_PORT
    }}
    forward_to = [loki.process.inputProcessor.receiver]
}}

otelcol.exporter.prometheus "out" {{
    add_metric_suffixes = false
    forward_to = [prometheus.echo.out.receiver]
}}

prometheus.echo "out" {{}}

loki.echo "out" {{}}

{gateway}"#
    )
}

// --- requests ----------------------------------------------------------------

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn attrs(pairs: &[(&str, Value)]) -> Value {
    Value::Array(
        pairs
            .iter()
            .map(|(k, v)| json!({ "key": k, "value": v }))
            .collect(),
    )
}

fn s(v: &str) -> Value {
    json!({ "stringValue": v })
}

fn log_body(resource: &[(&str, Value)], record: &[(&str, Value)]) -> String {
    json!({ "resourceLogs": [{
        "resource": { "attributes": attrs(resource) },
        "scopeLogs": [{ "logRecords": [{
            "timeUnixNano": now().to_string(),
            "body": s("tenant probe"),
            "attributes": attrs(record),
        }]}],
    }]})
    .to_string()
}

fn metric_body(resource: &[(&str, Value)], point: &[(&str, Value)]) -> String {
    json!({ "resourceMetrics": [{
        "resource": { "attributes": attrs(resource) },
        "scopeMetrics": [{ "metrics": [{
            "name": "tenant_probe",
            "gauge": { "dataPoints": [{
                "timeUnixNano": now().to_string(),
                "asDouble": 1.0,
                "attributes": attrs(point),
            }]},
        }]}],
    }]})
    .to_string()
}

fn post(port: u16, path: &str, headers: &[(&str, &str)], body: &str) {
    let (status, response) =
        http_with_headers(port, "POST", path, headers, body).expect("send the request");
    assert!(
        (200..300).contains(&status),
        "{path} on {port} refused with {status}: {response}"
    );
}

/// A log record labelled `fixture=<id>` through the bridge's label hint.
fn log_fixture(
    id: &str,
    extra_hint: &str,
    resource: &[(&str, Value)],
    record: &[(&str, Value)],
) -> String {
    let mut all = vec![
        ("fixture", s(id)),
        ("loki.attribute.labels", s(&format!("fixture{extra_hint}"))),
    ];
    all.extend(record.iter().cloned());
    log_body(resource, &all)
}

// --- reading what Alloy printed ----------------------------------------------

/// The labels of every entry `loki.echo` printed, by fixture.
fn echoed_logs(alloy: &Alloy) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for line in alloy.log_text().lines() {
        let fields = logfmt_fields(line);
        if fields.get("component_id").map(String::as_str) != Some("loki.echo.out") {
            continue;
        }
        let labels = label_set(fields.get("labels").map_or("", String::as_str));
        if let Some(id) = labels.get("fixture").cloned() {
            out.insert(id, labels);
        }
    }
    out
}

/// The label set of every series `prometheus.echo` printed.
fn echoed_series(alloy: &Alloy) -> Vec<(String, BTreeMap<String, String>)> {
    let mut out = Vec::new();
    for line in alloy.log_text().lines() {
        let fields = logfmt_fields(line);
        if fields.get("component_id").map(String::as_str) != Some("prometheus.echo.out") {
            continue;
        }
        for series in fields.get("metrics").map_or("", String::as_str).lines() {
            if let Some((name, rest)) = series.split_once('{') {
                out.push((name.to_string(), label_set(rest)));
            }
        }
    }
    out
}

#[test]
fn only_the_certificate_header_chooses_a_tenant() {
    if !alloy_available() {
        return;
    }
    let tenancy = free_port();
    let plain = free_port();
    let (alloy, loki_port) = Alloy::start(&config(tenancy, plain, "clientTenantSubject"));
    let subject = |cn: &str| format!("CN={cn},O=Materialize");
    let ok = subject("stack-a");
    let victim = || s("victim");

    // Dropped, so sent first: had any got through, it would arrive before the
    // records the wait below is for.
    post(
        tenancy,
        "/v1/logs",
        &[],
        &log_fixture("no-header", "", &[], &[]),
    );
    post(
        tenancy,
        "/v1/logs",
        &[(HEADER, "CN=../etc,O=Materialize")],
        &log_fixture("bad-cn", "", &[], &[]),
    );
    post(
        tenancy,
        "/v1/logs",
        &[(HEADER, &ok), (HEADER, &subject("victim"))],
        &log_fixture("twice", "", &[], &[]),
    );
    post(
        tenancy,
        "/v1/metrics",
        &[],
        &metric_body(
            &[("service.name", s("probe-noheader"))],
            &[("fixture", s("m-noheader"))],
        ),
    );

    // Logs that take their tenant from the header.
    post(
        tenancy,
        "/v1/logs",
        &[(HEADER, &ok)],
        &log_fixture("ok", "", &[], &[]),
    );
    post(
        tenancy,
        "/v1/logs",
        &[(HEADER, "O=Materialize, CN=stack-c")],
        &log_fixture("cn-not-first", "", &[], &[]),
    );
    post(
        tenancy,
        "/v1/logs",
        &[(HEADER, &ok)],
        &log_fixture(
            "forged-attribute",
            ",mzmonTenant",
            &[],
            &[("mzmonTenant", victim())],
        ),
    );
    post(
        tenancy,
        "/v1/logs",
        &[(HEADER, &ok)],
        &log_fixture(
            "forged-carrier",
            ",__mzmon_tenant__",
            &[],
            &[("__mzmon_tenant__", victim())],
        ),
    );
    post(
        tenancy,
        "/v1/logs",
        &[(HEADER, &ok)],
        &log_fixture(
            "forged-resource",
            "",
            &[("mzmonTenant", victim())],
            &[("loki.resource.labels", s("mzmonTenant"))],
        ),
    );
    post(
        tenancy,
        "/v1/logs",
        &[(HEADER, &subject("stack-b"))],
        &log_body(
            &[],
            &[
                ("fixture", s("list-hint")),
                (
                    "loki.attribute.labels",
                    json!({ "arrayValue": { "values": [s("fixture")] } }),
                ),
            ],
        ),
    );

    // Logs from every other way in, which keep the static tenant.
    post(
        plain,
        "/v1/logs",
        &[],
        &log_fixture(
            "plain-carrier",
            ",__mzmon_tenant__",
            &[],
            &[("__mzmon_tenant__", victim())],
        ),
    );
    post(
        plain,
        "/v1/logs",
        &[(HEADER, &ok)],
        &log_fixture(
            "plain-attribute",
            ",mzmonTenant",
            &[],
            &[("mzmonTenant", victim())],
        ),
    );
    let push = json!({ "streams": [{
        "stream": { "fixture": "api", "__mzmon_tenant__": "victim", "mzmonTenant": "victim" },
        "values": [[now().to_string(), "tenant probe"]],
    }]})
    .to_string();
    post(
        loki_port,
        "/loki/api/v1/push",
        &[("X-Scope-OrgID", "victim")],
        &push,
    );

    // Metrics: the header's tenant on the series and on `target_info`, and
    // none on a series that came another way.
    post(
        tenancy,
        "/v1/metrics",
        &[(HEADER, &ok)],
        &metric_body(
            &[("service.name", s("probe-ok")), ("mzmonTenant", victim())],
            &[("fixture", s("m-ok")), ("mzmonTenant", victim())],
        ),
    );
    post(
        plain,
        "/v1/metrics",
        &[(HEADER, &ok)],
        &metric_body(
            &[
                ("service.name", s("probe-local")),
                ("mzmonTenant", victim()),
            ],
            &[("fixture", s("m-local")), ("mzmonTenant", victim())],
        ),
    );

    let want_logs: BTreeMap<&str, &str> = [
        ("ok", "stack-a"),
        ("cn-not-first", "stack-c"),
        ("forged-attribute", "stack-a"),
        ("forged-carrier", "stack-a"),
        ("forged-resource", "stack-a"),
        ("list-hint", "stack-b"),
        ("plain-carrier", STATIC_TENANT),
        ("plain-attribute", STATIC_TENANT),
        ("api", STATIC_TENANT),
    ]
    .into_iter()
    .collect();
    let probe =
        |series: &[(String, BTreeMap<String, String>)], name: &str, key: &str, value: &str| {
            series
                .iter()
                .find(|(n, l)| n == name && l.get(key).map(String::as_str) == Some(value))
                .map(|(_, l)| l.clone())
        };

    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let logs = echoed_logs(&alloy);
        let series = echoed_series(&alloy);
        let arrived = want_logs.keys().all(|id| logs.contains_key(*id))
            && probe(&series, "tenant_probe", "fixture", "m-ok").is_some()
            && probe(&series, "tenant_probe", "fixture", "m-local").is_some();
        if arrived {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "not every record came out:\n{}",
            alloy.log_text()
        );
        std::thread::sleep(Duration::from_millis(250));
    }
    // Room for a dropped record that was only slow.
    std::thread::sleep(Duration::from_secs(1));
    let logs = echoed_logs(&alloy);
    let series = echoed_series(&alloy);

    let mut failures = Vec::new();
    for (id, tenant) in &want_logs {
        let labels = &logs[*id];
        let got = labels.get("__tenant_id__").map_or("<none>", String::as_str);
        if got != *tenant {
            failures.push(format!("{id}: tenant {got}, want {tenant}"));
        }
        if labels.contains_key("__mzmon_tenant__") {
            failures.push(format!("{id}: left with the __mzmon_tenant__ carrier"));
        }
    }
    for id in ["no-header", "bad-cn", "twice"] {
        if let Some(labels) = logs.get(id) {
            failures.push(format!("{id}: should be dropped, came out as {labels:?}"));
        }
    }

    let tenant_of = |labels: Option<BTreeMap<String, String>>| {
        labels.map(|l| {
            l.get("mzmonTenant")
                .cloned()
                .unwrap_or_else(|| "<none>".into())
        })
    };
    let checks = [
        (
            "tenant_probe m-ok",
            tenant_of(probe(&series, "tenant_probe", "fixture", "m-ok")),
            Some("stack-a"),
        ),
        (
            "target_info probe-ok",
            tenant_of(probe(&series, "target_info", "job", "probe-ok")),
            Some("stack-a"),
        ),
        (
            "tenant_probe m-local",
            tenant_of(probe(&series, "tenant_probe", "fixture", "m-local")),
            Some("<none>"),
        ),
    ];
    for (what, got, want) in checks {
        if got.as_deref() != want {
            failures.push(format!("{what}: mzmonTenant {got:?}, want {want:?}"));
        }
    }
    for (name, labels) in &series {
        if labels.get("job").map(String::as_str) == Some("probe-local")
            && labels.contains_key("mzmonTenant")
        {
            failures.push(format!(
                "{name} from probe-local carries mzmonTenant: {labels:?}"
            ));
        }
        if labels.get("fixture").map(String::as_str) == Some("m-noheader")
            || labels.get("job").map(String::as_str) == Some("probe-noheader")
        {
            failures.push(format!("{name}: should be dropped, came out as {labels:?}"));
        }
    }

    assert!(
        failures.is_empty(),
        "\n{}\n\n{}",
        failures.join("\n"),
        alloy.log_text()
    );
}

// --- formats and header names --------------------------------------------------

/// A log record's id, the headers it is sent with, and the tenant it should
/// leave with, or `None` where it should be dropped.
type Case<'a> = (&'a str, &'a [(&'a str, &'a str)], Option<&'a str>);

/// Sends one log record per case to a tenancy receiver that forwards to
/// `processor`, and checks the tenant each leaves with.
fn check_log_tenants(processor: &str, env: &[(&str, &str)], cases: &[Case]) {
    if !alloy_available() {
        return;
    }
    let tenancy = free_port();
    let plain = free_port();
    let (alloy, _) = Alloy::start_with_env(&config(tenancy, plain, processor), env);

    // Dropped records first, as in the forgery test.
    let (dropped, kept): (Vec<&Case>, Vec<&Case>) = cases.iter().partition(|c| c.2.is_none());
    for (id, headers, _) in dropped.iter().chain(kept.iter()) {
        post(tenancy, "/v1/logs", headers, &log_fixture(id, "", &[], &[]));
    }

    let deadline = Instant::now() + Duration::from_secs(30);
    while !kept.iter().all(|c| echoed_logs(&alloy).contains_key(c.0)) {
        assert!(
            Instant::now() < deadline,
            "not every record came out:\n{}",
            alloy.log_text()
        );
        std::thread::sleep(Duration::from_millis(250));
    }
    std::thread::sleep(Duration::from_secs(1));
    let logs = echoed_logs(&alloy);

    let mut failures = Vec::new();
    for (id, _, want) in cases {
        let got = logs
            .get(*id)
            .map(|l| l.get("__tenant_id__").map_or("<none>", String::as_str));
        if got != *want {
            failures.push(format!("{id}: tenant {got:?}, want {want:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "\n{}\n\n{}",
        failures.join("\n"),
        alloy.log_text()
    );
}

const XFCC: &str = "X-Forwarded-Client-Cert";

#[test]
fn xfcc_takes_the_subject_of_exactly_one_certificate() {
    let ok = "By=spiffe://gateway.example;Hash=0123abcd;Subject=\"CN=stack-a,O=Materialize\";URI=spiffe://c.materialize.cloud/stack/stack-a";
    check_log_tenants(
        "clientTenantXfcc",
        &[("GATEWAY_CLIENT_TENANT_HEADER", "x-forwarded-client-cert")],
        &[
            ("ok", &[(XFCC, ok)], Some("stack-a")),
            (
                "subject-only",
                &[(XFCC, "Subject=\"O=Materialize,CN=stack-c\"")],
                Some("stack-c"),
            ),
            (
                "quoted-comma",
                &[(XFCC, "URI=\"spiffe://example/a,b\";Subject=\"CN=stack-b\"")],
                Some("stack-b"),
            ),
            // Envoy's APPEND_FORWARD, or a client's element ahead of Envoy's.
            (
                "appended",
                &[(
                    XFCC,
                    "By=spiffe://client;Subject=\"CN=victim\",By=spiffe://gateway.example;Hash=ab;Subject=\"CN=stack-a\"",
                )],
                None,
            ),
            ("twice", &[(XFCC, ok), (XFCC, ok)], None),
            (
                "no-subject",
                &[(
                    XFCC,
                    "By=spiffe://gateway.example;Hash=ab;URI=spiffe://c.materialize.cloud/stack/stack-a",
                )],
                None,
            ),
            ("rfc2253", &[(XFCC, "CN=stack-a,O=Materialize")], None),
            (
                "escaped-comma",
                &[(XFCC, "Subject=\"CN=stack-a,O=dept\\,CN=victim\"")],
                None,
            ),
            (
                "two-cns",
                &[(XFCC, "Subject=\"CN=victim,CN=stack-a\"")],
                None,
            ),
            ("no-header", &[], None),
        ],
    );
}

#[test]
fn the_header_name_comes_from_the_environment() {
    check_log_tenants(
        "clientTenantSubject",
        &[("GATEWAY_CLIENT_TENANT_HEADER", "x-ssl-client-s-dn")],
        &[
            (
                "nginx",
                &[("X-SSL-Client-S-DN", "CN=stack-a,O=Materialize")],
                Some("stack-a"),
            ),
            (
                "default-header",
                &[(HEADER, "CN=victim,O=Materialize")],
                None,
            ),
        ],
    );
}

#[test]
fn a_subject_names_exactly_one_unescaped_cn() {
    check_log_tenants(
        "clientTenantSubject",
        &[],
        &[
            (
                "other-attributes",
                &[(HEADER, "C=US,O=Materialize,OU=Cloud,CN=stack-d")],
                Some("stack-d"),
            ),
            (
                "numeric-type",
                &[(HEADER, "CN=stack-e,0.9.2342.19200300.100.1.1=x")],
                Some("stack-e"),
            ),
            // The organization's escaped comma hides a second CN.
            (
                "escaped-comma",
                &[(HEADER, "CN=stack-a,O=department\\,CN=victim")],
                None,
            ),
            ("two-cns", &[(HEADER, "CN=victim,CN=stack-a")], None),
            ("lowercase-cn", &[(HEADER, "CN=stack-a,cn=victim")], None),
            ("no-cn", &[(HEADER, "O=Materialize,OU=stack-a")], None),
        ],
    );
}
