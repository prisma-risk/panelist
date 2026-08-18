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

use std::{fs, path::PathBuf};

use panelist::prelude::*;

fn golden_dashboard() -> Dashboard {
    dashboard! {
        title: "Golden service";
        uid: "golden-service";
        description: "Stable output contract.";
        tags: ["golden", "test"];
        refresh: "30s";
        datasource: prometheus("prometheus-main");

        variable "region" {
            values: ["us-east-1", "us-west-2"];
            default: "us-east-1";
        }

        row "Overview" {
            stat "Availability" {
                query: promql!("avg(up{region=\"$region\"})");
                unit: percent;
                width: 8;
                thresholds {
                    red: 0.0;
                    yellow: 99.0;
                    green: 99.9;
                }
            }

            timeseries "Traffic" {
                query: promql!("sum by (status) (rate(requests_total[$__rate_interval]))") {
                    legend_format: "{{status}}";
                }
                unit: reqps;
                width: 16;
            }
        }
    }
}

#[test]
fn generated_json_matches_committed_golden_file() {
    let actual = format!("{}\n", golden_dashboard().to_json_pretty().unwrap());
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/basic.json");

    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::write(&path, &actual).unwrap();
    }

    let expected = fs::read_to_string(path).unwrap();
    assert_eq!(actual, expected);
}

/// The acceptance dashboard for the whole typed-transformations effort: a
/// real operational dashboard expressed entirely through `dashboard!` and
/// typed builders, with no raw Grafana JSON anywhere. It also doubles as the
/// only golden coverage for the panel kinds `basic.json` doesn't exercise —
/// gauge, table, bar gauge, heatmap, and text all appear here with both
/// their bare defaults and their typed options exercised.
fn route_performance_dashboard() -> Dashboard {
    dashboard! {
        title: "Route performance";
        uid: "route-performance";
        refresh: "30s";
        datasource: prometheus("prometheus-main");

        variable "route" {
            query: promql!("label_values(http_requests_total, route)");
            include_all: true;
        }

        row "Overview" {
            stat "Requests / sec" {
                query: promql!("sum(rate(http_requests_total[$__rate_interval]))");
                unit: reqps;
                width: 6;
            }

            stat "Error rate" {
                query: promql!("sum(rate(http_requests_total{status=~\"5..\"}[$__rate_interval])) / sum(rate(http_requests_total[$__rate_interval]))");
                unit: percent_unit;
                width: 6;

                thresholds {
                    green: 0.0;
                    yellow: 0.01;
                    red: 0.05;
                }
            }

            stat "p50" {
                query: promql!("histogram_quantile(0.50, sum by (le) (rate(http_request_duration_seconds_bucket[$__rate_interval])))");
                unit: seconds;
                width: 6;
            }

            stat "p95" {
                query: promql!("histogram_quantile(0.95, sum by (le) (rate(http_request_duration_seconds_bucket[$__rate_interval])))");
                unit: seconds;
                width: 6;
            }
        }

        row "Traffic" {
            timeseries "Request rate by status" {
                query: promql!("sum(rate(http_requests_total{status=~\"2..\"}[$__rate_interval]))") {
                    legend_format: "2xx";
                }
                query: promql!("sum(rate(http_requests_total{status=~\"4..\"}[$__rate_interval]))") {
                    legend_format: "4xx";
                }
                query: promql!("sum(rate(http_requests_total{status=~\"5..\"}[$__rate_interval]))") {
                    legend_format: "5xx";
                }
                unit: reqps;
                width: 12;
                stacking: normal;
            }

            bar_gauge "Top routes by traffic" {
                query: promql!("topk(10, sum by (route) (rate(http_requests_total[$__rate_interval])))") {
                    legend_format: "{{route}}";
                    instant: true;
                }
                unit: reqps;
                width: 12;
            }
        }

        row "Routes" {
            table "Route performance" {
                query: promql!("sum by (route) (rate(http_requests_total[$__rate_interval]))") {
                    ref_id: "A";
                    format: table;
                    instant: true;
                }
                query: promql!("sum by (route) (rate(http_requests_total{status=~\"4..\"}[$__rate_interval]))") {
                    ref_id: "B";
                    format: table;
                    instant: true;
                }
                query: promql!("sum by (route) (rate(http_requests_total{status=~\"5..\"}[$__rate_interval]))") {
                    ref_id: "C";
                    format: table;
                    instant: true;
                }
                query: promql!("sum by (route) (rate(http_requests_total{status=~\"5..\"}[$__rate_interval])) / sum by (route) (rate(http_requests_total[$__rate_interval]))") {
                    ref_id: "D";
                    format: table;
                    instant: true;
                }
                query: promql!("histogram_quantile(0.50, sum by (route, le) (rate(http_request_duration_seconds_bucket[$__rate_interval])))") {
                    ref_id: "E";
                    format: table;
                    instant: true;
                }
                query: promql!("histogram_quantile(0.95, sum by (route, le) (rate(http_request_duration_seconds_bucket[$__rate_interval])))") {
                    ref_id: "F";
                    format: table;
                    instant: true;
                }
                query: promql!("histogram_quantile(0.99, sum by (route, le) (rate(http_request_duration_seconds_bucket[$__rate_interval])))") {
                    ref_id: "G";
                    format: table;
                    instant: true;
                }
                query: promql!("histogram_quantile(0.95, sum by (route, le) (rate(http_request_duration_seconds_bucket[$__rate_interval])))") {
                    ref_id: "H";
                    format: time_series;
                }
                width: 24;

                transform time_series_to_table {
                    query "H": last;
                }
                transform join_by_field("route", outer_tabular);
                transform organize {
                    rename "Value #A" => "RPS";
                    rename "Value #B" => "4xx rate";
                    rename "Value #C" => "5xx rate";
                    rename "Value #D" => "error %";
                    rename "Value #E" => "p50";
                    rename "Value #F" => "p95";
                    rename "Value #G" => "p99";
                    // The `Value #<refId>` names on A-G come from the
                    // Prometheus datasource's own response transform, which
                    // renames each query's `Value` field to `Value #<refId>`
                    // whenever a panel has more than one `format: table`
                    // query — before any panel transformation runs.
                    // `timeSeriesTable`, by contrast, is a panel transform
                    // that names its synthesized trend field `Trend #<refId>`
                    // directly at creation; it never produces a plain
                    // `Value` field, so it never goes through that
                    // datasource-side renaming. Renaming from `Value #H`
                    // here would be a silent no-op, so this one differs from
                    // the other seven on purpose — do not "fix" it back to
                    // match them.
                    rename "Trend #H" => "Trend";
                    // The joined frame carries seven separate `Time` fields,
                    // one per `format: table` query above — `excludeByName`
                    // matches by name, so this one entry hides all of them.
                    hide "Time";
                    order ["route", "RPS", "4xx rate", "5xx rate", "error %", "p50", "p95", "p99", "Trend"];
                }
                sort_by: ("p95", desc);

                override field("RPS") {
                    unit: reqps;
                }

                override field("4xx rate") {
                    unit: reqps;
                }

                override field("5xx rate") {
                    unit: reqps;
                }

                override field("error %") {
                    unit: percent_unit;
                    cell: colored_background;
                    thresholds {
                        green: 0.0;
                        yellow: 0.01;
                        red: 0.05;
                    }
                }

                override field("p50") {
                    unit: seconds;
                }

                override field("p95") {
                    unit: seconds;
                    cell: colored_background;
                    thresholds {
                        green: 0.0;
                        yellow: 0.3;
                        red: 1.0;
                    }
                }

                override field("p99") {
                    unit: seconds;
                }

                override field("Trend") {
                    cell: sparkline { hide_value: true; line_width: 2.0; };
                }
            }
        }

        row "Latency" {
            timeseries "p95 by route" {
                query: promql!("topk(5, histogram_quantile(0.95, sum by (route, le) (rate(http_request_duration_seconds_bucket[$__rate_interval]))))") {
                    legend_format: "{{route}}";
                }
                unit: seconds;
                width: 12;
            }

            heatmap "Latency distribution" {
                query: promql!("sum by (le) (rate(http_request_duration_seconds_bucket{route=~\"$route\"}[$__rate_interval]))") {
                    format: heatmap;
                }
                unit: seconds;
                width: 12;
                color_scheme: "Oranges";
                color_steps: 64;
                cell_gap: 1;
                calculate: false;

                y_axis {
                    unit: seconds;
                    placement: left;
                }
            }
        }

        row "Health" {
            gauge "Error budget remaining" {
                query: promql!("100 - ((sum(increase(http_requests_total{status=~\"5..\"}[30d])) / sum(increase(http_requests_total[30d]))) * 100 / 0.1)");
                unit: percent;
                width: 12;

                thresholds {
                    red: 0.0;
                    yellow: 20.0;
                    green: 50.0;
                }
            }

            text "Runbook" {
                content: "See the **route performance runbook** before paging the on-call engineer. Escalate to #eng-oncall if the error rate stays above threshold for more than 15 minutes.";
                mode: markdown;
                width: 12;
            }
        }
    }
}

#[test]
fn route_performance_matches_the_committed_golden_file() {
    let actual = format!(
        "{}\n",
        route_performance_dashboard().to_json_pretty().unwrap()
    );
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/route_performance.json");

    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::write(&path, &actual).unwrap();
    }

    let expected = fs::read_to_string(path).unwrap();
    assert_eq!(actual, expected);
}

// ---------------------------------------------------------------------------
// The "no raw Grafana JSON" guard.
//
// This is the test standing behind the owner's hard gate on the acceptance
// dashboard. It has been walked past twice — once with whitespace, once with
// a raw-string key — so it no longer pattern-matches call syntax at all.
// Instead it classifies the source into code and not-code, then looks for the
// *identifiers* an escape hatch cannot be written without. Three properties
// make that hard to evade:
//
//   1. Comments are classified as not-code, so `option /* hi */ "k": v;`
//      still reads as the identifier `option` sitting in code.
//   2. String and character literal *contents* are classified as not-code,
//      so a raw string cannot smuggle a key past a `"`-anchored pattern —
//      and, symmetrically, the forbidden identifiers written as string
//      literals in this very file do not trip the guard when it scans
//      itself.
//   3. Whole files are scanned, not an extracted function body, so a hatch
//      in a helper *within one of the scanned files* is still in range.
//      The scan covers exactly the two files listed above; a helper moved
//      into a new module is NOT in range. That is a known limit, recorded
//      rather than papered over — widening it means walking the crate's
//      test and example trees, which is a bigger change than this guard.
//
// Property 2 is what lets property 3 work: the guard can scan its own source
// because everything distinctive about it lives inside string literals.
// ---------------------------------------------------------------------------

/// One source character: its byte offset, the character, and whether it is
/// code rather than part of a comment or of a string/char literal's
/// contents. Delimiters (`//`, `/*`, `"`, `'`, `#`) count as not-code; only
/// real tokens survive.
struct Classified {
    offset: usize,
    ch: char,
    is_code: bool,
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// Splits Rust source into code and not-code. Handles line comments, nested
/// block comments, normal and byte strings with backslash escapes, raw
/// strings of any hash depth, and character literals — while leaving
/// lifetimes (`'a`) alone, which would otherwise desynchronize the scan.
fn classify(source: &str) -> Vec<Classified> {
    let chars: Vec<(usize, char)> = source.char_indices().collect();
    let mut out: Vec<Classified> = Vec::with_capacity(chars.len());
    let mut index = 0usize;

    let peek = |at: usize| chars.get(at).map(|(_, ch)| *ch);
    let take = |out: &mut Vec<Classified>, at: usize, is_code: bool| {
        let (offset, ch) = chars[at];
        out.push(Classified {
            offset,
            ch,
            is_code,
        });
    };

    while index < chars.len() {
        let ch = chars[index].1;

        if ch == '/' && peek(index + 1) == Some('/') {
            while index < chars.len() && chars[index].1 != '\n' {
                take(&mut out, index, false);
                index += 1;
            }
            continue;
        }

        if ch == '/' && peek(index + 1) == Some('*') {
            let mut depth = 0usize;
            while index < chars.len() {
                if chars[index].1 == '/' && peek(index + 1) == Some('*') {
                    depth += 1;
                    take(&mut out, index, false);
                    take(&mut out, index + 1, false);
                    index += 2;
                    continue;
                }
                if chars[index].1 == '*' && peek(index + 1) == Some('/') {
                    depth -= 1;
                    take(&mut out, index, false);
                    take(&mut out, index + 1, false);
                    index += 2;
                    if depth == 0 {
                        break;
                    }
                    continue;
                }
                take(&mut out, index, false);
                index += 1;
            }
            continue;
        }

        // Raw string: `r"…"`, `r#"…"#`, `br##"…"##`. The `r` must start a
        // token, otherwise it is just a letter inside an identifier.
        let starts_token = index == 0 || !is_identifier_char(chars[index - 1].1);
        let raw_start = if starts_token && ch == 'r' {
            Some(index + 1)
        } else if starts_token && ch == 'b' && peek(index + 1) == Some('r') {
            Some(index + 2)
        } else {
            None
        };
        if let Some(after_prefix) = raw_start {
            let mut hashes = after_prefix;
            while peek(hashes) == Some('#') {
                hashes += 1;
            }
            if peek(hashes) == Some('"') {
                let depth = hashes - after_prefix;
                while index <= hashes {
                    take(&mut out, index, false);
                    index += 1;
                }
                'raw: while index < chars.len() {
                    if chars[index].1 == '"'
                        && (1..=depth).all(|step| peek(index + step) == Some('#'))
                    {
                        for step in 0..=depth {
                            take(&mut out, index + step, false);
                        }
                        index += depth + 1;
                        break 'raw;
                    }
                    take(&mut out, index, false);
                    index += 1;
                }
                continue;
            }
        }

        // Normal or byte string.
        if ch == '"' || (ch == 'b' && peek(index + 1) == Some('"') && starts_token) {
            if ch == 'b' {
                take(&mut out, index, false);
                index += 1;
            }
            take(&mut out, index, false);
            index += 1;
            while index < chars.len() {
                if chars[index].1 == '\\' {
                    take(&mut out, index, false);
                    if index + 1 < chars.len() {
                        take(&mut out, index + 1, false);
                    }
                    index += 2;
                    continue;
                }
                let closing = chars[index].1 == '"';
                take(&mut out, index, false);
                index += 1;
                if closing {
                    break;
                }
            }
            continue;
        }

        // Character literal, but not a lifetime: `'a'` and `'\n'` are
        // literals, `'a` on its own is a lifetime and stays code.
        if ch == '\''
            && (peek(index + 1) == Some('\\')
                || (peek(index + 1).is_some() && peek(index + 2) == Some('\'')))
        {
            take(&mut out, index, false);
            index += 1;
            while index < chars.len() {
                if chars[index].1 == '\\' {
                    take(&mut out, index, false);
                    if index + 1 < chars.len() {
                        take(&mut out, index + 1, false);
                    }
                    index += 2;
                    continue;
                }
                let closing = chars[index].1 == '\'';
                take(&mut out, index, false);
                index += 1;
                if closing {
                    break;
                }
            }
            continue;
        }

        take(&mut out, index, true);
        index += 1;
    }

    out
}

/// The source with everything that is not code replaced by spaces, so a
/// scan cannot see inside comments or literals.
///
/// A blanked character is replaced by as many spaces as it occupied bytes,
/// which keeps byte offsets identical to the original — the license header's
/// box-drawing art is multi-byte, and `dashboard_block` slices the original
/// source using offsets found here.
fn code_only(source: &str) -> String {
    let mut blanked = String::with_capacity(source.len());
    for entry in classify(source) {
        if entry.is_code {
            blanked.push(entry.ch);
        } else {
            for _ in 0..entry.ch.len_utf8() {
                blanked.push(' ');
            }
        }
    }
    debug_assert_eq!(blanked.len(), source.len());
    blanked
}

/// Whether `identifier` appears in `code` as a whole token rather than as
/// part of a longer one — so `option` does not match `legend_options`, and
/// `json` does not match `serde_json`.
fn contains_identifier(code: &str, identifier: &str) -> bool {
    code.match_indices(identifier).any(|(at, _)| {
        let before = code[..at].chars().next_back();
        let after = code[at + identifier.len()..].chars().next();
        !before.is_some_and(is_identifier_char) && !after.is_some_and(is_identifier_char)
    })
}

/// Every escape hatch that can put author-supplied JSON into a dashboard,
/// named by an identifier it cannot be written without.
///
/// The three `Raw*` types and `Custom` are the hatches themselves.
/// `option` and `extra` cover both the builder methods and the DSL keys.
/// `json`, `serde_json`, and `Value` cover the value side: every remaining
/// hatch — `FieldConfig::custom` in particular — takes a `serde_json::Value`,
/// and a typed dashboard has no other reason to name serde_json at all.
///
/// Deliberately absent: bare `custom`, because `unit: custom("USD")` and
/// `Unit::custom(…)` are legitimate typed API. `FieldConfig::custom` is
/// caught by the dotted-call check below instead.
const RAW_JSON_IDENTIFIERS: &[&str] = &[
    "option",
    "extra",
    "Custom",
    "json",
    "serde_json",
    "Value",
    "RawPanel",
    "RawQuery",
    "RawTransformation",
];

/// Returns every escape hatch `source` reaches for. Empty means clean.
fn raw_escape_hatches(source: &str) -> Vec<String> {
    let code = code_only(source);
    let mut found: Vec<String> = RAW_JSON_IDENTIFIERS
        .iter()
        .filter(|identifier| contains_identifier(&code, identifier))
        .map(|identifier| (*identifier).to_owned())
        .collect();

    // `FieldConfig::custom(…)`, matched on the receiver dot so that
    // `Unit::custom(…)` and the DSL's `unit: custom(…)` stay legal. Spaces
    // are stripped first, so `. custom (` cannot slip through.
    let dense: String = code.chars().filter(|ch| !ch.is_whitespace()).collect();
    if dense.contains(".custom(") {
        found.push(".custom(".to_owned());
    }
    found
}

fn assert_no_raw_escape_hatches(label: &str, source: &str) {
    let found = raw_escape_hatches(source);
    assert!(
        found.is_empty(),
        "{label} must not use raw Grafana JSON, but reaches for: {}",
        found.join(", ")
    );
}

#[test]
fn route_performance_uses_no_raw_escape_hatches() {
    // The whole of this file, not just the acceptance dashboard's function
    // body: an escape hatch in a helper the dashboard calls, or in a
    // `panels:` fragment built elsewhere, used to be entirely out of range.
    assert_no_raw_escape_hatches(
        "the acceptance dashboard's test file",
        include_str!("golden.rs"),
    );

    // ...and the hand-maintained example, which is the artifact a user
    // actually runs and copies from. It duplicates the same `dashboard!`
    // invocation rather than importing it (examples cannot import from
    // `tests/`), so nothing here otherwise guards it against drifting to
    // include an escape hatch of its own.
    assert_no_raw_escape_hatches(
        "the route_performance example",
        include_str!("../examples/route_performance.rs"),
    );
}

/// Each of these walked past the previous guard, which pattern-matched call
/// syntax on a whitespace-collapsed copy of an extracted function body. They
/// were confirmed to slip through it before this one was written; the
/// raw-string form was additionally confirmed to compile and reach the
/// emitted JSON.
#[test]
fn the_escape_hatch_guard_catches_known_evasions() {
    for (label, attack) in [
        (
            "raw-string DSL key",
            r###"stat "S" { query: promql!("up"); option r"smuggled": true; }"###,
        ),
        (
            "hashed raw-string DSL key",
            r####"stat "S" { extra r#"smuggled"#: true; }"####,
        ),
        (
            "comment between the keyword and its key",
            r###"stat "S" { option /* nothing to see */ "smuggled": true; }"###,
        ),
        (
            "comment inside a builder call",
            r###"Stat::new("S").option /* hi */ ("smuggled", json!(true))"###,
        ),
        (
            "hatch in a helper rather than the dashboard body",
            r###"
fn route_performance_dashboard() -> Dashboard {
    dashboard! { title: "Clean"; panels: smuggled(); }
}

fn smuggled() -> Vec<Panel> {
    vec![Stat::new("S").option("smuggled", json!(true)).into()]
}
"###,
        ),
        (
            "escape hatch reached through a raw panel",
            r###"RawPanel::new("piechart", "S")"###,
        ),
        (
            "field custom, spaced out",
            r###"FieldConfig::new() . custom ("k", v)"###,
        ),
        (
            "typed property escape hatch",
            r###"FieldOverride::by_name("x").property(OverrideProperty::Custom { id: "k".into(), value: v })"###,
        ),
    ] {
        assert!(
            !raw_escape_hatches(attack).is_empty(),
            "the guard must catch the {label} evasion"
        );
    }
}

/// The guard scans real source, so it has to leave real source alone.
#[test]
fn the_escape_hatch_guard_allows_legitimate_typed_authoring() {
    for (label, clean) in [
        (
            "a custom unit through the DSL",
            r###"stat "S" { unit: custom("USD"); }"###,
        ),
        (
            "a custom unit through the builders",
            r###"Stat::new("S").unit(Unit::custom("USD"))"###,
        ),
        (
            "identifiers that merely contain a forbidden one",
            r###"Timeseries::new("S").legend_options(Legend::new()).reduce_options(o)"###,
        ),
        (
            "a forbidden identifier appearing only inside a string or comment",
            r###"
// This dashboard uses no .option(…) or json! escape hatch.
let name = "Value #A";
"###,
        ),
        (
            "a value mapping",
            r###"Stat::new("S").mapping(ValueMapping::new("0", "Down"))"###,
        ),
    ] {
        assert!(
            raw_escape_hatches(clean).is_empty(),
            "the guard must not flag {label}, but flagged: {}",
            raw_escape_hatches(clean).join(", ")
        );
    }
}

/// The body of the `dashboard!` invocation that follows `marker`, with
/// braces matched over code only — the acceptance dashboard's PromQL
/// contains `{…}` inside string literals, which would otherwise close the
/// block early.
fn dashboard_block(source: &str, marker: &str) -> String {
    let classified = classify(source);
    let code = code_only(source);

    let marker_at = code
        .find(marker)
        .unwrap_or_else(|| panic!("source should contain {marker}"));
    let invocation = code[marker_at..]
        .find("dashboard!")
        .map(|at| marker_at + at)
        .unwrap_or_else(|| panic!("{marker} should be followed by a dashboard! invocation"));
    let open = code[invocation..]
        .find('{')
        .map(|at| invocation + at)
        .unwrap_or_else(|| panic!("{marker}'s dashboard! should open a block"));

    let mut depth = 0usize;
    for entry in &classified {
        if entry.offset < open || !entry.is_code {
            continue;
        }
        match entry.ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return source[open + 1..entry.offset].to_owned();
                }
            }
            _ => {}
        }
    }
    panic!("dashboard! block should close");
}

/// `examples/route_performance.rs` is a hand-maintained copy of the
/// acceptance dashboard, because an example cannot import from `tests/`.
/// The duplication is deliberate; letting the two drift apart is not — the
/// example is what a user runs and copies from, so a fix applied to one and
/// not the other would ship a dashboard nobody verified.
#[test]
fn the_route_performance_example_matches_the_acceptance_dashboard() {
    let from_test = dashboard_block(include_str!("golden.rs"), "fn route_performance_dashboard");
    let from_example = dashboard_block(include_str!("../examples/route_performance.rs"), "fn main");

    assert_eq!(
        from_test, from_example,
        "the route_performance example has drifted from the acceptance dashboard"
    );
}
