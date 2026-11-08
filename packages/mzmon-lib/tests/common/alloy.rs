//! Running the Alloy binary from a test.
//!
//! A test starts `alloy run` on a small config, talks to it over HTTP, and
//! reads what its components log. Tests skip when `alloy` is not on `PATH`,
//! unless `MZMON_REQUIRE_ALLOY` is set, as it is in the CI job that pins the
//! binary.

// Each integration test is its own crate and uses a different subset.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use regex::Regex;

/// Whether to run; panics instead of skipping where `MZMON_REQUIRE_ALLOY` is set.
pub fn alloy_available() -> bool {
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

pub fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .expect("bind a free port")
        .port()
}

/// One HTTP/1.0 exchange, so Go answers without chunked encoding.
pub fn http(port: u16, method: &str, path: &str, body: &str) -> Option<(u16, String)> {
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

/// The fields of one line of Alloy's own logfmt log, with quoted values
/// unquoted.
pub fn logfmt_fields(line: &str) -> BTreeMap<&str, String> {
    let pair = Regex::new(r#"(\w+)=("(?:[^"\\]|\\.)*"|\S+)"#).unwrap();
    pair.captures_iter(line)
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
        .collect()
}

/// The labels in a label set as `loki.echo` prints it, `{a="b", c="d"}`.
pub fn label_set(text: &str) -> BTreeMap<String, String> {
    let label = Regex::new(r#"(\w+)="((?:[^"\\]|\\.)*)""#).unwrap();
    label
        .captures_iter(text)
        .map(|c| (c[1].to_string(), c[2].to_string()))
        .collect()
}

/// An `alloy run` that is killed when dropped.
pub struct Alloy {
    child: Child,
    pub server_port: u16,
    log: PathBuf,
    _dir: tempfile::TempDir,
}

impl Alloy {
    /// Starts `config`, in which `LOKI_PORT` stands for the push listener.
    ///
    /// Runs at the `experimental` stability level, as the gateway does, so a
    /// config that loads here loads there.
    pub fn start(config: &str) -> (Self, u16) {
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
            .arg("--stability.level=experimental")
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

    pub fn log_text(&self) -> String {
        std::fs::read_to_string(&self.log).unwrap_or_default()
    }
}

impl Drop for Alloy {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
