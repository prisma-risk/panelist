<!-- SPDX-License-Identifier: Apache-2.0 -->
# Grafana schema compatibility

Panelist 0.2 emits the Grafana Classic dashboard JSON model at schema version
41. The choice is a compatibility boundary, not an attempt to claim that
Classic is Grafana's newest schema.

Grafana 13 makes the V2 resource model and dynamic layouts generally available.
Grafana nevertheless continues to import and export Classic dashboards, and
its provisioning documentation explicitly supports Classic files. Classic is
also the model with numeric panel IDs, row panels, targets, and a 24-column
`gridPos`, which are the behaviors Panelist automates.

Primary references:

- [Grafana dashboard JSON models](https://grafana.com/docs/grafana/latest/visualizations/dashboards/build-dashboards/view-dashboard-json-model/)
- [Grafana dashboard HTTP API](https://grafana.com/docs/grafana/latest/developer-resources/api-reference/http-api/dashboard/)
- [Grafana dashboard provisioning](https://grafana.com/docs/grafana/latest/administration/provisioning/)
- [Grafana Foundation SDK](https://github.com/grafana/grafana-foundation-sdk)
- [Grafana V2 dashboard schema source](https://github.com/grafana/grafana/blob/main/apps/dashboard/pkg/apis/dashboard/v2/dashboard_spec.cue)

The Foundation SDK is a naming and schema reference. It currently publishes
builders for Go, TypeScript, Python, Java, and PHP, but not Rust. Panelist does
not reproduce its generated builder shape; its value is the smaller semantic
API, deterministic layout, and Rust-native composition.

## Serialization contract

The public authoring types do not expose the internal Grafana structs. A
serialization pass:

1. validates the authored dashboard,
2. reserves explicit IDs and assigns deterministic missing IDs,
3. assigns missing target refs in `A`, `B`, … order,
4. resolves dashboard, panel, and query datasource inheritance,
5. lays panels out in deterministic 24-column rows,
6. converts semantic units, thresholds, legends, fields, variables, and links,
7. serializes ordered maps and structs to stable JSON.

This boundary permits a future `GrafanaV2` output target to share the public
builders while replacing only layout normalization and the wire model.

## Compatibility policy

- A patch release must not change pretty JSON for identical authoring input
  unless it fixes invalid Grafana output; such a fix needs a golden-file diff.
- A minor release may add omitted properties or new typed features, but should
  preserve existing builder and DSL behavior.
- A schema-version bump must cite the Grafana change it adopts and include an
  import/provisioning verification against that Grafana release.
- Raw query/panel/extra values are best-effort escape hatches and cannot carry
  the same cross-version compatibility guarantee as typed properties.

## Current import proof

On 2026-08-17, both committed golden dashboards were round-tripped through a
real `grafana/grafana-oss:13.0.2` instance — the newest 13.x image published
to Docker Hub at the time. (An earlier version of this document claimed the
0.1 bootstrap import was verified against "Grafana OSS 13.1.0"; that version
was never published, the claim was wrong, and this section replaces it
rather than repeating it.)

The check is deliberately stricter than "Grafana accepted the POST and
returned 200." Grafana accepts a dashboard save even when it silently drops
a property it does not understand — a 200 status proves Panelist emitted
valid JSON, not that Grafana kept what was in it. So for each golden,
[`scripts/verify-grafana.sh`](../scripts/verify-grafana.sh):

1. `POST /api/dashboards/db` the golden JSON with `overwrite: true`.
2. `GET /api/dashboards/uid/<uid>` it straight back.
3. Walks every leaf of the posted document and asserts it is still present,
   with the same value, at the same path in the returned document. Additions
   are ignored automatically (the walk only visits paths the posted document
   has). The only rewrites ignored are the ones Grafana is documented to make
   on every save: the top-level `id`, `version`, and `uid`, and each panel's
   `pluginVersion`. Anything else missing or changed is reported as a dropped
   or altered property, with its JSON path.

Both goldens round-tripped with **zero dropped or altered properties**:

- [`basic.json`](../crates/panelist/tests/golden/basic.json) (uid
  `golden-service`, 84 leaf properties checked) — rows, stat and time-series
  panels, thresholds, datasource references, a custom variable, and grid
  positions all came back unchanged.
- [`route_performance.json`](../crates/panelist/tests/golden/route_performance.json)
  (uid `route-performance`, 430 leaf properties checked) — the acceptance
  dashboard for the whole typed-transformations effort. Confirmed preserved:
  panel transformations (`timeSeriesTable`, `joinByField`, and `organize`,
  chained on the table panel), typed table cell options
  (`color-background` and `sparkline` cell renderers, including the
  sparkline's `hideValue`/`lineWidth`), typed table column sorting
  (`options.sortBy`), Prometheus result formats (`time_series`, `table`, and
  `heatmap` all exercised across the dashboard's queries), and typed heatmap
  options (color scheme, color steps, cell gap, calculate, and Y axis unit
  and placement).

Run `make verify-grafana` to reproduce this check locally. It needs Docker
and network access, which is why it is not part of `make ci` or the CI
workflow; wiring it into CI remains a roadmap item.
