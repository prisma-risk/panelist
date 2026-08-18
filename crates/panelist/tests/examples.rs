//
//  ░█▀█░█▀█░█▀█░█▀▀░█░░░▀█▀░█▀▀░▀█▀
//  ░█▀▀░█▀█░█░█░█▀▀░█░░░░█░░▀▀█░░█░
//  ░▀░░░▀░▀░▀░▀░▀▀▀░▀▀▀░▀▀▀░▀▀▀░░▀░
//
//  Panelist — Strongly Typed Grafana Dashboards
//  https://github.com/prisma-risk/panelist
//
//  Copyright (c) 2026 Prisma Risk
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      https://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//

//! Guards on the shipped examples.
//!
//! `cargo test` compiles every example but never runs one, and until the
//! demo stack existed nothing rendered them either. That gap let a real bug
//! live in `full_dashboard.rs` unnoticed: three queries were written as raw
//! strings with escaped quotes, so Grafana received literal backslashes and
//! rejected all three. The dashboard still compiled, still serialized, and
//! still produced a golden-clean JSON document - it just did not work.
//!
//! These tests scan the example sources for the mistakes that a compiler
//! cannot see, in the same style as the escape-hatch guards in `golden.rs`.

/// Every example, as (name, source). Adding an example here is deliberate
/// work rather than a glob so that a new example cannot silently opt out.
const EXAMPLES: &[(&str, &str)] = &[
    ("basic", include_str!("../examples/basic.rs")),
    ("prometheus", include_str!("../examples/prometheus.rs")),
    ("variables", include_str!("../examples/variables.rs")),
    ("layout", include_str!("../examples/layout.rs")),
    (
        "full_dashboard",
        include_str!("../examples/full_dashboard.rs"),
    ),
    (
        "route_performance",
        include_str!("../examples/route_performance.rs"),
    ),
];

/// Spans of `r#"…"#` raw string literals in `source`, as (line, contents).
fn raw_string_spans(source: &str) -> Vec<(usize, String)> {
    let bytes: Vec<char> = source.chars().collect();
    let mut spans = Vec::new();
    let mut index = 0;
    let mut line = 1;

    while index < bytes.len() {
        if bytes[index] == '\n' {
            line += 1;
        }

        // Only `r#"` … `"#` is scanned. A plain `r"…"` cannot contain a
        // quote at all, so it has no way to express this mistake.
        if bytes[index] == 'r' && bytes.get(index + 1) == Some(&'#') {
            let mut hashes = 0;
            let mut cursor = index + 1;
            while bytes.get(cursor) == Some(&'#') {
                hashes += 1;
                cursor += 1;
            }
            if bytes.get(cursor) == Some(&'"') {
                let start_line = line;
                let body_start = cursor + 1;
                let terminator: String = std::iter::once('"')
                    .chain(std::iter::repeat_n('#', hashes))
                    .collect();
                let rest: String = bytes[body_start..].iter().collect();
                if let Some(end) = rest.find(&terminator) {
                    let body = rest[..end].to_owned();
                    line += body.matches('\n').count();
                    index = body_start + body.chars().count() + terminator.len();
                    spans.push((start_line, body));
                    continue;
                }
            }
        }
        index += 1;
    }
    spans
}

/// A raw string does not process escapes, so `\"` inside one is two
/// characters that reach the query verbatim. In PromQL and LogQL a
/// backslash before a label-matcher quote is never what the author meant:
/// the quote already terminates nothing, because the surrounding literal is
/// raw. This is the exact shape of the `full_dashboard.rs` bug.
#[test]
fn no_example_escapes_a_quote_inside_a_raw_string() {
    let mut offenders = Vec::new();
    for (name, source) in EXAMPLES {
        for (line, body) in raw_string_spans(source) {
            if body.contains("\\\"") {
                offenders.push(format!("{name}.rs:{line}: {body}"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "raw strings do not process escapes, so these `\\\"` sequences reach \
         Grafana as literal backslashes and make the query invalid. Drop the \
         backslashes, or use a normal string literal:\n  {}",
        offenders.join("\n  ")
    );
}

/// The demo stack provisions each example by uid and renders it by uid. An
/// example without one still serializes, but cannot be addressed, so
/// `scripts/demo.sh` refuses to continue and the gallery loses an image.
#[test]
fn every_example_sets_a_dashboard_uid() {
    let missing: Vec<&str> = EXAMPLES
        .iter()
        .filter(|(_, source)| !source.contains("uid: \"") && !source.contains(".uid("))
        .map(|(name, _)| *name)
        .collect();
    assert!(
        missing.is_empty(),
        "these examples set no uid, so the demo stack cannot render them: {missing:?}"
    );
}

/// The guard is only worth having if it fires. Both of these were confirmed
/// to be caught before the tests above were considered done.
#[test]
fn the_raw_string_scanner_finds_what_it_claims_to() {
    let offending = r####"query: promql!(r#"up{job=\"x\"}"#);"####;
    let spans = raw_string_spans(offending);
    assert_eq!(spans.len(), 1, "one raw string should be found: {spans:?}");
    assert!(spans[0].1.contains("\\\""), "the escape should be visible");

    // A normal string literal containing `\"` is correct Rust and must not
    // be reported: the compiler turns it into a plain quote.
    let fine = r#"query: promql!("up{job=\"x\"}");"#;
    assert!(
        raw_string_spans(fine).is_empty(),
        "a normal string literal is not a raw string"
    );

    // A raw string with no escape is fine.
    let clean = r####"query: promql!(r#"up{job="x"}"#);"####;
    let spans = raw_string_spans(clean);
    assert_eq!(spans.len(), 1);
    assert!(!spans[0].1.contains("\\\""));
}
