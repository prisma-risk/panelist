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

//! Transformation lowering.
//!
//! Option shapes follow Grafana's transformer definitions in
//! `packages/grafana-data/src/transformations/transformers/`. `timeSeriesTable`
//! options are keyed by query reference ID rather than taking a field list.

use serde_json::{Value, json};

use crate::{JoinMode, LabelsToFieldsMode, Transformation, TransformationFilter};

pub(crate) fn normalize_transformation(transformation: &Transformation) -> Value {
    let (id, options) = match transformation {
        Transformation::JoinByField(join) => (
            "joinByField",
            json!({"byField": join.field, "mode": join_mode(join.mode)}),
        ),
        Transformation::OrganizeFields(organize) => {
            let mut options = serde_json::Map::new();
            options.insert(
                "excludeByName".to_owned(),
                organize
                    .hidden
                    .iter()
                    .map(|name| (name.clone(), Value::Bool(true)))
                    .collect::<serde_json::Map<_, _>>()
                    .into(),
            );
            options.insert(
                "indexByName".to_owned(),
                organize
                    .order
                    .iter()
                    .enumerate()
                    .map(|(index, name)| (name.clone(), json!(index)))
                    .collect::<serde_json::Map<_, _>>()
                    .into(),
            );
            options.insert(
                "renameByName".to_owned(),
                organize
                    .renames
                    .iter()
                    .map(|(from, to)| (from.clone(), json!(to)))
                    .collect::<serde_json::Map<_, _>>()
                    .into(),
            );
            ("organize", Value::Object(options))
        }
        Transformation::SortBy(sort) => (
            "sortBy",
            json!({
                "sort": sort
                    .fields
                    .iter()
                    .map(|field| json!({"field": field.field, "desc": field.descending}))
                    .collect::<Vec<_>>(),
            }),
        ),
        Transformation::TimeSeriesToTable(convert) => (
            "timeSeriesTable",
            convert
                .queries
                .iter()
                .map(|(ref_id, query)| {
                    let mut entry = serde_json::Map::new();
                    if let Some(stat) = query.stat {
                        entry.insert("stat".to_owned(), json!(super::vocabulary::reducer(stat)));
                    }
                    if let Some(time_field) = &query.time_field {
                        entry.insert("timeField".to_owned(), json!(time_field));
                    }
                    (ref_id.clone(), Value::Object(entry))
                })
                .collect::<serde_json::Map<_, _>>()
                .into(),
        ),
        Transformation::LabelsToFields(labels) => {
            let mut options = serde_json::Map::new();
            options.insert("mode".to_owned(), json!(labels_to_fields_mode(labels.mode)));
            if !labels.keep.is_empty() {
                options.insert("keepLabels".to_owned(), json!(labels.keep));
            }
            if let Some(value_label) = &labels.value_label {
                options.insert("valueLabel".to_owned(), json!(value_label));
            }
            ("labelsToFields", Value::Object(options))
        }
        Transformation::Raw(raw) => (
            raw.id.as_str(),
            Value::Object(raw.options.clone().into_iter().collect()),
        ),
    };

    let mut output = serde_json::Map::new();
    output.insert("id".to_owned(), json!(id));
    output.insert("options".to_owned(), options);

    let envelope = transformation.envelope();
    if let Some(filter) = &envelope.filter {
        output.insert("filter".to_owned(), transformation_filter(filter));
    }
    if envelope.disabled {
        output.insert("disabled".to_owned(), json!(true));
    }

    Value::Object(output)
}

const fn join_mode(mode: JoinMode) -> &'static str {
    match mode {
        JoinMode::Outer => "outer",
        JoinMode::Inner => "inner",
        JoinMode::OuterTabular => "outerTabular",
    }
}

const fn labels_to_fields_mode(mode: LabelsToFieldsMode) -> &'static str {
    match mode {
        LabelsToFieldsMode::Columns => "columns",
        LabelsToFieldsMode::Rows => "rows",
    }
}

// Frame matchers use `byRefId`, distinct from the `byFrameRefID` field
// matcher that field overrides use for the same concept.
fn transformation_filter(filter: &TransformationFilter) -> Value {
    match filter {
        TransformationFilter::RefId(ref_id) => json!({"id": "byRefId", "options": ref_id}),
        TransformationFilter::FrameName(name) => json!({"id": "byName", "options": name}),
        TransformationFilter::FrameIndex(index) => json!({"id": "byIndex", "options": index}),
    }
}
