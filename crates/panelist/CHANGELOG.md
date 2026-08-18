# Changelog

All notable changes to Panelist will be documented in this file.

## [0.3.0](https://github.com/prisma-risk/panelist/compare/v0.2.1...v0.3.0) - 2026-08-18

### Added

- *(dsl)* reach dashboard links, cursor sync, and variable options ([#13](https://github.com/prisma-risk/panelist/pull/13))
- *(dsl)* reach every typed panel option from the macro DSL ([#11](https://github.com/prisma-risk/panelist/pull/11))

### Fixed

- [**breaking**] select variable kinds explicitly instead of guessing ([#15](https://github.com/prisma-risk/panelist/pull/15))

## [0.2.1](https://github.com/prisma-risk/panelist/compare/v0.2.0...v0.2.1) - 2026-08-18

### Added

- typed transformations, table cells, heatmaps, and DSL parity ([#9](https://github.com/prisma-risk/panelist/pull/9))

## [0.2.0](https://github.com/prisma-risk/panelist/compare/v0.1.0...v0.2.0) - 2026-08-17

### Added

- expand Grafana provisioning model ([#3](https://github.com/prisma-risk/panelist/pull/3))

### Other

- *(release)* protect release-plz changelogs ([#5](https://github.com/prisma-risk/panelist/pull/5))
- add branded source headers ([#2](https://github.com/prisma-risk/panelist/pull/2))

## 0.1.0 - 2026-08-16

- Initial public release.
- Add typed dashboard, panel, query, variable, field, and datasource builders.
- Add `dashboard!`, `promql!`, and `loki!` macros.
- Add deterministic Grafana JSON serialization and validation.
