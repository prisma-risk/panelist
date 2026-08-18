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

use serde_json::{Value, json};

use crate::{DataSource, Variable};

pub(crate) fn normalize_variable(
    variable: &Variable,
    default_datasource: Option<&DataSource>,
) -> BTreeMap<String, Value> {
    match variable {
        Variable::DataSource(variable) => {
            let mut output = variable_base(
                &variable.name,
                "datasource",
                variable.label.as_deref(),
                variable.hidden,
                variable.skip_url_sync,
            );
            output.insert("query".to_owned(), json!(variable.plugin_id));
            output.insert("regex".to_owned(), json!(variable.regex));
            output.insert("refresh".to_owned(), json!(variable.refresh.as_grafana()));
            output.insert("multi".to_owned(), json!(false));
            output.insert("includeAll".to_owned(), json!(false));
            output.insert("allowCustomValue".to_owned(), json!(false));
            output.insert("options".to_owned(), json!([]));
            insert_current(&mut output, variable.current.as_ref());
            output
        }
        Variable::Query(variable) => {
            let mut output = variable_base(
                &variable.name,
                "query",
                variable.label.as_deref(),
                variable.hidden,
                variable.skip_url_sync,
            );
            let expression = variable.query.expression();
            let ref_id = variable
                .query
                .options()
                .ref_id
                .as_deref()
                .unwrap_or("StandardVariableQuery");
            output.insert(
                "query".to_owned(),
                json!({"query": expression, "refId": ref_id}),
            );
            output.insert("definition".to_owned(), json!(expression));
            if let Some(datasource) = variable
                .datasource
                .as_ref()
                .or(variable.query.options().datasource.as_ref())
                .or(default_datasource)
            {
                output.insert("datasource".to_owned(), datasource_value(datasource));
            }
            output.insert("refresh".to_owned(), json!(variable.refresh.as_grafana()));
            output.insert("sort".to_owned(), json!(variable.sort.as_grafana()));
            output.insert("multi".to_owned(), json!(variable.multi));
            output.insert("includeAll".to_owned(), json!(variable.include_all));
            output.insert("allValue".to_owned(), json!(variable.all_value));
            output.insert(
                "allowCustomValue".to_owned(),
                json!(variable.allow_custom_value),
            );
            output.insert("regex".to_owned(), json!(variable.regex));
            output.insert("options".to_owned(), json!([]));
            insert_current(&mut output, variable.current.as_ref());
            output
        }
        Variable::Custom(variable) => {
            let fallback;
            let current = if let Some(current) = variable.current.as_ref() {
                Some(current)
            } else {
                fallback = variable
                    .values
                    .first()
                    .map(|value| crate::VariableSelection::new(value.clone(), value.clone()));
                fallback.as_ref()
            };
            let options = variable
                .values
                .iter()
                .map(|value| {
                    json!({
                        "selected": current.is_some_and(|current| current.value == *value),
                        "text": value,
                        "value": value,
                    })
                })
                .collect::<Vec<_>>();
            let mut output = variable_base(
                &variable.name,
                "custom",
                variable.label.as_deref(),
                variable.hidden,
                variable.skip_url_sync,
            );
            output.insert("query".to_owned(), json!(variable.values.join(",")));
            output.insert("options".to_owned(), json!(options));
            output.insert("multi".to_owned(), json!(variable.multi));
            output.insert("includeAll".to_owned(), json!(variable.include_all));
            output.insert("allValue".to_owned(), json!(variable.all_value));
            output.insert(
                "allowCustomValue".to_owned(),
                json!(variable.allow_custom_value),
            );
            insert_current(&mut output, current);
            output
        }
        Variable::Constant(variable) => {
            let mut output = variable_base(
                &variable.name,
                "constant",
                variable.label.as_deref(),
                variable.hidden,
                false,
            );
            output.insert("query".to_owned(), json!(variable.value));
            output.insert(
                "current".to_owned(),
                json!({"selected": true, "text": variable.value, "value": variable.value}),
            );
            output
        }
    }
}

pub(crate) fn variable_base(
    name: &str,
    kind: &str,
    label: Option<&str>,
    hidden: bool,
    skip_url_sync: bool,
) -> BTreeMap<String, Value> {
    let mut output = BTreeMap::new();
    output.insert("name".to_owned(), json!(name));
    output.insert("type".to_owned(), json!(kind));
    if let Some(label) = label {
        output.insert("label".to_owned(), json!(label));
    }
    output.insert("hide".to_owned(), json!(u8::from(hidden)));
    output.insert("skipUrlSync".to_owned(), json!(skip_url_sync));
    output
}

pub(crate) fn insert_current(
    output: &mut BTreeMap<String, Value>,
    current: Option<&crate::VariableSelection>,
) {
    if let Some(current) = current {
        output.insert(
            "current".to_owned(),
            json!({
                "selected": current.selected,
                "text": current.text,
                "value": current.value,
            }),
        );
    }
}

pub(crate) fn datasource_value(datasource: &DataSource) -> Value {
    json!({"type": datasource.kind(), "uid": datasource.uid()})
}
