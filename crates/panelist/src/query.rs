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

use crate::{Color, ColorScheme, DataSource, OverrideProperty, Thresholds, Unit};

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

/// Which evaluation a query asks its datasource for.
///
/// A query that sets none keeps the datasource's own default, which for Prometheus and Loki is a range query. Setting [`QueryType::Range`] explicitly is not the same as leaving it unset: it pins the choice in the emitted target, so a later change to the datasource default does not move it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    /// One value per step across the dashboard time range. The shape time series panels draw.
    Range,
    /// One value per series at the end of the time range. The shape stat, gauge, bar gauge and table panels usually want.
    Instant,
    /// Both a range and an instant evaluation in one target. Prometheus only: Loki has no combined mode, so validation rejects it on a [`LokiQuery`].
    Both,
}

/// Properties shared by datasource queries.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QueryOptions {
    pub(crate) ref_id: Option<String>,
    pub(crate) legend_format: Option<String>,
    pub(crate) query_type: Option<QueryType>,
    pub(crate) datasource: Option<DataSource>,
    pub(crate) hidden: bool,
    pub(crate) interval: Option<String>,
    pub(crate) editor_mode: Option<QueryEditorMode>,
    pub(crate) field_properties: Vec<OverrideProperty>,
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

            /// Selects a range, instant, or combined evaluation.
            ///
            /// Leave it unset to keep the datasource's default. See [`QueryType`].
            #[must_use]
            pub fn query_type(mut self, query_type: QueryType) -> Self {
                self.options.query_type = Some(query_type);
                self
            }

            /// Requests an instant query: shorthand for `.query_type(QueryType::Instant)`.
            ///
            /// Instant is the one non-default evaluation most dashboards reach for, on every stat, gauge, bar gauge and table that shows a current value, so it gets a name of its own. The rarer explicit range and combined evaluations go through [`Self::query_type`].
            #[must_use]
            pub fn instant(self) -> Self {
                self.query_type(QueryType::Instant)
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

            /// Applies a field property to every field this query returns.
            ///
            /// Serialization turns a query's properties into one `byFrameRefID` field override on the reference ID the query resolves to, so the query needs no explicit [`Self::ref_id`] for it. Query overrides come before the panel's own [`crate::FieldOverride`]s, in query order, and Grafana lets a later override win. So a panel-level override, for example one [`crate::FieldOverride::by_name`] series, still refines what the query set.
            ///
            /// A variable's query renders no fields, so properties set on it are not emitted.
            #[must_use]
            pub fn field_property(mut self, property: OverrideProperty) -> Self {
                self.options.field_properties.push(property);
                self
            }

            /// Sets the unit of every field this query returns. See [`Self::field_property`].
            #[must_use]
            pub fn unit(self, unit: Unit) -> Self {
                self.field_property(OverrideProperty::Unit(unit))
            }

            /// Gives every field this query returns one fixed color. See [`Self::field_property`].
            #[must_use]
            pub fn color(self, color: Color) -> Self {
                self.field_property(OverrideProperty::Color(ColorScheme::Fixed(color)))
            }

            /// Sets the display name of every field this query returns. See [`Self::field_property`].
            ///
            /// Unlike [`Self::legend_format`], which the datasource applies per series, this is a Grafana field property and may use field-config templates such as `${__field.labels.route}`.
            #[must_use]
            pub fn display_name(self, display_name: impl Into<String>) -> Self {
                self.field_property(OverrideProperty::DisplayName(display_name.into()))
            }

            /// Sets the displayed decimal count of every field this query returns. See [`Self::field_property`].
            #[must_use]
            pub fn decimals(self, decimals: u8) -> Self {
                self.field_property(OverrideProperty::Decimals(decimals))
            }

            /// Sets thresholds for every field this query returns. See [`Self::field_property`].
            #[must_use]
            pub fn thresholds(self, thresholds: Thresholds) -> Self {
                self.field_property(OverrideProperty::Thresholds(thresholds))
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
///
/// A raw target carries only the keys the author set, next to the `refId`, `hide` and datasource keys every target has. Expression-based datasources take [`Self::expression`]; others, such as CloudWatch, describe the whole query through [`Self::extra`]:
///
/// ```rust
/// use panelist::prelude::*;
/// use serde_json::json;
///
/// let query = RawQuery::new()
///     .datasource(DataSource::new("cloudwatch", "cloudwatch-main"))
///     .extra("namespace", json!("AWS/EC2"))
///     .extra("metricName", json!("CPUUtilization"))
///     .extra("statistic", json!("Average"));
///
/// let dashboard = Dashboard::new("Fleet").panel(Timeseries::new("CPU").query(query));
/// let target = &serde_json::to_value(&dashboard)?["panels"][0]["targets"][0];
/// assert!(target.get("expr").is_none());
/// assert_eq!(target["metricName"], "CPUUtilization");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RawQuery {
    pub(crate) expression: Option<String>,
    pub(crate) options: QueryOptions,
    pub(crate) extra: BTreeMap<String, Value>,
}

impl RawQuery {
    /// Creates an empty raw query. Set [`Self::expression`], [`Self::extra`] keys, or both.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the query expression, emitted as the target's `expr` key.
    ///
    /// `expr` is what Prometheus-compatible and Loki-compatible datasources read. A datasource that spells its query differently, such as `rawSql` or `query`, takes it through [`Self::extra`] instead.
    #[must_use]
    pub fn expression(mut self, expression: impl Into<String>) -> Self {
        self.expression = Some(expression.into());
        self
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
    /// The query expression, or `None` for a raw query that sets none.
    pub(crate) fn expression(&self) -> Option<&str> {
        match self {
            Self::Prometheus(query) => Some(&query.expression),
            Self::Loki(query) => Some(&query.expression),
            Self::Raw(query) => query.expression.as_deref(),
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
