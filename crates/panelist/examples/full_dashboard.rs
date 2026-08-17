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
        title: "GeoIP";
        uid: "geoip";
        description: "GeoIP service health and performance.";
        tags: ["geoip", "production"];
        refresh: "30s";
        datasource: prometheus("prometheus");

        variable "instance" {
            query: promql!("label_values(up{job=\"geoip\"}, instance)");
            multi: true;
            include_all: true;
        }

        row "Overview" {
            stat "Availability" {
                query: promql!("avg_over_time(up{job=\"geoip\"}[$__rate_interval])");
                unit: percent;
                width: 6;

                thresholds {
                    red: 0.0;
                    yellow: 0.99;
                    green: 0.999;
                }
            }

            stat "Requests / sec" {
                query: promql!("sum(rate(geoip_http_requests_total[$__rate_interval]))");
                unit: reqps;
                width: 6;
            }

            stat "p99 latency" {
                query: promql!(r#"histogram_quantile(0.99, sum by (le) (rate(geoip_http_request_duration_seconds_bucket[$__rate_interval])))"#);
                unit: seconds;
                width: 6;
            }

            stat "Resolution rate" {
                query: promql!("sum(rate(geoip_resolutions_total[$__rate_interval]))");
                unit: ops;
                width: 6;
            }
        }

        row "Traffic" {
            timeseries "HTTP traffic" {
                query: promql!(r#"sum by (status) (rate(geoip_http_requests_total{instance=~\"$instance\"}[$__rate_interval]))"#) {
                    legend: "{{status}}";
                }
                unit: reqps;
                width: 12;
            }

            timeseries "Latency" {
                query: promql!(r#"histogram_quantile(0.50, sum by (le) (rate(geoip_http_request_duration_seconds_bucket[$__rate_interval])))"#) {
                    legend: "p50";
                }
                query: promql!(r#"histogram_quantile(0.99, sum by (le) (rate(geoip_http_request_duration_seconds_bucket[$__rate_interval])))"#) {
                    legend: "p99";
                }
                unit: seconds;
                width: 12;

                legend {
                    position: bottom;
                    mode: table;
                    values: [last, min, max];
                }

                override field("p99") {
                    color: red;
                    line_width: 3;
                }
            }
        }

        row "Resolution" {
            timeseries "Resolution outcomes / second" {
                query: promql!(r#"sum by (outcome) (rate(geoip_resolutions_total{instance=~\"$instance\"}[$__rate_interval]))"#) {
                    legend: "{{outcome}}";
                }
                unit: ops;
                width: 12;
            }

            table "Failures" {
                query: loki!(r#"{service=\"geoip\"} |= \"resolution failed\""#);
                datasource: loki("loki");
                width: 12;
            }
        }

        row "Reference" {
            text "Runbook" {
                content: "See the **GeoIP runbook** before paging the service owner.";
                mode: markdown;
            }
        }
    };

    dashboard.validate()?;
    let _json = dashboard.to_json_pretty()?;
    Ok(())
}
