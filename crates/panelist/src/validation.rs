// SPDX-License-Identifier: Apache-2.0

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
}
