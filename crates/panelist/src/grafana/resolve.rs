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

use crate::{
    Dashboard, DashboardItem, DataSource, GridPos, Panel, ValidationErrors,
    resolved::{PanelRow, ResolvedDashboard, ResolvedItem, ResolvedPanel, ResolvedRow},
};

use super::layout::{FlowLayout, assign_panel_ids};
use super::query::resolve_queries;
use super::validate::validate;

/// Validates `dashboard` and resolves every panel ID, grid position, row membership, query reference ID, and inherited datasource.
///
/// This is the only place those values are decided. Serialization lowers the returned view to wire JSON, and [`Dashboard::resolve`] hands the same view to callers, so what a caller inspects is what Grafana receives.
pub(crate) fn resolve(dashboard: &Dashboard) -> Result<ResolvedDashboard<'_>, ValidationErrors> {
    // Both passes run before either reports, so one call lists every mistake.
    let ids = match (validate(dashboard), assign_panel_ids(dashboard)) {
        (Ok(()), Ok(ids)) => ids,
        (validated, assigned) => {
            let mut errors = validated.err().map(|errors| errors.0).unwrap_or_default();
            errors.extend(assigned.err().unwrap_or_default());
            return Err(ValidationErrors(errors));
        }
    };
    // `assign_panel_ids` yields IDs in emission order, which is the order this walk consumes them.
    let mut ids = ids.into_iter();
    let mut next_id = || ids.next().unwrap_or_default();

    let default_datasource = dashboard.datasource.as_ref();
    let mut items = Vec::with_capacity(dashboard.items.len());
    let mut current_y = 0;
    let mut top_level_flow = FlowLayout::new(current_y);

    for item in &dashboard.items {
        match item {
            DashboardItem::Panel(panel) => {
                let grid = top_level_flow.place(panel);
                items.push(ResolvedItem::Panel(resolve_panel(
                    panel,
                    next_id(),
                    grid,
                    None,
                    default_datasource,
                )));
                current_y = top_level_flow.bottom();
            }
            DashboardItem::Row(row) => {
                current_y = top_level_flow.bottom().max(current_y);
                let row_y = current_y;
                let row_id = next_id();
                let membership = PanelRow {
                    title: &row.title,
                    id: row_id,
                    collapsed: row.collapsed,
                };
                let mut row_flow = FlowLayout::new(row_y.saturating_add(1));
                let mut panels = Vec::with_capacity(row.panels.len());

                for panel in &row.panels {
                    let grid = row_flow.place(panel);
                    panels.push(resolve_panel(
                        panel,
                        next_id(),
                        grid,
                        Some(membership),
                        default_datasource,
                    ));
                }

                // A collapsed row only occupies its header; its panels reappear below it when expanded.
                current_y = if row.collapsed {
                    row_y.saturating_add(1)
                } else {
                    row_flow.bottom().max(row_y.saturating_add(1))
                };
                top_level_flow = FlowLayout::new(current_y);

                items.push(ResolvedItem::Row(ResolvedRow {
                    row,
                    id: row_id,
                    grid_pos: GridPos::new(0, row_y, 24, 1),
                    panels,
                }));
            }
        }
    }

    Ok(ResolvedDashboard { dashboard, items })
}

fn resolve_panel<'a>(
    panel: &'a Panel,
    id: u32,
    grid_pos: GridPos,
    row: Option<PanelRow<'a>>,
    default_datasource: Option<&'a DataSource>,
) -> ResolvedPanel<'a> {
    let datasource = panel.datasource.as_ref().or(default_datasource);
    ResolvedPanel {
        panel,
        id,
        grid_pos,
        row,
        datasource,
        queries: resolve_queries(panel, datasource),
    }
}
