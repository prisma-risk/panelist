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

/// Builds a dashboard using a small declarative syntax over the typed API.
///
/// The macro deliberately contains no control flow: use normal Rust functions,
/// iterators, and conditionals to create reusable `Vec<Panel>` fragments, then
/// insert them with `panels: fragment;`.
#[macro_export]
macro_rules! dashboard {
    (title: $title:expr; $($body:tt)*) => {{
        let mut __panelist_dashboard = $crate::Dashboard::new($title);
        $crate::__panelist_dashboard_items!(__panelist_dashboard; $($body)*);
        __panelist_dashboard
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_dashboard_items {
    ($dashboard:ident;) => {};

    ($dashboard:ident; uid: $uid:expr; $($rest:tt)*) => {
        $dashboard = $dashboard.uid($uid);
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };
    ($dashboard:ident; description: $description:expr; $($rest:tt)*) => {
        $dashboard = $dashboard.description($description);
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };
    ($dashboard:ident; tags: [$($tag:expr),* $(,)?]; $($rest:tt)*) => {
        $dashboard = $dashboard.tags([$($tag),*]);
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };
    ($dashboard:ident; refresh: $refresh:expr; $($rest:tt)*) => {
        $dashboard = $dashboard.refresh($refresh);
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };
    ($dashboard:ident; time: [$from:expr, $to:expr $(,)?]; $($rest:tt)*) => {
        $dashboard = $dashboard.time($crate::TimeRange::new($from, $to));
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };
    ($dashboard:ident; timezone: $timezone:expr; $($rest:tt)*) => {
        $dashboard = $dashboard.timezone($timezone);
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };
    ($dashboard:ident; editable: $editable:expr; $($rest:tt)*) => {
        $dashboard = $dashboard.editable($editable);
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };
    ($dashboard:ident; datasource: $datasource:expr; $($rest:tt)*) => {
        $dashboard = $dashboard.datasource($datasource);
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };
    ($dashboard:ident; panels: $panels:expr; $($rest:tt)*) => {
        $dashboard = $dashboard.panels($panels);
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };

    ($dashboard:ident; variable $name:literal { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_variable = $crate::VariableBuilder::new($name);
        $crate::__panelist_variable_items!(__panelist_variable; $($body)*);
        $dashboard = $dashboard.variable(__panelist_variable.build());
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };

    ($dashboard:ident; row $title:literal { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_row = $crate::Row::new($title);
        $crate::__panelist_row_items!(__panelist_row; $($body)*);
        $dashboard = $dashboard.row(__panelist_row);
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };

    ($dashboard:ident; timeseries $title:literal { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_panel = $crate::Timeseries::new($title);
        $crate::__panelist_panel_items!(__panelist_panel; $($body)*);
        $dashboard = $dashboard.panel(__panelist_panel);
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };
    ($dashboard:ident; stat $title:literal { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_panel = $crate::Stat::new($title);
        $crate::__panelist_panel_items!(__panelist_panel; $($body)*);
        $dashboard = $dashboard.panel(__panelist_panel);
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_row_items {
    ($row:ident;) => {};
    ($row:ident; collapsed: $collapsed:expr; $($rest:tt)*) => {
        $row = $row.collapsed($collapsed);
        $crate::__panelist_row_items!($row; $($rest)*);
    };
    ($row:ident; panels: $panels:expr; $($rest:tt)*) => {
        $row = $row.panels($panels);
        $crate::__panelist_row_items!($row; $($rest)*);
    };

    ($row:ident; timeseries $title:literal { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_panel = $crate::Timeseries::new($title);
        $crate::__panelist_panel_items!(__panelist_panel; $($body)*);
        $row = $row.panel(__panelist_panel);
        $crate::__panelist_row_items!($row; $($rest)*);
    };
    ($row:ident; stat $title:literal { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_panel = $crate::Stat::new($title);
        $crate::__panelist_panel_items!(__panelist_panel; $($body)*);
        $row = $row.panel(__panelist_panel);
        $crate::__panelist_row_items!($row; $($rest)*);
    };
    ($row:ident; gauge $title:literal { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_panel = $crate::Gauge::new($title);
        $crate::__panelist_panel_items!(__panelist_panel; $($body)*);
        $row = $row.panel(__panelist_panel);
        $crate::__panelist_row_items!($row; $($rest)*);
    };
    ($row:ident; table $title:literal { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_panel = $crate::Table::new($title);
        $crate::__panelist_panel_items!(__panelist_panel; $($body)*);
        $row = $row.panel(__panelist_panel);
        $crate::__panelist_row_items!($row; $($rest)*);
    };
    ($row:ident; text $title:literal { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_panel = $crate::Text::new($title);
        $crate::__panelist_panel_items!(__panelist_panel; $($body)*);
        $row = $row.panel(__panelist_panel);
        $crate::__panelist_row_items!($row; $($rest)*);
    };
    ($row:ident; bar_gauge $title:literal { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_panel = $crate::BarGauge::new($title);
        $crate::__panelist_panel_items!(__panelist_panel; $($body)*);
        $row = $row.panel(__panelist_panel);
        $crate::__panelist_row_items!($row; $($rest)*);
    };
    ($row:ident; heatmap $title:literal { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_panel = $crate::Heatmap::new($title);
        $crate::__panelist_panel_items!(__panelist_panel; $($body)*);
        $row = $row.panel(__panelist_panel);
        $crate::__panelist_row_items!($row; $($rest)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_panel_items {
    ($panel:ident;) => {};

    ($panel:ident; query: promql!($expression:expr) { $($options:tt)* } $($rest:tt)*) => {
        let mut __panelist_query = $crate::PrometheusQuery::new($expression);
        $crate::__panelist_query_items!(__panelist_query; $($options)*);
        $panel = $panel.query(__panelist_query);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; query: loki!($expression:expr) { $($options:tt)* } $($rest:tt)*) => {
        let mut __panelist_query = $crate::LokiQuery::new($expression);
        $crate::__panelist_query_items!(__panelist_query; $($options)*);
        $panel = $panel.query(__panelist_query);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; query: loki_query!($expression:expr) { $($options:tt)* } $($rest:tt)*) => {
        let mut __panelist_query = $crate::LokiQuery::new($expression);
        $crate::__panelist_query_items!(__panelist_query; $($options)*);
        $panel = $panel.query(__panelist_query);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; query: $query:expr; $($rest:tt)*) => {
        $panel = $panel.query($query);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; legend: $legend:expr; $($rest:tt)*) => {
        $panel = $panel.legend($legend);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; description: $description:expr; $($rest:tt)*) => {
        $panel = $panel.description($description);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; width: $width:expr; $($rest:tt)*) => {
        $panel = $panel.width($width);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; height: $height:expr; $($rest:tt)*) => {
        $panel = $panel.height($height);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; id: $id:expr; $($rest:tt)*) => {
        $panel = $panel.id($id);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; grid: [$x:expr, $y:expr, $width:expr, $height:expr $(,)?]; $($rest:tt)*) => {
        $panel = $panel.grid_pos($crate::GridPos::new($x, $y, $width, $height));
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; datasource: $datasource:expr; $($rest:tt)*) => {
        $panel = $panel.datasource($datasource);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; unit: custom($unit:expr); $($rest:tt)*) => {
        $panel = $panel.unit($crate::Unit::Custom(($unit).into()));
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; unit: $unit:ident; $($rest:tt)*) => {
        $panel = $panel.unit($crate::__panelist_unit!($unit));
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; min: $min:expr; $($rest:tt)*) => {
        $panel = $panel.min($min);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; max: $max:expr; $($rest:tt)*) => {
        $panel = $panel.max($max);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; decimals: $decimals:expr; $($rest:tt)*) => {
        $panel = $panel.decimals($decimals);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; display_name: $display_name:expr; $($rest:tt)*) => {
        $panel = $panel.display_name($display_name);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; transparent: $transparent:expr; $($rest:tt)*) => {
        $panel = $panel.transparent($transparent);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; content: $content:expr; $($rest:tt)*) => {
        $panel = $panel.content($content);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; mode: $mode:ident; $($rest:tt)*) => {
        $panel = $panel.mode($crate::__panelist_text_mode!($mode));
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; thresholds { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_thresholds = $crate::Thresholds::new();
        $crate::__panelist_threshold_items!(__panelist_thresholds; $($body)*);
        $panel = $panel.thresholds(__panelist_thresholds);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; legend { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_legend = $crate::Legend::new();
        $crate::__panelist_legend_items!(__panelist_legend; $($body)*);
        $panel = $panel.legend_options(__panelist_legend);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; override field($name:expr) { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_override = $crate::FieldOverride::by_name($name);
        $crate::__panelist_override_items!(__panelist_override; $($body)*);
        $panel = $panel.override_field(__panelist_override);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; override regex($regex:expr) { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_override = $crate::FieldOverride::by_regex($regex);
        $crate::__panelist_override_items!(__panelist_override; $($body)*);
        $panel = $panel.override_field(__panelist_override);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; option $key:literal: $value:expr; $($rest:tt)*) => {
        $panel = $panel.option($key, $crate::__private::serde_json::json!($value));
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; extra $key:literal: $value:expr; $($rest:tt)*) => {
        $panel = $panel.extra($key, $crate::__private::serde_json::json!($value));
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_query_items {
    ($query:ident;) => {};
    ($query:ident; legend: $legend:expr; $($rest:tt)*) => {
        $query = $query.legend($legend);
        $crate::__panelist_query_items!($query; $($rest)*);
    };
    ($query:ident; ref_id: $ref_id:expr; $($rest:tt)*) => {
        $query = $query.ref_id($ref_id);
        $crate::__panelist_query_items!($query; $($rest)*);
    };
    ($query:ident; instant: $instant:expr; $($rest:tt)*) => {
        $query = $query.instant($instant);
        $crate::__panelist_query_items!($query; $($rest)*);
    };
    ($query:ident; range: $range:expr; $($rest:tt)*) => {
        $query = $query.range($range);
        $crate::__panelist_query_items!($query; $($rest)*);
    };
    ($query:ident; datasource: $datasource:expr; $($rest:tt)*) => {
        $query = $query.datasource($datasource);
        $crate::__panelist_query_items!($query; $($rest)*);
    };
    ($query:ident; hidden: $hidden:expr; $($rest:tt)*) => {
        $query = $query.hidden($hidden);
        $crate::__panelist_query_items!($query; $($rest)*);
    };
    ($query:ident; interval: $interval:expr; $($rest:tt)*) => {
        $query = $query.interval($interval);
        $crate::__panelist_query_items!($query; $($rest)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_threshold_items {
    ($thresholds:ident;) => {};
    ($thresholds:ident; green: $value:expr; $($rest:tt)*) => {
        $thresholds = $thresholds.green($value);
        $crate::__panelist_threshold_items!($thresholds; $($rest)*);
    };
    ($thresholds:ident; yellow: $value:expr; $($rest:tt)*) => {
        $thresholds = $thresholds.yellow($value);
        $crate::__panelist_threshold_items!($thresholds; $($rest)*);
    };
    ($thresholds:ident; red: $value:expr; $($rest:tt)*) => {
        $thresholds = $thresholds.red($value);
        $crate::__panelist_threshold_items!($thresholds; $($rest)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_legend_items {
    ($legend:ident;) => {};
    ($legend:ident; position: bottom; $($rest:tt)*) => {
        $legend = $legend.placement($crate::LegendPlacement::Bottom);
        $crate::__panelist_legend_items!($legend; $($rest)*);
    };
    ($legend:ident; position: right; $($rest:tt)*) => {
        $legend = $legend.placement($crate::LegendPlacement::Right);
        $crate::__panelist_legend_items!($legend; $($rest)*);
    };
    ($legend:ident; mode: list; $($rest:tt)*) => {
        $legend = $legend.mode($crate::LegendMode::List);
        $crate::__panelist_legend_items!($legend; $($rest)*);
    };
    ($legend:ident; mode: table; $($rest:tt)*) => {
        $legend = $legend.mode($crate::LegendMode::Table);
        $crate::__panelist_legend_items!($legend; $($rest)*);
    };
    ($legend:ident; mode: hidden; $($rest:tt)*) => {
        $legend = $legend.mode($crate::LegendMode::Hidden);
        $crate::__panelist_legend_items!($legend; $($rest)*);
    };
    ($legend:ident; values: [$($value:ident),* $(,)?]; $($rest:tt)*) => {
        $legend = $legend.calculations([$($crate::__panelist_reducer!($value)),*]);
        $crate::__panelist_legend_items!($legend; $($rest)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_override_items {
    ($field:ident;) => {};
    ($field:ident; color: $color:literal; $($rest:tt)*) => {
        $field = $field.color($crate::Color::Custom(($color).into()));
        $crate::__panelist_override_items!($field; $($rest)*);
    };
    ($field:ident; color: $color:ident; $($rest:tt)*) => {
        $field = $field.color($crate::__panelist_color!($color));
        $crate::__panelist_override_items!($field; $($rest)*);
    };
    ($field:ident; line_width: $width:expr; $($rest:tt)*) => {
        $field = $field.line_width($width);
        $crate::__panelist_override_items!($field; $($rest)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_variable_items {
    ($variable:ident;) => {};
    ($variable:ident; query: $query:expr; $($rest:tt)*) => {
        $variable = $variable.query($query);
        $crate::__panelist_variable_items!($variable; $($rest)*);
    };
    ($variable:ident; values: [$($value:expr),* $(,)?]; $($rest:tt)*) => {
        $variable = $variable.values([$($value),*]);
        $crate::__panelist_variable_items!($variable; $($rest)*);
    };
    ($variable:ident; value: $value:expr; $($rest:tt)*) => {
        $variable = $variable.value($value);
        $crate::__panelist_variable_items!($variable; $($rest)*);
    };
    ($variable:ident; label: $label:expr; $($rest:tt)*) => {
        $variable = $variable.label($label);
        $crate::__panelist_variable_items!($variable; $($rest)*);
    };
    ($variable:ident; default: $default:expr; $($rest:tt)*) => {
        $variable = $variable.default($default);
        $crate::__panelist_variable_items!($variable; $($rest)*);
    };
    ($variable:ident; datasource: $datasource:expr; $($rest:tt)*) => {
        $variable = $variable.datasource($datasource);
        $crate::__panelist_variable_items!($variable; $($rest)*);
    };
    ($variable:ident; multi: $multi:expr; $($rest:tt)*) => {
        $variable = $variable.multi($multi);
        $crate::__panelist_variable_items!($variable; $($rest)*);
    };
    ($variable:ident; include_all: $include_all:expr; $($rest:tt)*) => {
        $variable = $variable.include_all($include_all);
        $crate::__panelist_variable_items!($variable; $($rest)*);
    };
    ($variable:ident; hidden: $hidden:expr; $($rest:tt)*) => {
        $variable = $variable.hidden($hidden);
        $crate::__panelist_variable_items!($variable; $($rest)*);
    };
    ($variable:ident; refresh: never; $($rest:tt)*) => {
        $variable = $variable.refresh($crate::VariableRefresh::Never);
        $crate::__panelist_variable_items!($variable; $($rest)*);
    };
    ($variable:ident; refresh: load; $($rest:tt)*) => {
        $variable = $variable.refresh($crate::VariableRefresh::OnDashboardLoad);
        $crate::__panelist_variable_items!($variable; $($rest)*);
    };
    ($variable:ident; refresh: time_range; $($rest:tt)*) => {
        $variable = $variable.refresh($crate::VariableRefresh::OnTimeRangeChange);
        $crate::__panelist_variable_items!($variable; $($rest)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_unit {
    (seconds) => {
        $crate::Unit::Seconds
    };
    (milliseconds) => {
        $crate::Unit::Milliseconds
    };
    (bytes) => {
        $crate::Unit::Bytes
    };
    (bytes_per_second) => {
        $crate::Unit::BytesPerSecond
    };
    (percent) => {
        $crate::Unit::Percent
    };
    (reqps) => {
        $crate::Unit::RequestsPerSecond
    };
    (ops) => {
        $crate::Unit::OperationsPerSecond
    };
    (short) => {
        $crate::Unit::Short
    };
    (none) => {
        $crate::Unit::None
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_text_mode {
    (markdown) => {
        $crate::TextMode::Markdown
    };
    (html) => {
        $crate::TextMode::Html
    };
    (code) => {
        $crate::TextMode::Code
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_color {
    (green) => {
        $crate::Color::Green
    };
    (yellow) => {
        $crate::Color::Yellow
    };
    (red) => {
        $crate::Color::Red
    };
    (blue) => {
        $crate::Color::Blue
    };
    (orange) => {
        $crate::Color::Orange
    };
    (purple) => {
        $crate::Color::Purple
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_reducer {
    (last) => {
        $crate::Reducer::Last
    };
    (min) => {
        $crate::Reducer::Min
    };
    (max) => {
        $crate::Reducer::Max
    };
    (mean) => {
        $crate::Reducer::Mean
    };
    (total) => {
        $crate::Reducer::Total
    };
}
