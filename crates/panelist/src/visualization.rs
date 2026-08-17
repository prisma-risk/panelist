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

use serde_json::{Value, json};

/// A Grafana reduction applied to a field's values.
///
/// Grafana calls this vocabulary `ReducerID`. It is used by table legends,
/// single-value visualizations, and the time-series-to-table transformation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Reducer {
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

impl Reducer {
    pub(crate) const fn as_grafana(self) -> &'static str {
        match self {
            Self::Last => "lastNotNull",
            Self::Min => "min",
            Self::Max => "max",
            Self::Mean => "mean",
            Self::Total => "sum",
        }
    }
}

/// Orientation used by stat, gauge, and bar-gauge visualizations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    /// Let Grafana select an orientation from the panel dimensions.
    #[default]
    Auto,
    /// Arrange values horizontally.
    Horizontal,
    /// Arrange values vertically.
    Vertical,
}

impl Orientation {
    pub(crate) const fn as_grafana(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }
}

/// How a stat panel colors its value and background.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatColorMode {
    /// Color the rendered value.
    #[default]
    Value,
    /// Color the panel background.
    Background,
    /// Do not apply field colors.
    None,
}

impl StatColorMode {
    pub(crate) const fn as_grafana(self) -> &'static str {
        match self {
            Self::Value => "value",
            Self::Background => "background",
            Self::None => "none",
        }
    }
}

/// Sparkline rendering mode for a stat panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatGraphMode {
    /// Render an area sparkline.
    #[default]
    Area,
    /// Hide the sparkline.
    None,
}

impl StatGraphMode {
    pub(crate) const fn as_grafana(self) -> &'static str {
        match self {
            Self::Area => "area",
            Self::None => "none",
        }
    }
}

/// Line interpolation used by a time-series panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineInterpolation {
    /// Connect points with straight lines.
    #[default]
    Linear,
    /// Smooth the line between points.
    Smooth,
    /// Step immediately before each point.
    StepBefore,
    /// Step immediately after each point.
    StepAfter,
}

impl LineInterpolation {
    pub(crate) const fn as_grafana(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Smooth => "smooth",
            Self::StepBefore => "stepBefore",
            Self::StepAfter => "stepAfter",
        }
    }
}

/// Point marker visibility for time-series panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PointVisibility {
    /// Let Grafana decide based on the data and panel size.
    #[default]
    Auto,
    /// Always show point markers.
    Always,
    /// Never show point markers.
    Never,
}

impl PointVisibility {
    pub(crate) const fn as_grafana(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Always => "always",
            Self::Never => "never",
        }
    }
}

/// Time-series stacking mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackingMode {
    /// Do not stack series.
    #[default]
    None,
    /// Stack raw values.
    Normal,
    /// Stack series as percentages.
    Percent,
}

impl StackingMode {
    pub(crate) const fn as_grafana(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Normal => "normal",
            Self::Percent => "percent",
        }
    }
}

/// Stacking configuration for compatible visualizations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stacking {
    mode: StackingMode,
    group: String,
}

impl Stacking {
    /// Creates stacking in Grafana's default group `A`.
    #[must_use]
    pub fn new(mode: StackingMode) -> Self {
        Self {
            mode,
            group: "A".to_owned(),
        }
    }

    /// Sets the stacking group shared by related series.
    #[must_use]
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = group.into();
        self
    }

    pub(crate) fn as_grafana(&self) -> Value {
        json!({"mode": self.mode.as_grafana(), "group": self.group})
    }
}

/// Tooltip display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TooltipMode {
    /// Show the hovered series.
    #[default]
    Single,
    /// Show every series at the hovered timestamp.
    Multi,
    /// Hide the tooltip.
    None,
}

impl TooltipMode {
    pub(crate) const fn as_grafana(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Multi => "multi",
            Self::None => "none",
        }
    }
}

/// Ordering of series inside a visualization tooltip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TooltipSort {
    /// Keep Grafana's natural order.
    #[default]
    None,
    /// Sort values from smallest to largest.
    Ascending,
    /// Sort values from largest to smallest.
    Descending,
}

impl TooltipSort {
    pub(crate) const fn as_grafana(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Ascending => "asc",
            Self::Descending => "desc",
        }
    }
}

/// Typed tooltip configuration for compatible panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Tooltip {
    mode: TooltipMode,
    sort: TooltipSort,
    hide_zeros: bool,
}

impl Tooltip {
    /// Creates the default single-series tooltip.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets single-series, multi-series, or hidden rendering.
    #[must_use]
    pub fn mode(mut self, mode: TooltipMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets series ordering.
    #[must_use]
    pub fn sort(mut self, sort: TooltipSort) -> Self {
        self.sort = sort;
        self
    }

    /// Hides series whose value is zero.
    #[must_use]
    pub fn hide_zeros(mut self, hide_zeros: bool) -> Self {
        self.hide_zeros = hide_zeros;
        self
    }

    pub(crate) fn as_grafana(self) -> Value {
        json!({
            "mode": self.mode.as_grafana(),
            "sort": self.sort.as_grafana(),
            "hideZeros": self.hide_zeros,
        })
    }
}

/// Reduction applied when a visualization displays one value per field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReduceOptions {
    values: bool,
    calculations: Vec<Reducer>,
    fields: String,
}

impl ReduceOptions {
    /// Creates last-non-null reduction over all fields.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Displays every row/value instead of reducing each field.
    #[must_use]
    pub fn values(mut self, values: bool) -> Self {
        self.values = values;
        self
    }

    /// Replaces the reduction calculations.
    #[must_use]
    pub fn calculations(mut self, calculations: impl IntoIterator<Item = Reducer>) -> Self {
        self.calculations = calculations.into_iter().collect();
        self
    }

    /// Selects fields using Grafana's field-filter expression.
    #[must_use]
    pub fn fields(mut self, fields: impl Into<String>) -> Self {
        self.fields = fields.into();
        self
    }

    pub(crate) fn as_grafana(&self) -> Value {
        json!({
            "values": self.values,
            "calcs": self
                .calculations
                .iter()
                .map(|calculation| calculation.as_grafana())
                .collect::<Vec<_>>(),
            "fields": self.fields,
        })
    }
}

impl Default for ReduceOptions {
    fn default() -> Self {
        Self {
            values: false,
            calculations: vec![Reducer::Last],
            fields: String::new(),
        }
    }
}

/// Visual fill style for a bar-gauge panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarGaugeDisplayMode {
    /// Solid basic bar.
    Basic,
    /// Gradient bar.
    #[default]
    Gradient,
    /// Segmented LCD-style bar.
    Lcd,
}

impl BarGaugeDisplayMode {
    pub(crate) const fn as_grafana(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Gradient => "gradient",
            Self::Lcd => "lcd",
        }
    }
}
