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

use crate::{DataSource, Query};

use super::layout::reference_id;
use super::wire::GrafanaTarget;

/// Returns the reference ID each query resolves to, explicit or assigned.
pub(crate) fn effective_ref_ids(queries: &[Query]) -> Vec<String> {
    let mut used: HashSet<String> = queries
        .iter()
        .filter_map(|query| query.options().ref_id.clone())
        .collect();
    let mut next = 0usize;

    queries
        .iter()
        .map(|query| {
            query.options().ref_id.clone().unwrap_or_else(|| {
                loop {
                    let candidate = reference_id(next);
                    next += 1;
                    if used.insert(candidate.clone()) {
                        break candidate;
                    }
                }
            })
        })
        .collect()
}

pub(crate) fn normalize_targets(
    queries: &[Query],
    panel_datasource: Option<&DataSource>,
) -> Vec<GrafanaTarget> {
    let ref_ids = effective_ref_ids(queries);

    queries
        .iter()
        .zip(ref_ids)
        .map(|(query, ref_id)| {
            let options = query.options();
            let query_type = match query {
                Query::Loki(_) => Some(if options.instant { "instant" } else { "range" }),
                Query::Prometheus(_) | Query::Raw(_) => None,
            };

            GrafanaTarget {
                ref_id,
                datasource: options.datasource.as_ref().or(panel_datasource).cloned(),
                expr: query.expression().to_owned(),
                editor_mode: options.editor_mode.map(crate::QueryEditorMode::as_grafana),
                legend_format: options.legend.clone(),
                instant: options.instant,
                range: options.range,
                hide: options.hidden,
                interval: options.interval.clone(),
                query_type,
                extra: query.raw_extra().cloned().unwrap_or_default(),
            }
        })
        .collect()
}
