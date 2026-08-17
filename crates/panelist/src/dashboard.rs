// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, fs, path::Path};

use serde::{Serialize, Serializer};
use serde_json::Value;

use crate::{DataSource, Error, Panel, Result, Variable, grafana, validation::ValidationErrors};

/// The default Classic dashboard schema emitted by Panelist.
pub const DEFAULT_SCHEMA_VERSION: u32 = 41;

/// A Grafana dashboard time range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeRange {
    pub(crate) from: String,
    pub(crate) to: String,
}

impl TimeRange {
    /// Creates a relative or absolute Grafana time range.
    #[must_use]
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}

impl Default for TimeRange {
    fn default() -> Self {
        Self::new("now-6h", "now")
    }
}

/// A dashboard or panel navigation link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardLink {
    pub(crate) title: String,
    pub(crate) url: String,
    pub(crate) tooltip: Option<String>,
    pub(crate) target_blank: bool,
    pub(crate) include_vars: bool,
    pub(crate) keep_time: bool,
    pub(crate) tags: Vec<String>,
}

impl DashboardLink {
    /// Creates a URL link.
    #[must_use]
    pub fn new(title: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            url: url.into(),
            tooltip: None,
            target_blank: false,
            include_vars: false,
            keep_time: false,
            tags: Vec::new(),
        }
    }

    /// Sets hover text.
    #[must_use]
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Opens the link in a new browser tab.
    #[must_use]
    pub fn target_blank(mut self, target_blank: bool) -> Self {
        self.target_blank = target_blank;
        self
    }

    /// Preserves dashboard variables in the generated URL.
    #[must_use]
    pub fn include_vars(mut self, include_vars: bool) -> Self {
        self.include_vars = include_vars;
        self
    }

    /// Preserves the current time range in the generated URL.
    #[must_use]
    pub fn keep_time(mut self, keep_time: bool) -> Self {
        self.keep_time = keep_time;
        self
    }

    /// Limits dashboard-discovery links to matching tags.
    #[must_use]
    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }
}

/// A semantic row that owns an ordered set of panels.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub(crate) title: String,
    pub(crate) collapsed: bool,
    pub(crate) panels: Vec<Panel>,
}

impl Row {
    /// Creates an expanded row.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            collapsed: false,
            panels: Vec::new(),
        }
    }

    /// Controls whether Grafana initially collapses this row.
    #[must_use]
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Appends one panel.
    #[must_use]
    pub fn panel(mut self, panel: impl Into<Panel>) -> Self {
        self.panels.push(panel.into());
        self
    }

    /// Appends a reusable fragment of panels.
    #[must_use]
    pub fn panels(mut self, panels: impl IntoIterator<Item = impl Into<Panel>>) -> Self {
        self.panels.extend(panels.into_iter().map(Into::into));
        self
    }
}

/// An ordered top-level dashboard item.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DashboardItem {
    /// A semantic row and its panels.
    Row(Row),
    /// A panel outside an explicit row.
    Panel(Box<Panel>),
}

/// A strongly typed dashboard authoring model.
///
/// Serialization performs validation, deterministic panel/query ID assignment,
/// and 24-column layout before producing Grafana Classic dashboard JSON.
#[derive(Debug, Clone, PartialEq)]
pub struct Dashboard {
    pub(crate) title: String,
    pub(crate) uid: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) refresh: Option<String>,
    pub(crate) time: TimeRange,
    pub(crate) timezone: String,
    pub(crate) editable: bool,
    pub(crate) schema_version: u32,
    pub(crate) datasource: Option<DataSource>,
    pub(crate) variables: Vec<Variable>,
    pub(crate) links: Vec<DashboardLink>,
    pub(crate) items: Vec<DashboardItem>,
    pub(crate) extra: BTreeMap<String, Value>,
}

impl Dashboard {
    /// Creates an empty dashboard with a six-hour time range.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            uid: None,
            description: None,
            tags: Vec::new(),
            refresh: None,
            time: TimeRange::default(),
            timezone: "browser".to_owned(),
            editable: true,
            schema_version: DEFAULT_SCHEMA_VERSION,
            datasource: None,
            variables: Vec::new(),
            links: Vec::new(),
            items: Vec::new(),
            extra: BTreeMap::new(),
        }
    }

    /// Sets a stable Grafana dashboard UID.
    #[must_use]
    pub fn uid(mut self, uid: impl Into<String>) -> Self {
        self.uid = Some(uid.into());
        self
    }

    /// Sets the dashboard description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Replaces dashboard tags.
    #[must_use]
    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Sets Grafana's auto-refresh interval, such as `30s` or `1m`.
    #[must_use]
    pub fn refresh(mut self, refresh: impl Into<String>) -> Self {
        self.refresh = Some(refresh.into());
        self
    }

    /// Sets the initial dashboard time range.
    #[must_use]
    pub fn time(mut self, time: TimeRange) -> Self {
        self.time = time;
        self
    }

    /// Sets the Grafana timezone string, commonly `browser` or `utc`.
    #[must_use]
    pub fn timezone(mut self, timezone: impl Into<String>) -> Self {
        self.timezone = timezone.into();
        self
    }

    /// Controls whether viewers can edit the dashboard.
    #[must_use]
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// Overrides the emitted Classic dashboard schema version.
    #[must_use]
    pub fn schema_version(mut self, schema_version: u32) -> Self {
        self.schema_version = schema_version;
        self
    }

    /// Sets the datasource inherited by panels, targets, and query variables.
    #[must_use]
    pub fn datasource(mut self, datasource: DataSource) -> Self {
        self.datasource = Some(datasource);
        self
    }

    /// Appends a dashboard variable.
    #[must_use]
    pub fn variable(mut self, variable: impl Into<Variable>) -> Self {
        self.variables.push(variable.into());
        self
    }

    /// Appends a dashboard link.
    #[must_use]
    pub fn link(mut self, link: DashboardLink) -> Self {
        self.links.push(link);
        self
    }

    /// Appends a semantic row.
    #[must_use]
    pub fn row(mut self, row: Row) -> Self {
        self.items.push(DashboardItem::Row(row));
        self
    }

    /// Appends a panel outside an explicit row.
    #[must_use]
    pub fn panel(mut self, panel: impl Into<Panel>) -> Self {
        self.items
            .push(DashboardItem::Panel(Box::new(panel.into())));
        self
    }

    /// Appends reusable top-level panels.
    #[must_use]
    pub fn panels(mut self, panels: impl IntoIterator<Item = impl Into<Panel>>) -> Self {
        self.items.extend(
            panels
                .into_iter()
                .map(Into::into)
                .map(Box::new)
                .map(DashboardItem::Panel),
        );
        self
    }

    /// Adds an unsupported top-level Grafana dashboard property.
    #[must_use]
    pub fn extra(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }

    /// Validates all authoring values without serializing JSON.
    pub fn validate(&self) -> std::result::Result<(), ValidationErrors> {
        grafana::normalize(self).map(|_| ())
    }

    /// Serializes compact deterministic Grafana JSON.
    pub fn to_json(&self) -> Result<String> {
        self.validate()?;
        Ok(serde_json::to_string(self)?)
    }

    /// Serializes pretty deterministic Grafana JSON.
    pub fn to_json_pretty(&self) -> Result<String> {
        self.validate()?;
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Writes pretty deterministic JSON to `path`.
    pub fn write_json(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let json = self.to_json_pretty()?;
        fs::write(path, format!("{json}\n")).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}

impl Serialize for Dashboard {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let normalized = grafana::normalize(self).map_err(serde::ser::Error::custom)?;
        normalized.serialize(serializer)
    }
}
