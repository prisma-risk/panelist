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

use serde_json::Value;

use crate::DataSource;

/// Datasource-query editor shown by Grafana.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryEditorMode {
    /// Structured visual query builder.
    Builder,
    /// Raw query-language editor.
    Code,
}

impl QueryEditorMode {
    pub(crate) const fn as_grafana(self) -> &'static str {
        match self {
            Self::Builder => "builder",
            Self::Code => "code",
        }
    }
}

/// Properties shared by datasource queries.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QueryOptions {
    pub(crate) ref_id: Option<String>,
    pub(crate) legend_format: Option<String>,
    pub(crate) instant: bool,
    pub(crate) range: Option<bool>,
    pub(crate) datasource: Option<DataSource>,
    pub(crate) hidden: bool,
    pub(crate) interval: Option<String>,
    pub(crate) editor_mode: Option<QueryEditorMode>,
}

macro_rules! impl_query_builder {
    ($type:ty) => {
        impl $type {
            /// Sets an explicit Grafana query reference ID.
            #[must_use]
            pub fn ref_id(mut self, ref_id: impl Into<String>) -> Self {
                self.options.ref_id = Some(ref_id.into());
                self
            }

            /// Sets the legend format emitted by the datasource.
            ///
            /// This is the per-series name template Grafana sends to the
            /// datasource as `legendFormat`, for example `"{{status}}"`. It
            /// is unrelated to [`crate::Legend`], which configures how the
            /// visualization *draws* its legend.
            #[must_use]
            pub fn legend_format(mut self, legend_format: impl Into<String>) -> Self {
                self.options.legend_format = Some(legend_format.into());
                self
            }

            /// Requests an instant query rather than a range query.
            #[must_use]
            pub fn instant(mut self, instant: bool) -> Self {
                self.options.instant = instant;
                if instant {
                    self.options.range = Some(false);
                }
                self
            }

            /// Explicitly enables or disables range-query behavior.
            #[must_use]
            pub fn range(mut self, range: bool) -> Self {
                self.options.range = Some(range);
                self
            }

            /// Overrides the panel datasource for this query.
            #[must_use]
            pub fn datasource(mut self, datasource: DataSource) -> Self {
                self.options.datasource = Some(datasource);
                self
            }

            /// Hides the query while retaining it in the dashboard.
            #[must_use]
            pub fn hidden(mut self, hidden: bool) -> Self {
                self.options.hidden = hidden;
                self
            }

            /// Sets a datasource-specific minimum interval.
            #[must_use]
            pub fn interval(mut self, interval: impl Into<String>) -> Self {
                self.options.interval = Some(interval.into());
                self
            }

            /// Selects Grafana's visual builder or raw code editor.
            #[must_use]
            pub fn editor_mode(mut self, editor_mode: QueryEditorMode) -> Self {
                self.options.editor_mode = Some(editor_mode);
                self
            }
        }
    };
}

/// The response shape a Prometheus query asks Grafana to produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrometheusFormat {
    /// One series per label set. Grafana's default.
    #[default]
    TimeSeries,
    /// Tabular result suitable for table panels.
    Table,
    /// Bucketed result suitable for heatmap panels.
    Heatmap,
}

/// A first-class Prometheus/PromQL query.
#[derive(Debug, Clone, PartialEq)]
pub struct PrometheusQuery {
    pub(crate) expression: String,
    pub(crate) options: QueryOptions,
    pub(crate) format: Option<PrometheusFormat>,
}

impl PrometheusQuery {
    /// Creates a PromQL query.
    #[must_use]
    pub fn new(expression: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
            options: QueryOptions::default(),
            format: None,
        }
    }

    /// Sets the Grafana response format for this query.
    ///
    /// Prometheus histogram queries feeding a heatmap panel need
    /// [`PrometheusFormat::Heatmap`]; table panels joining multiple queries
    /// usually need [`PrometheusFormat::Table`].
    #[must_use]
    pub fn format(mut self, format: PrometheusFormat) -> Self {
        self.format = Some(format);
        self
    }
}

impl_query_builder!(PrometheusQuery);

/// A first-class Loki/LogQL query.
#[derive(Debug, Clone, PartialEq)]
pub struct LokiQuery {
    pub(crate) expression: String,
    pub(crate) options: QueryOptions,
}

impl LokiQuery {
    /// Creates a LogQL query.
    #[must_use]
    pub fn new(expression: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
            options: QueryOptions::default(),
        }
    }
}

impl_query_builder!(LokiQuery);

/// A generic datasource query for properties Panelist does not model yet.
#[derive(Debug, Clone, PartialEq)]
pub struct RawQuery {
    pub(crate) expression: String,
    pub(crate) options: QueryOptions,
    pub(crate) extra: BTreeMap<String, Value>,
}

impl RawQuery {
    /// Creates a raw query with an `expr` property.
    #[must_use]
    pub fn new(expression: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
            options: QueryOptions::default(),
            extra: BTreeMap::new(),
        }
    }

    /// Adds or replaces an unsupported target property.
    #[must_use]
    pub fn extra(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }
}

impl_query_builder!(RawQuery);

/// A datasource query accepted by any data-backed panel.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Query {
    /// Prometheus/PromQL target.
    Prometheus(PrometheusQuery),
    /// Loki/LogQL target.
    Loki(LokiQuery),
    /// Explicit escape hatch for another datasource target.
    Raw(RawQuery),
}

impl Query {
    pub(crate) fn expression(&self) -> &str {
        match self {
            Self::Prometheus(query) => &query.expression,
            Self::Loki(query) => &query.expression,
            Self::Raw(query) => &query.expression,
        }
    }

    pub(crate) fn options(&self) -> &QueryOptions {
        match self {
            Self::Prometheus(query) => &query.options,
            Self::Loki(query) => &query.options,
            Self::Raw(query) => &query.options,
        }
    }

    pub(crate) fn options_mut(&mut self) -> &mut QueryOptions {
        match self {
            Self::Prometheus(query) => &mut query.options,
            Self::Loki(query) => &mut query.options,
            Self::Raw(query) => &mut query.options,
        }
    }

    pub(crate) fn raw_extra(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Self::Raw(query) => Some(&query.extra),
            Self::Prometheus(_) | Self::Loki(_) => None,
        }
    }

    pub(crate) const fn prometheus_format(&self) -> Option<PrometheusFormat> {
        match self {
            Self::Prometheus(query) => query.format,
            Self::Loki(_) | Self::Raw(_) => None,
        }
    }
}

impl From<PrometheusQuery> for Query {
    fn from(value: PrometheusQuery) -> Self {
        Self::Prometheus(value)
    }
}

impl From<LokiQuery> for Query {
    fn from(value: LokiQuery) -> Self {
        Self::Loki(value)
    }
}

impl From<RawQuery> for Query {
    fn from(value: RawQuery) -> Self {
        Self::Raw(value)
    }
}
