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

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{
    Dashboard, DashboardItem, OrganizeFields, OverrideMatcher, Panel, Query, Thresholds,
    Transformation, ValidationError, ValidationErrors, Variable, VariableReferenceSite,
    transformation::TransformationFilter, variable::VariableDiagnostic,
};

use super::query::effective_ref_ids;
use super::references::{is_grafana_global, is_plugin_macro, variable_references};
use super::transform::rename_sources;

pub(crate) fn validate(dashboard: &Dashboard) -> Result<(), ValidationErrors> {
    let mut errors = Vec::new();
    if dashboard.title.trim().is_empty() {
        errors.push(ValidationError::MissingDashboardTitle);
    }
    if let Some(uid) = &dashboard.uid
        && let Some(reason) = uid_problem(uid)
    {
        errors.push(ValidationError::InvalidDashboardUid {
            uid: uid.clone(),
            reason,
        });
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
        // Problems the builder recorded because Grafana's model cannot
        // express them. Reported rather than dropped: a silently ignored
        // `sort` is indistinguishable from a working one in the emitted JSON.
        let (diagnostics, kind) = match variable {
            Variable::Query(variable) => (variable.diagnostics.as_slice(), "query"),
            Variable::DataSource(variable) => (variable.diagnostics.as_slice(), "datasource"),
            Variable::Custom(variable) => (variable.diagnostics.as_slice(), "custom"),
            Variable::Constant(variable) => (variable.diagnostics.as_slice(), "constant"),
        };
        for diagnostic in diagnostics {
            errors.push(match diagnostic {
                VariableDiagnostic::OptionNotApplicable(option) => {
                    ValidationError::VariableOptionNotApplicable {
                        variable: variable.name().to_owned(),
                        option,
                        kind,
                    }
                }
                VariableDiagnostic::NoSelector => ValidationError::MissingVariableSelector {
                    variable: variable.name().to_owned(),
                },
                VariableDiagnostic::ConflictingSelectors { chosen, ignored } => {
                    ValidationError::ConflictingVariableSelectors {
                        variable: variable.name().to_owned(),
                        chosen,
                        ignored,
                    }
                }
            });
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
    validate_variable_references(dashboard, &mut errors);
    super::validate_options::validate_options(dashboard, &mut errors);
    ValidationErrors::from_vec(errors)
}

/// Checks that every template-variable reference in a string Grafana
/// interpolates resolves to a declared variable or a Grafana built-in.
///
/// Only fields that carry references by design are scanned: query
/// expressions, legend formats, datasource UIDs, link URLs, and query
/// variable definitions. Variable `regex` options are left alone. Grafana
/// does interpolate them, but a `$` in a regex is almost always an anchor.
fn validate_variable_references(dashboard: &Dashboard, errors: &mut Vec<ValidationError>) {
    // The first declaration of a name is the one Grafana resolves.
    let mut declared: HashMap<&str, usize> = HashMap::new();
    for (position, variable) in dashboard.variables.iter().enumerate() {
        declared.entry(variable.name()).or_insert(position);
    }

    let mut references = References::new(&declared, VariableReferenceSite::Dashboard, None);
    if let Some(datasource) = &dashboard.datasource {
        references.check("datasource", datasource.uid(), Leeway::None, errors);
    }
    for link in &dashboard.links {
        references.check("link URL", &link.url, Leeway::None, errors);
    }

    for (position, variable) in dashboard.variables.iter().enumerate() {
        let Variable::Query(variable) = variable else {
            continue;
        };
        let mut references = References::new(
            &declared,
            VariableReferenceSite::Variable(variable.name.clone()),
            Some(position),
        );
        check_query(&mut references, &variable.query, errors);
        if let Some(datasource) = &variable.datasource {
            references.check("datasource", datasource.uid(), Leeway::None, errors);
        } else if variable.query.options().datasource.is_none()
            && let Some(datasource) = &dashboard.datasource
        {
            references.check(
                "inherited datasource",
                datasource.uid(),
                Leeway::OrderOnly,
                errors,
            );
        }
    }

    let panels = dashboard.items.iter().flat_map(|item| match item {
        DashboardItem::Row(row) => row.panels.iter().collect::<Vec<_>>(),
        DashboardItem::Panel(panel) => vec![panel.as_ref()],
    });
    for panel in panels {
        let mut references = References::new(
            &declared,
            VariableReferenceSite::Panel(panel.title.clone()),
            None,
        );
        if let Some(datasource) = &panel.datasource {
            references.check("datasource", datasource.uid(), Leeway::None, errors);
        }
        for query in &panel.queries {
            check_query(&mut references, query, errors);
        }
        for link in &panel.links {
            references.check("link URL", &link.url, Leeway::None, errors);
        }
    }
}

fn check_query(references: &mut References<'_>, query: &Query, errors: &mut Vec<ValidationError>) {
    let leeway = match query {
        Query::Raw(_) => Leeway::PluginMacros,
        Query::Prometheus(_) | Query::Loki(_) => Leeway::None,
    };
    references.check("query expression", query.expression(), leeway, errors);
    let options = query.options();
    if let Some(legend_format) = &options.legend_format {
        references.check("legend format", legend_format, Leeway::None, errors);
    }
    if let Some(datasource) = &options.datasource {
        references.check("query datasource", datasource.uid(), Leeway::None, errors);
    }
}

/// Which unresolved names a field tolerates beyond Grafana's built-ins.
#[derive(Clone, Copy)]
enum Leeway {
    /// Every name must be declared or built in.
    None,
    /// A raw query's unknown datasource plugin may define `__` macros.
    PluginMacros,
    /// Only ordering is checked. Used for the dashboard datasource a query
    /// variable inherits, whose unknown names the dashboard already reports.
    OrderOnly,
}

/// Resolves the references found at one site and reports each unresolved
/// name once per field.
struct References<'a> {
    declared: &'a HashMap<&'a str, usize>,
    site: VariableReferenceSite,
    /// A query variable's own position. Only names declared before it
    /// resolve, because Grafana resolves variables in order.
    position: Option<usize>,
    reported: HashSet<(&'static str, String)>,
}

impl<'a> References<'a> {
    fn new(
        declared: &'a HashMap<&'a str, usize>,
        site: VariableReferenceSite,
        position: Option<usize>,
    ) -> Self {
        Self {
            declared,
            site,
            position,
            reported: HashSet::new(),
        }
    }

    fn check(
        &mut self,
        field: &'static str,
        text: &str,
        leeway: Leeway,
        errors: &mut Vec<ValidationError>,
    ) {
        for name in variable_references(text) {
            if is_grafana_global(name)
                || (matches!(leeway, Leeway::PluginMacros) && is_plugin_macro(name))
            {
                continue;
            }
            let error = match (self.declared.get(name), self.position, &self.site) {
                (Some(_), None, _) => continue,
                (Some(declared), Some(position), _) if *declared < position => continue,
                (Some(_), Some(_), VariableReferenceSite::Variable(variable)) => {
                    ValidationError::VariableReferencedBeforeDeclaration {
                        variable: variable.clone(),
                        field,
                        name: name.to_owned(),
                    }
                }
                (None, _, _) if !matches!(leeway, Leeway::OrderOnly) => {
                    ValidationError::UnknownVariableReference {
                        site: self.site.clone(),
                        field,
                        name: name.to_owned(),
                    }
                }
                _ => continue,
            };
            if self.reported.insert((field, name.to_owned())) {
                errors.push(error);
            }
        }
    }
}

/// Mirrors Grafana's `IsValidShortUID` and `MaxUIDLength`, which reject a dashboard save outright.
fn uid_problem(uid: &str) -> Option<&'static str> {
    if uid.is_empty() {
        Some("must not be empty")
    } else if uid.len() > 40 {
        Some("is longer than Grafana's 40-character limit")
    } else if !uid
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        Some("may only contain ASCII letters, digits, '-', and '_'")
    } else {
        None
    }
}

pub(crate) fn validate_panel(
    panel: &Panel,
    ids: &mut HashSet<u32>,
    errors: &mut Vec<ValidationError>,
) {
    if panel.title.trim().is_empty() {
        errors.push(ValidationError::MissingPanelTitle);
    }
    errors.extend(panel.diagnostics.iter().cloned());
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
        Transformation::OrganizeFields(organize) => {
            validate_organize_order(panel, organize, errors);
        }
        Transformation::JoinByField(_)
        | Transformation::SortBy(_)
        | Transformation::LabelsToFields(_)
        | Transformation::Raw(_) => {}
    }

    // One authoring mistake, one error. A transformation that both filters
    // on a reference ID and names the same one in its options — e.g.
    // `TimeSeriesToTable::new().query("Z").only_ref_id("Z")` — used to push
    // two identical errors for it.
    let mut seen = HashSet::new();
    referenced.retain(|ref_id| seen.insert(*ref_id));

    for ref_id in referenced {
        if !ref_ids.contains(ref_id) {
            errors.push(ValidationError::UnknownTransformationRefId {
                panel: panel.title.clone(),
                ref_id: ref_id.to_owned(),
            });
        }
    }
}

/// Rejects the two ways `OrganizeFields::order` can fail to lower.
///
/// `order` lists the FINAL, post-rename names an author sees, and
/// `super::transform::index_by_name_key` translates each back to the
/// pre-rename name Grafana's ordering step actually matches. Both failures
/// below would otherwise mis-order columns in silence: Grafana accepts,
/// stores, and returns an unresolvable `indexByName` unchanged, then ignores
/// it at render time.
fn validate_organize_order(
    panel: &Panel,
    organize: &OrganizeFields,
    errors: &mut Vec<ValidationError>,
) {
    let mut claimed: BTreeMap<&str, &str> = BTreeMap::new();

    for ordered in &organize.order {
        let sources: Vec<&str> = rename_sources(organize, ordered).collect();
        if sources.len() > 1 {
            errors.push(ValidationError::AmbiguousOrganizeOrder {
                panel: panel.title.clone(),
                field: ordered.clone(),
                sources: sources.into_iter().map(ToOwned::to_owned).collect(),
            });
            continue;
        }

        let key = sources.first().copied().unwrap_or(ordered.as_str());
        if let Some(first) = claimed.insert(key, ordered.as_str()) {
            errors.push(ValidationError::ConflictingOrganizeOrder {
                panel: panel.title.clone(),
                first: first.to_owned(),
                second: ordered.clone(),
                field: key.to_owned(),
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
