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

use serde::Serialize;

/// A Grafana datasource reference.
///
/// Grafana identifies modern datasources by plugin type and UID. The optional
/// display name is authoring metadata and is intentionally not serialized as a
/// substitute for the UID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DataSource {
    #[serde(rename = "type")]
    kind: String,
    uid: String,
    #[serde(skip)]
    name: Option<String>,
}

impl DataSource {
    /// Creates a datasource reference from a plugin type and stable UID.
    #[must_use]
    pub fn new(kind: impl Into<String>, uid: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            uid: uid.into(),
            name: None,
        }
    }

    /// Adds a human-readable name without changing the serialized identity.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Returns the Grafana datasource plugin type.
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Returns the datasource UID.
    #[must_use]
    pub fn uid(&self) -> &str {
        &self.uid
    }

    /// Returns the optional human-readable authoring name.
    #[must_use]
    pub fn display_name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

/// Creates a Prometheus datasource reference using `uid` as its stable ID.
#[must_use]
pub fn prometheus(uid: impl Into<String>) -> DataSource {
    DataSource::new("prometheus", uid)
}

/// Creates a Loki datasource reference using `uid` as its stable ID.
#[must_use]
pub fn loki(uid: impl Into<String>) -> DataSource {
    DataSource::new("loki", uid)
}
