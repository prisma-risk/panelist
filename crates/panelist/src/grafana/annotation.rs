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

use serde_json::{Map, Value, json};

use crate::{Annotation, BuiltInAnnotation, DataSource, annotation::AnnotationCommon};

use super::vocabulary::color;

/// Lowers the dashboard's annotation list, built-in entry first.
///
/// The keys follow `#AnnotationQuery` in `kinds/dashboard/dashboard_kind.cue` at Grafana v13.0.2. Prometheus queries use the shape the Prometheus datasource's `prepareAnnotation` (`packages/grafana-prometheus/src/annotations.ts`) migrates every annotation to on load: the expression and step live in `target`, with `refId: "Anno"` and the step as `target.interval`, while the title, text, and tag templates stay top-level. Emitting the migrated shape means Grafana has nothing to rewrite.
pub(crate) fn normalize_annotations(
    built_in: Option<&BuiltInAnnotation>,
    annotations: &[Annotation],
    default_datasource: Option<&DataSource>,
) -> Vec<Value> {
    let mut list = Vec::with_capacity(annotations.len() + 1);
    if let Some(built_in) = built_in {
        list.push(built_in_value(built_in));
    }
    list.extend(
        annotations
            .iter()
            .map(|annotation| annotation_value(annotation, default_datasource)),
    );
    list
}

/// The entry `addBuiltInAnnotationQuery` in `public/app/features/dashboard/state/DashboardModel.ts` inserts, with the author's settings applied.
fn built_in_value(built_in: &BuiltInAnnotation) -> Value {
    json!({
        "builtIn": 1,
        "datasource": {"type": "grafana", "uid": "-- Grafana --"},
        "enable": built_in.enable,
        "hide": built_in.hide,
        "iconColor": built_in.icon_color.as_ref().map_or("rgba(0, 211, 255, 1)", color),
        "name": "Annotations & Alerts",
        "type": "dashboard",
    })
}

fn annotation_value(annotation: &Annotation, default_datasource: Option<&DataSource>) -> Value {
    let (common, mut object) = match annotation {
        Annotation::Prometheus(prometheus) => {
            let mut object = Map::new();
            let mut target = Map::new();
            target.insert("expr".to_owned(), json!(prometheus.expr));
            if let Some(step) = &prometheus.step {
                target.insert("interval".to_owned(), json!(step));
            }
            target.insert("refId".to_owned(), json!("Anno"));
            object.insert("target".to_owned(), Value::Object(target));
            if let Some(format) = &prometheus.title_format {
                object.insert("titleFormat".to_owned(), json!(format));
            }
            if let Some(format) = &prometheus.text_format {
                object.insert("textFormat".to_owned(), json!(format));
            }
            if !prometheus.tag_keys.is_empty() {
                // Grafana splits this on commas, with no trimming.
                object.insert("tagKeys".to_owned(), json!(prometheus.tag_keys.join(",")));
            }
            if prometheus.use_value_for_time {
                object.insert("useValueForTime".to_owned(), json!(true));
            }
            (&prometheus.common, object)
        }
        Annotation::Raw(raw) => (&raw.common, Map::new()),
    };

    let mut output = common_value(common, default_datasource);
    output.append(&mut object);
    if let Annotation::Raw(raw) = annotation {
        output.extend(raw.fields.clone());
    }
    Value::Object(output)
}

fn common_value(
    common: &AnnotationCommon,
    default_datasource: Option<&DataSource>,
) -> Map<String, Value> {
    let mut output = Map::new();
    output.insert("name".to_owned(), json!(common.name));
    if let Some(datasource) = common.datasource.as_ref().or(default_datasource) {
        output.insert(
            "datasource".to_owned(),
            serde_json::to_value(datasource).unwrap_or(Value::Null),
        );
    }
    output.insert("enable".to_owned(), json!(common.enable));
    output.insert("hide".to_owned(), json!(common.hide));
    output.insert("iconColor".to_owned(), json!(color(&common.icon_color)));
    output
}
