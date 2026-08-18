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
///
/// The panel body grammar (`legend { … }`, `cell:`, `sort_by:`, the heatmap
/// options, and so on) is one shared rule set across every panel kind, and
/// `format:` inside a query block is one shared rule set across every query
/// kind. Rules that only make sense for one concrete type — for example
/// `format:` outside `promql!(…) { … }`, or the heatmap rules on a panel kind
/// other than `heatmap`— are still syntactically accepted, and only rejected
/// when the expansion calls a builder method that type doesn't have. Such
/// mistakes surface as a compile error pointing into the macro expansion
/// rather than at the offending line in your `dashboard!` block.
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
    ($dashboard:ident; gauge $title:literal { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_panel = $crate::Gauge::new($title);
        $crate::__panelist_panel_items!(__panelist_panel; $($body)*);
        $dashboard = $dashboard.panel(__panelist_panel);
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };
    ($dashboard:ident; table $title:literal { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_panel = $crate::Table::new($title);
        $crate::__panelist_panel_items!(__panelist_panel; $($body)*);
        $dashboard = $dashboard.panel(__panelist_panel);
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };
    ($dashboard:ident; text $title:literal { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_panel = $crate::Text::new($title);
        $crate::__panelist_panel_items!(__panelist_panel; $($body)*);
        $dashboard = $dashboard.panel(__panelist_panel);
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };
    ($dashboard:ident; bar_gauge $title:literal { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_panel = $crate::BarGauge::new($title);
        $crate::__panelist_panel_items!(__panelist_panel; $($body)*);
        $dashboard = $dashboard.panel(__panelist_panel);
        $crate::__panelist_dashboard_items!($dashboard; $($rest)*);
    };
    ($dashboard:ident; heatmap $title:literal { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_panel = $crate::Heatmap::new($title);
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

    ($panel:ident; transform join_by_field($field:expr, $mode:ident) only ref_id($ref:expr); $($rest:tt)*) => {
        $panel = $panel.transform(
            $crate::JoinByField::new($field)
                .mode($crate::__panelist_join_mode!($mode))
                .only_ref_id($ref),
        );
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; transform join_by_field($field:expr, $mode:ident); $($rest:tt)*) => {
        $panel = $panel.transform(
            $crate::JoinByField::new($field).mode($crate::__panelist_join_mode!($mode)),
        );
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; transform join_by_field($field:expr) only ref_id($ref:expr); $($rest:tt)*) => {
        $panel = $panel.transform($crate::JoinByField::new($field).only_ref_id($ref));
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; transform join_by_field($field:expr); $($rest:tt)*) => {
        $panel = $panel.transform($crate::JoinByField::new($field));
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };

    ($panel:ident; transform sort_by($field:expr, $direction:ident) only ref_id($ref:expr); $($rest:tt)*) => {
        $panel = $panel.transform(
            $crate::SortBy::new($field, $crate::__panelist_sort_direction!($direction))
                .only_ref_id($ref),
        );
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; transform sort_by($field:expr, $direction:ident); $($rest:tt)*) => {
        $panel = $panel.transform(
            $crate::SortBy::new($field, $crate::__panelist_sort_direction!($direction)),
        );
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };

    ($panel:ident; transform organize { $($body:tt)* } only ref_id($ref:expr); $($rest:tt)*) => {
        let mut __panelist_organize = $crate::OrganizeFields::new();
        $crate::__panelist_organize_items!(__panelist_organize; $($body)*);
        $panel = $panel.transform(__panelist_organize.only_ref_id($ref));
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; transform organize { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_organize = $crate::OrganizeFields::new();
        $crate::__panelist_organize_items!(__panelist_organize; $($body)*);
        $panel = $panel.transform(__panelist_organize);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };

    ($panel:ident; transform time_series_to_table { $($body:tt)* } only ref_id($ref:expr); $($rest:tt)*) => {
        let mut __panelist_convert = $crate::TimeSeriesToTable::new();
        $crate::__panelist_time_series_table_items!(__panelist_convert; $($body)*);
        $panel = $panel.transform(__panelist_convert.only_ref_id($ref));
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; transform time_series_to_table { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_convert = $crate::TimeSeriesToTable::new();
        $crate::__panelist_time_series_table_items!(__panelist_convert; $($body)*);
        $panel = $panel.transform(__panelist_convert);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };

    ($panel:ident; transform labels_to_fields { $($body:tt)* } only ref_id($ref:expr); $($rest:tt)*) => {
        let mut __panelist_labels = $crate::LabelsToFields::new();
        $crate::__panelist_labels_items!(__panelist_labels; $($body)*);
        $panel = $panel.transform(__panelist_labels.only_ref_id($ref));
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; transform labels_to_fields { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_labels = $crate::LabelsToFields::new();
        $crate::__panelist_labels_items!(__panelist_labels; $($body)*);
        $panel = $panel.transform(__panelist_labels);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; transform labels_to_fields; $($rest:tt)*) => {
        $panel = $panel.transform($crate::LabelsToFields::new());
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };

    ($panel:ident; transform: $transformation:expr; $($rest:tt)*) => {
        $panel = $panel.transform($transformation);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };

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
    ($panel:ident; override query($ref_id:expr) { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_override = $crate::FieldOverride::by_query($ref_id);
        $crate::__panelist_override_items!(__panelist_override; $($body)*);
        $panel = $panel.override_field(__panelist_override);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; override type($field_type:expr) { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_override = $crate::FieldOverride::by_type($field_type);
        $crate::__panelist_override_items!(__panelist_override; $($body)*);
        $panel = $panel.override_field(__panelist_override);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; override names [$($name:expr),* $(,)?] { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_override = $crate::FieldOverride::by_names([$($name),*]);
        $crate::__panelist_override_items!(__panelist_override; $($body)*);
        $panel = $panel.override_field(__panelist_override);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; override numeric { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_override = $crate::FieldOverride::numeric_fields();
        $crate::__panelist_override_items!(__panelist_override; $($body)*);
        $panel = $panel.override_field(__panelist_override);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; override time { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_override = $crate::FieldOverride::time_fields();
        $crate::__panelist_override_items!(__panelist_override; $($body)*);
        $panel = $panel.override_field(__panelist_override);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; sort_by: ($field:expr, $direction:ident); $($rest:tt)*) => {
        $panel = $panel.sort_by($field, $crate::__panelist_sort_direction!($direction));
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; cell: $cell:ident; $($rest:tt)*) => {
        $panel = $panel.cell($crate::__panelist_cell_type!($cell));
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; stacking: $mode:ident; $($rest:tt)*) => {
        $panel = $panel.stacking($crate::Stacking::new($crate::__panelist_stacking_mode!($mode)));
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; color_scheme: $scheme:expr; $($rest:tt)*) => {
        $panel = $panel.color_scheme($scheme);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; color_steps: $steps:expr; $($rest:tt)*) => {
        $panel = $panel.color_steps($steps);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; color_mode: scheme; $($rest:tt)*) => {
        $panel = $panel.color_mode($crate::HeatmapColorMode::Scheme);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; color_mode: opacity; $($rest:tt)*) => {
        $panel = $panel.color_mode($crate::HeatmapColorMode::Opacity);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; cell_gap: $gap:expr; $($rest:tt)*) => {
        $panel = $panel.cell_gap($gap);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; show_legend: $show:expr; $($rest:tt)*) => {
        $panel = $panel.show_legend($show);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; calculate: $calculate:expr; $($rest:tt)*) => {
        $panel = $panel.calculate($calculate);
        $crate::__panelist_panel_items!($panel; $($rest)*);
    };
    ($panel:ident; y_axis { $($body:tt)* } $($rest:tt)*) => {
        $crate::__panelist_y_axis_items!($panel; $($body)*);
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
macro_rules! __panelist_y_axis_items {
    ($panel:ident;) => {};
    ($panel:ident; unit: custom($unit:expr); $($rest:tt)*) => {
        $panel = $panel.y_axis_unit($crate::Unit::Custom(($unit).into()));
        $crate::__panelist_y_axis_items!($panel; $($rest)*);
    };
    ($panel:ident; unit: $unit:ident; $($rest:tt)*) => {
        $panel = $panel.y_axis_unit($crate::__panelist_unit!($unit));
        $crate::__panelist_y_axis_items!($panel; $($rest)*);
    };
    ($panel:ident; placement: $placement:ident; $($rest:tt)*) => {
        $panel = $panel.y_axis_placement($crate::__panelist_axis_placement!($placement));
        $crate::__panelist_y_axis_items!($panel; $($rest)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_axis_placement {
    (auto) => {
        $crate::AxisPlacement::Auto
    };
    (bottom) => {
        $crate::AxisPlacement::Bottom
    };
    (hidden) => {
        $crate::AxisPlacement::Hidden
    };
    (left) => {
        $crate::AxisPlacement::Left
    };
    (right) => {
        $crate::AxisPlacement::Right
    };
    (top) => {
        $crate::AxisPlacement::Top
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
    ($query:ident; format: time_series; $($rest:tt)*) => {
        $query = $query.format($crate::PrometheusFormat::TimeSeries);
        $crate::__panelist_query_items!($query; $($rest)*);
    };
    ($query:ident; format: table; $($rest:tt)*) => {
        $query = $query.format($crate::PrometheusFormat::Table);
        $crate::__panelist_query_items!($query; $($rest)*);
    };
    ($query:ident; format: heatmap; $($rest:tt)*) => {
        $query = $query.format($crate::PrometheusFormat::Heatmap);
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
macro_rules! __panelist_cell_type {
    (auto) => {
        $crate::TableCellType::Auto
    };
    (colored_text) => {
        $crate::TableCellType::ColoredText
    };
    (colored_background) => {
        $crate::TableCellType::ColoredBackground
    };
    (gauge) => {
        $crate::TableCellType::Gauge
    };
    (sparkline) => {
        $crate::TableCellType::Sparkline
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_sparkline_items {
    ($cell:ident;) => {};
    ($cell:ident; hide_value: $hide:expr; $($rest:tt)*) => {
        $cell = $cell.hide_value($hide);
        $crate::__panelist_sparkline_items!($cell; $($rest)*);
    };
    ($cell:ident; line_width: $width:expr; $($rest:tt)*) => {
        $cell = $cell.line_width($width);
        $crate::__panelist_sparkline_items!($cell; $($rest)*);
    };
    ($cell:ident; fill_opacity: $opacity:expr; $($rest:tt)*) => {
        $cell = $cell.fill_opacity($opacity);
        $crate::__panelist_sparkline_items!($cell; $($rest)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_background_items {
    ($cell:ident;) => {};
    ($cell:ident; mode: basic; $($rest:tt)*) => {
        $cell = $cell.mode($crate::CellBackgroundMode::Basic);
        $crate::__panelist_background_items!($cell; $($rest)*);
    };
    ($cell:ident; mode: gradient; $($rest:tt)*) => {
        $cell = $cell.mode($crate::CellBackgroundMode::Gradient);
        $crate::__panelist_background_items!($cell; $($rest)*);
    };
    ($cell:ident; apply_to_row: $apply:expr; $($rest:tt)*) => {
        $cell = $cell.apply_to_row($apply);
        $crate::__panelist_background_items!($cell; $($rest)*);
    };
    ($cell:ident; wrap_text: $wrap:expr; $($rest:tt)*) => {
        $cell = $cell.wrap_text($wrap);
        $crate::__panelist_background_items!($cell; $($rest)*);
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
    ($field:ident; cell: sparkline { $($body:tt)* }; $($rest:tt)*) => {
        let mut __panelist_cell = $crate::SparklineCell::new();
        $crate::__panelist_sparkline_items!(__panelist_cell; $($body)*);
        $field = $field.cell(__panelist_cell);
        $crate::__panelist_override_items!($field; $($rest)*);
    };
    ($field:ident; cell: colored_background { $($body:tt)* }; $($rest:tt)*) => {
        let mut __panelist_cell = $crate::ColoredBackgroundCell::new();
        $crate::__panelist_background_items!(__panelist_cell; $($body)*);
        $field = $field.cell(__panelist_cell);
        $crate::__panelist_override_items!($field; $($rest)*);
    };
    ($field:ident; cell: $cell:ident; $($rest:tt)*) => {
        $field = $field.cell_type($crate::__panelist_cell_type!($cell));
        $crate::__panelist_override_items!($field; $($rest)*);
    };
    ($field:ident; unit: custom($unit:expr); $($rest:tt)*) => {
        $field = $field.unit($crate::Unit::Custom(($unit).into()));
        $crate::__panelist_override_items!($field; $($rest)*);
    };
    ($field:ident; unit: $unit:ident; $($rest:tt)*) => {
        $field = $field.unit($crate::__panelist_unit!($unit));
        $crate::__panelist_override_items!($field; $($rest)*);
    };
    ($field:ident; min: $min:expr; $($rest:tt)*) => {
        $field = $field.min($min);
        $crate::__panelist_override_items!($field; $($rest)*);
    };
    ($field:ident; max: $max:expr; $($rest:tt)*) => {
        $field = $field.max($max);
        $crate::__panelist_override_items!($field; $($rest)*);
    };
    ($field:ident; decimals: $decimals:expr; $($rest:tt)*) => {
        $field = $field.decimals($decimals);
        $crate::__panelist_override_items!($field; $($rest)*);
    };
    ($field:ident; display_name: $name:expr; $($rest:tt)*) => {
        $field = $field.display_name($name);
        $crate::__panelist_override_items!($field; $($rest)*);
    };
    ($field:ident; thresholds { $($body:tt)* } $($rest:tt)*) => {
        let mut __panelist_thresholds = $crate::Thresholds::new();
        $crate::__panelist_threshold_items!(__panelist_thresholds; $($body)*);
        $field = $field.thresholds(__panelist_thresholds);
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
    (percent_unit) => {
        $crate::Unit::PercentUnit
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

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_stacking_mode {
    (none) => {
        $crate::StackingMode::None
    };
    (normal) => {
        $crate::StackingMode::Normal
    };
    (percent) => {
        $crate::StackingMode::Percent
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_join_mode {
    (outer) => {
        $crate::JoinMode::Outer
    };
    (inner) => {
        $crate::JoinMode::Inner
    };
    (outer_tabular) => {
        $crate::JoinMode::OuterTabular
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_sort_direction {
    (asc) => {
        $crate::SortDirection::Ascending
    };
    (desc) => {
        $crate::SortDirection::Descending
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_labels_mode {
    (columns) => {
        $crate::LabelsToFieldsMode::Columns
    };
    (rows) => {
        $crate::LabelsToFieldsMode::Rows
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_organize_items {
    ($organize:ident;) => {};
    ($organize:ident; rename $from:literal => $to:literal; $($rest:tt)*) => {
        $organize = $organize.rename($from, $to);
        $crate::__panelist_organize_items!($organize; $($rest)*);
    };
    ($organize:ident; hide $name:literal; $($rest:tt)*) => {
        $organize = $organize.hide($name);
        $crate::__panelist_organize_items!($organize; $($rest)*);
    };
    ($organize:ident; order [$($field:literal),* $(,)?]; $($rest:tt)*) => {
        $organize = $organize.order([$($field),*]);
        $crate::__panelist_organize_items!($organize; $($rest)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_time_series_table_items {
    ($convert:ident;) => {};
    ($convert:ident; query $ref_id:literal: $stat:ident; $($rest:tt)*) => {
        $convert = $convert.query_with($ref_id, $crate::__panelist_reducer!($stat));
        $crate::__panelist_time_series_table_items!($convert; $($rest)*);
    };
    ($convert:ident; query $ref_id:literal; $($rest:tt)*) => {
        $convert = $convert.query($ref_id);
        $crate::__panelist_time_series_table_items!($convert; $($rest)*);
    };
    ($convert:ident; time_field $ref_id:literal: $field:literal; $($rest:tt)*) => {
        $convert = $convert.time_field($ref_id, $field);
        $crate::__panelist_time_series_table_items!($convert; $($rest)*);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __panelist_labels_items {
    ($labels:ident;) => {};
    ($labels:ident; mode: $mode:ident; $($rest:tt)*) => {
        $labels = $labels.mode($crate::__panelist_labels_mode!($mode));
        $crate::__panelist_labels_items!($labels; $($rest)*);
    };
    ($labels:ident; keep [$($label:literal),* $(,)?]; $($rest:tt)*) => {
        $labels = $labels.keep([$($label),*]);
        $crate::__panelist_labels_items!($labels; $($rest)*);
    };
    ($labels:ident; value_label: $label:literal; $($rest:tt)*) => {
        $labels = $labels.value_label($label);
        $crate::__panelist_labels_items!($labels; $($rest)*);
    };
}
