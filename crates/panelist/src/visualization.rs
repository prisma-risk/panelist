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

/// Sparkline rendering mode for a stat panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatGraphMode {
    /// Render an area sparkline.
    #[default]
    Area,
    /// Hide the sparkline.
    None,
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

/// Stacking configuration for compatible visualizations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stacking {
    pub(crate) mode: StackingMode,
    pub(crate) group: String,
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

/// Typed tooltip configuration for compatible panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Tooltip {
    pub(crate) mode: TooltipMode,
    pub(crate) sort: TooltipSort,
    pub(crate) hide_zeros: bool,
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
}

/// Reduction applied when a visualization displays one value per field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReduceOptions {
    pub(crate) values: bool,
    pub(crate) calculations: Vec<Reducer>,
    pub(crate) fields: String,
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

/// Typed authoring state for the Grafana stat panel.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct StatOptions {
    pub(crate) color_mode: Option<StatColorMode>,
    pub(crate) graph_mode: Option<StatGraphMode>,
    pub(crate) orientation: Option<Orientation>,
    pub(crate) wide_layout: Option<bool>,
    pub(crate) reduce: Option<ReduceOptions>,
}

/// Typed authoring state for the Grafana gauge panel.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct GaugeOptions {
    pub(crate) orientation: Option<Orientation>,
    pub(crate) reduce: Option<ReduceOptions>,
}

/// Typed authoring state for the Grafana time series panel.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct TimeseriesOptions {
    pub(crate) fill_opacity: Option<f64>,
    pub(crate) line_width: Option<f64>,
    pub(crate) point_size: Option<f64>,
    pub(crate) line_interpolation: Option<LineInterpolation>,
    pub(crate) show_points: Option<PointVisibility>,
    pub(crate) span_nulls: Option<bool>,
    pub(crate) stacking: Option<Stacking>,
    pub(crate) tooltip: Option<Tooltip>,
}

/// Typed authoring state for the Grafana bar-gauge panel.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct BarGaugeOptions {
    pub(crate) display_mode: Option<BarGaugeDisplayMode>,
    pub(crate) orientation: Option<Orientation>,
    pub(crate) reduce: Option<ReduceOptions>,
}
