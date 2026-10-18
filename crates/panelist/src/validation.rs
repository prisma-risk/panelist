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

use std::{fmt, path::PathBuf};

use thiserror::Error;

/// A crate-wide result.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced while validating, serializing, or writing a dashboard.
#[derive(Debug, Error)]
pub enum Error {
    /// The authored dashboard contains one or more invalid values.
    #[error(transparent)]
    Validation(#[from] ValidationErrors),
    /// Grafana JSON could not be encoded.
    #[error("could not serialize dashboard JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// A generated dashboard could not be written to disk.
    #[error("could not write dashboard to {path}: {source}")]
    Io {
        /// Destination path.
        path: PathBuf,
        /// Underlying filesystem error.
        source: std::io::Error,
    },
}

/// A collection of authoring errors found in one validation pass.
#[derive(Debug, Clone, PartialEq, Error)]
#[error("dashboard validation failed with {} error(s): {}", .0.len(), format_validation_errors(.0))]
pub struct ValidationErrors(pub Vec<ValidationError>);

impl ValidationErrors {
    pub(crate) fn from_vec(errors: Vec<ValidationError>) -> std::result::Result<(), Self> {
        if errors.is_empty() {
            Ok(())
        } else {
            Err(Self(errors))
        }
    }

    /// Returns all individual validation errors.
    #[must_use]
    pub fn errors(&self) -> &[ValidationError] {
        &self.0
    }
}

fn format_validation_errors(errors: &[ValidationError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// A specific mistake in an authored dashboard.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum ValidationError {
    /// The dashboard title is empty.
    #[error("dashboard title must not be empty")]
    MissingDashboardTitle,
    /// A panel title is empty.
    #[error("panel title must not be empty")]
    MissingPanelTitle,
    /// A row title is empty.
    #[error("row title must not be empty")]
    MissingRowTitle,
    /// A variable name is empty.
    #[error("variable name must not be empty")]
    MissingVariableName,
    /// A panel width falls outside Grafana's 24-column grid.
    #[error("panel \"{panel}\": width must be between 1 and 24, got {width}")]
    InvalidPanelWidth {
        /// Panel title.
        panel: String,
        /// Invalid width.
        width: u16,
    },
    /// A panel height is zero.
    #[error("panel \"{panel}\": height must be at least 1, got {height}")]
    InvalidPanelHeight {
        /// Panel title.
        panel: String,
        /// Invalid height.
        height: u16,
    },
    /// An explicit grid position falls outside the 24-column grid.
    #[error("panel \"{panel}\": invalid grid position x={x}, y={y}, w={width}, h={height}")]
    InvalidGridPosition {
        /// Panel title.
        panel: String,
        /// X coordinate.
        x: u16,
        /// Y coordinate.
        y: u16,
        /// Width.
        width: u16,
        /// Height.
        height: u16,
    },
    /// Two authored panels use the same explicit ID.
    #[error("duplicate explicit panel ID {id} on panel \"{panel}\"")]
    DuplicatePanelId {
        /// Duplicate ID.
        id: u32,
        /// Later panel title.
        panel: String,
    },
    /// Two panels or rows end up with the same ID under [`PanelIds::Stable`](crate::PanelIds::Stable).
    ///
    /// Either both derived the same ID, or a derived ID equals an explicit one. Give one of them a distinct key, or a different explicit ID.
    #[error("panel ID {id} is shared by {first} and {second}; give one of them a distinct key")]
    PanelIdCollision {
        /// The shared ID.
        id: u32,
        /// The panel or row that claimed the ID first, in dashboard order.
        first: String,
        /// The panel or row that collided with it.
        second: String,
    },
    /// A panel key is empty or only whitespace.
    #[error("panel \"{panel}\": key must not be blank")]
    BlankPanelKey {
        /// Panel title.
        panel: String,
    },
    /// A row key is empty or only whitespace.
    #[error("row \"{row}\": key must not be blank")]
    BlankRowKey {
        /// Row title.
        row: String,
    },
    /// The dashboard UID is one Grafana refuses to save.
    ///
    /// Grafana accepts at most 40 characters, each an ASCII letter, digit, `-`, or `_`.
    #[error("dashboard UID \"{uid}\" {reason}")]
    InvalidDashboardUid {
        /// The rejected UID.
        uid: String,
        /// Why Grafana rejects it.
        reason: &'static str,
    },
    /// Two queries in a panel use the same explicit reference ID.
    #[error("panel \"{panel}\": duplicate query reference ID \"{ref_id}\"")]
    DuplicateQueryRefId {
        /// Panel title.
        panel: String,
        /// Duplicate query reference.
        ref_id: String,
    },
    /// A query expression is empty, or a raw query sets neither an expression nor any extra key.
    #[error("panel \"{panel}\": query expression must not be empty")]
    MissingQueryExpression {
        /// Panel title.
        panel: String,
    },
    /// A Loki query asks for [`crate::QueryType::Both`]. Loki has no combined range-and-instant mode, so the query would silently run as instant only.
    #[error(
        "panel \"{panel}\": Loki query \"{ref_id}\" asks for both range and instant results, which Loki cannot run as one query; split it into two queries"
    )]
    UnsupportedQueryType {
        /// Panel title.
        panel: String,
        /// Resolved reference ID of the query.
        ref_id: String,
    },
    /// A panel uses [`crate::DataSource::mixed`], directly or by inheriting it, and one of its queries names no datasource of its own. Grafana would run that query against its default datasource.
    #[error(
        "panel \"{panel}\": query \"{ref_id}\" sets no datasource, but the panel uses the mixed datasource; give each query its own"
    )]
    MissingMixedQueryDataSource {
        /// Panel title.
        panel: String,
        /// Resolved reference ID of the query.
        ref_id: String,
    },
    /// Threshold values are not strictly increasing, in the panel's thresholds or in any override or query thresholds on it.
    #[error("panel \"{panel}\": threshold values must be finite and strictly increasing")]
    InvalidThresholds {
        /// Panel title.
        panel: String,
    },
    /// A transformation references a query reference ID the panel does not have.
    #[error("panel \"{panel}\": transformation references unknown query reference ID \"{ref_id}\"")]
    UnknownTransformationRefId {
        /// Panel title.
        panel: String,
        /// Unresolved query reference.
        ref_id: String,
    },
    /// A field override's `by_query` matcher references a query reference ID
    /// the panel does not have.
    #[error("panel \"{panel}\": field override references unknown query reference ID \"{ref_id}\"")]
    UnknownOverrideRefId {
        /// Panel title.
        panel: String,
        /// Unresolved query reference.
        ref_id: String,
    },
    /// A variable option was set that the resolved variable kind has no
    /// Grafana key for, so it would be dropped without trace.
    #[error(
        "variable \"{variable}\": {option} does not apply to a {kind} variable and would be silently dropped"
    )]
    VariableOptionNotApplicable {
        /// Variable name.
        variable: String,
        /// The option that was set.
        option: &'static str,
        /// The variable kind it resolved to.
        kind: &'static str,
    },
    /// A variable definition set no selector, so its kind is undetermined.
    #[error("variable \"{variable}\": no selector — set one of query, plugin, value, or values")]
    MissingVariableSelector {
        /// Variable name.
        variable: String,
    },
    /// A variable definition set more than one kind selector.
    #[error(
        "variable \"{variable}\": {ignored} is ignored because {chosen} already selected the kind"
    )]
    ConflictingVariableSelectors {
        /// Variable name.
        variable: String,
        /// The selector that decided the kind.
        chosen: &'static str,
        /// The selector that was ignored.
        ignored: &'static str,
    },
    /// A keyed table has an empty key label, so there is nothing to join on.
    #[error("panel \"{panel}\": keyed table needs a key label to join on")]
    MissingTableKey {
        /// Panel title.
        panel: String,
    },
    /// A keyed table column has an empty name.
    #[error("panel \"{panel}\": keyed table column name must not be empty")]
    MissingTableColumnName {
        /// Panel title.
        panel: String,
    },
    /// Two keyed table columns, or a column and the key, share a name.
    ///
    /// A column name is its header, its rename target, and its override matcher at once, so a second column with the same name cannot be told apart from the first.
    #[error("panel \"{panel}\": keyed table column \"{column}\" is defined more than once")]
    DuplicateTableColumn {
        /// Panel title.
        panel: String,
        /// The repeated column name.
        column: String,
    },
    /// A transformation is missing a required field name.
    #[error("panel \"{panel}\": {transformation} transformation requires a field name")]
    MissingTransformationField {
        /// Panel title.
        panel: String,
        /// Transformation name.
        transformation: &'static str,
    },
    /// A raw transformation has no Grafana transformation ID.
    #[error("panel \"{panel}\": raw transformation ID must not be empty")]
    MissingTransformationId {
        /// Panel title.
        panel: String,
    },
    /// An `organize` transformation orders a field name that more than one
    /// rename produces, so the field it refers to is ambiguous.
    ///
    /// `OrganizeFields::order` takes the final, post-rename names. Panelist
    /// translates each back to the pre-rename name Grafana's ordering step
    /// matches on, which is impossible when two renames share a target.
    #[error(
        "panel \"{panel}\": organize transformation orders \"{field}\", but {} are all renamed to it; the ordering key is ambiguous",
        format_quoted(.sources)
    )]
    AmbiguousOrganizeOrder {
        /// Panel title.
        panel: String,
        /// The ordered field name that cannot be resolved.
        field: String,
        /// Every original field name renamed to `field`.
        sources: Vec<String>,
    },
    /// Two entries in an `organize` transformation's ordering resolve to the
    /// same underlying field.
    #[error(
        "panel \"{panel}\": organize transformation ordering resolves \"{first}\" and \"{second}\" to the same field \"{field}\"; one field cannot hold two positions"
    )]
    ConflictingOrganizeOrder {
        /// Panel title.
        panel: String,
        /// The ordered name that claimed the position first.
        first: String,
        /// The ordered name that collided with it.
        second: String,
        /// The underlying field both resolve to.
        field: String,
    },
    /// A string Grafana interpolates references a template variable that is
    /// neither declared on the dashboard nor one of Grafana's built-ins.
    ///
    /// Grafana leaves an unresolved reference in place, so a typo such as
    /// `$enviroment` reaches the datasource verbatim and the panel quietly
    /// shows no data.
    #[error("{site}: {field} references unknown variable \"{name}\"")]
    UnknownVariableReference {
        /// Where the reference was found.
        site: VariableReferenceSite,
        /// The field holding the reference, such as `"query expression"`.
        field: &'static str,
        /// The unresolved variable name, without `$`, braces, or format.
        name: String,
    },
    /// A query variable references a dashboard variable declared at or after
    /// its own position.
    ///
    /// Grafana resolves variables in declaration order, so a chained
    /// variable only sees the values of variables declared before it.
    #[error(
        "variable \"{variable}\": {field} references \"{name}\", which is not declared before it; Grafana resolves variables in declaration order"
    )]
    VariableReferencedBeforeDeclaration {
        /// The referencing variable.
        variable: String,
        /// The field holding the reference, such as `"query expression"`.
        field: &'static str,
        /// The referenced variable, declared later or the variable itself.
        name: String,
    },
    /// A panel or row repeats by a variable the dashboard does not declare, so Grafana would never expand it.
    #[error(
        "{kind} \"{title}\": repeats by variable \"{variable}\", which the dashboard does not declare"
    )]
    UnknownRepeatVariable {
        /// `"panel"` or `"row"`.
        kind: &'static str,
        /// Panel or row title.
        title: String,
        /// The undeclared variable name.
        variable: String,
    },
    /// A panel query option, time override, or repeat setting has a value Grafana cannot use.
    #[error("panel \"{panel}\": {option} \"{value}\" is invalid; expected {expected}")]
    InvalidPanelOption {
        /// Panel title.
        panel: String,
        /// The builder option, such as `time_shift`.
        option: &'static str,
        /// The rejected value.
        value: String,
        /// What Grafana accepts.
        expected: &'static str,
    },
    /// A value mapping can never match, or matches without changing anything.
    #[error("panel \"{panel}\": value mapping {reason}")]
    InvalidValueMapping {
        /// Panel title.
        panel: String,
        /// Why the mapping is unusable.
        reason: &'static str,
    },
    /// Two exact-value mappings in one mapping list match the same value. Grafana keys exact values in one object, so only the last would survive.
    #[error("panel \"{panel}\": more than one value mapping matches the exact value \"{value}\"")]
    DuplicateValueMapping {
        /// Panel title.
        panel: String,
        /// The value mapped more than once.
        value: String,
    },
    /// An annotation name is empty.
    #[error("annotation name must not be empty")]
    MissingAnnotationName,
}

/// Where a template-variable reference was found.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VariableReferenceSite {
    /// The dashboard itself: its default datasource or its links.
    Dashboard,
    /// A panel, named by its title.
    Panel(String),
    /// A dashboard variable, named by its name.
    Variable(String),
}

impl fmt::Display for VariableReferenceSite {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dashboard => formatter.write_str("dashboard"),
            Self::Panel(title) => write!(formatter, "panel \"{title}\""),
            Self::Variable(name) => write!(formatter, "variable \"{name}\""),
        }
    }
}

fn format_quoted(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("\"{value}\""))
        .collect::<Vec<_>>()
        .join(" and ")
}
