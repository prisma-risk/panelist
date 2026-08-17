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

use serde_json::{Value, json};

use crate::{
    BarGaugeDisplayMode, DataSource, GridPos, Legend, LegendMode, LineInterpolation, Orientation,
    Panel, PanelKind, PointVisibility, ReduceOptions, Row, Stacking, StackingMode, StatColorMode,
    StatGraphMode, TextMode, Tooltip, TooltipMode, TooltipSort, panel::PanelOptions,
};

use super::field::normalize_field_config;
use super::query::normalize_targets;
use super::wire::{GrafanaGridPos, GrafanaLink, GrafanaPanel};

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

pub(crate) fn stacking_value(stacking: &Stacking) -> Value {
    json!({"mode": stacking_mode(stacking.mode), "group": stacking.group})
}

pub(crate) fn tooltip_value(tooltip: Tooltip) -> Value {
    json!({
        "mode": tooltip_mode(tooltip.mode),
        "sort": tooltip_sort(tooltip.sort),
        "hideZeros": tooltip.hide_zeros,
    })
}

pub(crate) fn reduce_options_value(options: &ReduceOptions) -> Value {
    json!({
        "values": options.values,
        "calcs": options
            .calculations
            .iter()
            .map(|calculation| crate::grafana::field::reducer(*calculation))
            .collect::<Vec<_>>(),
        "fields": options.fields,
    })
}

pub(crate) fn normalize_row(row: &Row, id: u32, y: u16, panels: Vec<GrafanaPanel>) -> GrafanaPanel {
    GrafanaPanel {
        id,
        kind: "row".to_owned(),
        title: row.title.clone(),
        description: None,
        grid_pos: GrafanaGridPos {
            x: 0,
            y,
            w: 24,
            h: 1,
        },
        datasource: None,
        targets: Vec::new(),
        field_config: None,
        options: None,
        transparent: false,
        links: Vec::new(),
        collapsed: Some(row.collapsed),
        panels,
        extra: BTreeMap::new(),
    }
}

pub(crate) fn normalize_panel(
    panel: &Panel,
    id: u32,
    grid: GridPos,
    default_datasource: Option<&DataSource>,
) -> GrafanaPanel {
    let datasource = panel.datasource.as_ref().or(default_datasource).cloned();
    let targets = normalize_targets(&panel.queries, datasource.as_ref());
    let mut options = default_panel_options(panel);
    options.extend(typed_panel_options(&panel.kind_options));
    options.extend(panel.raw_options.clone());

    GrafanaPanel {
        id,
        kind: plugin_id(&panel.kind).to_owned(),
        title: panel.title.clone(),
        description: panel.description.clone(),
        grid_pos: grid.into(),
        datasource,
        targets,
        field_config: Some(normalize_field_config(
            &panel.field_config,
            &panel.kind,
            &panel.kind_options,
        )),
        options: Some(options),
        transparent: panel.transparent,
        links: panel.links.iter().map(GrafanaLink::from).collect(),
        collapsed: None,
        panels: Vec::new(),
        extra: panel.extra.clone(),
    }
}

pub(crate) fn default_panel_options(panel: &Panel) -> BTreeMap<String, Value> {
    let mut options = BTreeMap::new();
    match &panel.kind {
        PanelKind::Timeseries => {
            options.insert("legend".to_owned(), legend_value(panel.legend.as_ref()));
            options.insert(
                "tooltip".to_owned(),
                json!({"mode": "single", "sort": "none", "hideZeros": false}),
            );
        }
        PanelKind::Stat => {
            options.insert("colorMode".to_owned(), json!("value"));
            options.insert("graphMode".to_owned(), json!("area"));
            options.insert("justifyMode".to_owned(), json!("auto"));
            options.insert("orientation".to_owned(), json!("auto"));
            options.insert(
                "reduceOptions".to_owned(),
                json!({"values": false, "calcs": ["lastNotNull"], "fields": ""}),
            );
            options.insert("textMode".to_owned(), json!("auto"));
        }
        PanelKind::Gauge => {
            options.insert("orientation".to_owned(), json!("auto"));
            options.insert(
                "reduceOptions".to_owned(),
                json!({"values": false, "calcs": ["lastNotNull"], "fields": ""}),
            );
            options.insert("showThresholdLabels".to_owned(), json!(false));
            options.insert("showThresholdMarkers".to_owned(), json!(true));
        }
        PanelKind::Table => {
            options.insert("cellHeight".to_owned(), json!("sm"));
            options.insert("showHeader".to_owned(), json!(true));
        }
        PanelKind::Text => {
            let (mode, content) = panel
                .text
                .as_ref()
                .map_or((TextMode::Markdown, ""), |(mode, content)| {
                    (*mode, content.as_str())
                });
            options.insert("content".to_owned(), json!(content));
            options.insert("mode".to_owned(), json!(text_mode(mode)));
        }
        PanelKind::BarGauge => {
            options.insert("displayMode".to_owned(), json!("gradient"));
            options.insert("orientation".to_owned(), json!("horizontal"));
            options.insert(
                "reduceOptions".to_owned(),
                json!({"values": false, "calcs": ["lastNotNull"], "fields": ""}),
            );
            options.insert("showUnfilled".to_owned(), json!(true));
        }
        PanelKind::Heatmap => {
            options.insert("calculate".to_owned(), json!(false));
            options.insert("cellGap".to_owned(), json!(1));
            options.insert(
                "color".to_owned(),
                json!({"mode": "scheme", "scheme": "Oranges", "steps": 64}),
            );
            options.insert("legend".to_owned(), json!({"show": true}));
            options.insert("yAxis".to_owned(), json!({"axisPlacement": "left"}));
        }
        PanelKind::Raw(_) => {}
    }
    options
}

pub(crate) fn typed_panel_options(options: &PanelOptions) -> BTreeMap<String, Value> {
    let mut output = BTreeMap::new();
    match options {
        PanelOptions::None | PanelOptions::Table(_) | PanelOptions::Heatmap(_) => {}
        PanelOptions::Stat(stat) => {
            if let Some(mode) = stat.color_mode {
                output.insert("colorMode".to_owned(), json!(stat_color_mode(mode)));
            }
            if let Some(mode) = stat.graph_mode {
                output.insert("graphMode".to_owned(), json!(stat_graph_mode(mode)));
            }
            if let Some(value) = stat.orientation {
                output.insert("orientation".to_owned(), json!(orientation(value)));
            }
            if let Some(wide_layout) = stat.wide_layout {
                output.insert("wideLayout".to_owned(), json!(wide_layout));
            }
            if let Some(reduce) = &stat.reduce {
                output.insert("reduceOptions".to_owned(), reduce_options_value(reduce));
            }
        }
        PanelOptions::Gauge(gauge) => {
            if let Some(value) = gauge.orientation {
                output.insert("orientation".to_owned(), json!(orientation(value)));
            }
            if let Some(reduce) = &gauge.reduce {
                output.insert("reduceOptions".to_owned(), reduce_options_value(reduce));
            }
        }
        PanelOptions::BarGauge(bar_gauge) => {
            if let Some(mode) = bar_gauge.display_mode {
                output.insert(
                    "displayMode".to_owned(),
                    json!(bar_gauge_display_mode(mode)),
                );
            }
            if let Some(value) = bar_gauge.orientation {
                output.insert("orientation".to_owned(), json!(orientation(value)));
            }
            if let Some(reduce) = &bar_gauge.reduce {
                output.insert("reduceOptions".to_owned(), reduce_options_value(reduce));
            }
        }
        PanelOptions::Timeseries(timeseries) => {
            if let Some(tooltip) = timeseries.tooltip {
                output.insert("tooltip".to_owned(), tooltip_value(tooltip));
            }
        }
    }
    output
}

pub(crate) fn typed_field_custom(options: &PanelOptions) -> BTreeMap<String, Value> {
    let mut output = BTreeMap::new();
    let PanelOptions::Timeseries(timeseries) = options else {
        return output;
    };
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
    output
}

pub(crate) fn legend_value(legend: Option<&Legend>) -> Value {
    let legend = legend.cloned().unwrap_or_default();
    let (show, display_mode) = match legend.mode {
        LegendMode::List => (true, "list"),
        LegendMode::Table => (true, "table"),
        LegendMode::Hidden => (false, "list"),
    };
    json!({
        "showLegend": show,
        "displayMode": display_mode,
        "placement": match legend.placement {
            crate::LegendPlacement::Bottom => "bottom",
            crate::LegendPlacement::Right => "right",
        },
        "calcs": legend
            .calculations
            .into_iter()
            .map(|calculation| crate::grafana::field::reducer(calculation))
            .collect::<Vec<_>>(),
    })
}
