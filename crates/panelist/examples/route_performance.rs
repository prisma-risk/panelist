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

// Examples print the dashboard JSON so that `cargo run --example <name>`
// is useful on its own, and so the demo stack in `examples/demo` can
// provision the very same dashboards a reader builds. The workspace
// otherwise denies printing to stdout.
#![allow(clippy::print_stdout)]

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
            // One row per route, one column per query. The keyed table owns the reference IDs, the join, Grafana's `Value #<refId>` and `Trend #<refId>` field names, the column order, and one override per column, so each column name is written once. The macro has no keyed-table syntax yet, so the table is built with the typed builder and added through `panels:`.
            panels: [Table::new("Route performance")
                .width(24)
                .sort_by("p95", SortDirection::Descending)
                .keyed("route")
                .column(
                    Column::new("RPS", promql!("sum by (route) (rate(http_requests_total[$__rate_interval]))"))
                        .unit(Unit::RequestsPerSecond),
                )
                .column(
                    Column::new("4xx rate", promql!("sum by (route) (rate(http_requests_total{status=~\"4..\"}[$__rate_interval]))"))
                        .unit(Unit::RequestsPerSecond),
                )
                .column(
                    Column::new("5xx rate", promql!("sum by (route) (rate(http_requests_total{status=~\"5..\"}[$__rate_interval]))"))
                        .unit(Unit::RequestsPerSecond),
                )
                .column(
                    Column::new("error %", promql!("sum by (route) (rate(http_requests_total{status=~\"5..\"}[$__rate_interval])) / sum by (route) (rate(http_requests_total[$__rate_interval]))"))
                        .unit(Unit::PercentUnit)
                        .cell(TableCellType::ColoredBackground)
                        .thresholds(Thresholds::new().green(0.0).yellow(0.01).red(0.05)),
                )
                .column(
                    Column::new("p50", promql!("histogram_quantile(0.50, sum by (route, le) (rate(http_request_duration_seconds_bucket[$__rate_interval])))"))
                        .unit(Unit::Seconds),
                )
                .column(
                    Column::new("p95", promql!("histogram_quantile(0.95, sum by (route, le) (rate(http_request_duration_seconds_bucket[$__rate_interval])))"))
                        .unit(Unit::Seconds)
                        .cell(TableCellType::ColoredBackground)
                        .thresholds(Thresholds::new().green(0.0).yellow(0.3).red(1.0)),
                )
                .column(
                    Column::new("p99", promql!("histogram_quantile(0.99, sum by (route, le) (rate(http_request_duration_seconds_bucket[$__rate_interval])))"))
                        .unit(Unit::Seconds),
                )
                .column(
                    Column::trend("Trend", promql!("histogram_quantile(0.95, sum by (route, le) (rate(http_request_duration_seconds_bucket[$__rate_interval])))"), Reducer::Last)
                        .cell(SparklineCell::new().hide_value(true).line_width(2.0)),
                )];
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
    println!("{}", dashboard.to_json_pretty()?);
    Ok(())
}
