// SPDX-License-Identifier: Apache-2.0

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
