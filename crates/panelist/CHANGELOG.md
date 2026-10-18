# Changelog

All notable changes to Panelist will be documented in this file.

## [0.4.0](https://github.com/prisma-risk/panelist/compare/v0.3.0...v0.4.0) - 2026-10-18

### Added

- [**breaking**] replace the dashboard! tt-muncher with a proc macro ([#35](https://github.com/prisma-risk/panelist/pull/35))
- add a typed read-only dashboard view with Dashboard::resolve ([#36](https://github.com/prisma-risk/panelist/pull/36))
- add PromQL selector and legend helpers ([#30](https://github.com/prisma-risk/panelist/pull/30))
- [**breaking**] type query evaluation, thresholds, colors, units, and per-query field config ([#32](https://github.com/prisma-risk/panelist/pull/32))
- [**breaking**] add panel repeats, query options, value mapping kinds, data links, and annotations ([#34](https://github.com/prisma-risk/panelist/pull/34))
- add KeyedTable for tables joining queries on one label ([#33](https://github.com/prisma-risk/panelist/pull/33))
- [**breaking**] add stable panel IDs, Grafana targets, and a dashboard set renderer ([#31](https://github.com/prisma-risk/panelist/pull/31))
- *(validation)* reject unresolved template variable references ([#29](https://github.com/prisma-risk/panelist/pull/29))

### Fixed

- *(examples)* repair invalid queries and add a rendered gallery ([#17](https://github.com/prisma-risk/panelist/pull/17))

### Other

- improve README readability ([#18](https://github.com/prisma-risk/panelist/pull/18))

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
