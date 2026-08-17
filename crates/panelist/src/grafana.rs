// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, HashSet};

use serde::Serialize;
use serde_json::{Value, json};

use crate::{
    ColorScheme, Dashboard, DashboardItem, DashboardLink, DataSource, FieldConfig, FieldOverride,
    GridPos, Legend, LegendMode, OverrideMatcher, OverrideProperty, Panel, PanelKind, Query, Row,
    ThresholdMode, Thresholds, ValidationError, ValidationErrors, Variable,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NormalizedDashboard {
    id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uid: Option<String>,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    tags: Vec<String>,
    timezone: String,
    editable: bool,
    graph_tooltip: u8,
    panels: Vec<GrafanaPanel>,
    time: GrafanaTimeRange,
    timepicker: BTreeMap<String, Value>,
    templating: GrafanaTemplating,
    annotations: GrafanaAnnotations,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh: Option<String>,
    schema_version: u32,
    version: u32,
    links: Vec<GrafanaLink>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[derive(Debug, Serialize)]
struct GrafanaTimeRange {
    from: String,
    to: String,
}

#[derive(Debug, Serialize)]
struct GrafanaTemplating {
    list: Vec<BTreeMap<String, Value>>,
}

#[derive(Debug, Serialize)]
struct GrafanaAnnotations {
    list: Vec<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GrafanaLink {
    #[serde(rename = "type")]
    kind: &'static str,
    title: String,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tooltip: Option<String>,
    target_blank: bool,
    include_vars: bool,
    keep_time: bool,
    tags: Vec<String>,
}

impl From<&DashboardLink> for GrafanaLink {
    fn from(link: &DashboardLink) -> Self {
        Self {
            kind: "link",
            title: link.title.clone(),
            url: link.url.clone(),
            tooltip: link.tooltip.clone(),
            target_blank: link.target_blank,
            include_vars: link.include_vars,
            keep_time: link.keep_time,
            tags: link.tags.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GrafanaPanel {
    id: u32,
    #[serde(rename = "type")]
    kind: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    grid_pos: GrafanaGridPos,
    #[serde(skip_serializing_if = "Option::is_none")]
    datasource: Option<DataSource>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    targets: Vec<GrafanaTarget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    field_config: Option<GrafanaFieldConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<BTreeMap<String, Value>>,
    #[serde(default, skip_serializing_if = "is_false")]
    transparent: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    links: Vec<GrafanaLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    collapsed: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    panels: Vec<GrafanaPanel>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

const fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Copy, Serialize)]
struct GrafanaGridPos {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

impl From<GridPos> for GrafanaGridPos {
    fn from(value: GridPos) -> Self {
        Self {
            x: value.x,
            y: value.y,
            w: value.width,
            h: value.height,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GrafanaTarget {
    ref_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    datasource: Option<DataSource>,
    expr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    legend_format: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    instant: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<bool>,
    hide: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    interval: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    query_type: Option<&'static str>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[derive(Debug, Serialize)]
struct GrafanaFieldConfig {
    defaults: BTreeMap<String, Value>,
    overrides: Vec<GrafanaFieldOverride>,
}

#[derive(Debug, Serialize)]
struct GrafanaFieldOverride {
    matcher: GrafanaMatcher,
    properties: Vec<GrafanaOverrideProperty>,
}

#[derive(Debug, Serialize)]
struct GrafanaMatcher {
    id: &'static str,
    options: String,
}

#[derive(Debug, Serialize)]
struct GrafanaOverrideProperty {
    id: String,
    value: Value,
}

pub(crate) fn normalize(dashboard: &Dashboard) -> Result<NormalizedDashboard, ValidationErrors> {
    validate(dashboard)?;

    let mut allocator = IdAllocator::new(explicit_panel_ids(dashboard));
    let mut panels = Vec::new();
    let mut current_y = 0;
    let mut top_level_flow = FlowLayout::new(current_y);

    for item in &dashboard.items {
        match item {
            DashboardItem::Panel(panel) => {
                let grid = top_level_flow.place(panel);
                let id = allocator.panel_id(panel.id);
                panels.push(normalize_panel(
                    panel,
                    id,
                    grid,
                    dashboard.datasource.as_ref(),
                ));
                current_y = top_level_flow.bottom();
            }
            DashboardItem::Row(row) => {
                current_y = top_level_flow.bottom().max(current_y);
                let row_y = current_y;
                let row_id = allocator.panel_id(None);
                let mut row_flow = FlowLayout::new(row_y.saturating_add(1));
                let mut row_panels = Vec::with_capacity(row.panels.len());

                for panel in &row.panels {
                    let grid = row_flow.place(panel);
                    let id = allocator.panel_id(panel.id);
                    row_panels.push(normalize_panel(
                        panel,
                        id,
                        grid,
                        dashboard.datasource.as_ref(),
                    ));
                }

                let row_bottom = row_flow.bottom();
                if row.collapsed {
                    panels.push(normalize_row(row, row_id, row_y, row_panels));
                } else {
                    panels.push(normalize_row(row, row_id, row_y, Vec::new()));
                    panels.extend(row_panels);
                }

                current_y = if row.collapsed {
                    row_y.saturating_add(1)
                } else {
                    row_bottom.max(row_y.saturating_add(1))
                };
                top_level_flow = FlowLayout::new(current_y);
            }
        }
    }

    Ok(NormalizedDashboard {
        id: None,
        uid: dashboard.uid.clone(),
        title: dashboard.title.clone(),
        description: dashboard.description.clone(),
        tags: dashboard.tags.clone(),
        timezone: dashboard.timezone.clone(),
        editable: dashboard.editable,
        graph_tooltip: 0,
        panels,
        time: GrafanaTimeRange {
            from: dashboard.time.from.clone(),
            to: dashboard.time.to.clone(),
        },
        timepicker: BTreeMap::new(),
        templating: GrafanaTemplating {
            list: dashboard
                .variables
                .iter()
                .map(|variable| normalize_variable(variable, dashboard.datasource.as_ref()))
                .collect(),
        },
        annotations: GrafanaAnnotations { list: Vec::new() },
        refresh: dashboard.refresh.clone(),
        schema_version: dashboard.schema_version,
        version: 0,
        links: dashboard.links.iter().map(GrafanaLink::from).collect(),
        extra: dashboard.extra.clone(),
    })
}

fn normalize_row(row: &Row, id: u32, y: u16, panels: Vec<GrafanaPanel>) -> GrafanaPanel {
    GrafanaPanel {
        id,
        kind: "row".to_owned(),
        title: row.title.clone(),
        description: None,
        grid_pos: GrafanaGridPos {
            x: 0,
            y,
            w: 24,
            h: 1,
        },
        datasource: None,
        targets: Vec::new(),
        field_config: None,
        options: None,
        transparent: false,
        links: Vec::new(),
        collapsed: Some(row.collapsed),
        panels,
        extra: BTreeMap::new(),
    }
}

fn normalize_panel(
    panel: &Panel,
    id: u32,
    grid: GridPos,
    default_datasource: Option<&DataSource>,
) -> GrafanaPanel {
    let datasource = panel.datasource.as_ref().or(default_datasource).cloned();
    let targets = normalize_targets(&panel.queries, datasource.as_ref());
    let mut options = default_panel_options(panel);
    options.extend(panel.options.clone());

    GrafanaPanel {
        id,
        kind: panel.kind.plugin_id().to_owned(),
        title: panel.title.clone(),
        description: panel.description.clone(),
        grid_pos: grid.into(),
        datasource,
        targets,
        field_config: Some(normalize_field_config(&panel.field_config, &panel.kind)),
        options: Some(options),
        transparent: panel.transparent,
        links: panel.links.iter().map(GrafanaLink::from).collect(),
        collapsed: None,
        panels: Vec::new(),
        extra: panel.extra.clone(),
    }
}

fn normalize_targets(
    queries: &[Query],
    panel_datasource: Option<&DataSource>,
) -> Vec<GrafanaTarget> {
    let mut used: HashSet<String> = queries
        .iter()
        .filter_map(|query| query.options().ref_id.clone())
        .collect();
    let mut next = 0usize;

    queries
        .iter()
        .map(|query| {
            let options = query.options();
            let ref_id = options.ref_id.clone().unwrap_or_else(|| {
                loop {
                    let candidate = reference_id(next);
                    next += 1;
                    if used.insert(candidate.clone()) {
                        break candidate;
                    }
                }
            });
            let query_type = match query {
                Query::Loki(_) => Some(if options.instant { "instant" } else { "range" }),
                Query::Prometheus(_) | Query::Raw(_) => None,
            };

            GrafanaTarget {
                ref_id,
                datasource: options.datasource.as_ref().or(panel_datasource).cloned(),
                expr: query.expression().to_owned(),
                legend_format: options.legend.clone(),
                instant: options.instant,
                range: options.range,
                hide: options.hidden,
                interval: options.interval.clone(),
                query_type,
                extra: query.raw_extra().cloned().unwrap_or_default(),
            }
        })
        .collect()
}

fn reference_id(mut index: usize) -> String {
    let mut bytes = Vec::new();
    loop {
        bytes.push(b'A' + (index % 26) as u8);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    bytes.reverse();
    bytes.into_iter().map(char::from).collect()
}

fn default_panel_options(panel: &Panel) -> BTreeMap<String, Value> {
    let mut options = BTreeMap::new();
    match &panel.kind {
        PanelKind::Timeseries => {
            options.insert("legend".to_owned(), legend_value(panel.legend.as_ref()));
            options.insert(
                "tooltip".to_owned(),
                json!({"mode": "single", "sort": "none", "hideZeros": false}),
            );
        }
        PanelKind::Stat => {
            options.insert("colorMode".to_owned(), json!("value"));
            options.insert("graphMode".to_owned(), json!("area"));
            options.insert("justifyMode".to_owned(), json!("auto"));
            options.insert("orientation".to_owned(), json!("auto"));
            options.insert(
                "reduceOptions".to_owned(),
                json!({"values": false, "calcs": ["lastNotNull"], "fields": ""}),
            );
            options.insert("textMode".to_owned(), json!("auto"));
        }
        PanelKind::Gauge => {
            options.insert("orientation".to_owned(), json!("auto"));
            options.insert(
                "reduceOptions".to_owned(),
                json!({"values": false, "calcs": ["lastNotNull"], "fields": ""}),
            );
            options.insert("showThresholdLabels".to_owned(), json!(false));
            options.insert("showThresholdMarkers".to_owned(), json!(true));
        }
        PanelKind::Table => {
            options.insert("cellHeight".to_owned(), json!("sm"));
            options.insert("showHeader".to_owned(), json!(true));
        }
        PanelKind::Text => {
            let (mode, content) = panel
                .text
                .as_ref()
                .map_or((crate::TextMode::Markdown, ""), |(mode, content)| {
                    (*mode, content.as_str())
                });
            options.insert("content".to_owned(), json!(content));
            options.insert("mode".to_owned(), json!(mode.as_grafana()));
        }
        PanelKind::BarGauge => {
            options.insert("displayMode".to_owned(), json!("gradient"));
            options.insert("orientation".to_owned(), json!("horizontal"));
            options.insert(
                "reduceOptions".to_owned(),
                json!({"values": false, "calcs": ["lastNotNull"], "fields": ""}),
            );
            options.insert("showUnfilled".to_owned(), json!(true));
        }
        PanelKind::Heatmap => {
            options.insert("calculate".to_owned(), json!(false));
            options.insert("cellGap".to_owned(), json!(1));
            options.insert(
                "color".to_owned(),
                json!({"mode": "scheme", "scheme": "Oranges", "steps": 64}),
            );
            options.insert("legend".to_owned(), json!({"show": true}));
            options.insert("yAxis".to_owned(), json!({"axisPlacement": "left"}));
        }
        PanelKind::Raw(_) => {}
    }
    options
}

fn legend_value(legend: Option<&Legend>) -> Value {
    let legend = legend.cloned().unwrap_or_default();
    let (show, display_mode) = match legend.mode {
        LegendMode::List => (true, "list"),
        LegendMode::Table => (true, "table"),
        LegendMode::Hidden => (false, "list"),
    };
    json!({
        "showLegend": show,
        "displayMode": display_mode,
        "placement": match legend.placement {
            crate::LegendPlacement::Bottom => "bottom",
            crate::LegendPlacement::Right => "right",
        },
        "calcs": legend
            .calculations
            .into_iter()
            .map(|calculation| calculation.as_grafana())
            .collect::<Vec<_>>(),
    })
}

fn normalize_field_config(config: &FieldConfig, kind: &PanelKind) -> GrafanaFieldConfig {
    let mut defaults = BTreeMap::new();
    if let Some(unit) = &config.unit {
        defaults.insert("unit".to_owned(), json!(unit.as_grafana()));
    }
    insert_number(&mut defaults, "min", config.min);
    insert_number(&mut defaults, "max", config.max);
    if let Some(decimals) = config.decimals {
        defaults.insert("decimals".to_owned(), json!(decimals));
    }
    if let Some(display_name) = &config.display_name {
        defaults.insert("displayName".to_owned(), json!(display_name));
    }
    if let Some(color) = &config.color {
        defaults.insert("color".to_owned(), color_scheme_value(color));
    }
    if let Some(thresholds) = &config.thresholds {
        defaults.insert("thresholds".to_owned(), thresholds_value(thresholds));
    }

    let mut custom = default_field_custom(kind);
    custom.extend(config.custom.clone());
    if !custom.is_empty() {
        defaults.insert("custom".to_owned(), json!(custom));
    }

    GrafanaFieldConfig {
        defaults,
        overrides: config.overrides.iter().map(normalize_override).collect(),
    }
}

fn default_field_custom(kind: &PanelKind) -> BTreeMap<String, Value> {
    let mut custom = BTreeMap::new();
    match kind {
        PanelKind::Timeseries => {
            custom.insert("drawStyle".to_owned(), json!("line"));
            custom.insert("fillOpacity".to_owned(), json!(0));
            custom.insert("lineInterpolation".to_owned(), json!("linear"));
            custom.insert("lineWidth".to_owned(), json!(1));
            custom.insert("showPoints".to_owned(), json!("auto"));
            custom.insert("spanNulls".to_owned(), json!(false));
        }
        PanelKind::Table => {
            custom.insert("align".to_owned(), json!("auto"));
            custom.insert("inspect".to_owned(), json!(false));
        }
        PanelKind::Stat
        | PanelKind::Gauge
        | PanelKind::Text
        | PanelKind::BarGauge
        | PanelKind::Heatmap
        | PanelKind::Raw(_) => {}
    }
    custom
}

fn normalize_override(field_override: &FieldOverride) -> GrafanaFieldOverride {
    let matcher = match &field_override.matcher {
        OverrideMatcher::Name(name) => GrafanaMatcher {
            id: "byName",
            options: name.clone(),
        },
        OverrideMatcher::Regex(regex) => GrafanaMatcher {
            id: "byRegexp",
            options: regex.clone(),
        },
        OverrideMatcher::Type(field_type) => GrafanaMatcher {
            id: "byType",
            options: field_type.clone(),
        },
    };

    GrafanaFieldOverride {
        matcher,
        properties: field_override
            .properties
            .iter()
            .map(|property| match property {
                OverrideProperty::Unit(unit) => property_value("unit", unit.as_grafana()),
                OverrideProperty::Min(value) => number_property("min", *value),
                OverrideProperty::Max(value) => number_property("max", *value),
                OverrideProperty::Decimals(value) => property_value("decimals", *value),
                OverrideProperty::DisplayName(value) => property_value("displayName", value),
                OverrideProperty::Color(value) => GrafanaOverrideProperty {
                    id: "color".to_owned(),
                    value: color_scheme_value(value),
                },
                OverrideProperty::LineWidth(value) => property_value("custom.lineWidth", *value),
                OverrideProperty::Custom { id, value } => GrafanaOverrideProperty {
                    id: id.clone(),
                    value: value.clone(),
                },
            })
            .collect(),
    }
}

fn property_value(id: &str, value: impl Serialize) -> GrafanaOverrideProperty {
    GrafanaOverrideProperty {
        id: id.to_owned(),
        value: serde_json::to_value(value).unwrap_or(Value::Null),
    }
}

fn number_property(id: &str, value: f64) -> GrafanaOverrideProperty {
    GrafanaOverrideProperty {
        id: id.to_owned(),
        value: serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number),
    }
}

fn insert_number(map: &mut BTreeMap<String, Value>, key: &str, number: Option<f64>) {
    if let Some(value) = number.and_then(serde_json::Number::from_f64) {
        map.insert(key.to_owned(), Value::Number(value));
    }
}

fn color_scheme_value(color: &ColorScheme) -> Value {
    match color {
        ColorScheme::Thresholds => json!({"mode": "thresholds"}),
        ColorScheme::ClassicPalette => json!({"mode": "palette-classic"}),
        ColorScheme::Fixed(color) => json!({"mode": "fixed", "fixedColor": color.as_grafana()}),
        ColorScheme::Continuous(scheme) => json!({"mode": scheme}),
    }
}

fn thresholds_value(thresholds: &Thresholds) -> Value {
    let mut steps = Vec::new();
    if thresholds
        .steps
        .first()
        .is_none_or(|step| step.value.is_some())
    {
        let color = thresholds
            .steps
            .first()
            .map_or("green", |step| step.color.as_grafana());
        steps.push(json!({"color": color, "value": null}));
    }
    steps.extend(thresholds.steps.iter().map(|step| {
        json!({
            "color": step.color.as_grafana(),
            "value": step.value,
        })
    }));
    json!({
        "mode": match thresholds.mode {
            ThresholdMode::Absolute => "absolute",
            ThresholdMode::Percentage => "percentage",
        },
        "steps": steps,
    })
}

fn normalize_variable(
    variable: &Variable,
    default_datasource: Option<&DataSource>,
) -> BTreeMap<String, Value> {
    match variable {
        Variable::Query(variable) => {
            let mut output = BTreeMap::new();
            output.insert("name".to_owned(), json!(variable.name));
            output.insert("type".to_owned(), json!("query"));
            if let Some(label) = &variable.label {
                output.insert("label".to_owned(), json!(label));
            }
            let expression = variable.query.expression();
            output.insert("query".to_owned(), json!(expression));
            output.insert("definition".to_owned(), json!(expression));
            if let Some(datasource) = variable
                .datasource
                .as_ref()
                .or(variable.query.options().datasource.as_ref())
                .or(default_datasource)
            {
                output.insert("datasource".to_owned(), datasource_value(datasource));
            }
            output.insert("refresh".to_owned(), json!(variable.refresh.as_grafana()));
            output.insert("multi".to_owned(), json!(variable.multi));
            output.insert("includeAll".to_owned(), json!(variable.include_all));
            output.insert("hide".to_owned(), json!(u8::from(variable.hidden)));
            output.insert("regex".to_owned(), json!(""));
            output.insert("options".to_owned(), json!([]));
            if let Some(default) = &variable.default {
                output.insert(
                    "current".to_owned(),
                    json!({"selected": true, "text": default, "value": default}),
                );
            }
            output
        }
        Variable::Custom(variable) => {
            let selected = variable
                .default
                .as_ref()
                .or_else(|| variable.values.first());
            let options = variable
                .values
                .iter()
                .map(|value| {
                    json!({
                        "selected": selected == Some(value),
                        "text": value,
                        "value": value,
                    })
                })
                .collect::<Vec<_>>();
            let mut output = BTreeMap::new();
            output.insert("name".to_owned(), json!(variable.name));
            output.insert("type".to_owned(), json!("custom"));
            if let Some(label) = &variable.label {
                output.insert("label".to_owned(), json!(label));
            }
            output.insert("query".to_owned(), json!(variable.values.join(",")));
            output.insert("options".to_owned(), json!(options));
            output.insert("multi".to_owned(), json!(variable.multi));
            output.insert("includeAll".to_owned(), json!(variable.include_all));
            output.insert("hide".to_owned(), json!(u8::from(variable.hidden)));
            if let Some(selected) = selected {
                output.insert(
                    "current".to_owned(),
                    json!({"selected": true, "text": selected, "value": selected}),
                );
            }
            output
        }
        Variable::Constant(variable) => {
            let mut output = BTreeMap::new();
            output.insert("name".to_owned(), json!(variable.name));
            output.insert("type".to_owned(), json!("constant"));
            if let Some(label) = &variable.label {
                output.insert("label".to_owned(), json!(label));
            }
            output.insert("query".to_owned(), json!(variable.value));
            output.insert("hide".to_owned(), json!(u8::from(variable.hidden)));
            output.insert(
                "current".to_owned(),
                json!({"selected": true, "text": variable.value, "value": variable.value}),
            );
            output
        }
    }
}

fn datasource_value(datasource: &DataSource) -> Value {
    json!({"type": datasource.kind(), "uid": datasource.uid()})
}

fn validate(dashboard: &Dashboard) -> Result<(), ValidationErrors> {
    let mut errors = Vec::new();
    if dashboard.title.trim().is_empty() {
        errors.push(ValidationError::MissingDashboardTitle);
    }
    for variable in &dashboard.variables {
        if variable.name().trim().is_empty() {
            errors.push(ValidationError::MissingVariableName);
        }
        if let Variable::Query(variable) = variable
            && variable.query.expression().trim().is_empty()
        {
            errors.push(ValidationError::MissingQueryExpression {
                panel: format!("variable {}", variable.name),
            });
        }
    }

    let mut ids = HashSet::new();
    for item in &dashboard.items {
        match item {
            DashboardItem::Row(row) => {
                if row.title.trim().is_empty() {
                    errors.push(ValidationError::MissingRowTitle);
                }
                for panel in &row.panels {
                    validate_panel(panel, &mut ids, &mut errors);
                }
            }
            DashboardItem::Panel(panel) => validate_panel(panel, &mut ids, &mut errors),
        }
    }
    ValidationErrors::from_vec(errors)
}

fn validate_panel(panel: &Panel, ids: &mut HashSet<u32>, errors: &mut Vec<ValidationError>) {
    if panel.title.trim().is_empty() {
        errors.push(ValidationError::MissingPanelTitle);
    }
    if !(1..=24).contains(&panel.width) {
        errors.push(ValidationError::InvalidPanelWidth {
            panel: panel.title.clone(),
            width: panel.width,
        });
    }
    if panel.height == 0 {
        errors.push(ValidationError::InvalidPanelHeight {
            panel: panel.title.clone(),
            height: panel.height,
        });
    }
    if let Some(grid) = panel.grid_pos
        && (grid.width == 0
            || grid.width > 24
            || grid.height == 0
            || grid.x >= 24
            || grid.x.saturating_add(grid.width) > 24)
    {
        errors.push(ValidationError::InvalidGridPosition {
            panel: panel.title.clone(),
            x: grid.x,
            y: grid.y,
            width: grid.width,
            height: grid.height,
        });
    }
    if let Some(id) = panel.id
        && !ids.insert(id)
    {
        errors.push(ValidationError::DuplicatePanelId {
            id,
            panel: panel.title.clone(),
        });
    }

    let mut refs = HashSet::new();
    for query in &panel.queries {
        if query.expression().trim().is_empty() {
            errors.push(ValidationError::MissingQueryExpression {
                panel: panel.title.clone(),
            });
        }
        if let Some(ref_id) = &query.options().ref_id
            && !refs.insert(ref_id.clone())
        {
            errors.push(ValidationError::DuplicateQueryRefId {
                panel: panel.title.clone(),
                ref_id: ref_id.clone(),
            });
        }
    }
    if let Some(thresholds) = &panel.field_config.thresholds
        && !thresholds_are_valid(thresholds)
    {
        errors.push(ValidationError::InvalidThresholds {
            panel: panel.title.clone(),
        });
    }
}

fn thresholds_are_valid(thresholds: &Thresholds) -> bool {
    let mut previous = None;
    for (index, step) in thresholds.steps.iter().enumerate() {
        match step.value {
            None if index == 0 => {}
            None => return false,
            Some(value) if !value.is_finite() => return false,
            Some(value) => {
                if previous.is_some_and(|previous| value <= previous) {
                    return false;
                }
                previous = Some(value);
            }
        }
    }
    true
}

fn explicit_panel_ids(dashboard: &Dashboard) -> HashSet<u32> {
    let mut ids = HashSet::new();
    for item in &dashboard.items {
        match item {
            DashboardItem::Panel(panel) => {
                if let Some(id) = panel.id {
                    ids.insert(id);
                }
            }
            DashboardItem::Row(row) => {
                ids.extend(row.panels.iter().filter_map(|panel| panel.id));
            }
        }
    }
    ids
}

struct IdAllocator {
    used: HashSet<u32>,
    next: u32,
}

impl IdAllocator {
    fn new(reserved: HashSet<u32>) -> Self {
        Self {
            used: reserved,
            next: 1,
        }
    }

    fn panel_id(&mut self, explicit: Option<u32>) -> u32 {
        if let Some(explicit) = explicit {
            return explicit;
        }
        while self.used.contains(&self.next) {
            self.next = self.next.saturating_add(1);
        }
        let id = self.next;
        self.used.insert(id);
        self.next = self.next.saturating_add(1);
        id
    }
}

struct FlowLayout {
    x: u16,
    y: u16,
    line_height: u16,
    max_bottom: u16,
}

impl FlowLayout {
    const fn new(y: u16) -> Self {
        Self {
            x: 0,
            y,
            line_height: 0,
            max_bottom: y,
        }
    }

    fn place(&mut self, panel: &Panel) -> GridPos {
        if let Some(explicit) = panel.grid_pos {
            self.max_bottom = self
                .max_bottom
                .max(explicit.y.saturating_add(explicit.height));
            self.y = self.max_bottom;
            self.x = 0;
            self.line_height = 0;
            return explicit;
        }

        if self.x > 0 && self.x.saturating_add(panel.width) > 24 {
            self.y = self.y.saturating_add(self.line_height);
            self.x = 0;
            self.line_height = 0;
        }
        let grid = GridPos::new(self.x, self.y, panel.width, panel.height);
        self.x = self.x.saturating_add(panel.width);
        self.line_height = self.line_height.max(panel.height);
        self.max_bottom = self.max_bottom.max(self.y.saturating_add(self.line_height));
        if self.x == 24 {
            self.y = self.y.saturating_add(self.line_height);
            self.x = 0;
            self.line_height = 0;
        }
        grid
    }

    fn bottom(&self) -> u16 {
        self.max_bottom.max(self.y.saturating_add(self.line_height))
    }
}
