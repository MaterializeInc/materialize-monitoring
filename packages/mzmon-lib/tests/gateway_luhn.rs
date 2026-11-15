//! Runs the gateway's card-number redaction through the Alloy binary.
//!
//! `stage.luhn` replaces digit runs that pass the Luhn checksum. One random run
//! in ten passes, and so would one in ten of the epoch timestamps Materialize
//! logs as frontiers, so the stage carries a `skip_regex` for them. This test
//! renders the real `loki.process "inputPrefilter"` and the shadow branch from
//! `gateway.yaml`, runs them on `alloy run`, and pushes lines carrying card
//! numbers, timestamps, and both. It checks each line as it leaves the
//! prefilter, the stage's counter, and which lines the shadow branch keeps.
//!
//! The card numbers and the Luhn-valid timestamps are made here, so the
//! repository commits no card-shaped number.
//!
//! It skips when `alloy` is not on `PATH`, unless `MZMON_REQUIRE_ALLOY` is set,
//! as it is in the CI job that pins the binary.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use common::alloy::{
    Alloy, alloy_available, counter, gateway_block, http, label_set, logfmt_fields, refs,
};
use mzmon_lib::alloy::ast::AttributeValue;

const PLACEHOLDER: &str = "<redacted:luhn>";

/// The skip keys on the leading 1 of epoch timestamps, which millisecond time
/// keeps until it reaches 2000000000000 on 2033-05-18.
const ROLLOVER_MS: u128 = 2_000_000_000_000;

/// How long before the rollover the skip is due for review.
const NOTICE_MS: u128 = 365 * 24 * 60 * 60 * 1000;

#[test]
fn the_timestamp_skip_has_a_year_left() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    assert!(
        now + NOTICE_MS < ROLLOVER_MS,
        "Epoch timestamps start with 2 from 2033-05-18, and inputPrefilter's stage.luhn \
         skip_regex in gateway.yaml exempts only those that start with 1. Widen it to the \
         new leading digit before then, and move this deadline."
    );
}

// --- numbers made at test time --------------------------------------------------

/// `body` with the check digit appended that makes it pass the Luhn checksum.
fn luhn(body: &str) -> String {
    let sum: u32 = body
        .chars()
        .rev()
        .enumerate()
        .map(|(i, c)| {
            let d = c.to_digit(10).expect("a digit");
            if i % 2 == 0 {
                let doubled = d * 2;
                if doubled > 9 { doubled - 9 } else { doubled }
            } else {
                d
            }
        })
        .sum();
    format!("{body}{}", (10 - sum % 10) % 10)
}

/// `n` digits from a fixed seed, the same on every run.
fn digits(seed: u64, n: usize) -> String {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    (0..n)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            char::from(b'0' + (state % 10) as u8)
        })
        .collect()
}

/// Fills each `<<NAME>>` in a fixture.
struct Numbers(BTreeMap<&'static str, String>);

impl Numbers {
    fn new() -> Self {
        let since = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        // Each timestamp keeps its width and its leading 1 and takes the check
        // digit, so it is one the stage would replace without the skip.
        let ms = since.as_millis() / 10;
        let us = since.as_micros() / 10;
        let ns = since.as_nanos() / 10;
        let visa = luhn(&format!("4{}", digits(1, 14)));
        let grouped = visa
            .as_bytes()
            .chunks(4)
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect::<Vec<_>>()
            .join(" ");
        let numbers = BTreeMap::from([
            ("VISA", visa),
            ("VISA_GROUPED", grouped),
            ("AMEX", luhn(&format!("37{}", digits(2, 12)))),
            ("UATP", luhn(&format!("1{}", digits(3, 13)))),
            ("ONE_14", luhn(&format!("1{}", digits(4, 12)))),
            ("MS", luhn(&ms.to_string())),
            ("MS_NEXT", luhn(&(ms + 1).to_string())),
            ("US", luhn(&us.to_string())),
            ("NS", luhn(&ns.to_string())),
            ("UUID_TAIL", luhn(&digits(5, 11))),
        ]);
        for (name, len) in [
            ("VISA", 16),
            ("AMEX", 15),
            ("UATP", 15),
            ("ONE_14", 14),
            ("MS", 13),
            ("US", 16),
            ("NS", 19),
            ("UUID_TAIL", 12),
        ] {
            assert_eq!(numbers[name].len(), len, "{name} is {}", numbers[name]);
        }
        Numbers(numbers)
    }

    fn fill(&self, text: &str) -> String {
        self.0.iter().fold(text.to_string(), |t, (name, value)| {
            t.replace(&format!("<<{name}>>"), value)
        })
    }
}

// --- the fixtures ---------------------------------------------------------------

struct Fixture {
    id: &'static str,
    line: &'static str,
    /// The line as it leaves the prefilter, `PLACEHOLDER` where a number went.
    out: &'static str,
}

const FIXTURES: &[Fixture] = &[
    Fixture {
        id: "visa-quoted-by-an-error",
        line: r#"ERROR invalid input syntax for type bigint: "<<VISA>>""#,
        out: r#"ERROR invalid input syntax for type bigint: "<redacted:luhn>""#,
    },
    Fixture {
        id: "amex-in-logfmt",
        line: "level=warn msg=rejected card=<<AMEX>> reason=declined",
        out: "level=warn msg=rejected card=<redacted:luhn> reason=declined",
    },
    // UATP cards start with 1, like the timestamps, and are 15 digits, which
    // no timestamp width is.
    Fixture {
        id: "uatp-in-json",
        line: r#"{"level":"WARN","fields":{"message":"decode error","value":"<<UATP>>"}}"#,
        out: r#"{"level":"WARN","fields":{"message":"decode error","value":"<redacted:luhn>"}}"#,
    },
    Fixture {
        id: "millisecond-frontiers",
        line: "as_of: Antichain { elements: [<<MS>>] }, upper: Antichain { elements: [<<MS_NEXT>>] }",
        out: "as_of: Antichain { elements: [<<MS>>] }, upper: Antichain { elements: [<<MS_NEXT>>] }",
    },
    Fixture {
        id: "microsecond-timestamp",
        line: r#"{"timestamp_us":<<US>>,"level":"INFO"}"#,
        out: r#"{"timestamp_us":<<US>>,"level":"INFO"}"#,
    },
    Fixture {
        id: "nanosecond-timestamp",
        line: "ts_ns=<<NS>> level=info msg=tick",
        out: "ts_ns=<<NS>> level=info msg=tick",
    },
    // The skip needs no word boundary, so a unit suffix doesn't defeat it.
    Fixture {
        id: "timestamp-with-a-unit",
        line: "lag measured at <<MS>>ms",
        out: "lag measured at <<MS>>ms",
    },
    Fixture {
        id: "card-beside-a-timestamp",
        line: "as_of=<<MS>> value=<<VISA>>",
        out: "as_of=<<MS>> value=<redacted:luhn>",
    },
    Fixture {
        id: "uuid",
        line: "replica 5f0c2a1e-9b7d-4c3e-8a6f-<<UUID_TAIL>> ready",
        out: "replica 5f0c2a1e-9b7d-4c3e-8a6f-<<UUID_TAIL>> ready",
    },
    // The skip covers the three timestamp widths and nothing else.
    Fixture {
        id: "fourteen-digits-from-one",
        line: "object id <<ONE_14>> not found",
        out: "object id <redacted:luhn> not found",
    },
    // Separators are off, so a grouped number is four short runs. This is the
    // trade for not joining a date and a count into one candidate.
    Fixture {
        id: "grouped-card-passes",
        line: "card <<VISA_GROUPED>> on file",
        out: "card <<VISA_GROUPED>> on file",
    },
    // The shadow filter redacts the password, so the shadow branch keeps a copy.
    Fixture {
        id: "credential-only",
        line: "level=info msg=connecting url=postgres://materialize:Hunter2-not-real@mzmon-metadata-db:5432/materialize",
        out: "level=info msg=connecting url=postgres://materialize:Hunter2-not-real@mzmon-metadata-db:5432/materialize",
    },
    Fixture {
        id: "card-and-credential",
        line: "level=error msg=failed card=<<VISA>> url=postgres://materialize:Hunter2-not-real@mzmon-metadata-db:5432/materialize",
        out: "level=error msg=failed card=<redacted:luhn> url=postgres://materialize:Hunter2-not-real@mzmon-metadata-db:5432/materialize",
    },
];

/// The fixtures the shadow filter itself redacts, so the shadow branch keeps.
const SHADOW_KEEPS: &[&str] = &["credential-only", "card-and-credential"];

// --- running it -------------------------------------------------------------------

/// Each fixture's line as each `loki.echo` printed it, by echo and then id.
fn echoed(alloy: &Alloy) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for line in alloy.log_text().lines() {
        let fields = logfmt_fields(line);
        let Some(echo) = fields
            .get("component_id")
            .and_then(|c| c.strip_prefix("loki.echo."))
        else {
            continue;
        };
        let labels = label_set(fields.get("labels").map_or("", String::as_str));
        if let Some(id) = labels.get("fixture") {
            out.entry(echo.to_string())
                .or_default()
                .insert(id.clone(), fields.get("entry").cloned().unwrap_or_default());
        }
    }
    out
}

#[test]
fn the_prefilter_replaces_card_numbers_and_keeps_timestamps() {
    if !alloy_available() {
        return;
    }
    let rules = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../alloy-pipelines/secretfilter/ingest.toml");
    let config = format!(
        r#"loki.source.api "in" {{
    http {{
        listen_address = "127.0.0.1"
        listen_port = LOKI_PORT
    }}
    forward_to = [loki.process.inputPrefilter.receiver]
}}

loki.echo "main" {{}}
loki.echo "shadow" {{}}

{}
{}
{}"#,
        gateway_block(
            "loki.process",
            "inputPrefilter",
            vec![(
                "forward_to",
                refs(&[
                    "loki.echo.main.receiver",
                    "loki.secretfilter.shadow.receiver"
                ]),
            )],
        ),
        gateway_block(
            "loki.secretfilter",
            "shadow",
            vec![(
                "gitleaks_config",
                AttributeValue::String(rules.display().to_string()),
            )],
        ),
        gateway_block(
            "loki.process",
            "secretfilterShadow",
            vec![("forward_to", refs(&["loki.echo.shadow.receiver"]))],
        ),
    );
    let (alloy, loki_port) = Alloy::start(&config);

    let numbers = Numbers::new();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    // No stream carries `job`, which the counter's selector names: `=~".*"`
    // has to match a line without the label.
    let streams: Vec<_> = FIXTURES
        .iter()
        .enumerate()
        .map(|(i, f)| {
            serde_json::json!({
                "stream": {"fixture": f.id},
                "values": [[(now + i as u128).to_string(), numbers.fill(f.line)]],
            })
        })
        .collect();
    let body = serde_json::json!({ "streams": streams }).to_string();
    let (status, response) =
        http(loki_port, "POST", "/loki/api/v1/push", &body).expect("push the fixtures");
    assert_eq!(status, 204, "push refused: {response}");

    // Done once every fixture is out of the prefilter and the shadow filter has
    // scanned every copy, whichever the shadow branch kept.
    let deadline = Instant::now() + Duration::from_secs(30);
    let out = loop {
        let out = echoed(&alloy);
        let main = out.get("main").map_or(0, BTreeMap::len);
        let scanned = counter(
            &alloy,
            "loki_secretfilter_processing_duration_seconds_count",
            "loki.secretfilter.shadow",
        );
        let shadow = out.get("shadow").map_or(0, BTreeMap::len);
        if main == FIXTURES.len()
            && scanned >= FIXTURES.len() as f64
            && shadow >= SHADOW_KEEPS.len()
        {
            std::thread::sleep(Duration::from_millis(500));
            break echoed(&alloy);
        }
        assert!(
            Instant::now() < deadline,
            "{main} of {} fixtures left the prefilter, {scanned} were scanned, and {shadow} \
             shadow copies came out:\n{}",
            FIXTURES.len(),
            alloy.log_text()
        );
        std::thread::sleep(Duration::from_millis(250));
    };

    let mut failures = Vec::new();
    for f in FIXTURES {
        let want = numbers.fill(f.out);
        let got = &out["main"][f.id];
        if *got != want {
            failures.push(format!("{}:\n    want {want}\n    got  {got}", f.id));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));

    let redacted = FIXTURES
        .iter()
        .filter(|f| f.out.contains(PLACEHOLDER))
        .count();
    assert_eq!(
        counter(
            &alloy,
            "loki_process_custom_luhn_redacted_lines_total",
            "loki.process.inputPrefilter",
        ),
        redacted as f64,
        "the counter disagrees with the lines that carry {PLACEHOLDER}"
    );

    // A line whose only placeholder is the card number's is already redacted
    // in the main stream, so the shadow branch drops its copy.
    let kept: BTreeSet<&str> = out
        .get("shadow")
        .map(|m| m.keys().map(String::as_str).collect())
        .unwrap_or_default();
    assert_eq!(kept, SHADOW_KEEPS.iter().copied().collect());
}
