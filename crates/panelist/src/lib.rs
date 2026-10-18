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

#![warn(missing_docs)]

//! Concise, strongly typed Grafana dashboards as Rust code.
//!
//! Panelist keeps authoring concepts separate from Grafana's wire format. Build
//! dashboards with ordinary Rust builders or the [`dashboard!`] macro, then
//! serialize them with `serde_json` or [`Dashboard::to_json_pretty`].
//!
//! ```rust
//! use panelist::prelude::*;
//!
//! let dashboard = dashboard! {
//!     title: "Service health";
//!     refresh: "30s";
//!
//!     row "Traffic" {
//!         timeseries "Requests" {
//!             query: promql!("sum(rate(http_requests_total[$__rate_interval]))");
//!             unit: reqps;
//!             width: 12;
//!         }
//!     }
//! };
//!
//! dashboard.validate()?;
//! let json = dashboard.to_json_pretty()?;
//! assert!(json.contains("http_requests_total"));
//! # Ok::<(), panelist::Error>(())
//! ```
//!
//! [`Dashboard::resolve`] returns a read-only [`ResolvedDashboard`] with every panel ID, query reference ID, grid position, and inherited datasource filled in, so tests and lints can check dashboard invariants without parsing the JSON back.

mod annotation;
mod dashboard;
mod datasource;
mod field;
mod grafana;
mod heatmap;
mod keyed_table;
mod link;
mod macros;
mod panel;
mod promql;
mod query;
pub mod render;
mod resolved;
mod table;
mod target;
mod transformation;
mod validation;
mod variable;
mod visualization;

pub use annotation::{Annotation, BuiltInAnnotation, PrometheusAnnotation, RawAnnotation};
pub use dashboard::{
    Dashboard, DashboardCursorSync, DashboardItem, DashboardLink, PanelIds, Row, TimeRange,
};
pub use datasource::{DataSource, loki, prometheus};
pub use field::{
    Color, ColorScheme, ContinuousPalette, Currency, FieldConfig, FieldOverride, Legend,
    LegendMode, LegendPlacement, OverrideMatcher, OverrideProperty, SpecialValue, ThresholdMode,
    Thresholds, Unit, ValueMapping,
};
pub use heatmap::{HeatmapColorMode, HeatmapColorScheme};
pub use keyed_table::{Column, KeyedTable};
pub use link::{DataLink, DataLinkVariable};
pub use panel::{
    BarGauge, Gauge, GridPos, Heatmap, HorizontalRepeat, Panel, PanelBuilder, PanelKind, RawPanel,
    Repeat, Stat, Table, Text, TextMode, Timeseries, VerticalRepeat,
};
pub use promql::{LegendFormatError, Matchers, RangeSelector, Selector, legend};
pub use query::{
    LokiQuery, PrometheusFormat, PrometheusQuery, Query, QueryEditorMode, QueryOptions, QueryType,
    RawQuery,
};
pub use render::DashboardSet;
pub use resolved::{
    PanelRow, ResolvedDashboard, ResolvedFieldConfig, ResolvedItem, ResolvedPanel, ResolvedQuery,
    ResolvedRow, ResolvedThresholdStep, ResolvedThresholds,
};
pub use table::{
    CellBackgroundMode, CellValueDisplay, ColoredBackgroundCell, GaugeCell, SparklineCell,
    TableCell, TableCellType,
};
pub use target::GrafanaTarget;
pub use transformation::{
    JoinByField, JoinMode, LabelsToFields, LabelsToFieldsMode, OrganizeFields, RawTransformation,
    RenameByRegex, SortBy, TimeSeriesToTable, Transformation,
};
pub use validation::{Error, Result, ValidationError, ValidationErrors, VariableReferenceSite};
pub use variable::{
    ConstantVariable, CustomVariable, DataSourceVariable, QueryVariable, Variable, VariableBuilder,
    VariableRefresh, VariableSelection, VariableSort,
};
pub use visualization::{
    AxisPlacement, BarGaugeDisplayMode, LineInterpolation, Orientation, PointVisibility,
    ReduceOptions, Reducer, SortDirection, Stacking, StackingMode, StatColorMode, StatGraphMode,
    Tooltip, TooltipMode, TooltipSort,
};

/// Compiles and runs every Rust example in the README as a doctest.
#[cfg(doctest)]
#[doc = include_str!("../../../README.md")]
struct ReadmeDoctests;

#[doc(hidden)]
pub mod __private {
    pub use crate::promql::{check_legend, render_legend};
    pub use panelist_macros::dashboard;
    pub use serde_json;
}

/// Common dashboard-authoring types, builders, helpers, and macros.
pub mod prelude {
    pub use crate::{
        Annotation, AxisPlacement, BarGauge, BarGaugeDisplayMode, BuiltInAnnotation,
        CellBackgroundMode, CellValueDisplay, Color, ColorScheme, ColoredBackgroundCell, Column,
        ConstantVariable, ContinuousPalette, Currency, CustomVariable, Dashboard,
        DashboardCursorSync, DashboardLink, DataLink, DataLinkVariable, DataSource,
        DataSourceVariable, FieldConfig, FieldOverride, Gauge, GaugeCell, GrafanaTarget, GridPos,
        Heatmap, HeatmapColorMode, HeatmapColorScheme, JoinByField, JoinMode, KeyedTable,
        LabelsToFields, LabelsToFieldsMode, Legend, LegendMode, LegendPlacement, LineInterpolation,
        LokiQuery, Matchers, OrganizeFields, Orientation, OverrideMatcher, OverrideProperty, Panel,
        PanelIds, PanelKind, PointVisibility, PrometheusAnnotation, PrometheusFormat,
        PrometheusQuery, Query, QueryEditorMode, QueryType, QueryVariable, RangeSelector,
        RawAnnotation, RawPanel, RawQuery, RawTransformation, ReduceOptions, Reducer,
        RenameByRegex, Repeat, Row, Selector, SortBy, SortDirection, SparklineCell, SpecialValue,
        Stacking, StackingMode, Stat, StatColorMode, StatGraphMode, Table, TableCell,
        TableCellType, Text, TextMode, ThresholdMode, Thresholds, TimeRange, TimeSeriesToTable,
        Timeseries, Tooltip, TooltipMode, TooltipSort, Transformation, Unit, ValueMapping,
        Variable, VariableBuilder, VariableRefresh, VariableSelection, VariableSort, dashboard,
        legend, loki, loki_query, prometheus, promql,
    };
}
