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

use crate::{DataLink, Reducer, TableCell, TableCellType};

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
    /// Percentage in the 0–100 range (`percent`). A query yielding a raw
    /// fraction (0.0–1.0) needs [`Unit::PercentUnit`] instead, or must be
    /// multiplied by 100 to match this scale.
    Percent,
    /// Percentage expressed as a 0–1 fraction (`percentunit`), the scale
    /// Prometheus ratio queries such as `a / b` naturally produce. Contrast
    /// with [`Unit::Percent`]'s 0–100 range — thresholds authored against
    /// one scale will not fire correctly if the field actually uses the
    /// other.
    PercentUnit,
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
}

/// A Grafana value mapping: display text and an optional color substituted for the field values it matches.
///
/// Each constructor picks one of Grafana's four mapping kinds, and each kind carries only the match it needs: an exact value, a numeric range, a regular expression, or a [`SpecialValue`]. The result is set with [`ValueMapping::text`] and [`ValueMapping::color`]; a mapping must set at least one of them.
///
/// Grafana evaluates mappings in array order and stops at the first match. Panelist lowers mappings the way Grafana's own mapping editor saves them: every exact-value mapping joins one leading `value` entry, followed by the range, regex, and special mappings in authored order. Exact values therefore always take precedence over the other kinds, and each result records its authored position as `index`, which the editor uses to show the rows in the order they were written.
///
/// ```rust
/// use panelist::prelude::*;
///
/// let stat = Stat::new("Status")
///     .mapping(ValueMapping::value("1").text("Up").color(Color::Green))
///     .mapping(ValueMapping::range(0.0, 0.5).text("Degraded").color(Color::Yellow))
///     .mapping(ValueMapping::regex("^prod-(.*)$").text("$1"))
///     .mapping(ValueMapping::special(SpecialValue::Null).text("No data"));
/// # let _ = stat;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ValueMapping {
    pub(crate) matcher: ValueMatch,
    pub(crate) text: Option<String>,
    pub(crate) color: Option<Color>,
}

/// The match half of a [`ValueMapping`], one variant per Grafana mapping type.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ValueMatch {
    /// Grafana `value`: one exact value, compared as a string.
    Value(String),
    /// Grafana `range`: an inclusive numeric range. `None` is an open end.
    Range { from: Option<f64>, to: Option<f64> },
    /// Grafana `regex`: a regular expression matched against string values.
    Regex(String),
    /// Grafana `special`.
    Special(SpecialValue),
}

/// A value Grafana's `special` mapping type matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialValue {
    /// A null or missing value (`null`).
    Null,
    /// A numeric NaN (`nan`).
    NaN,
    /// Either a null value or a NaN (`null+nan`).
    NullOrNaN,
    /// A boolean `true` or the string `"true"` (`true`).
    True,
    /// A boolean `false` or the string `"false"` (`false`).
    False,
    /// An empty string (`empty`).
    Empty,
}

impl ValueMapping {
    fn matching(matcher: ValueMatch) -> Self {
        Self {
            matcher,
            text: None,
            color: None,
        }
    }

    /// Matches one exact field value.
    ///
    /// Grafana looks the value up as a string, so a numeric field value `1` matches `"1"`.
    #[must_use]
    pub fn value(value: impl Into<String>) -> Self {
        Self::matching(ValueMatch::Value(value.into()))
    }

    /// Matches numeric values between `from` and `to`, both inclusive.
    ///
    /// `None` leaves that end open, so `ValueMapping::range(90.0, None)` matches every value of at least 90. At least one end must be set, both must be finite, and `from` must not exceed `to`.
    #[must_use]
    pub fn range(from: impl Into<Option<f64>>, to: impl Into<Option<f64>>) -> Self {
        Self::matching(ValueMatch::Range {
            from: from.into(),
            to: to.into(),
        })
    }

    /// Matches string values against a regular expression.
    ///
    /// The display text may reference capture groups: with the pattern `^prod-(.*)$`, the text `$1` displays `api` for the value `prod-api`. Grafana skips this mapping for non-string values.
    #[must_use]
    pub fn regex(pattern: impl Into<String>) -> Self {
        Self::matching(ValueMatch::Regex(pattern.into()))
    }

    /// Matches null, NaN, boolean, or empty values.
    #[must_use]
    pub fn special(value: SpecialValue) -> Self {
        Self::matching(ValueMatch::Special(value))
    }

    /// Sets the text displayed instead of the matched value.
    ///
    /// Blank text is omitted from the JSON, as Grafana's mapping editor does, so the original value keeps displaying.
    #[must_use]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Colors the mapped result.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
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
    pub(crate) mappings: Vec<ValueMapping>,
    pub(crate) links: Vec<DataLink>,
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

    /// Adds a value mapping.
    #[must_use]
    pub fn mapping(mut self, mapping: ValueMapping) -> Self {
        self.mappings.push(mapping);
        self
    }

    /// Adds a data link shown when a viewer clicks a field value.
    #[must_use]
    pub fn data_link(mut self, link: DataLink) -> Self {
        self.links.push(link);
        self
    }

    /// Adds a plugin-specific field default.
    ///
    /// This escape hatch is applied after every typed field-custom setter
    /// (e.g. `PanelBuilder::line_width`), so it wins over the equivalent
    /// typed value regardless of call order.
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
    /// Match every field returned by one query reference ID.
    QueryRefId(String),
    /// Match an explicit list of field names.
    Names(Vec<String>),
    /// Match every numeric field.
    Numeric,
    /// Match every time field.
    Time,
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
    /// Override thresholds for the selected fields.
    Thresholds(Thresholds),
    /// Override the table cell renderer.
    Cell(TableCell),
    /// Replace the value mappings of the selected fields.
    Mappings(Vec<ValueMapping>),
    /// Replace the data links of the selected fields.
    Links(Vec<DataLink>),
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

    /// Matches every field returned by one query.
    #[must_use]
    pub fn by_query(ref_id: impl Into<String>) -> Self {
        Self {
            matcher: OverrideMatcher::QueryRefId(ref_id.into()),
            properties: Vec::new(),
        }
    }

    /// Matches an explicit list of field names.
    #[must_use]
    pub fn by_names(names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            matcher: OverrideMatcher::Names(names.into_iter().map(Into::into).collect()),
            properties: Vec::new(),
        }
    }

    /// Matches every numeric field.
    #[must_use]
    pub fn numeric_fields() -> Self {
        Self {
            matcher: OverrideMatcher::Numeric,
            properties: Vec::new(),
        }
    }

    /// Matches every time field.
    #[must_use]
    pub fn time_fields() -> Self {
        Self {
            matcher: OverrideMatcher::Time,
            properties: Vec::new(),
        }
    }

    /// Appends an override property.
    #[must_use]
    pub fn property(mut self, property: OverrideProperty) -> Self {
        self.properties.push(property);
        self
    }

    /// Sets the field unit.
    #[must_use]
    pub fn unit(self, unit: Unit) -> Self {
        self.property(OverrideProperty::Unit(unit))
    }

    /// Sets the field minimum.
    #[must_use]
    pub fn min(self, min: f64) -> Self {
        self.property(OverrideProperty::Min(min))
    }

    /// Sets the field maximum.
    #[must_use]
    pub fn max(self, max: f64) -> Self {
        self.property(OverrideProperty::Max(max))
    }

    /// Sets the displayed decimal count.
    #[must_use]
    pub fn decimals(self, decimals: u8) -> Self {
        self.property(OverrideProperty::Decimals(decimals))
    }

    /// Sets the display name.
    #[must_use]
    pub fn display_name(self, display_name: impl Into<String>) -> Self {
        self.property(OverrideProperty::DisplayName(display_name.into()))
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

    /// Sets thresholds for the selected fields.
    #[must_use]
    pub fn thresholds(self, thresholds: Thresholds) -> Self {
        self.property(OverrideProperty::Thresholds(thresholds))
    }

    /// Sets the table cell renderer for the selected fields.
    #[must_use]
    pub fn cell(self, cell: impl Into<TableCell>) -> Self {
        self.property(OverrideProperty::Cell(cell.into()))
    }

    /// Sets the table cell renderer using its default options.
    #[must_use]
    pub fn cell_type(self, cell_type: TableCellType) -> Self {
        self.cell(cell_type)
    }

    /// Adds a value mapping for the selected fields.
    ///
    /// Grafana's `mappings` override replaces the default mappings wholesale rather than merging with them. Repeated calls collect into that one property, so the selected fields get exactly the mappings added here.
    #[must_use]
    pub fn mapping(mut self, mapping: ValueMapping) -> Self {
        if let Some(OverrideProperty::Mappings(mappings)) = self
            .properties
            .iter_mut()
            .find(|property| matches!(property, OverrideProperty::Mappings(_)))
        {
            mappings.push(mapping);
            return self;
        }
        self.property(OverrideProperty::Mappings(vec![mapping]))
    }

    /// Adds a data link for the selected fields.
    ///
    /// Grafana's `links` override replaces the default links wholesale rather than merging with them. Repeated calls collect into that one property, so the selected fields get exactly the links added here.
    #[must_use]
    pub fn data_link(mut self, link: DataLink) -> Self {
        if let Some(OverrideProperty::Links(links)) = self
            .properties
            .iter_mut()
            .find(|property| matches!(property, OverrideProperty::Links(_)))
        {
            links.push(link);
            return self;
        }
        self.property(OverrideProperty::Links(vec![link]))
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

/// Common legend settings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Legend {
    pub(crate) mode: LegendMode,
    pub(crate) placement: LegendPlacement,
    pub(crate) calculations: Vec<Reducer>,
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
    pub fn calculations(mut self, calculations: impl IntoIterator<Item = Reducer>) -> Self {
        self.calculations = calculations.into_iter().collect();
        self
    }
}
