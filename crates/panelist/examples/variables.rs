// SPDX-License-Identifier: Apache-2.0

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
