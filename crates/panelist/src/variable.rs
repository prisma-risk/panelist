// SPDX-License-Identifier: Apache-2.0

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

/// A datasource-backed dashboard variable.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryVariable {
    pub(crate) name: String,
    pub(crate) label: Option<String>,
    pub(crate) query: Query,
    pub(crate) default: Option<String>,
    pub(crate) datasource: Option<DataSource>,
    pub(crate) multi: bool,
    pub(crate) include_all: bool,
    pub(crate) hidden: bool,
    pub(crate) refresh: VariableRefresh,
}

impl QueryVariable {
    /// Creates a datasource query variable.
    #[must_use]
    pub fn new(name: impl Into<String>, query: impl Into<Query>) -> Self {
        Self {
            name: name.into(),
            label: None,
            query: query.into(),
            default: None,
            datasource: None,
            multi: false,
            include_all: false,
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
        self.default = Some(value.into());
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
    pub(crate) default: Option<String>,
    pub(crate) multi: bool,
    pub(crate) include_all: bool,
    pub(crate) hidden: bool,
}

impl CustomVariable {
    /// Creates a custom variable.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            label: None,
            values: values.into_iter().map(Into::into).collect(),
            default: None,
            multi: false,
            include_all: false,
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
        self.default = Some(value.into());
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
}

impl ConstantVariable {
    /// Creates a constant variable.
    #[must_use]
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
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
            Self::Query(variable) => &variable.name,
            Self::Custom(variable) => &variable.name,
            Self::Constant(variable) => &variable.name,
        }
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

    /// Produces the selected typed variable. If no selector was supplied, an
    /// empty custom variable is emitted so validation and Grafana can surface
    /// the incomplete definition without a macro panic.
    #[must_use]
    pub fn build(self) -> Variable {
        if let Some(query) = self.query {
            Variable::Query(Box::new(QueryVariable {
                name: self.name,
                label: self.label,
                query,
                default: self.default,
                datasource: self.datasource,
                multi: self.multi,
                include_all: self.include_all,
                hidden: self.hidden,
                refresh: self.refresh,
            }))
        } else if let Some(value) = self.constant {
            Variable::Constant(ConstantVariable {
                name: self.name,
                label: self.label,
                value,
                hidden: self.hidden,
            })
        } else {
            Variable::Custom(CustomVariable {
                name: self.name,
                label: self.label,
                values: self.values,
                default: self.default,
                multi: self.multi,
                include_all: self.include_all,
                hidden: self.hidden,
            })
        }
    }
}
