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

use crate::{BarGaugeDisplayMode, SortDirection};

/// A Grafana table cell rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum TableCellType {
    /// Let Grafana choose a renderer from the field type.
    #[default]
    Auto,
    /// Color the text using the field color or thresholds.
    ColoredText,
    /// Color the cell background using the field color or thresholds.
    ColoredBackground,
    /// Render an inline bar gauge.
    Gauge,
    /// Render an inline sparkline from a trend field.
    Sparkline,
}

/// Background fill style for a colored-background cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellBackgroundMode {
    /// Solid fill.
    #[default]
    Basic,
    /// Gradient fill.
    Gradient,
}

/// How an inline gauge cell renders its numeric value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellValueDisplay {
    /// Plain text value.
    #[default]
    Text,
    /// Value colored by the field color.
    Color,
    /// No value, gauge only.
    Hidden,
}

/// Options for a colored-background table cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ColoredBackgroundCell {
    pub(crate) mode: Option<CellBackgroundMode>,
    pub(crate) apply_to_row: Option<bool>,
    pub(crate) wrap_text: Option<bool>,
}

impl ColoredBackgroundCell {
    /// Creates a colored-background cell using Grafana's defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets solid or gradient fill.
    #[must_use]
    pub fn mode(mut self, mode: CellBackgroundMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Applies the cell background color to the entire row.
    #[must_use]
    pub fn apply_to_row(mut self, apply_to_row: bool) -> Self {
        self.apply_to_row = Some(apply_to_row);
        self
    }

    /// Wraps long text inside the cell.
    #[must_use]
    pub fn wrap_text(mut self, wrap_text: bool) -> Self {
        self.wrap_text = Some(wrap_text);
        self
    }
}

/// Options for an inline bar gauge table cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GaugeCell {
    pub(crate) mode: Option<BarGaugeDisplayMode>,
    pub(crate) value_display: Option<CellValueDisplay>,
}

impl GaugeCell {
    /// Creates an inline gauge cell using Grafana's defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the bar's visual fill style.
    #[must_use]
    pub fn mode(mut self, mode: BarGaugeDisplayMode) -> Self {
        self.mode = Some(mode);
        self
    }

    /// Sets how the numeric value renders alongside the bar.
    #[must_use]
    pub fn value_display(mut self, value_display: CellValueDisplay) -> Self {
        self.value_display = Some(value_display);
        self
    }
}

/// Options for an inline sparkline table cell.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SparklineCell {
    pub(crate) hide_value: Option<bool>,
    pub(crate) line_width: Option<f64>,
    pub(crate) fill_opacity: Option<f64>,
}

impl SparklineCell {
    /// Creates an inline sparkline cell using Grafana's defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Hides the numeric value beside the sparkline.
    #[must_use]
    pub fn hide_value(mut self, hide_value: bool) -> Self {
        self.hide_value = Some(hide_value);
        self
    }

    /// Sets the sparkline stroke width in pixels.
    #[must_use]
    pub fn line_width(mut self, line_width: f64) -> Self {
        self.line_width = Some(line_width);
        self
    }

    /// Sets the sparkline area fill opacity from 0 to 100.
    #[must_use]
    pub fn fill_opacity(mut self, fill_opacity: f64) -> Self {
        self.fill_opacity = Some(fill_opacity);
        self
    }
}

/// A typed Grafana table cell configuration.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub enum TableCell {
    /// Automatic renderer.
    #[default]
    Auto,
    /// Colored text.
    ColoredText,
    /// Colored background with options.
    ColoredBackground(ColoredBackgroundCell),
    /// Inline gauge with options.
    Gauge(GaugeCell),
    /// Inline sparkline with options.
    Sparkline(SparklineCell),
}

impl From<TableCellType> for TableCell {
    fn from(value: TableCellType) -> Self {
        match value {
            TableCellType::Auto => Self::Auto,
            TableCellType::ColoredText => Self::ColoredText,
            TableCellType::ColoredBackground => {
                Self::ColoredBackground(ColoredBackgroundCell::default())
            }
            TableCellType::Gauge => Self::Gauge(GaugeCell::default()),
            TableCellType::Sparkline => Self::Sparkline(SparklineCell::default()),
        }
    }
}

impl From<ColoredBackgroundCell> for TableCell {
    fn from(value: ColoredBackgroundCell) -> Self {
        Self::ColoredBackground(value)
    }
}

impl From<GaugeCell> for TableCell {
    fn from(value: GaugeCell) -> Self {
        Self::Gauge(value)
    }
}

impl From<SparklineCell> for TableCell {
    fn from(value: SparklineCell) -> Self {
        Self::Sparkline(value)
    }
}

/// One field in a table panel's initial sort order.
///
/// This is the panel's own `options.sortBy` state, which Grafana keys on
/// the field's *display name*. It is distinct from the
/// [`crate::SortBy`] transformation, which reorders the underlying data
/// and keys on the raw field name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableSort {
    pub(crate) field: String,
    pub(crate) descending: bool,
}

impl TableSort {
    /// Sorts a table column in the given direction.
    #[must_use]
    pub fn new(field: impl Into<String>, direction: SortDirection) -> Self {
        Self {
            field: field.into(),
            descending: direction.is_descending(),
        }
    }
}

/// Typed authoring state for the Grafana table panel.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct TableOptions {
    pub(crate) sort_by: Vec<TableSort>,
    pub(crate) cell: Option<TableCell>,
}
