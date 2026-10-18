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

//! Grafana targets and the dashboard-level properties Grafana enforces on save.

use panelist::ValidationError;
use panelist::prelude::*;
use serde_json::Value;

fn schema_version(dashboard: &Dashboard) -> Value {
    match serde_json::to_value(dashboard) {
        Ok(value) => value["schemaVersion"].clone(),
        Err(error) => panic!("test dashboard should serialize: {error}"),
    }
}

/// Values read from `DASHBOARD_SCHEMA_VERSION` in Grafana's `DashboardMigrator.ts` at each verified release tag. Changing one is a wire-format change that needs a fresh `make verify-grafana` run.
#[test]
fn each_target_emits_its_releases_schema_version() {
    assert_eq!(GrafanaTarget::V11_5.schema_version(), 40);
    assert_eq!(GrafanaTarget::V11_5.verified_release(), "11.5.2");
    assert_eq!(GrafanaTarget::V13.schema_version(), 42);
    assert_eq!(GrafanaTarget::V13.verified_release(), "13.0.2");
    for &target in GrafanaTarget::ALL {
        assert_eq!(
            schema_version(&Dashboard::new("Service").target(target)),
            target.schema_version()
        );
    }
}

#[test]
fn the_default_target_is_the_newest() {
    assert_eq!(GrafanaTarget::default(), GrafanaTarget::V13);
    assert_eq!(GrafanaTarget::ALL.last(), Some(&GrafanaTarget::default()));
    assert_eq!(schema_version(&Dashboard::new("Service")), 42);
}

#[test]
fn an_explicit_schema_version_overrides_the_target_in_either_order() {
    let before = Dashboard::new("Service")
        .schema_version(41)
        .target(GrafanaTarget::V11_5);
    let after = Dashboard::new("Service")
        .target(GrafanaTarget::V11_5)
        .schema_version(41);
    assert_eq!(schema_version(&before), 41);
    assert_eq!(schema_version(&after), 41);
}

#[test]
fn grafana_rejects_these_uids_so_validation_does_too() {
    let cases = [
        ("", "must not be empty"),
        (
            "a-uid-that-is-much-longer-than-forty-chars",
            "is longer than Grafana's 40-character limit",
        ),
        (
            "has space",
            "may only contain ASCII letters, digits, '-', and '_'",
        ),
        (
            "../escape",
            "may only contain ASCII letters, digits, '-', and '_'",
        ),
        (
            "ünïcode",
            "may only contain ASCII letters, digits, '-', and '_'",
        ),
    ];
    for (uid, reason) in cases {
        let errors = Dashboard::new("Service").uid(uid).validate().unwrap_err();
        assert_eq!(
            errors.errors(),
            [ValidationError::InvalidDashboardUid {
                uid: uid.to_owned(),
                reason,
            }]
        );
    }
}

#[test]
fn grafana_style_uids_are_accepted() {
    for uid in ["service-health", "Service_Health_2", "a", &"x".repeat(40)] {
        Dashboard::new("Service").uid(uid).validate().unwrap();
    }
}
