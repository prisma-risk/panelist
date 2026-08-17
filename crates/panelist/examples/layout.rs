// SPDX-License-Identifier: Apache-2.0

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
