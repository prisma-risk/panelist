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

use crate::{AxisPlacement, Unit};

/// A named Grafana heatmap color palette.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum HeatmapColorScheme {
    /// Grafana's default heatmap palette.
    #[default]
    Oranges,
    /// Blues palette.
    Blues,
    /// Greens palette.
    Greens,
    /// Reds palette.
    Reds,
    /// Purples palette.
    Purples,
    /// Turbo palette.
    Turbo,
    /// Viridis palette.
    Viridis,
    /// Spectral palette.
    Spectral,
    /// Any other palette Grafana recognizes.
    Custom(String),
}

impl From<&str> for HeatmapColorScheme {
    fn from(value: &str) -> Self {
        match value {
            "Oranges" => Self::Oranges,
            "Blues" => Self::Blues,
            "Greens" => Self::Greens,
            "Reds" => Self::Reds,
            "Purples" => Self::Purples,
            "Turbo" => Self::Turbo,
            "Viridis" => Self::Viridis,
            "Spectral" => Self::Spectral,
            other => Self::Custom(other.to_owned()),
        }
    }
}

impl From<String> for HeatmapColorScheme {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

/// How a heatmap maps values to color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeatmapColorMode {
    /// Use a named palette.
    #[default]
    Scheme,
    /// Vary opacity of one color.
    Opacity,
}

/// Typed authoring state for the Grafana heatmap panel.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct HeatmapOptions {
    pub(crate) color_scheme: Option<HeatmapColorScheme>,
    pub(crate) color_steps: Option<u32>,
    pub(crate) color_mode: Option<HeatmapColorMode>,
    pub(crate) cell_gap: Option<u8>,
    pub(crate) legend: Option<bool>,
    pub(crate) y_axis_unit: Option<Unit>,
    pub(crate) y_axis_placement: Option<AxisPlacement>,
    pub(crate) calculate: Option<bool>,
}
