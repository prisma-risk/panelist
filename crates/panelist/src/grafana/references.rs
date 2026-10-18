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

//! Finds the template-variable references Grafana interpolates in a string.

/// Names Grafana itself defines in every dashboard, so a reference to one
/// resolves without a declared variable.
///
/// The list follows Grafana's global variables page
/// (<https://grafana.com/docs/grafana/latest/visualizations/dashboards/variables/global-variables/>),
/// plus the reserved names other pages document: `__all` (the "All"
/// selection value), `__auto` (the automatic step, Grafana 10.2+),
/// `__searchFilter` (the text typed into a query variable's dropdown), and
/// `__all_variables` (every current variable, for link URLs). Dotted forms
/// such as `${__user.login}` resolve through their base name.
const GRAFANA_GLOBALS: &[&str] = &[
    "__all",
    "__all_variables",
    "__auto",
    "__dashboard",
    "__from",
    "__interval",
    "__interval_ms",
    "__name",
    "__org",
    "__range",
    "__range_ms",
    "__range_s",
    "__rate_interval",
    "__rate_interval_ms",
    "__searchFilter",
    "__timeFilter",
    "__timezone",
    "__to",
    "__url_time_range",
    "__user",
    "timeFilter",
];

/// Whether `name` is a variable Grafana defines on every dashboard.
pub(crate) fn is_grafana_global(name: &str) -> bool {
    GRAFANA_GLOBALS.contains(&name)
}

/// Whether `name` may be a macro the target datasource plugin expands.
///
/// Only consulted for raw queries, whose plugin Panelist does not know.
/// Grafana reserves the `__` prefix, so no dashboard variable can claim it,
/// and plugins put their macros there: the SQL family's `$__timeGroup`,
/// ClickHouse's `$__dateFilter`, and so on. `interval` is InfluxQL's legacy
/// spelling of `$__interval`. Typed Prometheus and Loki queries get no such
/// leeway, so a typo like `$__rate_inteval` there is still caught.
pub(crate) fn is_plugin_macro(name: &str) -> bool {
    name.starts_with("__") || name == "interval"
}

/// Returns the name of every template-variable reference in `text`, in
/// order, repeats included.
///
/// This mirrors the `variableRegex` Grafana's template service matches with
/// (`public/app/features/variables/utils.ts`):
///
/// ```text
/// \$(\w+)|\[\[(\w+?)(?::(\w+))?\]\]|\${(\w+)(?:\.([^:^\}]+))?(?::([^\}]+))?}
/// ```
///
/// so `$name`, `[[name]]`, `[[name:format]]`, `${name}`, `${name.path}`,
/// and `${name:format}` are found exactly where Grafana would substitute
/// them, and nowhere else. A `$` that starts no reference, such as the
/// anchor in `"^api$"`, is skipped the way the regex skips it.
///
/// All-digit names are dropped. `$1` and `${1}` are regex capture
/// references in `label_replace` and rename patterns. Grafana matches them
/// but leaves them alone because no variable can be named `1`.
pub(crate) fn variable_references(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut names = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let matched = match bytes[index] {
            b'$' => dollar_reference(bytes, index),
            b'[' => bracket_reference(bytes, index),
            _ => None,
        };
        match matched {
            Some((name, end)) => {
                if !bytes[name.clone()].iter().all(u8::is_ascii_digit) {
                    // `name` spans only ASCII word bytes, so it sits on
                    // UTF-8 character boundaries.
                    names.push(&text[name]);
                }
                index = end;
            }
            None => index += 1,
        }
    }
    names
}

type Match = (std::ops::Range<usize>, usize);

/// `\w` in a JavaScript regex without the `u` flag: ASCII only.
const fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn word_end(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .iter()
        .position(|byte| !is_word(*byte))
        .map_or(bytes.len(), |offset| start + offset)
}

/// `\$(\w+)` or, failing that, `\${(\w+)(?:\.([^:^\}]+))?(?::([^\}]+))?}`.
fn dollar_reference(bytes: &[u8], dollar: usize) -> Option<Match> {
    let start = dollar + 1;
    let end = word_end(bytes, start);
    if end > start {
        return Some((start..end, end));
    }
    if bytes.get(start) != Some(&b'{') {
        return None;
    }

    let name_start = start + 1;
    let name_end = word_end(bytes, name_start);
    if name_end == name_start {
        return None;
    }
    let mut cursor = name_end;
    if bytes.get(cursor) == Some(&b'.') {
        let path_end = bytes[cursor + 1..]
            .iter()
            .position(|byte| matches!(byte, b':' | b'^' | b'}'))
            .map_or(bytes.len(), |offset| cursor + 1 + offset);
        if path_end == cursor + 1 {
            return None;
        }
        cursor = path_end;
    }
    if bytes.get(cursor) == Some(&b':') {
        let format_end = bytes[cursor + 1..]
            .iter()
            .position(|byte| *byte == b'}')
            .map_or(bytes.len(), |offset| cursor + 1 + offset);
        if format_end == cursor + 1 {
            return None;
        }
        cursor = format_end;
    }
    (bytes.get(cursor) == Some(&b'}')).then_some((name_start..name_end, cursor + 1))
}

/// `\[\[(\w+?)(?::(\w+))?\]\]`. The lazy `\w+?` must be followed by `:` or
/// `]`, neither of which is a word byte, so it behaves as a greedy `\w+`.
fn bracket_reference(bytes: &[u8], bracket: usize) -> Option<Match> {
    if bytes.get(bracket + 1) != Some(&b'[') {
        return None;
    }
    let name_start = bracket + 2;
    let name_end = word_end(bytes, name_start);
    if name_end == name_start {
        return None;
    }
    let mut cursor = name_end;
    if bytes.get(cursor) == Some(&b':') {
        let format_end = word_end(bytes, cursor + 1);
        if format_end == cursor + 1 {
            return None;
        }
        cursor = format_end;
    }
    (bytes.get(cursor..cursor + 2) == Some(b"]]")).then_some((name_start..name_end, cursor + 2))
}

#[cfg(test)]
mod tests {
    use super::variable_references;

    #[test]
    fn finds_every_grafana_syntax() {
        assert_eq!(
            variable_references("$a ${b} ${c:csv} ${d.path} ${e.path:raw} [[f]] [[g:regex]]"),
            ["a", "b", "c", "d", "e", "f", "g"]
        );
    }

    #[test]
    fn plain_syntax_is_greedy_like_grafana() {
        // Grafana reads `$pod_name` as a variable called `pod_name`, never
        // `pod` followed by `_name`. Only braces end a name early.
        assert_eq!(
            variable_references("$pod_name ${pod}_name"),
            ["pod_name", "pod"]
        );
    }

    #[test]
    fn dollars_that_start_no_reference_are_skipped() {
        assert_eq!(
            variable_references(r#"up{job=~"^(api|web)$"} $ ${} ${a ${a.} ${a:} [[]] [[a] [a]"#),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn capture_group_references_are_not_variables() {
        assert_eq!(
            variable_references(r#"label_replace(up, "x", "$1-${2}", "instance", "(.*):(.*)") $0"#),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn a_failed_brace_match_rescans_from_the_next_byte() {
        // Grafana's regex, failing at `${`, resumes one character later and
        // still finds the `$b` inside.
        assert_eq!(variable_references("${a $b"), ["b"]);
        assert_eq!(variable_references("[[$x]]"), ["x"]);
    }

    #[test]
    fn non_ascii_text_around_references_is_handled() {
        assert_eq!(variable_references("é$naïve ${b}µ"), ["na", "b"]);
    }
}
