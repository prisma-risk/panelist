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

use std::fmt::{self, Display, Formatter, Write as _};

use thiserror::Error;

/// An ordered list of PromQL label matchers, shared between selectors.
///
/// Build the matchers that several queries have in common once, then attach them to each [`Selector`] with [`Selector::with`]. Matchers render in insertion order, so generated expressions are deterministic.
///
/// ```rust
/// use panelist::prelude::*;
///
/// let base = Matchers::new().eq("job", "api").re("instance", "$instance");
/// let requests = Selector::metric("http_requests_total").with(&base);
/// let errors = Selector::metric("http_requests_total").with(&base).re("status", "5..");
///
/// assert_eq!(requests.to_string(), r#"http_requests_total{job="api",instance=~"$instance"}"#);
/// assert_eq!(
///     errors.to_string(),
///     r#"http_requests_total{job="api",instance=~"$instance",status=~"5.."}"#,
/// );
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Matchers {
    matchers: Vec<Matcher>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Matcher {
    label: String,
    op: MatchOp,
    value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MatchOp {
    Equal,
    NotEqual,
    Regex,
    NotRegex,
}

impl MatchOp {
    const fn as_promql(self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::NotEqual => "!=",
            Self::Regex => "=~",
            Self::NotRegex => "!~",
        }
    }
}

impl Matchers {
    /// Creates an empty matcher list.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an equality matcher, `label="value"`.
    #[must_use]
    pub fn eq(self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.push(label.into(), MatchOp::Equal, value.into())
    }

    /// Adds a negated equality matcher, `label!="value"`.
    #[must_use]
    pub fn ne(self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.push(label.into(), MatchOp::NotEqual, value.into())
    }

    /// Adds a regular-expression matcher, `label=~"value"`.
    ///
    /// Pass the regular expression itself. Panelist escapes it into a PromQL string literal, so `r"\d+"` reaches Prometheus as `\d+`.
    #[must_use]
    pub fn re(self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.push(label.into(), MatchOp::Regex, value.into())
    }

    /// Adds a negated regular-expression matcher, `label!~"value"`.
    #[must_use]
    pub fn nre(self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.push(label.into(), MatchOp::NotRegex, value.into())
    }

    /// Appends every matcher in `other`, keeping their order.
    #[must_use]
    pub fn with(mut self, other: &Matchers) -> Self {
        self.matchers.extend_from_slice(&other.matchers);
        self
    }

    fn push(mut self, label: String, op: MatchOp, value: String) -> Self {
        self.matchers.push(Matcher { label, op, value });
        self
    }
}

impl Display for Matcher {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write_name(f, &self.label, is_label_name(self.label.as_bytes()))?;
        f.write_str(self.op.as_promql())?;
        write_quoted(f, &self.value)
    }
}

/// A PromQL instant-vector selector: an optional metric name followed by label matchers.
///
/// A selector renders through [`Display`] and converts into a [`String`], so it goes anywhere a query expression is accepted today: [`PrometheusQuery::new`](crate::PrometheusQuery::new), [`promql!`](crate::promql!), or a `format!` argument. Building one replaces hand-written selectors inside `format!`, where every literal brace has to be doubled.
///
/// Label values are always quoted, with backslashes, double quotes, and line breaks escaped. Grafana variables such as `$instance` pass through untouched. Names follow Prometheus rules: a name in the classic grammar renders bare, a name containing `$` renders verbatim because Grafana substitutes it before Prometheus parses the query, and any other name renders quoted using Prometheus 3's UTF-8 name syntax.
///
/// ```rust
/// use panelist::prelude::*;
///
/// let errors = Selector::metric("http_requests_total")
///     .eq("job", "api")
///     .re("instance", "$instance")
///     .re("status", "5..");
/// let query = PrometheusQuery::new(format!(
///     "sum(rate({}))",
///     errors.range("$__rate_interval"),
/// ));
///
/// assert_eq!(
///     errors.to_string(),
///     r#"http_requests_total{job="api",instance=~"$instance",status=~"5.."}"#,
/// );
/// # let _ = query;
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Selector {
    metric: Option<String>,
    matchers: Matchers,
}

impl Selector {
    /// Creates a selector without a metric name, such as `{job="api"}`.
    ///
    /// Prometheus rejects a selector unless at least one matcher can only match a non-empty value, so add one such as `eq("job", "api")`. The same syntax is a LogQL stream selector.
    ///
    /// ```rust
    /// use panelist::prelude::*;
    ///
    /// let up = Selector::new().eq("job", "api");
    /// assert_eq!(up.to_string(), r#"{job="api"}"#);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a selector for one metric.
    ///
    /// ```rust
    /// use panelist::prelude::*;
    ///
    /// assert_eq!(Selector::metric("up").to_string(), "up");
    /// assert_eq!(Selector::metric("up").eq("job", "api").to_string(), r#"up{job="api"}"#);
    /// ```
    #[must_use]
    pub fn metric(name: impl Into<String>) -> Self {
        Self {
            metric: Some(name.into()),
            matchers: Matchers::new(),
        }
    }

    /// Adds an equality matcher, `label="value"`.
    #[must_use]
    pub fn eq(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.matchers = self.matchers.eq(label, value);
        self
    }

    /// Adds a negated equality matcher, `label!="value"`.
    #[must_use]
    pub fn ne(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.matchers = self.matchers.ne(label, value);
        self
    }

    /// Adds a regular-expression matcher, `label=~"value"`.
    ///
    /// Pass the regular expression itself. Panelist escapes it into a PromQL string literal:
    ///
    /// ```rust
    /// use panelist::prelude::*;
    ///
    /// let selector = Selector::metric("http_requests_total").re("status", r"5\d\d");
    /// assert_eq!(selector.to_string(), r#"http_requests_total{status=~"5\\d\\d"}"#);
    /// ```
    #[must_use]
    pub fn re(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.matchers = self.matchers.re(label, value);
        self
    }

    /// Adds a negated regular-expression matcher, `label!~"value"`.
    #[must_use]
    pub fn nre(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.matchers = self.matchers.nre(label, value);
        self
    }

    /// Appends shared matchers after the ones already on this selector.
    #[must_use]
    pub fn with(mut self, matchers: &Matchers) -> Self {
        self.matchers = self.matchers.with(matchers);
        self
    }

    /// Attaches a range, producing a range-vector selector such as `up{job="api"}[5m]`.
    ///
    /// The range always follows the selector's closing brace, never a parenthesized expression, so a `rate(…)` argument built this way cannot turn into the stepless subquery `(…)[5m]` that Prometheus rejects. The range is written verbatim, so Grafana variables such as `$__rate_interval` work. The selector is borrowed, so one selector can feed both a range and an instant query.
    ///
    /// ```rust
    /// use panelist::prelude::*;
    ///
    /// let requests = Selector::metric("http_requests_total").eq("job", "api");
    /// assert_eq!(
    ///     format!("rate({})", requests.range("$__rate_interval")),
    ///     r#"rate(http_requests_total{job="api"}[$__rate_interval])"#,
    /// );
    /// ```
    #[must_use]
    pub fn range(&self, range: impl Into<String>) -> RangeSelector {
        RangeSelector {
            selector: self.clone(),
            range: range.into(),
        }
    }
}

impl Display for Selector {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let metric = self.metric.as_deref();
        let bare_metric = metric.filter(|name| renders_bare(name, is_metric_name(name.as_bytes())));
        if let Some(name) = bare_metric {
            f.write_str(name)?;
            if self.matchers.matchers.is_empty() {
                return Ok(());
            }
        }

        f.write_char('{')?;
        let mut first = true;
        if let Some(name) = metric
            && bare_metric.is_none()
        {
            write_quoted(f, name)?;
            first = false;
        }
        for matcher in &self.matchers.matchers {
            if !first {
                f.write_char(',')?;
            }
            first = false;
            matcher.fmt(f)?;
        }
        f.write_char('}')
    }
}

/// A PromQL range-vector selector, such as `http_requests_total{job="api"}[$__rate_interval]`.
///
/// Created by [`Selector::range`]. It renders and converts like a [`Selector`].
///
/// ```rust
/// use panelist::prelude::*;
///
/// let up = Selector::metric("up").eq("job", "api").range("5m");
/// let query = PrometheusQuery::new(format!("avg_over_time({up})"));
/// assert_eq!(up.to_string(), r#"up{job="api"}[5m]"#);
/// # let _ = query;
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RangeSelector {
    selector: Selector,
    range: String,
}

impl Display for RangeSelector {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}]", self.selector, self.range)
    }
}

macro_rules! impl_into_string {
    ($($type:ty),* $(,)?) => {
        $(
            impl From<$type> for String {
                fn from(value: $type) -> Self {
                    value.to_string()
                }
            }

            impl From<&$type> for String {
                fn from(value: &$type) -> Self {
                    value.to_string()
                }
            }
        )*
    };
}

impl_into_string!(Selector, RangeSelector);

/// Whether a name renders without quotes: it fits the classic grammar, or it holds a Grafana variable that Grafana replaces before Prometheus sees the query.
fn renders_bare(name: &str, classic: bool) -> bool {
    classic || name.contains('$')
}

fn write_name(f: &mut Formatter<'_>, name: &str, classic: bool) -> fmt::Result {
    if renders_bare(name, classic) {
        f.write_str(name)
    } else {
        write_quoted(f, name)
    }
}

/// Writes a PromQL double-quoted string literal. PromQL strings use Go escape rules, and a raw line break would end the literal.
fn write_quoted(f: &mut Formatter<'_>, value: &str) -> fmt::Result {
    f.write_char('"')?;
    for character in value.chars() {
        match character {
            '\\' => f.write_str(r"\\")?,
            '"' => f.write_str(r#"\""#)?,
            '\n' => f.write_str(r"\n")?,
            '\r' => f.write_str(r"\r")?,
            '\t' => f.write_str(r"\t")?,
            other => f.write_char(other)?,
        }
    }
    f.write_char('"')
}

/// Classic Prometheus metric-name grammar, `[a-zA-Z_:][a-zA-Z0-9_:]*`.
fn is_metric_name(name: &[u8]) -> bool {
    is_name(name, 0, name.len(), true)
}

/// Classic Prometheus label-name grammar, `[a-zA-Z_][a-zA-Z0-9_]*`.
const fn is_label_name(name: &[u8]) -> bool {
    is_name(name, 0, name.len(), false)
}

const fn is_name(bytes: &[u8], start: usize, end: usize, allow_colon: bool) -> bool {
    if start >= end {
        return false;
    }
    let mut index = start;
    while index < end {
        let byte = bytes[index];
        let valid = byte.is_ascii_alphabetic()
            || byte == b'_'
            || (allow_colon && byte == b':')
            || (index > start && byte.is_ascii_digit());
        if !valid {
            return false;
        }
        index += 1;
    }
    true
}

/// A mistake in a legend template passed to [`legend()`](fn@crate::legend).
///
/// Offsets are byte offsets into the template.
///
/// ```rust
/// use panelist::{LegendFormatError, legend};
///
/// assert_eq!(
///     legend("{pod"),
///     Err(LegendFormatError::UnclosedPlaceholder { offset: 0 }),
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum LegendFormatError {
    /// A `{` opens a placeholder that is never closed.
    #[error(
        "legend placeholder opened at byte {offset} is never closed; write `{{{{` for a literal `{{`"
    )]
    UnclosedPlaceholder {
        /// Offset of the opening brace.
        offset: usize,
    },
    /// A `}` has no matching `{`.
    #[error("unmatched `}}` at byte {offset}; write `}}}}` for a literal `}}`")]
    UnmatchedBrace {
        /// Offset of the closing brace.
        offset: usize,
    },
    /// A placeholder is not a Prometheus label name.
    #[error(
        "legend placeholder `{{{name}}}` at byte {offset} is not a Prometheus label name like `pod` or `__name__`"
    )]
    InvalidLabelName {
        /// The text between the braces.
        name: String,
        /// Offset of the opening brace.
        offset: usize,
    },
    /// The template uses Grafana's own `{{label}}` spelling, which this syntax reads as the literal text `{label}`.
    #[error(
        "`{{{{{name}}}}}` at byte {offset} is Grafana's placeholder syntax; write `{{{name}}}` and Panelist emits the double braces"
    )]
    GrafanaPlaceholder {
        /// The label name between the double braces.
        name: String,
        /// Offset of the first opening brace.
        offset: usize,
    },
}

/// Converts a legend template with single-brace placeholders into Grafana's legend format.
///
/// Grafana writes a label placeholder as `{{label}}`, which turns into `{{{{label}}}}` inside `format!`. This function takes the `format!` spelling instead: `{label}` becomes `{{label}}`, a doubled brace (`{{` or `}}`) is a literal brace, and all other text passes through unchanged. The result goes to [`PrometheusQuery::legend_format`](crate::PrometheusQuery::legend_format) or the `legend_format:` key of [`dashboard!`](crate::dashboard!).
///
/// Prefer [`legend!`](macro@crate::legend) for constant templates. It runs the same checks at compile time and returns the [`String`] directly.
///
/// Placeholders use the classic label-name grammar, `[a-zA-Z_][a-zA-Z0-9_]*`. For a UTF-8 label name, pass Grafana's own spelling, such as `"{{service.name}}"`, straight to `legend_format`.
///
/// # Errors
///
/// Returns [`LegendFormatError`] when a placeholder is not closed, a `}` has no matching `{`, a placeholder is not a label name, or the template already uses Grafana's `{{label}}` spelling, which would otherwise render as the literal text `{label}`.
///
/// ```rust
/// use panelist::{LegendFormatError, legend};
///
/// assert_eq!(legend("{method} {status}")?, "{{method}} {{status}}");
/// assert_eq!(legend("{{ {pod} }}")?, "{ {{pod}} }");
/// assert!(matches!(
///     legend("{status-code}"),
///     Err(LegendFormatError::InvalidLabelName { .. }),
/// ));
/// # Ok::<(), LegendFormatError>(())
/// ```
pub fn legend(template: &str) -> Result<String, LegendFormatError> {
    match scan_legend(template.as_bytes()) {
        Ok(()) => Ok(render_legend(template)),
        Err(issue) => Err(issue.into_error(template)),
    }
}

/// Builds a Grafana legend format from a template checked at compile time.
///
/// Write each label placeholder with single braces, as in `format!`: `legend!("{namespace} · {pod}")` produces `{{namespace}} · {{pod}}`. Double a brace for a literal one. The conversion rules are those of [`legend()`](fn@crate::legend), but every mistake is a compile error instead of a [`LegendFormatError`].
///
/// The template must be a constant `&str`: a literal, a `const`, or a `concat!`. Use [`legend()`](fn@crate::legend) for templates built at run time.
///
/// ```rust
/// use panelist::prelude::*;
///
/// const BY_POD: &str = "{namespace} · {pod}";
///
/// let query = PrometheusQuery::new("sum by (namespace, pod) (up)").legend_format(legend!(BY_POD));
/// assert_eq!(legend!(BY_POD), "{{namespace}} · {{pod}}");
/// # let _ = query;
/// ```
///
/// A placeholder that is not a label name does not compile:
///
/// ```compile_fail
/// let legend = panelist::legend!("{status-code}");
/// ```
#[macro_export]
macro_rules! legend {
    ($template:expr $(,)?) => {
        $crate::__private::render_legend(const { $crate::__private::check_legend($template) })
    };
}

/// A legend-template mistake found by the `const` scanner, which cannot allocate the owned error.
#[derive(Debug, Clone, Copy)]
enum LegendIssue {
    Unclosed {
        offset: usize,
    },
    UnmatchedBrace {
        offset: usize,
    },
    InvalidLabelName {
        offset: usize,
        end: usize,
    },
    GrafanaPlaceholder {
        offset: usize,
        start: usize,
        end: usize,
    },
}

impl LegendIssue {
    fn into_error(self, template: &str) -> LegendFormatError {
        let slice =
            |start: usize, end: usize| template.get(start..end).unwrap_or_default().to_owned();
        match self {
            Self::Unclosed { offset } => LegendFormatError::UnclosedPlaceholder { offset },
            Self::UnmatchedBrace { offset } => LegendFormatError::UnmatchedBrace { offset },
            Self::InvalidLabelName { offset, end } => LegendFormatError::InvalidLabelName {
                name: slice(offset + 1, end),
                offset,
            },
            Self::GrafanaPlaceholder { offset, start, end } => {
                LegendFormatError::GrafanaPlaceholder {
                    name: slice(start, end),
                    offset,
                }
            }
        }
    }
}

/// Validates a legend template. Braces are ASCII, so a byte scan never splits a multi-byte character at a reported offset.
const fn scan_legend(template: &[u8]) -> Result<(), LegendIssue> {
    let len = template.len();
    let mut index = 0;
    while index < len {
        match template[index] {
            b'{' if index + 1 < len && template[index + 1] == b'{' => {
                if let Some((start, end)) = grafana_placeholder(template, index) {
                    return Err(LegendIssue::GrafanaPlaceholder {
                        offset: index,
                        start,
                        end,
                    });
                }
                index += 2;
            }
            b'{' => {
                let mut end = index + 1;
                while end < len && template[end] != b'}' {
                    end += 1;
                }
                if end == len {
                    return Err(LegendIssue::Unclosed { offset: index });
                }
                if !is_name(template, index + 1, end, false) {
                    return Err(LegendIssue::InvalidLabelName { offset: index, end });
                }
                index = end + 1;
            }
            b'}' if index + 1 < len && template[index + 1] == b'}' => index += 2,
            b'}' => return Err(LegendIssue::UnmatchedBrace { offset: index }),
            _ => index += 1,
        }
    }
    Ok(())
}

/// Recognizes Grafana's `{{label}}` spelling, including the inner spaces Grafana allows, starting at `offset`. Returns the label's byte range.
const fn grafana_placeholder(template: &[u8], offset: usize) -> Option<(usize, usize)> {
    let len = template.len();
    let mut start = offset + 2;
    while start < len && template[start] == b' ' {
        start += 1;
    }
    let mut end = start;
    while end < len && (template[end].is_ascii_alphanumeric() || template[end] == b'_') {
        end += 1;
    }
    let mut close = end;
    while close < len && template[close] == b' ' {
        close += 1;
    }
    let closed = close + 1 < len && template[close] == b'}' && template[close + 1] == b'}';
    if closed && is_name(template, start, end, false) {
        Some((start, end))
    } else {
        None
    }
}

/// Converts a template that [`scan_legend`] accepted. It never panics, even on input the scanner would reject.
#[doc(hidden)]
#[must_use]
pub fn render_legend(template: &str) -> String {
    let mut rendered = String::with_capacity(template.len() + 8);
    let mut rest = template;
    while let Some(position) = rest.find(['{', '}']) {
        rendered.push_str(&rest[..position]);
        let tail = &rest[position..];
        if let Some(after) = tail.strip_prefix("{{") {
            rendered.push('{');
            rest = after;
        } else if let Some(after) = tail.strip_prefix("}}") {
            rendered.push('}');
            rest = after;
        } else if let Some(after) = tail.strip_prefix('{') {
            let end = after.find('}').unwrap_or(after.len());
            rendered.push_str("{{");
            rendered.push_str(&after[..end]);
            rendered.push_str("}}");
            rest = after.get(end + 1..).unwrap_or_default();
        } else {
            rendered.push('}');
            rest = &tail[1..];
        }
    }
    rendered.push_str(rest);
    rendered
}

/// Checks a constant legend template, failing compilation through a `const` panic.
#[doc(hidden)]
#[must_use]
pub const fn check_legend(template: &'static str) -> &'static str {
    match scan_legend(template.as_bytes()) {
        Ok(()) => template,
        Err(LegendIssue::Unclosed { .. }) => {
            panic!("legend!: a `{{` placeholder is never closed; write `{{{{` for a literal `{{`")
        }
        Err(LegendIssue::UnmatchedBrace { .. }) => {
            panic!("legend!: unmatched `}}`; write `}}}}` for a literal `}}`")
        }
        Err(LegendIssue::InvalidLabelName { .. }) => {
            panic!("legend!: a placeholder is not a Prometheus label name like `pod` or `__name__`")
        }
        Err(LegendIssue::GrafanaPlaceholder { .. }) => panic!(
            "legend!: `{{{{label}}}}` is Grafana's placeholder syntax; write `{{label}}` and Panelist emits the double braces"
        ),
    }
}
