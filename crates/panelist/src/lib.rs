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

mod dashboard;
mod datasource;
mod field;
mod grafana;
mod macros;
mod panel;
mod query;
mod transformation;
mod validation;
mod variable;
mod visualization;

pub use dashboard::{Dashboard, DashboardCursorSync, DashboardItem, DashboardLink, Row, TimeRange};
pub use datasource::{DataSource, loki, prometheus};
pub use field::{
    Color, ColorScheme, FieldConfig, FieldOverride, Legend, LegendMode, LegendPlacement,
    OverrideMatcher, OverrideProperty, ThresholdMode, ThresholdStep, Thresholds, Unit,
    ValueMapping,
};
pub use panel::{
    BarGauge, Gauge, GridPos, Heatmap, Panel, PanelBuilder, PanelKind, RawPanel, Stat, Table, Text,
    TextMode, Timeseries,
};
pub use query::{
    LokiQuery, PrometheusFormat, PrometheusQuery, Query, QueryEditorMode, QueryOptions, RawQuery,
};
pub use transformation::{
    JoinByField, JoinMode, LabelsToFields, LabelsToFieldsMode, OrganizeFields, RawTransformation,
    SortBy, TimeSeriesToTable, Transformation, TransformationFilter,
};
pub use validation::{Error, Result, ValidationError, ValidationErrors};
pub use variable::{
    ConstantVariable, CustomVariable, DataSourceVariable, QueryVariable, Variable, VariableBuilder,
    VariableRefresh, VariableSelection, VariableSort,
};
pub use visualization::{
    BarGaugeDisplayMode, LineInterpolation, Orientation, PointVisibility, ReduceOptions, Reducer,
    SortDirection, Stacking, StackingMode, StatColorMode, StatGraphMode, Tooltip, TooltipMode,
    TooltipSort,
};

#[doc(hidden)]
pub mod __private {
    pub use serde_json;
}

/// Common dashboard-authoring types, builders, helpers, and macros.
pub mod prelude {
    pub use crate::{
        BarGauge, BarGaugeDisplayMode, Color, ColorScheme, ConstantVariable, CustomVariable,
        Dashboard, DashboardCursorSync, DashboardLink, DataSource, DataSourceVariable, FieldConfig,
        FieldOverride, Gauge, GridPos, Heatmap, JoinByField, JoinMode, LabelsToFields,
        LabelsToFieldsMode, Legend, LegendMode, LegendPlacement, LineInterpolation, LokiQuery,
        OrganizeFields, Orientation, OverrideMatcher, OverrideProperty, Panel, PanelKind,
        PointVisibility, PrometheusFormat, PrometheusQuery, Query, QueryEditorMode, QueryVariable,
        RawPanel, RawQuery, RawTransformation, ReduceOptions, Reducer, Row, SortBy, SortDirection,
        Stacking, StackingMode, Stat, StatColorMode, StatGraphMode, Table, Text, TextMode,
        ThresholdMode, Thresholds, TimeRange, TimeSeriesToTable, Timeseries, Tooltip, TooltipMode,
        TooltipSort, Transformation, TransformationFilter, Unit, ValueMapping, Variable,
        VariableRefresh, VariableSelection, VariableSort, dashboard, loki, loki_query, prometheus,
        promql,
    };
}
