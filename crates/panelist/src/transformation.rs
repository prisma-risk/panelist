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

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::{Reducer, SortDirection};

/// How Grafana joins multiple query results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JoinMode {
    /// Keep every row from every input. Best for time series.
    #[default]
    Outer,
    /// Keep only rows present in every input.
    Inner,
    /// Outer join permitting duplicate join values. Best for tabular data.
    OuterTabular,
}

/// Joins multiple query results on a shared field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinByField {
    pub(crate) field: String,
    pub(crate) mode: JoinMode,
}

impl JoinByField {
    /// Joins on a field using Grafana's outer join.
    #[must_use]
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            mode: JoinMode::Outer,
        }
    }

    /// Joins on a field using an inner join.
    #[must_use]
    pub fn inner(field: impl Into<String>) -> Self {
        Self::new(field).mode(JoinMode::Inner)
    }

    /// Joins on a field using a tabular outer join.
    #[must_use]
    pub fn outer_tabular(field: impl Into<String>) -> Self {
        Self::new(field).mode(JoinMode::OuterTabular)
    }

    /// Sets the join mode.
    #[must_use]
    pub fn mode(mut self, mode: JoinMode) -> Self {
        self.mode = mode;
        self
    }
}

/// Renames, hides, and reorders fields.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OrganizeFields {
    pub(crate) renames: BTreeMap<String, String>,
    pub(crate) hidden: BTreeSet<String>,
    pub(crate) order: Vec<String>,
}

impl OrganizeFields {
    /// Creates an empty field-organization transformation.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Renames a field.
    #[must_use]
    pub fn rename(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.renames.insert(from.into(), to.into());
        self
    }

    /// Hides a field.
    #[must_use]
    pub fn hide(mut self, name: impl Into<String>) -> Self {
        self.hidden.insert(name.into());
        self
    }

    /// Orders the listed fields; the nth listed field is assigned index
    /// `n`. Fields left out of the list keep Grafana's natural ordering.
    #[must_use]
    pub fn order(mut self, fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.order = fields.into_iter().map(Into::into).collect();
        self
    }
}

/// One field in a sort transformation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SortByField {
    pub(crate) field: String,
    pub(crate) descending: bool,
}

/// Sorts rows by one or more fields.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SortBy {
    pub(crate) fields: Vec<SortByField>,
}

impl SortBy {
    /// Sorts by one field in the given direction.
    #[must_use]
    pub fn new(field: impl Into<String>, direction: SortDirection) -> Self {
        Self {
            fields: vec![SortByField {
                field: field.into(),
                descending: direction.is_descending(),
            }],
        }
    }

    /// Sorts by one field, smallest value first.
    #[must_use]
    pub fn asc(field: impl Into<String>) -> Self {
        Self::new(field, SortDirection::Ascending)
    }

    /// Sorts by one field, largest value first.
    #[must_use]
    pub fn desc(field: impl Into<String>) -> Self {
        Self::new(field, SortDirection::Descending)
    }

    /// Appends a secondary sort field, used to break ties in prior fields.
    #[must_use]
    pub fn then_by(mut self, field: impl Into<String>, direction: SortDirection) -> Self {
        self.fields.push(SortByField {
            field: field.into(),
            descending: direction.is_descending(),
        });
        self
    }
}

/// Per-query configuration for the time-series-to-table transformation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TimeSeriesToTableQuery {
    pub(crate) stat: Option<Reducer>,
    pub(crate) time_field: Option<String>,
}

/// Converts time-series results into table rows with a trend field.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TimeSeriesToTable {
    pub(crate) queries: BTreeMap<String, TimeSeriesToTableQuery>,
}

impl TimeSeriesToTable {
    /// Creates an empty time-series-to-table transformation.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Includes a query's results, with no explicit trend statistic.
    #[must_use]
    pub fn query(mut self, ref_id: impl Into<String>) -> Self {
        self.queries.entry(ref_id.into()).or_default();
        self
    }

    /// Includes a query's results, reduced to a trend statistic.
    #[must_use]
    pub fn query_with(mut self, ref_id: impl Into<String>, stat: Reducer) -> Self {
        self.queries.entry(ref_id.into()).or_default().stat = Some(stat);
        self
    }

    /// Sets the time field backing the query's trend column.
    #[must_use]
    pub fn time_field(mut self, ref_id: impl Into<String>, field: impl Into<String>) -> Self {
        self.queries.entry(ref_id.into()).or_default().time_field = Some(field.into());
        self
    }
}

/// Whether labels become columns or rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelsToFieldsMode {
    /// One field per label.
    #[default]
    Columns,
    /// One row per label.
    Rows,
}

/// Converts datasource labels into fields.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LabelsToFields {
    pub(crate) mode: LabelsToFieldsMode,
    pub(crate) keep: Vec<String>,
    pub(crate) value_label: Option<String>,
}

impl LabelsToFields {
    /// Creates a labels-to-fields transformation in column mode.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets column or row mode.
    #[must_use]
    pub fn mode(mut self, mode: LabelsToFieldsMode) -> Self {
        self.mode = mode;
        self
    }

    /// Restricts which labels become fields. An empty set keeps every label.
    #[must_use]
    pub fn keep(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.keep = labels.into_iter().map(Into::into).collect();
        self
    }

    /// Names the field that holds the label value in row mode.
    #[must_use]
    pub fn value_label(mut self, label: impl Into<String>) -> Self {
        self.value_label = Some(label.into());
        self
    }
}

/// An escape hatch for a transformation Panelist does not model.
#[derive(Debug, Clone, PartialEq)]
pub struct RawTransformation {
    pub(crate) id: String,
    pub(crate) options: BTreeMap<String, Value>,
}

impl RawTransformation {
    /// Creates a raw transformation for a Grafana transformer ID.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            options: BTreeMap::new(),
        }
    }

    /// Sets an entry under the transformation's `options` object.
    #[must_use]
    pub fn option(mut self, key: impl Into<String>, value: Value) -> Self {
        self.options.insert(key.into(), value);
        self
    }
}

/// A panel transformation applied to query results before rendering.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Transformation {
    /// Join query results on a shared field.
    JoinByField(JoinByField),
    /// Rename, hide, and reorder fields.
    OrganizeFields(OrganizeFields),
    /// Sort rows by one or more fields.
    SortBy(SortBy),
    /// Convert time series into table rows.
    TimeSeriesToTable(TimeSeriesToTable),
    /// Convert labels into fields.
    LabelsToFields(LabelsToFields),
    /// An unmodeled Grafana transformation.
    Raw(RawTransformation),
}

impl From<JoinByField> for Transformation {
    fn from(value: JoinByField) -> Self {
        Self::JoinByField(value)
    }
}

impl From<OrganizeFields> for Transformation {
    fn from(value: OrganizeFields) -> Self {
        Self::OrganizeFields(value)
    }
}

impl From<SortBy> for Transformation {
    fn from(value: SortBy) -> Self {
        Self::SortBy(value)
    }
}

impl From<TimeSeriesToTable> for Transformation {
    fn from(value: TimeSeriesToTable) -> Self {
        Self::TimeSeriesToTable(value)
    }
}

impl From<LabelsToFields> for Transformation {
    fn from(value: LabelsToFields) -> Self {
        Self::LabelsToFields(value)
    }
}

impl From<RawTransformation> for Transformation {
    fn from(value: RawTransformation) -> Self {
        Self::Raw(value)
    }
}
