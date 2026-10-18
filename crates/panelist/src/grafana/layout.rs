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

use std::collections::{HashMap, HashSet};

use crate::{Dashboard, DashboardItem, GridPos, Panel, PanelIds, Row, ValidationError};

pub(crate) fn reference_id(mut index: usize) -> String {
    let mut bytes = Vec::new();
    loop {
        bytes.push(b'A' + (index % 26) as u8);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    bytes.reverse();
    bytes.into_iter().map(char::from).collect()
}

/// Assigns the ID of every panel and row, in emission order: for each top-level item, a panel's ID, or a row's ID followed by its panels' IDs.
///
/// Errors are authoring mistakes only this pass can see: a blank key, and, under [`PanelIds::Stable`], two panels or rows sharing an ID.
pub(crate) fn assign_panel_ids(dashboard: &Dashboard) -> Result<Vec<u32>, Vec<ValidationError>> {
    let mut errors = blank_key_errors(dashboard);
    let ids = match dashboard.panel_ids {
        PanelIds::Positional => positional_ids(dashboard),
        PanelIds::Stable => stable_ids(dashboard, &mut errors),
    };
    if errors.is_empty() {
        Ok(ids)
    } else {
        Err(errors)
    }
}

fn blank_key_errors(dashboard: &Dashboard) -> Vec<ValidationError> {
    fn is_blank(key: Option<&String>) -> bool {
        key.is_some_and(|key| key.trim().is_empty())
    }

    let mut errors = Vec::new();
    for item in &dashboard.items {
        let panels = match item {
            DashboardItem::Panel(panel) => std::slice::from_ref(&**panel),
            DashboardItem::Row(row) => {
                if is_blank(row.key.as_ref()) {
                    errors.push(ValidationError::BlankRowKey {
                        row: row.title.clone(),
                    });
                }
                row.panels.as_slice()
            }
        };
        for panel in panels {
            if is_blank(panel.key.as_ref()) {
                errors.push(ValidationError::BlankPanelKey {
                    panel: panel.title.clone(),
                });
            }
        }
    }
    errors
}

fn positional_ids(dashboard: &Dashboard) -> Vec<u32> {
    let reserved = dashboard
        .items
        .iter()
        .flat_map(|item| match item {
            DashboardItem::Panel(panel) => std::slice::from_ref(&**panel),
            DashboardItem::Row(row) => row.panels.as_slice(),
        })
        .filter_map(|panel| panel.id)
        .collect();
    let mut allocator = IdAllocator::new(reserved);
    let mut ids = Vec::new();
    for item in &dashboard.items {
        match item {
            DashboardItem::Panel(panel) => ids.push(allocator.panel_id(panel.id)),
            DashboardItem::Row(row) => {
                ids.push(allocator.panel_id(None));
                ids.extend(row.panels.iter().map(|panel| allocator.panel_id(panel.id)));
            }
        }
    }
    ids
}

struct IdAllocator {
    used: HashSet<u32>,
    next: u32,
}

impl IdAllocator {
    fn new(reserved: HashSet<u32>) -> Self {
        Self {
            used: reserved,
            next: 1,
        }
    }

    fn panel_id(&mut self, explicit: Option<u32>) -> u32 {
        if let Some(explicit) = explicit {
            return explicit;
        }
        while self.used.contains(&self.next) {
            self.next = self.next.saturating_add(1);
        }
        let id = self.next;
        self.used.insert(id);
        self.next = self.next.saturating_add(1);
        id
    }
}

/// The first ID [`PanelIds::Stable`] derives. IDs below it are left to authors who pin small IDs by hand.
pub(crate) const STABLE_ID_START: u64 = 1 << 16;
/// One past the last ID [`PanelIds::Stable`] derives, leaving the rest of Grafana's `uint32` range for panels Grafana's editor adds above the highest ID.
pub(crate) const STABLE_ID_END: u64 = 1 << 30;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Derives a stable panel ID from identity fields, exactly as documented on [`PanelIds::Stable`].
///
/// This is a fixed, hand-written 64-bit FNV-1a rather than `std::hash`, whose output is not guaranteed across Rust releases. Each field is prefixed with its length so that no two field lists encode to the same bytes.
pub(crate) fn stable_id(fields: &[&str]) -> u32 {
    let hash = fnv1a(fields.iter().flat_map(|field| {
        (field.len() as u64)
            .to_le_bytes()
            .into_iter()
            .chain(field.bytes())
    }));
    let id = STABLE_ID_START + hash % (STABLE_ID_END - STABLE_ID_START);
    u32::try_from(id).unwrap_or(u32::MAX)
}

fn fnv1a(bytes: impl IntoIterator<Item = u8>) -> u64 {
    bytes.into_iter().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
    })
}

/// The fields a row contributes to its own identity and to its unkeyed panels'.
fn row_identity(row: &Row) -> [&str; 2] {
    match &row.key {
        Some(key) => ["key", key],
        None => ["title", &row.title],
    }
}

fn row_site(row: &Row) -> String {
    match &row.key {
        Some(key) => format!("row \"{}\" (key \"{key}\")", row.title),
        None => format!("row \"{}\"", row.title),
    }
}

fn panel_site(panel: &Panel, row: Option<&Row>) -> String {
    let mut site = format!("panel \"{}\"", panel.title);
    if let Some(row) = row {
        site.push_str(&format!(" in row \"{}\"", row.title));
    }
    if panel.id.is_some() {
        site.push_str(" (explicit ID)");
    } else if let Some(key) = &panel.key {
        site.push_str(&format!(" (key \"{key}\")"));
    }
    site
}

fn stable_panel_id(panel: &Panel, row: Option<&Row>) -> u32 {
    if let Some(id) = panel.id {
        return id;
    }
    if let Some(key) = &panel.key {
        return stable_id(&["key", key]);
    }
    match row {
        Some(row) => {
            let [kind, value] = row_identity(row);
            stable_id(&["panel", kind, value, &panel.title])
        }
        None => stable_id(&["panel", &panel.title]),
    }
}

fn stable_ids(dashboard: &Dashboard, errors: &mut Vec<ValidationError>) -> Vec<u32> {
    // Each ID's first claimant, and whether it was explicit. Two explicit IDs that clash are already a `DuplicatePanelId`, so only clashes involving a derived ID are reported here.
    let mut claims: HashMap<u32, (String, bool)> = HashMap::new();
    let mut ids = Vec::new();
    let mut claim = |id: u32, site: String, explicit: bool| {
        match claims.get(&id) {
            Some((first, first_explicit)) => {
                if !(explicit && *first_explicit) {
                    errors.push(ValidationError::PanelIdCollision {
                        id,
                        first: first.clone(),
                        second: site,
                    });
                }
            }
            None => {
                claims.insert(id, (site, explicit));
            }
        }
        ids.push(id);
    };
    for item in &dashboard.items {
        match item {
            DashboardItem::Panel(panel) => {
                claim(
                    stable_panel_id(panel, None),
                    panel_site(panel, None),
                    panel.id.is_some(),
                );
            }
            DashboardItem::Row(row) => {
                let [kind, value] = row_identity(row);
                claim(stable_id(&["row", kind, value]), row_site(row), false);
                for panel in &row.panels {
                    claim(
                        stable_panel_id(panel, Some(row)),
                        panel_site(panel, Some(row)),
                        panel.id.is_some(),
                    );
                }
            }
        }
    }
    ids
}

pub(crate) struct FlowLayout {
    x: u16,
    y: u16,
    line_height: u16,
    max_bottom: u16,
}

impl FlowLayout {
    pub(crate) const fn new(y: u16) -> Self {
        Self {
            x: 0,
            y,
            line_height: 0,
            max_bottom: y,
        }
    }

    pub(crate) fn place(&mut self, panel: &Panel) -> GridPos {
        if let Some(explicit) = panel.grid_pos {
            self.max_bottom = self
                .max_bottom
                .max(explicit.y.saturating_add(explicit.height));
            self.y = self.max_bottom;
            self.x = 0;
            self.line_height = 0;
            return explicit;
        }

        let width = panel.layout_width();
        if self.x > 0 && self.x.saturating_add(width) > 24 {
            self.y = self.y.saturating_add(self.line_height);
            self.x = 0;
            self.line_height = 0;
        }
        let grid = GridPos::new(self.x, self.y, width, panel.height);
        self.x = self.x.saturating_add(width);
        self.line_height = self.line_height.max(panel.height);
        self.max_bottom = self.max_bottom.max(self.y.saturating_add(self.line_height));
        if self.x == 24 {
            self.y = self.y.saturating_add(self.line_height);
            self.x = 0;
            self.line_height = 0;
        }
        grid
    }

    pub(crate) fn bottom(&self) -> u16 {
        self.max_bottom.max(self.y.saturating_add(self.line_height))
    }
}

#[cfg(test)]
mod tests {
    use super::fnv1a;

    /// Reference vectors from the FNV specification's test suite.
    #[test]
    fn fnv1a_matches_the_reference_vectors() {
        assert_eq!(fnv1a(*b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(*b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a(*b"foobar"), 0x8594_4171_f739_67e8);
    }
}
