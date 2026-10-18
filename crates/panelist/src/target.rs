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

/// The Grafana release line a dashboard is written for.
///
/// A target picks the Classic `schemaVersion` Panelist emits and is the place where any wire difference between Grafana releases is gated. Grafana runs every dashboard migration newer than the stored `schemaVersion` each time it loads the dashboard, and a save writes the migrated version back. Emitting exactly the version the running Grafana considers current means no migration rewrites the JSON on load, so the committed file and Grafana's export stay identical.
///
/// Each target names the Grafana release the committed golden dashboards are round-tripped through by `make verify-grafana`. The schema version for each comes from `DASHBOARD_SCHEMA_VERSION` in Grafana's `DashboardMigrator.ts` at that release tag:
///
/// | Target | Verified release | `schemaVersion` |
/// | --- | --- | --- |
/// | [`V11_5`](Self::V11_5) | 11.5.2 | 40 |
/// | [`V13`](Self::V13) | 13.0.2 | 42 |
///
/// For what Panelist emits today, the two targets differ only in `schemaVersion`. Every other property round-trips unchanged through both releases. A future target may gate more than that, which is why the target, not a raw number, is the primary setting.
///
/// Releases between the targets use other numbers: 11.6, 12.0 and 12.1 consider 41 current, and 12.2 onward consider 42 current. For an unlisted release, pick the nearest older target and override the number with [`Dashboard::schema_version`](crate::Dashboard::schema_version).
///
/// ```rust
/// use panelist::{Dashboard, GrafanaTarget};
///
/// let json = Dashboard::new("Service health")
///     .target(GrafanaTarget::V11_5)
///     .to_json()?;
/// assert!(json.contains(r#""schemaVersion":40"#));
/// # Ok::<(), panelist::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum GrafanaTarget {
    /// Grafana 11.5.x, verified against 11.5.2. Emits `schemaVersion` 40.
    V11_5,
    /// Grafana 13.0.x, verified against 13.0.2. Emits `schemaVersion` 42, the final Classic schema version: Grafana stopped adding Classic migrations at 42 and moves new ones into panel plugins.
    #[default]
    V13,
}

impl GrafanaTarget {
    /// Every supported target, oldest first.
    pub const ALL: &'static [Self] = &[Self::V11_5, Self::V13];

    /// The Classic dashboard `schemaVersion` this target's Grafana considers current.
    #[must_use]
    pub const fn schema_version(self) -> u32 {
        match self {
            Self::V11_5 => 40,
            Self::V13 => 42,
        }
    }

    /// The exact Grafana release `make verify-grafana` round-trips this target's golden dashboards through.
    #[must_use]
    pub const fn verified_release(self) -> &'static str {
        match self {
            Self::V11_5 => "11.5.2",
            Self::V13 => "13.0.2",
        }
    }
}
