// SPDX-License-Identifier: Apache-2.0

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
