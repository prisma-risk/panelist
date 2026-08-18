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

use crate::{
    BarGaugeDisplayMode, Color, LineInterpolation, Orientation, PanelKind, PointVisibility,
    Reducer, StackingMode, StatColorMode, StatGraphMode, TextMode, TooltipMode, TooltipSort, Unit,
};

pub(crate) fn unit(unit: &Unit) -> &str {
    match unit {
        Unit::None => "none",
        Unit::Seconds => "s",
        Unit::Milliseconds => "ms",
        Unit::Bytes => "bytes",
        Unit::BytesPerSecond => "Bps",
        Unit::Percent => "percent",
        Unit::PercentUnit => "percentunit",
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

pub(crate) fn plugin_id(kind: &PanelKind) -> &str {
    match kind {
        PanelKind::Timeseries => "timeseries",
        PanelKind::Stat => "stat",
        PanelKind::Gauge => "gauge",
        PanelKind::Table => "table",
        PanelKind::Text => "text",
        PanelKind::BarGauge => "bargauge",
        PanelKind::Heatmap => "heatmap",
        PanelKind::Raw(plugin_id) => plugin_id,
    }
}

pub(crate) const fn text_mode(mode: TextMode) -> &'static str {
    match mode {
        TextMode::Markdown => "markdown",
        TextMode::Html => "html",
        TextMode::Code => "code",
    }
}

pub(crate) const fn orientation(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Auto => "auto",
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    }
}

pub(crate) const fn stat_color_mode(mode: StatColorMode) -> &'static str {
    match mode {
        StatColorMode::Value => "value",
        StatColorMode::Background => "background",
        StatColorMode::None => "none",
    }
}

pub(crate) const fn stat_graph_mode(mode: StatGraphMode) -> &'static str {
    match mode {
        StatGraphMode::Area => "area",
        StatGraphMode::None => "none",
    }
}

pub(crate) const fn line_interpolation(interpolation: LineInterpolation) -> &'static str {
    match interpolation {
        LineInterpolation::Linear => "linear",
        LineInterpolation::Smooth => "smooth",
        LineInterpolation::StepBefore => "stepBefore",
        LineInterpolation::StepAfter => "stepAfter",
    }
}

pub(crate) const fn point_visibility(visibility: PointVisibility) -> &'static str {
    match visibility {
        PointVisibility::Auto => "auto",
        PointVisibility::Always => "always",
        PointVisibility::Never => "never",
    }
}

pub(crate) const fn stacking_mode(mode: StackingMode) -> &'static str {
    match mode {
        StackingMode::None => "none",
        StackingMode::Normal => "normal",
        StackingMode::Percent => "percent",
    }
}

pub(crate) const fn tooltip_mode(mode: TooltipMode) -> &'static str {
    match mode {
        TooltipMode::Single => "single",
        TooltipMode::Multi => "multi",
        TooltipMode::None => "none",
    }
}

pub(crate) const fn tooltip_sort(sort: TooltipSort) -> &'static str {
    match sort {
        TooltipSort::None => "none",
        TooltipSort::Ascending => "asc",
        TooltipSort::Descending => "desc",
    }
}

pub(crate) const fn bar_gauge_display_mode(mode: BarGaugeDisplayMode) -> &'static str {
    match mode {
        BarGaugeDisplayMode::Basic => "basic",
        BarGaugeDisplayMode::Gradient => "gradient",
        BarGaugeDisplayMode::Lcd => "lcd",
    }
}
