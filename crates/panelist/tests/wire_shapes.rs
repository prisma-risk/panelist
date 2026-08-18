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

//! Exact-shape assertions for the JSON Panelist emits.
//!
//! The other test files assert that a value an author *set* comes out
//! somewhere. This one asserts the whole emitted object, key for key, for
//! every typed option struct — including the keys nobody set, which is the
//! half that was never covered.
//!
//! That gap is not academic. The `byNames` matcher shipped emitting a bare
//! array where Grafana wants `{"names": [...]}`, and the one test that
//! touched it pinned the wrong shape. Grafana accepts, stores, and returns
//! an options value of the wrong shape byte-for-byte unchanged, and only
//! ignores it at render time — so neither the golden test nor the Grafana
//! round-trip verifier can see a mistake here. **The only authority is
//! Grafana's source**, and every shape below cites the file it came from,
//! all at tag `v13.0.2`. Paths are relative to the Grafana repository root.
//!
//! Assert with a whole-object `json!({…})`, never a single-key lookup: a
//! key-by-key assertion cannot see a key that should not be there, and an
//! omission is exactly what a wrong shape looks like from the inside.

use panelist::prelude::*;
use serde_json::{Value, json};

fn as_value(dashboard: &Dashboard) -> Value {
    match serde_json::to_value(dashboard) {
        Ok(value) => value,
        Err(error) => panic!("test dashboard should serialize: {error}"),
    }
}

fn panel_of(dashboard: &Dashboard) -> Value {
    as_value(dashboard)["panels"][0].clone()
}

fn options_of(dashboard: &Dashboard) -> Value {
    panel_of(dashboard)["options"].clone()
}

fn custom_of(dashboard: &Dashboard) -> Value {
    panel_of(dashboard)["fieldConfig"]["defaults"]["custom"].clone()
}

fn first_transformation(dashboard: &Dashboard) -> Value {
    panel_of(dashboard)["transformations"][0].clone()
}

fn matcher_of(field_override: FieldOverride) -> Value {
    let dashboard = Dashboard::new("Matcher").panel(
        Table::new("Panel")
            .query(PrometheusQuery::new("up").ref_id("A"))
            .override_field(field_override),
    );
    panel_of(&dashboard)["fieldConfig"]["overrides"][0]["matcher"].clone()
}

fn cell_value(cell: impl Into<TableCell>) -> Value {
    let dashboard = Dashboard::new("Cell").panel(
        Table::new("Panel")
            .query(PrometheusQuery::new("up"))
            .cell(cell),
    );
    custom_of(&dashboard)["cellOptions"].clone()
}

// ---------------------------------------------------------------------------
// Field matchers
//
// packages/grafana-data/src/transformations/matchers/nameMatcher.ts and
// .../fieldTypeMatcher.ts. Each matcher's `options` shape is the argument
// type of its `FieldMatcherInfo<T>.get`.
// ---------------------------------------------------------------------------

#[test]
fn every_field_matcher_emits_its_grafana_options_shape() {
    // `FieldMatcherInfo<string>`, nameMatcher.ts L37-60. A bare string.
    assert_eq!(
        matcher_of(FieldOverride::by_name("RPS")),
        json!({"id": "byName", "options": "RPS"})
    );

    // `FieldMatcherInfo<string>`, nameMatcher.ts L126-144. A bare pattern
    // string, fed straight to `stringToJsRegex`.
    assert_eq!(
        matcher_of(FieldOverride::by_regex("^p9.*")),
        json!({"id": "byRegexp", "options": "^p9.*"})
    );

    // `FieldMatcherInfo<FieldType>`, fieldTypeMatcher.ts L7-22. `FieldType`
    // is a string enum (types/dataFrame.ts L18-33), so a bare string.
    assert_eq!(
        matcher_of(FieldOverride::by_type("number")),
        json!({"id": "byType", "options": "number"})
    );

    // `FieldMatcherInfo<string>`, nameMatcher.ts L151-166. A bare refId.
    assert_eq!(
        matcher_of(FieldOverride::by_query("A")),
        json!({"id": "byFrameRefID", "options": "A"})
    );

    // `FieldMatcherInfo<ByNamesMatcherOptions>`, nameMatcher.ts L62-101 —
    // an OBJECT, not an array. `get` destructures
    // `const { names, mode = ByNamesMatcherMode.include } = options`, so a
    // bare array yields `names === undefined` and the override matches zero
    // fields, silently. `mode` is omitted because `include` is its default
    // and the only mode this matcher models; the options editor
    // (grafana-ui .../MatchersUI/FieldNamesMatcherEditor.tsx) reads only
    // `names`, `readOnly`, and `prefix`.
    assert_eq!(
        matcher_of(FieldOverride::by_names(["p50", "p95"])),
        json!({"id": "byNames", "options": {"names": ["p50", "p95"]}})
    );

    // `FieldMatcherInfo` with no options generic, fieldTypeMatcher.ts
    // L44-56 and L59-71: both are `get: () => …` and never look at their
    // argument, so the key is omitted rather than sent as `""`.
    assert_eq!(
        matcher_of(FieldOverride::numeric_fields()),
        json!({"id": "numeric"})
    );
    assert_eq!(
        matcher_of(FieldOverride::time_fields()),
        json!({"id": "time"})
    );
}

// ---------------------------------------------------------------------------
// Table cell options
//
// packages/grafana-schema/src/common/common.gen.ts, the `TableCellOptions`
// union (L995) and its members. Renderer reads are in
// packages/grafana-ui/src/components/Table/TableNG/.
// ---------------------------------------------------------------------------

#[test]
fn every_table_cell_renderer_emits_its_grafana_options_shape() {
    // `TableCellDisplayMode` values, common.gen.ts L746-764.
    assert_eq!(cell_value(TableCellType::Auto), json!({"type": "auto"}));
    assert_eq!(
        cell_value(TableCellType::ColoredText),
        json!({"type": "color-text"})
    );

    // `TableBarGaugeCellOptions { mode?, type, valueDisplayMode? }`,
    // common.gen.ts L834-840. `mode` is `BarGaugeDisplayMode`
    // (basic/gradient/lcd, L694-698) and `valueDisplayMode` is
    // `BarGaugeValueMode` (color/hidden/text, L703-707); both are read from
    // the cell options object in Cells/BarGaugeCell.tsx L42-45.
    assert_eq!(
        cell_value(
            GaugeCell::new()
                .mode(BarGaugeDisplayMode::Lcd)
                .value_display(CellValueDisplay::Color)
        ),
        json!({"type": "gauge", "mode": "lcd", "valueDisplayMode": "color"})
    );
    assert_eq!(cell_value(GaugeCell::new()), json!({"type": "gauge"}));

    // `TableSparklineCellOptions extends GraphFieldConfig { hideValue?,
    // type }`, common.gen.ts L843-849. Extending GraphFieldConfig is why
    // `lineWidth` and `fillOpacity` sit directly on the cell object rather
    // than nested: Cells/SparklineCell.tsx L71-84 spreads the whole cell
    // options object into the sparkline config and reads `hideValue` off it.
    assert_eq!(
        cell_value(
            SparklineCell::new()
                .hide_value(true)
                .line_width(2.0)
                .fill_opacity(20.0)
        ),
        json!({"type": "sparkline", "hideValue": true, "lineWidth": 2.0, "fillOpacity": 20.0})
    );
    assert_eq!(
        cell_value(SparklineCell::new()),
        json!({"type": "sparkline"})
    );

    // `TableColoredBackgroundCellOptions { applyToRow?, mode?, type }`,
    // common.gen.ts L851-858. `mode` is `TableCellBackgroundDisplayMode`
    // (basic/gradient, L771-774), read at TableNG/utils.ts L572;
    // `applyToRow` is read at TableNG/utils.ts L1036-1043.
    assert_eq!(
        cell_value(
            ColoredBackgroundCell::new()
                .mode(CellBackgroundMode::Gradient)
                .apply_to_row(true)
        ),
        json!({"type": "color-background", "mode": "gradient", "applyToRow": true})
    );
    assert_eq!(
        cell_value(ColoredBackgroundCell::new()),
        json!({"type": "color-background"})
    );
}

// ---------------------------------------------------------------------------
// Panel options, per kind: the exact object with nothing set, and the exact
// object with every typed setter used.
//
// `options` is assembled in tiers — kind defaults, then typed setters, then
// the raw `.option()` hatch — and `BTreeMap::extend` replaces whole values.
// Nothing but these tests stops a typed default and the kind default it
// shadows from drifting apart.
// ---------------------------------------------------------------------------

fn reduce_options_default() -> Value {
    json!({"values": false, "calcs": ["lastNotNull"], "fields": ""})
}

#[test]
fn stat_panel_options_are_exact_with_and_without_typed_options() {
    let bare = Dashboard::new("Stat").panel(Stat::new("Value").query(PrometheusQuery::new("up")));
    assert_eq!(
        options_of(&bare),
        json!({
            "colorMode": "value",
            "graphMode": "area",
            "justifyMode": "auto",
            "orientation": "auto",
            "reduceOptions": reduce_options_default(),
            "textMode": "auto",
        })
    );

    // `BigValueColorMode` (common.gen.ts L567-572), `BigValueGraphMode`
    // (L577-581), `VizOrientation` (L499-503).
    let full = Dashboard::new("Stat").panel(
        Stat::new("Value")
            .query(PrometheusQuery::new("up"))
            .color_mode(StatColorMode::Background)
            .graph_mode(StatGraphMode::None)
            .orientation(Orientation::Vertical)
            .wide_layout(true)
            .reduce_options(
                ReduceOptions::new()
                    .values(true)
                    .calculations([Reducer::Mean, Reducer::Total])
                    .fields("/^p9.*/"),
            ),
    );
    assert_eq!(
        options_of(&full),
        json!({
            "colorMode": "background",
            "graphMode": "none",
            "justifyMode": "auto",
            "orientation": "vertical",
            "reduceOptions": {"values": true, "calcs": ["mean", "sum"], "fields": "/^p9.*/"},
            "textMode": "auto",
            "wideLayout": true,
        })
    );
}

/// Every `#[default]` variant of a stat option enum mirrors a hardcoded
/// literal in the kind defaults. Setting the default explicitly must
/// reproduce the kind default exactly, or the two have drifted.
#[test]
fn stat_default_enum_variants_reproduce_the_kind_defaults() {
    let bare = Dashboard::new("Stat").panel(Stat::new("Value").query(PrometheusQuery::new("up")));
    let defaults = Dashboard::new("Stat").panel(
        Stat::new("Value")
            .query(PrometheusQuery::new("up"))
            .color_mode(StatColorMode::default())
            .graph_mode(StatGraphMode::default())
            .orientation(Orientation::default())
            .reduce_options(ReduceOptions::default()),
    );

    assert_eq!(options_of(&bare), options_of(&defaults));
}

#[test]
fn gauge_panel_options_are_exact_with_and_without_typed_options() {
    let bare = Dashboard::new("Gauge").panel(Gauge::new("Value").query(PrometheusQuery::new("up")));
    assert_eq!(
        options_of(&bare),
        json!({
            "orientation": "auto",
            "reduceOptions": reduce_options_default(),
            "showThresholdLabels": false,
            "showThresholdMarkers": true,
        })
    );

    let full = Dashboard::new("Gauge").panel(
        Gauge::new("Value")
            .query(PrometheusQuery::new("up"))
            .orientation(Orientation::Horizontal)
            .reduce_options(ReduceOptions::new().calculations([Reducer::Max])),
    );
    assert_eq!(
        options_of(&full),
        json!({
            "orientation": "horizontal",
            "reduceOptions": {"values": false, "calcs": ["max"], "fields": ""},
            "showThresholdLabels": false,
            "showThresholdMarkers": true,
        })
    );
}

#[test]
fn gauge_default_enum_variants_reproduce_the_kind_defaults() {
    let bare = Dashboard::new("Gauge").panel(Gauge::new("Value").query(PrometheusQuery::new("up")));
    let defaults = Dashboard::new("Gauge").panel(
        Gauge::new("Value")
            .query(PrometheusQuery::new("up"))
            .orientation(Orientation::default())
            .reduce_options(ReduceOptions::default()),
    );

    assert_eq!(options_of(&bare), options_of(&defaults));
}

#[test]
fn bar_gauge_panel_options_are_exact_with_and_without_typed_options() {
    let bare =
        Dashboard::new("Bars").panel(BarGauge::new("Value").query(PrometheusQuery::new("up")));
    assert_eq!(
        options_of(&bare),
        json!({
            "displayMode": "gradient",
            "orientation": "horizontal",
            "reduceOptions": reduce_options_default(),
            "showUnfilled": true,
        })
    );

    let full = Dashboard::new("Bars").panel(
        BarGauge::new("Value")
            .query(PrometheusQuery::new("up"))
            .display_mode(BarGaugeDisplayMode::Lcd)
            .orientation(Orientation::Vertical)
            .reduce_options(ReduceOptions::new().values(true)),
    );
    assert_eq!(
        options_of(&full),
        json!({
            "displayMode": "lcd",
            "orientation": "vertical",
            "reduceOptions": {"values": true, "calcs": ["lastNotNull"], "fields": ""},
            "showUnfilled": true,
        })
    );
}

/// The bar gauge's kind default `"gradient"` and `BarGaugeDisplayMode`'s
/// `#[default]` variant are independent literals. Its kind default
/// orientation is `"horizontal"`, which is deliberately NOT
/// `Orientation::default()` — so only `display_mode` is compared here.
#[test]
fn bar_gauge_default_display_mode_reproduces_the_kind_default() {
    let bare =
        Dashboard::new("Bars").panel(BarGauge::new("Value").query(PrometheusQuery::new("up")));
    let defaults = Dashboard::new("Bars").panel(
        BarGauge::new("Value")
            .query(PrometheusQuery::new("up"))
            .display_mode(BarGaugeDisplayMode::default())
            .reduce_options(ReduceOptions::default()),
    );

    assert_eq!(options_of(&bare), options_of(&defaults));
    assert_ne!(
        options_of(&bare)["orientation"],
        json!("auto"),
        "the bar gauge kind default deliberately differs from Orientation::default()"
    );
}

#[test]
fn timeseries_panel_options_and_custom_are_exact_with_and_without_typed_options() {
    let bare =
        Dashboard::new("Traffic").panel(Timeseries::new("Rate").query(PrometheusQuery::new("up")));
    assert_eq!(
        options_of(&bare),
        json!({
            "legend": {
                "showLegend": true,
                "displayMode": "list",
                "placement": "bottom",
                "calcs": [],
            },
            "tooltip": {"mode": "single", "sort": "none", "hideZeros": false},
        })
    );
    assert_eq!(
        custom_of(&bare),
        json!({
            "drawStyle": "line",
            "fillOpacity": 0,
            "lineInterpolation": "linear",
            "lineWidth": 1,
            "showPoints": "auto",
            "spanNulls": false,
        })
    );

    // `LegendDisplayMode` (common.gen.ts L456-460), `TooltipDisplayMode`
    // (L643-647), `SortOrder` (L652-656), `LineInterpolation` (L252-257),
    // `VisibilityMode` for showPoints (L226-230), `StackingMode` (L282-286).
    let full = Dashboard::new("Traffic").panel(
        Timeseries::new("Rate")
            .query(PrometheusQuery::new("up"))
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
                    .sort(TooltipSort::Descending)
                    .hide_zeros(true),
            )
            .legend_options(
                Legend::new()
                    .mode(LegendMode::Table)
                    .placement(LegendPlacement::Right)
                    .calculations([Reducer::Last, Reducer::Max]),
            ),
    );
    assert_eq!(
        options_of(&full),
        json!({
            "legend": {
                "showLegend": true,
                "displayMode": "table",
                "placement": "right",
                "calcs": ["lastNotNull", "max"],
            },
            "tooltip": {"mode": "multi", "sort": "desc", "hideZeros": true},
        })
    );
    assert_eq!(
        custom_of(&full),
        json!({
            "drawStyle": "line",
            "fillOpacity": 10.0,
            "lineInterpolation": "stepAfter",
            "lineWidth": 2.0,
            "pointSize": 3.0,
            "showPoints": "never",
            "spanNulls": true,
            "stacking": {"mode": "normal", "group": "requests"},
        })
    );
}

#[test]
fn timeseries_default_typed_options_reproduce_the_kind_defaults() {
    let bare =
        Dashboard::new("Traffic").panel(Timeseries::new("Rate").query(PrometheusQuery::new("up")));
    let defaults = Dashboard::new("Traffic").panel(
        Timeseries::new("Rate")
            .query(PrometheusQuery::new("up"))
            .tooltip(Tooltip::default())
            .legend_options(Legend::default())
            .line_interpolation(LineInterpolation::default())
            .show_points(PointVisibility::default()),
    );

    assert_eq!(options_of(&bare), options_of(&defaults));
    assert_eq!(custom_of(&bare), custom_of(&defaults));
}

/// `LegendMode::Hidden` is the one legend mode that is not a `displayMode`
/// value of its own: Grafana's `LegendDisplayMode` has a `Hidden` variant
/// (common.gen.ts L456-460), but the time-series panel hides its legend
/// with `showLegend: false` and leaves `displayMode` alone.
#[test]
fn a_hidden_legend_lowers_to_show_legend_false() {
    let dashboard = Dashboard::new("Traffic").panel(
        Timeseries::new("Rate")
            .query(PrometheusQuery::new("up"))
            .legend_options(Legend::new().mode(LegendMode::Hidden)),
    );

    assert_eq!(
        options_of(&dashboard)["legend"],
        json!({
            "showLegend": false,
            "displayMode": "list",
            "placement": "bottom",
            "calcs": [],
        })
    );
}

#[test]
fn table_panel_options_are_exact_and_omit_an_empty_sort() {
    let bare = Dashboard::new("Routes").panel(Table::new("R").query(PrometheusQuery::new("up")));
    // No `sortBy` key at all: an empty authored sort must not emit an empty
    // array, which Grafana would read as an explicit "sort by nothing".
    assert_eq!(
        options_of(&bare),
        json!({"cellHeight": "sm", "showHeader": true})
    );
    assert_eq!(custom_of(&bare), json!({"align": "auto", "inspect": false}));

    // `TableSortByFieldState` (common.gen.ts L779), keyed on displayName.
    let sorted = Dashboard::new("Routes").panel(
        Table::new("R")
            .query(PrometheusQuery::new("up"))
            .sort_by("p95", SortDirection::Descending)
            .sort_by("route", SortDirection::Ascending),
    );
    assert_eq!(
        options_of(&sorted),
        json!({
            "cellHeight": "sm",
            "showHeader": true,
            "sortBy": [
                {"displayName": "p95", "desc": true},
                {"displayName": "route", "desc": false},
            ],
        })
    );
}

#[test]
fn heatmap_panel_options_are_exact_with_and_without_typed_options() {
    let bare = Dashboard::new("Latency")
        .panel(Heatmap::new("Distribution").query(PrometheusQuery::new("up")));
    assert_eq!(
        options_of(&bare),
        json!({
            "calculate": false,
            "cellGap": 1,
            "color": {"mode": "scheme", "scheme": "Oranges", "steps": 64},
            "legend": {"show": true},
            "yAxis": {"axisPlacement": "left"},
        })
    );

    // `AxisPlacement`, common.gen.ts L206-213.
    let full = Dashboard::new("Latency").panel(
        Heatmap::new("Distribution")
            .query(PrometheusQuery::new("up"))
            .color_scheme(HeatmapColorScheme::Blues)
            .color_steps(32)
            .color_mode(HeatmapColorMode::Opacity)
            .cell_gap(2)
            .show_legend(false)
            .y_axis_unit(Unit::Seconds)
            .y_axis_placement(AxisPlacement::Right)
            .calculate(true),
    );
    assert_eq!(
        options_of(&full),
        json!({
            "calculate": true,
            "cellGap": 2,
            "color": {"mode": "opacity", "scheme": "Blues", "steps": 32},
            "legend": {"show": false},
            "yAxis": {"axisPlacement": "right", "unit": "s"},
        })
    );
}

/// `color` and `yAxis` are whole objects in both the kind-default tier and
/// the typed tier, and `BTreeMap::extend` replaces a whole value rather
/// than merging keys. Setting one field of either must therefore rebuild
/// the rest from the same defaults, not drop them.
#[test]
fn partial_heatmap_options_keep_the_rest_of_the_kind_defaults() {
    let partial_axis = Dashboard::new("Latency").panel(
        Heatmap::new("Distribution")
            .query(PrometheusQuery::new("up"))
            .y_axis_unit(Unit::Seconds),
    );
    assert_eq!(
        options_of(&partial_axis)["yAxis"],
        json!({"axisPlacement": "left", "unit": "s"})
    );

    let partial_placement = Dashboard::new("Latency").panel(
        Heatmap::new("Distribution")
            .query(PrometheusQuery::new("up"))
            .y_axis_placement(AxisPlacement::Top),
    );
    assert_eq!(
        options_of(&partial_placement)["yAxis"],
        json!({"axisPlacement": "top"})
    );

    let partial_color = Dashboard::new("Latency").panel(
        Heatmap::new("Distribution")
            .query(PrometheusQuery::new("up"))
            .color_steps(16),
    );
    assert_eq!(
        options_of(&partial_color)["color"],
        json!({"mode": "scheme", "scheme": "Oranges", "steps": 16})
    );
}

#[test]
fn text_panel_options_are_exact_with_and_without_content() {
    let bare = Dashboard::new("Docs").panel(Text::new("Runbook"));
    assert_eq!(
        options_of(&bare),
        json!({"content": "", "mode": "markdown"})
    );

    let full = Dashboard::new("Docs").panel(
        Text::new("Runbook")
            .content("<b>hi</b>")
            .mode(TextMode::Html),
    );
    assert_eq!(
        options_of(&full),
        json!({"content": "<b>hi</b>", "mode": "html"})
    );
}

// ---------------------------------------------------------------------------
// Transformations
//
// packages/grafana-data/src/transformations/transformers/.
// ---------------------------------------------------------------------------

#[test]
fn join_by_field_emits_its_grafana_options_shape() {
    for (mode, wire) in [
        (JoinMode::Outer, "outer"),
        (JoinMode::Inner, "inner"),
        (JoinMode::OuterTabular, "outerTabular"),
    ] {
        let dashboard = Dashboard::new("Join").panel(
            Table::new("R")
                .query(PrometheusQuery::new("up"))
                .transform(JoinByField::new("route").mode(mode)),
        );
        assert_eq!(
            first_transformation(&dashboard),
            json!({"id": "joinByField", "options": {"byField": "route", "mode": wire}})
        );
    }
}

/// `organize`'s three option maps, and specifically which name space each
/// one is keyed in. `organize.ts` L35-46 pipes filter, then order, then
/// rename, so:
///
/// - `excludeByName` keys are pre-rename names — the filter runs first;
/// - `indexByName` keys are pre-rename names too — `order.ts` L73-80
///   compares display names as they stand before `rename.ts` L49-52
///   assigns the new ones;
/// - `renameByName` maps pre-rename name to post-rename name.
///
/// Panelist's `order [...]` takes the FINAL names an author sees and
/// translates. `route` is never renamed and passes through untouched;
/// `RPS` becomes `Value #A`.
#[test]
fn organize_emits_index_by_name_keyed_on_pre_rename_names() {
    let dashboard = Dashboard::new("Organize").panel(
        Table::new("R").query(PrometheusQuery::new("up")).transform(
            OrganizeFields::new()
                .rename("Value #A", "RPS")
                .hide("Time")
                .order(["route", "RPS"]),
        ),
    );

    assert_eq!(
        first_transformation(&dashboard),
        json!({
            "id": "organize",
            "options": {
                "excludeByName": {"Time": true},
                "indexByName": {"route": 0, "Value #A": 1},
                "renameByName": {"Value #A": "RPS"},
            }
        })
    );
}

/// Every map is emitted even when empty, matching `organize.ts`'s own
/// `defaultOptions` (L20-25).
#[test]
fn an_empty_organize_still_emits_all_three_maps() {
    let dashboard = Dashboard::new("Organize").panel(
        Table::new("R")
            .query(PrometheusQuery::new("up"))
            .transform(OrganizeFields::new()),
    );

    assert_eq!(
        first_transformation(&dashboard),
        json!({
            "id": "organize",
            "options": {"excludeByName": {}, "indexByName": {}, "renameByName": {}}
        })
    );
}

/// `sortBy.ts` L16-20 carries an array but says "only the first entry is
/// used", and L45-53 reads `s[0]` alone. Panelist therefore has no
/// secondary-sort API, and this pins that exactly one entry is emitted —
/// an emitted second entry would be JSON Grafana stores and ignores.
#[test]
fn sort_by_emits_exactly_one_sort_entry() {
    let dashboard = Dashboard::new("Sort").panel(
        Table::new("R")
            .query(PrometheusQuery::new("up"))
            .transform(SortBy::desc("p95")),
    );

    assert_eq!(
        first_transformation(&dashboard),
        json!({"id": "sortBy", "options": {"sort": [{"field": "p95", "desc": true}]}})
    );
    assert_eq!(
        first_transformation(&dashboard)["options"]["sort"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
}

#[test]
fn time_series_to_table_is_keyed_by_ref_id_and_omits_unset_entries() {
    let dashboard = Dashboard::new("Trend").panel(
        Table::new("R")
            .query(PrometheusQuery::new("up").ref_id("A"))
            .query(PrometheusQuery::new("up").ref_id("B"))
            .query(PrometheusQuery::new("up").ref_id("C"))
            .transform(
                TimeSeriesToTable::new()
                    .query("A")
                    .query_with("B", Reducer::Mean)
                    .time_field("C", "Time"),
            ),
    );

    assert_eq!(
        first_transformation(&dashboard),
        json!({
            "id": "timeSeriesTable",
            "options": {"A": {}, "B": {"stat": "mean"}, "C": {"timeField": "Time"}}
        })
    );
}

#[test]
fn labels_to_fields_omits_the_options_it_was_not_given() {
    let bare = Dashboard::new("Labels").panel(
        Table::new("R")
            .query(PrometheusQuery::new("up"))
            .transform(LabelsToFields::new()),
    );
    assert_eq!(
        first_transformation(&bare),
        json!({"id": "labelsToFields", "options": {"mode": "columns"}})
    );

    let full = Dashboard::new("Labels").panel(
        Table::new("R").query(PrometheusQuery::new("up")).transform(
            LabelsToFields::new()
                .mode(LabelsToFieldsMode::Rows)
                .keep(["route", "status"])
                .value_label("value"),
        ),
    );
    assert_eq!(
        first_transformation(&full),
        json!({
            "id": "labelsToFields",
            "options": {
                "mode": "rows",
                "keepLabels": ["route", "status"],
                "valueLabel": "value",
            }
        })
    );
}

/// The envelope keys, and the fact that both are omitted when unset — a
/// `"disabled": false` would be indistinguishable from an author having
/// deliberately disabled nothing.
#[test]
fn the_transformation_envelope_is_omitted_unless_set() {
    let bare = Dashboard::new("Envelope").panel(
        Table::new("R")
            .query(PrometheusQuery::new("up"))
            .transform(SortBy::asc("p95")),
    );
    assert_eq!(
        first_transformation(&bare),
        json!({"id": "sortBy", "options": {"sort": [{"field": "p95", "desc": false}]}})
    );

    // Frame matchers, not field matchers: "which query" is `byRefId` here
    // and `byFrameRefID` in a field override.
    // packages/grafana-data/src/transformations/matchers/refIdMatcher.ts
    // L8-41 takes a bare refId string.
    for (filtered, expected) in [
        (
            SortBy::asc("p95").only_ref_id("A"),
            json!({"id": "byRefId", "options": "A"}),
        ),
        (
            SortBy::asc("p95").only_frame_name("latency"),
            json!({"id": "byName", "options": "latency"}),
        ),
        // `byName` is a regex anchored as `^…$` by `stringToJsRegex`
        // (nameMatcher.ts), so an exact-name API has to escape. Without
        // this, `p95.total` would also match `p95Xtotal`, and a name
        // starting with `/` would be parsed as `/pattern/flags`.
        (
            SortBy::asc("p95").only_frame_name("p95.total"),
            json!({"id": "byName", "options": r"p95\.total"}),
        ),
        (
            SortBy::asc("p95").only_frame_name("/api/v1"),
            json!({"id": "byName", "options": r"\/api\/v1"}),
        ),
    ] {
        let dashboard = Dashboard::new("Envelope").panel(
            Table::new("R")
                .query(PrometheusQuery::new("up").ref_id("A"))
                .transform(filtered),
        );
        assert_eq!(first_transformation(&dashboard)["filter"], expected);
    }

    let disabled = Dashboard::new("Envelope").panel(
        Table::new("R")
            .query(PrometheusQuery::new("up"))
            .transform(SortBy::asc("p95").disabled(true)),
    );
    assert_eq!(first_transformation(&disabled)["disabled"], json!(true));
}

// ---------------------------------------------------------------------------
// Field defaults
// ---------------------------------------------------------------------------

/// Grafana's base threshold step is the one with a `null` value, and it has
/// to be there: without it the first real step has nothing below it. An
/// authored set that already starts with a base step must not get a second
/// one prepended.
#[test]
fn thresholds_always_carry_exactly_one_base_step() {
    let implicit = Dashboard::new("Thresholds").panel(
        Stat::new("Value")
            .query(PrometheusQuery::new("up"))
            .thresholds(Thresholds::new().green(0.0).red(5.0)),
    );
    assert_eq!(
        panel_of(&implicit)["fieldConfig"]["defaults"]["thresholds"],
        json!({
            "mode": "absolute",
            "steps": [
                {"color": "green", "value": null},
                {"color": "green", "value": 0.0},
                {"color": "red", "value": 5.0},
            ],
        })
    );

    let explicit = Dashboard::new("Thresholds").panel(
        Stat::new("Value")
            .query(PrometheusQuery::new("up"))
            .thresholds(
                Thresholds::new()
                    .step(Color::Red, None)
                    .green(1.0)
                    .mode(ThresholdMode::Percentage),
            ),
    );
    assert_eq!(
        panel_of(&explicit)["fieldConfig"]["defaults"]["thresholds"],
        json!({
            "mode": "percentage",
            "steps": [
                {"color": "red", "value": null},
                {"color": "green", "value": 1.0},
            ],
        })
    );
}

#[test]
fn every_color_scheme_emits_its_grafana_shape() {
    for (scheme, expected) in [
        (ColorScheme::Thresholds, json!({"mode": "thresholds"})),
        (
            ColorScheme::ClassicPalette,
            json!({"mode": "palette-classic"}),
        ),
        (
            ColorScheme::Fixed(Color::Purple),
            json!({"mode": "fixed", "fixedColor": "purple"}),
        ),
        (
            ColorScheme::Continuous("continuous-GrYlRd".to_owned()),
            json!({"mode": "continuous-GrYlRd"}),
        ),
    ] {
        let dashboard = Dashboard::new("Colors").panel(
            Stat::new("Value")
                .query(PrometheusQuery::new("up"))
                .color(scheme),
        );
        assert_eq!(
            panel_of(&dashboard)["fieldConfig"]["defaults"]["color"],
            expected
        );
    }
}

/// A field config with nothing set emits only the kind's own `custom`
/// block — no empty `unit`, no null `min`, no `mappings: []`.
#[test]
fn unset_field_defaults_are_omitted_entirely() {
    let dashboard =
        Dashboard::new("Bare").panel(Stat::new("Value").query(PrometheusQuery::new("up")));

    assert_eq!(
        panel_of(&dashboard)["fieldConfig"],
        json!({"defaults": {}, "overrides": []})
    );
}

#[test]
fn every_override_property_emits_its_grafana_field_property_id() {
    let dashboard = Dashboard::new("Properties").panel(
        Table::new("R")
            .query(PrometheusQuery::new("up"))
            .override_field(
                FieldOverride::by_name("p95")
                    .unit(Unit::Seconds)
                    .min(0.0)
                    .max(1.0)
                    .decimals(3)
                    .display_name("p95 latency")
                    .color(Color::Red)
                    .line_width(2)
                    .thresholds(Thresholds::new().green(0.0))
                    .cell(GaugeCell::new().mode(BarGaugeDisplayMode::Basic)),
            ),
    );

    assert_eq!(
        panel_of(&dashboard)["fieldConfig"]["overrides"][0]["properties"],
        json!([
            {"id": "unit", "value": "s"},
            {"id": "min", "value": 0.0},
            {"id": "max", "value": 1.0},
            {"id": "decimals", "value": 3},
            {"id": "displayName", "value": "p95 latency"},
            {"id": "color", "value": {"mode": "fixed", "fixedColor": "red"}},
            {"id": "custom.lineWidth", "value": 2},
            {
                "id": "thresholds",
                "value": {
                    "mode": "absolute",
                    "steps": [
                        {"color": "green", "value": null},
                        {"color": "green", "value": 0.0},
                    ],
                }
            },
            {"id": "custom.cellOptions", "value": {"type": "gauge", "mode": "basic"}},
        ])
    );
}

// ---------------------------------------------------------------------------
// Query targets
// ---------------------------------------------------------------------------

#[test]
fn a_bare_query_target_omits_everything_it_was_not_given() {
    let dashboard =
        Dashboard::new("Targets").panel(Timeseries::new("Rate").query(PrometheusQuery::new("up")));

    assert_eq!(
        panel_of(&dashboard)["targets"][0],
        json!({"refId": "A", "expr": "up", "hide": false})
    );
}

#[test]
fn a_fully_specified_query_target_is_exact() {
    let dashboard = Dashboard::new("Targets").panel(
        Timeseries::new("Rate").query(
            PrometheusQuery::new("up")
                .ref_id("Q")
                .legend_format("{{status}}")
                .format(PrometheusFormat::Table)
                .editor_mode(QueryEditorMode::Code)
                .instant(true)
                .hidden(true)
                .interval("30s"),
        ),
    );

    assert_eq!(
        panel_of(&dashboard)["targets"][0],
        json!({
            "refId": "Q",
            "expr": "up",
            "format": "table",
            "editorMode": "code",
            "legendFormat": "{{status}}",
            "instant": true,
            "range": false,
            "hide": true,
            "interval": "30s",
        })
    );
}
