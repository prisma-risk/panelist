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

The committed [`basic.json`](../crates/panelist/tests/golden/basic.json) golden
dashboard was file-provisioned into Grafana OSS 13.1.0 during the 0.1 bootstrap.
Grafana loaded it without a provisioning error and returned it from
`GET /api/dashboards/uid/golden-service` with its row, stat, time series,
datasource references, variable, thresholds, targets, and grid positions
preserved. Automating this container check in CI remains a roadmap item.
