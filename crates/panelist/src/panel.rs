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
    AxisPlacement, BarGaugeDisplayMode, ColorScheme, DataLink, DataSource, FieldConfig,
    FieldOverride, HeatmapColorMode, HeatmapColorScheme, Legend, LineInterpolation, Orientation,
    PointVisibility, Query, ReduceOptions, SortDirection, Stacking, StatColorMode, StatGraphMode,
    TableCell, Thresholds, Tooltip, Transformation, Unit, ValidationError, ValueMapping,
    heatmap::HeatmapOptions,
    table::{TableOptions, TableSort},
    visualization::{BarGaugeOptions, GaugeOptions, StatOptions, TimeseriesOptions},
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

/// How Grafana repeats a panel once per selected value of a dashboard variable.
///
/// Grafana expands repeats when the dashboard loads, so Panelist lays out and emits the authored panel once. The two directions treat the panel's size differently, following Grafana 13's `DashboardGridItem`:
///
/// - [`Repeat::horizontal`] always spans the full 24-column row, whatever the panel's width. Grafana splits that row evenly between the clones, at most `max_per_row` per line (4 when unset), and wraps the rest onto further lines of the authored height. Panelist reserves the full row for the panel, so the next panel starts on a new line, just as it does in Grafana. An explicit [`GridPos`] is emitted as written.
/// - [`Repeat::vertical`] keeps the panel's width and stacks the clones below each other.
///
/// Either way, Grafana pushes the panels below a repeat further down as clones appear. The `gridPos` Panelist emits for those panels is their position before expansion.
///
/// ```rust
/// use panelist::prelude::*;
///
/// let per_instance = Timeseries::new("Requests on $instance")
///     .repeat(Repeat::horizontal("instance").max_per_row(3));
/// let stacked = Stat::new("Errors on $instance").repeat(Repeat::vertical("instance"));
/// # let _ = (per_instance, stacked);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Repeat {
    /// Clones side by side across the row.
    Horizontal(HorizontalRepeat),
    /// Clones stacked down the dashboard.
    Vertical(VerticalRepeat),
}

impl Repeat {
    /// Repeats side by side for each value of the named variable.
    ///
    /// The name is the bare variable name, `instance` rather than `$instance`.
    #[must_use]
    pub fn horizontal(variable: impl Into<String>) -> HorizontalRepeat {
        HorizontalRepeat {
            variable: variable.into(),
            max_per_row: None,
        }
    }

    /// Repeats vertically for each value of the named variable.
    ///
    /// The name is the bare variable name, `instance` rather than `$instance`.
    #[must_use]
    pub fn vertical(variable: impl Into<String>) -> VerticalRepeat {
        VerticalRepeat {
            variable: variable.into(),
        }
    }

    /// Returns the name of the variable the panel repeats by.
    #[must_use]
    pub fn variable(&self) -> &str {
        match self {
            Self::Horizontal(repeat) => &repeat.variable,
            Self::Vertical(repeat) => &repeat.variable,
        }
    }
}

/// A horizontal [`Repeat`], the direction Grafana uses by default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HorizontalRepeat {
    pub(crate) variable: String,
    pub(crate) max_per_row: Option<u8>,
}

impl HorizontalRepeat {
    /// Caps how many clones share one line, between 1 and 24. Grafana's editor offers 2, 3, 4, 6, 8, and 12, and defaults to 4.
    #[must_use]
    pub fn max_per_row(mut self, max_per_row: u8) -> Self {
        self.max_per_row = Some(max_per_row);
        self
    }
}

/// A vertical [`Repeat`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerticalRepeat {
    pub(crate) variable: String,
}

impl From<HorizontalRepeat> for Repeat {
    fn from(value: HorizontalRepeat) -> Self {
        Self::Horizontal(value)
    }
}

impl From<VerticalRepeat> for Repeat {
    fn from(value: VerticalRepeat) -> Self {
        Self::Vertical(value)
    }
}

/// A bare variable name repeats horizontally, Grafana's default direction.
impl From<&str> for Repeat {
    fn from(value: &str) -> Self {
        Self::horizontal(value).into()
    }
}

/// A bare variable name repeats horizontally, Grafana's default direction.
impl From<String> for Repeat {
    fn from(value: String) -> Self {
        Self::horizontal(value).into()
    }
}

/// Panel-level query options from Grafana's query options pane.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PanelQueryOptions {
    pub(crate) min_interval: Option<String>,
    pub(crate) max_data_points: Option<u32>,
    pub(crate) time_from: Option<String>,
    pub(crate) time_shift: Option<String>,
    pub(crate) hide_time_override: bool,
}

/// The semantic panel model produced by typed builders and macros.
#[derive(Debug, Clone, PartialEq)]
pub struct Panel {
    pub(crate) kind: PanelKind,
    pub(crate) title: String,
    pub(crate) description: Option<String>,
    pub(crate) id: Option<u32>,
    pub(crate) key: Option<String>,
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) grid_pos: Option<GridPos>,
    pub(crate) datasource: Option<DataSource>,
    pub(crate) queries: Vec<Query>,
    pub(crate) transformations: Vec<Transformation>,
    pub(crate) field_config: FieldConfig,
    pub(crate) text: Option<(TextMode, String)>,
    pub(crate) links: Vec<DataLink>,
    pub(crate) transparent: bool,
    pub(crate) query_options: PanelQueryOptions,
    pub(crate) repeat: Option<Repeat>,
    pub(crate) kind_options: PanelOptions,
    pub(crate) raw_options: BTreeMap<String, Value>,
    pub(crate) extra: BTreeMap<String, Value>,
    /// Problems a builder found while lowering into this panel, reported by validation because builders themselves cannot fail.
    pub(crate) diagnostics: Vec<ValidationError>,
}

impl Panel {
    fn new(kind: PanelKind, title: impl Into<String>, width: u16, height: u16) -> Self {
        Self {
            kind,
            title: title.into(),
            description: None,
            id: None,
            key: None,
            width,
            height,
            grid_pos: None,
            datasource: None,
            queries: Vec::new(),
            transformations: Vec::new(),
            field_config: FieldConfig::default(),
            text: None,
            links: Vec::new(),
            transparent: false,
            query_options: PanelQueryOptions::default(),
            repeat: None,
            kind_options: PanelOptions::None,
            raw_options: BTreeMap::new(),
            extra: BTreeMap::new(),
            diagnostics: Vec::new(),
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

    /// Returns how the panel repeats, if it does.
    #[must_use]
    pub fn repeat(&self) -> Option<&Repeat> {
        self.repeat.as_ref()
    }

    /// The width automatic layout reserves: the full row for a horizontal repeat, which Grafana always stretches to 24 columns.
    pub(crate) fn layout_width(&self) -> u16 {
        match self.repeat {
            Some(Repeat::Horizontal(_)) => 24,
            Some(Repeat::Vertical(_)) | None => self.width,
        }
    }
}

/// Typed, visualization-specific panel options.
///
/// `Table` and `Heatmap` variants are added by Tasks 9 and 10, the tasks that
/// first construct them, so that a lowering the compiler could otherwise
/// silently skip becomes a compile error instead.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) enum PanelOptions {
    /// A visualization with no typed option surface.
    #[default]
    None,
    Stat(StatOptions),
    Gauge(GaugeOptions),
    Timeseries(TimeseriesOptions),
    BarGauge(BarGaugeOptions),
    Table(TableOptions),
    Heatmap(HeatmapOptions),
}

macro_rules! kind_options {
    ($($method:ident => $variant:ident($type:ty)),+ $(,)?) => {
        impl PanelOptions {
            $(
                pub(crate) fn $method(&mut self) -> &mut $type {
                    if !matches!(self, Self::$variant(_)) {
                        *self = Self::$variant(<$type>::default());
                    }
                    match self {
                        Self::$variant(options) => options,
                        _ => unreachable!(
                            concat!("just assigned PanelOptions::", stringify!($variant))
                        ),
                    }
                }
            )+
        }
    };
}

kind_options!(
    stat => Stat(StatOptions),
    gauge => Gauge(GaugeOptions),
    timeseries => Timeseries(TimeseriesOptions),
    bar_gauge => BarGauge(BarGaugeOptions),
    table => Table(TableOptions),
    heatmap => Heatmap(HeatmapOptions),
);

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

macro_rules! panel_query_option_setters {
    () => {
        /// Sets the panel's minimum query interval, such as `1m`.
        ///
        /// This is the lower limit for `$__interval` and `$__rate_interval` across every query in the panel (Grafana's `interval` key). A query's own `interval` overrides it for that query. The value is a number with a unit from `ms`, `s`, `m`, `h`, `d`, `w`, `M`, or `y`, or a `$variable`.
        #[must_use]
        pub fn min_interval(mut self, interval: impl Into<String>) -> Self {
            self.panel.query_options.min_interval = Some(interval.into());
            self
        }

        /// Caps the number of data points each query returns (Grafana's `maxDataPoints`).
        #[must_use]
        pub fn max_data_points(mut self, max_data_points: u32) -> Self {
            self.panel.query_options.max_data_points = Some(max_data_points);
            self
        }

        /// Replaces the dashboard's relative time range for this panel, for example `24h` for the last 24 hours or `now/d` for today so far (Grafana's `timeFrom`).
        ///
        /// Grafana ignores the override while the dashboard time range is absolute.
        #[must_use]
        pub fn time_from(mut self, time_from: impl Into<String>) -> Self {
            self.panel.query_options.time_from = Some(time_from.into());
            self
        }

        /// Shifts this panel's time range into the past, for example `1d` to show the same window yesterday, or `1d/d` to also round to whole days (Grafana's `timeShift`).
        #[must_use]
        pub fn time_shift(mut self, time_shift: impl Into<String>) -> Self {
            self.panel.query_options.time_shift = Some(time_shift.into());
            self
        }

        /// Hides the time override notice Grafana shows in the panel header when `time_from` or `time_shift` is set.
        #[must_use]
        pub fn hide_time_override(mut self, hide: bool) -> Self {
            self.panel.query_options.hide_time_override = hide;
            self
        }

        /// Repeats the panel for each selected value of a dashboard variable.
        ///
        /// A variable name on its own repeats horizontally. See [`Repeat`] for how each direction lays out.
        #[must_use]
        pub fn repeat(mut self, repeat: impl Into<Repeat>) -> Self {
            self.panel.repeat = Some(repeat.into());
            self
        }
    };
}

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

    /// Appends a transformation applied to this panel's query results.
    ///
    /// Transformations run in the order they are added.
    #[must_use]
    pub fn transform(mut self, transformation: impl Into<Transformation>) -> Self {
        self.panel.transformations.push(transformation.into());
        self
    }

    /// Sets the legend format on the most recently added query.
    ///
    /// This is the datasource's `legendFormat` series-name template, not the
    /// visualization's legend — for that see
    /// [`PanelBuilder::<TimeseriesKind>::legend_options`].
    ///
    /// This convenience makes the macro DSL read naturally. Builder-oriented
    /// code can set the same value directly on [`crate::PrometheusQuery`] or
    /// [`crate::LokiQuery`]. Calling it before adding a query is a no-op.
    #[must_use]
    pub fn legend_format(mut self, legend_format: impl Into<String>) -> Self {
        if let Some(query) = self.panel.queries.last_mut() {
            query.options_mut().legend_format = Some(legend_format.into());
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

    /// Sets the identity [`crate::PanelIds::Stable`] derives this panel's ID from.
    ///
    /// Without a key, a stable ID comes from the row and panel titles, so renaming either one changes it. A key replaces both: the ID then survives renames and moves between rows. Keys must be unique across the dashboard. Positional IDs ignore keys, and an explicit [`id`](Self::id) takes precedence over one.
    #[must_use]
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.panel.key = Some(key.into());
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

    /// Adds a value mapping to the field defaults.
    #[must_use]
    pub fn mapping(mut self, mapping: ValueMapping) -> Self {
        self.panel.field_config.mappings.push(mapping);
        self
    }

    /// Adds a data link to the field defaults, shown when a viewer clicks a value.
    ///
    /// Unlike [`PanelBuilder::link`], which sits in the panel header, a field link can interpolate the clicked value through [`crate::DataLinkVariable`].
    #[must_use]
    pub fn data_link(mut self, link: DataLink) -> Self {
        self.panel.field_config.links.push(link);
        self
    }

    /// Adds a field override.
    #[must_use]
    pub fn override_field(mut self, field_override: FieldOverride) -> Self {
        self.panel.field_config.overrides.push(field_override);
        self
    }

    /// Adds a link to the panel header.
    #[must_use]
    pub fn link(mut self, link: DataLink) -> Self {
        self.panel.links.push(link);
        self
    }

    /// Makes the panel background transparent.
    #[must_use]
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.panel.transparent = transparent;
        self
    }

    panel_query_option_setters!();

    /// Adds a plugin-specific value under the panel's `options` object.
    ///
    /// This escape hatch is applied after every typed option setter, so it
    /// wins over the equivalent typed value even if called before it.
    #[must_use]
    pub fn option(mut self, key: impl Into<String>, value: Value) -> Self {
        self.panel.raw_options.insert(key.into(), value);
        self
    }

    /// Adds an unsupported top-level Grafana panel property.
    ///
    /// Extras are flattened directly onto the serialized panel object,
    /// outside `options` and `fieldConfig`, so no typed setter can shadow
    /// them.
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

impl PanelBuilder<StatKind> {
    /// Sets whether Grafana colors the value, background, or neither.
    #[must_use]
    pub fn color_mode(mut self, mode: StatColorMode) -> Self {
        self.panel.kind_options.stat().color_mode = Some(mode);
        self
    }

    /// Sets area-sparkline or no-sparkline rendering.
    #[must_use]
    pub fn graph_mode(mut self, mode: StatGraphMode) -> Self {
        self.panel.kind_options.stat().graph_mode = Some(mode);
        self
    }

    /// Sets the value orientation.
    #[must_use]
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.panel.kind_options.stat().orientation = Some(orientation);
        self
    }

    /// Controls Grafana's wide-layout treatment for value names and values.
    #[must_use]
    pub fn wide_layout(mut self, wide_layout: bool) -> Self {
        self.panel.kind_options.stat().wide_layout = Some(wide_layout);
        self
    }

    /// Sets how fields are reduced to displayed values.
    #[must_use]
    pub fn reduce_options(mut self, options: ReduceOptions) -> Self {
        self.panel.kind_options.stat().reduce = Some(options);
        self
    }
}

impl PanelBuilder<GaugeKind> {
    /// Sets the value orientation.
    #[must_use]
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.panel.kind_options.gauge().orientation = Some(orientation);
        self
    }

    /// Sets how fields are reduced to displayed values.
    #[must_use]
    pub fn reduce_options(mut self, options: ReduceOptions) -> Self {
        self.panel.kind_options.gauge().reduce = Some(options);
        self
    }
}

impl PanelBuilder<TimeseriesKind> {
    /// Sets the area fill opacity from 0 to 100.
    #[must_use]
    pub fn fill_opacity(mut self, opacity: f64) -> Self {
        self.panel.kind_options.timeseries().fill_opacity = Some(opacity);
        self
    }

    /// Sets the line width in pixels.
    #[must_use]
    pub fn line_width(mut self, width: f64) -> Self {
        self.panel.kind_options.timeseries().line_width = Some(width);
        self
    }

    /// Sets the point-marker size in pixels.
    #[must_use]
    pub fn point_size(mut self, size: f64) -> Self {
        self.panel.kind_options.timeseries().point_size = Some(size);
        self
    }

    /// Sets line interpolation between samples.
    #[must_use]
    pub fn line_interpolation(mut self, interpolation: LineInterpolation) -> Self {
        self.panel.kind_options.timeseries().line_interpolation = Some(interpolation);
        self
    }

    /// Sets point-marker visibility.
    #[must_use]
    pub fn show_points(mut self, visibility: PointVisibility) -> Self {
        self.panel.kind_options.timeseries().show_points = Some(visibility);
        self
    }

    /// Controls whether lines span null samples.
    #[must_use]
    pub fn span_nulls(mut self, span_nulls: bool) -> Self {
        self.panel.kind_options.timeseries().span_nulls = Some(span_nulls);
        self
    }

    /// Sets series stacking mode and group.
    #[must_use]
    pub fn stacking(mut self, stacking: Stacking) -> Self {
        self.panel.kind_options.timeseries().stacking = Some(stacking);
        self
    }

    /// Sets typed tooltip behavior.
    #[must_use]
    pub fn tooltip(mut self, tooltip: Tooltip) -> Self {
        self.panel.kind_options.timeseries().tooltip = Some(tooltip);
        self
    }

    /// Sets visualization legend options.
    ///
    /// Only time-series panels read the `legend` key in Grafana; this
    /// method is available only on [`PanelBuilder<TimeseriesKind>`] so a
    /// call that would otherwise silently do nothing on another panel kind
    /// is a compile error instead.
    #[must_use]
    pub fn legend_options(mut self, legend: Legend) -> Self {
        self.panel.kind_options.timeseries().legend = Some(legend);
        self
    }
}

impl PanelBuilder<BarGaugeKind> {
    /// Sets the visual fill style.
    #[must_use]
    pub fn display_mode(mut self, mode: BarGaugeDisplayMode) -> Self {
        self.panel.kind_options.bar_gauge().display_mode = Some(mode);
        self
    }

    /// Sets the bar orientation.
    #[must_use]
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.panel.kind_options.bar_gauge().orientation = Some(orientation);
        self
    }

    /// Sets how fields are reduced to displayed bars.
    #[must_use]
    pub fn reduce_options(mut self, options: ReduceOptions) -> Self {
        self.panel.kind_options.bar_gauge().reduce = Some(options);
        self
    }
}

impl PanelBuilder<TableKind> {
    /// Appends a field to the table's initial sort order.
    ///
    /// This is the panel's own sort state, which Grafana keys on the field's
    /// display name. It is distinct from the [`crate::SortBy`] transformation,
    /// which reorders the underlying data and keys on the raw field name.
    #[must_use]
    pub fn sort_by(mut self, field: impl Into<String>, direction: SortDirection) -> Self {
        self.panel
            .kind_options
            .table()
            .sort_by
            .push(TableSort::new(field, direction));
        self
    }

    /// Sets the default cell renderer for every column.
    #[must_use]
    pub fn cell(mut self, cell: impl Into<TableCell>) -> Self {
        self.panel.kind_options.table().cell = Some(cell.into());
        self
    }
}

impl PanelBuilder<HeatmapKind> {
    /// Sets the color palette, for example `"Oranges"`.
    #[must_use]
    pub fn color_scheme(mut self, scheme: impl Into<HeatmapColorScheme>) -> Self {
        self.panel.kind_options.heatmap().color_scheme = Some(scheme.into());
        self
    }

    /// Sets the number of discrete color steps.
    #[must_use]
    pub fn color_steps(mut self, steps: u32) -> Self {
        self.panel.kind_options.heatmap().color_steps = Some(steps);
        self
    }

    /// Selects palette or opacity coloring.
    #[must_use]
    pub fn color_mode(mut self, mode: HeatmapColorMode) -> Self {
        self.panel.kind_options.heatmap().color_mode = Some(mode);
        self
    }

    /// Sets the gap between cells in pixels.
    #[must_use]
    pub fn cell_gap(mut self, gap: u8) -> Self {
        self.panel.kind_options.heatmap().cell_gap = Some(gap);
        self
    }

    /// Shows or hides the color-scale legend.
    ///
    /// Named `show_legend` rather than `legend` because it takes a bare
    /// boolean: the crate's other legend setter,
    /// [`PanelBuilder::<TimeseriesKind>::legend_options`], takes a
    /// [`Legend`], so a `legend` here would read as "configure the legend"
    /// and mislead.
    #[must_use]
    pub fn show_legend(mut self, show: bool) -> Self {
        self.panel.kind_options.heatmap().legend = Some(show);
        self
    }

    /// Sets the Y-axis unit.
    #[must_use]
    pub fn y_axis_unit(mut self, unit: Unit) -> Self {
        self.panel.kind_options.heatmap().y_axis_unit = Some(unit);
        self
    }

    /// Sets the Y-axis placement.
    #[must_use]
    pub fn y_axis_placement(mut self, placement: AxisPlacement) -> Self {
        self.panel.kind_options.heatmap().y_axis_placement = Some(placement);
        self
    }

    /// Chooses between bucketing raw data and consuming pre-bucketed data.
    #[must_use]
    pub fn calculate(mut self, calculate: bool) -> Self {
        self.panel.kind_options.heatmap().calculate = Some(calculate);
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

    /// Appends a transformation applied to this panel's query results.
    ///
    /// Transformations run in the order they are added.
    #[must_use]
    pub fn transform(mut self, transformation: impl Into<Transformation>) -> Self {
        self.panel.transformations.push(transformation.into());
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

    /// Sets the identity [`crate::PanelIds::Stable`] derives this panel's ID from.
    ///
    /// Without a key, a stable ID comes from the row and panel titles, so renaming either one changes it. A key replaces both: the ID then survives renames and moves between rows. Keys must be unique across the dashboard. Positional IDs ignore keys, and an explicit [`id`](Self::id) takes precedence over one.
    #[must_use]
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.panel.key = Some(key.into());
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

    /// Adds a link to the panel header.
    #[must_use]
    pub fn link(mut self, link: DataLink) -> Self {
        self.panel.links.push(link);
        self
    }

    /// Makes the panel background transparent.
    #[must_use]
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.panel.transparent = transparent;
        self
    }

    panel_query_option_setters!();

    /// Adds a plugin option.
    ///
    /// `RawPanel` has no kind-specific typed setters, so this is the only
    /// way to populate a plugin's `options` object.
    #[must_use]
    pub fn option(mut self, key: impl Into<String>, value: Value) -> Self {
        self.panel.raw_options.insert(key.into(), value);
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
