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

use std::collections::HashSet;

use crate::{
    Color, ColorScheme, FieldOverride, JoinByField, JoinMode, OrganizeFields, OverrideProperty,
    Panel, PrometheusFormat, PrometheusQuery, Query, Reducer, TableCell, Thresholds,
    TimeSeriesToTable, Transformation, Unit, ValidationError,
    grafana::effective_ref_ids,
    panel::{PanelBuilder, TableKind},
};

/// What a [`Column`] shows: one value per key, or a sparkline per key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColumnKind {
    /// An instant `format: table` query, one value per key.
    Value,
    /// A range query turned into a sparkline by `timeSeriesTable`, reduced to one displayed value with this reducer.
    Trend(Reducer),
}

/// One column of a [`KeyedTable`]: a header, the PromQL that fills it, and the field properties that style it.
///
/// The column name is everything Grafana needs to find the column again. It becomes the header, the rename target, the ordering key, and the override matcher, so it is written once instead of once per Grafana concept.
///
/// ```rust
/// use panelist::prelude::*;
///
/// let rps = Column::new("RPS", promql!("sum by (route) (rate(http_requests_total[$__rate_interval]))"))
///     .unit(Unit::RequestsPerSecond);
///
/// let trend = Column::trend(
///     "p95 trend",
///     promql!("histogram_quantile(0.95, sum by (route, le) (rate(http_request_duration_seconds_bucket[$__rate_interval])))"),
///     Reducer::Last,
/// )
/// .cell(SparklineCell::new().hide_value(true));
/// # let _ = (rps, trend);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    name: String,
    query: PrometheusQuery,
    kind: ColumnKind,
    properties: Vec<OverrideProperty>,
}

impl Column {
    /// A value column: an instant `format: table` query yielding one row per key.
    ///
    /// The query's format and instant flag are set by the table; anything else on it, such as a datasource, interval, or explicit reference ID, is kept.
    #[must_use]
    pub fn new(name: impl Into<String>, query: PrometheusQuery) -> Self {
        Self {
            name: name.into(),
            query,
            kind: ColumnKind::Value,
            properties: Vec::new(),
        }
    }

    /// A trend column: a range query that Grafana's time-series-to-table transformation turns into one sparkline per key.
    ///
    /// `reducer` picks the single value Grafana shows beside the sparkline and sorts the column by. Pair the column with [`crate::SparklineCell`] to draw it as a sparkline.
    #[must_use]
    pub fn trend(name: impl Into<String>, query: PrometheusQuery, reducer: Reducer) -> Self {
        Self {
            name: name.into(),
            query,
            kind: ColumnKind::Trend(reducer),
            properties: Vec::new(),
        }
    }

    /// Appends a field property to this column's override.
    ///
    /// Properties keep the order they are added in, like [`FieldOverride::property`].
    #[must_use]
    pub fn property(mut self, property: OverrideProperty) -> Self {
        self.properties.push(property);
        self
    }

    /// Sets the column unit.
    #[must_use]
    pub fn unit(self, unit: Unit) -> Self {
        self.property(OverrideProperty::Unit(unit))
    }

    /// Sets the column minimum.
    #[must_use]
    pub fn min(self, min: f64) -> Self {
        self.property(OverrideProperty::Min(min))
    }

    /// Sets the column maximum.
    #[must_use]
    pub fn max(self, max: f64) -> Self {
        self.property(OverrideProperty::Max(max))
    }

    /// Sets the displayed decimal count.
    #[must_use]
    pub fn decimals(self, decimals: u8) -> Self {
        self.property(OverrideProperty::Decimals(decimals))
    }

    /// Sets a fixed color.
    #[must_use]
    pub fn color(self, color: Color) -> Self {
        self.property(OverrideProperty::Color(ColorScheme::Fixed(color)))
    }

    /// Sets thresholds for this column.
    #[must_use]
    pub fn thresholds(self, thresholds: Thresholds) -> Self {
        self.property(OverrideProperty::Thresholds(thresholds))
    }

    /// Sets the cell renderer for this column.
    #[must_use]
    pub fn cell(self, cell: impl Into<TableCell>) -> Self {
        self.property(OverrideProperty::Cell(cell.into()))
    }
}

/// A table that joins several Prometheus queries on one shared label, one row per label value and one column per query.
///
/// Create one from a configured table with [`PanelBuilder::keyed`]:
///
/// ```rust
/// use panelist::prelude::*;
///
/// let routes = Table::new("Route performance")
///     .width(24)
///     .sort_by("p95", SortDirection::Descending)
///     .keyed("route")
///     .key_name("Route")
///     .column(
///         Column::new("RPS", promql!("sum by (route) (rate(http_requests_total[$__rate_interval]))"))
///             .unit(Unit::RequestsPerSecond),
///     )
///     .column(
///         Column::new("p95", promql!("histogram_quantile(0.95, sum by (route, le) (rate(http_request_duration_seconds_bucket[$__rate_interval])))"))
///             .unit(Unit::Seconds)
///             .cell(TableCellType::ColoredBackground)
///             .thresholds(Thresholds::new().green(0.0).yellow(0.3).red(1.0)),
///     )
///     .column(
///         Column::trend("Trend", promql!("sum by (route) (rate(http_requests_total[$__rate_interval]))"), Reducer::Last)
///             .cell(SparklineCell::new().hide_value(true)),
///     );
///
/// let dashboard = Dashboard::new("Routes")
///     .datasource(prometheus("prometheus-main"))
///     .row(Row::new("Routes").panel(routes));
/// dashboard.validate()?;
/// # Ok::<(), panelist::Error>(())
/// ```
///
/// # What it lowers to
///
/// A keyed table is not a new kind of panel. It lowers into the ordinary table authoring model: plain queries, transformations, and field overrides, the same ones you could write by hand. In order:
///
/// 1. One query per column, with reference IDs allocated in column order. A value column becomes an instant `format: table` query; a trend column becomes a range `format: time_series` query.
/// 2. A time-series-to-table transformation for the trend columns, if there are any.
/// 3. A join on the key label.
/// 4. An organize transformation that renames each query's value field to its column name, renames the key if [`KeyedTable::key_name`] is set, hides the per-query `Time` fields, and orders the key first and then the columns as added.
/// 5. One by-name override per column that sets any field property.
///
/// The renames depend on how Grafana names the fields, which Panelist tracks so you do not have to. The Prometheus datasource names a table query's value field `Value #<refId>` when the panel runs more than one `format: table` query, and plain `Value` when it runs exactly one. Time-series-to-table names its field `Trend #<refId>` directly.
///
/// # Joining
///
/// The join is Grafana's plain outer join. It merges frames that each hold one row per key into one row per key, and leaves a cell empty when a query has no series for that key. The tabular outer join is not used: Grafana 11.5 expands it into one row per key per query, and only Grafana 13 merges those rows correctly.
///
/// Grafana joins on exactly one field, so a key is one label. To key a table on several labels, build one label from them in PromQL with `label_join` and aggregate by it, for example `sum by (endpoint) (label_join(rate(http_requests_total[$__rate_interval]), "endpoint", " ", "method", "route"))`, then key the table on `endpoint`.
///
/// # Composing with the rest of the table
///
/// Panel-level settings go on the [`crate::Table`] before `keyed`: width, description, [`PanelBuilder::sort_by`] by column name, a default [`PanelBuilder::cell`], extra overrides, extra queries or transformations. The lowering puts its own queries, transformations, and overrides ahead of anything already on the table. So an extra override for a column wins over the column's own properties, and an extra transformation runs on the joined table, where the fields already carry their column names.
///
/// # Limits
///
/// - Columns are Prometheus queries, because the field naming above is the Prometheus datasource's.
/// - A query that returns no series contributes no frame. If every value column but one comes back empty, Grafana names the survivor plain `Value`, and that column loses its header.
/// - The `dashboard!` macro has no keyed-table syntax yet. Build the table here and add it to a row with `panels: [..];`.
///
/// # Validation
///
/// [`crate::Dashboard::validate`] rejects an empty key, an empty column name, and a column name used twice, including a column named like the key.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyedTable {
    table: Panel,
    key: String,
    key_name: Option<String>,
    columns: Vec<Column>,
}

impl PanelBuilder<TableKind> {
    /// Turns this table into a [`KeyedTable`] joined on the `key` label.
    ///
    /// Configure the panel itself first; the keyed table only adds columns.
    #[must_use]
    pub fn keyed(self, key: impl Into<String>) -> KeyedTable {
        KeyedTable {
            table: self.build(),
            key: key.into(),
            key_name: None,
            columns: Vec::new(),
        }
    }
}

impl KeyedTable {
    /// Sets the key column's header. Defaults to the key label itself.
    #[must_use]
    pub fn key_name(mut self, name: impl Into<String>) -> Self {
        self.key_name = Some(name.into());
        self
    }

    /// Appends a column. Columns appear in the order they are added, after the key.
    #[must_use]
    pub fn column(mut self, column: Column) -> Self {
        self.columns.push(column);
        self
    }

    /// Lowers the keyed table into a table panel.
    #[must_use]
    pub fn build(self) -> Panel {
        let Self {
            table,
            key,
            key_name,
            columns,
        } = self;
        let mut panel = table;

        let has_key = !key.trim().is_empty();
        let key_header = key_name.clone().unwrap_or_else(|| key.clone());
        let mut diagnostics = Vec::new();
        if !has_key {
            diagnostics.push(ValidationError::MissingTableKey {
                panel: panel.title.clone(),
            });
        }

        // A rejected column is left out entirely, so the one error reported for it is not followed by a second, more confusing one about the organize transformation it would have produced.
        let mut headers = HashSet::new();
        if has_key {
            headers.insert(key_header.clone());
        }
        let columns: Vec<Column> = columns
            .into_iter()
            .filter(|column| {
                if column.name.trim().is_empty() {
                    diagnostics.push(ValidationError::MissingTableColumnName {
                        panel: panel.title.clone(),
                    });
                    false
                } else if !headers.insert(column.name.clone()) {
                    diagnostics.push(ValidationError::DuplicateTableColumn {
                        panel: panel.title.clone(),
                        column: column.name.clone(),
                    });
                    false
                } else {
                    true
                }
            })
            .collect();

        let mut queries: Vec<Query> = columns
            .iter()
            .map(|column| {
                let query = column.query.clone();
                Query::from(match column.kind {
                    ColumnKind::Value => instant_table_query(query),
                    ColumnKind::Trend(_) => range_series_query(query),
                })
            })
            .collect();
        queries.append(&mut panel.queries);
        let ref_ids = effective_ref_ids(&queries);
        for (query, ref_id) in queries.iter_mut().zip(&ref_ids).take(columns.len()) {
            query.options_mut().ref_id = Some(ref_id.clone());
        }

        // Grafana only runs queries that are not hidden, and the Prometheus datasource counts the table results it actually received.
        let table_results = queries
            .iter()
            .filter(|query| {
                !query.options().hidden
                    && query.prometheus_format() == Some(PrometheusFormat::Table)
            })
            .count();

        let mut transformations: Vec<Transformation> = Vec::new();
        let mut trends = TimeSeriesToTable::new();
        let mut has_trend = false;
        let mut has_value = false;
        let mut organize = OrganizeFields::new();
        for (column, ref_id) in columns.iter().zip(&ref_ids) {
            let field = match column.kind {
                ColumnKind::Value => {
                    has_value = true;
                    if table_results > 1 {
                        format!("Value #{ref_id}")
                    } else {
                        "Value".to_owned()
                    }
                }
                ColumnKind::Trend(reducer) => {
                    has_trend = true;
                    trends = trends.query_with(ref_id.clone(), reducer);
                    format!("Trend #{ref_id}")
                }
            };
            organize = organize.rename(field, column.name.clone());
        }
        if has_trend {
            transformations.push(trends.into());
        }
        if has_key {
            transformations.push(JoinByField::new(key.clone()).mode(JoinMode::Outer).into());
            if let Some(key_name) = key_name {
                organize = organize.rename(key, key_name);
            }
        }
        if has_value {
            // Every instant table result carries its own `Time` field. Hiding matches incoming names, so one entry hides all of them.
            organize = organize.hide("Time");
        }
        organize = organize.order(
            has_key
                .then_some(key_header)
                .into_iter()
                .chain(columns.iter().map(|column| column.name.clone())),
        );
        transformations.push(organize.into());

        let overrides = columns
            .into_iter()
            .filter(|column| !column.properties.is_empty())
            .map(|column| {
                column
                    .properties
                    .into_iter()
                    .fold(FieldOverride::by_name(column.name), FieldOverride::property)
            });

        panel.queries = queries;
        panel.transformations.splice(0..0, transformations);
        panel.field_config.overrides.splice(0..0, overrides);
        panel.diagnostics.extend(diagnostics);
        panel
    }
}

impl From<KeyedTable> for Panel {
    fn from(value: KeyedTable) -> Self {
        value.build()
    }
}

// The two functions below are the only places a keyed table decides a query's type, so a change to how queries spell instant and range touches nothing else here.

/// One value per key: an instant query in table format.
fn instant_table_query(query: PrometheusQuery) -> PrometheusQuery {
    query.format(PrometheusFormat::Table).instant(true)
}

/// One series per key: a range query in time-series format. An explicit `range(true)` is kept; anything that would make it instant is dropped.
fn range_series_query(mut query: PrometheusQuery) -> PrometheusQuery {
    query.options.instant = false;
    query.options.range = query.options.range.filter(|range| *range);
    query.format(PrometheusFormat::TimeSeries)
}
