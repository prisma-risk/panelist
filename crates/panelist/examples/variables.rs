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
        title: "Variables";
        datasource: prometheus("prometheus");

        variable "environment" {
            values: ["production", "staging"];
            default: "production";
        }

        variable "instance" {
            query: promql!("label_values(up{environment=\"$environment\"}, instance)");
            multi: true;
            include_all: true;
            refresh: time_range;
        }

        row "Fleet" {
            timeseries "Instances up" {
                query: promql!("sum(up{instance=~\"$instance\"})");
                unit: short;
            }
        }
    };

    let _json = dashboard.to_json_pretty()?;
    Ok(())
}
