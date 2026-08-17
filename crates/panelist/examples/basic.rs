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
        title: "Service";
        description: "A compact service-health dashboard.";
        refresh: "30s";

        row "Traffic" {
            timeseries "Requests" {
                query: promql!("sum(rate(requests_total[$__rate_interval]))");
                unit: reqps;
                width: 12;
            }

            stat "Errors" {
                query: promql!("sum(rate(errors_total[$__rate_interval]))");
                unit: reqps;
                width: 12;
            }
        }
    };

    dashboard.validate()?;
    let _json = dashboard.to_json_pretty()?;
    Ok(())
}
