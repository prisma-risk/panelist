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

use panelist::prelude::*;
use serde_json::{Value, json};

fn as_value(dashboard: &Dashboard) -> Value {
    match serde_json::to_value(dashboard) {
        Ok(value) => value,
        Err(error) => panic!("test dashboard should serialize: {error}"),
    }
}

#[test]
fn assigns_deterministic_panel_and_query_ids() {
    let dashboard = Dashboard::new("IDs").row(
        Row::new("Queries").panel(
            Timeseries::new("Traffic")
                .query(PrometheusQuery::new("rate(foo_total[5m])"))
                .query(PrometheusQuery::new("rate(bar_total[5m])")),
        ),
    );

    let value = as_value(&dashboard);
    assert_eq!(value["panels"][0]["id"], 1);
    assert_eq!(value["panels"][1]["id"], 2);
    assert_eq!(value["panels"][1]["targets"][0]["refId"], "A");
    assert_eq!(value["panels"][1]["targets"][1]["refId"], "B");
    assert_eq!(
        dashboard.to_json_pretty().unwrap(),
        dashboard.to_json_pretty().unwrap()
    );
}

#[test]
fn explicit_ids_and_refs_are_preserved_and_reserved() {
    let dashboard = Dashboard::new("Explicit").row(
        Row::new("Row").panel(
            Timeseries::new("Panel")
                .id(1)
                .query(PrometheusQuery::new("foo").ref_id("B"))
                .query(PrometheusQuery::new("bar")),
        ),
    );

    let value = as_value(&dashboard);
    assert_eq!(value["panels"][0]["id"], 2);
    assert_eq!(value["panels"][1]["id"], 1);
    assert_eq!(value["panels"][1]["targets"][0]["refId"], "B");
    assert_eq!(value["panels"][1]["targets"][1]["refId"], "A");
}

#[test]
fn wraps_panels_after_twenty_four_columns_and_places_following_rows() {
    let dashboard = Dashboard::new("Layout")
        .row(
            Row::new("First")
                .panel(Timeseries::new("A").width(12))
                .panel(Timeseries::new("B").width(12))
                .panel(Timeseries::new("C").width(8)),
        )
        .row(Row::new("Second").panel(Timeseries::new("D").width(24)));

    let value = as_value(&dashboard);
    assert_eq!(
        value["panels"][1]["gridPos"],
        json!({"x": 0, "y": 1, "w": 12, "h": 8})
    );
    assert_eq!(
        value["panels"][2]["gridPos"],
        json!({"x": 12, "y": 1, "w": 12, "h": 8})
    );
    assert_eq!(
        value["panels"][3]["gridPos"],
        json!({"x": 0, "y": 9, "w": 8, "h": 8})
    );
    assert_eq!(
        value["panels"][4]["gridPos"],
        json!({"x": 0, "y": 17, "w": 24, "h": 1})
    );
    assert_eq!(
        value["panels"][5]["gridPos"],
        json!({"x": 0, "y": 18, "w": 24, "h": 8})
    );
}

#[test]
fn collapsed_rows_nest_panels_and_only_consume_the_row_height() {
    let dashboard = Dashboard::new("Collapsed")
        .row(
            Row::new("Hidden")
                .collapsed(true)
                .panel(Timeseries::new("Nested")),
        )
        .row(Row::new("Visible").panel(Stat::new("Value")));

    let value = as_value(&dashboard);
    assert_eq!(value["panels"].as_array().unwrap().len(), 3);
    assert_eq!(value["panels"][0]["collapsed"], true);
    assert_eq!(value["panels"][0]["panels"][0]["title"], "Nested");
    assert_eq!(value["panels"][1]["gridPos"]["y"], 1);
    assert_eq!(value["panels"][2]["gridPos"]["y"], 2);
}

#[test]
fn serializes_units_thresholds_legends_and_overrides() {
    let panel = Timeseries::new("Latency")
        .query(PrometheusQuery::new("latency").legend("p99"))
        .unit(Unit::Milliseconds)
        .thresholds(Thresholds::new().green(0.0).yellow(250.0).red(500.0))
        .legend_options(
            Legend::new()
                .mode(LegendMode::Table)
                .placement(LegendPlacement::Right)
                .calculations([Reducer::Last, Reducer::Max]),
        )
        .override_field(
            FieldOverride::by_name("p99")
                .color(Color::Red)
                .line_width(3),
        );
    let value = as_value(&Dashboard::new("Fields").panel(panel));
    let panel = &value["panels"][0];

    assert_eq!(panel["fieldConfig"]["defaults"]["unit"], "ms");
    assert_eq!(
        panel["fieldConfig"]["defaults"]["thresholds"]["steps"][0]["value"],
        Value::Null
    );
    assert_eq!(
        panel["fieldConfig"]["overrides"][0]["matcher"]["id"],
        "byName"
    );
    assert_eq!(panel["options"]["legend"]["displayMode"], "table");
    assert_eq!(panel["options"]["legend"]["placement"], "right");
}

#[test]
fn serializes_every_initial_panel_type() {
    let dashboard = Dashboard::new("Kinds")
        .panel(Timeseries::new("Timeseries"))
        .panel(Stat::new("Stat"))
        .panel(Gauge::new("Gauge"))
        .panel(Table::new("Table"))
        .panel(Text::new("Text").content("hello"))
        .panel(BarGauge::new("Bar gauge"))
        .panel(Heatmap::new("Heatmap"));
    let value = as_value(&dashboard);
    let kinds = value["panels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|panel| panel["type"].as_str().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(
        kinds,
        [
            "timeseries",
            "stat",
            "gauge",
            "table",
            "text",
            "bargauge",
            "heatmap"
        ]
    );
}

#[test]
fn serializes_query_custom_and_constant_variables() {
    let dashboard = Dashboard::new("Variables")
        .datasource(prometheus("prometheus"))
        .variable(QueryVariable::new(
            "instance",
            PrometheusQuery::new("label_values(up, instance)"),
        ))
        .variable(CustomVariable::new("region", ["east", "west"]).default("east"))
        .variable(ConstantVariable::new("cluster", "primary").hidden(true));
    let value = as_value(&dashboard);
    let variables = value["templating"]["list"].as_array().unwrap();

    assert_eq!(variables[0]["type"], "query");
    assert_eq!(variables[0]["datasource"]["uid"], "prometheus");
    assert_eq!(variables[1]["type"], "custom");
    assert_eq!(variables[2]["type"], "constant");
}

#[test]
fn serializes_datasource_and_complete_variable_state() {
    let dashboard = Dashboard::new("Variable state")
        .variable(
            DataSourceVariable::new("datasource", "prometheus")
                .label("Prometheus")
                .current(VariableSelection::new("Primary", "prometheus-main"))
                .regex("/^prometheus/")
                .refresh(VariableRefresh::OnDashboardLoad),
        )
        .variable(
            QueryVariable::new(
                "namespace",
                PrometheusQuery::new("label_values(kube_namespace_labels, namespace)")
                    .ref_id("NamespaceVariable"),
            )
            .datasource(prometheus("$datasource"))
            .current(VariableSelection::new("All", "$__all"))
            .include_all(true)
            .all_value(".*")
            .multi(true)
            .regex("/^(prod|staging)$/")
            .sort(VariableSort::AlphabeticalCaseInsensitiveAscending)
            .refresh(VariableRefresh::OnTimeRangeChange),
        );
    let value = as_value(&dashboard);
    let variables = value["templating"]["list"].as_array().unwrap();

    assert_eq!(variables[0]["type"], "datasource");
    assert_eq!(variables[0]["query"], "prometheus");
    assert_eq!(variables[0]["current"]["text"], "Primary");
    assert_eq!(variables[1]["query"]["refId"], "NamespaceVariable");
    assert_eq!(variables[1]["current"]["value"], "$__all");
    assert_eq!(variables[1]["allValue"], ".*");
    assert_eq!(variables[1]["refresh"], 2);
    assert_eq!(variables[1]["sort"], 5);
}

#[test]
fn serializes_value_mappings_and_threshold_overrides() {
    let panel = Stat::new("Health")
        .mapping(ValueMapping::new("0", "Unhealthy").color(Color::Red))
        .mapping(ValueMapping::new("1", "Healthy").color(Color::Green))
        .override_field(
            FieldOverride::by_name("availability")
                .thresholds(Thresholds::new().step(Color::Red, None).green(1.0)),
        );
    let value = as_value(&Dashboard::new("Mappings").panel(panel));
    let panel = &value["panels"][0];

    assert_eq!(
        panel["fieldConfig"]["defaults"]["mappings"][0]["options"]["0"]["text"],
        "Unhealthy"
    );
    assert_eq!(
        panel["fieldConfig"]["defaults"]["mappings"][0]["options"]["1"]["color"],
        "green"
    );
    assert_eq!(
        panel["fieldConfig"]["overrides"][0]["properties"][0]["id"],
        "thresholds"
    );
}

#[test]
fn serializes_typed_visualization_options_and_dashboard_metadata() {
    let dashboard = Dashboard::new("Typed options")
        .schema_version(39)
        .version(7)
        .cursor_sync(DashboardCursorSync::Crosshair)
        .panel(
            Stat::new("Status")
                .color_mode(StatColorMode::Background)
                .graph_mode(StatGraphMode::None)
                .orientation(Orientation::Horizontal)
                .wide_layout(true),
        )
        .panel(
            Timeseries::new("Traffic")
                .query(
                    PrometheusQuery::new("rate(requests_total[5m])")
                        .editor_mode(QueryEditorMode::Code),
                )
                .fill_opacity(10.0)
                .line_width(2.0)
                .point_size(3.0)
                .line_interpolation(LineInterpolation::StepAfter)
                .show_points(PointVisibility::Never)
                .span_nulls(true)
                .stacking(Stacking::new(StackingMode::Normal).group("requests"))
                .tooltip(
                    Tooltip::new()
                        .mode(TooltipMode::Multi)
                        .sort(TooltipSort::Descending),
                ),
        )
        .panel(
            BarGauge::new("Pods")
                .display_mode(BarGaugeDisplayMode::Lcd)
                .orientation(Orientation::Vertical)
                .reduce_options(ReduceOptions::new().values(true)),
        );
    let value = as_value(&dashboard);

    assert_eq!(value["schemaVersion"], 39);
    assert_eq!(value["version"], 7);
    assert_eq!(value["graphTooltip"], 1);
    assert_eq!(value["panels"][0]["options"]["colorMode"], "background");
    assert_eq!(
        value["panels"][1]["fieldConfig"]["defaults"]["custom"]["lineInterpolation"],
        "stepAfter"
    );
    assert_eq!(value["panels"][1]["options"]["tooltip"]["mode"], "multi");
    assert_eq!(value["panels"][1]["targets"][0]["editorMode"], "code");
    assert_eq!(value["panels"][2]["options"]["displayMode"], "lcd");
    assert_eq!(
        value["panels"][2]["options"]["reduceOptions"]["values"],
        true
    );
}

#[test]
fn rejects_invalid_dimensions_duplicate_ids_refs_and_thresholds() {
    let dashboard = Dashboard::new("Invalid")
        .panel(
            Timeseries::new("Too wide")
                .width(30)
                .id(7)
                .query(PrometheusQuery::new("foo").ref_id("A"))
                .query(PrometheusQuery::new("bar").ref_id("A"))
                .thresholds(Thresholds::new().red(5.0).green(1.0)),
        )
        .panel(Stat::new("Duplicate").id(7));
    let errors = dashboard.validate().unwrap_err();
    let messages = errors
        .errors()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(messages.contains("width must be between 1 and 24"));
    assert!(messages.contains("duplicate explicit panel ID 7"));
    assert!(messages.contains("duplicate query reference ID \"A\""));
    assert!(messages.contains("threshold values must be finite and strictly increasing"));
}

#[test]
fn writes_pretty_json_with_a_trailing_newline() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("dashboard.json");
    Dashboard::new("Write").write_json(&path).unwrap();
    let json = std::fs::read_to_string(path).unwrap();

    assert!(json.starts_with("{\n"));
    assert!(json.ends_with("}\n"));
}

#[test]
fn reducers_serialize_for_legends_and_reduce_options() {
    let dashboard = Dashboard::new("Reducers").panel(
        Stat::new("Value")
            .query(PrometheusQuery::new("up"))
            .reduce_options(ReduceOptions::new().calculations([Reducer::Mean, Reducer::Total])),
    );

    let value = as_value(&dashboard);
    assert_eq!(
        value["panels"][0]["options"]["reduceOptions"]["calcs"],
        json!(["mean", "sum"])
    );
}

#[test]
fn raw_options_still_override_typed_panel_options() {
    let dashboard = Dashboard::new("Precedence").panel(
        Stat::new("Value")
            .query(PrometheusQuery::new("up"))
            .color_mode(StatColorMode::Background)
            .option("colorMode", json!("none")),
    );

    let value = as_value(&dashboard);
    assert_eq!(value["panels"][0]["options"]["colorMode"], "none");
}

#[test]
fn raw_option_wins_even_when_set_before_the_typed_setter() {
    let dashboard = Dashboard::new("Precedence before").panel(
        Stat::new("Value")
            .query(PrometheusQuery::new("up"))
            .option("colorMode", json!("none"))
            .color_mode(StatColorMode::Background),
    );

    let value = as_value(&dashboard);
    assert_eq!(value["panels"][0]["options"]["colorMode"], "none");
}

#[test]
fn typed_panel_options_survive_a_later_unrelated_option() {
    let dashboard = Dashboard::new("Merge").panel(
        Stat::new("Value")
            .query(PrometheusQuery::new("up"))
            .color_mode(StatColorMode::Background)
            .option("textMode", json!("value")),
    );

    let value = as_value(&dashboard);
    assert_eq!(value["panels"][0]["options"]["colorMode"], "background");
    assert_eq!(value["panels"][0]["options"]["textMode"], "value");
    assert_eq!(value["panels"][0]["options"]["graphMode"], "area");
}

#[test]
fn field_config_no_longer_wipes_a_typed_setter() {
    let dashboard = Dashboard::new("Field config order").panel(
        Timeseries::new("Latency")
            .query(PrometheusQuery::new("up"))
            .fill_opacity(50.0)
            .field_config(FieldConfig::new().unit(Unit::Seconds)),
    );

    let value = as_value(&dashboard);
    assert_eq!(
        value["panels"][0]["fieldConfig"]["defaults"]["custom"]["fillOpacity"],
        50.0
    );
}

#[test]
fn field_config_custom_still_beats_a_typed_setter_regardless_of_order() {
    let dashboard = Dashboard::new("Field config custom wins").panel(
        Timeseries::new("Latency")
            .query(PrometheusQuery::new("up"))
            .field_config(FieldConfig::new().custom("lineWidth", json!(9)))
            .line_width(2.0),
    );

    let value = as_value(&dashboard);
    assert_eq!(
        value["panels"][0]["fieldConfig"]["defaults"]["custom"]["lineWidth"],
        9
    );
}

#[test]
fn gauge_lowers_orientation_and_reduce_options() {
    let dashboard = Dashboard::new("Gauge options").panel(
        Gauge::new("Value")
            .query(PrometheusQuery::new("up"))
            .orientation(Orientation::Vertical)
            .reduce_options(ReduceOptions::new().calculations([Reducer::Max])),
    );

    let value = as_value(&dashboard);
    assert_eq!(value["panels"][0]["options"]["orientation"], "vertical");
    assert_eq!(
        value["panels"][0]["options"]["reduceOptions"]["calcs"],
        json!(["max"])
    );
}

// The serializer emits panel options in tiers: a kind default, then a typed
// override, then the raw `.option()` escape hatch. Kind defaults and typed
// defaults are independent literals (`BTreeMap::extend` replaces whole
// values), so nothing stops them from drifting apart. These tests pin that
// the *default* typed value reproduces exactly the kind default it shadows,
// for every panel kind and key where that duplication exists today.

#[test]
fn stat_reduce_options_default_matches_kind_default() {
    let without = as_value(
        &Dashboard::new("Reduce default")
            .panel(Stat::new("Value").query(PrometheusQuery::new("up"))),
    );
    let with = as_value(
        &Dashboard::new("Reduce default").panel(
            Stat::new("Value")
                .query(PrometheusQuery::new("up"))
                .reduce_options(ReduceOptions::new()),
        ),
    );

    assert_eq!(
        with["panels"][0]["options"]["reduceOptions"],
        without["panels"][0]["options"]["reduceOptions"]
    );
}

#[test]
fn gauge_reduce_options_default_matches_kind_default() {
    let without = as_value(
        &Dashboard::new("Reduce default")
            .panel(Gauge::new("Value").query(PrometheusQuery::new("up"))),
    );
    let with = as_value(
        &Dashboard::new("Reduce default").panel(
            Gauge::new("Value")
                .query(PrometheusQuery::new("up"))
                .reduce_options(ReduceOptions::new()),
        ),
    );

    assert_eq!(
        with["panels"][0]["options"]["reduceOptions"],
        without["panels"][0]["options"]["reduceOptions"]
    );
}

#[test]
fn bar_gauge_reduce_options_default_matches_kind_default() {
    let without = as_value(
        &Dashboard::new("Reduce default")
            .panel(BarGauge::new("Value").query(PrometheusQuery::new("up"))),
    );
    let with = as_value(
        &Dashboard::new("Reduce default").panel(
            BarGauge::new("Value")
                .query(PrometheusQuery::new("up"))
                .reduce_options(ReduceOptions::new()),
        ),
    );

    assert_eq!(
        with["panels"][0]["options"]["reduceOptions"],
        without["panels"][0]["options"]["reduceOptions"]
    );
}

#[test]
fn timeseries_tooltip_default_matches_kind_default() {
    let without = as_value(
        &Dashboard::new("Tooltip default")
            .panel(Timeseries::new("Value").query(PrometheusQuery::new("up"))),
    );
    let with = as_value(
        &Dashboard::new("Tooltip default").panel(
            Timeseries::new("Value")
                .query(PrometheusQuery::new("up"))
                .tooltip(Tooltip::new()),
        ),
    );

    assert_eq!(
        with["panels"][0]["options"]["tooltip"],
        without["panels"][0]["options"]["tooltip"]
    );
}

#[test]
fn transformations_serialize_in_authored_order() {
    let dashboard = Dashboard::new("Transforms").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new(
                "sum by (route) (rate(requests_total[5m]))",
            ))
            .transform(JoinByField::new("route"))
            .transform(
                OrganizeFields::new()
                    .rename("Value #A", "RPS")
                    .hide("Time")
                    .order(["route", "RPS"]),
            )
            .transform(SortBy::desc("RPS")),
    );

    let transformations = &as_value(&dashboard)["panels"][0]["transformations"];
    assert_eq!(
        transformations[0],
        json!({"id": "joinByField", "options": {"byField": "route", "mode": "outer"}})
    );
    assert_eq!(
        transformations[1],
        json!({
            "id": "organize",
            "options": {
                "excludeByName": {"Time": true},
                "indexByName": {"RPS": 1, "route": 0},
                "renameByName": {"Value #A": "RPS"},
            }
        })
    );
    assert_eq!(
        transformations[2],
        json!({"id": "sortBy", "options": {"sort": [{"field": "RPS", "desc": true}]}})
    );
}

#[test]
fn time_series_to_table_options_are_keyed_by_ref_id() {
    let dashboard = Dashboard::new("Trend").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up").ref_id("A"))
            .query(PrometheusQuery::new("rate(x[5m])").ref_id("D"))
            .transform(TimeSeriesToTable::new().query_with("D", Reducer::Last)),
    );

    let transformations = &as_value(&dashboard)["panels"][0]["transformations"];
    assert_eq!(
        transformations[0],
        json!({"id": "timeSeriesTable", "options": {"D": {"stat": "lastNotNull"}}})
    );
}

#[test]
fn join_modes_and_labels_to_fields_serialize() {
    let dashboard = Dashboard::new("Modes").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up"))
            .transform(JoinByField::inner("route"))
            .transform(LabelsToFields::new().keep(["route"]))
            .transform(RawTransformation::new("myPlugin").option("depth", json!(3))),
    );

    let transformations = &as_value(&dashboard)["panels"][0]["transformations"];
    assert_eq!(transformations[0]["options"]["mode"], "inner");
    assert_eq!(
        transformations[1],
        json!({"id": "labelsToFields", "options": {"mode": "columns", "keepLabels": ["route"]}})
    );
    assert_eq!(
        transformations[2],
        json!({"id": "myPlugin", "options": {"depth": 3}})
    );
}
