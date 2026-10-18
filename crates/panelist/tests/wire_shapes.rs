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

/// Grafana's base threshold step is the one with a `null` value, and it has to be there: without it the first real step has nothing below it. It is always the authored base color, emitted exactly once, followed by each step in order.
#[test]
fn thresholds_emit_the_base_color_as_the_one_null_step() {
    let absolute = Dashboard::new("Thresholds").panel(
        Stat::new("Value")
            .query(PrometheusQuery::new("up"))
            .thresholds(Thresholds::absolute(Color::Green).step(5.0, Color::Red)),
    );
    assert_eq!(
        panel_of(&absolute)["fieldConfig"]["defaults"]["thresholds"],
        json!({
            "mode": "absolute",
            "steps": [
                {"color": "green", "value": null},
                {"color": "red", "value": 5.0},
            ],
        })
    );

    let percentage = Dashboard::new("Thresholds").panel(
        Stat::new("Value")
            .query(PrometheusQuery::new("up"))
            .thresholds(Thresholds::percentage(Color::Red).step(1.0, Color::Green)),
    );
    assert_eq!(
        panel_of(&percentage)["fieldConfig"]["defaults"]["thresholds"],
        json!({
            "mode": "percentage",
            "steps": [
                {"color": "red", "value": null},
                {"color": "green", "value": 1.0},
            ],
        })
    );

    let base_only = Dashboard::new("Thresholds").panel(
        Stat::new("Value")
            .query(PrometheusQuery::new("up"))
            .thresholds(Thresholds::new(
                ThresholdMode::Absolute,
                Color::custom("#7EB26D"),
            )),
    );
    assert_eq!(
        panel_of(&base_only)["fieldConfig"]["defaults"]["thresholds"],
        json!({
            "mode": "absolute",
            "steps": [{"color": "#7EB26D", "value": null}],
        })
    );
}

/// Every mode is a `FieldColorModeId` registered in `packages/grafana-data/src/field/fieldColor.ts`. Only `fixed` and `shades` read `fixedColor` (`getFixedColor`, `getShadedColor`).
#[test]
fn every_color_scheme_emits_its_grafana_shape() {
    for (scheme, expected) in [
        (ColorScheme::Thresholds, json!({"mode": "thresholds"})),
        (
            ColorScheme::ClassicPalette,
            json!({"mode": "palette-classic"}),
        ),
        (
            ColorScheme::ClassicPaletteByName,
            json!({"mode": "palette-classic-by-name"}),
        ),
        (
            ColorScheme::Fixed(Color::Purple),
            json!({"mode": "fixed", "fixedColor": "purple"}),
        ),
        (
            ColorScheme::Shades(Color::Blue),
            json!({"mode": "shades", "fixedColor": "blue"}),
        ),
        (
            ColorScheme::Continuous(ContinuousPalette::GreenYellowRed),
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

/// `FieldColorModeId` in `packages/grafana-data/src/types/fieldColor.ts`. Case matters: the two-hue palettes are camel-cased, the single-hue ones lower-cased.
#[test]
fn every_continuous_palette_emits_its_grafana_mode() {
    for (palette, mode) in [
        (ContinuousPalette::GreenYellowRed, "continuous-GrYlRd"),
        (ContinuousPalette::RedYellowGreen, "continuous-RdYlGr"),
        (ContinuousPalette::BlueYellowRed, "continuous-BlYlRd"),
        (ContinuousPalette::YellowRed, "continuous-YlRd"),
        (ContinuousPalette::BluePurple, "continuous-BlPu"),
        (ContinuousPalette::YellowBlue, "continuous-YlBl"),
        (ContinuousPalette::Blues, "continuous-blues"),
        (ContinuousPalette::Reds, "continuous-reds"),
        (ContinuousPalette::Greens, "continuous-greens"),
        (ContinuousPalette::Purples, "continuous-purples"),
        (ContinuousPalette::Viridis, "continuous-viridis"),
        (ContinuousPalette::Magma, "continuous-magma"),
        (ContinuousPalette::Plasma, "continuous-plasma"),
        (ContinuousPalette::Inferno, "continuous-inferno"),
        (ContinuousPalette::Cividis, "continuous-cividis"),
    ] {
        let dashboard = Dashboard::new("Colors").panel(
            Gauge::new("Value")
                .query(PrometheusQuery::new("up"))
                .color(ColorScheme::Continuous(palette)),
        );
        assert_eq!(
            panel_of(&dashboard)["fieldConfig"]["defaults"]["color"],
            json!({"mode": mode}),
            "{palette:?}"
        );
    }
}

/// Unit IDs from `packages/grafana-data/src/valueFormats/categories.ts`. Grafana renders an unknown ID as a literal suffix, so a wrong one shows up as text such as `5 hrs` rather than as an error.
#[test]
fn every_typed_unit_emits_its_grafana_unit_id() {
    for (unit, id) in [
        (Unit::None, "none"),
        (Unit::Nanoseconds, "ns"),
        (Unit::Microseconds, "\u{b5}s"),
        (Unit::Milliseconds, "ms"),
        (Unit::Seconds, "s"),
        (Unit::Minutes, "m"),
        (Unit::Hours, "h"),
        (Unit::Days, "d"),
        (Unit::DurationSeconds, "dtdurations"),
        (Unit::DurationMilliseconds, "dtdurationms"),
        (Unit::Bytes, "bytes"),
        (Unit::BytesPerSecond, "Bps"),
        (Unit::BitsPerSecond, "bps"),
        (Unit::PacketsPerSecond, "pps"),
        (Unit::Percent, "percent"),
        (Unit::PercentUnit, "percentunit"),
        (Unit::RequestsPerSecond, "reqps"),
        (Unit::OperationsPerSecond, "ops"),
        (Unit::ReadsPerSecond, "rps"),
        (Unit::WritesPerSecond, "wps"),
        (Unit::IoOperationsPerSecond, "iops"),
        (Unit::EventsPerSecond, "eps"),
        (Unit::Short, "short"),
        (Unit::Currency(Currency::Usd), "currencyUSD"),
        (Unit::custom("widgets"), "widgets"),
    ] {
        let dashboard = Dashboard::new("Units").panel(
            Stat::new("Value")
                .query(PrometheusQuery::new("up"))
                .unit(unit.clone()),
        );
        assert_eq!(
            panel_of(&dashboard)["fieldConfig"]["defaults"]["unit"],
            json!(id),
            "{unit:?}"
        );
    }
}

/// The whole currency category of `categories.ts`, in its own order. The micro-bitcoin ID uses U+03BC GREEK SMALL LETTER MU, unlike the U+00B5 MICRO SIGN in the `µs` time unit.
#[test]
fn every_currency_emits_its_grafana_unit_id() {
    for (currency, id) in [
        (Currency::Usd, "currencyUSD"),
        (Currency::Gbp, "currencyGBP"),
        (Currency::Eur, "currencyEUR"),
        (Currency::Jpy, "currencyJPY"),
        (Currency::Rub, "currencyRUB"),
        (Currency::Uah, "currencyUAH"),
        (Currency::Brl, "currencyBRL"),
        (Currency::Dkk, "currencyDKK"),
        (Currency::Isk, "currencyISK"),
        (Currency::Nok, "currencyNOK"),
        (Currency::Sek, "currencySEK"),
        (Currency::Czk, "currencyCZK"),
        (Currency::Chf, "currencyCHF"),
        (Currency::Pln, "currencyPLN"),
        (Currency::Btc, "currencyBTC"),
        (Currency::MilliBtc, "currencymBTC"),
        (Currency::MicroBtc, "currency\u{3bc}BTC"),
        (Currency::Zar, "currencyZAR"),
        (Currency::Inr, "currencyINR"),
        (Currency::Krw, "currencyKRW"),
        (Currency::Idr, "currencyIDR"),
        (Currency::Php, "currencyPHP"),
        (Currency::Vnd, "currencyVND"),
        (Currency::Try, "currencyTRY"),
        (Currency::Myr, "currencyMYR"),
        (Currency::Xpf, "currencyXPF"),
        (Currency::Bgn, "currencyBGN"),
        (Currency::Pyg, "currencyPYG"),
        (Currency::Uyu, "currencyUYU"),
        (Currency::Ils, "currencyILS"),
    ] {
        let dashboard = Dashboard::new("Currencies").panel(
            Stat::new("Value")
                .query(PrometheusQuery::new("revenue"))
                .unit(currency.into()),
        );
        assert_eq!(
            panel_of(&dashboard)["fieldConfig"]["defaults"]["unit"],
            json!(id),
            "{currency:?}"
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
                    .thresholds(Thresholds::absolute(Color::Green))
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
                    "steps": [{"color": "green", "value": null}],
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
                .instant()
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

// ---------------------------------------------------------------------------
// Value mappings
//
// packages/grafana-data/src/types/valueMapping.ts for each `options` shape, and `editModelToSaveModel` in public/app/features/dimensions/editors/ValueMappingsEditor/ValueMappingsEditorModal.tsx for the ordering: one leading `value` entry holding every exact value, the other kinds in authored order, and each result stamped with its row index.
// ---------------------------------------------------------------------------

fn mappings_of(panel: impl Into<Panel>) -> Value {
    let dashboard = Dashboard::new("Mappings").panel(panel);
    panel_of(&dashboard)["fieldConfig"]["defaults"]["mappings"].clone()
}

#[test]
fn every_value_mapping_kind_emits_its_grafana_options_shape() {
    let mappings = mappings_of(
        Stat::new("Status")
            .query(PrometheusQuery::new("up"))
            .mapping(ValueMapping::range(None, 0.5).text("Low").color(Color::Red))
            .mapping(ValueMapping::value("1").text("Up").color(Color::Green))
            .mapping(ValueMapping::regex("^prod-(.*)$").text("$1"))
            .mapping(ValueMapping::special(SpecialValue::Null).text("No data"))
            .mapping(ValueMapping::value("0").text("Down"))
            .mapping(ValueMapping::range(0.5, 2.0).color(Color::Yellow)),
    );

    assert_eq!(
        mappings,
        json!([
            {
                "type": "value",
                "options": {
                    "0": {"text": "Down", "index": 4},
                    "1": {"text": "Up", "color": "green", "index": 1},
                },
            },
            {
                "type": "range",
                "options": {"from": null, "to": 0.5, "result": {"text": "Low", "color": "red", "index": 0}},
            },
            {
                "type": "regex",
                "options": {"pattern": "^prod-(.*)$", "result": {"text": "$1", "index": 2}},
            },
            {
                "type": "special",
                "options": {"match": "null", "result": {"text": "No data", "index": 3}},
            },
            {
                "type": "range",
                "options": {"from": 0.5, "to": 2.0, "result": {"color": "yellow", "index": 5}},
            },
        ])
    );
}

#[test]
fn every_special_value_emits_its_grafana_match_name() {
    for (special, name) in [
        (SpecialValue::Null, "null"),
        (SpecialValue::NaN, "nan"),
        (SpecialValue::NullOrNaN, "null+nan"),
        (SpecialValue::True, "true"),
        (SpecialValue::False, "false"),
        (SpecialValue::Empty, "empty"),
    ] {
        let mappings = mappings_of(
            Stat::new("Status")
                .query(PrometheusQuery::new("up"))
                .mapping(ValueMapping::special(special).text("x")),
        );
        assert_eq!(
            mappings,
            json!([{"type": "special", "options": {"match": name, "result": {"text": "x", "index": 0}}}])
        );
    }
}

#[test]
fn blank_mapping_text_is_omitted_like_grafanas_editor_does() {
    let mappings = mappings_of(
        Stat::new("Status")
            .query(PrometheusQuery::new("up"))
            .mapping(ValueMapping::value("1").text("  ").color(Color::Green)),
    );

    assert_eq!(
        mappings,
        json!([{"type": "value", "options": {"1": {"color": "green", "index": 0}}}])
    );
}

#[test]
fn override_mappings_and_links_collect_into_one_property_each() {
    let dashboard = Dashboard::new("Overrides").panel(
        Table::new("Routes")
            .query(PrometheusQuery::new("up"))
            .override_field(
                FieldOverride::by_name("status")
                    .mapping(ValueMapping::value("0").text("Down"))
                    .data_link(DataLink::new("Runbook", "https://runbook.example/status"))
                    .mapping(ValueMapping::special(SpecialValue::Null).text("Unknown"))
                    .data_link(DataLink::new("Logs", "/explore").target_blank(true)),
            ),
    );

    assert_eq!(
        panel_of(&dashboard)["fieldConfig"]["overrides"][0]["properties"],
        json!([
            {
                "id": "mappings",
                "value": [
                    {"type": "value", "options": {"0": {"text": "Down", "index": 0}}},
                    {"type": "special", "options": {"match": "null", "result": {"text": "Unknown", "index": 1}}},
                ],
            },
            {
                "id": "links",
                "value": [
                    {"title": "Runbook", "url": "https://runbook.example/status"},
                    {"title": "Logs", "url": "/explore", "targetBlank": true},
                ],
            },
        ])
    );
}

// ---------------------------------------------------------------------------
// Data links
//
// packages/grafana-data/src/types/dataLink.ts for the stored shape, and `getDataLinkUIModel` in public/app/features/panel/panellinks/link_srv.ts, which renders panel header links and field links alike from `title`, `url`, and `targetBlank` only.
// ---------------------------------------------------------------------------

#[test]
fn panel_and_field_links_emit_the_data_link_shape() {
    let dashboard = Dashboard::new("Links").panel(
        Timeseries::new("Requests")
            .query(PrometheusQuery::new("up"))
            .link(DataLink::new("Runbook", "https://runbook.example/api"))
            .data_link(
                DataLink::new(
                    "Instance",
                    format!(
                        "/d/instance?var-instance={}",
                        DataLinkVariable::FieldLabel("instance".into())
                    ),
                )
                .target_blank(true),
            ),
    );
    let panel = panel_of(&dashboard);

    assert_eq!(
        panel["links"],
        json!([{"title": "Runbook", "url": "https://runbook.example/api"}])
    );
    assert_eq!(
        panel["fieldConfig"]["defaults"]["links"],
        json!([{
            "title": "Instance",
            "url": "/d/instance?var-instance=${__field.labels.instance}",
            "targetBlank": true,
        }])
    );
}

/// `getLinkUrl` and `urlUtil.appendQueryToUrl` (packages/grafana-data/src/utils/url.ts): time range first, then variables, joined with `?` or `&` as the URL needs.
#[test]
fn keep_time_and_include_vars_append_grafanas_url_parameters() {
    let cases = [
        ("/d/api", "/d/api?$__url_time_range&$__all_variables"),
        ("/d/api?", "/d/api?$__url_time_range&$__all_variables"),
        (
            "/d/api?orgId=1",
            "/d/api?orgId=1&$__url_time_range&$__all_variables",
        ),
    ];
    for (url, expected) in cases {
        let dashboard = Dashboard::new("Links").panel(
            Stat::new("Up").query(PrometheusQuery::new("up")).link(
                DataLink::new("Detail", url)
                    .include_vars(true)
                    .keep_time(true),
            ),
        );
        assert_eq!(
            panel_of(&dashboard)["links"][0]["url"],
            json!(expected),
            "{url}"
        );
    }
}

#[test]
fn every_data_link_variable_renders_grafanas_reference() {
    for (variable, expected) in [
        (DataLinkVariable::TimeRange, "${__url_time_range}"),
        (DataLinkVariable::From, "${__from}"),
        (DataLinkVariable::To, "${__to}"),
        (DataLinkVariable::AllVariables, "${__all_variables}"),
        (DataLinkVariable::SeriesName, "${__series.name}"),
        (DataLinkVariable::FieldName, "${__field.name}"),
        (DataLinkVariable::FieldDisplayName, "${__field.displayName}"),
        (
            DataLinkVariable::FieldLabel("job".into()),
            "${__field.labels.job}",
        ),
        (
            DataLinkVariable::FieldLabel("service.name".into()),
            "${__field.labels[\"service.name\"]}",
        ),
        (DataLinkVariable::ValueTime, "${__value.time}"),
        (DataLinkVariable::ValueNumeric, "${__value.numeric}"),
        (DataLinkVariable::ValueText, "${__value.text}"),
        (DataLinkVariable::ValueRaw, "${__value.raw}"),
        (DataLinkVariable::ValueCalc, "${__value.calc}"),
        (
            DataLinkVariable::DataField("route".into()),
            "${__data.fields.route}",
        ),
        (
            DataLinkVariable::DataField("Value #A".into()),
            "${__data.fields[\"Value #A\"]}",
        ),
        (
            DataLinkVariable::Dashboard("instance".into()),
            "${instance}",
        ),
    ] {
        assert_eq!(variable.to_string(), expected);
    }
}

// ---------------------------------------------------------------------------
// Panel query options and repeats
//
// `#Panel` and `#RowPanel` in kinds/dashboard/dashboard_kind.cue.
// ---------------------------------------------------------------------------

fn instance_variable() -> CustomVariable {
    CustomVariable::new("instance", ["a", "b"])
}

#[test]
fn panel_query_options_emit_their_classic_keys_and_nothing_when_unset() {
    let bare =
        Dashboard::new("Options").panel(Timeseries::new("Rate").query(PrometheusQuery::new("up")));
    let bare = panel_of(&bare);
    for key in [
        "interval",
        "maxDataPoints",
        "timeFrom",
        "timeShift",
        "hideTimeOverride",
        "repeat",
        "repeatDirection",
        "maxPerRow",
    ] {
        assert!(bare.get(key).is_none(), "{key} should be omitted");
    }

    let set = Dashboard::new("Options").panel(
        Timeseries::new("Rate")
            .query(PrometheusQuery::new("up"))
            .min_interval("1m")
            .max_data_points(500)
            .time_from("24h")
            .time_shift("1d/d")
            .hide_time_override(true),
    );
    let set = panel_of(&set);
    assert_eq!(set["interval"], json!("1m"));
    assert_eq!(set["maxDataPoints"], json!(500));
    assert_eq!(set["timeFrom"], json!("24h"));
    assert_eq!(set["timeShift"], json!("1d/d"));
    assert_eq!(set["hideTimeOverride"], json!(true));
}

#[test]
fn panel_repeats_emit_direction_and_max_per_row_like_grafana_saves_them() {
    let shape = |repeat: Repeat| {
        let dashboard = Dashboard::new("Repeat")
            .variable(instance_variable())
            .panel(
                Stat::new("Up")
                    .query(PrometheusQuery::new("up"))
                    .repeat(repeat),
            );
        let panel = panel_of(&dashboard);
        json!({
            "repeat": panel["repeat"],
            "repeatDirection": panel["repeatDirection"],
            "maxPerRow": panel["maxPerRow"],
        })
    };

    assert_eq!(
        shape("instance".into()),
        json!({"repeat": "instance", "repeatDirection": "h", "maxPerRow": null})
    );
    assert_eq!(
        shape(Repeat::horizontal("instance").max_per_row(3).into()),
        json!({"repeat": "instance", "repeatDirection": "h", "maxPerRow": 3})
    );
    assert_eq!(
        shape(Repeat::vertical("instance").into()),
        json!({"repeat": "instance", "repeatDirection": "v", "maxPerRow": null})
    );
}

#[test]
fn a_repeated_row_emits_only_the_variable_name() {
    let dashboard = Dashboard::new("Repeat").variable(instance_variable()).row(
        Row::new("Per instance")
            .repeat("instance")
            .panel(Stat::new("Up").query(PrometheusQuery::new("up"))),
    );
    let row = panel_of(&dashboard);

    assert_eq!(row["type"], json!("row"));
    assert_eq!(row["repeat"], json!("instance"));
    assert!(row.get("repeatDirection").is_none());
    assert!(row.get("maxPerRow").is_none());
}

/// Grafana 13's `transformSaveModelToScene` and `DashboardGridItem` force a horizontally repeated panel to 24 columns, so Panelist reserves the full row: the next panel starts below it instead of beside it.
#[test]
fn a_horizontal_repeat_reserves_the_full_row_and_a_vertical_one_keeps_its_width() {
    let dashboard = Dashboard::new("Repeat")
        .variable(instance_variable())
        .panel(
            Stat::new("Before")
                .query(PrometheusQuery::new("up"))
                .width(6),
        )
        .panel(
            Stat::new("Across")
                .query(PrometheusQuery::new("up"))
                .width(6)
                .repeat("instance"),
        )
        .panel(
            Stat::new("After")
                .query(PrometheusQuery::new("up"))
                .width(6),
        )
        .panel(
            Stat::new("Down")
                .query(PrometheusQuery::new("up"))
                .width(6)
                .repeat(Repeat::vertical("instance")),
        );
    let json = as_value(&dashboard);
    let grid = |index: usize| json["panels"][index]["gridPos"].clone();

    assert_eq!(grid(0), json!({"x": 0, "y": 0, "w": 6, "h": 6}));
    assert_eq!(grid(1), json!({"x": 0, "y": 6, "w": 24, "h": 6}));
    assert_eq!(grid(2), json!({"x": 0, "y": 12, "w": 6, "h": 6}));
    assert_eq!(grid(3), json!({"x": 6, "y": 12, "w": 6, "h": 6}));
}

// ---------------------------------------------------------------------------
// Annotations
//
// `#AnnotationQuery` in kinds/dashboard/dashboard_kind.cue; the Prometheus shape is what `prepareAnnotation` in packages/grafana-prometheus/src/annotations.ts migrates to; the built-in entry is `addBuiltInAnnotationQuery` in public/app/features/dashboard/state/DashboardModel.ts.
// ---------------------------------------------------------------------------

#[test]
fn no_annotations_emit_an_empty_list() {
    assert_eq!(
        as_value(&Dashboard::new("Plain"))["annotations"],
        json!({"list": []})
    );
}

#[test]
fn a_prometheus_annotation_emits_the_migrated_target_shape() {
    let dashboard = Dashboard::new("Annotations")
        .datasource(prometheus("prometheus-main"))
        .annotation(
            PrometheusAnnotation::new("Restarts", "changes(process_start_time_seconds[2m]) > 0")
                .step("60s")
                .title_format("Restart")
                .text_format("{{instance}}")
                .tag_keys(["job", "instance"])
                .use_value_for_time(true)
                .icon_color(Color::Orange)
                .hide(true)
                .enable(false),
        )
        .annotation(
            PrometheusAnnotation::new("Bare", "up == 0").datasource(prometheus("prometheus-other")),
        );

    assert_eq!(
        as_value(&dashboard)["annotations"]["list"],
        json!([
            {
                "name": "Restarts",
                "datasource": {"type": "prometheus", "uid": "prometheus-main"},
                "enable": false,
                "hide": true,
                "iconColor": "orange",
                "target": {
                    "expr": "changes(process_start_time_seconds[2m]) > 0",
                    "interval": "60s",
                    "refId": "Anno",
                },
                "titleFormat": "Restart",
                "textFormat": "{{instance}}",
                "tagKeys": "job,instance",
                "useValueForTime": true,
            },
            {
                "name": "Bare",
                "datasource": {"type": "prometheus", "uid": "prometheus-other"},
                "enable": true,
                "hide": false,
                "iconColor": "red",
                "target": {"expr": "up == 0", "refId": "Anno"},
            },
        ])
    );
}

#[test]
fn a_raw_annotation_flattens_its_fields_over_the_common_keys() {
    let dashboard = Dashboard::new("Annotations").annotation(
        RawAnnotation::new("Deploys")
            .datasource(loki("loki-main"))
            .field("expr", json!("{job=\"deployer\"}"))
            .field("iconColor", json!("purple")),
    );

    assert_eq!(
        as_value(&dashboard)["annotations"]["list"],
        json!([{
            "name": "Deploys",
            "datasource": {"type": "loki", "uid": "loki-main"},
            "enable": true,
            "hide": false,
            "iconColor": "purple",
            "expr": "{job=\"deployer\"}",
        }])
    );
}

#[test]
fn a_configured_built_in_annotation_comes_first_with_grafanas_identity() {
    let dashboard = Dashboard::new("Annotations")
        .annotation(PrometheusAnnotation::new("Restarts", "up == 0"))
        .built_in_annotation(BuiltInAnnotation::new().enable(false));
    let list = as_value(&dashboard)["annotations"]["list"].clone();

    assert_eq!(
        list[0],
        json!({
            "builtIn": 1,
            "datasource": {"type": "grafana", "uid": "-- Grafana --"},
            "enable": false,
            "hide": true,
            "iconColor": "rgba(0, 211, 255, 1)",
            "name": "Annotations & Alerts",
            "type": "dashboard",
        })
    );
    assert_eq!(list[1]["name"], json!("Restarts"));
}

/// The `instant`/`range` keys a Prometheus-style target carries for each evaluation. Unset leaves both out, so Grafana's own default applies.
#[test]
fn each_query_type_emits_its_instant_and_range_keys() {
    for (query_type, expected) in [
        (None, json!({"refId": "A", "expr": "up", "hide": false})),
        (
            Some(QueryType::Range),
            json!({"refId": "A", "expr": "up", "range": true, "hide": false}),
        ),
        (
            Some(QueryType::Instant),
            json!({"refId": "A", "expr": "up", "instant": true, "range": false, "hide": false}),
        ),
        (
            Some(QueryType::Both),
            json!({"refId": "A", "expr": "up", "instant": true, "range": true, "hide": false}),
        ),
    ] {
        let mut query = PrometheusQuery::new("up");
        if let Some(query_type) = query_type {
            query = query.query_type(query_type);
        }
        let dashboard = Dashboard::new("Targets").panel(Stat::new("Value").query(query));
        assert_eq!(
            panel_of(&dashboard)["targets"][0],
            expected,
            "{query_type:?}"
        );
    }
}

/// Loki reads `queryType` (`LokiQueryType` in `public/app/plugins/datasource/loki/dataquery.gen.ts`) and always gets one, `range` when nothing else was asked for.
#[test]
fn loki_targets_carry_query_type() {
    let dashboard = Dashboard::new("Logs").panel(
        Table::new("Errors")
            .query(LokiQuery::new("{job=\"api\"}"))
            .query(LokiQuery::new("{job=\"api\"}").instant()),
    );
    let targets = &panel_of(&dashboard)["targets"];
    assert_eq!(
        targets[0],
        json!({"refId": "A", "expr": "{job=\"api\"}", "hide": false, "queryType": "range"})
    );
    assert_eq!(
        targets[1],
        json!({
            "refId": "B",
            "expr": "{job=\"api\"}",
            "instant": true,
            "range": false,
            "hide": false,
            "queryType": "instant",
        })
    );
}

/// A raw target carries only what was set. CloudWatch, for one, has no `expr` key at all, and a stray empty one is noise in its query editor.
#[test]
fn a_raw_target_without_an_expression_has_no_expr_key() {
    let dashboard = Dashboard::new("Raw").panel(
        Timeseries::new("CPU").query(
            RawQuery::new()
                .datasource(DataSource::new("cloudwatch", "cloudwatch-main"))
                .extra("namespace", json!("AWS/EC2"))
                .extra("metricName", json!("CPUUtilization")),
        ),
    );
    assert_eq!(
        panel_of(&dashboard)["targets"][0],
        json!({
            "refId": "A",
            "datasource": {"type": "cloudwatch", "uid": "cloudwatch-main"},
            "hide": false,
            "metricName": "CPUUtilization",
            "namespace": "AWS/EC2",
        })
    );

    let with_expression = Dashboard::new("Raw").panel(
        Timeseries::new("Up").query(
            RawQuery::new()
                .expression("up")
                .extra("exemplar", json!(true)),
        ),
    );
    assert_eq!(
        panel_of(&with_expression)["targets"][0],
        json!({"refId": "A", "expr": "up", "hide": false, "exemplar": true})
    );
}

/// Built-in datasources are served with the plugin type `datasource` and their name as UID (`pkg/api/frontendsettings.go`, `public/app/features/plugins/datasource_srv.ts`). Each target keeps its own datasource; validation rejects one that would inherit the mixed ref.
#[test]
fn a_mixed_panel_emits_the_builtin_ref_and_targets_keep_their_own() {
    let dashboard = Dashboard::new("Mixed").panel(
        Timeseries::new("Requests and errors")
            .datasource(DataSource::mixed())
            .query(PrometheusQuery::new("up").datasource(prometheus("metrics")))
            .query(LokiQuery::new("{job=\"api\"}").datasource(loki("logs"))),
    );
    let panel = panel_of(&dashboard);
    assert_eq!(
        panel["datasource"],
        json!({"type": "datasource", "uid": "-- Mixed --"})
    );
    assert_eq!(
        panel["targets"][0]["datasource"],
        json!({"type": "prometheus", "uid": "metrics"})
    );
    assert_eq!(
        panel["targets"][1]["datasource"],
        json!({"type": "loki", "uid": "logs"})
    );
}

/// `RenameByRegexTransformerOptions` in `packages/grafana-data/src/transformations/transformers/renameByRegex.ts`.
#[test]
fn rename_by_regex_emits_regex_and_rename_pattern() {
    let dashboard = Dashboard::new("Rename").panel(
        Table::new("Services")
            .query(PrometheusQuery::new("up").ref_id("A"))
            .transform(RenameByRegex::new("(.*)-api", "$1").only_ref_id("A")),
    );
    assert_eq!(
        first_transformation(&dashboard),
        json!({
            "id": "renameByRegex",
            "options": {"regex": "(.*)-api", "renamePattern": "$1"},
            "filter": {"id": "byRefId", "options": "A"},
        })
    );
}

/// A query's own field properties become one `byFrameRefID` override on its resolved reference ID, explicit or assigned. Query overrides come first, in query order, then the panel's, so the panel can refine them.
#[test]
fn query_field_properties_become_leading_by_frame_ref_id_overrides() {
    let dashboard = Dashboard::new("Per query").panel(
        Timeseries::new("Traffic")
            .query(
                PrometheusQuery::new("sum(rate(http_requests_total[$__rate_interval]))")
                    .unit(Unit::RequestsPerSecond)
                    .color(Color::Green),
            )
            .query(PrometheusQuery::new("plain"))
            .query(
                PrometheusQuery::new(
                    "sum(rate(http_requests_total{status=~\"5..\"}[$__rate_interval]))",
                )
                .ref_id("errors")
                .color(Color::Red)
                .display_name("Errors"),
            )
            .override_field(FieldOverride::by_name("Errors").line_width(3)),
    );
    assert_eq!(
        panel_of(&dashboard)["fieldConfig"]["overrides"],
        json!([
            {
                "matcher": {"id": "byFrameRefID", "options": "A"},
                "properties": [
                    {"id": "unit", "value": "reqps"},
                    {"id": "color", "value": {"mode": "fixed", "fixedColor": "green"}},
                ],
            },
            {
                "matcher": {"id": "byFrameRefID", "options": "errors"},
                "properties": [
                    {"id": "color", "value": {"mode": "fixed", "fixedColor": "red"}},
                    {"id": "displayName", "value": "Errors"},
                ],
            },
            {
                "matcher": {"id": "byName", "options": "Errors"},
                "properties": [{"id": "custom.lineWidth", "value": 3}],
            },
        ])
    );
}
