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

use std::path::PathBuf;

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
    /// Two queries in a panel use the same explicit reference ID.
    #[error("panel \"{panel}\": duplicate query reference ID \"{ref_id}\"")]
    DuplicateQueryRefId {
        /// Panel title.
        panel: String,
        /// Duplicate query reference.
        ref_id: String,
    },
    /// A query expression is empty.
    #[error("panel \"{panel}\": query expression must not be empty")]
    MissingQueryExpression {
        /// Panel title.
        panel: String,
    },
    /// Threshold values are not strictly increasing.
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
}

fn format_quoted(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("\"{value}\""))
        .collect::<Vec<_>>()
        .join(" and ")
}
