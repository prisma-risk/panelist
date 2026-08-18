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
                    legend_format: "{{status}}";
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
                    .query(
                        PrometheusQuery::new("rate(requests_total[5m])")
                            .legend_format("{{status}}"),
                    )
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

#[test]
fn table_cell_and_sort_dsl_matches_the_builder_model() {
    let from_macro = dashboard! {
        title: "Cells";

        table "Route performance" {
            query: promql!("up");

            sort_by: ("p95", desc);

            override field("Error rate") {
                unit: percent;
                cell: colored_background;
                thresholds {
                    green: 0.0;
                    yellow: 1.0;
                    red: 5.0;
                }
            }

            override field("Trend") {
                cell: sparkline { hide_value: true; line_width: 2.0; };
            }

            override numeric {
                decimals: 2;
            }
        }
    };

    let from_builder = Dashboard::new("Cells").panel(
        Table::new("Route performance")
            .query(PrometheusQuery::new("up"))
            .sort_by("p95", SortDirection::Descending)
            .override_field(
                FieldOverride::by_name("Error rate")
                    .unit(Unit::Percent)
                    .cell_type(TableCellType::ColoredBackground)
                    .thresholds(Thresholds::new().green(0.0).yellow(1.0).red(5.0)),
            )
            .override_field(
                FieldOverride::by_name("Trend")
                    .cell(SparklineCell::new().hide_value(true).line_width(2.0)),
            )
            .override_field(FieldOverride::numeric_fields().decimals(2)),
    );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}

#[test]
fn table_override_matchers_and_properties_dsl_matches_the_builder_model() {
    let from_macro = dashboard! {
        title: "Overrides";

        table "Service overview" {
            query: promql!("up") { ref_id: "A"; }

            override query("A") {
                display_name: "Latency (ms)";
                min: 0.0;
                max: 500.0;
            }

            override type("number") {
                unit: custom("USD");
                cell: colored_text;
            }

            override names ["Latency", "Errors"] {
                cell: colored_background { mode: gradient; apply_to_row: true; };
            }

            override time {
                cell: auto;
            }
        }
    };

    let from_builder = Dashboard::new("Overrides").panel(
        Table::new("Service overview")
            .query(PrometheusQuery::new("up").ref_id("A"))
            .override_field(
                FieldOverride::by_query("A")
                    .display_name("Latency (ms)")
                    .min(0.0)
                    .max(500.0),
            )
            .override_field(
                FieldOverride::by_type("number")
                    .unit(Unit::custom("USD"))
                    .cell_type(TableCellType::ColoredText),
            )
            .override_field(
                FieldOverride::by_names(["Latency", "Errors"]).cell(
                    ColoredBackgroundCell::new()
                        .mode(CellBackgroundMode::Gradient)
                        .apply_to_row(true),
                ),
            )
            .override_field(FieldOverride::time_fields().cell_type(TableCellType::Auto)),
    );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}

#[test]
fn heatmap_dsl_and_top_level_panels_match_the_builder_model() {
    let from_macro = dashboard! {
        title: "Latency";

        table "Top level table" {}

        heatmap "Latency distribution" {
            query: promql!("sum by (le) (rate(duration_bucket[5m]))") {
                format: heatmap;
            }
            unit: seconds;
            color_scheme: "Blues";
            color_steps: 32;
            cell_gap: 2;
            calculate: false;
            show_legend: true;

            y_axis {
                unit: seconds;
                placement: left;
            }
        }
    };

    let from_builder = Dashboard::new("Latency")
        .panel(Table::new("Top level table"))
        .panel(
            Heatmap::new("Latency distribution")
                .query(
                    PrometheusQuery::new("sum by (le) (rate(duration_bucket[5m]))")
                        .format(PrometheusFormat::Heatmap),
                )
                .unit(Unit::Seconds)
                .color_scheme("Blues")
                .color_steps(32)
                .cell_gap(2)
                .calculate(false)
                .show_legend(true)
                .y_axis_unit(Unit::Seconds)
                .y_axis_placement(AxisPlacement::Left),
        );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}

#[test]
fn heatmap_dsl_covers_format_color_mode_and_axis_placement_variants() {
    // `heatmap_dsl_and_top_level_panels_match_the_builder_model` above only
    // exercises `format: heatmap;`, `color_mode`'s default-masking-unsafe-if-
    // untested `scheme` case is skipped there entirely, `y_axis { unit: … }`
    // only exercises the bare-ident unit form, and `placement: left;` is the
    // only one of `__panelist_axis_placement!`'s six arms it reaches. This
    // test rounds out every remaining `format:`, `color_mode:`, y-axis-unit,
    // and axis-placement arm so each new rule has DSL-vs-builder coverage.
    let from_macro = dashboard! {
        title: "Heatmap variants";

        heatmap "Time series format" {
            query: promql!("a") { format: time_series; }
        }
        heatmap "Table format" {
            query: promql!("b") { format: table; }
        }
        heatmap "Scheme color mode" {
            color_mode: scheme;
        }
        heatmap "Opacity color mode" {
            color_mode: opacity;
        }
        heatmap "Custom y axis unit" {
            y_axis { unit: custom("widgets"); }
        }
        heatmap "Auto placement" {
            y_axis { placement: auto; }
        }
        heatmap "Bottom placement" {
            y_axis { placement: bottom; }
        }
        heatmap "Hidden placement" {
            y_axis { placement: hidden; }
        }
        heatmap "Right placement" {
            y_axis { placement: right; }
        }
        heatmap "Top placement" {
            y_axis { placement: top; }
        }
    };

    let from_builder = Dashboard::new("Heatmap variants")
        .panel(
            Heatmap::new("Time series format")
                .query(PrometheusQuery::new("a").format(PrometheusFormat::TimeSeries)),
        )
        .panel(
            Heatmap::new("Table format")
                .query(PrometheusQuery::new("b").format(PrometheusFormat::Table)),
        )
        .panel(Heatmap::new("Scheme color mode").color_mode(HeatmapColorMode::Scheme))
        .panel(Heatmap::new("Opacity color mode").color_mode(HeatmapColorMode::Opacity))
        .panel(Heatmap::new("Custom y axis unit").y_axis_unit(Unit::Custom("widgets".to_owned())))
        .panel(Heatmap::new("Auto placement").y_axis_placement(AxisPlacement::Auto))
        .panel(Heatmap::new("Bottom placement").y_axis_placement(AxisPlacement::Bottom))
        .panel(Heatmap::new("Hidden placement").y_axis_placement(AxisPlacement::Hidden))
        .panel(Heatmap::new("Right placement").y_axis_placement(AxisPlacement::Right))
        .panel(Heatmap::new("Top placement").y_axis_placement(AxisPlacement::Top));

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}

#[test]
fn stacking_dsl_matches_the_builder_model() {
    // Covers all three `__panelist_stacking_mode!` arms. `none` is
    // `StackingMode`'s `#[default]` variant, so it is paired here with an
    // explicit `.stacking(Stacking::new(StackingMode::None))` on the
    // builder side rather than an omitted call, so a dropped macro arm
    // would still produce a detectable `None` vs. `Some(..)` mismatch.
    let from_macro = dashboard! {
        title: "Stacking";

        timeseries "Normal stacking" {
            query: promql!("a");
            stacking: normal;
        }
        timeseries "Percent stacking" {
            query: promql!("b");
            stacking: percent;
        }
        timeseries "No stacking" {
            query: promql!("c");
            stacking: none;
        }
    };

    let from_builder = Dashboard::new("Stacking")
        .panel(
            Timeseries::new("Normal stacking")
                .query(PrometheusQuery::new("a"))
                .stacking(Stacking::new(StackingMode::Normal)),
        )
        .panel(
            Timeseries::new("Percent stacking")
                .query(PrometheusQuery::new("b"))
                .stacking(Stacking::new(StackingMode::Percent)),
        )
        .panel(
            Timeseries::new("No stacking")
                .query(PrometheusQuery::new("c"))
                .stacking(Stacking::new(StackingMode::None)),
        );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}

#[test]
fn unit_percent_unit_dsl_matches_the_builder_model() {
    let from_macro = dashboard! {
        title: "Percent unit";

        stat "Error ratio" {
            query: promql!("a");
            unit: percent_unit;
        }
    };

    let from_builder = Dashboard::new("Percent unit").panel(
        Stat::new("Error ratio")
            .query(PrometheusQuery::new("a"))
            .unit(Unit::PercentUnit),
    );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}

/// `GaugeCell::mode` and `GaugeCell::value_display` were reachable from the
/// builders but from neither DSL `cell:` position, and the panel-level
/// `cell:` accepted only a bare renderer name, so no table default cell
/// could carry options at all. This covers every arm of both new block
/// forms; the builder side sets each option explicitly so a dropped macro
/// arm shows up as `None` vs `Some(..)` rather than agreeing by default.
#[test]
fn cell_option_blocks_dsl_matches_the_builder_model() {
    let from_macro = dashboard! {
        title: "Gauge cells";

        table "Load" {
            query: promql!("up");

            cell: gauge { mode: lcd; value_display: hidden; };

            override field("Basic") {
                cell: gauge { mode: basic; value_display: text; };
            }

            override field("Gradient") {
                cell: gauge { mode: gradient; value_display: color; };
            }
        }

        table "Panel sparkline default" {
            query: promql!("up");
            cell: sparkline { hide_value: true; line_width: 2.0; fill_opacity: 20.0; };
        }

        table "Panel background default" {
            query: promql!("up");
            cell: colored_background { mode: gradient; apply_to_row: true; };
        }
    };

    let from_builder = Dashboard::new("Gauge cells")
        .panel(
            Table::new("Load")
                .query(PrometheusQuery::new("up"))
                .cell(
                    GaugeCell::new()
                        .mode(BarGaugeDisplayMode::Lcd)
                        .value_display(CellValueDisplay::Hidden),
                )
                .override_field(
                    FieldOverride::by_name("Basic").cell(
                        GaugeCell::new()
                            .mode(BarGaugeDisplayMode::Basic)
                            .value_display(CellValueDisplay::Text),
                    ),
                )
                .override_field(
                    FieldOverride::by_name("Gradient").cell(
                        GaugeCell::new()
                            .mode(BarGaugeDisplayMode::Gradient)
                            .value_display(CellValueDisplay::Color),
                    ),
                ),
        )
        .panel(
            Table::new("Panel sparkline default")
                .query(PrometheusQuery::new("up"))
                .cell(
                    SparklineCell::new()
                        .hide_value(true)
                        .line_width(2.0)
                        .fill_opacity(20.0),
                ),
        )
        .panel(
            Table::new("Panel background default")
                .query(PrometheusQuery::new("up"))
                .cell(
                    ColoredBackgroundCell::new()
                        .mode(CellBackgroundMode::Gradient)
                        .apply_to_row(true),
                ),
        );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}

/// The transformation envelope — `only_frame_name`, `only_frame_index`, and
/// `disabled` — has no dedicated `transform …` keyword form. It is reachable
/// from the DSL through the typed-expression rule `transform: <expr>;`,
/// which takes any `impl Into<Transformation>`. That is a first-class typed
/// rule, not a raw-JSON escape hatch, so the two surfaces stay at parity;
/// this pins that they actually agree.
#[test]
fn transformation_envelope_dsl_matches_the_builder_model() {
    let from_macro = dashboard! {
        title: "Envelopes";

        table "Filtered" {
            query: promql!("sum(a)") { ref_id: "A"; }

            transform: SortBy::desc("p95").only_frame_name("latency");
            transform: JoinByField::new("route").only_frame_index(1);
            transform: LabelsToFields::new().disabled(true);
        }
    };

    let from_builder = Dashboard::new("Envelopes").panel(
        Table::new("Filtered")
            .query(PrometheusQuery::new("sum(a)").ref_id("A"))
            .transform(SortBy::desc("p95").only_frame_name("latency"))
            .transform(JoinByField::new("route").only_frame_index(1))
            .transform(LabelsToFields::new().disabled(true)),
    );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}

#[test]
fn table_panel_default_cell_and_remaining_cell_variants_dsl_matches_the_builder_model() {
    let from_macro = dashboard! {
        title: "Cell defaults";

        table "Latency" {
            query: promql!("up");

            cell: gauge;

            override field("Raw") {
                cell: sparkline;
            }

            override regex("^p9.*") {
                cell: sparkline { fill_opacity: 20.0; };
            }

            override field("Steady") {
                cell: colored_background { mode: basic; };
            }
        }
    };

    let from_builder = Dashboard::new("Cell defaults").panel(
        Table::new("Latency")
            .query(PrometheusQuery::new("up"))
            .cell(TableCellType::Gauge)
            .override_field(FieldOverride::by_name("Raw").cell_type(TableCellType::Sparkline))
            .override_field(
                FieldOverride::by_regex("^p9.*").cell(SparklineCell::new().fill_opacity(20.0)),
            )
            .override_field(
                FieldOverride::by_name("Steady")
                    .cell(ColoredBackgroundCell::new().mode(CellBackgroundMode::Basic)),
            ),
    );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}
