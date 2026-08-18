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

use serde::Serialize;
use serde_json::{Value, json};

use crate::{
    CellBackgroundMode, CellValueDisplay, ColorScheme, FieldConfig, FieldOverride, OverrideMatcher,
    OverrideProperty, PanelKind, Stacking, TableCell, ThresholdMode, Thresholds,
    panel::PanelOptions,
};

use super::vocabulary::{color, line_interpolation, point_visibility, stacking_mode, unit};
use super::wire::{
    GrafanaFieldConfig, GrafanaFieldOverride, GrafanaMatcher, GrafanaOverrideProperty,
};

pub(crate) fn normalize_field_config(
    config: &FieldConfig,
    kind: &PanelKind,
    kind_options: &PanelOptions,
) -> GrafanaFieldConfig {
    let mut defaults = BTreeMap::new();
    if let Some(field_unit) = &config.unit {
        defaults.insert("unit".to_owned(), json!(unit(field_unit)));
    }
    insert_number(&mut defaults, "min", config.min);
    insert_number(&mut defaults, "max", config.max);
    if let Some(decimals) = config.decimals {
        defaults.insert("decimals".to_owned(), json!(decimals));
    }
    if let Some(display_name) = &config.display_name {
        defaults.insert("displayName".to_owned(), json!(display_name));
    }
    if let Some(color) = &config.color {
        defaults.insert("color".to_owned(), color_scheme_value(color));
    }
    if let Some(thresholds) = &config.thresholds {
        defaults.insert("thresholds".to_owned(), thresholds_value(thresholds));
    }
    if !config.mappings.is_empty() {
        let options = config
            .mappings
            .iter()
            .map(|mapping| {
                let mut result = serde_json::Map::new();
                result.insert("text".to_owned(), json!(mapping.text));
                if let Some(mapping_color) = &mapping.color {
                    result.insert("color".to_owned(), json!(color(mapping_color)));
                }
                (mapping.value.clone(), Value::Object(result))
            })
            .collect::<serde_json::Map<_, _>>();
        defaults.insert(
            "mappings".to_owned(),
            json!([{"type": "value", "options": options}]),
        );
    }

    let mut custom = default_field_custom(kind);
    custom.extend(typed_field_custom(kind_options));
    custom.extend(config.custom.clone());
    if !custom.is_empty() {
        defaults.insert("custom".to_owned(), json!(custom));
    }

    GrafanaFieldConfig {
        defaults,
        overrides: config.overrides.iter().map(normalize_override).collect(),
    }
}

pub(crate) fn default_field_custom(kind: &PanelKind) -> BTreeMap<String, Value> {
    let mut custom = BTreeMap::new();
    match kind {
        PanelKind::Timeseries => {
            custom.insert("drawStyle".to_owned(), json!("line"));
            custom.insert("fillOpacity".to_owned(), json!(0));
            custom.insert("lineInterpolation".to_owned(), json!("linear"));
            custom.insert("lineWidth".to_owned(), json!(1));
            custom.insert("showPoints".to_owned(), json!("auto"));
            custom.insert("spanNulls".to_owned(), json!(false));
        }
        PanelKind::Table => {
            custom.insert("align".to_owned(), json!("auto"));
            custom.insert("inspect".to_owned(), json!(false));
        }
        PanelKind::Stat
        | PanelKind::Gauge
        | PanelKind::Text
        | PanelKind::BarGauge
        | PanelKind::Heatmap
        | PanelKind::Raw(_) => {}
    }
    custom
}

pub(crate) fn typed_field_custom(options: &PanelOptions) -> BTreeMap<String, Value> {
    let mut output = BTreeMap::new();
    match options {
        // Exhaustive on purpose: a new `PanelOptions` variant that owns
        // `fieldConfig.defaults.custom` keys must fail to compile here until
        // it is spelled out, rather than silently falling through and
        // dropping its keys. Heatmap's options all live under the panel's
        // `options` object rather than `fieldConfig.defaults.custom`, so it
        // joins this no-op group. Table has its own arm below because its
        // cell renderer lives at `custom.cellOptions`.
        PanelOptions::None
        | PanelOptions::Stat(_)
        | PanelOptions::Gauge(_)
        | PanelOptions::BarGauge(_)
        | PanelOptions::Heatmap(_) => {}
        PanelOptions::Table(table) => {
            if let Some(cell) = &table.cell {
                output.insert("cellOptions".to_owned(), cell_options_value(cell));
            }
        }
        PanelOptions::Timeseries(timeseries) => {
            if let Some(opacity) = timeseries.fill_opacity {
                output.insert("fillOpacity".to_owned(), json!(opacity));
            }
            if let Some(width) = timeseries.line_width {
                output.insert("lineWidth".to_owned(), json!(width));
            }
            if let Some(size) = timeseries.point_size {
                output.insert("pointSize".to_owned(), json!(size));
            }
            if let Some(interpolation) = timeseries.line_interpolation {
                output.insert(
                    "lineInterpolation".to_owned(),
                    json!(line_interpolation(interpolation)),
                );
            }
            if let Some(visibility) = timeseries.show_points {
                output.insert("showPoints".to_owned(), json!(point_visibility(visibility)));
            }
            if let Some(span_nulls) = timeseries.span_nulls {
                output.insert("spanNulls".to_owned(), json!(span_nulls));
            }
            if let Some(stacking) = &timeseries.stacking {
                output.insert("stacking".to_owned(), stacking_value(stacking));
            }
        }
    }
    output
}

pub(crate) fn normalize_override(field_override: &FieldOverride) -> GrafanaFieldOverride {
    // Field matchers, distinct from the frame matchers used by transformation
    // filters (see `grafana/transform.rs`). "which query" is spelled
    // `byFrameRefID` here and `byRefId` there — do not conflate the two.
    //
    // Every `options` shape below is the argument type of the matching
    // `FieldMatcherInfo<T>.get` in Grafana v13.0.2. Grafana stores and
    // returns an `options` value of the wrong shape untouched and only
    // ignores it at render time, so neither the golden test nor the
    // round-trip verifier can catch a mistake here — check the source, not
    // the JSON:
    //
    //   `packages/grafana-data/src/transformations/matchers/nameMatcher.ts`
    //     byName        `FieldMatcherInfo<string>`, L37-60  — bare string
    //     byRegexp      `FieldMatcherInfo<string>`, L126-144 — bare string
    //     byNames       `FieldMatcherInfo<ByNamesMatcherOptions>`, L62-101 —
    //                   OBJECT. `get` does
    //                   `const { names, mode = include } = options`, so a
    //                   bare array yields `names === undefined` and matches
    //                   zero fields. `mode` is omitted deliberately: it
    //                   defaults to `include`, which is the only mode this
    //                   matcher models.
    //     byFrameRefID  `FieldMatcherInfo<string>`, L151-166 — bare string
    //
    //   `packages/grafana-data/src/transformations/matchers/fieldTypeMatcher.ts`
    //     byType        `FieldMatcherInfo<FieldType>`, L7-22 — bare string;
    //                   `FieldType` is a string enum
    //                   (`packages/grafana-data/src/types/dataFrame.ts` L18-33)
    //     numeric       `FieldMatcherInfo`, L44-56 — `get: () => …` ignores
    //                   its argument, so the key is omitted
    //     time          `FieldMatcherInfo`, L59-71 — same
    let matcher = match &field_override.matcher {
        OverrideMatcher::Name(name) => GrafanaMatcher {
            id: "byName",
            options: Some(json!(name)),
        },
        OverrideMatcher::Regex(regex) => GrafanaMatcher {
            id: "byRegexp",
            options: Some(json!(regex)),
        },
        OverrideMatcher::Type(field_type) => GrafanaMatcher {
            id: "byType",
            options: Some(json!(field_type)),
        },
        OverrideMatcher::QueryRefId(ref_id) => GrafanaMatcher {
            id: "byFrameRefID",
            options: Some(json!(ref_id)),
        },
        OverrideMatcher::Names(names) => GrafanaMatcher {
            id: "byNames",
            options: Some(json!({"names": names})),
        },
        OverrideMatcher::Numeric => GrafanaMatcher {
            id: "numeric",
            options: None,
        },
        OverrideMatcher::Time => GrafanaMatcher {
            id: "time",
            options: None,
        },
    };

    GrafanaFieldOverride {
        matcher,
        properties: field_override
            .properties
            .iter()
            .map(|property| match property {
                OverrideProperty::Unit(value) => property_value("unit", unit(value)),
                OverrideProperty::Min(value) => number_property("min", *value),
                OverrideProperty::Max(value) => number_property("max", *value),
                OverrideProperty::Decimals(value) => property_value("decimals", *value),
                OverrideProperty::DisplayName(value) => property_value("displayName", value),
                OverrideProperty::Color(value) => GrafanaOverrideProperty {
                    id: "color".to_owned(),
                    value: color_scheme_value(value),
                },
                OverrideProperty::LineWidth(value) => property_value("custom.lineWidth", *value),
                OverrideProperty::Thresholds(value) => GrafanaOverrideProperty {
                    id: "thresholds".to_owned(),
                    value: thresholds_value(value),
                },
                OverrideProperty::Cell(cell) => GrafanaOverrideProperty {
                    id: "custom.cellOptions".to_owned(),
                    value: cell_options_value(cell),
                },
                OverrideProperty::Custom { id, value } => GrafanaOverrideProperty {
                    id: id.clone(),
                    value: value.clone(),
                },
            })
            .collect(),
    }
}

pub(crate) fn property_value(id: &str, value: impl Serialize) -> GrafanaOverrideProperty {
    GrafanaOverrideProperty {
        id: id.to_owned(),
        value: serde_json::to_value(value).unwrap_or(Value::Null),
    }
}

pub(crate) fn number_property(id: &str, value: f64) -> GrafanaOverrideProperty {
    GrafanaOverrideProperty {
        id: id.to_owned(),
        value: serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number),
    }
}

pub(crate) fn insert_number(map: &mut BTreeMap<String, Value>, key: &str, number: Option<f64>) {
    if let Some(value) = number.and_then(serde_json::Number::from_f64) {
        map.insert(key.to_owned(), Value::Number(value));
    }
}

pub(crate) fn stacking_value(stacking: &Stacking) -> Value {
    json!({"mode": stacking_mode(stacking.mode), "group": stacking.group})
}

pub(crate) fn color_scheme_value(scheme: &ColorScheme) -> Value {
    match scheme {
        ColorScheme::Thresholds => json!({"mode": "thresholds"}),
        ColorScheme::ClassicPalette => json!({"mode": "palette-classic"}),
        ColorScheme::Fixed(value) => json!({"mode": "fixed", "fixedColor": color(value)}),
        ColorScheme::Continuous(scheme) => json!({"mode": scheme}),
    }
}

pub(crate) fn thresholds_value(thresholds: &Thresholds) -> Value {
    let mut steps = Vec::new();
    if thresholds
        .steps
        .first()
        .is_none_or(|step| step.value.is_some())
    {
        let step_color = thresholds
            .steps
            .first()
            .map_or("green", |step| color(&step.color));
        steps.push(json!({"color": step_color, "value": null}));
    }
    steps.extend(thresholds.steps.iter().map(|step| {
        json!({
            "color": color(&step.color),
            "value": step.value,
        })
    }));
    json!({
        "mode": match thresholds.mode {
            ThresholdMode::Absolute => "absolute",
            ThresholdMode::Percentage => "percentage",
        },
        "steps": steps,
    })
}

// Cell option shapes follow Grafana's TableCellOptions union in
// packages/grafana-schema/src/common/common.gen.ts. Sparkline cells extend
// GraphFieldConfig, so line and fill properties sit directly on the object.
pub(crate) fn cell_options_value(cell: &TableCell) -> Value {
    let mut output = serde_json::Map::new();
    match cell {
        TableCell::Auto => {
            output.insert("type".to_owned(), json!("auto"));
        }
        TableCell::ColoredText => {
            output.insert("type".to_owned(), json!("color-text"));
        }
        TableCell::ColoredBackground(options) => {
            output.insert("type".to_owned(), json!("color-background"));
            if let Some(mode) = options.mode {
                output.insert(
                    "mode".to_owned(),
                    json!(match mode {
                        CellBackgroundMode::Basic => "basic",
                        CellBackgroundMode::Gradient => "gradient",
                    }),
                );
            }
            if let Some(apply_to_row) = options.apply_to_row {
                output.insert("applyToRow".to_owned(), json!(apply_to_row));
            }
            if let Some(wrap_text) = options.wrap_text {
                output.insert("wrapText".to_owned(), json!(wrap_text));
            }
        }
        TableCell::Gauge(options) => {
            output.insert("type".to_owned(), json!("gauge"));
            if let Some(mode) = options.mode {
                output.insert(
                    "mode".to_owned(),
                    json!(super::vocabulary::bar_gauge_display_mode(mode)),
                );
            }
            if let Some(display) = options.value_display {
                output.insert(
                    "valueDisplayMode".to_owned(),
                    json!(match display {
                        CellValueDisplay::Text => "text",
                        CellValueDisplay::Color => "color",
                        CellValueDisplay::Hidden => "hidden",
                    }),
                );
            }
        }
        TableCell::Sparkline(options) => {
            output.insert("type".to_owned(), json!("sparkline"));
            if let Some(hide_value) = options.hide_value {
                output.insert("hideValue".to_owned(), json!(hide_value));
            }
            if let Some(width) = options.line_width {
                output.insert("lineWidth".to_owned(), json!(width));
            }
            if let Some(opacity) = options.fill_opacity {
                output.insert("fillOpacity".to_owned(), json!(opacity));
            }
        }
    }
    Value::Object(output)
}
