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

use serde_json::Value;

/// A Grafana field unit with common units represented as Rust variants.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Unit {
    /// Grafana's automatic/default unit behavior.
    #[default]
    None,
    /// Seconds (`s`).
    Seconds,
    /// Milliseconds (`ms`).
    Milliseconds,
    /// Bytes using IEC scaling (`bytes`).
    Bytes,
    /// Bytes per second using IEC scaling (`Bps`).
    BytesPerSecond,
    /// Percentage in the 0–100 range (`percent`).
    Percent,
    /// Requests per second (`reqps`).
    RequestsPerSecond,
    /// Operations per second (`ops`).
    OperationsPerSecond,
    /// Compact short-number formatting (`short`).
    Short,
    /// A custom Grafana unit identifier or literal suffix.
    Custom(String),
}

impl Unit {
    /// Creates a custom Grafana unit identifier or literal suffix.
    #[must_use]
    pub fn custom(unit: impl Into<String>) -> Self {
        Self::Custom(unit.into())
    }

    pub(crate) fn as_grafana(&self) -> &str {
        match self {
            Self::None => "none",
            Self::Seconds => "s",
            Self::Milliseconds => "ms",
            Self::Bytes => "bytes",
            Self::BytesPerSecond => "Bps",
            Self::Percent => "percent",
            Self::RequestsPerSecond => "reqps",
            Self::OperationsPerSecond => "ops",
            Self::Short => "short",
            Self::Custom(value) => value,
        }
    }
}

/// A Grafana color name or custom color value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Color {
    /// Green.
    Green,
    /// Yellow.
    Yellow,
    /// Red.
    Red,
    /// Blue.
    Blue,
    /// Orange.
    Orange,
    /// Purple.
    Purple,
    /// A Grafana color name or CSS-compatible color value.
    Custom(String),
}

impl Color {
    /// Creates a custom Grafana color name or CSS-compatible color value.
    #[must_use]
    pub fn custom(color: impl Into<String>) -> Self {
        Self::Custom(color.into())
    }

    pub(crate) fn as_grafana(&self) -> &str {
        match self {
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Red => "red",
            Self::Blue => "blue",
            Self::Orange => "orange",
            Self::Purple => "purple",
            Self::Custom(value) => value,
        }
    }
}

/// How Grafana interprets threshold values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThresholdMode {
    /// Thresholds are absolute field values.
    #[default]
    Absolute,
    /// Thresholds are percentages between configured minimum and maximum.
    Percentage,
}

/// A color transition at an optional numeric value.
#[derive(Debug, Clone, PartialEq)]
pub struct ThresholdStep {
    pub(crate) color: Color,
    pub(crate) value: Option<f64>,
}

impl ThresholdStep {
    /// Creates a threshold step. `None` represents Grafana's base step.
    #[must_use]
    pub fn new(color: Color, value: Option<f64>) -> Self {
        Self { color, value }
    }
}

/// Ordered threshold configuration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Thresholds {
    pub(crate) mode: ThresholdMode,
    pub(crate) steps: Vec<ThresholdStep>,
}

impl Thresholds {
    /// Creates absolute thresholds.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Changes how Grafana interprets the values.
    #[must_use]
    pub fn mode(mut self, mode: ThresholdMode) -> Self {
        self.mode = mode;
        self
    }

    /// Appends an ordered threshold step.
    #[must_use]
    pub fn step(mut self, color: Color, value: impl Into<Option<f64>>) -> Self {
        self.steps.push(ThresholdStep::new(color, value.into()));
        self
    }

    /// Appends a green threshold.
    #[must_use]
    pub fn green(self, value: f64) -> Self {
        self.step(Color::Green, value)
    }

    /// Appends a yellow threshold.
    #[must_use]
    pub fn yellow(self, value: f64) -> Self {
        self.step(Color::Yellow, value)
    }

    /// Appends a red threshold.
    #[must_use]
    pub fn red(self, value: f64) -> Self {
        self.step(Color::Red, value)
    }
}

/// A typed Grafana field color scheme.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColorScheme {
    /// Use thresholds to determine the color.
    Thresholds,
    /// Use a classic palette indexed by series.
    ClassicPalette,
    /// Use one fixed color.
    Fixed(Color),
    /// Use a named Grafana continuous palette.
    Continuous(String),
}

/// Common field defaults shared by panel types.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FieldConfig {
    pub(crate) unit: Option<Unit>,
    pub(crate) min: Option<f64>,
    pub(crate) max: Option<f64>,
    pub(crate) decimals: Option<u8>,
    pub(crate) display_name: Option<String>,
    pub(crate) color: Option<ColorScheme>,
    pub(crate) thresholds: Option<Thresholds>,
    pub(crate) custom: BTreeMap<String, Value>,
    pub(crate) overrides: Vec<FieldOverride>,
}

impl FieldConfig {
    /// Creates empty field defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the display unit.
    #[must_use]
    pub fn unit(mut self, unit: Unit) -> Self {
        self.unit = Some(unit);
        self
    }

    /// Sets the field minimum.
    #[must_use]
    pub fn min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    /// Sets the field maximum.
    #[must_use]
    pub fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    /// Sets the displayed decimal count.
    #[must_use]
    pub fn decimals(mut self, decimals: u8) -> Self {
        self.decimals = Some(decimals);
        self
    }

    /// Sets the display-name template.
    #[must_use]
    pub fn display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    /// Sets the field color scheme.
    #[must_use]
    pub fn color(mut self, color: ColorScheme) -> Self {
        self.color = Some(color);
        self
    }

    /// Sets threshold configuration.
    #[must_use]
    pub fn thresholds(mut self, thresholds: Thresholds) -> Self {
        self.thresholds = Some(thresholds);
        self
    }

    /// Adds a plugin-specific field default.
    #[must_use]
    pub fn custom(mut self, key: impl Into<String>, value: Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }

    /// Adds a field override.
    #[must_use]
    pub fn override_field(mut self, field_override: FieldOverride) -> Self {
        self.overrides.push(field_override);
        self
    }
}

/// Selects fields to which an override applies.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OverrideMatcher {
    /// Match an exact field name.
    Name(String),
    /// Match a regular expression.
    Regex(String),
    /// Match a Grafana field type such as `number` or `string`.
    Type(String),
}

/// A typed or custom field override property.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum OverrideProperty {
    /// Override the field unit.
    Unit(Unit),
    /// Override the minimum.
    Min(f64),
    /// Override the maximum.
    Max(f64),
    /// Override decimal rendering.
    Decimals(u8),
    /// Override display name.
    DisplayName(String),
    /// Override the field color scheme.
    Color(ColorScheme),
    /// Override line width for compatible panels.
    LineWidth(u8),
    /// Explicit Grafana property escape hatch.
    Custom {
        /// Grafana field-property identifier.
        id: String,
        /// Property value.
        value: Value,
    },
}

/// A matcher and ordered set of field properties.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldOverride {
    pub(crate) matcher: OverrideMatcher,
    pub(crate) properties: Vec<OverrideProperty>,
}

impl FieldOverride {
    /// Matches one exact field name.
    #[must_use]
    pub fn by_name(name: impl Into<String>) -> Self {
        Self {
            matcher: OverrideMatcher::Name(name.into()),
            properties: Vec::new(),
        }
    }

    /// Matches fields using a regular expression.
    #[must_use]
    pub fn by_regex(regex: impl Into<String>) -> Self {
        Self {
            matcher: OverrideMatcher::Regex(regex.into()),
            properties: Vec::new(),
        }
    }

    /// Matches fields by Grafana field type.
    #[must_use]
    pub fn by_type(field_type: impl Into<String>) -> Self {
        Self {
            matcher: OverrideMatcher::Type(field_type.into()),
            properties: Vec::new(),
        }
    }

    /// Appends an override property.
    #[must_use]
    pub fn property(mut self, property: OverrideProperty) -> Self {
        self.properties.push(property);
        self
    }

    /// Sets a fixed color.
    #[must_use]
    pub fn color(self, color: Color) -> Self {
        self.property(OverrideProperty::Color(ColorScheme::Fixed(color)))
    }

    /// Sets line width for compatible visualizations.
    #[must_use]
    pub fn line_width(self, width: u8) -> Self {
        self.property(OverrideProperty::LineWidth(width))
    }
}

/// Grafana legend rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LegendMode {
    /// Compact vertical list.
    #[default]
    List,
    /// Tabular legend with calculations.
    Table,
    /// Hide the legend.
    Hidden,
}

/// Grafana legend placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LegendPlacement {
    /// Below the visualization.
    #[default]
    Bottom,
    /// To the right of the visualization.
    Right,
}

/// A calculation displayed in a table legend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LegendCalculation {
    /// Last non-null value.
    Last,
    /// Minimum value.
    Min,
    /// Maximum value.
    Max,
    /// Mean value.
    Mean,
    /// Sum.
    Total,
}

impl LegendCalculation {
    pub(crate) fn as_grafana(self) -> &'static str {
        match self {
            Self::Last => "lastNotNull",
            Self::Min => "min",
            Self::Max => "max",
            Self::Mean => "mean",
            Self::Total => "sum",
        }
    }
}

/// Common legend settings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Legend {
    pub(crate) mode: LegendMode,
    pub(crate) placement: LegendPlacement,
    pub(crate) calculations: Vec<LegendCalculation>,
}

impl Legend {
    /// Creates the default visible bottom list legend.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets list, table, or hidden mode.
    #[must_use]
    pub fn mode(mut self, mode: LegendMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets bottom or right placement.
    #[must_use]
    pub fn placement(mut self, placement: LegendPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Replaces the displayed calculations.
    #[must_use]
    pub fn calculations(
        mut self,
        calculations: impl IntoIterator<Item = LegendCalculation>,
    ) -> Self {
        self.calculations = calculations.into_iter().collect();
        self
    }
}
