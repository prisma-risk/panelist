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
    Color, ColorScheme, FieldConfig, FieldOverride, OverrideMatcher, OverrideProperty, PanelKind,
    Reducer, ThresholdMode, Thresholds, Unit,
};

use super::wire::{
    GrafanaFieldConfig, GrafanaFieldOverride, GrafanaMatcher, GrafanaOverrideProperty,
};

pub(crate) fn unit(unit: &Unit) -> &str {
    match unit {
        Unit::None => "none",
        Unit::Seconds => "s",
        Unit::Milliseconds => "ms",
        Unit::Bytes => "bytes",
        Unit::BytesPerSecond => "Bps",
        Unit::Percent => "percent",
        Unit::RequestsPerSecond => "reqps",
        Unit::OperationsPerSecond => "ops",
        Unit::Short => "short",
        Unit::Custom(value) => value,
    }
}

pub(crate) fn color(color: &Color) -> &str {
    match color {
        Color::Green => "green",
        Color::Yellow => "yellow",
        Color::Red => "red",
        Color::Blue => "blue",
        Color::Orange => "orange",
        Color::Purple => "purple",
        Color::Custom(value) => value,
    }
}

pub(crate) const fn reducer(reducer: Reducer) -> &'static str {
    match reducer {
        Reducer::Last => "lastNotNull",
        Reducer::Min => "min",
        Reducer::Max => "max",
        Reducer::Mean => "mean",
        Reducer::Total => "sum",
    }
}

pub(crate) fn normalize_field_config(config: &FieldConfig, kind: &PanelKind) -> GrafanaFieldConfig {
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

pub(crate) fn normalize_override(field_override: &FieldOverride) -> GrafanaFieldOverride {
    let matcher = match &field_override.matcher {
        OverrideMatcher::Name(name) => GrafanaMatcher {
            id: "byName",
            options: name.clone(),
        },
        OverrideMatcher::Regex(regex) => GrafanaMatcher {
            id: "byRegexp",
            options: regex.clone(),
        },
        OverrideMatcher::Type(field_type) => GrafanaMatcher {
            id: "byType",
            options: field_type.clone(),
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
