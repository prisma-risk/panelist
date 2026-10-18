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

//! The grammar table: every block the DSL accepts, every key inside it, the
//! panel kinds each key applies to, and the builder call it lowers to.
//!
//! This is the one place that knows which keys exist. Parsing, lowering,
//! "did you mean" suggestions, and the wrong-panel-kind diagnostics are all
//! driven from it, so adding DSL syntax for a new builder method is usually
//! one `Key` line in the right `Context`. Names on the right-hand side are
//! Panelist builder methods and Rust type or variant names, never Grafana
//! wire names.

/// A set of kinds within one context, as a bit per index into
/// [`Context::kinds`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Kinds(u16);

impl Kinds {
    /// Every kind of the context, or the only one when it has no kinds.
    pub(crate) const ALL: Self = Self(u16::MAX);

    const fn bit(index: u16) -> Self {
        Self(1 << index)
    }

    /// The union of two sets.
    pub(crate) const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether the kind at `index` is in the set.
    pub(crate) const fn contains(self, index: usize) -> bool {
        index < 16 && self.0 & (1 << index) != 0
    }
}

/// One kind of a context, such as the `stat` panel or the `loki!` query.
#[derive(Debug)]
pub(crate) struct Kind {
    /// The DSL spelling, used to introduce the kind and in messages.
    pub(crate) keyword: &'static str,
    /// The Panelist builder type the kind constructs.
    pub(crate) ty: &'static str,
}

const TIMESERIES: Kinds = Kinds::bit(0);
const STAT: Kinds = Kinds::bit(1);
const GAUGE: Kinds = Kinds::bit(2);
const TABLE: Kinds = Kinds::bit(3);
const TEXT: Kinds = Kinds::bit(4);
const BAR_GAUGE: Kinds = Kinds::bit(5);
const HEATMAP: Kinds = Kinds::bit(6);
const KEYED_TABLE: Kinds = Kinds::bit(7);
/// Tables and keyed tables, which start from the same table builder.
const TABLES: Kinds = TABLE.or(KEYED_TABLE);

/// Panel kinds, in the bit order of the constants above. A new panel builder
/// is a new entry here plus a new bit.
pub(crate) static PANEL_KINDS: &[Kind] = &[
    Kind {
        keyword: "timeseries",
        ty: "Timeseries",
    },
    Kind {
        keyword: "stat",
        ty: "Stat",
    },
    Kind {
        keyword: "gauge",
        ty: "Gauge",
    },
    Kind {
        keyword: "table",
        ty: "Table",
    },
    Kind {
        keyword: "text",
        ty: "Text",
    },
    Kind {
        keyword: "bar_gauge",
        ty: "BarGauge",
    },
    Kind {
        keyword: "heatmap",
        ty: "Heatmap",
    },
    Kind {
        keyword: "keyed_table",
        ty: "Table",
    },
];

/// The panel kind written `keyed_table <title> by <key> { … }`, whose
/// table builder becomes a `KeyedTable` through `.keyed(key)`.
pub(crate) const KEYED_TABLE_INDEX: usize = 7;

const PROMQL: Kinds = Kinds::bit(0);

/// Query kinds that have a block form, in bit order.
pub(crate) static QUERY_KINDS: &[Kind] = &[
    Kind {
        keyword: "promql!",
        ty: "PrometheusQuery",
    },
    Kind {
        keyword: "loki!",
        ty: "LokiQuery",
    },
];

/// The query macros a `query:` value may start with, and the index into
/// [`QUERY_KINDS`] each one builds.
pub(crate) static QUERY_MACROS: &[(&str, usize)] = &[("promql", 0), ("loki", 1), ("loki_query", 1)];

/// A closed set of words that lower to the variants of one Rust enum.
#[derive(Debug)]
pub(crate) struct Vocab {
    /// What a word names, for messages: "unit", "tooltip mode".
    pub(crate) what: &'static str,
    /// The Panelist enum the words select a variant of.
    pub(crate) ty: &'static str,
    /// `(word, variant)` pairs.
    pub(crate) words: &'static [(&'static str, &'static str)],
}

impl Vocab {
    /// The variant a word selects.
    pub(crate) fn variant(&self, word: &str) -> Option<&'static str> {
        self.words
            .iter()
            .find(|(candidate, _)| *candidate == word)
            .map(|(_, variant)| *variant)
    }
}

macro_rules! vocab {
    ($name:ident, $what:literal, $ty:literal, [$($word:literal => $variant:literal),+ $(,)?]) => {
        pub(crate) static $name: Vocab = Vocab {
            what: $what,
            ty: $ty,
            words: &[$(($word, $variant)),+],
        };
    };
}

vocab!(UNIT, "unit", "Unit", [
    "nanoseconds" => "Nanoseconds",
    "microseconds" => "Microseconds",
    "milliseconds" => "Milliseconds",
    "seconds" => "Seconds",
    "minutes" => "Minutes",
    "hours" => "Hours",
    "days" => "Days",
    "duration_seconds" => "DurationSeconds",
    "duration_milliseconds" => "DurationMilliseconds",
    "bytes" => "Bytes",
    "bytes_per_second" => "BytesPerSecond",
    "bits_per_second" => "BitsPerSecond",
    "packets_per_second" => "PacketsPerSecond",
    "percent" => "Percent",
    "percent_unit" => "PercentUnit",
    "reqps" => "RequestsPerSecond",
    "ops" => "OperationsPerSecond",
    "reads_per_second" => "ReadsPerSecond",
    "writes_per_second" => "WritesPerSecond",
    "iops" => "IoOperationsPerSecond",
    "events_per_second" => "EventsPerSecond",
    "short" => "Short",
    "none" => "None",
]);
vocab!(COLOR, "color", "Color", [
    "green" => "Green",
    "yellow" => "Yellow",
    "red" => "Red",
    "blue" => "Blue",
    "orange" => "Orange",
    "purple" => "Purple",
]);
vocab!(REDUCER, "reducer", "Reducer", [
    "last" => "Last",
    "min" => "Min",
    "max" => "Max",
    "mean" => "Mean",
    "total" => "Total",
]);
vocab!(TEXT_MODE, "text mode", "TextMode", [
    "markdown" => "Markdown",
    "html" => "Html",
    "code" => "Code",
]);
vocab!(STAT_COLOR_MODE, "stat color mode", "StatColorMode", [
    "value" => "Value",
    "background" => "Background",
    "none" => "None",
]);
vocab!(HEATMAP_COLOR_MODE, "heatmap color mode", "HeatmapColorMode", [
    "scheme" => "Scheme",
    "opacity" => "Opacity",
]);
vocab!(STAT_GRAPH_MODE, "graph mode", "StatGraphMode", [
    "area" => "Area",
    "none" => "None",
]);
vocab!(ORIENTATION, "orientation", "Orientation", [
    "auto" => "Auto",
    "horizontal" => "Horizontal",
    "vertical" => "Vertical",
]);
vocab!(BAR_GAUGE_DISPLAY_MODE, "display mode", "BarGaugeDisplayMode", [
    "basic" => "Basic",
    "gradient" => "Gradient",
    "lcd" => "Lcd",
]);
vocab!(LINE_INTERPOLATION, "line interpolation", "LineInterpolation", [
    "linear" => "Linear",
    "smooth" => "Smooth",
    "step_before" => "StepBefore",
    "step_after" => "StepAfter",
]);
vocab!(POINT_VISIBILITY, "point visibility", "PointVisibility", [
    "auto" => "Auto",
    "always" => "Always",
    "never" => "Never",
]);
vocab!(STACKING_MODE, "stacking mode", "StackingMode", [
    "none" => "None",
    "normal" => "Normal",
    "percent" => "Percent",
]);
vocab!(TOOLTIP_MODE, "tooltip mode", "TooltipMode", [
    "single" => "Single",
    "multi" => "Multi",
    "none" => "None",
]);
vocab!(TOOLTIP_SORT, "tooltip sort", "TooltipSort", [
    "none" => "None",
    "asc" => "Ascending",
    "desc" => "Descending",
]);
vocab!(LEGEND_PLACEMENT, "legend position", "LegendPlacement", [
    "bottom" => "Bottom",
    "right" => "Right",
]);
vocab!(LEGEND_MODE, "legend mode", "LegendMode", [
    "list" => "List",
    "table" => "Table",
    "hidden" => "Hidden",
]);
vocab!(CELL_TYPE, "cell type", "TableCellType", [
    "auto" => "Auto",
    "colored_text" => "ColoredText",
    "colored_background" => "ColoredBackground",
    "gauge" => "Gauge",
    "sparkline" => "Sparkline",
]);
vocab!(CELL_BACKGROUND_MODE, "background mode", "CellBackgroundMode", [
    "basic" => "Basic",
    "gradient" => "Gradient",
]);
vocab!(CELL_VALUE_DISPLAY, "value display", "CellValueDisplay", [
    "text" => "Text",
    "color" => "Color",
    "hidden" => "Hidden",
]);
vocab!(AXIS_PLACEMENT, "axis placement", "AxisPlacement", [
    "auto" => "Auto",
    "bottom" => "Bottom",
    "hidden" => "Hidden",
    "left" => "Left",
    "right" => "Right",
    "top" => "Top",
]);
vocab!(PROMETHEUS_FORMAT, "query format", "PrometheusFormat", [
    "time_series" => "TimeSeries",
    "table" => "Table",
    "heatmap" => "Heatmap",
]);
vocab!(QUERY_EDITOR_MODE, "editor mode", "QueryEditorMode", [
    "builder" => "Builder",
    "code" => "Code",
]);
vocab!(CURSOR_SYNC, "cursor sync", "DashboardCursorSync", [
    "off" => "Off",
    "crosshair" => "Crosshair",
    "tooltip" => "Tooltip",
]);
vocab!(GRAFANA_TARGET, "Grafana target", "GrafanaTarget", [
    "v11_5" => "V11_5",
    "v13" => "V13",
]);
vocab!(PANEL_IDS, "panel ID scheme", "PanelIds", [
    "positional" => "Positional",
    "stable" => "Stable",
]);
vocab!(VARIABLE_SORT, "variable sort", "VariableSort", [
    "disabled" => "Disabled",
    "alphabetical_asc" => "AlphabeticalAscending",
    "alphabetical_desc" => "AlphabeticalDescending",
    "numerical_asc" => "NumericalAscending",
    "numerical_desc" => "NumericalDescending",
    "alphabetical_case_insensitive_asc" => "AlphabeticalCaseInsensitiveAscending",
    "alphabetical_case_insensitive_desc" => "AlphabeticalCaseInsensitiveDescending",
]);
vocab!(VARIABLE_REFRESH, "variable refresh", "VariableRefresh", [
    "never" => "Never",
    "load" => "OnDashboardLoad",
    "time_range" => "OnTimeRangeChange",
]);
vocab!(JOIN_MODE, "join mode", "JoinMode", [
    "outer" => "Outer",
    "inner" => "Inner",
    "outer_tabular" => "OuterTabular",
]);
vocab!(SORT_DIRECTION, "sort direction", "SortDirection", [
    "asc" => "Ascending",
    "desc" => "Descending",
]);
vocab!(LABELS_MODE, "labels mode", "LabelsToFieldsMode", [
    "columns" => "Columns",
    "rows" => "Rows",
]);
vocab!(COLOR_SCHEME, "color scheme", "ColorScheme", [
    "thresholds" => "Thresholds",
    "classic_palette" => "ClassicPalette",
    "by_name" => "ClassicPaletteByName",
]);
vocab!(CONTINUOUS_PALETTE, "continuous palette", "ContinuousPalette", [
    "green_yellow_red" => "GreenYellowRed",
    "red_yellow_green" => "RedYellowGreen",
    "blue_yellow_red" => "BlueYellowRed",
    "yellow_red" => "YellowRed",
    "blue_purple" => "BluePurple",
    "yellow_blue" => "YellowBlue",
    "blues" => "Blues",
    "reds" => "Reds",
    "greens" => "Greens",
    "purples" => "Purples",
    "viridis" => "Viridis",
    "magma" => "Magma",
    "plasma" => "Plasma",
    "inferno" => "Inferno",
    "cividis" => "Cividis",
]);
vocab!(CURRENCY, "currency", "Currency", [
    "usd" => "Usd",
    "gbp" => "Gbp",
    "eur" => "Eur",
    "jpy" => "Jpy",
    "rub" => "Rub",
    "uah" => "Uah",
    "brl" => "Brl",
    "dkk" => "Dkk",
    "isk" => "Isk",
    "nok" => "Nok",
    "sek" => "Sek",
    "czk" => "Czk",
    "chf" => "Chf",
    "pln" => "Pln",
    "btc" => "Btc",
    "milli_btc" => "MilliBtc",
    "micro_btc" => "MicroBtc",
    "zar" => "Zar",
    "inr" => "Inr",
    "krw" => "Krw",
    "idr" => "Idr",
    "php" => "Php",
    "vnd" => "Vnd",
    "try" => "Try",
    "myr" => "Myr",
    "xpf" => "Xpf",
    "bgn" => "Bgn",
    "pyg" => "Pyg",
    "uyu" => "Uyu",
    "ils" => "Ils",
]);
vocab!(QUERY_TYPE, "query type", "QueryType", [
    "range" => "Range",
    "instant" => "Instant",
    "both" => "Both",
]);
vocab!(SPECIAL_VALUE, "special value", "SpecialValue", [
    "null" => "Null",
    "nan" => "NaN",
    "null_or_nan" => "NullOrNaN",
    "true" => "True",
    "false" => "False",
    "empty" => "Empty",
]);
vocab!(THRESHOLD_MODE, "threshold mode", "ThresholdMode", [
    "absolute" => "Absolute",
    "percentage" => "Percentage",
]);

/// How the tokens after a key are read, and the builder call they become.
#[derive(Debug)]
pub(crate) enum Form {
    /// `key: <expr>;` calls `.method(<expr>)`.
    Expr(&'static str),
    /// `key <expr>;`, with no colon, calls `.method(<expr>)`.
    Word(&'static str),
    /// `key: <word>;` calls `.method(Enum::Variant)`. Any other expression
    /// is passed through unchanged.
    Enum(&'static str, &'static Vocab),
    /// `key: <unit word> | custom(<expr>) | <expr>;`
    Unit(&'static str),
    /// `key: <color word> | "<custom color>" | <expr>;`
    Color(&'static str),
    /// `key: thresholds | classic_palette | fixed(<color>) | continuous(<palette>) | <expr>;`
    ColorScheme(&'static str),
    /// `key: [<reducer>, …] | <expr>;`
    Reducers(&'static str),
    /// `key: <stacking mode>;` calls `.method(Stacking::new(mode))`.
    Stacking(&'static str),
    /// `key: [<x>, <y>, <width>, <height>] | <expr>;` builds a `GridPos`.
    Grid(&'static str),
    /// `key: [<from>, <to>] | <expr>;` builds a `TimeRange`.
    TimeRange(&'static str),
    /// `key: (<field>, <direction>);` calls `.method(field, direction)`.
    SortBy(&'static str),
    /// `key { … }` builds `ty::new()`, applies the block, and passes it to
    /// `.method(…)`.
    Block {
        method: &'static str,
        ty: &'static str,
        body: &'static Context,
    },
    /// `key { … }` applies the block's keys to the enclosing builder.
    Inline(&'static Context),
    /// `key { … }` in the thresholds grammar, passed to `.method(…)`.
    Thresholds(&'static str),
    /// `key: promql!(<expr>) [{ … }] | <expr>;` passed to `.method(…)`.
    Query(&'static str),
    /// `key: <cell type> | <cell type> { … } | <expr>;`. A bare cell type is
    /// passed to `.bare(…)`, everything else to `.cell(…)`.
    Cell { bare: &'static str },
    /// `key <expr>: <json>;` calls `.method(<expr>, json!(<json>))`.
    Json(&'static str),
    /// `key <a> => <b> [{ … }]` builds `ty::new(a, b)`, applies the
    /// optional block, and passes it to `.method(…)`.
    Pair {
        method: &'static str,
        ty: &'static str,
        body: &'static Context,
    },
    /// `key <value> => <text> [{ … }]`, or `range(<from>, <to>)`,
    /// `regex(<pattern>)`, or `special(<value>)` before the `=>`, building a
    /// `ValueMapping` passed to `.method(…)`.
    Mapping(&'static str),
    /// `key: <variable> | horizontal(<variable>[, <max per row>]) |
    /// vertical(<variable>) | <expr>;` passed to `.method(…)`.
    Repeat(&'static str),
    /// `annotation <name> => <expr> [{ … }]` builds a Prometheus annotation.
    Annotation,
    /// `column <name> => <query> [{ … }]`, or with `trend` set,
    /// `trend <reducer> <name> => <query> [{ … }]`: a keyed table column.
    Column { trend: bool },
    /// `key: <expr>;` calling `.method(<expr>)` on the keyed table that a
    /// `keyed_table` panel becomes, after every table key.
    Keyed(&'static str),
    /// `datasource: <expr>;`. On a panel (`mixed` set) the word `mixed`
    /// selects Grafana's mixed datasource; elsewhere it is rejected.
    DataSource { mixed: bool },
    /// `override <matcher> { … }`, see [`MATCHERS`].
    Override,
    /// `transform …`, see [`TRANSFORMS`].
    Transform,
    /// `rename <from> => <to>;`
    Rename,
    /// `query <ref id> [: <reducer>];` in `time_series_to_table`.
    SeriesQuery,
    /// `time_field <ref id>: <field>;` in `time_series_to_table`.
    TimeField,
    /// `row <title> | row from <expr>` followed by a row block.
    Row,
    /// `variable <name> { … }`
    Variable,
}

/// One key of a context.
#[derive(Debug)]
pub(crate) struct Key {
    /// The DSL spelling.
    pub(crate) name: &'static str,
    /// The kinds of the context the key applies to.
    pub(crate) kinds: Kinds,
    /// How the key is read and lowered.
    pub(crate) form: Form,
}

const fn key(name: &'static str, kinds: Kinds, form: Form) -> Key {
    Key { name, kinds, form }
}

const fn expr(name: &'static str, kinds: Kinds) -> Key {
    key(name, kinds, Form::Expr(name))
}

const ALL: Kinds = Kinds::ALL;

/// A block of the DSL and the keys it accepts.
#[derive(Debug)]
pub(crate) struct Context {
    /// The block, for messages: "panel", "tooltip block".
    pub(crate) what: &'static str,
    /// What the block's entries are called in messages.
    pub(crate) entry: &'static str,
    /// The kinds a key can be restricted to, or empty.
    pub(crate) kinds: &'static [Kind],
    /// The local the expansion binds the block's builder to.
    pub(crate) var: &'static str,
    /// Whether panel items such as `stat "X" { … }` are accepted here.
    pub(crate) panels: bool,
    /// Every key. A name may appear more than once with disjoint kinds.
    pub(crate) keys: &'static [Key],
}

impl Context {
    const fn new(what: &'static str, var: &'static str, keys: &'static [Key]) -> Self {
        Self {
            what,
            entry: "key",
            kinds: &[],
            var,
            panels: false,
            keys,
        }
    }
}

pub(crate) static DASHBOARD: Context = Context {
    what: "dashboard",
    entry: "item",
    kinds: &[],
    var: "__panelist_dashboard",
    panels: true,
    keys: &[
        expr("uid", ALL),
        expr("description", ALL),
        expr("tags", ALL),
        expr("refresh", ALL),
        key("time", ALL, Form::TimeRange("time")),
        expr("timezone", ALL),
        expr("editable", ALL),
        key("datasource", ALL, Form::DataSource { mixed: false }),
        key("cursor_sync", ALL, Form::Enum("cursor_sync", &CURSOR_SYNC)),
        key("target", ALL, Form::Enum("target", &GRAFANA_TARGET)),
        expr("schema_version", ALL),
        key("panel_ids", ALL, Form::Enum("panel_ids", &PANEL_IDS)),
        expr("version", ALL),
        key(
            "link",
            ALL,
            Form::Pair {
                method: "link",
                ty: "DashboardLink",
                body: &LINK,
            },
        ),
        key("variable", ALL, Form::Variable),
        key("annotation", ALL, Form::Annotation),
        expr("built_in_annotation", ALL),
        key("row", ALL, Form::Row),
        expr("panel", ALL),
        expr("panels", ALL),
        key("extra", ALL, Form::Json("extra")),
    ],
};

pub(crate) static ROW: Context = Context {
    what: "row",
    entry: "item",
    kinds: &[],
    var: "__panelist_row",
    panels: true,
    keys: &[
        expr("key", ALL),
        expr("collapsed", ALL),
        expr("repeat", ALL),
        expr("panel", ALL),
        expr("panels", ALL),
    ],
};

const REDUCING: Kinds = STAT.or(GAUGE).or(BAR_GAUGE);

pub(crate) static PANEL: Context = Context {
    what: "panel",
    entry: "key",
    kinds: PANEL_KINDS,
    var: "__panelist_panel",
    panels: false,
    keys: &[
        // Every panel kind.
        key("query", ALL, Form::Query("query")),
        expr("legend_format", ALL),
        expr("description", ALL),
        expr("width", ALL),
        expr("height", ALL),
        expr("id", ALL),
        expr("key", ALL),
        key("grid", ALL, Form::Grid("grid_pos")),
        key("datasource", ALL, Form::DataSource { mixed: true }),
        key("unit", ALL, Form::Unit("unit")),
        expr("min", ALL),
        expr("max", ALL),
        expr("decimals", ALL),
        expr("display_name", ALL),
        expr("transparent", ALL),
        key("color", ALL, Form::ColorScheme("color")),
        key("thresholds", ALL, Form::Thresholds("thresholds")),
        key("mapping", ALL, Form::Mapping("mapping")),
        key(
            "link",
            ALL,
            Form::Pair {
                method: "link",
                ty: "DataLink",
                body: &DATA_LINK,
            },
        ),
        key(
            "data_link",
            ALL,
            Form::Pair {
                method: "data_link",
                ty: "DataLink",
                body: &DATA_LINK,
            },
        ),
        key("repeat", ALL, Form::Repeat("repeat")),
        expr("min_interval", ALL),
        expr("max_data_points", ALL),
        expr("time_from", ALL),
        expr("time_shift", ALL),
        expr("hide_time_override", ALL),
        key("override", ALL, Form::Override),
        key("transform", ALL, Form::Transform),
        key("option", ALL, Form::Json("option")),
        key("extra", ALL, Form::Json("extra")),
        // Text.
        expr("content", TEXT),
        key("mode", TEXT, Form::Enum("mode", &TEXT_MODE)),
        // Stat, gauge, and bar gauge.
        key(
            "color_mode",
            STAT,
            Form::Enum("color_mode", &STAT_COLOR_MODE),
        ),
        key(
            "graph_mode",
            STAT,
            Form::Enum("graph_mode", &STAT_GRAPH_MODE),
        ),
        expr("wide_layout", STAT),
        key(
            "orientation",
            REDUCING,
            Form::Enum("orientation", &ORIENTATION),
        ),
        key(
            "reduce",
            REDUCING,
            Form::Block {
                method: "reduce_options",
                ty: "ReduceOptions",
                body: &REDUCE,
            },
        ),
        key(
            "display_mode",
            BAR_GAUGE,
            Form::Enum("display_mode", &BAR_GAUGE_DISPLAY_MODE),
        ),
        // Time series.
        expr("fill_opacity", TIMESERIES),
        expr("line_width", TIMESERIES),
        expr("point_size", TIMESERIES),
        expr("span_nulls", TIMESERIES),
        key(
            "line_interpolation",
            TIMESERIES,
            Form::Enum("line_interpolation", &LINE_INTERPOLATION),
        ),
        key(
            "show_points",
            TIMESERIES,
            Form::Enum("show_points", &POINT_VISIBILITY),
        ),
        key("stacking", TIMESERIES, Form::Stacking("stacking")),
        key(
            "tooltip",
            TIMESERIES,
            Form::Block {
                method: "tooltip",
                ty: "Tooltip",
                body: &TOOLTIP,
            },
        ),
        key(
            "legend",
            TIMESERIES,
            Form::Block {
                method: "legend_options",
                ty: "Legend",
                body: &LEGEND,
            },
        ),
        // Table and keyed table.
        key("sort_by", TABLES, Form::SortBy("sort_by")),
        key("cell", TABLES, Form::Cell { bare: "cell" }),
        // Keyed table.
        key("key_name", KEYED_TABLE, Form::Keyed("key_name")),
        key("column", KEYED_TABLE, Form::Column { trend: false }),
        key("trend", KEYED_TABLE, Form::Column { trend: true }),
        // Heatmap.
        key(
            "color_mode",
            HEATMAP,
            Form::Enum("color_mode", &HEATMAP_COLOR_MODE),
        ),
        expr("color_scheme", HEATMAP),
        expr("color_steps", HEATMAP),
        expr("cell_gap", HEATMAP),
        expr("show_legend", HEATMAP),
        expr("calculate", HEATMAP),
        key("y_axis", HEATMAP, Form::Inline(&Y_AXIS)),
    ],
};

pub(crate) static QUERY: Context = Context {
    what: "query",
    entry: "key",
    kinds: QUERY_KINDS,
    var: "__panelist_query",
    panels: false,
    keys: &[
        expr("legend_format", ALL),
        expr("ref_id", ALL),
        key("query_type", ALL, Form::Enum("query_type", &QUERY_TYPE)),
        key("datasource", ALL, Form::DataSource { mixed: false }),
        expr("hidden", ALL),
        expr("interval", ALL),
        key(
            "editor_mode",
            ALL,
            Form::Enum("editor_mode", &QUERY_EDITOR_MODE),
        ),
        key("format", PROMQL, Form::Enum("format", &PROMETHEUS_FORMAT)),
        // Field configuration for the fields this query returns.
        key("unit", ALL, Form::Unit("unit")),
        key("color", ALL, Form::Color("color")),
        expr("display_name", ALL),
        expr("decimals", ALL),
        key("thresholds", ALL, Form::Thresholds("thresholds")),
        expr("field_property", ALL),
    ],
};

pub(crate) static VARIABLE: Context = Context::new(
    "variable",
    "__panelist_variable",
    &[
        key("query", ALL, Form::Query("query")),
        expr("values", ALL),
        expr("value", ALL),
        expr("plugin", ALL),
        expr("label", ALL),
        expr("default", ALL),
        key("datasource", ALL, Form::DataSource { mixed: false }),
        expr("multi", ALL),
        expr("include_all", ALL),
        expr("hidden", ALL),
        expr("regex", ALL),
        expr("all_value", ALL),
        expr("allow_custom_value", ALL),
        expr("skip_url_sync", ALL),
        key("sort", ALL, Form::Enum("sort", &VARIABLE_SORT)),
        key("refresh", ALL, Form::Enum("refresh", &VARIABLE_REFRESH)),
        key(
            "current",
            ALL,
            Form::Pair {
                method: "current",
                ty: "VariableSelection",
                body: &SELECTION,
            },
        ),
    ],
);

pub(crate) static SELECTION: Context = Context::new(
    "current block",
    "__panelist_selection",
    &[expr("selected", ALL)],
);

pub(crate) static LINK: Context = Context::new(
    "link block",
    "__panelist_link",
    &[
        expr("tooltip", ALL),
        expr("target_blank", ALL),
        expr("include_vars", ALL),
        expr("keep_time", ALL),
        expr("tags", ALL),
    ],
);

/// The options of a panel or field data link.
pub(crate) static DATA_LINK: Context = Context::new(
    "data link block",
    "__panelist_link",
    &[
        expr("target_blank", ALL),
        expr("include_vars", ALL),
        expr("keep_time", ALL),
    ],
);

pub(crate) static ANNOTATION: Context = Context::new(
    "annotation block",
    "__panelist_annotation",
    &[
        key("datasource", ALL, Form::DataSource { mixed: false }),
        expr("enable", ALL),
        expr("hide", ALL),
        key("icon_color", ALL, Form::Color("icon_color")),
        expr("step", ALL),
        expr("title_format", ALL),
        expr("text_format", ALL),
        expr("tag_keys", ALL),
        expr("use_value_for_time", ALL),
    ],
);

/// The field properties of a keyed table column.
pub(crate) static COLUMN: Context = Context::new(
    "column block",
    "__panelist_column",
    &[
        key("unit", ALL, Form::Unit("unit")),
        expr("min", ALL),
        expr("max", ALL),
        expr("decimals", ALL),
        key("color", ALL, Form::Color("color")),
        key("thresholds", ALL, Form::Thresholds("thresholds")),
        key("cell", ALL, Form::Cell { bare: "cell" }),
        expr("property", ALL),
    ],
);

pub(crate) static MAPPING: Context = Context::new(
    "mapping block",
    "__panelist_mapping",
    &[key("color", ALL, Form::Color("color"))],
);

pub(crate) static LEGEND: Context = Context::new(
    "legend block",
    "__panelist_legend",
    &[
        key("position", ALL, Form::Enum("placement", &LEGEND_PLACEMENT)),
        key("mode", ALL, Form::Enum("mode", &LEGEND_MODE)),
        key("values", ALL, Form::Reducers("calculations")),
    ],
);

pub(crate) static TOOLTIP: Context = Context::new(
    "tooltip block",
    "__panelist_tooltip",
    &[
        key("mode", ALL, Form::Enum("mode", &TOOLTIP_MODE)),
        key("sort", ALL, Form::Enum("sort", &TOOLTIP_SORT)),
        expr("hide_zeros", ALL),
    ],
);

pub(crate) static REDUCE: Context = Context::new(
    "reduce block",
    "__panelist_reduce",
    &[
        expr("values", ALL),
        key("calculations", ALL, Form::Reducers("calculations")),
        expr("fields", ALL),
    ],
);

pub(crate) static Y_AXIS: Context = Context::new(
    "y_axis block",
    "__panelist_panel",
    &[
        key("unit", ALL, Form::Unit("y_axis_unit")),
        key(
            "placement",
            ALL,
            Form::Enum("y_axis_placement", &AXIS_PLACEMENT),
        ),
    ],
);

pub(crate) static OVERRIDE: Context = Context::new(
    "override block",
    "__panelist_override",
    &[
        key("color", ALL, Form::Color("color")),
        expr("line_width", ALL),
        key("cell", ALL, Form::Cell { bare: "cell_type" }),
        key("unit", ALL, Form::Unit("unit")),
        expr("min", ALL),
        expr("max", ALL),
        expr("decimals", ALL),
        expr("display_name", ALL),
        key("thresholds", ALL, Form::Thresholds("thresholds")),
        key("mapping", ALL, Form::Mapping("mapping")),
        key(
            "data_link",
            ALL,
            Form::Pair {
                method: "data_link",
                ty: "DataLink",
                body: &DATA_LINK,
            },
        ),
    ],
);

pub(crate) static GAUGE_CELL: Context = Context::new(
    "gauge cell block",
    "__panelist_cell",
    &[
        key("mode", ALL, Form::Enum("mode", &BAR_GAUGE_DISPLAY_MODE)),
        key(
            "value_display",
            ALL,
            Form::Enum("value_display", &CELL_VALUE_DISPLAY),
        ),
    ],
);

pub(crate) static SPARKLINE_CELL: Context = Context::new(
    "sparkline cell block",
    "__panelist_cell",
    &[
        expr("hide_value", ALL),
        expr("line_width", ALL),
        expr("fill_opacity", ALL),
    ],
);

pub(crate) static BACKGROUND_CELL: Context = Context::new(
    "colored_background cell block",
    "__panelist_cell",
    &[
        key("mode", ALL, Form::Enum("mode", &CELL_BACKGROUND_MODE)),
        expr("apply_to_row", ALL),
    ],
);

/// Cell types with an options block, as `(word, builder type, block)`.
pub(crate) static CELL_BLOCKS: &[(&str, &str, &Context)] = &[
    ("gauge", "GaugeCell", &GAUGE_CELL),
    ("sparkline", "SparklineCell", &SPARKLINE_CELL),
    (
        "colored_background",
        "ColoredBackgroundCell",
        &BACKGROUND_CELL,
    ),
];

pub(crate) static ORGANIZE: Context = Context::new(
    "organize block",
    "__panelist_transform",
    &[
        key("rename", ALL, Form::Rename),
        key("hide", ALL, Form::Word("hide")),
        key("order", ALL, Form::Word("order")),
    ],
);

pub(crate) static SERIES_TO_TABLE: Context = Context::new(
    "time_series_to_table block",
    "__panelist_transform",
    &[
        key("query", ALL, Form::SeriesQuery),
        key("time_field", ALL, Form::TimeField),
    ],
);

pub(crate) static LABELS_TO_FIELDS: Context = Context::new(
    "labels_to_fields block",
    "__panelist_transform",
    &[
        key("mode", ALL, Form::Enum("mode", &LABELS_MODE)),
        key("keep", ALL, Form::Word("keep")),
        expr("value_label", ALL),
    ],
);

/// How a field override selects its fields.
#[derive(Debug)]
pub(crate) enum Matcher {
    /// `name(<expr>)` calls `FieldOverride::constructor(<expr>)`.
    Call(&'static str),
    /// `name <expr>` calls `FieldOverride::constructor(<expr>)`.
    Expr(&'static str),
    /// `name` calls `FieldOverride::constructor()`.
    Bare(&'static str),
}

pub(crate) static MATCHERS: &[(&str, Matcher)] = &[
    ("field", Matcher::Call("by_name")),
    ("regex", Matcher::Call("by_regex")),
    ("query", Matcher::Call("by_query")),
    ("type", Matcher::Call("by_type")),
    ("names", Matcher::Expr("by_names")),
    ("numeric", Matcher::Bare("numeric_fields")),
    ("time", Matcher::Bare("time_fields")),
];

/// How a typed transformation is written after `transform`.
#[derive(Debug)]
pub(crate) enum TransformForm {
    /// `name(<field>[, <word>])` builds `ty::new(field)`, and the optional
    /// word is passed to `.method(…)`.
    CallWithOption {
        ty: &'static str,
        method: &'static str,
        vocab: &'static Vocab,
    },
    /// `name(<field>, <word>)` builds `ty::new(field, word)`.
    CallWithArgument {
        ty: &'static str,
        vocab: &'static Vocab,
    },
    /// `name(<a>, <b>)` builds `ty::new(a, b)`.
    CallWithTwo { ty: &'static str },
    /// `name { … }`, or a bare `name` when `bare` is set.
    Block {
        ty: &'static str,
        body: &'static Context,
        bare: bool,
    },
}

pub(crate) static TRANSFORMS: &[(&str, TransformForm)] = &[
    (
        "join_by_field",
        TransformForm::CallWithOption {
            ty: "JoinByField",
            method: "mode",
            vocab: &JOIN_MODE,
        },
    ),
    (
        "sort_by",
        TransformForm::CallWithArgument {
            ty: "SortBy",
            vocab: &SORT_DIRECTION,
        },
    ),
    (
        "rename_by_regex",
        TransformForm::CallWithTwo {
            ty: "RenameByRegex",
        },
    ),
    (
        "organize",
        TransformForm::Block {
            ty: "OrganizeFields",
            body: &ORGANIZE,
            bare: false,
        },
    ),
    (
        "time_series_to_table",
        TransformForm::Block {
            ty: "TimeSeriesToTable",
            body: &SERIES_TO_TABLE,
            bare: false,
        },
    ),
    (
        "labels_to_fields",
        TransformForm::Block {
            ty: "LabelsToFields",
            body: &LABELS_TO_FIELDS,
            bare: true,
        },
    ),
];

/// Scopes a typed transformation with `only <word>(<expr>)`, as
/// `(word, method)`.
pub(crate) static TRANSFORM_SCOPES: &[(&str, &str)] =
    &[("ref_id", "only_ref_id"), ("frame_name", "only_frame_name")];

/// Keys of a thresholds block besides the steps, which are written
/// `<value> => <color>;` or, for a color word, `<color>: <value>;`.
pub(crate) static THRESHOLD_KEYS: &[&str] = &["base", "mode"];
