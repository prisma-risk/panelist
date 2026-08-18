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

use crate::{DataSource, Query};

/// When Grafana should refresh a query variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VariableRefresh {
    /// Do not refresh automatically.
    #[default]
    Never,
    /// Refresh when the dashboard loads.
    OnDashboardLoad,
    /// Refresh whenever the dashboard time range changes.
    OnTimeRangeChange,
}

impl VariableRefresh {
    pub(crate) const fn as_grafana(self) -> u8 {
        match self {
            Self::Never => 0,
            Self::OnDashboardLoad => 1,
            Self::OnTimeRangeChange => 2,
        }
    }
}

/// A problem `VariableBuilder` found while resolving a variable, carried on
/// the produced variable so validation can report it.
///
/// Carrying these is deliberate. The alternative is dropping them, and a
/// dropped option is indistinguishable from a working one in the emitted
/// JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VariableDiagnostic {
    /// An option was set that this variable kind has no Grafana key for.
    OptionNotApplicable(&'static str),
    /// Nothing selected a variable kind.
    NoSelector,
    /// More than one selector was set.
    ConflictingSelectors {
        /// The selector that decided the kind.
        chosen: &'static str,
        /// A selector that was ignored.
        ignored: &'static str,
    },
}

/// Ordering applied to values returned for a query variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VariableSort {
    /// Keep the datasource response order.
    #[default]
    Disabled,
    /// Sort text alphabetically in ascending order.
    AlphabeticalAscending,
    /// Sort text alphabetically in descending order.
    AlphabeticalDescending,
    /// Sort numeric values in ascending order.
    NumericalAscending,
    /// Sort numeric values in descending order.
    NumericalDescending,
    /// Sort alphabetically without regard to letter case, ascending.
    AlphabeticalCaseInsensitiveAscending,
    /// Sort alphabetically without regard to letter case, descending.
    AlphabeticalCaseInsensitiveDescending,
}

impl VariableSort {
    pub(crate) const fn as_grafana(self) -> u8 {
        match self {
            Self::Disabled => 0,
            Self::AlphabeticalAscending => 1,
            Self::AlphabeticalDescending => 2,
            Self::NumericalAscending => 3,
            Self::NumericalDescending => 4,
            Self::AlphabeticalCaseInsensitiveAscending => 5,
            Self::AlphabeticalCaseInsensitiveDescending => 6,
        }
    }
}

/// The current text and value persisted for a dashboard variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableSelection {
    pub(crate) text: String,
    pub(crate) value: String,
    pub(crate) selected: bool,
}

impl VariableSelection {
    /// Creates a selected variable value with independently controlled display
    /// text and interpolation value.
    #[must_use]
    pub fn new(text: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            value: value.into(),
            selected: true,
        }
    }

    /// Controls whether Grafana marks this persisted value as selected.
    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

/// A variable that lets viewers select one Grafana datasource instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataSourceVariable {
    pub(crate) name: String,
    pub(crate) label: Option<String>,
    pub(crate) plugin_id: String,
    pub(crate) current: Option<VariableSelection>,
    pub(crate) regex: String,
    pub(crate) refresh: VariableRefresh,
    pub(crate) hidden: bool,
    pub(crate) skip_url_sync: bool,
    /// Problems `VariableBuilder` found while producing this variable.
    /// Never serialized; validation reports them.
    pub(crate) diagnostics: Vec<VariableDiagnostic>,
}

impl DataSourceVariable {
    /// Creates a variable for datasource instances of `plugin_id`, such as
    /// `prometheus` or `loki`.
    #[must_use]
    pub fn new(name: impl Into<String>, plugin_id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: None,
            diagnostics: Vec::new(),
            plugin_id: plugin_id.into(),
            current: None,
            regex: String::new(),
            refresh: VariableRefresh::OnDashboardLoad,
            hidden: false,
            skip_url_sync: false,
        }
    }

    /// Sets the visible label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets a persisted selection whose display text and value are identical.
    #[must_use]
    pub fn default(mut self, value: impl Into<String>) -> Self {
        let value = value.into();
        self.current = Some(VariableSelection::new(value.clone(), value));
        self
    }

    /// Sets the complete persisted selection.
    #[must_use]
    pub fn current(mut self, current: VariableSelection) -> Self {
        self.current = Some(current);
        self
    }

    /// Filters selectable datasource names with a Grafana regular expression.
    #[must_use]
    pub fn regex(mut self, regex: impl Into<String>) -> Self {
        self.regex = regex.into();
        self
    }

    /// Controls when Grafana refreshes the datasource list.
    #[must_use]
    pub fn refresh(mut self, refresh: VariableRefresh) -> Self {
        self.refresh = refresh;
        self
    }

    /// Hides the variable control.
    #[must_use]
    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// Excludes the variable from dashboard URLs.
    #[must_use]
    pub fn skip_url_sync(mut self, skip_url_sync: bool) -> Self {
        self.skip_url_sync = skip_url_sync;
        self
    }
}

/// A datasource-backed dashboard variable.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryVariable {
    pub(crate) name: String,
    pub(crate) label: Option<String>,
    pub(crate) query: Query,
    pub(crate) current: Option<VariableSelection>,
    pub(crate) datasource: Option<DataSource>,
    pub(crate) multi: bool,
    pub(crate) include_all: bool,
    pub(crate) all_value: Option<String>,
    pub(crate) allow_custom_value: bool,
    pub(crate) skip_url_sync: bool,
    pub(crate) regex: String,
    pub(crate) sort: VariableSort,
    pub(crate) hidden: bool,
    pub(crate) refresh: VariableRefresh,
    /// Problems `VariableBuilder` found while producing this variable.
    /// Never serialized; validation reports them.
    pub(crate) diagnostics: Vec<VariableDiagnostic>,
}

impl QueryVariable {
    /// Creates a datasource query variable.
    #[must_use]
    pub fn new(name: impl Into<String>, query: impl Into<Query>) -> Self {
        Self {
            diagnostics: Vec::new(),
            name: name.into(),
            label: None,
            query: query.into(),
            current: None,
            datasource: None,
            multi: false,
            include_all: false,
            all_value: None,
            allow_custom_value: false,
            skip_url_sync: false,
            regex: String::new(),
            sort: VariableSort::AlphabeticalAscending,
            hidden: false,
            refresh: VariableRefresh::OnDashboardLoad,
        }
    }

    /// Sets the visible label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the selected value saved in generated JSON.
    #[must_use]
    pub fn default(mut self, value: impl Into<String>) -> Self {
        let value = value.into();
        self.current = Some(VariableSelection::new(value.clone(), value));
        self
    }

    /// Sets the complete persisted selection.
    #[must_use]
    pub fn current(mut self, current: VariableSelection) -> Self {
        self.current = Some(current);
        self
    }

    /// Sets a datasource specifically for this variable.
    #[must_use]
    pub fn datasource(mut self, datasource: DataSource) -> Self {
        self.datasource = Some(datasource);
        self
    }

    /// Enables multiple selection.
    #[must_use]
    pub fn multi(mut self, multi: bool) -> Self {
        self.multi = multi;
        self
    }

    /// Adds Grafana's `All` selection.
    #[must_use]
    pub fn include_all(mut self, include_all: bool) -> Self {
        self.include_all = include_all;
        self
    }

    /// Sets the interpolated value used for Grafana's `All` selection.
    #[must_use]
    pub fn all_value(mut self, all_value: impl Into<String>) -> Self {
        self.all_value = Some(all_value.into());
        self
    }

    /// Allows values not returned by the datasource query.
    #[must_use]
    pub fn allow_custom_value(mut self, allow_custom_value: bool) -> Self {
        self.allow_custom_value = allow_custom_value;
        self
    }

    /// Excludes the variable from dashboard URLs.
    #[must_use]
    pub fn skip_url_sync(mut self, skip_url_sync: bool) -> Self {
        self.skip_url_sync = skip_url_sync;
        self
    }

    /// Filters values returned by the datasource query.
    #[must_use]
    pub fn regex(mut self, regex: impl Into<String>) -> Self {
        self.regex = regex.into();
        self
    }

    /// Controls how returned values are sorted.
    #[must_use]
    pub fn sort(mut self, sort: VariableSort) -> Self {
        self.sort = sort;
        self
    }

    /// Hides the variable control.
    #[must_use]
    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// Controls automatic refresh.
    #[must_use]
    pub fn refresh(mut self, refresh: VariableRefresh) -> Self {
        self.refresh = refresh;
        self
    }
}

/// A dashboard variable with a fixed list of selectable values.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomVariable {
    pub(crate) name: String,
    pub(crate) label: Option<String>,
    pub(crate) values: Vec<String>,
    pub(crate) current: Option<VariableSelection>,
    pub(crate) multi: bool,
    pub(crate) include_all: bool,
    pub(crate) all_value: Option<String>,
    pub(crate) allow_custom_value: bool,
    pub(crate) skip_url_sync: bool,
    pub(crate) hidden: bool,
    /// Problems `VariableBuilder` found while producing this variable.
    /// Never serialized; validation reports them.
    pub(crate) diagnostics: Vec<VariableDiagnostic>,
}

impl CustomVariable {
    /// Creates a custom variable.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            diagnostics: Vec::new(),
            name: name.into(),
            label: None,
            values: values.into_iter().map(Into::into).collect(),
            current: None,
            multi: false,
            include_all: false,
            all_value: None,
            allow_custom_value: false,
            skip_url_sync: false,
            hidden: false,
        }
    }

    /// Sets the visible label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the selected value saved in generated JSON.
    #[must_use]
    pub fn default(mut self, value: impl Into<String>) -> Self {
        let value = value.into();
        self.current = Some(VariableSelection::new(value.clone(), value));
        self
    }

    /// Sets the complete persisted selection.
    #[must_use]
    pub fn current(mut self, current: VariableSelection) -> Self {
        self.current = Some(current);
        self
    }

    /// Enables multiple selection.
    #[must_use]
    pub fn multi(mut self, multi: bool) -> Self {
        self.multi = multi;
        self
    }

    /// Adds Grafana's `All` selection.
    #[must_use]
    pub fn include_all(mut self, include_all: bool) -> Self {
        self.include_all = include_all;
        self
    }

    /// Sets the interpolated value used for Grafana's `All` selection.
    #[must_use]
    pub fn all_value(mut self, all_value: impl Into<String>) -> Self {
        self.all_value = Some(all_value.into());
        self
    }

    /// Allows values outside the configured list.
    #[must_use]
    pub fn allow_custom_value(mut self, allow_custom_value: bool) -> Self {
        self.allow_custom_value = allow_custom_value;
        self
    }

    /// Excludes the variable from dashboard URLs.
    #[must_use]
    pub fn skip_url_sync(mut self, skip_url_sync: bool) -> Self {
        self.skip_url_sync = skip_url_sync;
        self
    }

    /// Hides the variable control.
    #[must_use]
    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }
}

/// A dashboard variable with one constant value.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstantVariable {
    pub(crate) name: String,
    pub(crate) label: Option<String>,
    pub(crate) value: String,
    pub(crate) hidden: bool,
    /// Problems `VariableBuilder` found while producing this variable.
    /// Never serialized; validation reports them.
    pub(crate) diagnostics: Vec<VariableDiagnostic>,
}

impl ConstantVariable {
    /// Creates a constant variable.
    #[must_use]
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            diagnostics: Vec::new(),
            name: name.into(),
            label: None,
            value: value.into(),
            hidden: false,
        }
    }

    /// Sets the visible label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Hides the variable control.
    #[must_use]
    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }
}

/// Any supported dashboard variable.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Variable {
    /// Grafana datasource instance selector.
    DataSource(DataSourceVariable),
    /// Datasource query variable.
    Query(Box<QueryVariable>),
    /// Fixed list variable.
    Custom(CustomVariable),
    /// Constant variable.
    Constant(ConstantVariable),
}

impl Variable {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::DataSource(variable) => &variable.name,
            Self::Query(variable) => &variable.name,
            Self::Custom(variable) => &variable.name,
            Self::Constant(variable) => &variable.name,
        }
    }
}

impl From<DataSourceVariable> for Variable {
    fn from(value: DataSourceVariable) -> Self {
        Self::DataSource(value)
    }
}

impl From<QueryVariable> for Variable {
    fn from(value: QueryVariable) -> Self {
        Self::Query(Box::new(value))
    }
}

impl From<CustomVariable> for Variable {
    fn from(value: CustomVariable) -> Self {
        Self::Custom(value)
    }
}

impl From<ConstantVariable> for Variable {
    fn from(value: ConstantVariable) -> Self {
        Self::Constant(value)
    }
}

/// A flexible helper used by [`crate::dashboard!`] to select a typed variable.
///
/// Prefer [`QueryVariable`], [`CustomVariable`], or [`ConstantVariable`] in
/// builder-oriented application code.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableBuilder {
    name: String,
    label: Option<String>,
    query: Option<Query>,
    values: Vec<String>,
    constant: Option<String>,
    default: Option<String>,
    datasource: Option<DataSource>,
    multi: bool,
    include_all: bool,
    hidden: bool,
    refresh: VariableRefresh,
    plugin: Option<String>,
    regex: Option<String>,
    sort: Option<VariableSort>,
    all_value: Option<String>,
    allow_custom_value: bool,
    skip_url_sync: bool,
    current: Option<VariableSelection>,
}

impl VariableBuilder {
    /// Starts a macro-friendly variable definition.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: None,
            query: None,
            values: Vec::new(),
            constant: None,
            default: None,
            datasource: None,
            multi: false,
            include_all: false,
            hidden: false,
            refresh: VariableRefresh::OnDashboardLoad,
            plugin: None,
            regex: None,
            sort: None,
            all_value: None,
            allow_custom_value: false,
            skip_url_sync: false,
            current: None,
        }
    }

    /// Selects a query variable.
    #[must_use]
    pub fn query(mut self, query: impl Into<Query>) -> Self {
        self.query = Some(query.into());
        self
    }

    /// Selects a custom variable.
    #[must_use]
    pub fn values(mut self, values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.values = values.into_iter().map(Into::into).collect();
        self
    }

    /// Selects a constant variable.
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.constant = Some(value.into());
        self
    }

    /// Sets a visible label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the selected value.
    #[must_use]
    pub fn default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Sets the datasource.
    #[must_use]
    pub fn datasource(mut self, datasource: DataSource) -> Self {
        self.datasource = Some(datasource);
        self
    }

    /// Enables multiple values.
    #[must_use]
    pub fn multi(mut self, multi: bool) -> Self {
        self.multi = multi;
        self
    }

    /// Enables the `All` option.
    #[must_use]
    pub fn include_all(mut self, include_all: bool) -> Self {
        self.include_all = include_all;
        self
    }

    /// Hides the variable.
    #[must_use]
    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// Sets query refresh behavior.
    #[must_use]
    pub fn refresh(mut self, refresh: VariableRefresh) -> Self {
        self.refresh = refresh;
        self
    }

    /// Selects a datasource variable over the given Grafana plugin id
    /// (for example `"prometheus"` or `"loki"`).
    ///
    /// Distinct from [`Self::datasource`], which sets the datasource a
    /// *query* variable runs against. This one chooses the variable's kind.
    #[must_use]
    pub fn plugin(mut self, plugin_id: impl Into<String>) -> Self {
        self.plugin = Some(plugin_id.into());
        self
    }

    /// Restricts a query or datasource variable's values with a regex.
    ///
    /// Custom and constant variables have no `regex` key in Grafana, so
    /// setting this on one is an authoring mistake rather than a no-op, and
    /// validation reports it.
    #[must_use]
    pub fn regex(mut self, regex: impl Into<String>) -> Self {
        self.regex = Some(regex.into());
        self
    }

    /// Orders a query variable's values.
    ///
    /// Reported by validation on kinds with no `sort` key, same as
    /// [`Self::regex`].
    #[must_use]
    pub fn sort(mut self, sort: VariableSort) -> Self {
        self.sort = Some(sort);
        self
    }

    /// Sets the value substituted when "All" is selected.
    #[must_use]
    pub fn all_value(mut self, all_value: impl Into<String>) -> Self {
        self.all_value = Some(all_value.into());
        self
    }

    /// Allows values typed by the viewer that the query did not return.
    #[must_use]
    pub fn allow_custom_value(mut self, allow_custom_value: bool) -> Self {
        self.allow_custom_value = allow_custom_value;
        self
    }

    /// Keeps the variable's value out of the dashboard URL.
    #[must_use]
    pub fn skip_url_sync(mut self, skip_url_sync: bool) -> Self {
        self.skip_url_sync = skip_url_sync;
        self
    }

    /// Sets the initially selected value, overriding [`Self::default`].
    #[must_use]
    pub fn current(mut self, current: VariableSelection) -> Self {
        self.current = Some(current);
        self
    }

    /// Resolves the accumulated fields into a concrete variable.
    ///
    /// The kind is chosen by which selector was set: `query`, `plugin`,
    /// `value` (constant), or `values` (custom). A builder with none of them,
    /// or with more than one, still produces a variable rather than panicking
    /// inside a macro expansion — it records the problem, and validation
    /// turns it into an error before serialization.
    #[must_use]
    pub fn build(self) -> Variable {
        // The selector carries its own payload, so the kind is decided exactly
        // once and there is no second match to fall out of step with the
        // first. The previous shape was a chain ending in a catch-all `else`,
        // which silently turned anything unrecognised into an empty custom
        // variable — that is how asking for a datasource variable produced a
        // custom one with no warning.
        enum Selected {
            Query(Box<Query>),
            Plugin(String),
            Constant(String),
            Custom(Vec<String>),
        }

        let mut diagnostics = Vec::new();
        let mut candidates: Vec<(&'static str, Selected)> = Vec::new();
        if let Some(query) = self.query {
            candidates.push(("query", Selected::Query(Box::new(query))));
        }
        if let Some(plugin) = self.plugin {
            candidates.push(("plugin", Selected::Plugin(plugin)));
        }
        if let Some(value) = self.constant {
            candidates.push(("value", Selected::Constant(value)));
        }
        if !self.values.is_empty() {
            candidates.push(("values", Selected::Custom(self.values)));
        }

        let mut candidates = candidates.into_iter();
        let Some((chosen, selected)) = candidates.next() else {
            diagnostics.push(VariableDiagnostic::NoSelector);
            return Variable::Custom(CustomVariable {
                diagnostics,
                name: self.name,
                label: self.label,
                values: Vec::new(),
                current: None,
                multi: false,
                include_all: false,
                all_value: None,
                allow_custom_value: false,
                skip_url_sync: false,
                hidden: self.hidden,
            });
        };
        for (ignored, _) in candidates {
            diagnostics.push(VariableDiagnostic::ConflictingSelectors { chosen, ignored });
        }

        let mut not_applicable = |set: bool, option: &'static str| {
            if set {
                diagnostics.push(VariableDiagnostic::OptionNotApplicable(option));
            }
        };
        match selected {
            Selected::Query(_) => {}
            Selected::Plugin(_) => {
                not_applicable(self.sort.is_some(), "sort");
                not_applicable(self.all_value.is_some(), "all_value");
                not_applicable(self.allow_custom_value, "allow_custom_value");
                not_applicable(self.multi, "multi");
                not_applicable(self.include_all, "include_all");
            }
            Selected::Constant(_) | Selected::Custom(_) => {
                not_applicable(self.regex.is_some(), "regex");
                not_applicable(self.sort.is_some(), "sort");
            }
        }

        let current = self.current.or_else(|| {
            self.default
                .map(|value| VariableSelection::new(value.clone(), value))
        });

        match selected {
            Selected::Query(query) => Variable::Query(Box::new(QueryVariable {
                diagnostics,
                name: self.name,
                label: self.label,
                query: *query,
                current,
                datasource: self.datasource,
                multi: self.multi,
                include_all: self.include_all,
                all_value: self.all_value,
                allow_custom_value: self.allow_custom_value,
                skip_url_sync: self.skip_url_sync,
                regex: self.regex.unwrap_or_default(),
                // Deliberately not `VariableSort::default()` (Disabled): this
                // builder has always defaulted query variables to
                // alphabetical, and switching now would silently reorder the
                // values in every existing dashboard.
                sort: self.sort.unwrap_or(VariableSort::AlphabeticalAscending),
                hidden: self.hidden,
                refresh: self.refresh,
            })),
            Selected::Plugin(plugin_id) => Variable::DataSource(DataSourceVariable {
                diagnostics,
                name: self.name,
                label: self.label,
                plugin_id,
                current,
                regex: self.regex.unwrap_or_default(),
                refresh: self.refresh,
                hidden: self.hidden,
                skip_url_sync: self.skip_url_sync,
            }),
            Selected::Constant(value) => Variable::Constant(ConstantVariable {
                diagnostics,
                name: self.name,
                label: self.label,
                value,
                hidden: self.hidden,
            }),
            Selected::Custom(values) => Variable::Custom(CustomVariable {
                diagnostics,
                name: self.name,
                label: self.label,
                values,
                current,
                multi: self.multi,
                include_all: self.include_all,
                all_value: self.all_value,
                allow_custom_value: self.allow_custom_value,
                skip_url_sync: self.skip_url_sync,
                hidden: self.hidden,
            }),
        }
    }
}
