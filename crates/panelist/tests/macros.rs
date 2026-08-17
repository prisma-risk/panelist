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
use serde_json::json;

#[test]
fn macro_and_builder_produce_the_same_authoring_model() {
    let from_macro = dashboard! {
        title: "Service";
        description: "Health";
        tags: ["service", "production"];
        refresh: "30s";

        variable "environment" {
            values: ["production", "staging"];
            default: "production";
        }

        row "Traffic" {
            timeseries "Requests" {
                query: promql!("rate(requests_total[5m])") {
                    legend: "{{status}}";
                }
                unit: reqps;
                width: 12;
            }
        }
    };

    let from_builder = Dashboard::new("Service")
        .description("Health")
        .tags(["service", "production"])
        .refresh("30s")
        .variable(
            CustomVariable::new("environment", ["production", "staging"]).default("production"),
        )
        .row(
            Row::new("Traffic").panel(
                Timeseries::new("Requests")
                    .query(PrometheusQuery::new("rate(requests_total[5m])").legend("{{status}}"))
                    .unit(Unit::RequestsPerSecond)
                    .width(12),
            ),
        );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}

#[test]
fn macro_supports_all_panel_kinds_and_reusable_fragments() {
    let fragment: Vec<Panel> = vec![Gauge::new("Fragment gauge").into()];
    let dashboard = dashboard! {
        title: "All kinds";

        row "Panels" {
            stat "Stat" {}
            gauge "Gauge" {}
            table "Table" {}
            text "Text" {
                content: "hello";
                mode: markdown;
            }
            bar_gauge "Bar gauge" {}
            heatmap "Heatmap" {}
            panels: fragment;
        }
    };

    dashboard.validate().unwrap();
    let json = dashboard.to_json_pretty().unwrap();
    assert!(json.contains("\"bargauge\""));
    assert!(json.contains("Fragment gauge"));
}

#[test]
fn transform_dsl_matches_the_builder_model() {
    let from_macro = dashboard! {
        title: "Routes";

        table "Route performance" {
            query: promql!("sum by (route) (rate(requests_total[5m]))") {
                ref_id: "A";
            }
            query: promql!("histogram_quantile(0.95, sum by (route, le) (rate(d_bucket[5m])))") {
                ref_id: "D";
            }

            transform join_by_field("route");
            transform organize {
                rename "Value #A" => "RPS";
                hide "Time";
                order ["route", "RPS"];
            }
            transform sort_by("RPS", desc);
            transform time_series_to_table { query "D": last; } only ref_id("D");
            transform labels_to_fields { mode: columns; keep ["route"]; }
        }
    };

    let from_builder = Dashboard::new("Routes").panel(
        Table::new("Route performance")
            .query(PrometheusQuery::new("sum by (route) (rate(requests_total[5m]))").ref_id("A"))
            .query(
                PrometheusQuery::new(
                    "histogram_quantile(0.95, sum by (route, le) (rate(d_bucket[5m])))",
                )
                .ref_id("D"),
            )
            .transform(JoinByField::new("route"))
            .transform(
                OrganizeFields::new()
                    .rename("Value #A", "RPS")
                    .hide("Time")
                    .order(["route", "RPS"]),
            )
            .transform(SortBy::desc("RPS"))
            .transform(
                TimeSeriesToTable::new()
                    .query_with("D", Reducer::Last)
                    .only_ref_id("D"),
            )
            .transform(LabelsToFields::new().keep(["route"])),
    );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}

#[test]
fn transform_dsl_covers_join_sort_raw_and_top_level_panel_kinds() {
    let from_macro = dashboard! {
        title: "Joins and sorting";

        gauge "Gauge" {}
        text "Text" {
            content: "hello";
            mode: markdown;
        }
        bar_gauge "Bar gauge" {}
        heatmap "Heatmap" {}

        table "Multi-source" {
            query: promql!("sum(a)") { ref_id: "A"; }
            query: promql!("sum(b)") { ref_id: "B"; }

            transform join_by_field("host", inner);
            transform join_by_field("cluster", outer) only ref_id("A");
            transform join_by_field("region", outer_tabular) only ref_id("B");
            transform join_by_field("zone") only ref_id("A");
            transform sort_by("host", asc) only ref_id("B");
            transform: RawTransformation::new("filterFieldsByName")
                .option("include", json!("host"));
        }
    };

    let from_builder = Dashboard::new("Joins and sorting")
        .panel(Gauge::new("Gauge"))
        .panel(Text::new("Text").content("hello").mode(TextMode::Markdown))
        .panel(BarGauge::new("Bar gauge"))
        .panel(Heatmap::new("Heatmap"))
        .panel(
            Table::new("Multi-source")
                .query(PrometheusQuery::new("sum(a)").ref_id("A"))
                .query(PrometheusQuery::new("sum(b)").ref_id("B"))
                .transform(JoinByField::new("host").mode(JoinMode::Inner))
                .transform(
                    JoinByField::new("cluster")
                        .mode(JoinMode::Outer)
                        .only_ref_id("A"),
                )
                .transform(
                    JoinByField::new("region")
                        .mode(JoinMode::OuterTabular)
                        .only_ref_id("B"),
                )
                .transform(JoinByField::new("zone").only_ref_id("A"))
                .transform(SortBy::asc("host").only_ref_id("B"))
                .transform(
                    RawTransformation::new("filterFieldsByName").option("include", json!("host")),
                ),
        );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}

#[test]
fn transform_dsl_covers_organize_convert_and_labels_variants() {
    let from_macro = dashboard! {
        title: "Conversions";

        table "Conversions" {
            query: promql!("sum(p)") { ref_id: "P"; }
            query: promql!("sum(q)") { ref_id: "Q"; }

            transform organize {
                rename "Value #P" => "Latency";
            } only ref_id("P");
            transform time_series_to_table {
                query "P";
                time_field "Q": "Time";
            }
            transform labels_to_fields {
                mode: rows;
                value_label: "value";
            } only ref_id("Q");
            transform labels_to_fields;
        }
    };

    let from_builder = Dashboard::new("Conversions").panel(
        Table::new("Conversions")
            .query(PrometheusQuery::new("sum(p)").ref_id("P"))
            .query(PrometheusQuery::new("sum(q)").ref_id("Q"))
            .transform(
                OrganizeFields::new()
                    .rename("Value #P", "Latency")
                    .only_ref_id("P"),
            )
            .transform(TimeSeriesToTable::new().query("P").time_field("Q", "Time"))
            .transform(
                LabelsToFields::new()
                    .mode(LabelsToFieldsMode::Rows)
                    .value_label("value")
                    .only_ref_id("Q"),
            )
            .transform(LabelsToFields::new()),
    );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}
