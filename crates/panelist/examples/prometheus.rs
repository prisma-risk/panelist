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

// Examples print the dashboard JSON so that `cargo run --example <name>`
// is useful on its own, and so the demo stack in `examples/demo` can
// provision the very same dashboards a reader builds. The workspace
// otherwise denies printing to stdout.
#![allow(clippy::print_stdout)]

use panelist::prelude::*;

fn main() -> panelist::Result<()> {
    let latency = Timeseries::new("Request latency")
        .query(
            PrometheusQuery::new(
                "histogram_quantile(0.50, sum by (le) (rate(request_duration_seconds_bucket[$__rate_interval])))",
            )
            .legend_format("p50"),
        )
        .query(
            PrometheusQuery::new(
                "histogram_quantile(0.99, sum by (le) (rate(request_duration_seconds_bucket[$__rate_interval])))",
            )
            .legend_format("p99"),
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
                .calculations([Reducer::Last, Reducer::Min, Reducer::Max]),
        );

    let dashboard = Dashboard::new("Prometheus queries")
        .uid("prometheus-queries")
        .datasource(prometheus("prometheus"))
        .row(Row::new("Latency").panel(latency));

    dashboard.validate()?;
    println!("{}", dashboard.to_json_pretty()?);
    Ok(())
}
