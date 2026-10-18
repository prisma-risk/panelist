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

//! A read-only, typed view of a dashboard after layout and inheritance resolution.
//!
//! Serialization resolves several values the authored model leaves open: panel IDs, query reference IDs, grid positions, and the datasource each panel and query inherits. [`crate::Dashboard::resolve`] runs that same pass and returns these types instead of Grafana JSON, so a test or lint reads the dashboard Grafana will receive without parsing it back out of `serde_json::Value`.
//!
//! The view pairs each resolved fact with the authored typed value it belongs to. A unit is still a [`Unit`] and a color scheme is still a [`ColorScheme`], not the string Grafana stores. The internal wire structs stay private, as `docs/schema-compatibility.md` requires.

use crate::{
    Color, ColorScheme, Dashboard, DataSource, FieldConfig, FieldOverride, GridPos, Panel,
    PanelKind, PrometheusFormat, Query, QueryEditorMode, Row, ThresholdMode, Thresholds,
    Transformation, Unit,
};

/// The color Grafana gives a synthesized base threshold step when no step is authored at all.
static DEFAULT_BASE_COLOR: Color = Color::Green;

/// A validated dashboard with every panel ID, query reference ID, grid position, and inherited datasource resolved.
///
/// Created by [`Dashboard::resolve`]. The view borrows the dashboard it describes and reflects exactly what serialization emits for it: both are produced by one resolution pass, so the two cannot disagree.
///
/// [`ResolvedDashboard::panels`] visits every panel, including those inside collapsed rows. Grafana nests a collapsed row's panels inside the row object, so a walk over the top-level `panels` array of the JSON skips them; this view does not.
#[derive(Debug, Clone)]
pub struct ResolvedDashboard<'a> {
    pub(crate) dashboard: &'a Dashboard,
    pub(crate) items: Vec<ResolvedItem<'a>>,
}

impl<'a> ResolvedDashboard<'a> {
    /// Returns the dashboard title.
    #[must_use]
    pub fn title(&self) -> &'a str {
        &self.dashboard.title
    }

    /// Returns the dashboard UID, if one is set.
    #[must_use]
    pub fn uid(&self) -> Option<&'a str> {
        self.dashboard.uid.as_deref()
    }

    /// Returns the dashboard-level datasource that panels and queries inherit when they set none.
    #[must_use]
    pub fn datasource(&self) -> Option<&'a DataSource> {
        self.dashboard.datasource.as_ref()
    }

    /// Returns the top-level rows and row-less panels in authored order.
    #[must_use]
    pub fn items(&self) -> &[ResolvedItem<'a>] {
        &self.items
    }

    /// Iterates over every row, collapsed or expanded, in authored order.
    pub fn rows(&self) -> impl Iterator<Item = &ResolvedRow<'a>> {
        self.items.iter().filter_map(|item| match item {
            ResolvedItem::Row(row) => Some(row),
            ResolvedItem::Panel(_) => None,
        })
    }

    /// Iterates over every panel in authored order, flattening rows.
    ///
    /// Panels inside collapsed rows are included. Row headers are not panels here; use [`ResolvedDashboard::rows`] for them.
    pub fn panels(&self) -> impl Iterator<Item = &ResolvedPanel<'a>> {
        self.items.iter().flat_map(|item| match item {
            ResolvedItem::Row(row) => row.panels.as_slice(),
            ResolvedItem::Panel(panel) => std::slice::from_ref(panel),
        })
    }
}

/// One top-level dashboard item after resolution.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ResolvedItem<'a> {
    /// A row and the panels it owns.
    Row(ResolvedRow<'a>),
    /// A panel outside any row.
    Panel(ResolvedPanel<'a>),
}

/// A row with its assigned panel ID, header position, and resolved panels.
#[derive(Debug, Clone)]
pub struct ResolvedRow<'a> {
    pub(crate) row: &'a Row,
    pub(crate) id: u32,
    pub(crate) grid_pos: GridPos,
    pub(crate) panels: Vec<ResolvedPanel<'a>>,
}

impl<'a> ResolvedRow<'a> {
    /// Returns the row title.
    #[must_use]
    pub fn title(&self) -> &'a str {
        &self.row.title
    }

    /// Returns the panel ID Grafana gives the row header.
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.id
    }

    /// Returns the row's identity key, if one is set.
    #[must_use]
    pub fn key(&self) -> Option<&'a str> {
        self.row.key.as_deref()
    }

    /// Returns whether Grafana initially collapses this row.
    #[must_use]
    pub fn collapsed(&self) -> bool {
        self.row.collapsed
    }

    /// Returns the position of the full-width, one-unit row header.
    #[must_use]
    pub const fn grid_pos(&self) -> GridPos {
        self.grid_pos
    }

    /// Returns the row's panels in authored order.
    #[must_use]
    pub fn panels(&self) -> &[ResolvedPanel<'a>] {
        &self.panels
    }
}

/// The row a [`ResolvedPanel`] belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelRow<'a> {
    pub(crate) title: &'a str,
    pub(crate) id: u32,
    pub(crate) collapsed: bool,
}

impl<'a> PanelRow<'a> {
    /// Returns the row title.
    #[must_use]
    pub const fn title(&self) -> &'a str {
        self.title
    }

    /// Returns the row header's panel ID.
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.id
    }

    /// Returns whether the row is initially collapsed.
    #[must_use]
    pub const fn collapsed(&self) -> bool {
        self.collapsed
    }
}

/// A panel with its assigned ID, grid position, row, inherited datasource, and resolved queries.
#[derive(Debug, Clone)]
pub struct ResolvedPanel<'a> {
    pub(crate) panel: &'a Panel,
    pub(crate) id: u32,
    pub(crate) grid_pos: GridPos,
    pub(crate) row: Option<PanelRow<'a>>,
    pub(crate) datasource: Option<&'a DataSource>,
    pub(crate) queries: Vec<ResolvedQuery<'a>>,
}

impl<'a> ResolvedPanel<'a> {
    /// Returns the panel ID, explicit or assigned.
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.id
    }

    /// Returns the panel's identity key, if one is set.
    #[must_use]
    pub fn key(&self) -> Option<&'a str> {
        self.panel.key.as_deref()
    }

    /// Returns the panel kind.
    #[must_use]
    pub fn kind(&self) -> &'a PanelKind {
        &self.panel.kind
    }

    /// Returns the panel title.
    #[must_use]
    pub fn title(&self) -> &'a str {
        &self.panel.title
    }

    /// Returns the panel description, if one is set.
    #[must_use]
    pub fn description(&self) -> Option<&'a str> {
        self.panel.description.as_deref()
    }

    /// Returns the panel's grid position, explicit or laid out automatically.
    #[must_use]
    pub const fn grid_pos(&self) -> GridPos {
        self.grid_pos
    }

    /// Returns the row that owns this panel, or `None` for a panel outside any row.
    #[must_use]
    pub const fn row(&self) -> Option<PanelRow<'a>> {
        self.row
    }

    /// Returns the panel datasource: the panel's own, else the dashboard's.
    #[must_use]
    pub const fn datasource(&self) -> Option<&'a DataSource> {
        self.datasource
    }

    /// Returns the panel-level minimum interval, if one is set.
    ///
    /// Each query's [`ResolvedQuery::interval`] already falls back to this value.
    #[must_use]
    pub fn min_interval(&self) -> Option<&'a str> {
        self.panel.query_options.min_interval.as_deref()
    }

    /// Returns the panel's queries in authored order, with reference IDs, datasources, and intervals resolved.
    #[must_use]
    pub fn queries(&self) -> &[ResolvedQuery<'a>] {
        &self.queries
    }

    /// Returns the panel's field defaults and overrides.
    #[must_use]
    pub fn field_config(&self) -> ResolvedFieldConfig<'a> {
        ResolvedFieldConfig {
            config: &self.panel.field_config,
        }
    }

    /// Returns the panel's transformations in the order Grafana applies them.
    #[must_use]
    pub fn transformations(&self) -> &'a [Transformation] {
        &self.panel.transformations
    }
}

/// A query with its reference ID and datasource resolved.
#[derive(Debug, Clone)]
pub struct ResolvedQuery<'a> {
    pub(crate) query: &'a Query,
    pub(crate) ref_id: String,
    pub(crate) datasource: Option<&'a DataSource>,
    pub(crate) panel_min_interval: Option<&'a str>,
}

impl<'a> ResolvedQuery<'a> {
    /// Returns the authored query, for matching on its datasource family.
    #[must_use]
    pub const fn query(&self) -> &'a Query {
        self.query
    }

    /// Returns the reference ID, explicit or assigned in `A`, `B`, … order.
    ///
    /// This is the ID a [`crate::OverrideMatcher::QueryRefId`] override or a ref-ID-scoped transformation matches.
    #[must_use]
    pub fn ref_id(&self) -> &str {
        &self.ref_id
    }

    /// Returns the query expression.
    #[must_use]
    pub fn expression(&self) -> &'a str {
        self.query.expression()
    }

    /// Returns the datasource the query runs against: its own, else the panel's, else the dashboard's.
    #[must_use]
    pub const fn datasource(&self) -> Option<&'a DataSource> {
        self.datasource
    }

    /// Returns the per-series legend template sent as `legendFormat`.
    #[must_use]
    pub fn legend_format(&self) -> Option<&'a str> {
        self.query.options().legend_format.as_deref()
    }

    /// Returns the minimum interval the query runs with, such as `1m`: its own, else the panel's minimum interval.
    ///
    /// This is the floor Grafana applies to `$__interval` and `$__rate_interval` for this query.
    #[must_use]
    pub fn interval(&self) -> Option<&'a str> {
        self.query
            .options()
            .interval
            .as_deref()
            .or(self.panel_min_interval)
    }

    /// Returns the result format of a Prometheus query, or `None` for any other query.
    ///
    /// Grafana's Prometheus datasource reads an absent format as a time series, so a Prometheus query that sets none returns `Some(PrometheusFormat::TimeSeries)`.
    #[must_use]
    pub fn format(&self) -> Option<PrometheusFormat> {
        match self.query {
            Query::Prometheus(query) => Some(query.format.unwrap_or_default()),
            Query::Loki(_) | Query::Raw(_) => None,
        }
    }

    /// Returns whether the query is an instant query.
    #[must_use]
    pub fn instant(&self) -> bool {
        self.query.options().instant
    }

    /// Returns whether the query is hidden from the visualization.
    #[must_use]
    pub fn hidden(&self) -> bool {
        self.query.options().hidden
    }

    /// Returns the Grafana query editor mode, if one is set.
    #[must_use]
    pub fn editor_mode(&self) -> Option<QueryEditorMode> {
        self.query.options().editor_mode
    }
}

/// A panel's field defaults and overrides.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedFieldConfig<'a> {
    config: &'a FieldConfig,
}

impl<'a> ResolvedFieldConfig<'a> {
    /// Returns the default unit, if one is set.
    #[must_use]
    pub fn unit(&self) -> Option<&'a Unit> {
        self.config.unit.as_ref()
    }

    /// Returns the default minimum, if one is set.
    #[must_use]
    pub const fn min(&self) -> Option<f64> {
        self.config.min
    }

    /// Returns the default maximum, if one is set.
    #[must_use]
    pub const fn max(&self) -> Option<f64> {
        self.config.max
    }

    /// Returns the default decimal count, if one is set.
    #[must_use]
    pub const fn decimals(&self) -> Option<u8> {
        self.config.decimals
    }

    /// Returns the default display-name template, if one is set.
    #[must_use]
    pub fn display_name(&self) -> Option<&'a str> {
        self.config.display_name.as_deref()
    }

    /// Returns the default color scheme, if one is set.
    ///
    /// `None` means the panel leaves the choice to Grafana's per-visualization default.
    #[must_use]
    pub fn color(&self) -> Option<&'a ColorScheme> {
        self.config.color.as_ref()
    }

    /// Returns the default thresholds, if any are set.
    ///
    /// `None` means the panel emits no thresholds, so Grafana substitutes its own default set.
    #[must_use]
    pub fn thresholds(&self) -> Option<ResolvedThresholds<'a>> {
        self.config
            .thresholds
            .as_ref()
            .map(ResolvedThresholds::from)
    }

    /// Returns the field overrides in authored order.
    #[must_use]
    pub fn overrides(&self) -> &'a [FieldOverride] {
        &self.config.overrides
    }
}

/// Thresholds as Grafana receives them, with the base step made explicit.
///
/// Grafana requires the first step to be a base step with no value. When the authored steps start with a valued step, serialization adds a base step in that step's color; when no steps are authored, it adds a green one. [`ResolvedThresholds::steps`] includes that base step. Thresholds inside an [`crate::OverrideProperty::Thresholds`] resolve the same way through `ResolvedThresholds::from`.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedThresholds<'a> {
    thresholds: &'a Thresholds,
}

impl<'a> From<&'a Thresholds> for ResolvedThresholds<'a> {
    fn from(thresholds: &'a Thresholds) -> Self {
        Self { thresholds }
    }
}

impl<'a> ResolvedThresholds<'a> {
    /// Returns whether step values are absolute or percentages.
    #[must_use]
    pub const fn mode(&self) -> ThresholdMode {
        self.thresholds.mode
    }

    /// Iterates over the emitted steps, base step first.
    pub fn steps(&self) -> impl Iterator<Item = ResolvedThresholdStep<'a>> + use<'a> {
        let steps = self.thresholds.steps.as_slice();
        let base = match steps.first() {
            Some(first) if first.value.is_none() => None,
            first => Some(ResolvedThresholdStep {
                color: first.map_or(&DEFAULT_BASE_COLOR, |step| &step.color),
                value: None,
            }),
        };
        base.into_iter()
            .chain(steps.iter().map(|step| ResolvedThresholdStep {
                color: &step.color,
                value: step.value,
            }))
    }
}

/// One emitted threshold step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedThresholdStep<'a> {
    color: &'a Color,
    value: Option<f64>,
}

impl<'a> ResolvedThresholdStep<'a> {
    /// Returns the step color.
    #[must_use]
    pub const fn color(&self) -> &'a Color {
        self.color
    }

    /// Returns the value at which the step starts, or `None` for the base step.
    #[must_use]
    pub const fn value(&self) -> Option<f64> {
        self.value
    }
}
