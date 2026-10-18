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

use serde_json::Value;

use crate::{Color, DataSource};

/// A dashboard annotation query: events from a datasource drawn as markers on every time-based panel.
///
/// Annotations are listed in the dashboard's annotation toggles in the order they are added. Use [`PrometheusAnnotation`] for the typed Prometheus query and [`RawAnnotation`] for any other datasource.
///
/// Grafana also has a built-in "Annotations & Alerts" query for annotations stored in Grafana itself. Grafana inserts it at the top of the list whenever a dashboard loads without it, so Panelist does not emit it by default; set [`crate::Dashboard::built_in_annotation`] to configure it instead.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Annotation {
    /// A typed Prometheus annotation query.
    Prometheus(PrometheusAnnotation),
    /// An annotation for a datasource Panelist does not model.
    Raw(RawAnnotation),
}

impl Annotation {
    /// Returns the name shown on the annotation's dashboard toggle.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Prometheus(annotation) => &annotation.common.name,
            Self::Raw(annotation) => &annotation.common.name,
        }
    }
}

impl From<PrometheusAnnotation> for Annotation {
    fn from(value: PrometheusAnnotation) -> Self {
        Self::Prometheus(value)
    }
}

impl From<RawAnnotation> for Annotation {
    fn from(value: RawAnnotation) -> Self {
        Self::Raw(value)
    }
}

/// Settings every Grafana annotation query has, whatever its datasource.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AnnotationCommon {
    pub(crate) name: String,
    pub(crate) datasource: Option<DataSource>,
    pub(crate) enable: bool,
    pub(crate) hide: bool,
    pub(crate) icon_color: Color,
}

impl AnnotationCommon {
    fn new(name: String) -> Self {
        Self {
            name,
            datasource: None,
            enable: true,
            hide: false,
            icon_color: Color::Red,
        }
    }
}

macro_rules! annotation_common_setters {
    () => {
        /// Sets the datasource. Without one, the dashboard datasource is used, as it is for panels.
        #[must_use]
        pub fn datasource(mut self, datasource: DataSource) -> Self {
            self.common.datasource = Some(datasource);
            self
        }

        /// Controls whether the query runs when the dashboard loads. Defaults to `true`.
        ///
        /// A disabled annotation can still be switched on from its dashboard toggle.
        #[must_use]
        pub fn enable(mut self, enable: bool) -> Self {
            self.common.enable = enable;
            self
        }

        /// Hides the annotation's toggle from the dashboard controls. Defaults to `false`.
        #[must_use]
        pub fn hide(mut self, hide: bool) -> Self {
            self.common.hide = hide;
            self
        }

        /// Sets the color of the event markers. Defaults to red, as in Grafana's annotation editor.
        #[must_use]
        pub fn icon_color(mut self, color: Color) -> Self {
            self.common.icon_color = color;
            self
        }
    };
}

/// A Prometheus annotation query.
///
/// Grafana creates one event for each run of consecutive samples above zero, so an expression such as `changes(process_start_time_seconds[2m]) > 0` marks restarts. The title and text are legend-style templates over the series labels, and `tag_keys` turns label values into event tags.
///
/// ```rust
/// use panelist::prelude::*;
///
/// let restarts = PrometheusAnnotation::new(
///     "Restarts",
///     "changes(process_start_time_seconds{job=\"api\"}[2m]) > 0",
/// )
/// .title_format("Restart")
/// .text_format("{{instance}} restarted")
/// .tag_keys(["job", "instance"])
/// .step("60s")
/// .icon_color(Color::Orange);
///
/// let dashboard = Dashboard::new("API")
///     .datasource(prometheus("prometheus-main"))
///     .annotation(restarts);
/// # let _ = dashboard;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PrometheusAnnotation {
    pub(crate) common: AnnotationCommon,
    pub(crate) expr: String,
    pub(crate) step: Option<String>,
    pub(crate) title_format: Option<String>,
    pub(crate) text_format: Option<String>,
    pub(crate) tag_keys: Vec<String>,
    pub(crate) use_value_for_time: bool,
}

impl PrometheusAnnotation {
    /// Creates an enabled annotation from a PromQL expression.
    #[must_use]
    pub fn new(name: impl Into<String>, expr: impl Into<String>) -> Self {
        Self {
            common: AnnotationCommon::new(name.into()),
            expr: expr.into(),
            step: None,
            title_format: None,
            text_format: None,
            tag_keys: Vec::new(),
            use_value_for_time: false,
        }
    }

    annotation_common_setters!();

    /// Sets the minimum query step, such as `60s`. Grafana groups samples closer together than one step into a single region event.
    #[must_use]
    pub fn step(mut self, step: impl Into<String>) -> Self {
        self.step = Some(step.into());
        self
    }

    /// Sets the event title template, for example `{{instance}}`.
    #[must_use]
    pub fn title_format(mut self, format: impl Into<String>) -> Self {
        self.title_format = Some(format.into());
        self
    }

    /// Sets the event text template, for example `{{job}} deployed {{version}}`.
    #[must_use]
    pub fn text_format(mut self, format: impl Into<String>) -> Self {
        self.text_format = Some(format.into());
        self
    }

    /// Sets the series labels whose values become event tags.
    #[must_use]
    pub fn tag_keys(mut self, keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tag_keys = keys.into_iter().map(Into::into).collect();
        self
    }

    /// Reads each sample's value as the event timestamp in milliseconds, instead of the sample time.
    #[must_use]
    pub fn use_value_for_time(mut self, use_value_for_time: bool) -> Self {
        self.use_value_for_time = use_value_for_time;
        self
    }
}

/// An annotation query for a datasource Panelist does not model.
///
/// Panelist emits the common keys (`name`, `datasource`, `enable`, `hide`, `iconColor`) and flattens every [`RawAnnotation::field`] onto the same object, after the common keys, so a field wins over a common key of the same name.
///
/// ```rust
/// use panelist::prelude::*;
/// use serde_json::json;
///
/// let deploys = RawAnnotation::new("Deploys")
///     .datasource(loki("loki-main"))
///     .field("target", json!({"expr": "{job=\"deployer\"}", "refId": "Anno"}))
///     .field("textFormat", json!("{{version}}"));
/// # let _ = deploys;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RawAnnotation {
    pub(crate) common: AnnotationCommon,
    pub(crate) fields: BTreeMap<String, Value>,
}

impl RawAnnotation {
    /// Creates an enabled annotation with no datasource-specific fields.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            common: AnnotationCommon::new(name.into()),
            fields: BTreeMap::new(),
        }
    }

    annotation_common_setters!();

    /// Adds a datasource-specific key to the annotation object.
    #[must_use]
    pub fn field(mut self, key: impl Into<String>, value: Value) -> Self {
        self.fields.insert(key.into(), value);
        self
    }
}

/// Grafana's built-in "Annotations & Alerts" query, which shows annotations and alert state changes stored in Grafana.
///
/// Grafana adds this entry at the top of the annotation list whenever a dashboard loads without one, enabled and with its toggle hidden. Setting it with [`crate::Dashboard::built_in_annotation`] makes Panelist emit it first with the settings chosen here, for example to stop alert annotations from drawing on every panel.
///
/// ```rust
/// use panelist::prelude::*;
///
/// let dashboard = Dashboard::new("API").built_in_annotation(BuiltInAnnotation::new().enable(false));
/// # let _ = dashboard;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltInAnnotation {
    pub(crate) enable: bool,
    pub(crate) hide: bool,
    pub(crate) icon_color: Option<Color>,
}

impl BuiltInAnnotation {
    /// Starts from the settings Grafana itself inserts: enabled, toggle hidden, default color.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Controls whether Grafana's stored annotations and alert events are drawn.
    #[must_use]
    pub fn enable(mut self, enable: bool) -> Self {
        self.enable = enable;
        self
    }

    /// Hides the toggle from the dashboard controls.
    #[must_use]
    pub fn hide(mut self, hide: bool) -> Self {
        self.hide = hide;
        self
    }

    /// Sets the marker color. Defaults to Grafana's `rgba(0, 211, 255, 1)`.
    #[must_use]
    pub fn icon_color(mut self, color: Color) -> Self {
        self.icon_color = Some(color);
        self
    }
}

impl Default for BuiltInAnnotation {
    fn default() -> Self {
        Self {
            enable: true,
            hide: true,
            icon_color: None,
        }
    }
}
