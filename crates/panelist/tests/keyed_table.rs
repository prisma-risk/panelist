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

//! The keyed table builder.
//!
//! A keyed table must lower into exactly the panel an author could write by hand: plain queries, transformations, and overrides. The model tests below hold it to that by comparing whole `Panel` values, the wire tests pin the Grafana field names it relies on, and the golden pins the full output.

use std::{fs, path::PathBuf};

use panelist::{ValidationError, prelude::*};
use serde_json::{Value, json};

const RPS: &str = "sum by (route) (rate(http_requests_total[$__rate_interval]))";
const ERRORS: &str =
    "sum by (route) (rate(http_requests_total{status=~\"5..\"}[$__rate_interval]))";
const P95: &str = "histogram_quantile(0.95, sum by (route, le) (rate(http_request_duration_seconds_bucket[$__rate_interval])))";

fn instant_table(expression: &str, ref_id: &str) -> PrometheusQuery {
    PrometheusQuery::new(expression)
        .ref_id(ref_id)
        .format(PrometheusFormat::Table)
        .instant(true)
}

fn dashboard_with(panel: impl Into<Panel>) -> Dashboard {
    Dashboard::new("Routes")
        .datasource(prometheus("prometheus-main"))
        .panel(panel)
}

fn panel_json(panel: impl Into<Panel>) -> Value {
    match serde_json::to_value(dashboard_with(panel)) {
        Ok(value) => value["panels"][0].clone(),
        Err(error) => panic!("test dashboard should serialize: {error}"),
    }
}

fn organize_of(panel: &Value) -> Value {
    panel["transformations"]
        .as_array()
        .and_then(|transformations| {
            transformations
                .iter()
                .find(|transformation| transformation["id"] == "organize")
        })
        .map(|organize| organize["options"].clone())
        .unwrap_or_else(|| panic!("panel should have an organize transformation: {panel}"))
}

fn validation_errors(panel: impl Into<Panel>) -> Vec<ValidationError> {
    match dashboard_with(panel).validate() {
        Ok(()) => Vec::new(),
        Err(errors) => errors.errors().to_vec(),
    }
}

// ---------------------------------------------------------------------------
// Model: the keyed table is the hand-written table, nothing more.
// ---------------------------------------------------------------------------

#[test]
fn keyed_table_lowers_to_the_hand_written_table() {
    let keyed = Table::new("Routes")
        .width(24)
        .sort_by("p95", SortDirection::Descending)
        .keyed("route")
        .key_name("Route")
        .column(Column::new("RPS", promql!(RPS)).unit(Unit::RequestsPerSecond))
        .column(
            Column::new("p95", promql!(P95))
                .unit(Unit::Seconds)
                .cell(TableCellType::ColoredBackground)
                .thresholds(Thresholds::new().green(0.0).red(1.0)),
        )
        .column(
            Column::trend("Trend", promql!(P95), Reducer::Last)
                .cell(SparklineCell::new().hide_value(true)),
        );

    let by_hand = Table::new("Routes")
        .width(24)
        .sort_by("p95", SortDirection::Descending)
        .query(instant_table(RPS, "A"))
        .query(instant_table(P95, "B"))
        .query(
            PrometheusQuery::new(P95)
                .ref_id("C")
                .format(PrometheusFormat::TimeSeries),
        )
        .transform(TimeSeriesToTable::new().query_with("C", Reducer::Last))
        .transform(JoinByField::new("route"))
        .transform(
            OrganizeFields::new()
                .rename("Value #A", "RPS")
                .rename("Value #B", "p95")
                .rename("Trend #C", "Trend")
                .rename("route", "Route")
                .hide("Time")
                .order(["Route", "RPS", "p95", "Trend"]),
        )
        .override_field(FieldOverride::by_name("RPS").unit(Unit::RequestsPerSecond))
        .override_field(
            FieldOverride::by_name("p95")
                .unit(Unit::Seconds)
                .cell(TableCellType::ColoredBackground)
                .thresholds(Thresholds::new().green(0.0).red(1.0)),
        )
        .override_field(FieldOverride::by_name("Trend").cell(SparklineCell::new().hide_value(true)))
        .build();

    assert_eq!(Panel::from(keyed), by_hand);
}

/// Anything already on the table runs after the lowering: extra queries after the column queries, extra transformations on the joined and renamed table, and extra overrides after the column overrides so that they win.
#[test]
fn table_settings_compose_after_the_lowering() {
    let keyed = Table::new("Routes")
        .cell(TableCellType::ColoredText)
        .query(PrometheusQuery::new("count(up)").hidden(true))
        .transform(SortBy::desc("RPS"))
        .override_field(FieldOverride::by_name("RPS").decimals(1))
        .keyed("route")
        .column(Column::new("RPS", promql!(RPS)).unit(Unit::RequestsPerSecond))
        .column(Column::new("5xx", promql!(ERRORS)));

    let by_hand = Table::new("Routes")
        .cell(TableCellType::ColoredText)
        .query(instant_table(RPS, "A"))
        .query(instant_table(ERRORS, "B"))
        .query(PrometheusQuery::new("count(up)").hidden(true))
        .transform(JoinByField::new("route"))
        .transform(
            OrganizeFields::new()
                .rename("Value #A", "RPS")
                .rename("Value #B", "5xx")
                .hide("Time")
                .order(["route", "RPS", "5xx"]),
        )
        .transform(SortBy::desc("RPS"))
        .override_field(FieldOverride::by_name("RPS").unit(Unit::RequestsPerSecond))
        .override_field(FieldOverride::by_name("RPS").decimals(1))
        .build();

    assert_eq!(Panel::from(keyed), by_hand);
}

#[test]
fn a_column_without_properties_gets_no_override() {
    let panel = panel_json(
        Table::new("Routes")
            .keyed("route")
            .column(Column::new("RPS", promql!(RPS)))
            .column(Column::new("5xx", promql!(ERRORS))),
    );
    assert_eq!(panel["fieldConfig"]["overrides"], json!([]));
}

#[test]
fn explicit_ref_ids_on_column_queries_are_kept_and_skipped() {
    let panel = panel_json(
        Table::new("Routes")
            .keyed("route")
            .column(Column::new("RPS", promql!(RPS)))
            .column(Column::new("5xx", promql!(ERRORS).ref_id("A")))
            .column(Column::trend("Trend", promql!(P95), Reducer::Mean)),
    );

    let ref_ids: Vec<&Value> = panel["targets"]
        .as_array()
        .map(|targets| targets.iter().map(|target| &target["refId"]).collect())
        .unwrap_or_default();
    assert_eq!(ref_ids, [&json!("B"), &json!("A"), &json!("C")]);
    assert_eq!(
        organize_of(&panel)["renameByName"],
        json!({"Trend #C": "Trend", "Value #A": "5xx", "Value #B": "RPS"})
    );
    assert_eq!(
        panel["transformations"][0],
        json!({"id": "timeSeriesTable", "options": {"C": {"stat": "mean"}}})
    );
}

/// Extra queries added to the table without a reference ID are assigned one at serialization. They must not collide with the IDs the columns took.
#[test]
fn extra_queries_get_reference_ids_after_the_columns() {
    let panel = panel_json(
        Table::new("Routes")
            .query(PrometheusQuery::new("count(up)").hidden(true))
            .keyed("route")
            .column(Column::new("RPS", promql!(RPS)))
            .column(Column::new("5xx", promql!(ERRORS))),
    );
    let ref_ids: Vec<&Value> = panel["targets"]
        .as_array()
        .map(|targets| targets.iter().map(|target| &target["refId"]).collect())
        .unwrap_or_default();
    assert_eq!(ref_ids, [&json!("A"), &json!("B"), &json!("C")]);
}

// ---------------------------------------------------------------------------
// Wire: the Grafana field names and query shapes the lowering relies on.
// ---------------------------------------------------------------------------

/// `packages/grafana-prometheus/src/result_transformer.ts`, `transformDFToTable`: a table result's value field is `Value #<refId>` only when the response holds more than one table result, and plain `Value` otherwise. Verified on Grafana 13.0.2 and 11.5.2.
#[test]
fn one_table_query_renames_plain_value() {
    let panel = panel_json(
        Table::new("Routes")
            .keyed("route")
            .column(Column::new("RPS", promql!(RPS)))
            .column(Column::trend("Trend", promql!(RPS), Reducer::Last)),
    );
    assert_eq!(
        organize_of(&panel),
        json!({
            "excludeByName": {"Time": true},
            "indexByName": {"Trend #B": 2, "Value": 1, "route": 0},
            "renameByName": {"Trend #B": "Trend", "Value": "RPS"}
        })
    );
}

/// The datasource counts every table result in the response, not just the keyed table's own, so an extra visible table query switches the naming.
#[test]
fn an_extra_table_query_switches_to_numbered_value_names() {
    let extra = PrometheusQuery::new("count by (route) (up)").format(PrometheusFormat::Table);
    let panel = panel_json(
        Table::new("Routes")
            .query(extra.clone())
            .keyed("route")
            .column(Column::new("RPS", promql!(RPS))),
    );
    assert_eq!(
        organize_of(&panel)["renameByName"],
        json!({"Value #A": "RPS"})
    );

    // A hidden query is never sent, so it returns no result to count.
    let panel = panel_json(
        Table::new("Routes")
            .query(extra.hidden(true))
            .keyed("route")
            .column(Column::new("RPS", promql!(RPS))),
    );
    assert_eq!(organize_of(&panel)["renameByName"], json!({"Value": "RPS"}));
}

#[test]
fn keyed_table_emits_exact_targets_and_transformations() {
    let panel = panel_json(
        Table::new("Routes")
            .keyed("route")
            .key_name("Route")
            .column(Column::new("RPS", promql!(RPS)))
            .column(Column::new("5xx", promql!(ERRORS)))
            .column(Column::trend("Trend", promql!(P95), Reducer::Last)),
    );
    let datasource = json!({"type": "prometheus", "uid": "prometheus-main"});

    assert_eq!(
        panel["targets"],
        json!([
            {"refId": "A", "datasource": datasource, "expr": RPS, "format": "table", "instant": true, "range": false, "hide": false},
            {"refId": "B", "datasource": datasource, "expr": ERRORS, "format": "table", "instant": true, "range": false, "hide": false},
            {"refId": "C", "datasource": datasource, "expr": P95, "format": "time_series", "hide": false}
        ])
    );
    assert_eq!(
        panel["transformations"],
        json!([
            {"id": "timeSeriesTable", "options": {"C": {"stat": "lastNotNull"}}},
            {"id": "joinByField", "options": {"byField": "route", "mode": "outer"}},
            {"id": "organize", "options": {
                "excludeByName": {"Time": true},
                "indexByName": {"Trend #C": 3, "Value #A": 1, "Value #B": 2, "route": 0},
                "renameByName": {"Trend #C": "Trend", "Value #A": "RPS", "Value #B": "5xx", "route": "Route"}
            }}
        ])
    );
}

/// A trend column is a range query whatever the author passed in, and a value column is an instant one. Other query options survive.
#[test]
fn column_kinds_set_the_query_type_and_keep_other_options() {
    let panel = panel_json(
        Table::new("Routes")
            .keyed("route")
            .column(Column::new("RPS", promql!(RPS).interval("1m").range(true)))
            .column(Column::trend(
                "Trend",
                promql!(P95).instant(true).datasource(prometheus("other")),
                Reducer::Last,
            )),
    );

    assert_eq!(panel["targets"][0]["instant"], json!(true));
    assert_eq!(panel["targets"][0]["range"], json!(false));
    assert_eq!(panel["targets"][0]["interval"], json!("1m"));
    assert_eq!(panel["targets"][1].get("instant"), None);
    assert_eq!(panel["targets"][1].get("range"), None);
    assert_eq!(panel["targets"][1]["format"], json!("time_series"));
    assert_eq!(
        panel["targets"][1]["datasource"],
        json!({"type": "prometheus", "uid": "other"})
    );
}

/// A table of trend columns only has no table results, so no `Time` fields to hide.
#[test]
fn trend_only_table_hides_nothing() {
    let panel = panel_json(Table::new("Routes").keyed("route").column(Column::trend(
        "Trend",
        promql!(P95),
        Reducer::Last,
    )));
    assert_eq!(organize_of(&panel)["excludeByName"], json!({}));
}

// ---------------------------------------------------------------------------
// Validation: one clear error per mistake, and nothing downstream of it.
// ---------------------------------------------------------------------------

#[test]
fn a_duplicate_column_name_is_one_error() {
    let errors = validation_errors(
        Table::new("Routes")
            .keyed("route")
            .column(Column::new("RPS", promql!(RPS)))
            .column(Column::new("RPS", promql!(ERRORS))),
    );
    assert_eq!(
        errors,
        [ValidationError::DuplicateTableColumn {
            panel: "Routes".to_owned(),
            column: "RPS".to_owned(),
        }]
    );
    assert_eq!(
        errors[0].to_string(),
        "panel \"Routes\": keyed table column \"RPS\" is defined more than once"
    );
}

#[test]
fn a_column_named_like_the_key_is_a_duplicate() {
    let errors = validation_errors(
        Table::new("Routes")
            .keyed("route")
            .column(Column::new("route", promql!(RPS))),
    );
    assert!(matches!(
        errors.as_slice(),
        [ValidationError::DuplicateTableColumn { column, .. }] if column == "route"
    ));

    let errors = validation_errors(
        Table::new("Routes")
            .keyed("route")
            .key_name("Route")
            .column(Column::new("Route", promql!(RPS))),
    );
    assert!(matches!(
        errors.as_slice(),
        [ValidationError::DuplicateTableColumn { column, .. }] if column == "Route"
    ));
}

#[test]
fn an_empty_key_is_one_error() {
    let errors = validation_errors(
        Table::new("Routes")
            .keyed(" ")
            .column(Column::new("RPS", promql!(RPS))),
    );
    assert_eq!(
        errors,
        [ValidationError::MissingTableKey {
            panel: "Routes".to_owned(),
        }]
    );
}

#[test]
fn an_empty_column_name_is_one_error() {
    let errors = validation_errors(
        Table::new("Routes")
            .keyed("route")
            .column(Column::new("", promql!(RPS))),
    );
    assert_eq!(
        errors,
        [ValidationError::MissingTableColumnName {
            panel: "Routes".to_owned(),
        }]
    );
}

#[test]
fn a_valid_keyed_table_validates() {
    let errors = validation_errors(
        Table::new("Routes")
            .keyed("route")
            .key_name("Route")
            .column(Column::new("RPS", promql!(RPS)))
            .column(Column::trend("Trend", promql!(P95), Reducer::Last)),
    );
    assert_eq!(errors, []);
}

// ---------------------------------------------------------------------------
// Golden.
// ---------------------------------------------------------------------------

/// Two keyed tables covering what `route_performance` does not: a renamed key, a single value column named plain `Value`, and table-level settings composing with the lowering.
fn keyed_table_dashboard() -> Dashboard {
    Dashboard::new("Keyed tables")
        .uid("keyed-tables")
        .datasource(prometheus("prometheus-main"))
        .row(
            Row::new("Routes")
                .panel(
                    Table::new("Traffic by route")
                        .width(12)
                        .sort_by("RPS", SortDirection::Descending)
                        .keyed("route")
                        .key_name("Route")
                        .column(Column::new("RPS", promql!(RPS)).unit(Unit::RequestsPerSecond))
                        .column(
                            Column::trend("Traffic", promql!(RPS), Reducer::Mean)
                                .unit(Unit::RequestsPerSecond)
                                .cell(SparklineCell::new().line_width(1.0)),
                        ),
                )
                .panel(
                    Table::new("Errors by route")
                        .width(12)
                        .cell(TableCellType::ColoredText)
                        .override_field(FieldOverride::by_name("5xx").decimals(2))
                        .transform(SortBy::desc("5xx"))
                        .keyed("route")
                        .column(Column::new("RPS", promql!(RPS)).unit(Unit::RequestsPerSecond))
                        .column(
                            Column::new("5xx", promql!(ERRORS))
                                .unit(Unit::RequestsPerSecond)
                                .thresholds(Thresholds::new().green(0.0).red(0.5)),
                        ),
                ),
        )
}

#[test]
fn keyed_tables_match_the_committed_golden_file() {
    let dashboard = keyed_table_dashboard();
    dashboard.validate().unwrap();
    let actual = format!("{}\n", dashboard.to_json_pretty().unwrap());
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/keyed_table.json");

    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::write(&path, &actual).unwrap();
    }

    let expected = fs::read_to_string(path).unwrap();
    assert_eq!(actual, expected);
}
