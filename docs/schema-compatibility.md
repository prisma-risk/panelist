<!-- SPDX-License-Identifier: Apache-2.0 -->
# Grafana schema compatibility

Panelist emits the Grafana Classic dashboard JSON model, at the schema version of the [Grafana target](#grafana-targets) the dashboard is written for. The choice of Classic is a compatibility boundary, not an attempt to claim that Classic is Grafana's newest schema.

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

## Grafana targets

`GrafanaTarget` names the Grafana release line a dashboard is written for. It chooses the emitted `schemaVersion`, and it is the single place where a wire difference between Grafana releases is gated.

The schema version matters because Grafana runs every Classic migration newer than a dashboard's stored `schemaVersion` each time it loads that dashboard, and writes the migrated version back on save. Emitting a version older than the running Grafana's current one makes Grafana rewrite the dashboard, so its export no longer matches the committed file. Emitting a newer one tells Grafana to skip migrations: an older Grafana relabels the dashboard with its own current version and runs nothing.

Each target's number is `DASHBOARD_SCHEMA_VERSION` in [`public/app/features/dashboard/state/DashboardMigrator.ts`](https://github.com/grafana/grafana/blob/v13.0.2/public/app/features/dashboard/state/DashboardMigrator.ts) at the verified release tag. Grafana 13's backend constant `LATEST_VERSION` in [`apps/dashboard/pkg/migration/schemaversion/migrations.go`](https://github.com/grafana/grafana/blob/v13.0.2/apps/dashboard/pkg/migration/schemaversion/migrations.go) agrees.

| Target | Verified release | `schemaVersion` |
| --- | --- | --- |
| `V11_5` | 11.5.2 | 40 |
| `V13` (default) | 13.0.2 | 42 |

The same constant at the release tags in between, read on 2026-10-17: 11.0.0 has 39; 11.4.0 and 11.5.2 have 40; 11.6.0, 12.0.0 and 12.1.0 have 41; 12.2.0, 12.3.0, 12.4.0 and 13.0.2 have 42. Two migrations separate 40 from 42. Version 41 deletes the legacy `timepicker.time_options` key, which Panelist never emits. Version 42 makes every `custom.hideFrom` override with `viz: true` also set `tooltip: true`; Panelist has no typed `hideFrom`, so the only way to emit one is a raw override, and at 42 Grafana takes that value as written. Grafana 13's migrator calls 42 the final Classic version: new migrations go into panel plugins instead.

For everything Panelist emits today, the two targets differ only in `schemaVersion`. The goldens rendered for each target are byte-identical apart from that line, and a golden test enforces it, so a future gate has to be listed there on purpose.

`Dashboard::schema_version(N)` stays as an escape hatch for releases no target covers, such as 11.6 through 12.1. It overrides only the number; every other property is still shaped for the target.

## Serialization contract

The public authoring types do not expose the internal Grafana structs. A
serialization pass:

1. validates the authored dashboard,
2. reserves explicit IDs and assigns the missing ones, positionally or from a stable hash of each panel's identity,
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
- Every `GrafanaTarget` must have its own golden directory and a round trip through its verified release in `make verify-grafana`. A target is added only together with that proof, and a target's `schemaVersion` changes only with a new proof.
- The default target moves to a newer release only in a minor release, because it changes `schemaVersion` for every dashboard that does not name a target.
- Stable panel IDs are part of the output contract. The derivation documented on `PanelIds::Stable` never changes; a different derivation would be a new `PanelIds` variant.
- Raw query/panel/extra values are best-effort escape hatches and cannot carry
  the same cross-version compatibility guarantee as typed properties.

## Current import proof

On 2026-10-17, every committed golden dashboard was rendered for each target and round-tripped through that target's release: the goldens in [`tests/golden/v11_5`](../crates/panelist/tests/golden/v11_5) through `grafana/grafana-oss:11.5.2`, and the goldens in [`tests/golden`](../crates/panelist/tests/golden) through `grafana/grafana-oss:13.0.2`. The script checks that each container reports exactly the expected version before posting anything. All six round trips came back with zero dropped, altered, retyped, or blocked properties:

| Golden | Leaves | Empty containers | 11.5.2 | 13.0.2 |
| --- | --- | --- | --- | --- |
| `basic.json` | 97 | 6 | pass | pass |
| `route_performance.json` | 483 | 18 | pass | pass |
| `stable_ids.json` | 109 | 12 | pass | pass |

`stable_ids.json` uses `PanelIds::Stable`, so this also proves both releases store panel IDs from the stable range (up to nine digits, on rows, keyed and unkeyed panels, and panels nested in a collapsed row) unchanged. Grafana's dashboard schema types a panel `id` as `uint32` in both releases.

The proof is about storage through the HTTP API, which is what provisioning and import rely on. A probe the same day posted the previous goldens, at `schemaVersion` 41, to Grafana 11.5.2, whose current version is 40, and got 41 back, so the legacy API does not migrate on save; the migrations described above run in the browser when a dashboard is opened.

The rest of this section records the method and the first proof.

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
3. Walks the posted document in two categories, checked to different
   standards:
   - Every **leaf** — every value that is not itself an object or array,
     including `null` and `false` — must still be present, with the same
     value, at the same path in the returned document.
   - Every **empty container** (`{}` or `[]` in the posted document) must
     still be present as the same container type (object stays an object,
     array stays an array), but its *value* is not compared. Grafana
     routinely adds entries into an empty container on save —
     `annotations.list: []` picking up Grafana's built-in "Annotations &
     Alerts" entry is a documented case — and that addition is exactly the
     kind this check already ignores everywhere else, so comparing values
     here would misreport a legitimate addition as a drop.

   Additions elsewhere are ignored automatically in both categories (the
   walk only visits paths the posted document has). The only rewrites
   ignored are the ones Grafana is documented to make on every save: the
   top-level `id`, `version`, and `uid`, and each panel's `pluginVersion`.
   Anything else missing, value-altered (a leaf), or retyped (a container)
   is reported with its JSON path.

   A fourth outcome, `blocked`, covers the case where a checked path cannot
   be reached because Grafana returned a scalar where the posted document
   had a non-empty container. The path walk resolves one segment at a time
   for this reason: jq's `getpath` treats indexing *through* a scalar as a
   hard error, which would abort the comparison — and, under
   `set -euo pipefail`, the whole run, silently skipping every golden later
   in the loop. A `blocked` report names the offending ancestor and the
   type it came back as.

Both goldens round-tripped with **zero dropped, altered, retyped, or
blocked properties**:

- [`basic.json`](../crates/panelist/tests/golden/basic.json) (uid
  `golden-service`, 97 leaf properties and 6 empty containers checked) —
  rows, stat and time-series panels, thresholds, datasource references, a
  custom variable, and grid positions all came back unchanged, including
  every `false`-valued `hide` flag on a query target, every `collapsed`
  flag on a row, and every empty container (`timepicker`, `annotations.list`,
  `links`, the time-series panel's empty `options.legend.calcs`, and both
  panels' empty `fieldConfig.overrides` arrays).
- [`route_performance.json`](../crates/panelist/tests/golden/route_performance.json)
  (uid `route-performance`, 483 leaf properties and 18 empty containers
  checked) — the acceptance dashboard for the whole typed-transformations
  effort. Confirmed preserved:
  panel transformations (`timeSeriesTable`, `joinByField`, and `organize`,
  chained on the table panel), typed table cell options
  (`color-background` and `sparkline` cell renderers, including the
  sparkline's `hideValue`/`lineWidth`), typed table column sorting
  (`options.sortBy`), Prometheus result formats (`time_series`, `table`, and
  `heatmap` all exercised across the dashboard's queries), and typed heatmap
  options (color scheme, color steps, cell gap, calculate, and Y axis unit
  and placement).

Run `make verify-grafana` to reproduce this check locally, or `scripts/verify-grafana.sh 11.5.2` for one release. Set `VERIFY_GRAFANA_PORT` when port 3000 is taken. It needs Docker and network access, which is why it is not part of `make ci` or the CI workflow; wiring it into CI remains a roadmap item.
