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
    // only one of the six axis placements it reaches. This
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
    // Covers all three stacking-mode words. `none` is
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

/// Closes issue #14: the DSL can now produce a datasource variable, and the
/// `plugin:` selector is distinct from `datasource:`.
#[test]
fn datasource_variable_dsl_matches_the_builder_model() {
    let from_macro = dashboard! {
        title: "Sources";
        variable "source" {
            plugin: "prometheus";
            label: "Data source";
            regex: "prod-.*";
            skip_url_sync: true;
            refresh: time_range;
            current "Main" => "prometheus-main" { selected: false; }
        }
    };

    let from_builder = Dashboard::new("Sources").variable(
        VariableBuilder::new("source")
            .plugin("prometheus")
            .label("Data source")
            .regex("prod-.*")
            .skip_url_sync(true)
            .refresh(VariableRefresh::OnTimeRangeChange)
            .current(VariableSelection::new("Main", "prometheus-main").selected(false))
            .build(),
    );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}

/// Pins the wire shape. `type` and `query` are what distinguish a datasource
/// variable from the custom one this used to silently produce.
#[test]
fn datasource_variable_emits_the_grafana_wire_values() {
    let dashboard = dashboard! {
        title: "Sources";
        variable "source" {
            plugin: "prometheus";
            regex: "prod-.*";
            skip_url_sync: true;
            current "Main" => "prometheus-main";
        }
    };
    let json: serde_json::Value = serde_json::from_str(&dashboard.to_json().unwrap()).unwrap();
    let variable = &json["templating"]["list"][0];

    assert_eq!(variable["type"], json!("datasource"));
    assert_eq!(variable["query"], json!("prometheus"));
    assert_eq!(variable["regex"], json!("prod-.*"));
    assert_eq!(variable["skipUrlSync"], json!(true));
    assert_eq!(
        variable["current"],
        json!({"selected": true, "text": "Main", "value": "prometheus-main"})
    );
}

/// `datasource:` sets the datasource a QUERY variable runs against. It must
/// keep meaning that, and must not select the variable's kind.
#[test]
fn datasource_option_still_configures_a_query_variable() {
    let dashboard = dashboard! {
        title: "Sources";
        variable "instance" {
            query: promql!("label_values(up, instance)");
            datasource: prometheus("prometheus-main");
        }
    };
    let json: serde_json::Value = serde_json::from_str(&dashboard.to_json().unwrap()).unwrap();
    let variable = &json["templating"]["list"][0];

    assert_eq!(variable["type"], json!("query"));
    assert_eq!(variable["datasource"]["uid"], json!("prometheus-main"));
}

/// The defect this issue was filed for: a variable whose only option was
/// `datasource:` used to serialize cleanly as an empty CUSTOM variable.
/// Every degenerate selector combination now fails before serialization.
#[test]
fn variable_selector_mistakes_are_reported_not_guessed() {
    let cases = [
        (
            dashboard! { title: "t"; variable "d" { datasource: prometheus("p"); } },
            "no selector",
        ),
        (
            dashboard! { title: "t"; variable "x" { label: "nothing"; } },
            "no selector",
        ),
        (
            dashboard! { title: "t"; variable "b" { query: promql!("up"); plugin: "prometheus"; } },
            "plugin is ignored because query already selected the kind",
        ),
        (
            dashboard! { title: "t"; variable "s" { plugin: "prometheus"; sort: numerical_desc; } },
            "sort does not apply to a datasource variable",
        ),
    ];
    for (dashboard, expected) in cases {
        let error = dashboard.to_json().expect_err("must not serialize");
        let rendered = error.to_string();
        assert!(
            rendered.contains(expected),
            "expected {expected:?} in {rendered:?}"
        );
    }
}

/// Titles, names, link targets, mapping values, and organize arguments are
/// ordinary expressions, so dashboards can be generated from data.
#[test]
fn expressions_are_accepted_wherever_a_literal_used_to_be_required() {
    let service = "checkout";
    let environments = vec!["production".to_owned(), "staging".to_owned()];
    let order: Vec<String> = vec!["route".into(), "RPS".into()];
    let runbook = format!("https://runbooks.example.com/{service}");

    let from_macro = dashboard! {
        title: format!("{service} overview");
        link format!("{service} runbook") => runbook.clone();

        variable format!("{service}_environment") {
            values: environments.clone();
            current environments[0].clone() => environments[0].clone();
        }

        row format!("{service} traffic") {
            stat format!("{service} errors") {
                query: promql!(format!("sum(rate({service}_errors_total[5m]))"));
                mapping 0.to_string() => format!("{service} healthy");
                link service.to_uppercase() => runbook.clone() { target_blank: true; }
            }
            table (format!("{service} routes")) {
                transform organize {
                    rename format!("Value #{}", "A") => service.to_owned() + " RPS";
                    hide String::from("Time");
                    order order.clone();
                }
            }
        }
    };

    let from_builder = Dashboard::new("checkout overview")
        .link(DashboardLink::new(
            "checkout runbook",
            "https://runbooks.example.com/checkout",
        ))
        .variable(
            VariableBuilder::new("checkout_environment")
                .values(["production", "staging"])
                .current(VariableSelection::new("production", "production"))
                .build(),
        )
        .row(
            Row::new("checkout traffic")
                .panel(
                    Stat::new("checkout errors")
                        .query(PrometheusQuery::new("sum(rate(checkout_errors_total[5m]))"))
                        .mapping(ValueMapping::new("0", "checkout healthy"))
                        .link(
                            DashboardLink::new("CHECKOUT", "https://runbooks.example.com/checkout")
                                .target_blank(true),
                        ),
                )
                .panel(
                    Table::new("checkout routes").transform(
                        OrganizeFields::new()
                            .rename("Value #A", "checkout RPS")
                            .hide("Time")
                            .order(["route", "RPS"]),
                    ),
                ),
        );

    assert_eq!(from_macro, from_builder);
}

/// Panels can be generated in Rust and dropped in one at a time with
/// `panel:`, at dashboard and row level, alongside the `panels:` fragment
/// form.
#[test]
fn single_panel_items_match_the_builder_model() {
    let services = ["auth", "billing"];
    let from_macro = dashboard! {
        title: "Services";
        panel: Stat::new("Top level").unit(Unit::Percent);

        row "Per service" {
            panel: Timeseries::new(format!("{} latency", services[0]));
            panels: services[1..].iter().map(|service| Stat::new(format!("{service} errors")));
            panel: Text::new("Notes").content("hello");
        }
    };

    let from_builder = Dashboard::new("Services")
        .panel(Stat::new("Top level").unit(Unit::Percent))
        .row(
            Row::new("Per service")
                .panel(Timeseries::new("auth latency"))
                .panel(Stat::new("billing errors"))
                .panel(Text::new("Notes").content("hello")),
        );

    assert_eq!(from_macro, from_builder);
}

mod house {
    use panelist::prelude::*;

    /// A house-default stat that every team starts from.
    pub fn stat(title: &str) -> Stat {
        Stat::new(title)
            .unit(Unit::Percent)
            .graph_mode(StatGraphMode::None)
            .width(4)
    }

    pub fn row(title: &str) -> Row {
        Row::new(title).collapsed(true)
    }
}

/// `from <builder>` continues an existing builder. The base keeps its own
/// title and every key in the block applies on top of it.
#[test]
fn panels_and_rows_continue_from_a_base_builder() {
    let from_macro = dashboard! {
        title: "House style";

        stat from house::stat("Availability") {
            query: promql!("avg(up)");
            width: 6;
        }

        row from house::row("Details") {
            stat from house::stat("Saturation") {}
            timeseries from Timeseries::new("Latency").unit(Unit::Seconds) {
                fill_opacity: 10.0;
            }
        }
    };

    let from_builder = Dashboard::new("House style")
        .panel(
            house::stat("Availability")
                .query(PrometheusQuery::new("avg(up)"))
                .width(6),
        )
        .row(
            house::row("Details")
                .panel(house::stat("Saturation"))
                .panel(
                    Timeseries::new("Latency")
                        .unit(Unit::Seconds)
                        .fill_opacity(10.0),
                ),
        );

    assert_eq!(from_macro, from_builder);
}

/// The general thresholds grammar: a mandatory base color, then ascending
/// `<value> => <color>` steps with any `Color`, including custom names.
#[test]
fn thresholds_with_a_base_and_arbitrary_colors_match_the_builder_model() {
    let warn = 70.5;
    let from_macro = dashboard! {
        title: "Thresholds";

        gauge "Saturation" {
            thresholds {
                base: blue;
                1 => green;
                warn => orange;
                90.0 => "dark-red";
                99 => Color::custom("#ff00ff");
            }
        }

        stat "Ratio" {
            thresholds {
                mode: percentage;
                base: green;
                80 => purple;
            }
        }

        table "Cells" {
            override field("Errors") {
                thresholds { base: "transparent"; 5 => red; }
            }
        }
    };

    let from_builder = Dashboard::new("Thresholds")
        .panel(
            Gauge::new("Saturation").thresholds(
                Thresholds::new()
                    .step(Color::Blue, None)
                    .step(Color::Green, 1.0)
                    .step(Color::Orange, 70.5)
                    .step(Color::custom("dark-red"), 90.0)
                    .step(Color::custom("#ff00ff"), 99.0),
            ),
        )
        .panel(
            Stat::new("Ratio").thresholds(
                Thresholds::new()
                    .mode(ThresholdMode::Percentage)
                    .step(Color::Green, None)
                    .step(Color::Purple, 80.0),
            ),
        )
        .panel(
            Table::new("Cells").override_field(
                FieldOverride::by_name("Errors").thresholds(
                    Thresholds::new()
                        .step(Color::custom("transparent"), None)
                        .step(Color::Red, 5.0),
                ),
            ),
        );

    assert_eq!(from_macro, from_builder);
    from_macro.validate().unwrap();
}

/// Keys that take a word also take any expression of the builder argument's
/// type, so house constants and computed values need no special syntax.
#[test]
fn word_valued_keys_accept_expressions() {
    let orientation = Orientation::Vertical;
    let from_macro = dashboard! {
        title: "Expressions";
        cursor_sync: DashboardCursorSync::Tooltip;
        time: TimeRange::new("now-1h", "now");

        stat "Status" {
            unit: Unit::Bytes;
            orientation: (orientation);
            color: ColorScheme::Fixed(Color::Blue);
            reduce { calculations: vec![Reducer::Last]; }
            mapping "1" => "Up" { color: Color::custom("dark-green"); }
        }

        timeseries "Series" {
            stacking: Stacking::new(StackingMode::Normal).group("A");
            tooltip { mode: TooltipMode::Multi; }
            grid: GridPos::new(0, 0, 12, 8);
        }

        table "Cells" {
            cell: GaugeCell::new().mode(BarGaugeDisplayMode::Lcd);
            sort_by: ("p95", SortDirection::Ascending);
        }
    };

    let from_builder = Dashboard::new("Expressions")
        .cursor_sync(DashboardCursorSync::Tooltip)
        .time(TimeRange::new("now-1h", "now"))
        .panel(
            Stat::new("Status")
                .unit(Unit::Bytes)
                .orientation(Orientation::Vertical)
                .color(ColorScheme::Fixed(Color::Blue))
                .reduce_options(ReduceOptions::new().calculations([Reducer::Last]))
                .mapping(ValueMapping::new("1", "Up").color(Color::custom("dark-green"))),
        )
        .panel(
            Timeseries::new("Series")
                .stacking(Stacking::new(StackingMode::Normal).group("A"))
                .tooltip(Tooltip::new().mode(TooltipMode::Multi))
                .grid_pos(GridPos::new(0, 0, 12, 8)),
        )
        .panel(
            Table::new("Cells")
                .cell(GaugeCell::new().mode(BarGaugeDisplayMode::Lcd))
                .sort_by("p95", SortDirection::Ascending),
        );

    assert_eq!(from_macro, from_builder);
}

/// Keys the DSL gained alongside the proc-macro rewrite, each mapped to the
/// builder method it names.
#[test]
fn newly_reachable_builder_options_match_the_builder_model() {
    let from_macro = dashboard! {
        title: "New keys";
        schema_version: 39;
        version: 3;
        time: ["now-6h", "now"];
        extra "weekStart": "monday";

        table "Scoped" {
            query: promql!("up") { ref_id: "A"; editor_mode: code; }
            query: loki!("{job=\"api\"}") { ref_id: "B"; editor_mode: builder; }
            transform labels_to_fields only frame_name("series");
            transform sort_by("Time", desc) only frame_name("series");
        }
    };

    let from_builder = Dashboard::new("New keys")
        .schema_version(39)
        .version(3)
        .time(TimeRange::new("now-6h", "now"))
        .extra("weekStart", json!("monday"))
        .panel(
            Table::new("Scoped")
                .query(
                    PrometheusQuery::new("up")
                        .ref_id("A")
                        .editor_mode(QueryEditorMode::Code),
                )
                .query(
                    LokiQuery::new("{job=\"api\"}")
                        .ref_id("B")
                        .editor_mode(QueryEditorMode::Builder),
                )
                .transform(LabelsToFields::new().only_frame_name("series"))
                .transform(SortBy::desc("Time").only_frame_name("series")),
        );

    assert_eq!(from_macro, from_builder);
    assert_eq!(
        from_macro.to_json_pretty().unwrap(),
        from_builder.to_json_pretty().unwrap()
    );
}

/// Compile-time check of the macro's key table: every key, on every panel
/// kind the table says accepts it, must lower to a builder method that
/// exists for that kind. A table entry that claims too much fails to
/// compile here instead of in a user's dashboard.
#[test]
fn every_key_lowers_to_a_builder_method_on_every_kind_that_accepts_it() {
    let ids = std::cell::Cell::new(0);
    let next_id = || {
        ids.set(ids.get() + 1);
        ids.get()
    };
    macro_rules! common {
        ($($kind:ident)*) => {
            dashboard! {
                title: "Common";
                $(
                    $kind "Common" {
                        query: promql!("up") {
                            legend_format: "{{instance}}";
                            ref_id: "A";
                            instant: true;
                            range: false;
                            datasource: prometheus("main");
                            hidden: false;
                            interval: "1m";
                            editor_mode: code;
                            format: table;
                        }
                        query: loki!("{job=\"x\"}") {
                            legend_format: "l";
                            ref_id: "B";
                            instant: false;
                            range: true;
                            datasource: loki("logs");
                            hidden: true;
                            interval: "5m";
                            editor_mode: builder;
                        }
                        query: loki_query!("{job=\"y\"}");
                        legend_format: "{{job}}";
                        description: "d";
                        width: 6;
                        height: 4;
                        id: next_id();
                        grid: [0, 0, 6, 4];
                        datasource: prometheus("main");
                        unit: custom("widgets");
                        min: 0.0;
                        max: 1.0;
                        decimals: 2;
                        display_name: "n";
                        transparent: true;
                        color: continuous("Blues");
                        thresholds { base: green; 1 => red; }
                        mapping "0" => "zero" { color: red; }
                        link "Docs" => "https://docs.example.com" {
                            tooltip: "t";
                            target_blank: true;
                            include_vars: true;
                            keep_time: true;
                            tags: ["a"];
                        }
                        override field("a") {
                            color: "dark-red";
                            line_width: 2;
                            cell: auto;
                            unit: bytes;
                            min: 0.0;
                            max: 1.0;
                            decimals: 1;
                            display_name: "A";
                            thresholds { green: 0.0; }
                        }
                        override regex("^b") { cell: gauge { mode: basic; value_display: color; }; }
                        override query("A") { cell: sparkline { hide_value: true; line_width: 1.0; fill_opacity: 5.0; }; }
                        override type("number") { cell: colored_background { mode: basic; apply_to_row: false; }; }
                        override names ["c", "d"] { cell: colored_text; }
                        override numeric { unit: none; }
                        override time { unit: short; }
                        transform join_by_field("a", outer) only ref_id("A");
                        transform sort_by("a", asc) only frame_name("f");
                        transform organize { rename "a" => "b"; hide "c"; order ["b"]; }
                        transform time_series_to_table { query "A"; query "B": mean; time_field "A": "Time"; }
                        transform labels_to_fields { mode: rows; keep ["x"]; value_label: "v"; }
                        transform: RawTransformation::new("merge");
                        option "o": 1;
                        extra "e": 2;
                    }
                )*
            }
        };
    }
    let common = common!(timeseries stat gauge table text bar_gauge heatmap);

    let specific = dashboard! {
        title: "Kind specific";

        text "Text" {
            content: "c";
            mode: html;
        }
        stat "Stat" {
            color_mode: none;
            graph_mode: area;
            wide_layout: false;
            orientation: auto;
            reduce { values: true; calculations: [last, min, max, mean, total]; fields: "/x/"; }
        }
        gauge "Gauge" {
            orientation: horizontal;
            reduce { values: false; }
        }
        bar_gauge "Bar gauge" {
            display_mode: gradient;
            orientation: vertical;
            reduce { fields: "/y/"; }
        }
        timeseries "Series" {
            fill_opacity: 1.0;
            line_width: 1.0;
            point_size: 1.0;
            span_nulls: false;
            line_interpolation: step_before;
            show_points: always;
            stacking: percent;
            tooltip { mode: single; sort: asc; hide_zeros: false; }
            legend { position: right; mode: list; values: [last]; }
        }
        table "Table" {
            sort_by: ("a", asc);
            cell: sparkline { hide_value: false; };
        }
        heatmap "Heatmap" {
            color_mode: opacity;
            color_scheme: "Greens";
            color_steps: 8;
            cell_gap: 0;
            show_legend: false;
            calculate: true;
            y_axis { unit: milliseconds; placement: right; }
        }
    };

    // Compiling is the main assertion; both must also validate.
    let common: serde_json::Value = serde_json::from_str(&common.to_json().unwrap()).unwrap();
    assert_eq!(common["panels"].as_array().map(Vec::len), Some(7));
    specific.validate().unwrap();
}

/// The old `macro_rules!` implementation hit the compiler's recursion limit
/// at a little over a hundred items in one block. The proc macro has no
/// such limit.
#[test]
fn a_block_with_three_hundred_panels_expands() {
    let dashboard = dashboard! {
        title: "Many panels";
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
        stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {} stat "p" {}
    };

    let json: serde_json::Value = serde_json::from_str(&dashboard.to_json().unwrap()).unwrap();
    assert_eq!(json["panels"].as_array().map(Vec::len), Some(300));
}
