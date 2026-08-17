// SPDX-License-Identifier: Apache-2.0

use panelist::prelude::*;

fn main() -> panelist::Result<()> {
    let latency = Timeseries::new("Request latency")
        .query(
            PrometheusQuery::new(
                "histogram_quantile(0.50, sum by (le) (rate(request_duration_seconds_bucket[$__rate_interval])))",
            )
            .legend("p50"),
        )
        .query(
            PrometheusQuery::new(
                "histogram_quantile(0.99, sum by (le) (rate(request_duration_seconds_bucket[$__rate_interval])))",
            )
            .legend("p99"),
        )
        .unit(Unit::Seconds)
        .thresholds(
            Thresholds::new()
                .green(0.0)
                .yellow(0.5)
                .red(1.0),
        )
        .legend_options(
            Legend::new()
                .mode(LegendMode::Table)
                .calculations([
                    LegendCalculation::Last,
                    LegendCalculation::Min,
                    LegendCalculation::Max,
                ]),
        );

    let dashboard = Dashboard::new("Prometheus queries")
        .datasource(prometheus("prometheus"))
        .row(Row::new("Latency").panel(latency));

    let _json = dashboard.to_json_pretty()?;
    Ok(())
}
