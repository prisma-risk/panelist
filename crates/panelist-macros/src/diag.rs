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

//! Error collection and the wording of the macro's own diagnostics.

use proc_macro2::{Ident, TokenStream};

use crate::spec::{Context, Form, Key, Kinds, Vocab};

/// Every error found in one invocation, reported together.
#[derive(Debug, Default)]
pub(crate) struct Diagnostics {
    errors: Option<syn::Error>,
}

impl Diagnostics {
    pub(crate) fn push(&mut self, error: syn::Error) {
        match &mut self.errors {
            Some(errors) => errors.combine(error),
            None => self.errors = Some(error),
        }
    }

    pub(crate) fn into_compile_errors(self) -> Option<TokenStream> {
        self.errors.map(syn::Error::into_compile_error)
    }
}

/// The closest candidate to `word`, if any is close enough to be a typo.
pub(crate) fn suggest<'a>(
    word: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let limit = (word.chars().count() / 3).max(1);
    candidates
        .into_iter()
        .map(|candidate| (distance(word, candidate), candidate))
        .filter(|(distance, _)| *distance <= limit)
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate)
}

/// Optimal string alignment distance: edits, counting an adjacent
/// transposition as one.
fn distance(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    let width = right.len() + 1;
    let mut table = vec![0usize; (left.len() + 1) * width];
    for (i, cell) in table.iter_mut().step_by(width).enumerate() {
        *cell = i;
    }
    for (j, cell) in table.iter_mut().take(width).enumerate() {
        *cell = j;
    }
    for i in 1..=left.len() {
        for j in 1..=right.len() {
            let cost = usize::from(left[i - 1] != right[j - 1]);
            let mut best = (table[(i - 1) * width + j] + 1)
                .min(table[i * width + j - 1] + 1)
                .min(table[(i - 1) * width + j - 1] + cost);
            if i > 1 && j > 1 && left[i - 1] == right[j - 2] && left[i - 2] == right[j - 1] {
                best = best.min(table[(i - 2) * width + j - 2] + 1);
            }
            table[i * width + j] = best;
        }
    }
    table[left.len() * width + right.len()]
}

fn backticked<'a>(names: impl IntoIterator<Item = &'a str>) -> String {
    let mut seen = Vec::new();
    for name in names {
        if !seen.contains(&name) {
            seen.push(name);
        }
    }
    seen.iter()
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// "a `stat` panel", or "a tooltip block" in a context without kinds.
pub(crate) fn describe(context: &Context, kind: Option<usize>) -> String {
    match kind.and_then(|index| context.kinds.get(index)) {
        Some(kind) => format!("a `{}` {}", kind.keyword, context.what),
        None if context.what == "dashboard" => "the dashboard".to_owned(),
        None => format!("{} {}", article(context.what), context.what),
    }
}

fn article(noun: &str) -> &'static str {
    if noun.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    }
}

/// The entries a context accepts for one of its kinds, panel items included.
fn accepted(context: &Context, kind: Option<usize>) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = context
        .keys
        .iter()
        .filter(|key| kind.is_none_or(|index| key.kinds.contains(index)))
        .map(|key| key.name)
        .collect();
    if context.panels {
        names.extend(crate::spec::PANEL_KINDS.iter().map(|kind| kind.keyword));
    }
    names
}

/// An entry the context does not have under any kind.
pub(crate) fn unknown_entry(
    context: &Context,
    kind: Option<usize>,
    ident: &Ident,
    extra_help: Option<&str>,
) -> syn::Error {
    let word = ident.to_string();
    let here = accepted(context, kind);
    let everywhere = accepted(context, None);
    let mut message = format!(
        "unknown {} `{word}` in {}",
        context.entry,
        describe(context, kind)
    );
    if let Some(close) = suggest(&word, here.iter().copied()) {
        message.push_str(&format!("; did you mean `{close}`?"));
    } else if let Some(close) = suggest(&word, everywhere.iter().copied()) {
        let kinds = kinds_accepting(context, close);
        message.push_str(&format!(
            "; did you mean `{close}`? It applies to {kinds} only"
        ));
    }
    message.push_str(&format!(
        "\n{}s accepted here: {}",
        context.entry,
        backticked(here)
    ));
    if let Some(help) = extra_help {
        message.push('\n');
        message.push_str(help);
    }
    syn::Error::new(ident.span(), message)
}

fn kinds_accepting(context: &Context, name: &str) -> String {
    let mut union = None::<Kinds>;
    for key in context.keys.iter().filter(|key| key.name == name) {
        union = Some(union.map_or(key.kinds, |kinds| kinds.or(key.kinds)));
    }
    let union = union.unwrap_or(Kinds::ALL);
    let names = context
        .kinds
        .iter()
        .enumerate()
        .filter(|(index, _)| union.contains(*index))
        .map(|(_, kind)| kind.keyword);
    let noun = if context.what == "panel" {
        "panels"
    } else {
        "queries"
    };
    format!("{} {noun}", backticked(names))
}

/// A key the context has, but not for the kind being configured.
pub(crate) fn wrong_kind(context: &Context, kind: usize, ident: &Ident) -> syn::Error {
    let word = ident.to_string();
    syn::Error::new(
        ident.span(),
        format!(
            "`{word}` does not apply to {}; it applies to {}\n{}s accepted here: {}",
            describe(context, Some(kind)),
            kinds_accepting(context, &word),
            context.entry,
            backticked(accepted(context, Some(kind)))
        ),
    )
}

/// A word that is not in a vocabulary.
pub(crate) fn unknown_word(vocab: &Vocab, ident: &Ident, also: &[&str]) -> syn::Error {
    let word = ident.to_string();
    let words = vocab.words.iter().map(|(word, _)| *word);
    let mut message = format!("unknown {} `{word}`", vocab.what);
    if let Some(close) = suggest(&word, words.clone().chain(also.iter().copied())) {
        message.push_str(&format!("; did you mean `{close}`?"));
    }
    message.push_str(&format!(
        "\naccepted: {}; or any expression of type `{}`, such as `({word})` for a variable",
        backticked(words.chain(also.iter().copied())),
        vocab.ty
    ));
    syn::Error::new(ident.span(), message)
}

/// A word that is not one of a fixed list of grammar keywords.
pub(crate) fn unknown_keyword<'a>(
    what: &str,
    ident: &Ident,
    candidates: impl IntoIterator<Item = &'a str> + Clone,
    help: Option<&str>,
) -> syn::Error {
    let word = ident.to_string();
    let mut message = format!("unknown {what} `{word}`");
    if let Some(close) = suggest(&word, candidates.clone()) {
        message.push_str(&format!("; did you mean `{close}`?"));
    }
    message.push_str(&format!("\naccepted: {}", backticked(candidates)));
    if let Some(help) = help {
        message.push('\n');
        message.push_str(help);
    }
    syn::Error::new(ident.span(), message)
}

/// A word that is not in this kind's vocabulary for a key, but is in the
/// vocabulary the same key has for another kind, such as `color_mode:
/// scheme;` on a stat panel.
pub(crate) fn word_of_other_kind(
    context: &Context,
    kind: usize,
    key: &Key,
    vocab: &Vocab,
    ident: &Ident,
) -> Option<syn::Error> {
    let word = ident.to_string();
    if vocab.variant(&word).is_some() {
        return None;
    }
    let other = context.keys.iter().find(|other| {
        other.name == key.name
            && !other.kinds.contains(kind)
            && matches!(&other.form, Form::Enum(_, other) if other.variant(&word).is_some())
    })?;
    let Form::Enum(_, other_vocab) = &other.form else {
        return None;
    };
    let kinds = context
        .kinds
        .iter()
        .enumerate()
        .filter(|(index, _)| other.kinds.contains(*index))
        .map(|(_, kind)| kind.keyword);
    Some(syn::Error::new(
        ident.span(),
        format!(
            "`{word}` is a {} for {} {}s, not for {}\naccepted here: {}",
            other_vocab.what,
            backticked(kinds),
            context.what,
            describe(context, Some(kind)),
            backticked(vocab.words.iter().map(|(word, _)| *word))
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_counts_transpositions_as_one_edit() {
        assert_eq!(distance("fill_opactiy", "fill_opacity"), 1);
        assert_eq!(distance("timeserie", "timeseries"), 1);
        assert_eq!(distance("", "abc"), 3);
        assert_eq!(distance("same", "same"), 0);
    }

    #[test]
    fn suggestions_reject_distant_words() {
        assert_eq!(suggest("widht", ["width", "height"]), Some("width"));
        assert_eq!(suggest("colour", ["color", "unit"]), Some("color"));
        assert_eq!(suggest("zzz", ["width", "height"]), None);
    }
}
