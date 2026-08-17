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

use std::{collections::BTreeMap, marker::PhantomData};

use serde_json::Value;

use crate::{
    ColorScheme, DashboardLink, DataSource, FieldConfig, FieldOverride, Legend, Query, Thresholds,
    Unit,
};

/// An explicit position in Grafana's 24-column dashboard grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPos {
    /// Horizontal offset, between 0 and 23.
    pub x: u16,
    /// Vertical offset in 30-pixel grid units.
    pub y: u16,
    /// Width, between 1 and 24.
    pub width: u16,
    /// Height in 30-pixel grid units.
    pub height: u16,
}

impl GridPos {
    /// Creates an explicit Grafana grid position.
    #[must_use]
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Supported panel visualizations plus a plugin escape hatch.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PanelKind {
    /// Grafana time series panel.
    Timeseries,
    /// Grafana stat panel.
    Stat,
    /// Grafana gauge panel.
    Gauge,
    /// Grafana table panel.
    Table,
    /// Grafana text panel.
    Text,
    /// Grafana bar gauge panel.
    BarGauge,
    /// Grafana heatmap panel.
    Heatmap,
    /// A panel plugin ID not modeled by Panelist.
    Raw(String),
}

impl PanelKind {
    pub(crate) fn plugin_id(&self) -> &str {
        match self {
            Self::Timeseries => "timeseries",
            Self::Stat => "stat",
            Self::Gauge => "gauge",
            Self::Table => "table",
            Self::Text => "text",
            Self::BarGauge => "bargauge",
            Self::Heatmap => "heatmap",
            Self::Raw(plugin_id) => plugin_id,
        }
    }
}

/// Markdown, HTML, or code mode for a text panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextMode {
    /// Render Markdown.
    #[default]
    Markdown,
    /// Render sanitized HTML according to Grafana configuration.
    Html,
    /// Render plain code/text.
    Code,
}

impl TextMode {
    pub(crate) fn as_grafana(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Html => "html",
            Self::Code => "code",
        }
    }
}

/// The semantic panel model produced by typed builders and macros.
#[derive(Debug, Clone, PartialEq)]
pub struct Panel {
    pub(crate) kind: PanelKind,
    pub(crate) title: String,
    pub(crate) description: Option<String>,
    pub(crate) id: Option<u32>,
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) grid_pos: Option<GridPos>,
    pub(crate) datasource: Option<DataSource>,
    pub(crate) queries: Vec<Query>,
    pub(crate) field_config: FieldConfig,
    pub(crate) legend: Option<Legend>,
    pub(crate) text: Option<(TextMode, String)>,
    pub(crate) links: Vec<DashboardLink>,
    pub(crate) transparent: bool,
    pub(crate) options: BTreeMap<String, Value>,
    pub(crate) extra: BTreeMap<String, Value>,
}

impl Panel {
    fn new(kind: PanelKind, title: impl Into<String>, width: u16, height: u16) -> Self {
        Self {
            kind,
            title: title.into(),
            description: None,
            id: None,
            width,
            height,
            grid_pos: None,
            datasource: None,
            queries: Vec::new(),
            field_config: FieldConfig::default(),
            legend: None,
            text: None,
            links: Vec::new(),
            transparent: false,
            options: BTreeMap::new(),
            extra: BTreeMap::new(),
        }
    }

    /// Returns the panel title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the authored panel kind.
    #[must_use]
    pub fn kind(&self) -> &PanelKind {
        &self.kind
    }
}

/// Marker trait implemented by typed panel builders.
#[doc(hidden)]
pub trait PanelType {
    /// Grafana panel kind.
    const KIND: PanelKindTag;
    /// Default width.
    const WIDTH: u16;
    /// Default height.
    const HEIGHT: u16;
}

/// Copyable internal counterpart of [`PanelKind`].
#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub enum PanelKindTag {
    Timeseries,
    Stat,
    Gauge,
    Table,
    Text,
    BarGauge,
    Heatmap,
}

impl From<PanelKindTag> for PanelKind {
    fn from(value: PanelKindTag) -> Self {
        match value {
            PanelKindTag::Timeseries => Self::Timeseries,
            PanelKindTag::Stat => Self::Stat,
            PanelKindTag::Gauge => Self::Gauge,
            PanelKindTag::Table => Self::Table,
            PanelKindTag::Text => Self::Text,
            PanelKindTag::BarGauge => Self::BarGauge,
            PanelKindTag::Heatmap => Self::Heatmap,
        }
    }
}

macro_rules! panel_markers {
    ($(($marker:ident, $alias:ident, $kind:ident, $width:literal, $height:literal, $doc:literal)),+ $(,)?) => {
        $(
            #[doc = $doc]
            #[derive(Debug, Clone, Copy)]
            pub struct $marker;

            impl PanelType for $marker {
                const KIND: PanelKindTag = PanelKindTag::$kind;
                const WIDTH: u16 = $width;
                const HEIGHT: u16 = $height;
            }

            #[doc = $doc]
            pub type $alias = PanelBuilder<$marker>;
        )+
    };
}

panel_markers!(
    (
        TimeseriesKind,
        Timeseries,
        Timeseries,
        12,
        8,
        "A typed time series panel builder."
    ),
    (StatKind, Stat, Stat, 6, 6, "A typed stat panel builder."),
    (
        GaugeKind,
        Gauge,
        Gauge,
        6,
        8,
        "A typed gauge panel builder."
    ),
    (
        TableKind,
        Table,
        Table,
        12,
        8,
        "A typed table panel builder."
    ),
    (TextKind, Text, Text, 24, 6, "A typed text panel builder."),
    (
        BarGaugeKind,
        BarGauge,
        BarGauge,
        12,
        8,
        "A typed bar gauge panel builder."
    ),
    (
        HeatmapKind,
        Heatmap,
        Heatmap,
        12,
        8,
        "A typed heatmap panel builder."
    ),
);

/// A lightweight handwritten builder for one statically known panel type.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelBuilder<K: PanelType> {
    panel: Panel,
    marker: PhantomData<K>,
}

impl<K: PanelType> PanelBuilder<K> {
    /// Creates a panel with sensible type-specific dimensions.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            panel: Panel::new(K::KIND.into(), title, K::WIDTH, K::HEIGHT),
            marker: PhantomData,
        }
    }

    /// Adds a datasource query.
    #[must_use]
    pub fn query(mut self, query: impl Into<Query>) -> Self {
        self.panel.queries.push(query.into());
        self
    }

    /// Sets the legend format on the most recently added query.
    ///
    /// This convenience makes the macro DSL read naturally. Builder-oriented
    /// code can set the same value directly on [`crate::PrometheusQuery`] or
    /// [`crate::LokiQuery`]. Calling it before adding a query is a no-op.
    #[must_use]
    pub fn legend(mut self, legend: impl Into<String>) -> Self {
        if let Some(query) = self.panel.queries.last_mut() {
            query.options_mut().legend = Some(legend.into());
        }
        self
    }

    /// Sets the panel description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.panel.description = Some(description.into());
        self
    }

    /// Sets a deterministic explicit panel ID escape hatch.
    #[must_use]
    pub fn id(mut self, id: u32) -> Self {
        self.panel.id = Some(id);
        self
    }

    /// Sets panel width in Grafana's 24-column grid.
    #[must_use]
    pub fn width(mut self, width: u16) -> Self {
        self.panel.width = width;
        self
    }

    /// Sets panel height in Grafana's 30-pixel grid units.
    #[must_use]
    pub fn height(mut self, height: u16) -> Self {
        self.panel.height = height;
        self
    }

    /// Bypasses automatic placement for this panel.
    #[must_use]
    pub fn grid_pos(mut self, grid_pos: GridPos) -> Self {
        self.panel.width = grid_pos.width;
        self.panel.height = grid_pos.height;
        self.panel.grid_pos = Some(grid_pos);
        self
    }

    /// Sets the panel-level datasource.
    #[must_use]
    pub fn datasource(mut self, datasource: DataSource) -> Self {
        self.panel.datasource = Some(datasource);
        self
    }

    /// Replaces the complete typed field configuration.
    #[must_use]
    pub fn field_config(mut self, field_config: FieldConfig) -> Self {
        self.panel.field_config = field_config;
        self
    }

    /// Sets the default field unit.
    #[must_use]
    pub fn unit(mut self, unit: Unit) -> Self {
        self.panel.field_config.unit = Some(unit);
        self
    }

    /// Sets the default field minimum.
    #[must_use]
    pub fn min(mut self, min: f64) -> Self {
        self.panel.field_config.min = Some(min);
        self
    }

    /// Sets the default field maximum.
    #[must_use]
    pub fn max(mut self, max: f64) -> Self {
        self.panel.field_config.max = Some(max);
        self
    }

    /// Sets the default field decimal count.
    #[must_use]
    pub fn decimals(mut self, decimals: u8) -> Self {
        self.panel.field_config.decimals = Some(decimals);
        self
    }

    /// Sets the default display-name template.
    #[must_use]
    pub fn display_name(mut self, display_name: impl Into<String>) -> Self {
        self.panel.field_config.display_name = Some(display_name.into());
        self
    }

    /// Sets the default color scheme.
    #[must_use]
    pub fn color(mut self, color: ColorScheme) -> Self {
        self.panel.field_config.color = Some(color);
        self
    }

    /// Sets thresholds.
    #[must_use]
    pub fn thresholds(mut self, thresholds: Thresholds) -> Self {
        self.panel.field_config.thresholds = Some(thresholds);
        self
    }

    /// Sets visualization legend options.
    #[must_use]
    pub fn legend_options(mut self, legend: Legend) -> Self {
        self.panel.legend = Some(legend);
        self
    }

    /// Adds a field override.
    #[must_use]
    pub fn override_field(mut self, field_override: FieldOverride) -> Self {
        self.panel.field_config.overrides.push(field_override);
        self
    }

    /// Adds a panel link.
    #[must_use]
    pub fn link(mut self, link: DashboardLink) -> Self {
        self.panel.links.push(link);
        self
    }

    /// Makes the panel background transparent.
    #[must_use]
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.panel.transparent = transparent;
        self
    }

    /// Adds a plugin-specific value under the panel's `options` object.
    #[must_use]
    pub fn option(mut self, key: impl Into<String>, value: Value) -> Self {
        self.panel.options.insert(key.into(), value);
        self
    }

    /// Adds an unsupported top-level Grafana panel property.
    #[must_use]
    pub fn extra(mut self, key: impl Into<String>, value: Value) -> Self {
        self.panel.extra.insert(key.into(), value);
        self
    }

    /// Finishes the typed builder.
    #[must_use]
    pub fn build(self) -> Panel {
        self.panel
    }
}

impl PanelBuilder<TextKind> {
    /// Sets the text panel content.
    #[must_use]
    pub fn content(mut self, content: impl Into<String>) -> Self {
        let mode = self
            .panel
            .text
            .as_ref()
            .map_or(TextMode::Markdown, |(mode, _)| *mode);
        self.panel.text = Some((mode, content.into()));
        self
    }

    /// Sets Markdown, HTML, or code rendering mode.
    #[must_use]
    pub fn mode(mut self, mode: TextMode) -> Self {
        let content = self
            .panel
            .text
            .take()
            .map_or_else(String::new, |(_, content)| content);
        self.panel.text = Some((mode, content));
        self
    }
}

impl<K: PanelType> From<PanelBuilder<K>> for Panel {
    fn from(value: PanelBuilder<K>) -> Self {
        value.build()
    }
}

/// A panel-plugin escape hatch that retains the common typed builder surface.
#[derive(Debug, Clone, PartialEq)]
pub struct RawPanel {
    panel: Panel,
}

impl RawPanel {
    /// Creates a panel for an arbitrary Grafana panel plugin ID.
    #[must_use]
    pub fn new(plugin_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            panel: Panel::new(PanelKind::Raw(plugin_id.into()), title, 12, 8),
        }
    }

    /// Adds a query.
    #[must_use]
    pub fn query(mut self, query: impl Into<Query>) -> Self {
        self.panel.queries.push(query.into());
        self
    }

    /// Sets the panel description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.panel.description = Some(description.into());
        self
    }

    /// Sets an explicit panel ID.
    #[must_use]
    pub fn id(mut self, id: u32) -> Self {
        self.panel.id = Some(id);
        self
    }

    /// Sets width.
    #[must_use]
    pub fn width(mut self, width: u16) -> Self {
        self.panel.width = width;
        self
    }

    /// Sets height.
    #[must_use]
    pub fn height(mut self, height: u16) -> Self {
        self.panel.height = height;
        self
    }

    /// Sets an explicit grid position.
    #[must_use]
    pub fn grid_pos(mut self, grid_pos: GridPos) -> Self {
        self.panel.width = grid_pos.width;
        self.panel.height = grid_pos.height;
        self.panel.grid_pos = Some(grid_pos);
        self
    }

    /// Sets the panel-level datasource.
    #[must_use]
    pub fn datasource(mut self, datasource: DataSource) -> Self {
        self.panel.datasource = Some(datasource);
        self
    }

    /// Replaces the complete typed field configuration.
    #[must_use]
    pub fn field_config(mut self, field_config: FieldConfig) -> Self {
        self.panel.field_config = field_config;
        self
    }

    /// Adds a panel link.
    #[must_use]
    pub fn link(mut self, link: DashboardLink) -> Self {
        self.panel.links.push(link);
        self
    }

    /// Makes the panel background transparent.
    #[must_use]
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.panel.transparent = transparent;
        self
    }

    /// Adds a plugin option.
    #[must_use]
    pub fn option(mut self, key: impl Into<String>, value: Value) -> Self {
        self.panel.options.insert(key.into(), value);
        self
    }

    /// Adds an unsupported top-level property.
    #[must_use]
    pub fn extra(mut self, key: impl Into<String>, value: Value) -> Self {
        self.panel.extra.insert(key.into(), value);
        self
    }
}

impl From<RawPanel> for Panel {
    fn from(value: RawPanel) -> Self {
        value.panel
    }
}
