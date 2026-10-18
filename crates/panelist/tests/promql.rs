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

//! PromQL selector and legend helpers.

use panelist::{LegendFormatError, prelude::*};
use serde_json::Value;

fn first_target(dashboard: &Dashboard) -> Value {
    match serde_json::to_value(dashboard) {
        Ok(value) => value["panels"][0]["targets"][0].clone(),
        Err(error) => panic!("test dashboard should serialize: {error}"),
    }
}

#[test]
fn metric_without_matchers_renders_without_braces() {
    assert_eq!(Selector::metric("up").to_string(), "up");
    assert_eq!(
        Selector::metric("node:cpu_seconds:rate5m").to_string(),
        "node:cpu_seconds:rate5m"
    );
}

#[test]
fn renders_every_operator_in_insertion_order() {
    let selector = Selector::metric("http_requests_total")
        .ne("method", "OPTIONS")
        .eq("job", "api")
        .nre("path", "/health.*")
        .re("instance", "$instance")
        .eq("job", "api-canary");

    assert_eq!(
        selector.to_string(),
        r#"http_requests_total{method!="OPTIONS",job="api",path!~"/health.*",instance=~"$instance",job="api-canary"}"#
    );
}

#[test]
fn escapes_label_values_as_promql_string_literals() {
    let selector = Selector::metric("events_total")
        .eq("message", "say \"hi\"\nthen\tleave\r")
        .eq("path", r"C:\temp");

    assert_eq!(
        selector.to_string(),
        r#"events_total{message="say \"hi\"\nthen\tleave\r",path="C:\\temp"}"#
    );
}

#[test]
fn regex_values_are_escaped_once() {
    let selector = Selector::metric("http_requests_total").re("route", r"/api/v\d+/.*");

    assert_eq!(
        selector.to_string(),
        r#"http_requests_total{route=~"/api/v\\d+/.*"}"#
    );
}

#[test]
fn grafana_variables_pass_through_values_untouched() {
    let selector = Selector::metric("up")
        .re("instance", "${instance:regex}")
        .eq("job", "$job");

    assert_eq!(
        selector.to_string(),
        r#"up{instance=~"${instance:regex}",job="$job"}"#
    );
}

#[test]
fn selector_without_metric_name() {
    assert_eq!(
        Selector::new()
            .eq("job", "api")
            .re("instance", "$instance")
            .to_string(),
        r#"{job="api",instance=~"$instance"}"#
    );
    assert_eq!(Selector::new().to_string(), "{}");
}

#[test]
fn cloned_selector_extends_without_changing_the_base() {
    let base = Selector::metric("http_requests_total").eq("job", "api");
    let errors = base.clone().re("status", "5..");

    assert_eq!(base.to_string(), r#"http_requests_total{job="api"}"#);
    assert_eq!(
        errors.to_string(),
        r#"http_requests_total{job="api",status=~"5.."}"#
    );
}

#[test]
fn shared_matchers_append_in_order() {
    let base = Matchers::new().eq("job", "api").re("instance", "$instance");
    let extended = Matchers::new().eq("env", "prod").with(&base).ne("pod", "");

    assert_eq!(
        Selector::metric("http_requests_total")
            .eq("method", "GET")
            .with(&base)
            .to_string(),
        r#"http_requests_total{method="GET",job="api",instance=~"$instance"}"#
    );
    assert_eq!(
        Selector::new().with(&extended).to_string(),
        r#"{env="prod",job="api",instance=~"$instance",pod!=""}"#
    );
}

#[test]
fn non_classic_names_use_quoted_utf8_syntax() {
    assert_eq!(
        Selector::metric("http.server.duration").to_string(),
        r#"{"http.server.duration"}"#
    );
    assert_eq!(
        Selector::metric("http.server.duration")
            .eq("service.name", "api")
            .eq("job", "api")
            .to_string(),
        r#"{"http.server.duration","service.name"="api",job="api"}"#
    );
    // Colons are valid in metric names but not in label names.
    assert_eq!(Selector::new().eq("a:b", "c").to_string(), r#"{"a:b"="c"}"#);
    assert_eq!(
        Selector::new().eq("9lives", "x").to_string(),
        r#"{"9lives"="x"}"#
    );
}

#[test]
fn names_holding_grafana_variables_render_verbatim() {
    assert_eq!(
        Selector::metric("${prefix}_requests_total")
            .eq("$label", "api")
            .to_string(),
        r#"${prefix}_requests_total{$label="api"}"#
    );
}

#[test]
fn range_attaches_to_the_selector() {
    assert_eq!(Selector::metric("up").range("5m").to_string(), "up[5m]");
    assert_eq!(
        Selector::metric("http_requests_total")
            .eq("job", "api")
            .range("$__rate_interval")
            .to_string(),
        r#"http_requests_total{job="api"}[$__rate_interval]"#
    );
    assert_eq!(
        Selector::new().eq("job", "api").range("1h").to_string(),
        r#"{job="api"}[1h]"#
    );
    assert_eq!(
        Selector::metric("http.server.duration")
            .range("5m")
            .to_string(),
        r#"{"http.server.duration"}[5m]"#
    );
}

#[test]
fn factored_selector_never_becomes_a_subquery() {
    let base = Selector::metric("http_requests_total").eq("job", "api");
    let expression = format!(
        "sum(rate({})) / sum(rate({}))",
        base.clone().re("status", "5..").range("$__rate_interval"),
        base.range("$__rate_interval"),
    );

    assert_eq!(
        expression,
        r#"sum(rate(http_requests_total{job="api",status=~"5.."}[$__rate_interval])) / sum(rate(http_requests_total{job="api"}[$__rate_interval]))"#
    );
    assert!(!expression.contains(")["));
}

#[test]
fn selectors_are_accepted_as_query_expressions() {
    let selector = Selector::metric("up").eq("job", "api");
    let range = selector.clone().range("5m");
    let owned: String = selector.clone().into();
    let borrowed: String = (&range).into();

    assert_eq!(owned, r#"up{job="api"}"#);
    assert_eq!(borrowed, r#"up{job="api"}[5m]"#);

    let dashboard = Dashboard::new("Selectors")
        .panel(Timeseries::new("Up").query(PrometheusQuery::new(&selector)));
    assert_eq!(first_target(&dashboard)["expr"], r#"up{job="api"}"#);

    let dashboard = Dashboard::new("Selectors")
        .panel(Timeseries::new("Up").query(promql!(range).legend_format(legend!("{instance}"))));
    let target = first_target(&dashboard);
    assert_eq!(target["expr"], r#"up{job="api"}[5m]"#);
    assert_eq!(target["legendFormat"], "{{instance}}");
}

#[test]
fn helpers_work_inside_the_dashboard_macro() {
    let requests = Selector::metric("http_requests_total").eq("job", "api");
    let dashboard = dashboard! {
        title: "HTTP";
        timeseries "Requests" {
            query: promql!(format!("sum by (status) (rate({}))", requests.range("$__rate_interval"))) {
                legend_format: legend!("{status}");
            }
        }
    };

    let target = first_target(&dashboard);
    assert_eq!(
        target["expr"],
        r#"sum by (status) (rate(http_requests_total{job="api"}[$__rate_interval]))"#
    );
    assert_eq!(target["legendFormat"], "{{status}}");
}

#[test]
fn legend_converts_single_brace_placeholders() {
    assert_eq!(legend("{status}").unwrap(), "{{status}}");
    assert_eq!(
        legend("{namespace} · {pod}").unwrap(),
        "{{namespace}} · {{pod}}"
    );
    assert_eq!(legend("{a}{b}").unwrap(), "{{a}}{{b}}");
    assert_eq!(legend("{__name__}").unwrap(), "{{__name__}}");
    assert_eq!(legend("p99").unwrap(), "p99");
    assert_eq!(legend("").unwrap(), "");
}

#[test]
fn legend_doubled_braces_are_literal() {
    assert_eq!(legend("{{ {pod} }}").unwrap(), "{ {{pod}} }");
    assert_eq!(legend("{{}}").unwrap(), "{}");
    assert_eq!(legend("{{1}}").unwrap(), "{1}");
    assert_eq!(legend("}}{{").unwrap(), "}{");
}

#[test]
fn legend_macro_matches_the_function() {
    const TEMPLATE: &str = "{namespace}/{pod} {{ {container} }}";

    assert_eq!(legend!(TEMPLATE), legend(TEMPLATE).unwrap());
    assert_eq!(
        legend!(concat!("{job}", " · ", "{instance}")),
        "{{job}} · {{instance}}"
    );
    assert_eq!(legend!("{status}",), "{{status}}");
}

#[test]
fn legend_reports_unclosed_and_unmatched_braces() {
    assert_eq!(
        legend("ok {pod"),
        Err(LegendFormatError::UnclosedPlaceholder { offset: 3 })
    );
    assert_eq!(
        legend("· {pod}}"),
        Err(LegendFormatError::UnmatchedBrace { offset: 8 })
    );
    assert_eq!(
        legend("}"),
        Err(LegendFormatError::UnmatchedBrace { offset: 0 })
    );
}

#[test]
fn legend_rejects_invalid_label_names() {
    for (template, name, offset) in [
        ("{status-code}", "status-code", 0),
        ("x {}", "", 2),
        ("{9lives}", "9lives", 0),
        ("{ pod }", " pod ", 0),
        ("{service.name}", "service.name", 0),
        ("{a{b}", "a{b", 0),
    ] {
        assert_eq!(
            legend(template),
            Err(LegendFormatError::InvalidLabelName {
                name: name.to_owned(),
                offset
            }),
            "{template}"
        );
    }
}

#[test]
fn legend_rejects_grafana_placeholders() {
    for (template, offset) in [("{{pod}}", 0), ("x {{ pod }}", 2), ("{{{{pod}}}}", 2)] {
        assert_eq!(
            legend(template),
            Err(LegendFormatError::GrafanaPlaceholder {
                name: "pod".to_owned(),
                offset
            }),
            "{template}"
        );
    }
}

#[test]
fn legend_errors_explain_the_fix() {
    let message = |template: &str| legend(template).unwrap_err().to_string();

    assert_eq!(
        message("{pod"),
        "legend placeholder opened at byte 0 is never closed; write `{{` for a literal `{`"
    );
    assert_eq!(
        message("}"),
        "unmatched `}` at byte 0; write `}}` for a literal `}`"
    );
    assert_eq!(
        message("{status-code}"),
        "legend placeholder `{status-code}` at byte 0 is not a Prometheus label name like `pod` or `__name__`"
    );
    assert_eq!(
        message("{{pod}}"),
        "`{{pod}}` at byte 0 is Grafana's placeholder syntax; write `{pod}` and Panelist emits the double braces"
    );
}
