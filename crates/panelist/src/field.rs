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

use crate::{Reducer, TableCell, TableCellType};

/// A Grafana field unit with common units represented as Rust variants.
///
/// Each variant names the unit ID Grafana stores, taken from `packages/grafana-data/src/valueFormats/categories.ts`. Anything not listed here goes through [`Unit::custom`], which takes either another Grafana unit ID or a literal suffix.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Unit {
    /// Grafana's automatic/default unit behavior.
    #[default]
    None,
    /// Nanoseconds (`ns`).
    Nanoseconds,
    /// Microseconds (`µs`).
    Microseconds,
    /// Milliseconds (`ms`).
    Milliseconds,
    /// Seconds (`s`).
    Seconds,
    /// Minutes (`m`).
    Minutes,
    /// Hours (`h`).
    Hours,
    /// Days (`d`).
    Days,
    /// A duration in seconds rendered as its largest whole units, for example `2 days, 3 hours` (`dtdurations`). Suits uptimes and ages better than [`Unit::Seconds`], which scales to a single unit.
    DurationSeconds,
    /// A duration in milliseconds rendered as its largest whole units (`dtdurationms`).
    DurationMilliseconds,
    /// Bytes using IEC scaling, so 1 KiB is 1024 bytes (`bytes`).
    Bytes,
    /// Bytes per second using SI scaling, so 1 kB/s is 1000 bytes per second (`Bps`).
    BytesPerSecond,
    /// Bits per second using SI scaling, the convention for network throughput (`bps`).
    BitsPerSecond,
    /// Packets per second (`pps`).
    PacketsPerSecond,
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
    /// Reads per second (`rps`). Not requests per second, which is [`Unit::RequestsPerSecond`].
    ReadsPerSecond,
    /// Writes per second (`wps`).
    WritesPerSecond,
    /// I/O operations per second (`iops`).
    IoOperationsPerSecond,
    /// Events per second (`eps`).
    EventsPerSecond,
    /// Compact short-number formatting (`short`).
    Short,
    /// A monetary amount in one currency, for example `currencyUSD`.
    Currency(Currency),
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

impl From<Currency> for Unit {
    fn from(currency: Currency) -> Self {
        Self::Currency(currency)
    }
}

/// A currency Grafana formats amounts in: every entry of Grafana's currency unit category.
///
/// Grafana adds the currency symbol and scales large amounts with `K`, `M`, `B`, and `T`, so `1500000` in [`Currency::Usd`] reads as `$1.5M`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Currency {
    /// US dollars (`currencyUSD`).
    Usd,
    /// Pounds sterling (`currencyGBP`).
    Gbp,
    /// Euros (`currencyEUR`).
    Eur,
    /// Japanese yen (`currencyJPY`).
    Jpy,
    /// Russian rubles (`currencyRUB`).
    Rub,
    /// Ukrainian hryvnias (`currencyUAH`).
    Uah,
    /// Brazilian reais (`currencyBRL`).
    Brl,
    /// Danish kroner (`currencyDKK`).
    Dkk,
    /// Icelandic krónur (`currencyISK`).
    Isk,
    /// Norwegian kroner (`currencyNOK`).
    Nok,
    /// Swedish kronor (`currencySEK`).
    Sek,
    /// Czech koruny (`currencyCZK`).
    Czk,
    /// Swiss francs (`currencyCHF`).
    Chf,
    /// Polish złoty (`currencyPLN`).
    Pln,
    /// Bitcoin (`currencyBTC`).
    Btc,
    /// Millibitcoin (`currencymBTC`).
    MilliBtc,
    /// Microbitcoin (`currencyμBTC`).
    MicroBtc,
    /// South African rand (`currencyZAR`).
    Zar,
    /// Indian rupees (`currencyINR`).
    Inr,
    /// South Korean won (`currencyKRW`).
    Krw,
    /// Indonesian rupiah (`currencyIDR`).
    Idr,
    /// Philippine pesos (`currencyPHP`).
    Php,
    /// Vietnamese dong (`currencyVND`).
    Vnd,
    /// Turkish lira (`currencyTRY`).
    Try,
    /// Malaysian ringgit (`currencyMYR`).
    Myr,
    /// CFP francs (`currencyXPF`).
    Xpf,
    /// Bulgarian leva (`currencyBGN`).
    Bgn,
    /// Paraguayan guaraní (`currencyPYG`).
    Pyg,
    /// Uruguayan pesos (`currencyUYU`).
    Uyu,
    /// Israeli new shekels (`currencyILS`).
    Ils,
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

/// Display text and optional color substituted for one exact field value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueMapping {
    pub(crate) value: String,
    pub(crate) text: String,
    pub(crate) color: Option<Color>,
}

impl ValueMapping {
    /// Maps an exact Grafana field value to display text.
    #[must_use]
    pub fn new(value: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            text: text.into(),
            color: None,
        }
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

/// A color that takes over from one value upward.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ThresholdStep {
    pub(crate) value: f64,
    pub(crate) color: Color,
}

/// Ordered threshold configuration: a base color, then the steps above it.
///
/// The base color applies below the first step, and every value when there are no steps. Grafana requires that base, so every constructor takes it first and a threshold set without one cannot be built.
///
/// ```rust
/// use panelist::prelude::*;
///
/// // Green below 1, yellow from 1, red from 5.
/// let latency = Thresholds::absolute(Color::Green)
///     .step(1.0, Color::Yellow)
///     .step(5.0, Color::Red);
///
/// // Red below 50% of the field's range, green from there.
/// let budget = Thresholds::percentage(Color::Red).step(50.0, Color::Green);
/// # let _ = (latency, budget);
/// ```
///
/// Step values must be finite and strictly increasing; validation reports a set that is not.
#[derive(Debug, Clone, PartialEq)]
pub struct Thresholds {
    pub(crate) mode: ThresholdMode,
    pub(crate) base: Color,
    pub(crate) steps: Vec<ThresholdStep>,
}

impl Thresholds {
    /// Creates thresholds in `mode` with a base color and no steps yet.
    #[must_use]
    pub fn new(mode: ThresholdMode, base: Color) -> Self {
        Self {
            mode,
            base,
            steps: Vec::new(),
        }
    }

    /// Creates thresholds compared against absolute field values.
    #[must_use]
    pub fn absolute(base: Color) -> Self {
        Self::new(ThresholdMode::Absolute, base)
    }

    /// Creates thresholds compared against a percentage of the field's minimum-to-maximum range.
    #[must_use]
    pub fn percentage(base: Color) -> Self {
        Self::new(ThresholdMode::Percentage, base)
    }

    /// Switches to `color` from `value` upward. Steps must be added in increasing order of value.
    #[must_use]
    pub fn step(mut self, value: f64, color: Color) -> Self {
        self.steps.push(ThresholdStep { value, color });
        self
    }
}

/// A typed Grafana field color scheme.
///
/// The two classic palettes differ in how a series picks its color. [`ColorScheme::ClassicPalette`] assigns colors by position, so the third series is always the third color: when a series appears or disappears, every series after it shifts to a new color. [`ColorScheme::ClassicPaletteByName`] derives the color from the series name, so `checkout` keeps its color whatever else the query returns. Prefer by-name when the set of series changes over time or with a variable, for example one series per pod or per `$instance`, and when the same series appears on several panels that should agree. Keep the positional palette for a small fixed set of series, where it spreads colors evenly and by-name could give two series similar hues.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColorScheme {
    /// Use thresholds to determine the color (`thresholds`).
    Thresholds,
    /// Use the classic palette indexed by series position (`palette-classic`).
    ClassicPalette,
    /// Use the classic palette keyed by series name (`palette-classic-by-name`), so a series keeps its color when others come and go.
    ClassicPaletteByName,
    /// Use one fixed color (`fixed`).
    Fixed(Color),
    /// Use shades of one color, one shade per series (`shades`).
    Shades(Color),
    /// Color each value along a continuous gradient between the field's minimum and maximum.
    Continuous(ContinuousPalette),
}

/// A continuous gradient Grafana colors values by.
///
/// The gradient runs from the field's minimum to its maximum, so set [`FieldConfig::min`] and [`FieldConfig::max`] when the data does not span the meaningful range on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContinuousPalette {
    /// Green to yellow to red (`continuous-GrYlRd`). Suits values where higher is worse, such as latency or error rate.
    GreenYellowRed,
    /// Red to yellow to green (`continuous-RdYlGr`). Suits values where higher is better, such as availability.
    RedYellowGreen,
    /// Blue to yellow to red (`continuous-BlYlRd`).
    BlueYellowRed,
    /// Yellow to red (`continuous-YlRd`).
    YellowRed,
    /// Blue to purple (`continuous-BlPu`).
    BluePurple,
    /// Yellow to blue (`continuous-YlBl`).
    YellowBlue,
    /// Shades of blue (`continuous-blues`).
    Blues,
    /// Shades of red (`continuous-reds`).
    Reds,
    /// Shades of green (`continuous-greens`).
    Greens,
    /// Shades of purple (`continuous-purples`).
    Purples,
    /// Viridis, perceptually uniform and color-blind safe (`continuous-viridis`).
    Viridis,
    /// Magma, perceptually uniform (`continuous-magma`).
    Magma,
    /// Plasma, perceptually uniform (`continuous-plasma`).
    Plasma,
    /// Inferno, perceptually uniform (`continuous-inferno`).
    Inferno,
    /// Cividis, perceptually uniform and color-blind safe (`continuous-cividis`).
    Cividis,
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

    /// Adds an exact-value display mapping.
    #[must_use]
    pub fn mapping(mut self, mapping: ValueMapping) -> Self {
        self.mappings.push(mapping);
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
