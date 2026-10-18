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

use std::fmt;

/// A Grafana data link: a URL template attached to a panel header or to the values of a field.
///
/// Grafana uses one link model for both places. A panel link (set with `PanelBuilder::link`) appears in the panel header. A field link (set with `PanelBuilder::data_link`, `FieldConfig::data_link`, or `FieldOverride::data_link`) appears when a viewer clicks a value, and its URL can interpolate the clicked value through [`DataLinkVariable`].
///
/// This is deliberately a separate type from [`crate::DashboardLink`]. Grafana renders panel and field links through `getDataLinkUIModel`, which reads only `title`, `url`, and `targetBlank`; a dashboard link's `tooltip`, `tags`, `includeVars`, and `keepTime` keys would be stored and then ignored.
///
/// ```rust
/// use panelist::prelude::*;
///
/// let link = DataLink::new(
///     "Instance logs",
///     format!("/explore?instance={}", DataLinkVariable::FieldLabel("instance".into())),
/// )
/// .keep_time(true)
/// .target_blank(true);
/// # let _ = link;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataLink {
    pub(crate) title: String,
    pub(crate) url: String,
    pub(crate) target_blank: bool,
    pub(crate) include_vars: bool,
    pub(crate) keep_time: bool,
}

impl DataLink {
    /// Creates a link. Both the title and the URL may contain Grafana variables.
    #[must_use]
    pub fn new(title: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            url: url.into(),
            target_blank: false,
            include_vars: false,
            keep_time: false,
        }
    }

    /// Opens the link in a new browser tab.
    #[must_use]
    pub fn target_blank(mut self, target_blank: bool) -> Self {
        self.target_blank = target_blank;
        self
    }

    /// Appends every dashboard variable to the URL as query parameters.
    ///
    /// Data links have no `includeVars` key in Grafana, so Panelist appends `$__all_variables` to the URL, the same parameter Grafana appends for a dashboard link with `includeVars` set.
    #[must_use]
    pub fn include_vars(mut self, include_vars: bool) -> Self {
        self.include_vars = include_vars;
        self
    }

    /// Appends the current time range to the URL as query parameters.
    ///
    /// Data links have no `keepTime` key in Grafana, so Panelist appends `$__url_time_range` to the URL, the same parameter Grafana appends for a dashboard link with `keepTime` set.
    #[must_use]
    pub fn keep_time(mut self, keep_time: bool) -> Self {
        self.keep_time = keep_time;
        self
    }
}

/// A built-in variable Grafana interpolates into a data link's title or URL.
///
/// `Display` renders the `${…}` reference, so variables compose with `format!`. Grafana URL-encodes interpolated values in the URL. The value and field variables only resolve in field links, not in panel links.
///
/// ```rust
/// use panelist::DataLinkVariable;
///
/// assert_eq!(DataLinkVariable::ValueRaw.to_string(), "${__value.raw}");
/// assert_eq!(
///     DataLinkVariable::FieldLabel("instance".into()).to_string(),
///     "${__field.labels.instance}"
/// );
/// assert_eq!(
///     DataLinkVariable::DataField("Route name".into()).to_string(),
///     "${__data.fields[\"Route name\"]}"
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataLinkVariable {
    /// The dashboard time range as `from=…&to=…` query parameters (`__url_time_range`).
    TimeRange,
    /// The start of the dashboard time range (`__from`).
    From,
    /// The end of the dashboard time range (`__to`).
    To,
    /// Every dashboard variable as `var-…` query parameters (`__all_variables`).
    AllVariables,
    /// The name of the clicked series (`__series.name`).
    SeriesName,
    /// The name of the clicked field (`__field.name`).
    FieldName,
    /// The display name of the clicked field, after overrides and transformations (`__field.displayName`).
    FieldDisplayName,
    /// One label of the clicked field (`__field.labels.<name>`).
    FieldLabel(String),
    /// The timestamp of the clicked value (`__value.time`).
    ValueTime,
    /// The clicked value as a number (`__value.numeric`).
    ValueNumeric,
    /// The clicked value as formatted text (`__value.text`).
    ValueText,
    /// The clicked value before formatting (`__value.raw`).
    ValueRaw,
    /// The calculation the clicked value represents, such as `Last` (`__value.calc`).
    ValueCalc,
    /// The formatted value of another field on the same row (`__data.fields.<name>`).
    DataField(String),
    /// A dashboard variable by name.
    Dashboard(String),
}

impl fmt::Display for DataLinkVariable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TimeRange => formatter.write_str("${__url_time_range}"),
            Self::From => formatter.write_str("${__from}"),
            Self::To => formatter.write_str("${__to}"),
            Self::AllVariables => formatter.write_str("${__all_variables}"),
            Self::SeriesName => formatter.write_str("${__series.name}"),
            Self::FieldName => formatter.write_str("${__field.name}"),
            Self::FieldDisplayName => formatter.write_str("${__field.displayName}"),
            Self::FieldLabel(label) => write!(formatter, "${{__field.labels{}}}", path(label)),
            Self::ValueTime => formatter.write_str("${__value.time}"),
            Self::ValueNumeric => formatter.write_str("${__value.numeric}"),
            Self::ValueText => formatter.write_str("${__value.text}"),
            Self::ValueRaw => formatter.write_str("${__value.raw}"),
            Self::ValueCalc => formatter.write_str("${__value.calc}"),
            Self::DataField(field) => write!(formatter, "${{__data.fields{}}}", path(field)),
            Self::Dashboard(name) => write!(formatter, "${{{name}}}"),
        }
    }
}

/// Grafana's `buildLabelPath`: dotted access unless the name contains a dot or a space, which need bracket access.
fn path(name: &str) -> String {
    if name.contains('.') || name.trim().contains(' ') {
        format!("[\"{name}\"]")
    } else {
        format!(".{name}")
    }
}
