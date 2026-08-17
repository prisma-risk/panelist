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

use crate::{Dashboard, DashboardItem, ValidationErrors};

mod field;
mod layout;
mod panel;
mod query;
mod validate;
mod variable;
mod wire;

pub(crate) use wire::NormalizedDashboard;

use layout::{FlowLayout, IdAllocator, explicit_panel_ids};
use panel::{normalize_panel, normalize_row};
use validate::validate;
use variable::normalize_variable;
use wire::{GrafanaAnnotations, GrafanaLink, GrafanaTemplating, GrafanaTimeRange};

pub(crate) fn normalize(dashboard: &Dashboard) -> Result<NormalizedDashboard, ValidationErrors> {
    validate(dashboard)?;

    let mut allocator = IdAllocator::new(explicit_panel_ids(dashboard));
    let mut panels = Vec::new();
    let mut current_y = 0;
    let mut top_level_flow = FlowLayout::new(current_y);

    for item in &dashboard.items {
        match item {
            DashboardItem::Panel(panel) => {
                let grid = top_level_flow.place(panel);
                let id = allocator.panel_id(panel.id);
                panels.push(normalize_panel(
                    panel,
                    id,
                    grid,
                    dashboard.datasource.as_ref(),
                ));
                current_y = top_level_flow.bottom();
            }
            DashboardItem::Row(row) => {
                current_y = top_level_flow.bottom().max(current_y);
                let row_y = current_y;
                let row_id = allocator.panel_id(None);
                let mut row_flow = FlowLayout::new(row_y.saturating_add(1));
                let mut row_panels = Vec::with_capacity(row.panels.len());

                for panel in &row.panels {
                    let grid = row_flow.place(panel);
                    let id = allocator.panel_id(panel.id);
                    row_panels.push(normalize_panel(
                        panel,
                        id,
                        grid,
                        dashboard.datasource.as_ref(),
                    ));
                }

                let row_bottom = row_flow.bottom();
                if row.collapsed {
                    panels.push(normalize_row(row, row_id, row_y, row_panels));
                } else {
                    panels.push(normalize_row(row, row_id, row_y, Vec::new()));
                    panels.extend(row_panels);
                }

                current_y = if row.collapsed {
                    row_y.saturating_add(1)
                } else {
                    row_bottom.max(row_y.saturating_add(1))
                };
                top_level_flow = FlowLayout::new(current_y);
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
        annotations: GrafanaAnnotations { list: Vec::new() },
        refresh: dashboard.refresh.clone(),
        schema_version: dashboard.schema_version,
        version: dashboard.version,
        links: dashboard.links.iter().map(GrafanaLink::from).collect(),
        extra: dashboard.extra.clone(),
    })
}
