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
