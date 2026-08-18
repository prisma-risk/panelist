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
                    legend: "{{status}}";
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
                    legend: "2xx";
                }
                query: promql!("sum(rate(http_requests_total{status=~\"4..\"}[$__rate_interval]))") {
                    legend: "4xx";
                }
                query: promql!("sum(rate(http_requests_total{status=~\"5..\"}[$__rate_interval]))") {
                    legend: "5xx";
                }
                unit: reqps;
                width: 12;
                stacking: normal;
            }

            bar_gauge "Top routes by traffic" {
                query: promql!("topk(10, sum by (route) (rate(http_requests_total[$__rate_interval])))") {
                    legend: "{{route}}";
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
                    legend: "{{route}}";
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

/// Collapses runs of whitespace (spaces, tabs, newlines) into a single
/// space so the escape-hatch grep below can't be defeated by reformatting
/// tricks like `option  "custom_flag" : true;` — rustfmt does not reformat
/// the inside of macro invocations it doesn't recognize, so extra
/// whitespace there survives `cargo fmt --check` untouched.
fn normalize_whitespace(source: &str) -> String {
    let mut normalized = String::with_capacity(source.len());
    let mut in_whitespace = false;
    for ch in source.chars() {
        if ch.is_whitespace() {
            if !in_whitespace {
                normalized.push(' ');
            }
            in_whitespace = true;
        } else {
            normalized.push(ch);
            in_whitespace = false;
        }
    }
    normalized
}

fn assert_no_raw_escape_hatches(label: &str, source: &str) {
    let normalized = normalize_whitespace(source);
    for hatch in [
        "option \"", // DSL:     option "key": value;
        ".option(",  // builder: .option("key", json!(…))
        "extra \"",  // DSL:     extra "key": value;
        ".extra(",   // builder: .extra("key", json!(…))
        ".custom(",  // builder: FieldConfig::custom("key", json!(…))
        "RawPanel",
        "RawQuery",
        "RawTransformation",
    ] {
        assert!(
            !normalized.contains(hatch),
            "{label} must not use the {hatch} escape hatch"
        );
    }
}

#[test]
fn route_performance_uses_no_raw_escape_hatches() {
    // Covers the dashboard as defined in this test file...
    let source = include_str!("golden.rs");
    let dashboard = source
        .split("fn route_performance_dashboard")
        .nth(1)
        .expect("acceptance dashboard should be defined");
    let body = &dashboard[..dashboard.find("\n}\n").expect("function should close")];
    assert_no_raw_escape_hatches("the acceptance dashboard", body);

    // ...and the hand-maintained example, which is the artifact a user
    // actually runs and copies from. It duplicates the same `dashboard!`
    // invocation rather than importing it (examples cannot import from
    // `tests/`), so nothing here otherwise guards it against drifting to
    // include an escape hatch of its own.
    let example_source = include_str!("../examples/route_performance.rs");
    assert_no_raw_escape_hatches("the route_performance example", example_source);
}
