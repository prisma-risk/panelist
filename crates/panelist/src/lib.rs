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
mod validation;
mod variable;

pub use dashboard::{Dashboard, DashboardItem, DashboardLink, Row, TimeRange};
pub use datasource::{DataSource, loki, prometheus};
pub use field::{
    Color, ColorScheme, FieldConfig, FieldOverride, Legend, LegendCalculation, LegendMode,
    LegendPlacement, OverrideMatcher, OverrideProperty, ThresholdMode, ThresholdStep, Thresholds,
    Unit,
};
pub use panel::{
    BarGauge, Gauge, GridPos, Heatmap, Panel, PanelBuilder, PanelKind, RawPanel, Stat, Table, Text,
    TextMode, Timeseries,
};
pub use query::{LokiQuery, PrometheusQuery, Query, QueryOptions, RawQuery};
pub use validation::{Error, Result, ValidationError, ValidationErrors};
pub use variable::{
    ConstantVariable, CustomVariable, QueryVariable, Variable, VariableBuilder, VariableRefresh,
};

#[doc(hidden)]
pub mod __private {
    pub use serde_json;
}

/// Common dashboard-authoring types, builders, helpers, and macros.
pub mod prelude {
    pub use crate::{
        BarGauge, Color, ColorScheme, ConstantVariable, CustomVariable, Dashboard, DashboardLink,
        DataSource, FieldConfig, FieldOverride, Gauge, GridPos, Heatmap, Legend, LegendCalculation,
        LegendMode, LegendPlacement, LokiQuery, OverrideMatcher, OverrideProperty, Panel,
        PanelKind, PrometheusQuery, Query, QueryVariable, RawPanel, RawQuery, Row, Stat, Table,
        Text, TextMode, ThresholdMode, Thresholds, TimeRange, Timeseries, Unit, Variable,
        VariableRefresh, dashboard, loki, loki_query, prometheus, promql,
    };
}
