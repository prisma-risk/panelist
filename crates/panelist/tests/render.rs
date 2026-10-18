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

//! Rendering a dashboard set into a directory and checking it.

use std::{fs, path::PathBuf};

use panelist::prelude::*;
use panelist::{
    DashboardSet, ValidationError,
    render::{DashboardProblem, RenderError, StaleReport},
};

fn service(uid: &str) -> Dashboard {
    Dashboard::new(format!("Service {uid}"))
        .uid(uid)
        .panel(Stat::new("Uptime"))
}

fn set() -> DashboardSet {
    DashboardSet::new()
        .dashboard(service("beta"))
        .dashboard(service("alpha"))
}

fn stale_report(result: Result<(), RenderError>) -> StaleReport {
    match result {
        Err(RenderError::Stale(report)) => report,
        other => panic!("expected a stale report, got {other:?}"),
    }
}

fn problems(result: Result<impl std::fmt::Debug, RenderError>) -> Vec<DashboardProblem> {
    match result {
        Err(RenderError::Dashboards(problems)) => problems,
        other => panic!("expected dashboard problems, got {other:?}"),
    }
}

#[test]
fn renders_one_file_per_uid_with_write_json_bytes() {
    let files = set().render().unwrap();
    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        ["alpha.json", "beta.json"]
    );

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("expected.json");
    service("alpha").write_json(&path).unwrap();
    assert_eq!(files["alpha.json"], fs::read_to_string(path).unwrap());
    assert!(files["alpha.json"].ends_with("}\n"));
    assert_eq!(
        files["alpha.json"],
        format!("{}\n", service("alpha").to_json_pretty().unwrap())
    );
}

#[test]
fn write_then_check_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("nested/dashboards");

    let report = set().write_dir(&out).unwrap();
    assert_eq!(
        report.written(),
        [out.join("alpha.json"), out.join("beta.json")]
    );
    assert!(report.unchanged().is_empty());
    assert!(report.removed().is_empty());
    set().check_dir(&out).unwrap();

    let again = set().write_dir(&out).unwrap();
    assert!(again.written().is_empty());
    assert_eq!(again.unchanged().len(), 2);
}

#[test]
fn check_reports_missing_files_and_a_missing_directory() {
    let dir = tempfile::tempdir().unwrap();
    let absent = dir.path().join("never-rendered");
    let report = stale_report(set().check_dir(&absent));
    assert_eq!(
        report.missing(),
        [absent.join("alpha.json"), absent.join("beta.json")]
    );
    assert!(report.stale().is_empty());
    assert!(report.orphaned().is_empty());
    assert!(!absent.exists(), "check must not create the directory");

    set().write_dir(dir.path()).unwrap();
    fs::remove_file(dir.path().join("beta.json")).unwrap();
    let report = stale_report(set().check_dir(dir.path()));
    assert_eq!(report.missing(), [dir.path().join("beta.json")]);
}

#[test]
fn check_reports_stale_files_and_write_fixes_them() {
    let dir = tempfile::tempdir().unwrap();
    set().write_dir(dir.path()).unwrap();

    let changed = DashboardSet::new()
        .dashboard(service("beta").refresh("1m"))
        .dashboard(service("alpha"));
    let report = stale_report(changed.check_dir(dir.path()));
    assert_eq!(report.stale(), [dir.path().join("beta.json")]);
    assert!(report.missing().is_empty());

    // A file differing only by its trailing newline is stale too.
    let alpha = dir.path().join("alpha.json");
    let trimmed = fs::read_to_string(&alpha).unwrap().trim_end().to_owned();
    fs::write(&alpha, trimmed).unwrap();
    let report = stale_report(changed.check_dir(dir.path()));
    assert_eq!(report.stale(), [alpha, dir.path().join("beta.json")]);

    let written = changed.write_dir(dir.path()).unwrap();
    assert_eq!(written.written().len(), 2);
    changed.check_dir(dir.path()).unwrap();
}

#[test]
fn check_reports_orphans_and_write_removes_only_them() {
    let dir = tempfile::tempdir().unwrap();
    set().write_dir(dir.path()).unwrap();
    let orphan = dir.path().join("renamed-away.json");
    fs::write(&orphan, "{}\n").unwrap();
    fs::write(dir.path().join("README.md"), "kept\n").unwrap();
    fs::create_dir(dir.path().join("subdir.json")).unwrap();
    fs::write(dir.path().join("subdir.json/inner.json"), "{}\n").unwrap();

    let report = stale_report(set().check_dir(dir.path()));
    assert_eq!(report.orphaned(), std::slice::from_ref(&orphan));
    assert!(report.stale().is_empty());
    assert!(report.missing().is_empty());
    let message = report.to_string();
    assert!(message.contains("1 orphaned"), "{message}");
    assert!(
        message.contains(&format!("orphaned: {}", orphan.display())),
        "{message}"
    );

    let written = set().write_dir(dir.path()).unwrap();
    assert_eq!(written.removed(), std::slice::from_ref(&orphan));
    assert!(!orphan.exists());
    assert!(dir.path().join("README.md").exists());
    assert!(dir.path().join("subdir.json/inner.json").exists());
    set().check_dir(dir.path()).unwrap();
}

#[test]
fn a_report_lists_every_kind_of_drift_at_once() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("alpha.json"), "{}\n").unwrap();
    fs::write(dir.path().join("gone.json"), "{}\n").unwrap();
    let report = stale_report(set().check_dir(dir.path()));
    let paths = |names: &[&str]| -> Vec<PathBuf> {
        names.iter().map(|name| dir.path().join(name)).collect()
    };
    assert_eq!(report.stale(), paths(&["alpha.json"]));
    assert_eq!(report.missing(), paths(&["beta.json"]));
    assert_eq!(report.orphaned(), paths(&["gone.json"]));
}

#[test]
fn missing_and_duplicate_uids_are_reported_together() {
    let set = DashboardSet::new()
        .dashboard(service("ops"))
        .dashboard(Dashboard::new("No UID"))
        .dashboard(Dashboard::new("Shouting").uid("OPS"));
    let found = problems(set.render());
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(matches!(
        &found[0],
        DashboardProblem::MissingUid { title } if title == "No UID"
    ));
    assert!(matches!(
        &found[1],
        DashboardProblem::DuplicateUid { uid, first, second }
            if uid == "OPS" && first == "Service ops" && second == "Shouting"
    ));
}

#[test]
fn invalid_dashboards_are_reported_by_title_and_nothing_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let set = DashboardSet::new()
        .dashboard(service("ops"))
        .dashboard(Dashboard::new("Broken").uid("broken").panel(Stat::new("")));
    let found = problems(set.write_dir(dir.path()));
    assert_eq!(found.len(), 1);
    match &found[0] {
        DashboardProblem::Invalid {
            title,
            source: panelist::Error::Validation(errors),
        } => {
            assert_eq!(title, "Broken");
            assert_eq!(errors.errors(), [ValidationError::MissingPanelTitle]);
        }
        other => panic!("unexpected problem {other:?}"),
    }
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn an_empty_set_is_refused_rather_than_wiping_the_directory() {
    let dir = tempfile::tempdir().unwrap();
    set().write_dir(dir.path()).unwrap();
    assert!(matches!(
        DashboardSet::new().write_dir(dir.path()),
        Err(RenderError::EmptySet)
    ));
    set().check_dir(dir.path()).unwrap();
}

#[test]
fn sets_collect_from_iterators() {
    let set: DashboardSet = ["a", "b"].into_iter().map(service).collect();
    assert_eq!(set.len(), 2);
    let mut extended = DashboardSet::new();
    assert!(extended.is_empty());
    extended.extend([service("c")]);
    extended.push(service("d"));
    assert_eq!(extended.render().unwrap().len(), 2);
}

#[test]
fn a_path_that_is_a_file_is_an_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("not-a-dir");
    fs::write(&file, "").unwrap();
    assert!(matches!(
        set().check_dir(&file),
        Err(RenderError::Io { path, .. }) if path == file
    ));
}
