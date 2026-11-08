//! Runs the shipped secret-filter rules through the Alloy binary.
//!
//! `loki.secretfilter` loads Gitleaks rules at run time, and the default set
//! it extends changes with each Alloy release, so the only test that means
//! anything runs the pinned binary. Each test starts `alloy run` on a small
//! config, pushes `ingest.corpus.yaml` to a `loki.source.api`, and reads what
//! comes back: the per-rule counter, keyed by the fixture through
//! `origin_label`, and the line `loki.echo` prints.
//!
//! The tests skip when `alloy` is not on `PATH`, unless `MZMON_REQUIRE_ALLOY`
//! is set, as it is in the CI job that pins the binary.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use regex::Regex;
use serde::Deserialize;

fn secretfilter_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alloy-pipelines/secretfilter")
}

fn ingest_rules() -> PathBuf {
    secretfilter_dir().join("ingest.toml")
}

// --- the corpus --------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    id: String,
    line: String,
    #[serde(default)]
    rules: Vec<String>,
    #[serde(default)]
    secret: Option<String>,
}

/// Fills `<<NAME>>` placeholders with values made here, so no provider-shaped
/// credential is committed. A fixed seed keeps a failure reproducible.
struct Generated {
    values: BTreeMap<&'static str, String>,
}

impl Generated {
    fn new() -> Self {
        let mut state: u64 = 0x5eed_5ec7_e7f1_17e5;
        let mut pick = |alphabet: &str, n: usize| -> String {
            let alphabet: Vec<char> = alphabet.chars().collect();
            (0..n)
                .map(|_| {
                    // xorshift64
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    alphabet[(state % alphabet.len() as u64) as usize]
                })
                .collect()
        };
        const ALNUM: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        const HEX: &str = "0123456789abcdef";
        // Clear of the 3.5-bit entropy floor in upstream's generic-api-key, so
        // a fixture's result does not depend on the draw. The corpus pins a
        // UUID below the floor separately.
        let uuid = loop {
            let h = pick(HEX, 32);
            let uuid = format!(
                "{}-{}-4{}-a{}-{}",
                &h[0..8],
                &h[8..12],
                &h[13..16],
                &h[17..20],
                &h[20..32]
            );
            if shannon_entropy(&uuid) > 3.7 {
                break uuid;
            }
        };
        let jwt = |payload: usize, pick: &mut dyn FnMut(&str, usize) -> String| {
            format!(
                "eyJ{}.eyJ{}.{}",
                pick(ALNUM, 33),
                pick(ALNUM, payload),
                pick(ALNUM, 43)
            )
        };
        let values = BTreeMap::from([
            (
                "AWS_ACCESS_KEY_ID",
                format!("AKIA{}", pick("ABCDEFGHIJKLMNOPQRSTUVWXYZ234567", 16)),
            ),
            ("JWT", jwt(60, &mut pick)),
            // A Materialize license key is a JWT, three dot-separated base64url
            // sections, with a long payload.
            ("LICENSE_KEY", jwt(400, &mut pick)),
            ("PEM_HEADER", ["-----BEGIN ", "PRIVATE KEY-----"].concat()),
            ("MZP_TOKEN", format!("mzp_{}", pick(ALNUM, 40))),
            ("UUID", uuid),
            ("ALNUM32", pick(ALNUM, 32)),
            ("ALNUM40", pick(ALNUM, 40)),
            ("HEX16", pick(HEX, 16)),
            ("HEX32", pick(HEX, 32)),
            ("HEX40", pick(HEX, 40)),
            ("HEX64", pick(HEX, 64)),
        ]);
        Generated { values }
    }

    fn fill(&self, text: &str) -> String {
        let placeholder = Regex::new(r"<<([A-Z0-9_]+)>>").unwrap();
        let filled = placeholder.replace_all(text, |c: &regex::Captures| {
            self.values
                .get(&c[1])
                .unwrap_or_else(|| panic!("unknown placeholder <<{}>>", &c[1]))
                .clone()
        });
        filled.into_owned()
    }
}

/// Bits per character, as Gitleaks measures a rule's `entropy`.
fn shannon_entropy(s: &str) -> f64 {
    let mut counts: BTreeMap<char, usize> = BTreeMap::new();
    for c in s.chars() {
        *counts.entry(c).or_default() += 1;
    }
    let n = s.chars().count() as f64;
    counts
        .values()
        .map(|&c| {
            let p = c as f64 / n;
            -p * p.log2()
        })
        .sum()
}

fn corpus() -> Vec<Fixture> {
    let path = secretfilter_dir().join("ingest.corpus.yaml");
    let yaml = std::fs::read_to_string(&path).expect("read the corpus");
    let generated = Generated::new();
    let mut fixtures: Vec<Fixture> = serde_saphyr::from_str(&yaml).expect("parse the corpus");
    let mut ids = BTreeSet::new();
    for f in &mut fixtures {
        assert!(ids.insert(f.id.clone()), "fixture {} appears twice", f.id);
        f.line = generated.fill(&f.line);
        f.secret = f.secret.as_deref().map(|s| generated.fill(s));
    }
    fixtures
}

// --- running Alloy -----------------------------------------------------------

/// Whether to run; panics instead of skipping where `MZMON_REQUIRE_ALLOY` is set.
fn alloy_available() -> bool {
    let found = Command::new("alloy")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !found {
        assert!(
            std::env::var_os("MZMON_REQUIRE_ALLOY").is_none(),
            "MZMON_REQUIRE_ALLOY is set and `alloy` is not on PATH"
        );
        eprintln!("skipping: `alloy` is not on PATH");
    }
    found
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .expect("bind a free port")
        .port()
}

/// One HTTP/1.0 exchange, so Go answers without chunked encoding.
fn http(port: u16, method: &str, path: &str, body: &str) -> Option<(u16, String)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .ok()?;
    write!(
        stream,
        "{method} {path} HTTP/1.0\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .ok()?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    let status = response.split(' ').nth(1)?.parse().ok()?;
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.to_string())?;
    Some((status, body))
}

/// An `alloy run` that is killed when dropped.
struct Alloy {
    child: Child,
    server_port: u16,
    log: PathBuf,
    _dir: tempfile::TempDir,
}

impl Alloy {
    /// Starts `config`, in which `LOKI_PORT` stands for the push listener.
    fn start(config: &str) -> (Self, u16) {
        let dir = tempfile::tempdir().expect("tempdir");
        let loki_port = free_port();
        let server_port = free_port();
        let config_path = dir.path().join("config.alloy");
        std::fs::write(
            &config_path,
            config.replace("LOKI_PORT", &loki_port.to_string()),
        )
        .expect("write the config");
        let log = dir.path().join("alloy.log");
        let child = Command::new("alloy")
            .arg("run")
            .arg("--disable-reporting")
            .arg(format!(
                "--storage.path={}",
                dir.path().join("data").display()
            ))
            .arg(format!("--server.http.listen-addr=127.0.0.1:{server_port}"))
            .arg(&config_path)
            .stdout(Stdio::null())
            .stderr(std::fs::File::create(&log).expect("create the log"))
            .spawn()
            .expect("spawn alloy");
        let mut alloy = Alloy {
            child,
            server_port,
            log,
            _dir: dir,
        };
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(status) = alloy.child.try_wait().expect("poll alloy") {
                panic!("alloy exited with {status}:\n{}", alloy.log_text());
            }
            if http(server_port, "GET", "/-/ready", "").is_some_and(|(s, _)| s == 200) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "alloy was not ready:\n{}",
                alloy.log_text()
            );
            std::thread::sleep(Duration::from_millis(250));
        }
        (alloy, loki_port)
    }

    fn log_text(&self) -> String {
        std::fs::read_to_string(&self.log).unwrap_or_default()
    }

    /// Pushes one stream per fixture, labelled `fixture`.
    fn push(&self, loki_port: u16, lines: &[(&str, &str)]) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let streams: Vec<_> = lines
            .iter()
            .enumerate()
            .map(|(i, (id, line))| {
                serde_json::json!({
                    "stream": {"fixture": id},
                    "values": [[(now + i as u128).to_string(), line]],
                })
            })
            .collect();
        let body = serde_json::json!({ "streams": streams }).to_string();
        let (status, response) =
            http(loki_port, "POST", "/loki/api/v1/push", &body).expect("push the corpus");
        assert_eq!(status, 204, "push refused: {response}");
    }

    /// What each `loki.echo` printed, by echo label and then fixture, once
    /// every echo in `expected` has printed that many entries.
    fn echoed(&self, expected: &[(&str, usize)]) -> BTreeMap<String, BTreeMap<String, Echoed>> {
        let pair = Regex::new(r#"(\w+)=("(?:[^"\\]|\\.)*"|\S+)"#).unwrap();
        let fixture = Regex::new(r#"fixture="([^"]*)""#).unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let mut out: BTreeMap<String, BTreeMap<String, Echoed>> = BTreeMap::new();
            for line in self.log_text().lines() {
                let fields: BTreeMap<&str, String> = pair
                    .captures_iter(line)
                    .map(|c| {
                        let raw = c.get(2).unwrap().as_str();
                        // Alloy's logfmt quoting is a JSON string.
                        let value = if raw.starts_with('"') {
                            serde_json::from_str(raw).expect("a quoted logfmt value")
                        } else {
                            raw.to_string()
                        };
                        (c.get(1).unwrap().as_str(), value)
                    })
                    .collect();
                let Some(echo) = fields
                    .get("component_id")
                    .and_then(|c| c.strip_prefix("loki.echo."))
                else {
                    continue;
                };
                let labels = fields.get("labels").cloned().unwrap_or_default();
                let id = fixture.captures(&labels).expect("a fixture label")[1].to_string();
                out.entry(echo.to_string()).or_default().insert(
                    id,
                    Echoed {
                        entry: fields.get("entry").cloned().unwrap_or_default(),
                        labels,
                    },
                );
            }
            if expected
                .iter()
                .all(|(echo, n)| out.get(*echo).map_or(0, BTreeMap::len) == *n)
            {
                return out;
            }
            assert!(
                Instant::now() < deadline,
                "expected {expected:?} entries; the log holds:\n{}",
                self.log_text()
            );
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    /// `loki_secretfilter_secrets_redacted_by_category_total`, as
    /// (component, origin) -> the rules that fired.
    fn redactions(&self) -> BTreeMap<(String, String), BTreeSet<String>> {
        let (_, body) = http(self.server_port, "GET", "/metrics", "").expect("scrape alloy");
        let label = Regex::new(r#"(\w+)="((?:[^"\\]|\\.)*)""#).unwrap();
        let mut out: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
        for line in body.lines() {
            let Some(rest) =
                line.strip_prefix("loki_secretfilter_secrets_redacted_by_category_total{")
            else {
                continue;
            };
            let labels: BTreeMap<&str, &str> = label
                .captures_iter(rest)
                .map(|c| (c.get(1).unwrap().as_str(), c.get(2).unwrap().as_str()))
                .collect();
            let component = labels["component_id"].trim_start_matches("loki.secretfilter.");
            out.entry((component.to_string(), labels["origin"].to_string()))
                .or_default()
                .insert(labels["rule"].to_string());
        }
        out
    }

    fn counter(&self, name: &str, component: &str) -> f64 {
        let (_, body) = http(self.server_port, "GET", "/metrics", "").expect("scrape alloy");
        body.lines()
            .filter(|l| l.starts_with(&format!("{name}{{")))
            .filter(|l| l.contains(&format!("component_id=\"loki.secretfilter.{component}\"")))
            .filter_map(|l| l.rsplit(' ').next()?.parse::<f64>().ok())
            .sum()
    }
}

impl Drop for Alloy {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Debug)]
struct Echoed {
    entry: String,
    labels: String,
}

/// A JSON value with every scalar blanked, so two values compare by shape.
fn shape(value: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            Value::Object(map.iter().map(|(k, v)| (k.clone(), shape(v))).collect())
        }
        Value::Array(items) => Value::Array(items.iter().map(shape).collect()),
        _ => Value::Null,
    }
}

const PLACEHOLDER: &str = "<redacted:$SECRET_NAME>";

fn filter(label: &str, rules: &Path, forward_to: &str, extra: &str) -> String {
    format!(
        r#"
loki.secretfilter "{label}" {{
	forward_to      = [{forward_to}]
	gitleaks_config = "{rules}"
	redact_with     = "{PLACEHOLDER}"
	origin_label    = "fixture"
	{extra}
}}
"#,
        rules = rules.display()
    )
}

const SOURCE: &str = r#"
loki.source.api "corpus" {
	forward_to = [FORWARD_TO]

	http {
		listen_port = LOKI_PORT
	}
}
"#;

// --- tests -------------------------------------------------------------------

#[test]
fn the_ingest_rules_redact_the_corpus() {
    if !alloy_available() {
        return;
    }
    let fixtures = corpus();
    // A second pass over the redacted text, with the same rules.
    let config = [
        SOURCE.replace("FORWARD_TO", "loki.secretfilter.ingest.receiver"),
        filter(
            "ingest",
            &ingest_rules(),
            "loki.secretfilter.rescan.receiver",
            "",
        ),
        filter("rescan", &ingest_rules(), "loki.echo.out.receiver", ""),
        "loki.echo \"out\" { }\n".to_string(),
    ]
    .concat();
    let (alloy, loki_port) = Alloy::start(&config);
    let lines: Vec<(&str, &str)> = fixtures
        .iter()
        .map(|f| (f.id.as_str(), f.line.as_str()))
        .collect();
    alloy.push(loki_port, &lines);
    let echoed = alloy.echoed(&[("out", fixtures.len())]);
    let redactions = alloy.redactions();

    let mut failures = Vec::new();
    for f in &fixtures {
        let out = &echoed["out"][&f.id].entry;
        let fired = redactions
            .get(&("ingest".to_string(), f.id.clone()))
            .cloned()
            .unwrap_or_default();
        let expected: BTreeSet<String> = f.rules.iter().cloned().collect();
        if fired != expected {
            failures.push(format!(
                "{}: expected {expected:?}, fired {fired:?}\n    {out}",
                f.id
            ));
        }
        if f.rules.is_empty() && out != &f.line {
            failures.push(format!(
                "{}: changed\n    in:  {}\n    out: {out}",
                f.id, f.line
            ));
        }
        // Rules that match the same secret are each counted, and the secret
        // is replaced once, so one of their placeholders is enough.
        if !f.rules.is_empty()
            && !f
                .rules
                .iter()
                .any(|rule| out.contains(&format!("<redacted:{rule}>")))
        {
            failures.push(format!(
                "{}: no placeholder for {:?} in\n    {out}",
                f.id, f.rules
            ));
        }
        if let Some(secret) = &f.secret
            && out.contains(secret.as_str())
        {
            failures.push(format!("{}: the secret survived\n    {out}", f.id));
        }
        if let Ok(before) = serde_json::from_str::<serde_json::Value>(&f.line) {
            match serde_json::from_str::<serde_json::Value>(out) {
                Ok(after) if shape(&after) == shape(&before) => {}
                Ok(_) => failures.push(format!("{}: the JSON changed shape\n    {out}", f.id)),
                Err(e) => failures.push(format!("{}: no longer JSON ({e})\n    {out}", f.id)),
            }
        }
    }
    // A placeholder must not match any rule, or every redaction is counted
    // again wherever the text is scanned twice.
    for ((component, origin), rules) in &redactions {
        if component == "rescan" {
            failures.push(format!("{origin}: the second pass matched {rules:?}"));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn a_customer_extension_keeps_the_shipped_and_default_rules() {
    if !alloy_available() {
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let extension = dir.path().join("extension.toml");
    std::fs::write(
        &extension,
        format!(
            r#"[extend]
path = "{}"

[[rules]]
id = "customer-rule"
regex = '''cust-[0-9a-f]{{12}}'''
keywords = ["cust-"]
"#,
            ingest_rules().display()
        ),
    )
    .expect("write the extension");
    // One instance per process. Two instances that load an `[extend] path`
    // chain in the same process race at startup, and one of them can come up
    // with no rules at all; see DEP-314.
    let config = [
        SOURCE.replace("FORWARD_TO", "loki.secretfilter.extended.receiver"),
        filter("extended", &extension, "loki.echo.out.receiver", ""),
        "loki.echo \"out\" { }\n".to_string(),
    ]
    .concat();
    let (alloy, loki_port) = Alloy::start(&config);
    let lines = probe_lines();
    let pushed: Vec<(&str, &str)> = lines.iter().map(|(i, l)| (*i, l.as_str())).collect();
    alloy.push(loki_port, &pushed);
    let echoed = alloy.echoed(&[("out", lines.len())]);
    let redactions = alloy.redactions();
    for (id, rule) in [
        ("default", "aws-access-token"),
        ("shipped", "mzmon-url-userinfo"),
        ("customer", "customer-rule"),
    ] {
        assert_eq!(
            redactions.get(&("extended".to_string(), id.to_string())),
            Some(&BTreeSet::from([rule.to_string()])),
            "{id} through the extension"
        );
        let out = &echoed["out"][id];
        assert!(
            out.entry.contains(&format!("<redacted:{rule}>")),
            "{id}: {out:?}"
        );
    }
}

#[test]
fn the_ingest_timeout_forwards_a_marked_line() {
    if !alloy_available() {
        return;
    }
    // Every line runs out of time at 1ns. The other settings are the ingest
    // instances' in gateway.yaml: forward the line rather than drop it, and
    // mark it.
    let config = [
        SOURCE.replace("FORWARD_TO", "loki.secretfilter.timeout.receiver"),
        filter(
            "timeout",
            &ingest_rules(),
            "loki.echo.out.receiver",
            "processing_timeout = \"1ns\"\n\tdrop_on_timeout    = false\n\tlabel_timed_out    = true",
        ),
        "loki.echo \"out\" { }\n".to_string(),
    ]
    .concat();
    let (alloy, loki_port) = Alloy::start(&config);
    let lines = probe_lines();
    let pushed: Vec<(&str, &str)> = lines.iter().map(|(i, l)| (*i, l.as_str())).collect();
    alloy.push(loki_port, &pushed);
    let echoed = alloy.echoed(&[("out", lines.len())]);
    for (id, line) in &lines {
        let out = &echoed["out"][*id];
        assert!(
            out.labels.contains(r#"secretfilter="timed-out""#),
            "{id}: {out:?}"
        );
        assert_eq!(&out.entry, line, "{id} was changed after timing out");
    }
    assert_eq!(
        alloy.counter("loki_secretfilter_lines_timed_out_total", "timeout"),
        lines.len() as f64
    );
}

/// A line for the default rules, one for the shipped rules, and one for the
/// customer rule in the extension test.
fn probe_lines() -> Vec<(&'static str, String)> {
    let generated = Generated::new();
    vec![
        (
            "default",
            generated.fill("access key <<AWS_ACCESS_KEY_ID>> rejected"),
        ),
        (
            "shipped",
            "metadata_backend_url=postgres://materialize:Hunter2-not-real@db:5432/mz".to_string(),
        ),
        ("customer", "token cust-0123456789ab issued".to_string()),
    ]
}

/// Every rule file the chart ships, by name.
fn shipped_rule_files() -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = std::fs::read_dir(secretfilter_dir())
        .expect("list the rule files")
        .map(|e| e.expect("a directory entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "toml"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&p).expect("read a rule file"))
        })
        .collect();
    files.sort();
    assert!(!files.is_empty());
    files
}

#[test]
fn every_shipped_rule_file_is_flat() {
    // Gitleaks follows two levels of `[extend]`, and a third loads without the
    // default rules. The customer extension is the second level, so a shipped
    // file has to be the first. A shipped file that extends another by `path`
    // also races at startup when two instances in one process load it.
    for (name, toml) in shipped_rule_files() {
        let sections: Vec<&str> = toml
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with('['))
            .collect();
        assert_eq!(
            sections.iter().filter(|s| **s == "[extend]").count(),
            1,
            "{name}"
        );
        let extend: Vec<&str> = toml
            .lines()
            .map(str::trim)
            .skip_while(|l| *l != "[extend]")
            .skip(1)
            .take_while(|l| !l.starts_with('['))
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();
        assert_eq!(
            extend,
            ["useDefault = true"],
            "{name}: the [extend] section"
        );
    }
}

#[test]
fn every_materialize_rule_declares_keywords() {
    // Without keywords, a rule's regex runs on every line instead of only the
    // lines the keyword prefilter passes.
    for (name, toml) in shipped_rule_files() {
        for rule in toml.split("[[rules]]").skip(1) {
            let id = rule
                .lines()
                .find_map(|l| l.trim().strip_prefix("id = "))
                .expect("a rule id")
                .trim_matches('"');
            if id.starts_with("mzmon-") {
                assert!(
                    rule.contains("\nkeywords = ["),
                    "{name}: {id} has no keywords"
                );
            }
        }
    }
}

#[test]
fn the_chart_ships_the_rules_unchanged() {
    let chart = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../charts/materialize-monitoring/pre-rendered/secretfilter");
    for (name, toml) in shipped_rule_files() {
        assert_eq!(
            std::fs::read_to_string(chart.join(&name))
                .unwrap_or_else(|_| panic!("the chart has no {name}; run `make pipelines`")),
            toml,
            "the chart's {name} is stale; run `make pipelines`"
        );
    }
}
