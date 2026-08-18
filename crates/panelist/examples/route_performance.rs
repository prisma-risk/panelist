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

use panelist::prelude::*;

fn main() -> panelist::Result<()> {
    let dashboard = dashboard! {
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
    };

    dashboard.validate()?;
    let _json = dashboard.to_json_pretty()?;
    Ok(())
}
