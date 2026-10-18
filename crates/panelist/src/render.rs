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

//! Rendering a set of dashboards into a directory, and checking that directory in CI.
//!
//! A dashboards-as-code repository usually commits the generated JSON next to the Rust that produces it, so reviewers see the wire diff and Grafana can provision the files directly. That needs two operations: write every dashboard into a directory, and fail CI when the committed directory no longer matches the code. [`DashboardSet`] does both.
//!
//! Each dashboard is written to `<uid>.json` as validated, pretty-printed JSON with a trailing newline, the same bytes [`Dashboard::write_json`] produces. The directory belongs to the set: any other `.json` file directly inside it is an orphan, typically left behind by a renamed or deleted dashboard. Files with other extensions, hidden temporary files, and subdirectories are never touched.
//!
//! A small binary is enough to wire this into a repository:
//!
//! ```rust,no_run
//! use std::process::ExitCode;
//!
//! use panelist::{Dashboard, DashboardSet};
//!
//! fn dashboards() -> DashboardSet {
//!     DashboardSet::new()
//!         .dashboard(Dashboard::new("Service health").uid("service-health"))
//!         .dashboard(Dashboard::new("Database").uid("database"))
//! }
//!
//! fn main() -> ExitCode {
//!     let dir = "dashboards";
//!     let result = if std::env::args().any(|arg| arg == "--check") {
//!         dashboards().check_dir(dir)
//!     } else {
//!         dashboards().write_dir(dir).map(drop)
//!     };
//!     match result {
//!         Ok(()) => ExitCode::SUCCESS,
//!         Err(error) => {
//!             eprintln!("{error}");
//!             ExitCode::FAILURE
//!         }
//!     }
//! }
//! ```

use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsString,
    fmt, fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::Dashboard;

/// An ordered set of dashboards rendered together into one directory.
///
/// ```rust
/// use panelist::{Dashboard, DashboardSet, render::RenderError};
///
/// let dir = tempfile::tempdir()?;
/// let set = DashboardSet::new().dashboard(Dashboard::new("Service health").uid("service-health"));
///
/// // A directory that was never rendered is missing a file.
/// assert!(matches!(set.check_dir(dir.path()), Err(RenderError::Stale(_))));
///
/// set.write_dir(dir.path())?;
/// assert!(dir.path().join("service-health.json").exists());
/// set.check_dir(dir.path())?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DashboardSet {
    dashboards: Vec<Dashboard>,
}

impl DashboardSet {
    /// Creates an empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends one dashboard.
    #[must_use]
    pub fn dashboard(mut self, dashboard: Dashboard) -> Self {
        self.dashboards.push(dashboard);
        self
    }

    /// Appends several dashboards.
    #[must_use]
    pub fn dashboards(mut self, dashboards: impl IntoIterator<Item = Dashboard>) -> Self {
        self.dashboards.extend(dashboards);
        self
    }

    /// Appends one dashboard in place.
    pub fn push(&mut self, dashboard: Dashboard) {
        self.dashboards.push(dashboard);
    }

    /// Returns the number of dashboards in the set.
    #[must_use]
    pub fn len(&self) -> usize {
        self.dashboards.len()
    }

    /// Returns `true` if the set holds no dashboards.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.dashboards.is_empty()
    }

    /// Renders every dashboard, keyed by file name (`<uid>.json`) in sorted order.
    ///
    /// Every problem in the set is reported at once: each dashboard without a UID, each pair sharing a UID, and each dashboard that fails validation. UIDs are compared without regard to ASCII case, because `Ops.json` and `ops.json` are the same file on case-insensitive filesystems. An empty set is an error too, because writing it would delete every dashboard in the directory.
    pub fn render(&self) -> Result<BTreeMap<String, String>, RenderError> {
        if self.dashboards.is_empty() {
            return Err(RenderError::EmptySet);
        }
        let mut problems = Vec::new();
        let mut files = BTreeMap::new();
        let mut claimed: HashMap<String, &str> = HashMap::new();

        for dashboard in &self.dashboards {
            let title = dashboard.title.as_str();
            let contents = match dashboard.to_file_contents() {
                Ok(contents) => Some(contents),
                Err(source) => {
                    problems.push(DashboardProblem::Invalid {
                        title: title.to_owned(),
                        source,
                    });
                    None
                }
            };
            let Some(uid) = &dashboard.uid else {
                problems.push(DashboardProblem::MissingUid {
                    title: title.to_owned(),
                });
                continue;
            };
            if let Some(first) = claimed.insert(uid.to_ascii_lowercase(), title) {
                problems.push(DashboardProblem::DuplicateUid {
                    uid: uid.clone(),
                    first: first.to_owned(),
                    second: title.to_owned(),
                });
                continue;
            }
            if let Some(contents) = contents {
                files.insert(format!("{uid}.json"), contents);
            }
        }

        if problems.is_empty() {
            Ok(files)
        } else {
            Err(RenderError::Dashboards(problems))
        }
    }

    /// Renders the set into `dir`, creating it if needed, so that [`check_dir`](Self::check_dir) passes afterwards.
    ///
    /// Orphaned `.json` files are deleted first. A file whose contents already match is left untouched, so its modification time does not change. Every other file is written to a hidden temporary file and renamed into place, so a process watching the directory, such as Grafana's file provisioner, never reads a half-written dashboard. Nothing is written or deleted when the set fails to render.
    pub fn write_dir(&self, dir: impl AsRef<Path>) -> Result<WriteReport, RenderError> {
        let dir = dir.as_ref();
        let files = self.render()?;
        fs::create_dir_all(dir).map_err(|source| io_error(dir, source))?;

        let mut report = WriteReport::default();
        // Orphans go first. On a case-insensitive filesystem an orphan such as `ops.json` can be the same file as the expected `Ops.json`; deleting it before writing leaves the correctly spelled file behind, where deleting after writing would delete the dashboard just written.
        for orphan in orphans(dir, &files)? {
            fs::remove_file(&orphan).map_err(|source| io_error(&orphan, source))?;
            report.removed.push(orphan);
        }
        for (name, contents) in &files {
            let path = dir.join(name);
            if read_existing(&path)?.as_deref() == Some(contents.as_bytes()) {
                report.unchanged.push(path);
                continue;
            }
            write_atomically(dir, name, contents)?;
            report.written.push(path);
        }
        Ok(report)
    }

    /// Fails unless `dir` holds exactly what [`write_dir`](Self::write_dir) would write.
    ///
    /// This is the CI check. It never modifies `dir`. A missing directory counts as every file missing. Rendering problems are reported as [`RenderError::Dashboards`]; a directory that is out of date is reported as [`RenderError::Stale`], listing every stale, missing, and orphaned path.
    pub fn check_dir(&self, dir: impl AsRef<Path>) -> Result<(), RenderError> {
        let dir = dir.as_ref();
        let files = self.render()?;
        let mut report = StaleReport {
            orphaned: orphans(dir, &files)?,
            ..StaleReport::default()
        };
        for (name, contents) in &files {
            let path = dir.join(name);
            match read_existing(&path)? {
                None => report.missing.push(path),
                Some(existing) if existing != contents.as_bytes() => report.stale.push(path),
                Some(_) => {}
            }
        }
        if report.is_empty() {
            Ok(())
        } else {
            Err(RenderError::Stale(report))
        }
    }
}

impl FromIterator<Dashboard> for DashboardSet {
    fn from_iter<I: IntoIterator<Item = Dashboard>>(dashboards: I) -> Self {
        Self::new().dashboards(dashboards)
    }
}

impl Extend<Dashboard> for DashboardSet {
    fn extend<I: IntoIterator<Item = Dashboard>>(&mut self, dashboards: I) {
        self.dashboards.extend(dashboards);
    }
}

/// `.json` files directly inside `dir` that the set would not write, sorted.
fn orphans(dir: &Path, files: &BTreeMap<String, String>) -> Result<Vec<PathBuf>, RenderError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(io_error(dir, source)),
    };
    let expected: Vec<OsString> = files.keys().map(OsString::from).collect();
    let mut orphans = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| io_error(dir, source))?;
        let path = entry.path();
        let is_file = entry
            .file_type()
            .map_err(|source| io_error(&path, source))?
            .is_file();
        let is_json = path
            .extension()
            .is_some_and(|extension| extension == "json");
        if is_file && is_json && !expected.contains(&entry.file_name()) {
            orphans.push(path);
        }
    }
    orphans.sort();
    Ok(orphans)
}

fn read_existing(path: &Path) -> Result<Option<Vec<u8>>, RenderError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(io_error(path, source)),
    }
}

fn write_atomically(dir: &Path, name: &str, contents: &str) -> Result<(), RenderError> {
    let temporary = dir.join(format!(".{name}.tmp"));
    let path = dir.join(name);
    fs::write(&temporary, contents).map_err(|source| io_error(&temporary, source))?;
    fs::rename(&temporary, &path).map_err(|source| {
        // Best effort: the rename error is the one worth reporting.
        let _ = fs::remove_file(&temporary);
        io_error(&path, source)
    })
}

fn io_error(path: &Path, source: io::Error) -> RenderError {
    RenderError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// Why a [`DashboardSet`] could not be rendered, written, or checked.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RenderError {
    /// The set holds no dashboards.
    #[error("the dashboard set is empty; writing it would delete every dashboard in the directory")]
    EmptySet,
    /// One or more dashboards cannot be rendered to a file.
    #[error("{} dashboard problem(s): {}", .0.len(), join(.0))]
    Dashboards(Vec<DashboardProblem>),
    /// The directory does not match what the set renders.
    #[error(transparent)]
    Stale(StaleReport),
    /// A filesystem operation failed.
    #[error("could not access {path}: {source}")]
    Io {
        /// The path being read, written, or listed.
        path: PathBuf,
        /// Underlying filesystem error.
        source: io::Error,
    },
}

/// One reason a dashboard in a [`DashboardSet`] cannot be rendered to a file.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DashboardProblem {
    /// The dashboard has no UID, so it has no file name. Set one with [`Dashboard::uid`].
    #[error("dashboard \"{title}\" has no UID, which names its file")]
    MissingUid {
        /// Dashboard title.
        title: String,
    },
    /// Two dashboards share a UID, compared without regard to ASCII case.
    #[error("dashboards \"{first}\" and \"{second}\" share the UID \"{uid}\"")]
    DuplicateUid {
        /// The UID as the later dashboard spells it.
        uid: String,
        /// Title of the dashboard that claimed the UID first.
        first: String,
        /// Title of the dashboard that repeated it.
        second: String,
    },
    /// The dashboard fails validation or serialization.
    #[error("dashboard \"{title}\": {source}")]
    Invalid {
        /// Dashboard title.
        title: String,
        /// The validation or serialization error.
        source: crate::Error,
    },
}

/// The paths that make a directory differ from what a [`DashboardSet`] renders.
///
/// Each list is sorted. Run [`DashboardSet::write_dir`] to bring the directory up to date.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StaleReport {
    stale: Vec<PathBuf>,
    missing: Vec<PathBuf>,
    orphaned: Vec<PathBuf>,
}

impl StaleReport {
    /// Files that exist but differ from the rendered dashboard.
    #[must_use]
    pub fn stale(&self) -> &[PathBuf] {
        &self.stale
    }

    /// Files a dashboard in the set renders to that do not exist.
    #[must_use]
    pub fn missing(&self) -> &[PathBuf] {
        &self.missing
    }

    /// `.json` files in the directory that no dashboard in the set renders to.
    #[must_use]
    pub fn orphaned(&self) -> &[PathBuf] {
        &self.orphaned
    }

    /// Returns `true` if the directory is up to date.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stale.is_empty() && self.missing.is_empty() && self.orphaned.is_empty()
    }
}

impl fmt::Display for StaleReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "rendered dashboards are out of date ({} stale, {} missing, {} orphaned); re-render them",
            self.stale.len(),
            self.missing.len(),
            self.orphaned.len()
        )?;
        for (label, paths) in [
            ("stale", &self.stale),
            ("missing", &self.missing),
            ("orphaned", &self.orphaned),
        ] {
            for path in paths {
                write!(formatter, "\n  {label}: {}", path.display())?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for StaleReport {}

/// What [`DashboardSet::write_dir`] changed. Each list is in file-name order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WriteReport {
    written: Vec<PathBuf>,
    unchanged: Vec<PathBuf>,
    removed: Vec<PathBuf>,
}

impl WriteReport {
    /// Files created or overwritten.
    #[must_use]
    pub fn written(&self) -> &[PathBuf] {
        &self.written
    }

    /// Files that already matched and were left untouched.
    #[must_use]
    pub fn unchanged(&self) -> &[PathBuf] {
        &self.unchanged
    }

    /// Orphaned `.json` files deleted.
    #[must_use]
    pub fn removed(&self) -> &[PathBuf] {
        &self.removed
    }
}

fn join(problems: &[DashboardProblem]) -> String {
    problems
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}
