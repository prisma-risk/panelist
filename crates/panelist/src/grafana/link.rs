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

use serde::Serialize;

use crate::DataLink;

/// A panel or field data link as Grafana stores it.
///
/// Mirrors `DataLink` in `packages/grafana-data/src/types/dataLink.ts` at Grafana v13.0.2, minus the callback-only and internal members, which are never saved. `targetBlank` is omitted when false: Grafana reads a missing key as "same tab".
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GrafanaDataLink {
    pub(crate) title: String,
    pub(crate) url: String,
    #[serde(skip_serializing_if = "is_false")]
    pub(crate) target_blank: bool,
}

const fn is_false(value: &bool) -> bool {
    !*value
}

impl From<&DataLink> for GrafanaDataLink {
    fn from(link: &DataLink) -> Self {
        // Same order and spelling as Grafana's `getLinkUrl` for dashboard links: time range first, then variables.
        let mut url = link.url.clone();
        if link.keep_time {
            append_query(&mut url, "$__url_time_range");
        }
        if link.include_vars {
            append_query(&mut url, "$__all_variables");
        }
        Self {
            title: link.title.clone(),
            url,
            target_blank: link.target_blank,
        }
    }
}

/// Grafana's `urlUtil.appendQueryToUrl` (`packages/grafana-data/src/utils/url.ts`).
fn append_query(url: &mut String, query: &str) {
    match url.find('?') {
        Some(position) if url.len() - position > 1 => url.push('&'),
        Some(_) => {}
        None => url.push('?'),
    }
    url.push_str(query);
}

pub(crate) fn data_links_value(links: &[DataLink]) -> serde_json::Value {
    serde_json::to_value(links.iter().map(GrafanaDataLink::from).collect::<Vec<_>>())
        .unwrap_or(serde_json::Value::Null)
}
