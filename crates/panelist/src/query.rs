// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use serde_json::Value;

use crate::DataSource;

/// Properties shared by datasource queries.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QueryOptions {
    pub(crate) ref_id: Option<String>,
    pub(crate) legend: Option<String>,
    pub(crate) instant: bool,
    pub(crate) range: Option<bool>,
    pub(crate) datasource: Option<DataSource>,
    pub(crate) hidden: bool,
    pub(crate) interval: Option<String>,
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
            #[must_use]
            pub fn legend(mut self, legend: impl Into<String>) -> Self {
                self.options.legend = Some(legend.into());
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
        }
    };
}

/// A first-class Prometheus/PromQL query.
#[derive(Debug, Clone, PartialEq)]
pub struct PrometheusQuery {
    pub(crate) expression: String,
    pub(crate) options: QueryOptions,
}

impl PrometheusQuery {
    /// Creates a PromQL query.
    #[must_use]
    pub fn new(expression: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
            options: QueryOptions::default(),
        }
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
