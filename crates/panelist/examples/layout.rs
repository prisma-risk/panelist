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

fn service_stats(service: &str) -> Vec<Panel> {
    vec![
        Stat::new("Requests")
            .query(PrometheusQuery::new(format!(
                "sum(rate({service}_requests_total[$__rate_interval]))"
            )))
            .unit(Unit::RequestsPerSecond)
            .width(8)
            .into(),
        Stat::new("Errors")
            .query(PrometheusQuery::new(format!(
                "sum(rate({service}_errors_total[$__rate_interval]))"
            )))
            .unit(Unit::RequestsPerSecond)
            .width(8)
            .into(),
        Gauge::new("Availability")
            .query(PrometheusQuery::new(format!(
                "avg_over_time(up{{job=\"{service}\"}}[$__rate_interval])"
            )))
            .unit(Unit::Percent)
            .width(8)
            .into(),
    ]
}

fn main() -> panelist::Result<()> {
    let panels = service_stats("checkout");
    let dashboard = dashboard! {
        title: "Automatic layout";

        row "Golden signals" {
            panels: panels;
        }
    };

    let _json = dashboard.to_json_pretty()?;
    Ok(())
}
