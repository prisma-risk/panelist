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

use panelist::{ValidationError, VariableReferenceSite, prelude::*};
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
        .query(PrometheusQuery::new("latency").legend_format("p99"))
        .unit(Unit::Milliseconds)
        .thresholds(
            Thresholds::absolute(Color::Green)
                .step(250.0, Color::Yellow)
                .step(500.0, Color::Red),
        )
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
fn matchers_cover_queries_names_and_field_classes() {
    let dashboard = Dashboard::new("Matchers").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up").ref_id("B"))
            .override_field(FieldOverride::by_query("B").unit(Unit::Percent))
            .override_field(FieldOverride::by_names(["p50", "p95"]).decimals(2))
            .override_field(FieldOverride::numeric_fields().decimals(3))
            .override_field(FieldOverride::time_fields().display_name("When")),
    );

    let overrides = &as_value(&dashboard)["panels"][0]["fieldConfig"]["overrides"];
    assert_eq!(
        overrides[0]["matcher"],
        json!({"id": "byFrameRefID", "options": "B"})
    );
    // `byNames` takes an object, not a bare array: Grafana's
    // `nameMatcher.ts` destructures `const { names, mode = include } =
    // options`, so an array yields `names === undefined` and the override
    // matches nothing. Grafana stores and returns either shape unchanged,
    // so only the source says which is right.
    assert_eq!(
        overrides[1]["matcher"],
        json!({"id": "byNames", "options": {"names": ["p50", "p95"]}})
    );
    assert_eq!(overrides[2]["matcher"], json!({"id": "numeric"}));
    assert_eq!(overrides[3]["matcher"], json!({"id": "time"}));
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
                .thresholds(Thresholds::absolute(Color::Red).step(1.0, Color::Green)),
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
                .thresholds(
                    Thresholds::absolute(Color::Green)
                        .step(5.0, Color::Red)
                        .step(1.0, Color::Green),
                ),
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
fn cell_survives_a_later_field_config() {
    let dashboard = Dashboard::new("Cell survives field config").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up"))
            .cell(TableCellType::Gauge)
            .field_config(FieldConfig::new().unit(Unit::Seconds)),
    );

    let value = as_value(&dashboard);
    assert_eq!(
        value["panels"][0]["fieldConfig"]["defaults"]["custom"]["cellOptions"],
        json!({"type": "gauge"})
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
    // `order` is authored in final names (`RPS`), but Grafana's organize
    // transformer orders before it renames, so `indexByName` has to be keyed
    // on the pre-rename name (`Value #A`). `route` is never renamed and
    // passes through; `excludeByName` is correct as authored because the
    // filter step runs first of all.
    assert_eq!(
        transformations[1],
        json!({
            "id": "organize",
            "options": {
                "excludeByName": {"Time": true},
                "indexByName": {"Value #A": 1, "route": 0},
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
fn organize_ordering_of_an_unrenamed_field_passes_through_untranslated() {
    let dashboard = Dashboard::new("Order passthrough").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up"))
            .transform(OrganizeFields::new().order(["route", "status"])),
    );

    let transformations = &as_value(&dashboard)["panels"][0]["transformations"];
    assert_eq!(
        transformations[0]["options"]["indexByName"],
        json!({"route": 0, "status": 1})
    );
}

#[test]
fn organize_ordering_a_name_two_renames_produce_is_rejected() {
    let dashboard = Dashboard::new("Ambiguous order").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up"))
            .transform(
                OrganizeFields::new()
                    .rename("Value #A", "Latency")
                    .rename("Value #B", "Latency")
                    .order(["Latency"]),
            ),
    );

    let errors = dashboard.validate().unwrap_err();
    assert!(matches!(
        errors.errors(),
        [ValidationError::AmbiguousOrganizeOrder { field, sources, .. }]
            if field == "Latency" && sources == &["Value #A".to_owned(), "Value #B".to_owned()]
    ));
    let message = errors.errors()[0].to_string();
    assert!(
        message.contains("\"Value #A\" and \"Value #B\""),
        "{message}"
    );
}

#[test]
fn organize_ordering_two_names_onto_one_field_is_rejected() {
    // `Latency` resolves back to `Value #A`, and `Value #A` — nobody's
    // rename target — resolves to itself. Both ordering entries would land
    // on the same `indexByName` key and one would silently overwrite the
    // other.
    let dashboard = Dashboard::new("Conflicting order").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up"))
            .transform(
                OrganizeFields::new()
                    .rename("Value #A", "Latency")
                    .order(["Latency", "Value #A"]),
            ),
    );

    let errors = dashboard.validate().unwrap_err();
    assert!(matches!(
        errors.errors(),
        [ValidationError::ConflictingOrganizeOrder { first, second, field, .. }]
            if first == "Latency" && second == "Value #A" && field == "Value #A"
    ));
}

#[test]
fn organize_ordering_without_ambiguity_passes_validation() {
    Dashboard::new("Fine")
        .panel(
            Table::new("Routes")
                .query(PrometheusQuery::new("up"))
                .transform(
                    OrganizeFields::new()
                        .rename("Value #A", "RPS")
                        .rename("Value #B", "Errors")
                        .hide("Time")
                        .order(["route", "RPS", "Errors"]),
                ),
        )
        .validate()
        .unwrap();
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

#[test]
fn transformation_filters_target_one_query() {
    let dashboard = Dashboard::new("Filtered").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up").ref_id("A"))
            .query(PrometheusQuery::new("rate(x[5m])").ref_id("D"))
            .transform(TimeSeriesToTable::new().query("D").only_ref_id("D"))
            .transform(SortBy::desc("p95").disabled(true)),
    );

    let transformations = &as_value(&dashboard)["panels"][0]["transformations"];
    assert_eq!(
        transformations[0]["filter"],
        json!({"id": "byRefId", "options": "D"})
    );
    assert_eq!(transformations[1]["disabled"], true);
    assert!(transformations[1].get("filter").is_none());
}

#[test]
fn unknown_transformation_ref_ids_are_rejected() {
    let dashboard = Dashboard::new("Bad ref").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up"))
            .transform(TimeSeriesToTable::new().query("Z")),
    );

    let errors = dashboard.validate().unwrap_err();
    assert!(matches!(
        errors.errors(),
        [ValidationError::UnknownTransformationRefId { ref_id, .. }] if ref_id == "Z"
    ));
}

/// One authoring mistake, one error. A transformation that both filters on
/// a reference ID and names the same one in its own options used to report
/// the same unknown reference twice.
#[test]
fn one_unknown_ref_id_reported_once_even_when_named_twice() {
    let dashboard = Dashboard::new("Doubled").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up"))
            .transform(TimeSeriesToTable::new().query("Z").only_ref_id("Z")),
    );

    let errors = dashboard.validate().unwrap_err();
    assert_eq!(
        errors.errors().len(),
        1,
        "{:?}",
        errors
            .errors()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );
    assert!(matches!(
        errors.errors(),
        [ValidationError::UnknownTransformationRefId { ref_id, .. }] if ref_id == "Z"
    ));
}

/// Two genuinely different unknown references are still two errors.
#[test]
fn two_unknown_ref_ids_are_reported_separately() {
    let dashboard = Dashboard::new("Two bad refs").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up"))
            .transform(TimeSeriesToTable::new().query("Y").only_ref_id("Z")),
    );

    let errors = dashboard.validate().unwrap_err();
    assert_eq!(errors.errors().len(), 2);
}

#[test]
fn auto_assigned_ref_ids_satisfy_transformation_validation() {
    let dashboard = Dashboard::new("Auto ref").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up"))
            .query(PrometheusQuery::new("rate(x[5m])"))
            .transform(TimeSeriesToTable::new().query("B")),
    );

    dashboard.validate().unwrap();
}

#[test]
fn unknown_override_ref_ids_are_rejected() {
    let dashboard = Dashboard::new("Bad override ref").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up").ref_id("A"))
            .override_field(FieldOverride::by_query("ZZZ").unit(Unit::Percent)),
    );

    let errors = dashboard.validate().unwrap_err();
    assert!(matches!(
        errors.errors(),
        [ValidationError::UnknownOverrideRefId { ref_id, .. }] if ref_id == "ZZZ"
    ));
}

#[test]
fn known_override_ref_ids_pass_validation() {
    let dashboard = Dashboard::new("Good override ref").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up").ref_id("A"))
            .override_field(FieldOverride::by_query("A").unit(Unit::Percent)),
    );

    dashboard.validate().unwrap();
}

#[test]
fn empty_transformation_fields_are_rejected() {
    let dashboard = Dashboard::new("Empty").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up"))
            .transform(JoinByField::new("  "))
            .transform(RawTransformation::new("")),
    );

    let errors = dashboard.validate().unwrap_err();
    assert_eq!(errors.errors().len(), 2);
}

#[test]
fn prometheus_format_serializes_and_is_omitted_by_default() {
    let dashboard = Dashboard::new("Formats").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up").format(PrometheusFormat::Table))
            .query(PrometheusQuery::new("rate(x[5m])")),
    );

    let targets = &as_value(&dashboard)["panels"][0]["targets"];
    assert_eq!(targets[0]["format"], "table");
    assert!(targets[1].get("format").is_none());
}

#[test]
fn heatmap_format_serializes() {
    let dashboard = Dashboard::new("Heatmap").panel(Heatmap::new("Latency").query(
        PrometheusQuery::new("sum by (le) (rate(b[5m]))").format(PrometheusFormat::Heatmap),
    ));

    assert_eq!(
        as_value(&dashboard)["panels"][0]["targets"][0]["format"],
        "heatmap"
    );
}

#[test]
fn cell_types_serialize_through_field_overrides() {
    let dashboard = Dashboard::new("Cells").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up"))
            .override_field(
                FieldOverride::by_name("Error rate").cell_type(TableCellType::ColoredBackground),
            )
            .override_field(FieldOverride::by_name("Trend").cell_type(TableCellType::Sparkline)),
    );

    let overrides = &as_value(&dashboard)["panels"][0]["fieldConfig"]["overrides"];
    assert_eq!(
        overrides[0]["properties"][0],
        json!({"id": "custom.cellOptions", "value": {"type": "color-background"}})
    );
    assert_eq!(
        overrides[1]["properties"][0]["value"],
        json!({"type": "sparkline"})
    );
}

#[test]
fn cell_options_serialize_per_variant() {
    let dashboard = Dashboard::new("Cell options").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up"))
            .override_field(
                FieldOverride::by_name("Error rate").cell(
                    ColoredBackgroundCell::new()
                        .mode(CellBackgroundMode::Gradient)
                        .apply_to_row(true),
                ),
            )
            .override_field(
                FieldOverride::by_name("Load").cell(
                    GaugeCell::new()
                        .mode(BarGaugeDisplayMode::Lcd)
                        .value_display(CellValueDisplay::Color),
                ),
            )
            .override_field(
                FieldOverride::by_name("Trend").cell(
                    SparklineCell::new()
                        .hide_value(true)
                        .line_width(2.0)
                        .fill_opacity(20.0),
                ),
            ),
    );

    let overrides = &as_value(&dashboard)["panels"][0]["fieldConfig"]["overrides"];
    assert_eq!(
        overrides[0]["properties"][0]["value"],
        json!({"type": "color-background", "mode": "gradient", "applyToRow": true})
    );
    assert_eq!(
        overrides[1]["properties"][0]["value"],
        json!({"type": "gauge", "mode": "lcd", "valueDisplayMode": "color"})
    );
    assert_eq!(
        overrides[2]["properties"][0]["value"],
        json!({"type": "sparkline", "hideValue": true, "lineWidth": 2.0, "fillOpacity": 20.0})
    );
}

#[test]
fn table_sorting_serializes_as_a_panel_option() {
    let dashboard = Dashboard::new("Sorted").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up"))
            .sort_by("p95", SortDirection::Descending)
            .sort_by("route", SortDirection::Ascending),
    );

    assert_eq!(
        as_value(&dashboard)["panels"][0]["options"]["sortBy"],
        json!([
            {"displayName": "p95", "desc": true},
            {"displayName": "route", "desc": false}
        ])
    );
}

#[test]
fn heatmap_options_merge_over_grafana_defaults() {
    let dashboard = Dashboard::new("Heatmap").panel(
        Heatmap::new("Latency")
            .query(PrometheusQuery::new("sum by (le) (rate(b[5m]))"))
            .color_scheme("Blues")
            .color_steps(32)
            .cell_gap(2)
            .show_legend(false)
            .y_axis_unit(Unit::Seconds)
            .y_axis_placement(AxisPlacement::Right)
            .calculate(false),
    );

    let options = &as_value(&dashboard)["panels"][0]["options"];
    assert_eq!(
        options["color"],
        json!({"mode": "scheme", "scheme": "Blues", "steps": 32})
    );
    assert_eq!(options["cellGap"], 2);
    assert_eq!(options["legend"], json!({"show": false}));
    assert_eq!(
        options["yAxis"],
        json!({"axisPlacement": "right", "unit": "s"})
    );
    assert_eq!(options["calculate"], false);
}

fn unknown_names(dashboard: &Dashboard) -> Vec<String> {
    let Err(errors) = dashboard.validate() else {
        return Vec::new();
    };
    errors
        .errors()
        .iter()
        .filter_map(|error| match error {
            ValidationError::UnknownVariableReference { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

fn environment() -> CustomVariable {
    CustomVariable::new("env", ["prod", "staging"])
}

#[test]
fn every_reference_syntax_resolves_to_a_declared_variable() {
    let dashboard = Dashboard::new("Syntaxes").variable(environment()).panel(
        Timeseries::new("Up")
            .query(PrometheusQuery::new(r#"up{env="$env"}"#))
            .query(PrometheusQuery::new(r#"up{env="${env}"}"#))
            .query(PrometheusQuery::new(r#"up{env=~"${env:regex}"}"#))
            .query(PrometheusQuery::new(r#"up{env="${env.text}"}"#))
            .query(PrometheusQuery::new(r#"up{env="[[env]]"}"#))
            .query(PrometheusQuery::new(r#"up{env=~"[[env:pipe]]"}"#)),
    );

    dashboard.validate().unwrap();
}

#[test]
fn every_reference_syntax_reports_an_unknown_variable() {
    let dashboard = Dashboard::new("Syntaxes").variable(environment()).panel(
        Timeseries::new("Up")
            .query(PrometheusQuery::new(r#"up{a="$plain"}"#))
            .query(PrometheusQuery::new(r#"up{a="${braced}"}"#))
            .query(PrometheusQuery::new(r#"up{a=~"${formatted:regex}"}"#))
            .query(PrometheusQuery::new(r#"up{a="${pathed.text}"}"#))
            .query(PrometheusQuery::new(r#"up{a="[[bracketed]]"}"#))
            .query(PrometheusQuery::new(r#"up{a=~"[[bracketed_format:csv]]"}"#)),
    );

    assert_eq!(
        unknown_names(&dashboard),
        [
            "plain",
            "braced",
            "formatted",
            "pathed",
            "bracketed",
            "bracketed_format"
        ]
    );
}

/// `$env_name` is one variable called `env_name` to Grafana, not `$env`
/// followed by `_name`. Panelist reads it the same way.
#[test]
fn a_plain_reference_runs_to_the_end_of_the_word() {
    let dashboard = Dashboard::new("Greedy").variable(environment()).panel(
        Stat::new("Up")
            .query(PrometheusQuery::new(r#"up{job="$env_name"}"#))
            .query(PrometheusQuery::new(r#"up{job="${env}_name"}"#)),
    );

    assert_eq!(unknown_names(&dashboard), ["env_name"]);
}

#[test]
fn unknown_references_name_the_site_and_field() {
    let dashboard = Dashboard::new("Sites")
        .datasource(prometheus("$dashboard_source"))
        .link(DashboardLink::new(
            "Runbook",
            "https://runbooks/$dashboard_link",
        ))
        .variable(QueryVariable::new(
            "pod",
            PrometheusQuery::new("label_values(up{job=\"$job\"}, pod)"),
        ))
        .panel(
            Timeseries::new("Requests")
                .datasource(prometheus("$panel_source"))
                .query(
                    PrometheusQuery::new("rate(requests_total[$__rate_interval])")
                        .legend_format("{{route}} $legend")
                        .datasource(prometheus("$query_source")),
                )
                .link(DashboardLink::new("Logs", "/explore?pod=$panel_link")),
        );

    let errors = dashboard.validate().unwrap_err();
    let unknown = |site: VariableReferenceSite, field: &'static str, name: &str| {
        ValidationError::UnknownVariableReference {
            site,
            field,
            name: name.to_owned(),
        }
    };
    let panel = || VariableReferenceSite::Panel("Requests".to_owned());
    assert_eq!(
        errors.errors(),
        [
            unknown(
                VariableReferenceSite::Dashboard,
                "datasource",
                "dashboard_source"
            ),
            unknown(
                VariableReferenceSite::Dashboard,
                "link URL",
                "dashboard_link"
            ),
            unknown(
                VariableReferenceSite::Variable("pod".to_owned()),
                "query expression",
                "job"
            ),
            unknown(panel(), "datasource", "panel_source"),
            unknown(panel(), "legend format", "legend"),
            unknown(panel(), "query datasource", "query_source"),
            unknown(panel(), "link URL", "panel_link"),
        ]
    );
    assert_eq!(
        errors.errors()[4].to_string(),
        "panel \"Requests\": legend format references unknown variable \"legend\""
    );
}

#[test]
fn panels_in_rows_are_checked() {
    let dashboard = Dashboard::new("Rows").row(
        Row::new("HTTP").panel(Stat::new("Up").query(PrometheusQuery::new("up{a=\"$missing\"}"))),
    );

    assert_eq!(unknown_names(&dashboard), ["missing"]);
}

/// One mistake, one error: a name repeated in one field is reported once.
#[test]
fn an_unknown_name_is_reported_once_per_field() {
    let dashboard = Dashboard::new("Repeated").panel(
        Stat::new("Ratio")
            .query(PrometheusQuery::new("a{x=\"$nope\"} / b{x=\"$nope\"}"))
            .query(PrometheusQuery::new("c{x=\"${nope}\"}").legend_format("$nope")),
    );

    let errors = dashboard.validate().unwrap_err();
    let fields: Vec<_> = errors
        .errors()
        .iter()
        .map(|error| match error {
            ValidationError::UnknownVariableReference { field, .. } => *field,
            other => panic!("unexpected error: {other}"),
        })
        .collect();
    assert_eq!(fields, ["query expression", "legend format"]);
}

#[test]
fn grafana_builtins_need_no_declaration() {
    let dashboard = Dashboard::new("Builtins")
        .link(DashboardLink::new(
            "Same window",
            "/d/other?${__url_time_range}&${__all_variables}&org=${__org.name}",
        ))
        .panel(
            Timeseries::new("Everything")
                .query(PrometheusQuery::new(
                    "rate(a[$__rate_interval]) + rate(b[$__interval]) + rate(c[$__range]) + rate(d[$__auto])",
                ))
                .query(PrometheusQuery::new(
                    "e * $__interval_ms * $__range_s * $__range_ms * $__rate_interval_ms * [[__interval]]",
                ))
                .query(
                    PrometheusQuery::new("f{user=\"${__user.login}\", tz=\"$__timezone\"}")
                        .legend_format("${__from:date:iso} to ${__to:date:seconds} on $__dashboard"),
                )
                .link(DashboardLink::new("Panel", "/d/x?from=$__from&to=$__to&v=$__all")),
        );

    dashboard.validate().unwrap();
}

/// A typed Prometheus or Loki query has a fixed macro set, so a misspelt
/// built-in is still an unknown variable there.
#[test]
fn a_misspelt_builtin_in_a_typed_query_is_reported() {
    let dashboard = Dashboard::new("Typo").panel(
        Timeseries::new("Requests")
            .query(PrometheusQuery::new(
                "rate(requests_total[$__rate_inteval])",
            ))
            .query(LokiQuery::new("count_over_time({app=\"x\"}[$__autoo])")),
    );

    assert_eq!(unknown_names(&dashboard), ["__rate_inteval", "__autoo"]);
}

/// Panelist cannot know a raw query's plugin, and plugins define their own
/// macros in Grafana's reserved `__` namespace. Ordinary names are still
/// checked.
#[test]
fn raw_queries_tolerate_plugin_macros_but_not_unknown_variables() {
    let dashboard = Dashboard::new("SQL").variable(environment()).panel(
        Table::new("Rows")
            .query(RawQuery::new().expression(
                "SELECT $__timeGroup(ts, $__interval), count(*) FROM t WHERE $__timeFilter(ts) AND $__unixEpochFilter(epoch) AND env = '$env' GROUP BY 1",
            ))
            .query(RawQuery::new().expression("SELECT mean(v) FROM m WHERE $timeFilter GROUP BY time($interval)"))
            .query(RawQuery::new().expression("SELECT * FROM t WHERE region = '$region'")),
    );

    assert_eq!(unknown_names(&dashboard), ["region"]);
}

#[test]
fn chained_variables_may_reference_earlier_variables() {
    let dashboard = Dashboard::new("Chain")
        .variable(DataSourceVariable::new("datasource", "prometheus"))
        .variable(
            QueryVariable::new(
                "namespace",
                PrometheusQuery::new("label_values(up, namespace)"),
            )
            .datasource(prometheus("${datasource}")),
        )
        .variable(
            QueryVariable::new(
                "pod",
                PrometheusQuery::new("label_values(up{namespace=~\"$namespace\"}, pod)"),
            )
            .datasource(prometheus("$datasource")),
        );

    dashboard.validate().unwrap();
}

#[test]
fn chained_variables_may_not_reference_later_variables_or_themselves() {
    let dashboard = Dashboard::new("Chain")
        .variable(
            QueryVariable::new(
                "pod",
                PrometheusQuery::new(
                    "label_values(up{namespace=~\"$namespace\", pod=~\"$pod\"}, pod)",
                ),
            )
            .datasource(prometheus("$datasource")),
        )
        .variable(QueryVariable::new(
            "namespace",
            PrometheusQuery::new("label_values(up, namespace)"),
        ))
        .variable(DataSourceVariable::new("datasource", "prometheus"));

    let errors = dashboard.validate().unwrap_err();
    let early =
        |field: &'static str, name: &str| ValidationError::VariableReferencedBeforeDeclaration {
            variable: "pod".to_owned(),
            field,
            name: name.to_owned(),
        };
    assert_eq!(
        errors.errors(),
        [
            early("query expression", "namespace"),
            early("query expression", "pod"),
            early("datasource", "datasource"),
        ]
    );
    assert_eq!(
        errors.errors()[0].to_string(),
        "variable \"pod\": query expression references \"namespace\", which is not declared before it; Grafana resolves variables in declaration order"
    );
}

/// A query variable without its own datasource runs against the dashboard
/// default, so that default must also resolve before the variable does.
#[test]
fn an_inherited_dashboard_datasource_must_resolve_before_the_variable() {
    let dashboard = Dashboard::new("Inherited")
        .datasource(prometheus("$datasource"))
        .variable(QueryVariable::new(
            "namespace",
            PrometheusQuery::new("label_values(up, namespace)"),
        ))
        .variable(DataSourceVariable::new("datasource", "prometheus"))
        .panel(Stat::new("Up").query(PrometheusQuery::new("up")));

    let errors = dashboard.validate().unwrap_err();
    assert_eq!(
        errors.errors(),
        [ValidationError::VariableReferencedBeforeDeclaration {
            variable: "namespace".to_owned(),
            field: "inherited datasource",
            name: "datasource".to_owned(),
        }]
    );
}

/// An unknown name in the dashboard datasource is the dashboard's mistake.
/// Variables inheriting it do not repeat the report.
#[test]
fn an_unknown_inherited_datasource_is_reported_once_at_the_dashboard() {
    let dashboard = Dashboard::new("Inherited")
        .datasource(prometheus("$nowhere"))
        .variable(QueryVariable::new(
            "a",
            PrometheusQuery::new("label_values(up, a)"),
        ))
        .variable(QueryVariable::new(
            "b",
            PrometheusQuery::new("label_values(up, b)"),
        ));

    let errors = dashboard.validate().unwrap_err();
    assert_eq!(
        errors.errors(),
        [ValidationError::UnknownVariableReference {
            site: VariableReferenceSite::Dashboard,
            field: "datasource",
            name: "nowhere".to_owned(),
        }]
    );
}

/// Panels resolve after every variable, so declaration order is irrelevant
/// to them.
#[test]
fn panels_may_reference_variables_declared_in_any_order() {
    let dashboard = Dashboard::new("Order")
        .panel(Stat::new("Up").query(PrometheusQuery::new("up{env=\"$env\"}")))
        .variable(environment());

    dashboard.validate().unwrap();
}

#[test]
fn dollar_signs_that_are_not_references_are_ignored() {
    let dashboard = Dashboard::new("False positives")
        .variable(
            QueryVariable::new("env", PrometheusQuery::new("label_values(up, env)"))
                .regex("/^(prod|staging)$/"),
        )
        .variable(DataSourceVariable::new("source", "prometheus").regex("/^prometheus-$/"))
        .panel(
            Table::new("Hosts")
                .query(PrometheusQuery::new(
                    r#"label_replace(up{job=~"^(api|web)$"}, "host", "$1", "instance", "(.*):.*")"#,
                ))
                .query(PrometheusQuery::new(
                    r#"label_replace(up, "pair", "${1}-${2}", "instance", "(.*):(.*)")"#,
                ))
                .query(PrometheusQuery::new("vector(1)").legend_format("costs $ 5"))
                .transform(OrganizeFields::new().rename("Value", "$1 total")),
        );

    dashboard.validate().unwrap();
}

fn validation_errors(dashboard: &Dashboard) -> Vec<ValidationError> {
    match dashboard.validate() {
        Ok(()) => Vec::new(),
        Err(errors) => errors.errors().to_vec(),
    }
}

#[test]
fn instant_is_shorthand_for_the_instant_query_type() {
    assert_eq!(
        PrometheusQuery::new("up").instant(),
        PrometheusQuery::new("up").query_type(QueryType::Instant)
    );
    assert_eq!(
        LokiQuery::new("{job=\"api\"}").instant(),
        LokiQuery::new("{job=\"api\"}").query_type(QueryType::Instant)
    );
}

#[test]
fn loki_rejects_the_combined_query_type_and_prometheus_accepts_it() {
    let dashboard = Dashboard::new("Query types").panel(
        Timeseries::new("Mixed evaluations")
            .query(PrometheusQuery::new("up").query_type(QueryType::Both))
            .query(LokiQuery::new("{job=\"api\"}").query_type(QueryType::Both)),
    );
    assert_eq!(
        validation_errors(&dashboard),
        [ValidationError::UnsupportedQueryType {
            panel: "Mixed evaluations".to_owned(),
            ref_id: "B".to_owned(),
        }]
    );
}

#[test]
fn a_mixed_panel_requires_a_datasource_on_every_query() {
    let panel = Timeseries::new("Requests and logs")
        .query(PrometheusQuery::new("up").datasource(prometheus("metrics")))
        .query(LokiQuery::new("{job=\"api\"}"));
    let expected = [ValidationError::MissingMixedQueryDataSource {
        panel: "Requests and logs".to_owned(),
        ref_id: "B".to_owned(),
    }];

    let on_panel = Dashboard::new("Mixed").panel(panel.clone().datasource(DataSource::mixed()));
    assert_eq!(validation_errors(&on_panel), expected);

    let inherited = Dashboard::new("Mixed")
        .datasource(DataSource::mixed())
        .panel(panel);
    assert_eq!(validation_errors(&inherited), expected);
}

#[test]
fn mixed_is_the_builtin_datasource_reference() {
    let mixed = DataSource::mixed();
    assert_eq!(mixed.kind(), "datasource");
    assert_eq!(mixed.uid(), "-- Mixed --");
    assert!(mixed.is_mixed());
    assert!(!prometheus("-- Mixed --").is_mixed());
}

#[test]
fn a_raw_query_needs_an_expression_or_an_extra_key() {
    let missing = |query: RawQuery| {
        validation_errors(&Dashboard::new("Raw").panel(Timeseries::new("Raw").query(query)))
            .iter()
            .any(|error| matches!(error, ValidationError::MissingQueryExpression { .. }))
    };

    assert!(missing(RawQuery::new()));
    assert!(missing(RawQuery::new().expression("  ")));
    assert!(!missing(RawQuery::new().expression("up")));
    assert!(!missing(
        RawQuery::new().extra("metricName", json!("CPUUtilization"))
    ));
}

#[test]
fn a_variable_query_still_needs_an_expression() {
    let dashboard = Dashboard::new("Variables").variable(QueryVariable::new(
        "region",
        RawQuery::new().extra("namespace", json!("AWS/EC2")),
    ));
    assert!(matches!(
        validation_errors(&dashboard).as_slice(),
        [ValidationError::MissingQueryExpression { panel }] if panel == "variable region"
    ));
}

#[test]
fn thresholds_are_validated_wherever_they_appear() {
    let decreasing = Thresholds::absolute(Color::Green)
        .step(5.0, Color::Red)
        .step(1.0, Color::Yellow);
    let non_finite = Thresholds::absolute(Color::Green).step(f64::NAN, Color::Red);

    for panel in [
        Stat::new("Panel")
            .query(PrometheusQuery::new("up"))
            .thresholds(decreasing.clone()),
        Stat::new("Override")
            .query(PrometheusQuery::new("up"))
            .override_field(FieldOverride::by_name("up").thresholds(decreasing.clone())),
        Stat::new("Query").query(PrometheusQuery::new("up").thresholds(non_finite.clone())),
    ] {
        let errors = validation_errors(&Dashboard::new("Thresholds").panel(panel));
        assert!(
            matches!(
                errors.as_slice(),
                [ValidationError::InvalidThresholds { .. }]
            ),
            "{errors:?}"
        );
    }

    let valid = Dashboard::new("Thresholds").panel(
        Stat::new("Base only")
            .query(PrometheusQuery::new("up"))
            .thresholds(Thresholds::percentage(Color::Red)),
    );
    assert!(validation_errors(&valid).is_empty());
}

/// Per-query overrides follow reference ID assignment: a query without an explicit ID gets the first letter no explicit ID already holds.
#[test]
fn query_field_properties_target_the_assigned_ref_id() {
    let dashboard = Dashboard::new("Per query").panel(
        Timeseries::new("Traffic")
            .query(PrometheusQuery::new("first").unit(Unit::Seconds))
            .query(PrometheusQuery::new("second").ref_id("A")),
    );
    let value = as_value(&dashboard);
    let panel = &value["panels"][0];
    assert_eq!(panel["targets"][0]["refId"], "B");
    assert_eq!(
        panel["fieldConfig"]["overrides"][0]["matcher"],
        json!({"id": "byFrameRefID", "options": "B"})
    );
}

#[test]
fn duplicate_explicit_ref_ids_are_caught_with_query_field_properties() {
    let dashboard = Dashboard::new("Duplicates").panel(
        Timeseries::new("Traffic")
            .query(
                PrometheusQuery::new("first")
                    .ref_id("A")
                    .color(Color::Green),
            )
            .query(PrometheusQuery::new("second").ref_id("A").color(Color::Red)),
    );
    assert_eq!(
        validation_errors(&dashboard),
        [ValidationError::DuplicateQueryRefId {
            panel: "Traffic".to_owned(),
            ref_id: "A".to_owned(),
        }]
    );
}

/// Variables render no fields, so a variable query's field properties have nowhere to go and are left out rather than misplaced.
#[test]
fn field_properties_on_a_variable_query_are_not_emitted() {
    let dashboard = Dashboard::new("Variables").variable(QueryVariable::new(
        "job",
        PrometheusQuery::new("label_values(up, job)").unit(Unit::Seconds),
    ));
    let variable = &as_value(&dashboard)["templating"]["list"][0];
    assert_eq!(
        variable["query"],
        json!({"query": "label_values(up, job)", "refId": "StandardVariableQuery"})
    );
    assert!(variable.get("fieldConfig").is_none());
}

#[test]
fn rename_by_regex_reports_an_unknown_ref_id_filter() {
    let dashboard = Dashboard::new("Rename").panel(
        Table::new("Services")
            .query(PrometheusQuery::new("up"))
            .transform(RenameByRegex::new("(.*)-api", "$1").only_ref_id("Z")),
    );
    assert!(matches!(
        validation_errors(&dashboard).as_slice(),
        [ValidationError::UnknownTransformationRefId { ref_id, .. }] if ref_id == "Z"
    ));
}
