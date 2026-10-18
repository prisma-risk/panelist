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

//! Checks for panel query options, repeats, value mappings, and annotations.
//!
//! Kept apart from `validate.rs` so these rules read as one unit. Every check here guards a value Grafana stores without complaint and then ignores or misreads at render time.

use std::collections::HashSet;

use crate::{
    Annotation, Dashboard, DashboardItem, OverrideProperty, Panel, Repeat, ValidationError,
    ValueMapping, field::ValueMatch,
};

pub(crate) fn validate_options(dashboard: &Dashboard, errors: &mut Vec<ValidationError>) {
    let variables: HashSet<&str> = dashboard
        .variables
        .iter()
        .map(crate::Variable::name)
        .collect();

    for item in &dashboard.items {
        match item {
            DashboardItem::Row(row) => {
                if let Some(variable) = &row.repeat
                    && !variables.contains(variable.as_str())
                {
                    errors.push(ValidationError::UnknownRepeatVariable {
                        kind: "row",
                        title: row.title.clone(),
                        variable: variable.clone(),
                    });
                }
                for panel in &row.panels {
                    validate_panel_options(panel, &variables, errors);
                }
            }
            DashboardItem::Panel(panel) => validate_panel_options(panel, &variables, errors),
        }
    }

    for annotation in &dashboard.annotations {
        if annotation.name().trim().is_empty() {
            errors.push(ValidationError::MissingAnnotationName);
        }
        if let Annotation::Prometheus(prometheus) = annotation
            && prometheus.expr.trim().is_empty()
        {
            errors.push(ValidationError::MissingQueryExpression {
                panel: format!("annotation {}", prometheus.common.name),
            });
        }
    }
}

fn validate_panel_options(
    panel: &Panel,
    variables: &HashSet<&str>,
    errors: &mut Vec<ValidationError>,
) {
    let options = &panel.query_options;
    let mut invalid = |option: &'static str, value: String, expected: &'static str| {
        errors.push(ValidationError::InvalidPanelOption {
            panel: panel.title.clone(),
            option,
            value,
            expected,
        });
    };

    if let Some(interval) = &options.min_interval
        && !is_interval(interval)
    {
        invalid("min_interval", interval.clone(), INTERVAL);
    }
    if options.max_data_points == Some(0) {
        invalid("max_data_points", "0".to_owned(), "a positive number");
    }
    if let Some(time_from) = &options.time_from
        && !is_time_from(time_from)
    {
        invalid("time_from", time_from.clone(), TIME_FROM);
    }
    if let Some(time_shift) = &options.time_shift
        && !is_time_span(time_shift)
    {
        invalid("time_shift", time_shift.clone(), TIME_SPAN);
    }
    if let Some(Repeat::Horizontal(repeat)) = &panel.repeat
        && let Some(max_per_row) = repeat.max_per_row
        && !(1..=24).contains(&max_per_row)
    {
        invalid(
            "max_per_row",
            max_per_row.to_string(),
            "a number between 1 and 24",
        );
    }

    if let Some(repeat) = &panel.repeat
        && !variables.contains(repeat.variable())
    {
        errors.push(ValidationError::UnknownRepeatVariable {
            kind: "panel",
            title: panel.title.clone(),
            variable: repeat.variable().to_owned(),
        });
    }

    validate_mappings(panel, &panel.field_config.mappings, errors);
    for field_override in &panel.field_config.overrides {
        for property in &field_override.properties {
            if let OverrideProperty::Mappings(mappings) = property {
                validate_mappings(panel, mappings, errors);
            }
        }
    }
}

/// Rejects mappings Grafana would store and then never apply, or apply as a no-op.
///
/// A two-sided open range is what Grafana's mapping editor discards on save; a non-finite end cannot be written as JSON; an inverted range and an empty pattern never match; a mapping with neither text nor color matches and then stops later mappings from running while changing nothing.
fn validate_mappings(panel: &Panel, mappings: &[ValueMapping], errors: &mut Vec<ValidationError>) {
    let mut exact = HashSet::new();
    for mapping in mappings {
        let reason = match &mapping.matcher {
            ValueMatch::Value(value) => {
                if !exact.insert(value.as_str()) {
                    errors.push(ValidationError::DuplicateValueMapping {
                        panel: panel.title.clone(),
                        value: value.clone(),
                    });
                }
                None
            }
            ValueMatch::Range {
                from: None,
                to: None,
            } => Some("range must set at least one end"),
            ValueMatch::Range { from, to }
                if from.is_some_and(|from| !from.is_finite())
                    || to.is_some_and(|to| !to.is_finite()) =>
            {
                Some("range ends must be finite")
            }
            ValueMatch::Range {
                from: Some(from),
                to: Some(to),
            } if from > to => Some("range starts after it ends, so it can never match"),
            ValueMatch::Regex(pattern) if pattern.is_empty() => {
                Some("regex pattern must not be empty")
            }
            ValueMatch::Range { .. } | ValueMatch::Regex(_) | ValueMatch::Special(_) => None,
        };
        if let Some(reason) = reason {
            errors.push(ValidationError::InvalidValueMapping {
                panel: panel.title.clone(),
                reason,
            });
        }

        let has_text = mapping
            .text
            .as_ref()
            .is_some_and(|text| !text.trim().is_empty());
        if !has_text && mapping.color.is_none() {
            errors.push(ValidationError::InvalidValueMapping {
                panel: panel.title.clone(),
                reason: "sets neither text nor color, so it changes nothing",
            });
        }
    }
}

const INTERVAL: &str =
    "a number with a unit from ms, s, m, h, d, w, M, y (such as 30s or 1m), or a $variable";
const TIME_SPAN: &str = "a whole number with a unit from s, m, h, d, w, M, y, optionally rounded with /unit (such as 1h or 1d/d), or a $variable";
const TIME_FROM: &str = "a whole number with a unit from s, m, h, d, w, M, y (such as 24h), a now-relative expression (such as now/d), or a $variable";

/// Grafana's `describeInterval` (`packages/grafana-data/src/datetime/rangeutil.ts`, v13.0.2): a bare number of seconds, or a number followed by a unit. Grafana only anchors the pattern at the start and ignores trailing text; this check requires the whole string to match.
fn is_interval(value: &str) -> bool {
    if is_variable(value) {
        return true;
    }
    let digits = value.trim_end_matches(|ch: char| ch.is_ascii_alphabetic());
    let unit = &value[digits.len()..];
    is_decimal(digits) && (unit.is_empty() || unit == "ms" || is_unit(unit))
}

/// A relative span as Grafana's `describeTextRange` reads a panel time shift: `now-` is implied, the amount is whole, and a `/unit` suffix rounds the result.
fn is_time_span(value: &str) -> bool {
    if is_variable(value) {
        return true;
    }
    let (span, rounding) = value.split_once('/').unwrap_or((value, ""));
    let (amount, unit) = span.split_at(
        span.trim_end_matches(|ch: char| ch.is_ascii_alphabetic())
            .len(),
    );
    !amount.is_empty()
        && amount.bytes().all(|byte| byte.is_ascii_digit())
        && is_unit(unit)
        && (rounding.is_empty() && !value.ends_with('/') || is_unit(rounding))
}

/// A panel time override is a relative span, or a `now`-based date-math expression, which is passed through unchecked because Grafana's grammar for it is far wider than this check needs to be.
fn is_time_from(value: &str) -> bool {
    value.starts_with("now") || is_time_span(value)
}

fn is_variable(value: &str) -> bool {
    value.starts_with('$')
}

fn is_unit(unit: &str) -> bool {
    matches!(unit, "s" | "m" | "h" | "d" | "w" | "M" | "y")
}

fn is_decimal(value: &str) -> bool {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, "0"));
    !whole.is_empty()
        && !fraction.is_empty()
        && whole.bytes().all(|byte| byte.is_ascii_digit())
        && fraction.bytes().all(|byte| byte.is_ascii_digit())
}
