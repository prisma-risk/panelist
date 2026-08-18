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

/// The transformation envelope — `only_frame_name` and `disabled` — has no dedicated `transform …` keyword form. It is reachable
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
            transform: LabelsToFields::new().disabled(true);
        }
    };

    let from_builder = Dashboard::new("Envelopes").panel(
        Table::new("Filtered")
            .query(PrometheusQuery::new("sum(a)").ref_id("A"))
            .transform(SortBy::desc("p95").only_frame_name("latency"))
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

/// Every variant is deliberately a NON-default one: `StatGraphMode::Area`,
/// `Orientation::Auto` and `BarGaugeDisplayMode::Gradient` are the
/// `#[default]`s, and this test's whole job is to catch a DSL arm that
/// silently does nothing. A default-valued variant on a bare (non-`Option`)
/// field would leave both sides equal whether the arm fires or not, so the
/// assertion would pass against a macro that dropped the rule entirely.
#[test]
fn panel_option_dsl_matches_the_builder_model() {
    let from_macro = dashboard! {
        title: "Panel options";

        stat "Requests" {
            query: promql!("rate(requests_total[5m])");
            graph_mode: none;
            orientation: vertical;
        }

        gauge "Saturation" {
            query: promql!("saturation");
            orientation: horizontal;
        }

        bar_gauge "Top routes" {
            query: promql!("topk(10, rate(requests_total[5m]))");
            display_mode: lcd;
            orientation: horizontal;
        }
    };

    let from_builder = Dashboard::new("Panel options")
        .panel(
            Stat::new("Requests")
                .query(PrometheusQuery::new("rate(requests_total[5m])"))
                .graph_mode(StatGraphMode::None)
                .orientation(Orientation::Vertical),
        )
        .panel(
            Gauge::new("Saturation")
                .query(PrometheusQuery::new("saturation"))
                .orientation(Orientation::Horizontal),
        )
        .panel(
            BarGauge::new("Top routes")
                .query(PrometheusQuery::new("topk(10, rate(requests_total[5m]))"))
                .display_mode(BarGaugeDisplayMode::Lcd)
                .orientation(Orientation::Horizontal),
        );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}

/// Pins the emitted wire values, not just DSL/builder agreement. The parity
/// test above compares the two surfaces to each other, so it stays green if
/// BOTH are wrong; this one is the check that can disagree with them.
#[test]
fn panel_option_dsl_emits_the_grafana_wire_values() {
    let json = dashboard! {
        title: "Panel options";

        stat "Requests" {
            query: promql!("up");
            graph_mode: none;
            orientation: vertical;
        }

        bar_gauge "Top routes" {
            query: promql!("up");
            display_mode: lcd;
        }
    };
    let json: serde_json::Value = serde_json::from_str(&json.to_json().unwrap()).unwrap();

    assert_eq!(json["panels"][0]["options"]["graphMode"], json!("none"));
    assert_eq!(
        json["panels"][0]["options"]["orientation"],
        json!("vertical")
    );
    assert_eq!(json["panels"][1]["options"]["displayMode"], json!("lcd"));
}

/// Closes the DSL-parity gaps tracked in issue #8.
///
/// Every enum value here is deliberately NOT the type's `#[default]`:
/// `LineInterpolation::Linear`, `PointVisibility::Auto`, `StatColorMode::Value`,
/// `TooltipMode::Single` and `TooltipSort::None` are the defaults, and this
/// assertion cannot see a DSL arm that silently does nothing when the value it
/// would have set is what the field already holds.
#[test]
fn panel_option_parity_dsl_matches_the_builder_model() {
    let from_macro = dashboard! {
        title: "Parity";

        timeseries "Latency" {
            query: promql!("latency");
            fill_opacity: 30.0;
            line_width: 3.0;
            point_size: 7.0;
            span_nulls: true;
            line_interpolation: step_after;
            show_points: never;
            tooltip { mode: multi; sort: desc; hide_zeros: true; }
        }

        stat "Status" {
            query: promql!("up");
            color_mode: background;
            wide_layout: true;
            color: fixed(red);
            mapping "0" => "Down";
            mapping "1" => "Up" { color: green; }
            link "Runbook" => "https://runbook";
            link "Dashboard" => "https://dash" { target_blank: true; tags: ["ops"]; }
            reduce { values: true; calculations: [mean, max]; fields: "/.*/"; }
        }

        gauge "Saturation" {
            query: promql!("saturation");
            color: continuous("BlYlRd");
            reduce { values: false; }
        }

        bar_gauge "Routes" {
            query: promql!("routes");
            color: classic_palette;
            reduce { fields: "/route/"; }
        }
    };

    let from_builder = Dashboard::new("Parity")
        .panel(
            Timeseries::new("Latency")
                .query(PrometheusQuery::new("latency"))
                .fill_opacity(30.0)
                .line_width(3.0)
                .point_size(7.0)
                .span_nulls(true)
                .line_interpolation(LineInterpolation::StepAfter)
                .show_points(PointVisibility::Never)
                .tooltip(
                    Tooltip::new()
                        .mode(TooltipMode::Multi)
                        .sort(TooltipSort::Descending)
                        .hide_zeros(true),
                ),
        )
        .panel(
            Stat::new("Status")
                .query(PrometheusQuery::new("up"))
                .color_mode(StatColorMode::Background)
                .wide_layout(true)
                .color(ColorScheme::Fixed(Color::Red))
                .mapping(ValueMapping::new("0", "Down"))
                .mapping(ValueMapping::new("1", "Up").color(Color::Green))
                .link(DashboardLink::new("Runbook", "https://runbook"))
                .link(
                    DashboardLink::new("Dashboard", "https://dash")
                        .target_blank(true)
                        .tags(["ops"]),
                )
                .reduce_options(
                    ReduceOptions::new()
                        .values(true)
                        .calculations([Reducer::Mean, Reducer::Max])
                        .fields("/.*/"),
                ),
        )
        .panel(
            Gauge::new("Saturation")
                .query(PrometheusQuery::new("saturation"))
                .color(ColorScheme::Continuous("BlYlRd".to_owned()))
                .reduce_options(ReduceOptions::new().values(false)),
        )
        .panel(
            BarGauge::new("Routes")
                .query(PrometheusQuery::new("routes"))
                .color(ColorScheme::ClassicPalette)
                .reduce_options(ReduceOptions::new().fields("/route/")),
        );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}

/// Pins the emitted wire keys and value spellings. The parity test above only
/// proves the two surfaces agree with each other, which stays true when both
/// are wrong; this is the assertion that can disagree with both.
#[test]
fn panel_option_parity_emits_the_grafana_wire_values() {
    let dashboard = dashboard! {
        title: "Wire";

        timeseries "Latency" {
            query: promql!("latency");
            fill_opacity: 30.0;
            line_width: 3.0;
            point_size: 7.0;
            span_nulls: true;
            line_interpolation: step_after;
            show_points: never;
            tooltip { mode: multi; sort: desc; hide_zeros: true; }
        }

        stat "Status" {
            query: promql!("up");
            color_mode: background;
            wide_layout: true;
            color: fixed(red);
            mapping "1" => "Up" { color: green; }
            link "Runbook" => "https://runbook" { target_blank: true; tags: ["ops"]; }
            reduce { values: true; calculations: [mean, max]; fields: "/.*/"; }
        }
    };
    let json: serde_json::Value = serde_json::from_str(&dashboard.to_json().unwrap()).unwrap();

    let series = &json["panels"][0];
    assert_eq!(
        series["fieldConfig"]["defaults"]["custom"],
        json!({
            "drawStyle": "line",
            "fillOpacity": 30.0,
            "lineInterpolation": "stepAfter",
            "lineWidth": 3.0,
            "pointSize": 7.0,
            "showPoints": "never",
            "spanNulls": true
        })
    );
    assert_eq!(
        series["options"]["tooltip"],
        json!({"hideZeros": true, "mode": "multi", "sort": "desc"})
    );

    let stat = &json["panels"][1];
    assert_eq!(stat["options"]["colorMode"], json!("background"));
    assert_eq!(stat["options"]["wideLayout"], json!(true));
    assert_eq!(
        stat["options"]["reduceOptions"],
        json!({"calcs": ["mean", "max"], "fields": "/.*/", "values": true})
    );
    assert_eq!(
        stat["fieldConfig"]["defaults"]["color"],
        json!({"fixedColor": "red", "mode": "fixed"})
    );
    assert_eq!(
        stat["fieldConfig"]["defaults"]["mappings"],
        json!([{"options": {"1": {"color": "green", "text": "Up"}}, "type": "value"}])
    );
    assert_eq!(
        stat["links"],
        json!([{
            "includeVars": false, "keepTime": false, "tags": ["ops"],
            "targetBlank": true, "title": "Runbook", "type": "link",
            "url": "https://runbook"
        }])
    );
}

/// Closes the dashboard- and variable-level DSL parity gaps from issue #12.
///
/// Non-default values throughout: `VariableSort::Disabled`,
/// `DashboardCursorSync::Off` and `selected: true` are the defaults, and a
/// DSL-vs-builder assertion cannot see an arm that does nothing when the
/// value it would set is what the field already holds.
#[test]
fn dashboard_and_variable_parity_dsl_matches_the_builder_model() {
    let from_macro = dashboard! {
        title: "Parity";
        cursor_sync: crosshair;
        link "Runbook" => "https://runbook";
        link "Dashboards" => "https://dash" { target_blank: true; tags: ["ops"]; }

        variable "instance" {
            query: promql!("label_values(up, instance)");
            regex: "prod-.*";
            sort: alphabetical_case_insensitive_desc;
            all_value: ".*";
            allow_custom_value: true;
            skip_url_sync: true;
            current "Production" => "prod" { selected: false; }
        }

        variable "tier" {
            values: ["gold", "silver"];
            all_value: "*";
            allow_custom_value: true;
            skip_url_sync: true;
            current "Gold" => "gold";
        }
    };

    let from_builder = Dashboard::new("Parity")
        .cursor_sync(DashboardCursorSync::Crosshair)
        .link(DashboardLink::new("Runbook", "https://runbook"))
        .link(
            DashboardLink::new("Dashboards", "https://dash")
                .target_blank(true)
                .tags(["ops"]),
        )
        .variable(
            VariableBuilder::new("instance")
                .query(PrometheusQuery::new("label_values(up, instance)"))
                .regex("prod-.*")
                .sort(VariableSort::AlphabeticalCaseInsensitiveDescending)
                .all_value(".*")
                .allow_custom_value(true)
                .skip_url_sync(true)
                .current(VariableSelection::new("Production", "prod").selected(false))
                .build(),
        )
        .variable(
            VariableBuilder::new("tier")
                .values(["gold", "silver"])
                .all_value("*")
                .allow_custom_value(true)
                .skip_url_sync(true)
                .current(VariableSelection::new("Gold", "gold"))
                .build(),
        );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}

/// Pins the emitted wire keys. The parity test above only shows the two
/// surfaces agree, which stays true when both are wrong together.
#[test]
fn dashboard_and_variable_parity_emits_the_grafana_wire_values() {
    let dashboard = dashboard! {
        title: "Wire";
        cursor_sync: tooltip;
        link "Runbook" => "https://runbook" { target_blank: true; }

        variable "instance" {
            query: promql!("label_values(up, instance)");
            regex: "prod-.*";
            sort: numerical_desc;
            all_value: ".*";
            allow_custom_value: true;
            skip_url_sync: true;
            current "Production" => "prod";
        }
    };
    let json: serde_json::Value = serde_json::from_str(&dashboard.to_json().unwrap()).unwrap();

    assert_eq!(json["graphTooltip"], json!(2));
    assert_eq!(json["links"][0]["targetBlank"], json!(true));

    let variable = &json["templating"]["list"][0];
    assert_eq!(variable["regex"], json!("prod-.*"));
    // Grafana encodes VariableSort numerically; NumericalDescending is 4.
    assert_eq!(variable["sort"], json!(4));
    assert_eq!(variable["allValue"], json!(".*"));
    assert_eq!(variable["allowCustomValue"], json!(true));
    assert_eq!(variable["skipUrlSync"], json!(true));
    assert_eq!(
        variable["current"],
        json!({"selected": true, "text": "Production", "value": "prod"})
    );
}

/// `regex` and `sort` have no Grafana key on a custom variable. Setting them
/// has to fail loudly: in the emitted JSON a silently ignored `sort` is
/// indistinguishable from one that worked.
#[test]
fn variable_options_that_do_not_apply_are_reported_not_dropped() {
    let dashboard = dashboard! {
        title: "Bad";
        variable "tier" {
            values: ["gold"];
            regex: "ignored";
            sort: numerical_desc;
        }
    };

    let error = dashboard.to_json().expect_err("must not serialize");
    let rendered = error.to_string();
    assert!(
        rendered.contains("regex does not apply to a custom variable"),
        "{rendered}"
    );
    assert!(
        rendered.contains("sort does not apply to a custom variable"),
        "{rendered}"
    );
}
