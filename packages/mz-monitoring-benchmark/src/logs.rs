// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Log lines in each role's real format, at the scenario's rates.
//!
//! The lines go to the container's own stdout and stderr, so they take the
//! path Materialize's do: the container log file, the agent, the gateway's
//! per-container parsing. The streams match the real processes: tracing and
//! panics on stderr, environmentd's startup banner on stdout.
//!
//! The message catalog is hand-written after lines seen on a live install,
//! with every identifier generated. It is not captured, because a log message
//! is free text and no scrub can promise what a label scrub can.

use std::fmt::Write as _;
use std::io::Write as _;
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::role::{LogFormat, Role};
use crate::scenario::{Level, LogScenario};
use crate::seed::Seed;

pub const CANARY: &str = "mzmon-canary-v1-7f3a9c2e4b6d8015";
pub const EGRESS_CANARY: &str = "mzmon-egress-canary-v1-3b8e1d6a9c4f2e07";

const TICK: Duration = Duration::from_millis(100);

/// One line's shape. Placeholders in `{braces}` are filled per line.
struct Line {
    target: &'static str,
    message: &'static str,
    fields: &'static [(&'static str, &'static str)],
    /// tracing's `span` object, raw JSON, for the JSON format; the span prefix
    /// for the text format.
    span: Option<&'static str>,
}

const fn l(target: &'static str, message: &'static str) -> Line {
    Line {
        target,
        message,
        fields: &[],
        span: None,
    }
}

const ENVIRONMENTD: [&[Line]; 5] = [
    // TRACE
    &[
        l("mz_adapter::coord", "coordinator received message"),
        l(
            "mz_persist_client::internal::machine",
            "applied state for shard {shard} at seqno {n}",
        ),
    ],
    // DEBUG
    &[
        l(
            "mz_adapter::coord::appends",
            "group commit of {n} table writes",
        ),
        l(
            "mz_storage_controller",
            "downgrading read capability for {coll} to [{ts}]",
        ),
        l(
            "mz_compute_client::controller::instance",
            "replica {replica} reported frontier upper for {coll}",
        ),
    ],
    // INFO
    &[
        Line {
            target: "mz_transform::equivalence_propagation",
            message: "ColumnKnowledge performed work after EquivalencePropagation",
            fields: &[("ctx.global_id", "Some(Transient({n}))")],
            span: None,
        },
        Line {
            target: "mz_adapter::coord::sequencer",
            message: "executing statement",
            fields: &[
                ("statement_type", "select"),
                ("cluster", "{cluster}"),
                ("duration_ms", "{ms}"),
            ],
            span: None,
        },
        l(
            "mz_storage_controller",
            "enabling ingestion {coll} on cluster {cluster}",
        ),
        l(
            "mz_persist_client::internal::compact",
            "compaction of shard {shard} completed in {ms}ms",
        ),
        Line {
            target: "mz_compute_client::controller::instance",
            message: "replica connected",
            fields: &[("replica_id", "{replica}"), ("cluster_id", "{cluster}")],
            span: None,
        },
        l(
            "mz_adapter::catalog",
            "catalog transaction committed with {n} ops",
        ),
    ],
    // WARN
    &[
        l(
            "mz_persist_client::internal::machine",
            "external operation consensus::compare_and_set failed, retrying in 64ms: indeterminate: timeout",
        ),
        Line {
            target: "mz_adapter::coord::timeline",
            message: "timestamp oracle read is slow",
            fields: &[("elapsed_ms", "{ms}")],
            span: None,
        },
        l(
            "mz_compute_client::controller::instance",
            "replica {replica} of cluster {cluster} disconnected; reconnecting",
        ),
        l(
            "mz_storage_controller::statistics",
            "statistics for {coll} are {ms}ms behind",
        ),
    ],
    // ERROR
    &[
        l(
            "mz_adapter::coord",
            "failed to install peek on cluster {cluster}: replica unavailable",
        ),
        l(
            "mz_persist_client::internal::blob",
            "fetch of part for shard {shard} failed: NoSuchKey",
        ),
        Line {
            target: "mz_storage::upsert",
            message: "decode error in collection {coll}",
            fields: &[("error", "unexpected end of input at offset {n}")],
            span: None,
        },
    ],
];

const STORAGE_SPAN: &str = r#"{"name":"storage","worker_id":0,"name":"timely"}"#;
const COMPUTE_SPAN: &str = r#"{"name":"compute","worker_id":0,"name":"timely"}"#;

const CLUSTERD: [&[Line]; 5] = [
    &[Line {
        target: "mz_compute::render",
        message: "worker {worker} stepping dataflow {coll}",
        fields: &[],
        span: Some(COMPUTE_SPAN),
    }],
    &[
        Line {
            target: "mz_storage::render::persist_sink",
            message: "persist_sink description lookahead",
            fields: &[("collection_id", "{coll}"), ("lookahead_ms", "0")],
            span: Some(STORAGE_SPAN),
        },
        Line {
            target: "mz_compute::sink::materialized_view",
            message: "mv sink {coll} wrote batch",
            fields: &[("rows", "{n}")],
            span: Some(COMPUTE_SPAN),
        },
    ],
    &[
        Line {
            target: "mz_storage::render",
            message: "timely-0: persisting export {coll} of {coll2}",
            fields: &[("id", "{coll2}")],
            span: Some(STORAGE_SPAN),
        },
        Line {
            target: "mz_storage::source::source_reader_pipeline",
            message: "timely-0 building load-generator source pipeline",
            fields: &[
                ("id", "{coll}"),
                ("as_of", "{0}"),
                ("resume_uppers", "{uppers}"),
            ],
            span: Some(STORAGE_SPAN),
        },
        Line {
            target: "mz_storage::storage_state",
            message: "worker 0/1 trying to (re-)start ingestion {coll}",
            fields: &[
                ("as_of", "Antichain { elements: [0] }"),
                ("resume_uppers", "{uppers}"),
            ],
            span: None,
        },
        Line {
            target: "mz_compute::compute_state",
            message: "hydration complete",
            fields: &[("collection_id", "{coll}"), ("elapsed_ms", "{ms}")],
            span: Some(COMPUTE_SPAN),
        },
    ],
    &[
        Line {
            target: "mz_storage::source::reclock",
            message: "remap shard {shard} is {ms}ms behind its source",
            fields: &[],
            span: Some(STORAGE_SPAN),
        },
        l(
            "mz_persist_client::fetch",
            "slow fetch of blob for shard {shard}: {ms}ms",
        ),
    ],
    &[
        Line {
            target: "mz_storage::upsert",
            message: "upsert state for {coll} failed to rehydrate",
            fields: &[("error", "fetch of part failed: NoSuchKey")],
            span: Some(STORAGE_SPAN),
        },
        l("mz_compute::logging", "dataflow {coll} reported an error"),
    ],
];

const BALANCERD: [&[Line]; 5] = [
    &[l(
        "mz_balancerd",
        "polling resolver for environmentd address",
    )],
    &[Line {
        target: "mz_balancerd",
        message: "resolved environmentd address",
        fields: &[("addr", "10.0.{worker}.{n}:6875")],
        span: Some("pgwire{conn_id={n}}"),
    }],
    &[
        Line {
            target: "mz_balancerd",
            message: "new connection",
            fields: &[("conn_id", "{n}")],
            span: Some("pgwire{conn_id={n}}"),
        },
        l("mz_balancerd", "connection closed after {ms}ms"),
    ],
    &[l(
        "mz_balancerd",
        "upstream connection slow to establish: {ms}ms",
    )],
    &[l(
        "mz_balancerd",
        "failed to connect to upstream server: connection refused (os error 111)",
    )],
];

const ORCHESTRATORD: [&[Line]; 5] = [
    &[Line {
        target: "mz_orchestratord::controller::materialize",
        message: "comparing desired and observed state",
        fields: &[],
        span: Some(
            "reconcile{object.ref=Materialize.v1alpha1.materialize.cloud/bench-{n}.materialize}",
        ),
    }],
    &[Line {
        target: "tower_http::trace::on_request",
        message: "started processing request",
        fields: &[],
        span: Some(
            r#"HTTP request{request.uri=/metrics request.version=HTTP/1.1 request.method=GET request.headers="{\"accept\":\"text/plain\",\"accept-encoding\":\"gzip\"}"}"#,
        ),
    }],
    &[
        Line {
            target: "mz_orchestratord::controller::materialize",
            message: "reconciling environment",
            fields: &[("generation", "{worker}")],
            span: Some(
                "reconcile{object.ref=Materialize.v1alpha1.materialize.cloud/bench-{n}.materialize object.reason=object updated}",
            ),
        },
        l(
            "mz_orchestratord::controller::materialize::balancer",
            "balancerd deployment is ready",
        ),
    ],
    &[l(
        "mz_orchestratord::controller::materialize",
        "environmentd rollout is waiting on {n} pods",
    )],
    &[l(
        "kube_runtime::controller",
        "reconciler for Materialize failed: ApiError: conflict (409)",
    )],
];

#[derive(Clone, Copy, Debug)]
pub struct Ids {
    pub collections: u64,
    pub shards: u64,
    pub clusters: u64,
}

pub struct LogGen {
    role: Role,
    scenario: LogScenario,
    seed: Seed,
    jitter: f64,
    ids: Ids,
    counter: u64,
}

impl LogGen {
    pub fn new(role: Role, scenario: LogScenario, seed: Seed, jitter: f64, ids: Ids) -> Self {
        LogGen {
            role,
            scenario,
            seed: seed.derive("logs"),
            jitter,
            ids,
            counter: 0,
        }
    }

    /// Write forever. Runs on its own thread: the writes block, and the HTTP
    /// side must not wait on a slow log pipe.
    pub fn run(mut self) -> ! {
        let mut stdout = std::io::BufWriter::with_capacity(256 * 1024, std::io::stdout());
        let mut stderr = std::io::BufWriter::with_capacity(256 * 1024, std::io::stderr());
        let period = self.scenario.period;
        // Seeded, so the rare streams across a fleet (a panic every few periods)
        // don't all fire on the same tick.
        let mut carry: [f64; 8] = std::array::from_fn(|i| self.seed.unit(1000 + i as u64));
        let mut last = unix_now();
        let mut tick = 0u64;
        let offset = if self.scenario.synced {
            0.0
        } else {
            self.seed.unit(0) * period
        };

        loop {
            std::thread::sleep(TICK);
            let now = unix_now();
            let dt = (now - last).max(0.0);
            last = now;
            tick += 1;
            let x = ((now + offset) / period).rem_euclid(1.0);

            // Per period: each level at its shape, then the unshaped streams.
            let mut streams = [0.0f64; 8];
            for level in Level::ALL {
                let i = level as usize;
                streams[i] = self.scenario.rates[i] * self.scenario.shapes[i].factor(x, period);
            }
            streams[5] = self.scenario.non_json;
            streams[6] = self.scenario.eicar;
            streams[7] = self.scenario.panic;

            for (i, per_period) in streams.into_iter().enumerate() {
                if per_period <= 0.0 {
                    continue;
                }
                let jitter = self
                    .seed
                    .jitter(self.seed.mix(&[i as u64, tick]), self.jitter);
                carry[i] += per_period / period * dt * jitter;
                let n = carry[i].floor();
                carry[i] -= n;
                let n = n as u64;
                for k in 0..n {
                    let ts = now - dt + dt * (k as f64 + 0.5) / n as f64;
                    let mut line = String::with_capacity(256);
                    match i {
                        5 => {
                            self.non_json(&mut line);
                            let _ = stdout.write_all(line.as_bytes());
                        }
                        6 => {
                            self.canary(&mut line, ts);
                            let _ = stderr.write_all(line.as_bytes());
                        }
                        7 => {
                            let _ = stdout.flush();
                            let _ = stderr.flush();
                            self.panic(&mut line, ts);
                            // One write, as the real handler does, so the lines
                            // reach the container log together.
                            let _ = std::io::stderr().write_all(line.as_bytes());
                            if self.scenario.panic_exit {
                                std::process::exit(101);
                            }
                        }
                        _ => {
                            let level = Level::ALL[i];
                            self.line(&mut line, level, ts);
                            let _ = if self.to_stdout(level) {
                                stdout.write_all(line.as_bytes())
                            } else {
                                stderr.write_all(line.as_bytes())
                            };
                        }
                    }
                }
            }
            let _ = stdout.flush();
            let _ = stderr.flush();
        }
    }

    /// nginx writes its access log to stdout and its error log to stderr;
    /// everything else logs to stderr.
    fn to_stdout(&self, level: Level) -> bool {
        self.role.log_format() == LogFormat::Nginx && level <= Level::Info
    }

    fn next(&mut self) -> u64 {
        self.counter += 1;
        self.seed.hash(self.counter)
    }

    fn fill(&mut self, s: &str) -> String {
        if !s.contains('{') {
            return s.to_string();
        }
        let h = self.next();
        let pick = |n: u64, salt: u64| {
            if n == 0 {
                0
            } else {
                crate::seed::splitmix(h ^ salt) % n
            }
        };
        let coll = 1000 + pick(self.ids.collections.max(1), 1);
        let mut out = s
            .replace("{coll}", &format!("u{coll}"))
            .replace("{coll2}", &format!("u{}", coll + 1))
            .replace(
                "{cluster}",
                &format!("u{}", pick(self.ids.clusters.max(1), 2) + 1),
            )
            .replace(
                "{replica}",
                &format!("u{}", pick(self.ids.clusters.max(1), 3) + 1),
            )
            .replace("{worker}", &(h % 4).to_string())
            .replace("{ms}", &(h % 5000).to_string())
            .replace(
                "{ts}",
                &(1_700_000_000_000 + h % 100_000_000_000).to_string(),
            );
        if out.contains("{shard}") {
            let shard = self
                .seed
                .derive("shard")
                .uuid(pick(self.ids.shards.max(1), 4));
            out = out.replace("{shard}", &format!("s{shard}"));
        }
        if out.contains("{uppers}") {
            let mut u = String::from("{");
            for i in 0..(h % 7 + 2) {
                if i > 0 {
                    u.push_str(", ");
                }
                let _ = write!(u, "User({}): Antichain {{ elements: [0] }}", coll + i);
            }
            u.push('}');
            out = out.replace("{uppers}", &u);
        }
        out.replace("{n}", &(h % 100_000).to_string())
    }

    fn catalog(&self, level: Level) -> &'static [Line] {
        let table = match self.role {
            Role::Environmentd => &ENVIRONMENTD,
            Role::Clusterd => &CLUSTERD,
            Role::Balancerd => &BALANCERD,
            Role::Orchestratord | Role::Console => &ORCHESTRATORD,
        };
        table[level as usize]
    }

    fn line(&mut self, out: &mut String, level: Level, ts: f64) {
        match self.role.log_format() {
            LogFormat::Nginx => self.nginx(out, level, ts),
            format => {
                let lines = self.catalog(level);
                let line = &lines[(self.next() % lines.len() as u64) as usize];
                let message = self.fill(line.message);
                let fields: Vec<(&str, String)> = line
                    .fields
                    .iter()
                    .map(|(k, v)| (*k, self.fill(v)))
                    .collect();
                if format == LogFormat::TracingJson {
                    json_line(
                        out,
                        ts,
                        level.upper(),
                        line.target,
                        &message,
                        &fields,
                        line.span,
                    );
                } else {
                    let span = line.span.map(|s| self.fill(s));
                    text_line(
                        out,
                        ts,
                        level.upper(),
                        span.as_deref(),
                        line.target,
                        &message,
                        &fields,
                    );
                }
            }
        }
    }

    fn nginx(&mut self, out: &mut String, level: Level, ts: f64) {
        let h = self.next();
        const PATHS: &[&str] = &[
            "/",
            "/api/region",
            "/assets/index.js",
            "/static/app.css",
            "/oauth/metadata",
        ];
        let path = PATHS[(h % PATHS.len() as u64) as usize];
        let ip = format!("10.0.{}.{}", (h >> 8) % 255, h % 254 + 1);
        if level <= Level::Info {
            let status = if h.is_multiple_of(50) { 304 } else { 200 };
            let _ = writeln!(
                out,
                "{ip} - - [{}] \"GET {path} HTTP/1.1\" {status} {} \"-\" \"kube-probe/1.35\" \"-\"",
                nginx_access_time(ts),
                1000 + h % 4000
            );
        } else {
            let (tag, msg) = if level == Level::Warn {
                (
                    "warn",
                    "an upstream response is buffered to a temporary file",
                )
            } else {
                (
                    "error",
                    "connect() failed (111: Connection refused) while connecting to upstream",
                )
            };
            let _ = writeln!(
                out,
                "{} [{tag}] 29#29: *{} {msg}, client: {ip}, server: , request: \"GET {path} HTTP/1.1\", host: \"console\"",
                nginx_error_time(ts),
                h % 100_000
            );
        }
    }

    fn canary(&mut self, out: &mut String, ts: f64) {
        let fields = [
            ("canary", CANARY.to_string()),
            ("egress_canary", EGRESS_CANARY.to_string()),
        ];
        match self.role.log_format() {
            LogFormat::TracingJson => json_line(
                out,
                ts,
                "WARN",
                "mz_monitoring_benchmark",
                "mzmon canary",
                &fields,
                None,
            ),
            LogFormat::TracingText => text_line(
                out,
                ts,
                "WARN",
                None,
                "mz_monitoring_benchmark",
                "mzmon canary",
                &fields,
            ),
            LogFormat::Nginx => {
                let _ = writeln!(
                    out,
                    "{} [warn] 29#29: mzmon canary canary={CANARY} egress_canary={EGRESS_CANARY}",
                    nginx_error_time(ts)
                );
            }
        }
    }

    /// A line in no structured format. environmentd's startup banner is the
    /// real-world case: it goes to stdout while tracing goes to stderr.
    fn non_json(&mut self, out: &mut String) {
        let h = self.next();
        let role = self.role.as_str();
        let _ = match h % 4 {
            0 => writeln!(out, "{role} v26.49.1 (debe6d2ed) listening..."),
            1 => writeln!(out, " Internal HTTP address: 0.0.0.0:6878"),
            2 => writeln!(
                out,
                " root trace ID: {:032x}",
                u128::from(h) << 64 | u128::from(self.next())
            ),
            _ => writeln!(out, "    SQL address: 0.0.0.0:6875"),
        };
    }

    fn panic(&mut self, out: &mut String, ts: f64) {
        let h = self.next();
        let location = match self.role {
            Role::Environmentd => "src/adapter/src/coord/sequencer/inner.rs",
            Role::Clusterd => "src/storage/src/render/sources.rs",
            Role::Balancerd => "src/balancerd/src/lib.rs",
            _ => "src/orchestratord/src/controller/materialize.rs",
        };
        let _ = writeln!(
            out,
            "{}  thread '{}' panicked at {location}:{}:{}:",
            rfc3339_micros(ts),
            self.role.panic_thread(),
            100 + h % 2000,
            1 + h % 60
        );
        let _ = writeln!(
            out,
            "mz-monitoring-benchmark: synthetic panic {} injected by the load scenario",
            h % 100_000
        );
        out.push_str("stack backtrace:\n");
        const FRAMES: &[&str] = &[
            "__rustc::rust_begin_unwind",
            "core::panicking::panic_fmt",
            "core::panicking::panic_display",
            "core::option::expect_failed",
            "mz_ore::panic::install_enhanced_handler::{{closure}}",
            "<mz_adapter::coord::Coordinator>::sequence_plan::{{closure}}",
            "<core::pin::Pin<P> as core::future::future::Future>::poll",
            "tokio::runtime::task::core::Core<T,S>::poll",
            "tokio::runtime::task::harness::Harness<T,S>::poll",
            "tokio::runtime::scheduler::multi_thread::worker::Context::run_task",
            "tokio::runtime::scheduler::multi_thread::worker::run",
            "std::sys::backtrace::__rust_begin_short_backtrace",
            "core::ops::function::FnOnce::call_once{{vtable.shim}}",
            "std::sys::pal::unix::thread::Thread::new::thread_start",
        ];
        for (i, frame) in FRAMES.iter().enumerate() {
            let _ = writeln!(out, "  {i:>2}: {frame}");
        }
        out.push_str(
            "note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.\n",
        );
    }
}

fn json_line(
    out: &mut String,
    ts: f64,
    level: &str,
    target: &str,
    message: &str,
    fields: &[(&str, String)],
    span: Option<&str>,
) {
    let _ = write!(
        out,
        r#"{{"timestamp":"{}","level":"{level}","fields":{{"message":"#,
        rfc3339_micros(ts)
    );
    json_string(out, message);
    for (k, v) in fields {
        out.push(',');
        json_string(out, k);
        out.push(':');
        if v.parse::<i64>().is_ok() {
            out.push_str(v);
        } else {
            json_string(out, v);
        }
    }
    out.push_str(r#"},"target":"#);
    json_string(out, target);
    if let Some(span) = span {
        let _ = write!(out, r#","span":{span},"spans":[{span}]"#);
    }
    out.push_str("}\n");
}

fn text_line(
    out: &mut String,
    ts: f64,
    level: &str,
    span: Option<&str>,
    target: &str,
    message: &str,
    fields: &[(&str, String)],
) {
    let _ = write!(out, "{} {level:>5} ", rfc3339_micros(ts));
    if let Some(span) = span {
        out.push_str(span);
        out.push_str(": ");
    }
    let _ = write!(out, "{target}: {message}");
    for (k, v) in fields {
        let _ = write!(out, " {k}={v}");
    }
    out.push('\n');
}

fn json_string(out: &mut String, s: &str) {
    out.push_str(&serde_json::Value::from(s).to_string());
}

pub fn unix_now() -> f64 {
    Utc::now().timestamp_micros() as f64 / 1e6
}

fn utc(ts: f64) -> DateTime<Utc> {
    DateTime::from_timestamp_micros((ts * 1e6).round() as i64).unwrap_or_default()
}

pub fn rfc3339_micros(ts: f64) -> String {
    utc(ts).format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()
}

fn nginx_access_time(ts: f64) -> String {
    utc(ts).format("%d/%b/%Y:%H:%M:%S +0000").to_string()
}

fn nginx_error_time(ts: f64) -> String {
    utc(ts).format("%Y/%m/%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generator(role: Role) -> LogGen {
        LogGen::new(
            role,
            LogScenario::default(),
            Seed::new("t"),
            0.1,
            Ids {
                collections: 100,
                shards: 10,
                clusters: 3,
            },
        )
    }

    #[test]
    fn timestamps() {
        assert_eq!(
            rfc3339_micros(1_794_946_351.114_216),
            "2026-11-17T20:12:31.114216Z"
        );
        assert_eq!(
            nginx_access_time(1_794_946_351.0),
            "17/Nov/2026:20:12:31 +0000"
        );
        assert_eq!(rfc3339_micros(951_782_400.0), "2000-02-29T00:00:00.000000Z");
    }

    #[test]
    fn json_lines_parse_the_way_the_gateway_expects() {
        let mut g = generator(Role::Clusterd);
        let re =
            regex::Regex::new(r#"^\{"timestamp":"[^"]*","level":"(TRACE|DEBUG|INFO|WARN|ERROR)""#)
                .unwrap();
        for level in Level::ALL {
            for _ in 0..20 {
                let mut s = String::new();
                g.line(&mut s, level, 1_794_946_351.0);
                assert!(re.is_match(&s), "{s}");
                assert!(!s.contains('{') || !s.contains("{coll}"), "unfilled: {s}");
                assert!(s.ends_with("}\n"));
            }
        }
    }

    #[test]
    fn text_lines_match_the_gateway_regex() {
        let mut g = generator(Role::Orchestratord);
        let re = regex::Regex::new(r"^(\S+)\s+(TRACE|DEBUG|INFO|WARN|ERROR)\s+(.*)$").unwrap();
        for level in Level::ALL {
            let mut s = String::new();
            g.line(&mut s, level, 1_794_946_351.0);
            assert!(re.is_match(s.trim_end()), "{s}");
        }
    }

    #[test]
    fn panics_match_the_gateway_regex() {
        let mut g = generator(Role::Environmentd);
        let mut s = String::new();
        g.panic(&mut s, 1_794_946_351.0);
        let re = regex::Regex::new(
            r"^(?:\d{4}-\d{2}-\d{2}T\S+\s+)?thread '(?P<t>[^']*)' panicked at (?P<l>\S+:\d+:\d+):\n(?P<m>[^\n]*)",
        )
        .unwrap();
        let c = re.captures(&s).unwrap_or_else(|| panic!("{s}"));
        assert_eq!(&c["t"], "coordinator");
        assert!(c["m"].contains("synthetic panic"));
    }

    #[test]
    fn canary_carries_both_strings_and_no_keywords() {
        let mut g = generator(Role::Environmentd);
        let mut s = String::new();
        g.canary(&mut s, 1_794_946_351.0);
        assert!(s.contains(CANARY) && s.contains(EGRESS_CANARY));
        for word in [
            "secret",
            "key",
            "token",
            "auth",
            "api",
            "access",
            "credential",
            "passw",
        ] {
            assert!(!s.to_lowercase().contains(word), "{word} in {s}");
        }
    }
}
