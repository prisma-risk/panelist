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

use crate::{Dashboard, ValidationErrors, resolved::ResolvedItem};

mod annotation;
mod field;
mod layout;
mod link;
mod panel;
mod query;
mod references;
mod resolve;
mod transform;
mod validate;
mod validate_options;
mod variable;
mod vocabulary;
mod wire;

pub(crate) use query::effective_ref_ids;
pub(crate) use resolve::resolve;
pub(crate) use wire::NormalizedDashboard;

use panel::{normalize_panel, normalize_row};
use variable::normalize_variable;
use wire::{GrafanaAnnotations, GrafanaLink, GrafanaTemplating, GrafanaTimeRange};

pub(crate) fn normalize(dashboard: &Dashboard) -> Result<NormalizedDashboard, ValidationErrors> {
    let resolved = resolve(dashboard)?;
    let mut panels = Vec::new();

    for item in resolved.items() {
        match item {
            ResolvedItem::Panel(panel) => panels.push(normalize_panel(panel)),
            ResolvedItem::Row(row) => {
                let row_panels = row.panels().iter().map(normalize_panel);
                // Grafana Classic nests a collapsed row's panels inside the row object and lists an expanded row's panels after it.
                if row.collapsed() {
                    panels.push(normalize_row(row, row_panels.collect()));
                } else {
                    panels.push(normalize_row(row, Vec::new()));
                    panels.extend(row_panels);
                }
            }
        }
    }

    Ok(NormalizedDashboard {
        id: None,
        uid: dashboard.uid.clone(),
        title: dashboard.title.clone(),
        description: dashboard.description.clone(),
        tags: dashboard.tags.clone(),
        timezone: dashboard.timezone.clone(),
        editable: dashboard.editable,
        graph_tooltip: dashboard.cursor_sync.as_grafana(),
        panels,
        time: GrafanaTimeRange {
            from: dashboard.time.from.clone(),
            to: dashboard.time.to.clone(),
        },
        timepicker: BTreeMap::new(),
        templating: GrafanaTemplating {
            list: dashboard
                .variables
                .iter()
                .map(|variable| normalize_variable(variable, dashboard.datasource.as_ref()))
                .collect(),
        },
        annotations: GrafanaAnnotations {
            list: annotation::normalize_annotations(
                dashboard.built_in_annotation.as_ref(),
                &dashboard.annotations,
                dashboard.datasource.as_ref(),
            ),
        },
        refresh: dashboard.refresh.clone(),
        schema_version: dashboard.effective_schema_version(),
        version: dashboard.version,
        links: dashboard.links.iter().map(GrafanaLink::from).collect(),
        extra: dashboard.extra.clone(),
    })
}
