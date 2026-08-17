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

use std::collections::HashSet;

use crate::{Dashboard, DashboardItem, GridPos, Panel};

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

pub(crate) fn explicit_panel_ids(dashboard: &Dashboard) -> HashSet<u32> {
    let mut ids = HashSet::new();
    for item in &dashboard.items {
        match item {
            DashboardItem::Panel(panel) => {
                if let Some(id) = panel.id {
                    ids.insert(id);
                }
            }
            DashboardItem::Row(row) => {
                ids.extend(row.panels.iter().filter_map(|panel| panel.id));
            }
        }
    }
    ids
}

pub(crate) struct IdAllocator {
    used: HashSet<u32>,
    next: u32,
}

impl IdAllocator {
    pub(crate) fn new(reserved: HashSet<u32>) -> Self {
        Self {
            used: reserved,
            next: 1,
        }
    }

    pub(crate) fn panel_id(&mut self, explicit: Option<u32>) -> u32 {
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

        if self.x > 0 && self.x.saturating_add(panel.width) > 24 {
            self.y = self.y.saturating_add(self.line_height);
            self.x = 0;
            self.line_height = 0;
        }
        let grid = GridPos::new(self.x, self.y, panel.width, panel.height);
        self.x = self.x.saturating_add(panel.width);
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
