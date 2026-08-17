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
    AxisPlacement, DataSource, GridPos, HeatmapColorMode, HeatmapColorScheme, Legend, LegendMode,
    LegendPlacement, Panel, PanelKind, ReduceOptions, Row, TextMode, Tooltip, panel::PanelOptions,
};

use super::field::normalize_field_config;
use super::query::normalize_targets;
use super::vocabulary::{
    bar_gauge_display_mode, orientation, plugin_id, reducer, stat_color_mode, stat_graph_mode,
    text_mode, tooltip_mode, tooltip_sort,
};
use super::wire::{GrafanaGridPos, GrafanaLink, GrafanaPanel};

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
            .map(|calculation| reducer(*calculation))
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
        transformations: Vec::new(),
        field_config: None,
        options: None,
        transparent: false,
        links: Vec::new(),
        collapsed: Some(row.collapsed),
        panels,
        extra: BTreeMap::new(),
    }
}

/// Lowers an authored [`crate::Panel`] into wire JSON.
///
/// `options` is assembled in three ordered layers, each free to overwrite a
/// key the previous layer set:
///
/// 1. Kind defaults (`default_panel_options`) — the JSON Grafana expects
///    for this panel kind when no typed setter has been called.
/// 2. Typed options (`typed_panel_options`) — values set through the
///    kind-specific builder methods, e.g. `PanelBuilder::color_mode`.
/// 3. Raw options (`panel.raw_options`) — the `.option()` escape hatch,
///    applied last so it always wins regardless of call order.
///
/// `fieldConfig.defaults.custom` follows the same three-tier ordering
/// through `super::field::normalize_field_config`.
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
        transformations: panel
            .transformations
            .iter()
            .map(super::transform::normalize_transformation)
            .collect(),
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
            // Unconditional: the typed `legend` override below only replaces
            // this when a legend has actually been authored, so a panel
            // that never calls `.legend_options()` must still serialize the
            // same default legend object it always has.
            options.insert("legend".to_owned(), legend_value(None));
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
        PanelOptions::None => {}
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
            if let Some(legend) = &timeseries.legend {
                output.insert("legend".to_owned(), legend_value(Some(legend)));
            }
        }
        PanelOptions::Table(table) => {
            if !table.sort_by.is_empty() {
                output.insert(
                    "sortBy".to_owned(),
                    json!(
                        table
                            .sort_by
                            .iter()
                            .map(|sort| json!({
                                "displayName": sort.field,
                                "desc": sort.descending,
                            }))
                            .collect::<Vec<_>>()
                    ),
                );
            }
        }
        PanelOptions::Heatmap(heatmap) => {
            // `color`, `legend`, and `yAxis` are complete objects in
            // `default_panel_options`. `BTreeMap::extend` replaces whole
            // values rather than merging keys, so each arm below must
            // rebuild the entire object from the same defaults tier 1 uses,
            // filling in only the fields the caller actually set.
            if heatmap.color_scheme.is_some()
                || heatmap.color_steps.is_some()
                || heatmap.color_mode.is_some()
            {
                let mode = heatmap.color_mode.unwrap_or(HeatmapColorMode::Scheme);
                output.insert(
                    "color".to_owned(),
                    json!({
                        "mode": match mode {
                            HeatmapColorMode::Scheme => "scheme",
                            HeatmapColorMode::Opacity => "opacity",
                        },
                        "scheme": heatmap
                            .color_scheme
                            .as_ref()
                            .map_or("Oranges", heatmap_color_scheme),
                        "steps": heatmap.color_steps.unwrap_or(64),
                    }),
                );
            }
            if let Some(gap) = heatmap.cell_gap {
                output.insert("cellGap".to_owned(), json!(gap));
            }
            if let Some(show) = heatmap.legend {
                output.insert("legend".to_owned(), json!({"show": show}));
            }
            if heatmap.y_axis_unit.is_some() || heatmap.y_axis_placement.is_some() {
                let mut axis = serde_json::Map::new();
                axis.insert(
                    "axisPlacement".to_owned(),
                    json!(axis_placement(
                        heatmap.y_axis_placement.unwrap_or(AxisPlacement::Left)
                    )),
                );
                if let Some(unit) = &heatmap.y_axis_unit {
                    axis.insert("unit".to_owned(), json!(super::vocabulary::unit(unit)));
                }
                output.insert("yAxis".to_owned(), Value::Object(axis));
            }
            if let Some(calculate) = heatmap.calculate {
                output.insert("calculate".to_owned(), json!(calculate));
            }
        }
    }
    output
}

fn heatmap_color_scheme(scheme: &HeatmapColorScheme) -> &str {
    match scheme {
        HeatmapColorScheme::Oranges => "Oranges",
        HeatmapColorScheme::Blues => "Blues",
        HeatmapColorScheme::Greens => "Greens",
        HeatmapColorScheme::Reds => "Reds",
        HeatmapColorScheme::Purples => "Purples",
        HeatmapColorScheme::Turbo => "Turbo",
        HeatmapColorScheme::Viridis => "Viridis",
        HeatmapColorScheme::Spectral => "Spectral",
        HeatmapColorScheme::Custom(value) => value,
    }
}

const fn axis_placement(placement: AxisPlacement) -> &'static str {
    match placement {
        AxisPlacement::Auto => "auto",
        AxisPlacement::Bottom => "bottom",
        AxisPlacement::Hidden => "hidden",
        AxisPlacement::Left => "left",
        AxisPlacement::Right => "right",
        AxisPlacement::Top => "top",
    }
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
            LegendPlacement::Bottom => "bottom",
            LegendPlacement::Right => "right",
        },
        "calcs": legend
            .calculations
            .into_iter()
            .map(reducer)
            .collect::<Vec<_>>(),
    })
}
