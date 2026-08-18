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

use crate::{
    JoinMode, LabelsToFieldsMode, OrganizeFields, Transformation,
    transformation::TransformationFilter,
};

/// Every rename in `organize` whose *new* name is `target`, yielded as the
/// original pre-rename field names.
///
/// Grafana's rename step keys on the field's pre-rename display name and
/// writes the new one
/// (`packages/grafana-data/src/transformations/transformers/rename.ts`
/// L49-52), so a rename entry reads `source -> target`.
pub(crate) fn rename_sources<'a>(
    organize: &'a OrganizeFields,
    target: &str,
) -> impl Iterator<Item = &'a str> {
    let target = target.to_owned();
    organize
        .renames
        .iter()
        .filter(move |(_, to)| **to == target)
        .map(|(from, _)| from.as_str())
}

/// Translates one authored `order` entry — a FINAL, post-rename display
/// name — into the `indexByName` key Grafana will actually match.
///
/// Grafana's `organize` transformer pipes filter, then order, then rename
/// (`packages/grafana-data/src/transformations/transformers/organize.ts`
/// L35-46). `indexByName` is consumed by the *order* step, which compares
/// each field's display name as it stands at that moment (`order.ts` L73-80,
/// `createFieldsOrdererManual`), and renaming has not happened yet. So an
/// `indexByName` keyed on the names an author sees in the finished table
/// resolves only for columns that were never renamed; every renamed column
/// silently falls through to `Number.MAX_SAFE_INTEGER` (`order.ts` L82-87)
/// and the ordering block becomes a no-op.
///
/// `excludeByName` is deliberately NOT translated the same way: the filter
/// step runs first, so its keys are legitimately pre-rename names.
///
/// A name that no rename targets is already the name the order step sees and
/// passes through unchanged. A name that two or more renames target is
/// ambiguous; validation rejects that before lowering ever runs, and this
/// falls back to the untranslated name so the lowering stays total.
fn index_by_name_key<'a>(organize: &'a OrganizeFields, ordered: &'a str) -> &'a str {
    let mut sources = rename_sources(organize, ordered);
    match (sources.next(), sources.next()) {
        (Some(source), None) => source,
        _ => ordered,
    }
}

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
                    .map(|(index, name)| {
                        (index_by_name_key(organize, name).to_owned(), json!(index))
                    })
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
//
// There is no `byIndex` frame matcher. `DataTransformerID` declares the
// constant, but `frameMatchers` is built from exactly
// `getFramePredicateMatchers()`, `getFrameNameMatchers()` (`byName`) and
// `getRefIdMatchers()` (`byRefId`) in
// `packages/grafana-data/src/transformations/matchers.ts`, so nothing
// registers it and `Registry.get` throws on lookup.
fn transformation_filter(filter: &TransformationFilter) -> Value {
    match filter {
        TransformationFilter::RefId(ref_id) => json!({"id": "byRefId", "options": ref_id}),
        TransformationFilter::FrameName(name) => {
            json!({"id": "byName", "options": escape_frame_name(name)})
        }
    }
}

// Grafana's frame `byName` matcher runs its option through
// `stringToJsRegex`, which anchors a plain string as `^…$` and treats a
// leading `/` as a `/pattern/flags` literal. Panelist's API promises an
// exact name, so every metacharacter is escaped here — including `/`,
// which keeps a name like `/api/v1` from being parsed as a regex literal
// and throwing.
fn escape_frame_name(name: &str) -> String {
    const METACHARACTERS: &[char] = &[
        '\\', '^', '$', '.', '|', '?', '*', '+', '(', ')', '[', ']', '{', '}', '/',
    ];
    let mut escaped = String::with_capacity(name.len());
    for character in name.chars() {
        if METACHARACTERS.contains(&character) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}
