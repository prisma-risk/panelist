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

//! Panel ID strategies: positional by default, stable on request.

use panelist::ValidationError;
use panelist::prelude::*;
use serde_json::Value;

/// Every emitted panel and row as `(title, id)`, in emission order, including panels nested in collapsed rows.
fn ids(dashboard: &Dashboard) -> Vec<(String, u64)> {
    fn walk(panels: &[Value], out: &mut Vec<(String, u64)>) {
        for panel in panels {
            match (panel["title"].as_str(), panel["id"].as_u64()) {
                (Some(title), Some(id)) => out.push((title.to_owned(), id)),
                _ => panic!("panel without a title or numeric ID: {panel}"),
            }
            if let Some(nested) = panel["panels"].as_array() {
                walk(nested, out);
            }
        }
    }
    let value = match serde_json::to_value(dashboard) {
        Ok(value) => value,
        Err(error) => panic!("test dashboard should serialize: {error}"),
    };
    let mut out = Vec::new();
    walk(
        value["panels"].as_array().map_or(&[], Vec::as_slice),
        &mut out,
    );
    out
}

fn id_of(dashboard: &Dashboard, title: &str) -> u64 {
    let matches: Vec<u64> = ids(dashboard)
        .into_iter()
        .filter(|(candidate, _)| candidate == title)
        .map(|(_, id)| id)
        .collect();
    assert_eq!(matches.len(), 1, "expected exactly one {title:?}");
    matches[0]
}

fn errors(dashboard: &Dashboard) -> Vec<ValidationError> {
    match dashboard.validate() {
        Ok(()) => panic!("expected validation errors"),
        Err(errors) => errors.errors().to_vec(),
    }
}

/// An independent implementation of the derivation documented on `PanelIds::Stable`, written from the rustdoc rather than shared with the library, so the two cannot drift apart silently.
fn documented_id(fields: &[&str]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for field in fields {
        let mut bytes = (field.len() as u64).to_le_bytes().to_vec();
        bytes.extend_from_slice(field.as_bytes());
        for byte in bytes {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    65_536 + hash % ((1 << 30) - 65_536)
}

fn service_dashboard() -> Dashboard {
    Dashboard::new("Service")
        .panel_ids(PanelIds::Stable)
        .panel(Stat::new("Uptime"))
        .row(
            Row::new("Traffic")
                .panel(Timeseries::new("Requests"))
                .panel(Timeseries::new("Errors")),
        )
        .row(Row::new("Latency").panel(Timeseries::new("p99")))
}

#[test]
fn positional_is_the_default_and_numbers_in_order() {
    let dashboard = Dashboard::new("Service")
        .panel(Stat::new("Uptime"))
        .row(Row::new("Traffic").panel(Timeseries::new("Requests").key("requests")));
    assert_eq!(
        ids(&dashboard),
        [
            ("Uptime".to_owned(), 1),
            ("Traffic".to_owned(), 2),
            ("Requests".to_owned(), 3),
        ],
        "keys are ignored by positional IDs"
    );
    assert_eq!(
        ids(&dashboard),
        ids(&dashboard.clone().panel_ids(PanelIds::Positional))
    );
}

#[test]
fn positional_ids_shift_when_a_panel_is_inserted() {
    let before = Dashboard::new("Service").row(Row::new("Traffic").panel(Stat::new("Requests")));
    let after = Dashboard::new("Service")
        .panel(Stat::new("New"))
        .row(Row::new("Traffic").panel(Stat::new("Requests")));
    assert_ne!(id_of(&before, "Requests"), id_of(&after, "Requests"));
}

#[test]
fn stable_ids_follow_the_documented_derivation() {
    let dashboard = service_dashboard();
    assert_eq!(
        id_of(&dashboard, "Uptime"),
        documented_id(&["panel", "Uptime"])
    );
    assert_eq!(
        id_of(&dashboard, "Traffic"),
        documented_id(&["row", "title", "Traffic"])
    );
    assert_eq!(
        id_of(&dashboard, "Requests"),
        documented_id(&["panel", "title", "Traffic", "Requests"])
    );

    let keyed = Dashboard::new("Service").panel_ids(PanelIds::Stable).row(
        Row::new("Traffic")
            .key("traffic")
            .panel(Stat::new("Requests"))
            .panel(Stat::new("Errors").key("errors")),
    );
    assert_eq!(
        id_of(&keyed, "Traffic"),
        documented_id(&["row", "key", "traffic"])
    );
    assert_eq!(
        id_of(&keyed, "Requests"),
        documented_id(&["panel", "key", "traffic", "Requests"])
    );
    assert_eq!(id_of(&keyed, "Errors"), documented_id(&["key", "errors"]));
}

/// Pins concrete values so that a change to the derivation, which would break every published link, cannot pass by updating the reference implementation alongside it.
#[test]
fn stable_ids_are_pinned() {
    let dashboard = service_dashboard();
    assert_eq!(
        ids(&dashboard),
        [
            ("Uptime".to_owned(), PINNED_UPTIME),
            ("Traffic".to_owned(), PINNED_TRAFFIC),
            ("Requests".to_owned(), PINNED_REQUESTS),
            ("Errors".to_owned(), PINNED_ERRORS),
            ("Latency".to_owned(), PINNED_LATENCY),
            ("p99".to_owned(), PINNED_P99),
        ]
    );
}

const PINNED_UPTIME: u64 = 226_477_972;
const PINNED_TRAFFIC: u64 = 109_424_745;
const PINNED_REQUESTS: u64 = 115_962_683;
const PINNED_ERRORS: u64 = 913_694_408;
const PINNED_LATENCY: u64 = 738_901_714;
const PINNED_P99: u64 = 161_795_505;

#[test]
fn stable_ids_stay_in_grafanas_safe_range() {
    for (title, id) in ids(&service_dashboard()) {
        assert!((65_536..1 << 30).contains(&id), "{title}: {id}");
    }
}

#[test]
fn stable_ids_survive_insertion_and_removal() {
    let before = ids(&service_dashboard());
    let inserted = Dashboard::new("Service")
        .panel_ids(PanelIds::Stable)
        .panel(Text::new("Read me"))
        .panel(Stat::new("Uptime"))
        .row(
            Row::new("Traffic")
                .panel(Stat::new("Saturation"))
                .panel(Timeseries::new("Requests"))
                .panel(Timeseries::new("Errors")),
        )
        .row(Row::new("New row").panel(Stat::new("Other")))
        .row(Row::new("Latency").panel(Timeseries::new("p99")));
    let after = ids(&inserted);
    for (title, id) in before {
        assert!(
            after.contains(&(title.clone(), id)),
            "{title} moved to a new ID"
        );
    }
}

#[test]
fn stable_ids_survive_reordering() {
    let reordered = Dashboard::new("Service")
        .panel_ids(PanelIds::Stable)
        .panel(Stat::new("Uptime"))
        .row(Row::new("Latency").panel(Timeseries::new("p99")))
        .row(
            Row::new("Traffic")
                .panel(Timeseries::new("Errors"))
                .panel(Timeseries::new("Requests")),
        );
    let mut before = ids(&service_dashboard());
    let mut after = ids(&reordered);
    before.sort();
    after.sort();
    assert_eq!(before, after);
}

#[test]
fn stable_ids_survive_collapsing_a_row() {
    let collapsed = Dashboard::new("Service")
        .panel_ids(PanelIds::Stable)
        .panel(Stat::new("Uptime"))
        .row(
            Row::new("Traffic")
                .collapsed(true)
                .panel(Timeseries::new("Requests"))
                .panel(Timeseries::new("Errors")),
        )
        .row(Row::new("Latency").panel(Timeseries::new("p99")));
    assert_eq!(ids(&collapsed), ids(&service_dashboard()));
}

#[test]
fn renaming_changes_an_unkeyed_id_but_not_a_keyed_one() {
    let dashboard = |row: &str, title: &str, key: Option<&str>| {
        let mut panel = Timeseries::new(title);
        if let Some(key) = key {
            panel = panel.key(key);
        }
        Dashboard::new("Service")
            .panel_ids(PanelIds::Stable)
            .row(Row::new(row).panel(panel))
    };

    let unkeyed = id_of(&dashboard("Traffic", "Requests", None), "Requests");
    assert_ne!(
        unkeyed,
        id_of(&dashboard("Traffic", "Request rate", None), "Request rate")
    );
    assert_ne!(
        unkeyed,
        id_of(&dashboard("HTTP", "Requests", None), "Requests")
    );

    let keyed = id_of(
        &dashboard("Traffic", "Requests", Some("requests")),
        "Requests",
    );
    assert_eq!(
        keyed,
        id_of(
            &dashboard("HTTP", "Request rate", Some("requests")),
            "Request rate"
        )
    );
}

#[test]
fn a_row_key_keeps_its_panels_ids_across_a_row_rename() {
    let dashboard = |row: &str| {
        Dashboard::new("Service")
            .panel_ids(PanelIds::Stable)
            .row(Row::new(row).key("traffic").panel(Stat::new("Requests")))
    };
    assert_eq!(ids(&dashboard("Traffic")).len(), 2);
    assert_eq!(
        ids(&dashboard("Traffic"))
            .into_iter()
            .map(|(_, id)| id)
            .collect::<Vec<_>>(),
        ids(&dashboard("HTTP traffic"))
            .into_iter()
            .map(|(_, id)| id)
            .collect::<Vec<_>>()
    );
}

#[test]
fn equal_titles_in_different_rows_do_not_collide() {
    let dashboard = Dashboard::new("Service")
        .panel_ids(PanelIds::Stable)
        .panel(Stat::new("Requests"))
        .row(Row::new("East").panel(Stat::new("Requests")))
        .row(Row::new("West").panel(Stat::new("Requests")));
    let mut seen: Vec<u64> = ids(&dashboard).into_iter().map(|(_, id)| id).collect();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), 5);
}

#[test]
fn repeated_titles_in_one_row_are_a_collision_naming_both() {
    let dashboard = Dashboard::new("Service").panel_ids(PanelIds::Stable).row(
        Row::new("Traffic")
            .panel(Stat::new("Requests"))
            .panel(Stat::new("Requests")),
    );
    let id = u32::try_from(documented_id(&["panel", "title", "Traffic", "Requests"])).unwrap();
    assert_eq!(
        errors(&dashboard),
        [ValidationError::PanelIdCollision {
            id,
            first: "panel \"Requests\" in row \"Traffic\"".to_owned(),
            second: "panel \"Requests\" in row \"Traffic\"".to_owned(),
        }]
    );

    let fixed = Dashboard::new("Service").panel_ids(PanelIds::Stable).row(
        Row::new("Traffic")
            .panel(Stat::new("Requests"))
            .panel(Stat::new("Requests").key("requests-total")),
    );
    fixed.validate().unwrap();
}

#[test]
fn repeated_row_titles_collide_but_row_keys_resolve_them() {
    let dashboard = Dashboard::new("Service")
        .panel_ids(PanelIds::Stable)
        .row(Row::new("Misc").panel(Stat::new("A")))
        .row(Row::new("Misc").panel(Stat::new("B")));
    let id = u32::try_from(documented_id(&["row", "title", "Misc"])).unwrap();
    assert_eq!(
        errors(&dashboard),
        [ValidationError::PanelIdCollision {
            id,
            first: "row \"Misc\"".to_owned(),
            second: "row \"Misc\"".to_owned(),
        }]
    );

    Dashboard::new("Service")
        .panel_ids(PanelIds::Stable)
        .row(Row::new("Misc").key("misc-1").panel(Stat::new("A")))
        .row(Row::new("Misc").key("misc-2").panel(Stat::new("B")))
        .validate()
        .unwrap();
}

#[test]
fn duplicate_keys_collide_across_rows() {
    let dashboard = Dashboard::new("Service")
        .panel_ids(PanelIds::Stable)
        .row(Row::new("East").panel(Stat::new("Requests").key("requests")))
        .row(Row::new("West").panel(Stat::new("Requests").key("requests")));
    let id = u32::try_from(documented_id(&["key", "requests"])).unwrap();
    assert_eq!(
        errors(&dashboard),
        [ValidationError::PanelIdCollision {
            id,
            first: "panel \"Requests\" in row \"East\" (key \"requests\")".to_owned(),
            second: "panel \"Requests\" in row \"West\" (key \"requests\")".to_owned(),
        }]
    );
}

#[test]
fn an_explicit_id_equal_to_a_derived_one_is_a_collision() {
    let derived = u32::try_from(documented_id(&["panel", "Uptime"])).unwrap();
    let dashboard = Dashboard::new("Service")
        .panel_ids(PanelIds::Stable)
        .panel(Stat::new("Uptime"))
        .row(Row::new("Traffic").panel(Stat::new("Requests").id(derived)));
    assert_eq!(
        errors(&dashboard),
        [ValidationError::PanelIdCollision {
            id: derived,
            first: "panel \"Uptime\"".to_owned(),
            second: "panel \"Requests\" in row \"Traffic\" (explicit ID)".to_owned(),
        }]
    );
}

#[test]
fn explicit_ids_are_kept_and_duplicates_are_reported_once() {
    let dashboard = Dashboard::new("Service")
        .panel_ids(PanelIds::Stable)
        .panel(Stat::new("Uptime").id(7))
        .row(Row::new("Traffic").panel(Stat::new("Requests")));
    assert_eq!(id_of(&dashboard, "Uptime"), 7);

    let duplicated = Dashboard::new("Service")
        .panel_ids(PanelIds::Stable)
        .panel(Stat::new("Uptime").id(7))
        .panel(Stat::new("Errors").id(7));
    assert_eq!(
        errors(&duplicated),
        [ValidationError::DuplicatePanelId {
            id: 7,
            panel: "Errors".to_owned(),
        }]
    );
}

#[test]
fn collisions_are_reported_alongside_other_validation_errors() {
    let dashboard = Dashboard::new("")
        .panel_ids(PanelIds::Stable)
        .panel(Stat::new("Uptime"))
        .panel(Stat::new("Uptime"));
    let found = errors(&dashboard);
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(found[0], ValidationError::MissingDashboardTitle);
    assert!(matches!(found[1], ValidationError::PanelIdCollision { .. }));
}

#[test]
fn blank_keys_are_rejected_in_every_mode() {
    for mode in [PanelIds::Positional, PanelIds::Stable] {
        let dashboard = Dashboard::new("Service").panel_ids(mode).row(
            Row::new("Traffic")
                .key(" ")
                .panel(Stat::new("Requests").key("")),
        );
        assert_eq!(
            errors(&dashboard),
            [
                ValidationError::BlankRowKey {
                    row: "Traffic".to_owned(),
                },
                ValidationError::BlankPanelKey {
                    panel: "Requests".to_owned(),
                },
            ],
            "{mode:?}"
        );
    }
}

#[test]
fn raw_panels_take_keys_too() {
    let dashboard = Dashboard::new("Service")
        .panel_ids(PanelIds::Stable)
        .panel(RawPanel::new("news", "Feed").key("feed"));
    assert_eq!(id_of(&dashboard, "Feed"), documented_id(&["key", "feed"]));
}
