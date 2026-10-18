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

use std::collections::BTreeMap;

use panelist::{ResolvedDashboard, ResolvedPanel, ValidationError, prelude::*};
use serde_json::Value;

fn resolve(dashboard: &Dashboard) -> ResolvedDashboard<'_> {
    match dashboard.resolve() {
        Ok(resolved) => resolved,
        Err(error) => panic!("test dashboard should resolve: {error}"),
    }
}

fn as_value(dashboard: &Dashboard) -> Value {
    match serde_json::to_value(dashboard) {
        Ok(value) => value,
        Err(error) => panic!("test dashboard should serialize: {error}"),
    }
}

fn panel<'r, 'a>(resolved: &'r ResolvedDashboard<'a>, title: &str) -> &'r ResolvedPanel<'a> {
    match resolved.panels().find(|panel| panel.title() == title) {
        Some(panel) => panel,
        None => panic!("no panel titled {title:?}"),
    }
}

/// Every emitted panel object keyed by ID, including row headers and the panels Grafana nests inside collapsed rows.
fn emitted_panels(value: &Value) -> BTreeMap<u64, &Value> {
    fn walk<'v>(panels: &'v Value, out: &mut BTreeMap<u64, &'v Value>) {
        for panel in panels.as_array().into_iter().flatten() {
            if let Some(id) = panel["id"].as_u64() {
                out.insert(id, panel);
            }
            walk(&panel["panels"], out);
        }
    }
    let mut out = BTreeMap::new();
    walk(&value["panels"], &mut out);
    out
}

/// A dashboard that exercises every resolution rule: assigned and explicit panel IDs, assigned and explicit ref IDs, automatic and explicit grid positions, collapsed and expanded rows, and datasource inheritance at all three levels.
fn mixed_dashboard() -> Dashboard {
    Dashboard::new("Mixed")
        .datasource(prometheus("dashboard-prom"))
        .panel(
            Stat::new("Top stat")
                .query(PrometheusQuery::new("up"))
                .thresholds(Thresholds::absolute(Color::Green).step(80.0, Color::Red)),
        )
        .panel(
            Timeseries::new("Top series")
                .id(40)
                .datasource(prometheus("panel-prom"))
                .query(PrometheusQuery::new("rate(a_total[5m])").ref_id("B"))
                .query(PrometheusQuery::new("rate(b_total[5m])"))
                .query(
                    LokiQuery::new("sum(count_over_time({app=\"api\"}[5m]))")
                        .datasource(loki("query-loki")),
                ),
        )
        .row(
            Row::new("Expanded")
                .panel(Gauge::new("Expanded gauge").query(PrometheusQuery::new("gauge")))
                .panel(
                    Table::new("Pinned table")
                        .grid_pos(GridPos::new(4, 40, 10, 5))
                        .query(PrometheusQuery::new("table").format(PrometheusFormat::Table)),
                ),
        )
        .row(
            Row::new("Collapsed")
                .collapsed(true)
                .panel(
                    Stat::new("Hidden stat")
                        .id(7)
                        .query(PrometheusQuery::new("hidden").interval("1m")),
                )
                .panel(Text::new("Notes").content("No queries here.")),
        )
        .panel(Stat::new("After rows").query(PrometheusQuery::new("after")))
}

#[test]
fn panels_flatten_every_row_in_authored_order() {
    let dashboard = mixed_dashboard();
    let resolved = resolve(&dashboard);

    let titles: Vec<_> = resolved.panels().map(ResolvedPanel::title).collect();
    assert_eq!(
        titles,
        [
            "Top stat",
            "Top series",
            "Expanded gauge",
            "Pinned table",
            "Hidden stat",
            "Notes",
            "After rows",
        ]
    );

    let rows: Vec<_> = resolved
        .panels()
        .map(|panel| panel.row().map(|row| (row.title(), row.collapsed())))
        .collect();
    assert_eq!(
        rows,
        [
            None,
            None,
            Some(("Expanded", false)),
            Some(("Expanded", false)),
            Some(("Collapsed", true)),
            Some(("Collapsed", true)),
            None,
        ]
    );

    let row_titles: Vec<_> = resolved.rows().map(|row| row.title()).collect();
    assert_eq!(row_titles, ["Expanded", "Collapsed"]);
    assert_eq!(resolved.items().len(), 5);
}

#[test]
fn panels_reach_inside_collapsed_rows_where_a_top_level_json_walk_does_not() {
    let dashboard = mixed_dashboard();
    let value = as_value(&dashboard);

    let top_level: Vec<_> = value["panels"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|panel| panel["title"].as_str())
        .collect();
    assert!(!top_level.contains(&"Hidden stat"));
    assert!(
        resolve(&dashboard)
            .panels()
            .any(|panel| panel.title() == "Hidden stat")
    );
}

/// Asserts that every resolved row, panel, query, and threshold step agrees with the JSON serialization emits for the same dashboard.
fn assert_view_matches_json(dashboard: &Dashboard) {
    let resolved = resolve(dashboard);
    let value = as_value(dashboard);
    let emitted = emitted_panels(&value);
    let mut visited = 0;

    for row in resolved.rows() {
        let json = emitted[&u64::from(row.id())];
        assert_eq!(json["type"], "row");
        assert_eq!(json["title"], row.title());
        assert_eq!(json["collapsed"], row.collapsed());
        assert_eq!(json["gridPos"]["y"], row.grid_pos().y);
        assert_eq!(json["gridPos"]["w"], 24);
        visited += 1;
    }

    for panel in resolved.panels() {
        let json = emitted[&u64::from(panel.id())];
        visited += 1;
        assert_eq!(json["title"], panel.title(), "panel {}", panel.id());
        let grid = panel.grid_pos();
        assert_eq!(
            [
                &json["gridPos"]["x"],
                &json["gridPos"]["y"],
                &json["gridPos"]["w"],
                &json["gridPos"]["h"]
            ],
            [grid.x, grid.y, grid.width, grid.height],
            "{} grid",
            panel.title()
        );
        assert_eq!(
            json["datasource"]["uid"].as_str(),
            panel.datasource().map(DataSource::uid),
            "{} datasource",
            panel.title()
        );
        assert_eq!(json["interval"].as_str(), panel.min_interval());

        let targets = json["targets"].as_array().cloned().unwrap_or_default();
        assert_eq!(
            targets.len(),
            panel.queries().len(),
            "{} targets",
            panel.title()
        );
        for (query, target) in panel.queries().iter().zip(&targets) {
            assert_eq!(target["refId"], query.ref_id());
            assert_eq!(target["expr"].as_str(), query.expression());
            assert_eq!(
                target["datasource"]["uid"].as_str(),
                query.datasource().map(DataSource::uid)
            );
            assert_eq!(
                target["datasource"]["type"].as_str(),
                query.datasource().map(DataSource::kind)
            );
            // A query's own interval wins over the panel's minimum interval.
            assert_eq!(
                target["interval"].as_str().or(json["interval"].as_str()),
                query.interval()
            );
            assert_eq!(target["legendFormat"].as_str(), query.legend_format());
        }

        let thresholds = &json["fieldConfig"]["defaults"]["thresholds"];
        match panel.field_config().thresholds() {
            None => assert!(thresholds.is_null(), "{} thresholds", panel.title()),
            Some(resolved) => {
                let steps = thresholds["steps"].as_array().cloned().unwrap_or_default();
                assert_eq!(steps.len(), resolved.steps().count());
                for (step, json) in resolved.steps().zip(&steps) {
                    assert_eq!(json["value"].as_f64(), step.value());
                }
            }
        }

        let overrides = json["fieldConfig"]["overrides"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert_eq!(overrides.len(), panel.field_config().overrides().len());
        for (resolved, json) in panel.field_config().overrides().iter().zip(&overrides) {
            assert_eq!(
                json["properties"].as_array().map_or(0, Vec::len),
                resolved.properties().len()
            );
            if let OverrideMatcher::QueryRefId(ref_id) = resolved.matcher() {
                assert_eq!(json["matcher"]["id"], "byFrameRefID");
                assert_eq!(json["matcher"]["options"], ref_id.as_str());
            }
        }
    }

    // Every emitted panel object, nested or not, is accounted for by the view.
    assert_eq!(visited, emitted.len());
}

#[test]
fn resolved_values_match_the_emitted_json() {
    assert_view_matches_json(&mixed_dashboard());
}

#[test]
fn resolved_values_match_the_emitted_json_under_stable_ids() {
    let dashboard = mixed_dashboard().panel_ids(PanelIds::Stable);
    assert_view_matches_json(&dashboard);

    let resolved = resolve(&dashboard);
    // An explicit ID still wins; derived IDs come from identity rather than position.
    assert_eq!(panel(&resolved, "Top series").id(), 40);
    assert_ne!(panel(&resolved, "Top stat").id(), 1);
}

#[test]
fn keyed_tables_are_resolved_like_any_other_table() {
    let dashboard = Dashboard::new("Routes")
        .datasource(prometheus("prometheus-main"))
        .row(
            Row::new("Routes").collapsed(true).panel(
                Table::new("Route performance")
                    .keyed("route")
                    .column(
                        Column::new(
                            "RPS",
                            promql!("sum by (route) (rate(http_requests_total[$__rate_interval]))"),
                        )
                        .unit(Unit::RequestsPerSecond),
                    )
                    .column(
                        Column::new(
                            "Errors",
                            promql!(
                                "sum by (route) (rate(http_requests_total{status=~\"5..\"}[$__rate_interval]))"
                            ),
                        )
                        .thresholds(Thresholds::absolute(Color::Green).step(1.0, Color::Red)),
                    ),
            ),
        );
    assert_view_matches_json(&dashboard);

    let resolved = resolve(&dashboard);
    let table = panel(&resolved, "Route performance");
    assert!(matches!(table.kind(), PanelKind::Table));
    assert_eq!(table.row().map(|row| row.collapsed()), Some(true));
    assert_eq!(table.queries().len(), 2);
    for query in table.queries() {
        assert_eq!(
            query.datasource().map(DataSource::uid),
            Some("prometheus-main")
        );
        assert!(matches!(query.query(), Query::Prometheus(_)));
    }
    // The keyed table's per-column settings arrive as ordinary field overrides.
    assert!(!table.field_config().overrides().is_empty());
}

#[test]
fn keys_and_intervals_are_resolved() {
    let dashboard = Dashboard::new("Keys").row(
        Row::new("Traffic").key("traffic").panel(
            Timeseries::new("Requests")
                .key("requests")
                .min_interval("1m")
                .query(PrometheusQuery::new("rate(a_total[$__rate_interval])"))
                .query(PrometheusQuery::new("rate(b_total[$__rate_interval])").interval("5m")),
        ),
    );
    let resolved = resolve(&dashboard);
    let requests = panel(&resolved, "Requests");

    assert_eq!(
        resolved.rows().next().and_then(|row| row.key()),
        Some("traffic")
    );
    assert_eq!(requests.key(), Some("requests"));
    assert_eq!(requests.min_interval(), Some("1m"));
    let intervals: Vec<_> = requests
        .queries()
        .iter()
        .map(|query| query.interval())
        .collect();
    assert_eq!(intervals, [Some("1m"), Some("5m")]);
    assert_view_matches_json(&dashboard);
}

#[test]
fn ids_ref_ids_and_layout_are_resolved() {
    let dashboard = mixed_dashboard();
    let resolved = resolve(&dashboard);

    let ids: Vec<_> = resolved
        .panels()
        .map(|panel| (panel.title(), panel.id()))
        .collect();
    // Explicit IDs 40 and 7 are reserved; everything else is assigned in order, row headers included.
    assert_eq!(
        ids,
        [
            ("Top stat", 1),
            ("Top series", 40),
            ("Expanded gauge", 3),
            ("Pinned table", 4),
            ("Hidden stat", 7),
            ("Notes", 6),
            ("After rows", 8),
        ]
    );
    let row_ids: Vec<_> = resolved.rows().map(|row| row.id()).collect();
    assert_eq!(row_ids, [2, 5]);
    assert_eq!(
        panel(&resolved, "Hidden stat").row().map(|row| row.id()),
        Some(5)
    );

    let ref_ids: Vec<_> = panel(&resolved, "Top series")
        .queries()
        .iter()
        .map(|query| query.ref_id().to_owned())
        .collect();
    assert_eq!(ref_ids, ["B", "A", "C"]);

    assert_eq!(
        panel(&resolved, "Top stat").grid_pos(),
        GridPos::new(0, 0, 6, 6)
    );
    assert_eq!(
        panel(&resolved, "Top series").grid_pos(),
        GridPos::new(6, 0, 12, 8)
    );
    assert_eq!(
        panel(&resolved, "Pinned table").grid_pos(),
        GridPos::new(4, 40, 10, 5)
    );
}

#[test]
fn datasources_inherit_from_query_then_panel_then_dashboard() {
    let dashboard = mixed_dashboard();
    let resolved = resolve(&dashboard);
    assert_eq!(
        resolved.datasource().map(DataSource::uid),
        Some("dashboard-prom")
    );

    let top_stat = panel(&resolved, "Top stat");
    assert_eq!(
        top_stat.datasource().map(DataSource::uid),
        Some("dashboard-prom")
    );
    assert_eq!(
        top_stat.queries()[0].datasource().map(DataSource::uid),
        Some("dashboard-prom")
    );

    let series = panel(&resolved, "Top series");
    assert_eq!(series.datasource().map(DataSource::uid), Some("panel-prom"));
    let query_sources: Vec<_> = series
        .queries()
        .iter()
        .map(|query| query.datasource().map(DataSource::uid))
        .collect();
    assert_eq!(
        query_sources,
        [Some("panel-prom"), Some("panel-prom"), Some("query-loki")]
    );

    let bare = Dashboard::new("Bare").panel(Stat::new("Lonely").query(PrometheusQuery::new("up")));
    let bare = resolve(&bare);
    let lonely = panel(&bare, "Lonely");
    assert_eq!(lonely.datasource(), None);
    assert_eq!(lonely.queries()[0].datasource(), None);
}

#[test]
fn query_properties_are_typed() {
    let dashboard = mixed_dashboard();
    let resolved = resolve(&dashboard);

    let series = panel(&resolved, "Top series");
    let formats: Vec<_> = series
        .queries()
        .iter()
        .map(|query| query.format())
        .collect();
    // An unset Prometheus format is what Grafana reads it as; a Loki query has no format.
    assert_eq!(
        formats,
        [
            Some(PrometheusFormat::TimeSeries),
            Some(PrometheusFormat::TimeSeries),
            None
        ]
    );
    assert!(matches!(series.queries()[2].query(), Query::Loki(_)));

    let table = panel(&resolved, "Pinned table");
    assert_eq!(table.queries()[0].format(), Some(PrometheusFormat::Table));
    assert_eq!(
        panel(&resolved, "Hidden stat").queries()[0].interval(),
        Some("1m")
    );
    assert!(panel(&resolved, "Notes").queries().is_empty());
}

#[test]
fn thresholds_start_with_their_base_step() {
    let dashboard = Dashboard::new("Thresholds")
        .panel(
            Stat::new("Base only")
                .query(PrometheusQuery::new("a"))
                .thresholds(Thresholds::absolute(Color::Green)),
        )
        .panel(
            Stat::new("Stepped")
                .query(PrometheusQuery::new("b"))
                .thresholds(
                    Thresholds::percentage(Color::Blue)
                        .step(50.0, Color::Yellow)
                        .step(80.0, Color::Red),
                ),
        )
        .panel(Stat::new("Unset").query(PrometheusQuery::new("c")));
    let resolved = resolve(&dashboard);

    let steps = |title: &str| -> Vec<(Color, Option<f64>)> {
        panel(&resolved, title)
            .field_config()
            .thresholds()
            .map(|thresholds| {
                thresholds
                    .steps()
                    .map(|step| (step.color().clone(), step.value()))
                    .collect()
            })
            .unwrap_or_default()
    };

    assert_eq!(steps("Base only"), [(Color::Green, None)]);
    assert_eq!(
        steps("Stepped"),
        [
            (Color::Blue, None),
            (Color::Yellow, Some(50.0)),
            (Color::Red, Some(80.0)),
        ]
    );
    assert!(
        panel(&resolved, "Unset")
            .field_config()
            .thresholds()
            .is_none()
    );
    assert_eq!(
        panel(&resolved, "Stepped")
            .field_config()
            .thresholds()
            .map(|thresholds| thresholds.mode()),
        Some(ThresholdMode::Percentage)
    );
    assert_view_matches_json(&dashboard);
}

#[test]
fn query_field_properties_resolve_into_overrides_at_their_ref_ids() {
    let dashboard = Dashboard::new("Per query")
        .datasource(prometheus("prometheus-main"))
        .panel(
            Timeseries::new("Latency")
                .query(
                    PrometheusQuery::new("histogram_quantile(0.99, rate(latency_bucket[5m]))")
                        .unit(Unit::Seconds)
                        .instant(),
                )
                .query(PrometheusQuery::new("rate(requests_total[5m])"))
                .query(
                    PrometheusQuery::new("rate(errors_total[5m])")
                        .ref_id("ERR")
                        .query_type(QueryType::Range)
                        .color(Color::Red),
                )
                .override_field(FieldOverride::by_query("ERR").decimals(2)),
        );
    assert_view_matches_json(&dashboard);

    let resolved = resolve(&dashboard);
    let latency = panel(&resolved, "Latency");
    let types: Vec<_> = latency
        .queries()
        .iter()
        .map(|query| query.query_type())
        .collect();
    assert_eq!(
        types,
        [Some(QueryType::Instant), None, Some(QueryType::Range)]
    );
    assert_eq!(
        latency.queries()[0].field_properties(),
        [OverrideProperty::Unit(Unit::Seconds)]
    );

    // Query properties come first, at each query's resolved ref ID, then the panel's own overrides.
    let matchers: Vec<_> = latency
        .field_config()
        .overrides()
        .iter()
        .map(|item| item.matcher().clone())
        .collect();
    assert_eq!(
        matchers,
        [
            OverrideMatcher::QueryRefId("A".into()),
            OverrideMatcher::QueryRefId("ERR".into()),
            OverrideMatcher::QueryRefId("ERR".into()),
        ]
    );
    assert_eq!(
        latency.field_config().overrides()[2].properties(),
        [OverrideProperty::Decimals(2)]
    );
}

#[test]
fn raw_queries_without_an_expression_resolve_to_none() {
    let dashboard = Dashboard::new("Raw").panel(
        RawPanel::new("table", "Rows")
            .datasource(DataSource::new(
                "grafana-postgresql-datasource",
                "postgres-main",
            ))
            .query(RawQuery::new().extra("rawSql", serde_json::json!("SELECT 1"))),
    );
    assert_view_matches_json(&dashboard);

    let resolved = resolve(&dashboard);
    let query = &panel(&resolved, "Rows").queries()[0];
    assert_eq!(query.expression(), None);
    assert_eq!(query.format(), None);
    assert_eq!(
        query.datasource().map(DataSource::kind),
        Some("grafana-postgresql-datasource")
    );
}

#[test]
fn field_defaults_and_overrides_are_typed() {
    let dashboard = Dashboard::new("Fields").panel(
        Timeseries::new("Latency")
            .query(PrometheusQuery::new("p99").ref_id("P99"))
            .unit(Unit::Seconds)
            .min(0.0)
            .decimals(2)
            .display_name("${__field.labels.pod}")
            .color(ColorScheme::ClassicPaletteByName)
            .override_field(
                FieldOverride::by_query("P99")
                    .color(Color::Red)
                    .unit(Unit::Milliseconds),
            )
            .override_field(
                FieldOverride::by_name("errors")
                    .thresholds(Thresholds::absolute(Color::Green).step(1.0, Color::Red)),
            ),
    );
    let resolved = resolve(&dashboard);
    let fields = panel(&resolved, "Latency").field_config();

    assert_eq!(fields.unit(), Some(&Unit::Seconds));
    assert_eq!(fields.min(), Some(0.0));
    assert_eq!(fields.max(), None);
    assert_eq!(fields.decimals(), Some(2));
    assert_eq!(fields.display_name(), Some("${__field.labels.pod}"));
    assert_eq!(fields.color(), Some(&ColorScheme::ClassicPaletteByName));

    let [by_query, by_name] = fields.overrides() else {
        panic!("expected two overrides");
    };
    assert_eq!(
        by_query.matcher(),
        &OverrideMatcher::QueryRefId("P99".into())
    );
    assert_eq!(
        by_query.properties(),
        [
            OverrideProperty::Color(ColorScheme::Fixed(Color::Red)),
            OverrideProperty::Unit(Unit::Milliseconds),
        ]
    );

    // Override thresholds resolve through the same view as field defaults.
    let [OverrideProperty::Thresholds(thresholds)] = by_name.properties() else {
        panic!("expected one thresholds property");
    };
    let steps: Vec<_> = panelist::ResolvedThresholds::from(thresholds)
        .steps()
        .map(|step| step.value())
        .collect();
    assert_eq!(steps, [None, Some(1.0)]);
}

#[test]
fn resolve_reports_the_same_errors_as_validate() {
    let dashboard = Dashboard::new("Broken")
        .panel(Stat::new("One").id(3).query(PrometheusQuery::new("a")))
        .panel(Stat::new("Two").id(3).query(PrometheusQuery::new("b")));

    let Err(errors) = dashboard.resolve() else {
        panic!("duplicate IDs must not resolve");
    };
    assert!(
        errors
            .errors()
            .iter()
            .any(|error| matches!(error, ValidationError::DuplicatePanelId { id: 3, .. }))
    );
    assert_eq!(dashboard.validate().err(), Some(errors));
}
