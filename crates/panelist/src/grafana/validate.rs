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

use crate::{
    Dashboard, DashboardItem, OverrideMatcher, Panel, Thresholds, Transformation, ValidationError,
    ValidationErrors, Variable, transformation::TransformationFilter,
};

use super::query::effective_ref_ids;

pub(crate) fn validate(dashboard: &Dashboard) -> Result<(), ValidationErrors> {
    let mut errors = Vec::new();
    if dashboard.title.trim().is_empty() {
        errors.push(ValidationError::MissingDashboardTitle);
    }
    for variable in &dashboard.variables {
        if variable.name().trim().is_empty() {
            errors.push(ValidationError::MissingVariableName);
        }
        match variable {
            Variable::Query(variable) if variable.query.expression().trim().is_empty() => {
                errors.push(ValidationError::MissingQueryExpression {
                    panel: format!("variable {}", variable.name),
                });
            }
            Variable::DataSource(_)
            | Variable::Query(_)
            | Variable::Custom(_)
            | Variable::Constant(_) => {}
        }
    }

    let mut ids = HashSet::new();
    for item in &dashboard.items {
        match item {
            DashboardItem::Row(row) => {
                if row.title.trim().is_empty() {
                    errors.push(ValidationError::MissingRowTitle);
                }
                for panel in &row.panels {
                    validate_panel(panel, &mut ids, &mut errors);
                }
            }
            DashboardItem::Panel(panel) => validate_panel(panel, &mut ids, &mut errors),
        }
    }
    ValidationErrors::from_vec(errors)
}

pub(crate) fn validate_panel(
    panel: &Panel,
    ids: &mut HashSet<u32>,
    errors: &mut Vec<ValidationError>,
) {
    if panel.title.trim().is_empty() {
        errors.push(ValidationError::MissingPanelTitle);
    }
    if !(1..=24).contains(&panel.width) {
        errors.push(ValidationError::InvalidPanelWidth {
            panel: panel.title.clone(),
            width: panel.width,
        });
    }
    if panel.height == 0 {
        errors.push(ValidationError::InvalidPanelHeight {
            panel: panel.title.clone(),
            height: panel.height,
        });
    }
    if let Some(grid) = panel.grid_pos
        && (grid.width == 0
            || grid.width > 24
            || grid.height == 0
            || grid.x >= 24
            || grid.x.saturating_add(grid.width) > 24)
    {
        errors.push(ValidationError::InvalidGridPosition {
            panel: panel.title.clone(),
            x: grid.x,
            y: grid.y,
            width: grid.width,
            height: grid.height,
        });
    }
    if let Some(id) = panel.id
        && !ids.insert(id)
    {
        errors.push(ValidationError::DuplicatePanelId {
            id,
            panel: panel.title.clone(),
        });
    }

    let mut refs = HashSet::new();
    for query in &panel.queries {
        if query.expression().trim().is_empty() {
            errors.push(ValidationError::MissingQueryExpression {
                panel: panel.title.clone(),
            });
        }
        if let Some(ref_id) = &query.options().ref_id
            && !refs.insert(ref_id.clone())
        {
            errors.push(ValidationError::DuplicateQueryRefId {
                panel: panel.title.clone(),
                ref_id: ref_id.clone(),
            });
        }
    }
    if let Some(thresholds) = &panel.field_config.thresholds
        && !thresholds_are_valid(thresholds)
    {
        errors.push(ValidationError::InvalidThresholds {
            panel: panel.title.clone(),
        });
    }

    let ref_ids: HashSet<String> = effective_ref_ids(&panel.queries).into_iter().collect();
    for transformation in &panel.transformations {
        validate_transformation(panel, transformation, &ref_ids, errors);
    }
    for field_override in &panel.field_config.overrides {
        if let OverrideMatcher::QueryRefId(ref_id) = &field_override.matcher
            && !ref_ids.contains(ref_id)
        {
            errors.push(ValidationError::UnknownOverrideRefId {
                panel: panel.title.clone(),
                ref_id: ref_id.clone(),
            });
        }
    }
}

fn validate_transformation(
    panel: &Panel,
    transformation: &Transformation,
    ref_ids: &HashSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    let mut referenced: Vec<&str> = Vec::new();
    if let Some(TransformationFilter::RefId(ref_id)) = &transformation.envelope().filter {
        referenced.push(ref_id);
    }

    match transformation {
        Transformation::JoinByField(join) if join.field.trim().is_empty() => {
            errors.push(ValidationError::MissingTransformationField {
                panel: panel.title.clone(),
                transformation: "join by field",
            });
        }
        Transformation::SortBy(sort)
            if sort
                .fields
                .iter()
                .any(|field| field.field.trim().is_empty()) =>
        {
            errors.push(ValidationError::MissingTransformationField {
                panel: panel.title.clone(),
                transformation: "sort by",
            });
        }
        Transformation::Raw(raw) if raw.id.trim().is_empty() => {
            errors.push(ValidationError::MissingTransformationId {
                panel: panel.title.clone(),
            });
        }
        Transformation::TimeSeriesToTable(convert) => {
            referenced.extend(convert.queries.keys().map(String::as_str));
        }
        Transformation::JoinByField(_)
        | Transformation::SortBy(_)
        | Transformation::OrganizeFields(_)
        | Transformation::LabelsToFields(_)
        | Transformation::Raw(_) => {}
    }

    for ref_id in referenced {
        if !ref_ids.contains(ref_id) {
            errors.push(ValidationError::UnknownTransformationRefId {
                panel: panel.title.clone(),
                ref_id: ref_id.to_owned(),
            });
        }
    }
}

pub(crate) fn thresholds_are_valid(thresholds: &Thresholds) -> bool {
    let mut previous = None;
    for (index, step) in thresholds.steps.iter().enumerate() {
        match step.value {
            None if index == 0 => {}
            None => return false,
            Some(value) if !value.is_finite() => return false,
            Some(value) => {
                if previous.is_some_and(|previous| value <= previous) {
                    return false;
                }
                previous = Some(value);
            }
        }
    }
    true
}
