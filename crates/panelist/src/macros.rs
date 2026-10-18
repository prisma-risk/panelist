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

/// Creates a [`crate::PrometheusQuery`] from a PromQL expression.
#[macro_export]
macro_rules! promql {
    ($expression:expr $(,)?) => {
        $crate::PrometheusQuery::new($expression)
    };
}

/// Creates a [`crate::LokiQuery`] from a LogQL expression.
#[macro_export]
macro_rules! loki_query {
    ($expression:expr $(,)?) => {
        $crate::LokiQuery::new($expression)
    };
}

/// Short alias for [`loki_query!`], sharing a name with the `loki()` datasource
/// helper in Rust's separate macro namespace.
#[macro_export]
macro_rules! loki {
    ($expression:expr $(,)?) => {
        $crate::LokiQuery::new($expression)
    };
}

/// Builds a [`Dashboard`](crate::Dashboard) from a small declarative syntax over the typed builders.
///
/// Every entry lowers to exactly one builder call, so the macro and the builder API build the same authoring model. Values are ordinary Rust expressions, type-checked where the builder receives them. The macro adds no loops or conditions of its own: Rust stays the composition language, and generated panels drop in with `panel:` and `panels:`.
///
/// ```rust
/// use panelist::prelude::*;
///
/// let service = "checkout";
/// let dashboard = dashboard! {
///     title: format!("{service} health");
///     refresh: "30s";
///
///     row "Traffic" {
///         timeseries format!("{service} requests") {
///             query: promql!(format!("sum(rate({service}_requests_total[$__rate_interval]))")) {
///                 legend_format: "{{status}}";
///             }
///             unit: reqps;
///             width: 12;
///         }
///
///         stat "Errors" {
///             query: promql!("sum(rate(errors_total[$__rate_interval]))");
///             thresholds {
///                 base: green;
///                 1 => orange;
///                 5 => red;
///             }
///         }
///     }
/// };
/// dashboard.validate()?;
/// # Ok::<(), panelist::ValidationErrors>(())
/// ```
///
/// # Structure
///
/// The body starts with `title: <expr>;`. Dashboard items follow: `uid`, `description`, `tags`, `refresh`, `time: [<from>, <to>]`, `timezone`, `editable`, `datasource`, `cursor_sync: off | crosshair | tooltip`, `target: v11_5 | v13`, `schema_version`, `panel_ids: positional | stable`, `version`, `link`, `variable`, `row`, `panel`, `panels`, `extra "<key>": <json>`, and panels.
///
/// A row is `row <title> { … }` and holds `key`, `collapsed`, `panel`, `panels`, and panels. A panel is `<kind> <title> { … }`, where the kind is `timeseries`, `stat`, `gauge`, `table`, `text`, `bar_gauge`, or `heatmap`.
///
/// Titles, variable names, link titles and URLs, mapping values, and the arguments of `rename`, `hide`, `order`, and `keep` are expressions. A title runs up to the `{` that opens its block, so wrap one that contains a block of its own in parentheses: `stat (if critical { "Errors!" } else { "Errors" }) { … }`.
///
/// # Generated and house-default panels
///
/// `panel: <expr>;` adds one panel built in Rust and `panels: <expr>;` adds any iterator of them, at dashboard and row level. `<kind> from <expr> { … }` continues an existing builder, so a team's defaults can live in ordinary functions. The base keeps its own title, and writing a title as well is a compile error. A base of the wrong panel kind is a type error at the base expression. Rows accept `row from <expr> { … }` too.
///
/// ```rust
/// use panelist::prelude::*;
///
/// fn house_stat(title: &str) -> Stat {
///     Stat::new(title).graph_mode(StatGraphMode::None).width(4)
/// }
///
/// let services = ["auth", "billing"];
/// let dashboard = dashboard! {
///     title: "Services";
///
///     stat from house_stat("Availability") {
///         query: promql!("avg(up)");
///         unit: percent_unit;
///     }
///
///     row "Per service" {
///         panels: services.iter().map(|service| house_stat(&format!("{service} errors")));
///         panel: Timeseries::new("Latency").unit(Unit::Seconds);
///     }
/// };
/// # let _ = dashboard;
/// ```
///
/// # Panel keys
///
/// Every panel kind accepts `query`, `legend_format`, `description`, `width`, `height`, `id`, `key`, `grid: [<x>, <y>, <width>, <height>]`, `datasource`, `unit`, `min`, `max`, `decimals`, `display_name`, `transparent`, `color`, `thresholds { … }`, `mapping`, `link`, `override`, `transform`, `option "<key>": <json>`, and `extra "<key>": <json>`. The rest apply to some kinds only:
///
/// | Kind | Keys |
/// |---|---|
/// | `timeseries` | `fill_opacity`, `line_width`, `point_size`, `span_nulls`, `line_interpolation`, `show_points`, `stacking`, `tooltip { … }`, `legend { … }` |
/// | `stat` | `color_mode: value \| background \| none`, `graph_mode`, `wide_layout`, `orientation`, `reduce { … }` |
/// | `gauge` | `orientation`, `reduce { … }` |
/// | `bar_gauge` | `display_mode`, `orientation`, `reduce { … }` |
/// | `table` | `sort_by: (<field>, asc \| desc)`, `cell` |
/// | `text` | `content`, `mode: markdown \| html \| code` |
/// | `heatmap` | `color_mode: scheme \| opacity`, `color_scheme`, `color_steps`, `cell_gap`, `show_legend`, `calculate`, `y_axis { unit; placement }` |
///
/// A key that takes a word, such as `unit: seconds;` or `orientation: vertical;`, also takes any expression of the builder argument's type, such as `unit: Unit::custom("USD");` or `orientation: house::ORIENTATION;`. A lone identifier is always read as a word, so wrap a local variable in parentheses: `orientation: (orientation);`.
///
/// # Thresholds
///
/// `thresholds { … }` takes a mandatory `base: <color>;`, the color below the lowest step, and then `<value> => <color>;` steps in ascending order. A color is a word (`green`, `yellow`, `red`, `blue`, `orange`, `purple`), a string naming any other Grafana color such as `"dark-red"` or `"#ff00ff"`, or an expression of type [`Color`](crate::Color). Values are anything convertible to `f64`. `mode: percentage;` switches to percentage thresholds. The original `green: <value>; yellow: …; red: …;` shorthand still works on its own, but cannot be mixed with the general form.
///
/// ```rust
/// use panelist::prelude::*;
///
/// let warning = 70.0;
/// let dashboard = dashboard! {
///     title: "Saturation";
///     gauge "CPU" {
///         unit: percent;
///         thresholds {
///             base: blue;
///             50 => green;
///             warning => orange;
///             90 => "dark-red";
///         }
///     }
/// };
/// # let _ = dashboard;
/// ```
///
/// # Queries
///
/// `query: promql!(<expr>) { … }`, `query: loki!(<expr>) { … }`, and `query: loki_query!(<expr>) { … }` take an options block with `legend_format`, `ref_id`, `instant`, `range`, `datasource`, `hidden`, `interval`, and `editor_mode: builder | code`. `format: time_series | table | heatmap` applies to `promql!` only. Any other query is written `query: <expr>;`.
///
/// # Tables
///
/// `transform` takes `join_by_field(<field>[, outer | inner | outer_tabular])`, `sort_by(<field>, asc | desc)`, `organize { rename <a> => <b>; hide <name>; order <names>; }`, `time_series_to_table { query <ref>[: <reducer>]; time_field <ref>: <field>; }`, and `labels_to_fields [{ mode; keep <labels>; value_label; }]`. Scope any of them with `only ref_id(<expr>)` or `only frame_name(<expr>)`. Any other transformation is written `transform: <expr>;`.
///
/// `override <matcher> { … }` selects fields with `field(<name>)`, `regex(<pattern>)`, `query(<ref id>)`, `type(<field type>)`, `names <names>`, `numeric`, or `time`, and sets `color`, `line_width`, `cell`, `unit`, `min`, `max`, `decimals`, `display_name`, and `thresholds { … }`.
///
/// `cell:` takes `auto`, `colored_text`, `colored_background`, `gauge`, or `sparkline`, and the last three also take an options block: `cell: gauge { mode: lcd; value_display: hidden; };`.
///
/// ```rust
/// use panelist::prelude::*;
///
/// let dashboard = dashboard! {
///     title: "Routes";
///     table "Route performance" {
///         query: promql!("sum by (route) (rate(http_requests_total[5m]))") {
///             ref_id: "A";
///             format: table;
///             instant: true;
///         }
///         transform organize {
///             rename "Value #A" => "RPS";
///             order ["route", "RPS"];
///         }
///         sort_by: ("RPS", desc);
///         override field("RPS") {
///             unit: reqps;
///             cell: colored_background { mode: gradient; };
///         }
///     }
/// };
/// # let _ = dashboard;
/// ```
///
/// # Variables and links
///
/// `variable <name> { … }` picks its kind from its selector: `query`, `plugin`, `value`, or `values`. It also takes `label`, `default`, `datasource`, `multi`, `include_all`, `hidden`, `regex`, `all_value`, `allow_custom_value`, `skip_url_sync`, `sort`, `refresh: never | load | time_range`, and `current <text> => <value> [{ selected: <bool>; }]`. `link <title> => <url> [{ … }]` adds a dashboard or panel link, with `tooltip`, `target_blank`, `include_vars`, `keep_time`, and `tags`.
///
/// # Errors
///
/// Mistakes are reported at the token that caused them. An unknown key names the closest valid one and lists every key accepted in that block. A key that exists but does not apply to the panel kind, such as `fill_opacity` on a `stat`, names the kinds it applies to. A value of the wrong type is a compiler type error at the value.
#[macro_export]
macro_rules! dashboard {
    ($($body:tt)*) => {
        $crate::__private::dashboard! { $crate; $($body)* }
    };
}
