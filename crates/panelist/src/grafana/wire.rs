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

use serde::Serialize;
use serde_json::Value;

use crate::{DashboardLink, DataSource, GridPos};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NormalizedDashboard {
    pub(crate) id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) uid: Option<String>,
    pub(crate) title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) timezone: String,
    pub(crate) editable: bool,
    pub(crate) graph_tooltip: u8,
    pub(crate) panels: Vec<GrafanaPanel>,
    pub(crate) time: GrafanaTimeRange,
    pub(crate) timepicker: BTreeMap<String, Value>,
    pub(crate) templating: GrafanaTemplating,
    pub(crate) annotations: GrafanaAnnotations,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) refresh: Option<String>,
    pub(crate) schema_version: u32,
    pub(crate) version: u32,
    pub(crate) links: Vec<GrafanaLink>,
    #[serde(flatten)]
    pub(crate) extra: BTreeMap<String, Value>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GrafanaTimeRange {
    pub(crate) from: String,
    pub(crate) to: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct GrafanaTemplating {
    pub(crate) list: Vec<BTreeMap<String, Value>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GrafanaAnnotations {
    pub(crate) list: Vec<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GrafanaLink {
    #[serde(rename = "type")]
    pub(crate) kind: &'static str,
    pub(crate) title: String,
    pub(crate) url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tooltip: Option<String>,
    pub(crate) target_blank: bool,
    pub(crate) include_vars: bool,
    pub(crate) keep_time: bool,
    pub(crate) tags: Vec<String>,
}

impl From<&DashboardLink> for GrafanaLink {
    fn from(link: &DashboardLink) -> Self {
        Self {
            kind: "link",
            title: link.title.clone(),
            url: link.url.clone(),
            tooltip: link.tooltip.clone(),
            target_blank: link.target_blank,
            include_vars: link.include_vars,
            keep_time: link.keep_time,
            tags: link.tags.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GrafanaPanel {
    pub(crate) id: u32,
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    pub(crate) grid_pos: GrafanaGridPos,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) datasource: Option<DataSource>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) targets: Vec<GrafanaTarget>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) transformations: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) field_config: Option<GrafanaFieldConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) options: Option<BTreeMap<String, Value>>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) transparent: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) links: Vec<GrafanaLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) collapsed: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) panels: Vec<GrafanaPanel>,
    #[serde(flatten)]
    pub(crate) extra: BTreeMap<String, Value>,
}

const fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct GrafanaGridPos {
    pub(crate) x: u16,
    pub(crate) y: u16,
    pub(crate) w: u16,
    pub(crate) h: u16,
}

impl From<GridPos> for GrafanaGridPos {
    fn from(value: GridPos) -> Self {
        Self {
            x: value.x,
            y: value.y,
            w: value.width,
            h: value.height,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GrafanaTarget {
    pub(crate) ref_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) datasource: Option<DataSource>,
    pub(crate) expr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) format: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) editor_mode: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) legend_format: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) instant: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) range: Option<bool>,
    pub(crate) hide: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) interval: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) query_type: Option<&'static str>,
    #[serde(flatten)]
    pub(crate) extra: BTreeMap<String, Value>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GrafanaFieldConfig {
    pub(crate) defaults: BTreeMap<String, Value>,
    pub(crate) overrides: Vec<GrafanaFieldOverride>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GrafanaFieldOverride {
    pub(crate) matcher: GrafanaMatcher,
    pub(crate) properties: Vec<GrafanaOverrideProperty>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GrafanaMatcher {
    pub(crate) id: &'static str,
    pub(crate) options: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct GrafanaOverrideProperty {
    pub(crate) id: String,
    pub(crate) value: Value,
}
