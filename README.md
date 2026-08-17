# Panelist

Panelist is a Rust library for writing Grafana dashboards as concise, strongly
typed code.

```rust
use panelist::prelude::*;

let dashboard = dashboard! {
    title: "Service health";
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
dashboard.write_json("service.json")?;
# Ok::<(), panelist::Error>(())
```

Panel IDs, query reference IDs, and 24-column grid positions are assigned
automatically and deterministically. The resulting JSON is stable enough to
review in Git and close to Grafana's native dashboard model.

## Status

Panelist is under active development. Add the current release from crates.io:

```toml
[dependencies]
panelist = "0.1"
```

The minimum supported Rust version is 1.96. Panelist uses Rust 2024 and has no
async runtime or network client dependency.

## Macro DSL

The macro translates a small amount of syntax into ordinary typed builders. It
does not add loops, conditions, interpolation, or its own composition language:
Rust remains the composition language.

```rust
use panelist::prelude::*;

let dashboard = dashboard! {
    title: "API";
    uid: "api";
    description: "API health and performance.";
    tags: ["api", "production"];
    refresh: "30s";
    datasource: prometheus("prometheus-main");

    variable "instance" {
        query: promql!("label_values(up{job=\"api\"}, instance)");
        multi: true;
        include_all: true;
    }

    row "Latency" {
        timeseries "Request latency" {
            query: promql!("histogram_quantile(0.99, rate(request_duration_seconds_bucket[$__rate_interval]))") {
                legend: "p99";
            }
            unit: seconds;
            width: 12;

            legend {
                position: bottom;
                mode: table;
                values: [last, min, max];
            }
        }
    }
};
# let _ = dashboard;
```

The DSL supports `timeseries`, `stat`, `gauge`, `table`, `text`, `bar_gauge`,
and `heatmap` panels. `promql!` and `loki!` create typed query builders rather
than opaque JSON values.

## Builder API

Every important DSL construct has a normal Rust equivalent for IDE discovery,
dynamic generation, and users who prefer builders:

```rust
use panelist::prelude::*;

let panel = Timeseries::new("Requests")
    .query(
        PrometheusQuery::new("rate(http_requests_total[$__rate_interval])")
            .legend("{{status}}"),
    )
    .unit(Unit::RequestsPerSecond)
    .width(12);

let dashboard = Dashboard::new("HTTP")
    .datasource(prometheus("prometheus-main"))
    .row(Row::new("Traffic").panel(panel));
# let _ = dashboard;
```

Typed models cover dashboard metadata, rows, datasources, Prometheus and Loki
queries, query/custom/constant variables, time ranges, links, field defaults,
thresholds, legends, and field overrides. `RawQuery`, `RawPanel`, and ordered
`extra`/`option` methods provide explicit escape hatches for Grafana features
that Panelist does not model yet.

## Automatic layout

Grafana dashboards use 24 columns. Panelist places panels from left to right,
wraps before a panel would exceed column 24, and advances following rows below
the tallest panel on each line. Defaults are visualization-specific; use
`.width()`, `.height()`, or `width:`/`height:` in the DSL to adjust them.

An explicit `GridPos` remains available for unusual layouts:

```rust
# use panelist::prelude::*;
let panel = Timeseries::new("Pinned")
    .grid_pos(GridPos::new(6, 10, 12, 8));
# let _ = panel;
```

Expanded rows serialize their panels at the dashboard level, as Grafana Classic
expects. Collapsed rows retain their panels inside the row object and only
consume the one-unit row header while collapsed.

## Reusable components

Fragments are ordinary Rust values:

```rust
use panelist::prelude::*;

fn http_panels(service: &str) -> Vec<Panel> {
    vec![
        Timeseries::new("Requests")
            .query(PrometheusQuery::new(format!(
                "sum(rate({service}_requests_total[$__rate_interval]))"
            )))
            .into(),
        Stat::new("Errors")
            .query(PrometheusQuery::new(format!(
                "sum(rate({service}_errors_total[$__rate_interval]))"
            )))
            .into(),
    ]
}

let panels = http_panels("checkout");
let dashboard = dashboard! {
    title: "Checkout";
    row "HTTP" {
        panels: panels;
    }
};
# let _ = dashboard;
```

## Serialization and validation

`Dashboard` implements `serde::Serialize`, so normal `serde_json` APIs work.
The convenience methods add structured errors and file output:

```rust
# use panelist::prelude::*;
# let dashboard = Dashboard::new("Example");
dashboard.validate()?;
let compact = dashboard.to_json()?;
let pretty = dashboard.to_json_pretty()?;
dashboard.write_json("dashboard.json")?;
# let _ = (compact, pretty);
# Ok::<(), panelist::Error>(())
```

Validation reports duplicate explicit panel IDs and query refs, missing titles
or expressions, invalid widths and positions, zero heights, and malformed
threshold ordering. Invalid authored dashboards return errors instead of
panicking.

## Grafana compatibility

Panelist 0.1 emits Grafana Classic dashboard JSON with schema version 41. This
is deliberate: Classic remains importable/exportable in Grafana 13, works with
file provisioning, and retains the numeric panel IDs and `gridPos` behavior
that Panelist automates. Grafana's newer V2 resource model uses a different
layout representation.

The authoring model, layout/normalization pass, and Grafana serialization model
are separate modules so a future V2 serializer does not have to break the DSL or
builders. See [the schema strategy](https://github.com/prisma-risk/panelist/blob/main/docs/schema-compatibility.md)
for scope and references.

## Examples

The package examples are compiled as part of the normal test suite:

- [`basic.rs`](crates/panelist/examples/basic.rs) — smallest useful dashboard
- [`prometheus.rs`](crates/panelist/examples/prometheus.rs) — multiple targets,
  thresholds, overrides, and legend calculations
- [`variables.rs`](crates/panelist/examples/variables.rs) — query and custom
  variables
- [`layout.rs`](crates/panelist/examples/layout.rs) — automatic wrapping and a
  reusable Rust fragment
- [`full_dashboard.rs`](crates/panelist/examples/full_dashboard.rs) — realistic
  GeoIP dashboard using Prometheus and Loki

Run one with `cargo run -p panelist --example basic`.

## Roadmap

- Validate generated dashboards against live Grafana 13 in CI.
- Add an opt-in Grafana V2 resource serializer and dynamic layouts.
- Expand panel-specific typed options, transformations, annotations, mappings,
  and data links.
- Add more datasource query types without turning the core crate into an API
  client.
- Stabilize the API from real-world dashboard authoring feedback.

## Development

The repository is a Rust 2024 virtual workspace. Run `make ci` before sending a
change; it checks formatting, Clippy, builds, tests, rustdoc, source headers,
packaging, dependency licenses, and security advisories. See
[`CONTRIBUTING.md`](CONTRIBUTING.md) for the full workflow.

Panelist is licensed under [Apache-2.0](LICENSE).
