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

//! Interprets the value of a key: vocabulary words, units, colors, color
//! schemes, reducer lists, and threshold blocks.
//!
//! Each function takes a value that has already been read in full, so an
//! error here is recorded and never needs the parser to recover. A value
//! that is not one of the grammar's words is passed through as a Rust
//! expression of the builder argument's type.

use proc_macro2::{Delimiter, Group, Ident, Span, TokenStream};
use quote::{ToTokens, quote};

use crate::{
    diag::{suggest, unknown_word},
    lower::Cx,
    spec::{
        COLOR, COLOR_SCHEME, REDUCER, THRESHOLD_KEYS, THRESHOLD_MODE, THRESHOLD_NAMED_STEPS, UNIT,
        Vocab,
    },
    tokens::{Expr, Stop, Tokens},
};

/// `Enum::Variant` for a word of `vocab`, or the expression unchanged.
pub(crate) fn word(cx: &mut Cx, vocab: &Vocab, value: &Expr) -> Option<TokenStream> {
    word_or(cx, vocab, value, &[])
}

fn word_or(cx: &mut Cx, vocab: &Vocab, value: &Expr, also: &[&str]) -> Option<TokenStream> {
    let Some(ident) = value.single_ident() else {
        return Some(value.to_token_stream());
    };
    match vocab.variant(&ident.to_string()) {
        Some(variant) => Some(cx.variant(vocab.ty, variant, ident.span())),
        None => {
            cx.error(unknown_word(vocab, ident, also));
            None
        }
    }
}

/// A unit word, `custom(<expr>)`, or an expression of type `Unit`.
pub(crate) fn unit(cx: &mut Cx, value: &Expr) -> Option<TokenStream> {
    if let Some((name, inner)) = value.as_call()
        && name == "custom"
    {
        let ty = cx.path("Unit", name.span());
        let method = Ident::new("custom", name.span());
        return Some(quote!(#ty::#method(#inner)));
    }
    word_or(cx, &UNIT, value, &["custom(<unit>)"])
}

/// A color word, a string naming any other color, or an expression of type
/// `Color`.
pub(crate) fn color(cx: &mut Cx, value: &Expr) -> Option<TokenStream> {
    if let Some(literal) = value.single_string() {
        let ty = cx.path("Color", literal.span());
        return Some(quote!(#ty::custom(#literal)));
    }
    word_or(cx, &COLOR, value, &["\"<any color>\""])
}

/// A color scheme word, `fixed(<color>)`, `continuous(<palette>)`, or an
/// expression of type `ColorScheme`.
pub(crate) fn color_scheme(cx: &mut Cx, value: &Expr) -> Option<TokenStream> {
    if let Some((name, inner)) = value.as_call() {
        let ty = cx.path("ColorScheme", name.span());
        if name == "fixed" {
            let color = color(cx, &inner)?;
            return Some(quote!(#ty::Fixed(#color)));
        }
        if name == "continuous" {
            return Some(quote!(#ty::Continuous(::core::convert::Into::into(#inner))));
        }
    }
    word_or(
        cx,
        &COLOR_SCHEME,
        value,
        &["fixed(<color>)", "continuous(<palette>)"],
    )
}

/// `[<reducer>, …]`, or an expression yielding reducers.
pub(crate) fn reducers(cx: &mut Cx, value: &Expr) -> Option<TokenStream> {
    let Some(elements) = value.as_list(Delimiter::Bracket) else {
        return Some(value.to_token_stream());
    };
    let mut reducers = Vec::with_capacity(elements.len());
    for element in &elements {
        reducers.push(word(cx, &REDUCER, element)?);
    }
    Some(quote!([#(#reducers),*]))
}

/// One entry of a thresholds block.
enum Threshold {
    Base(TokenStream, Span),
    Step(TokenStream, TokenStream, Span),
    Named(Ident, Expr),
    Mode(TokenStream),
}

/// The thresholds a block with a `base:` starts from. Kept in one place so
/// the base-first constructor is the only line that changes if the builder
/// grows one.
fn with_base(cx: &Cx, base: &TokenStream) -> TokenStream {
    let ty = cx.path("Thresholds", Span::call_site());
    quote!(#ty::new().step(#base, ::core::option::Option::<f64>::None))
}

/// Lowers a thresholds block to an expression of type `Thresholds`.
///
/// Two grammars are accepted, but not mixed. The general one is a mandatory
/// `base: <color>;` followed by `<value> => <color>;` steps in ascending
/// order. The named form, `green: <value>;`, `yellow: …`, `red: …`, is the
/// original shorthand and keeps building exactly what it always did.
pub(crate) fn thresholds(cx: &mut Cx, keyword: &Ident, group: &Group) -> TokenStream {
    let mut tokens = Tokens::inside(group);
    let mut entries = Vec::new();
    while !tokens.is_empty() {
        if tokens.eat_punct(';') {
            continue;
        }
        match threshold_entry(cx, &mut tokens) {
            Ok(Some(entry)) => entries.push(entry),
            Ok(None) => {}
            Err(error) => {
                cx.error(error);
                tokens.recover();
            }
        }
    }

    let mut base = None;
    let mut first_step = None;
    for entry in &entries {
        match entry {
            Threshold::Base(color, span) => {
                if base.is_some() {
                    cx.error(syn::Error::new(*span, "thresholds take one `base:`"));
                } else if first_step.is_some() {
                    cx.error(syn::Error::new(
                        *span,
                        "`base:` must come before the `<value> => <color>;` steps",
                    ));
                }
                base.get_or_insert_with(|| color.clone());
            }
            Threshold::Step(_, _, span) => {
                first_step.get_or_insert(*span);
            }
            Threshold::Named(..) | Threshold::Mode(_) => {}
        }
    }
    if let (None, Some(span)) = (&base, first_step) {
        cx.error(syn::Error::new(
            span,
            "thresholds with `<value> => <color>;` steps need a `base: <color>;` first, the color below the lowest step",
        ));
    }
    let general = base.is_some() || first_step.is_some();
    for entry in &entries {
        if let (true, Threshold::Named(name, _)) = (general, entry) {
            cx.error(syn::Error::new(
                name.span(),
                format!(
                    "`{name}:` cannot be mixed with `base:` and `<value> => <color>;` steps; write `<value> => {name};`"
                ),
            ));
        }
    }

    let var = Ident::new("__panelist_thresholds", Span::mixed_site());
    let init = match &base {
        Some(base) => with_base(cx, base),
        None => {
            let ty = cx.path("Thresholds", keyword.span());
            quote!(#ty::new())
        }
    };
    let mut statements = TokenStream::new();
    for entry in entries {
        statements.extend(match entry {
            Threshold::Base(..) => continue,
            Threshold::Step(value, color, _) => {
                quote!(let #var = #var.step(#color, <f64 as ::core::convert::From<_>>::from(#value));)
            }
            Threshold::Named(name, value) => quote!(let #var = #var.#name(#value);),
            Threshold::Mode(mode) => quote!(let #var = #var.mode(#mode);),
        });
    }
    quote!({
        let #var = #init;
        #statements
        #var
    })
}

fn threshold_entry(cx: &mut Cx, tokens: &mut Tokens) -> syn::Result<Option<Threshold>> {
    if tokens.is_key_colon() {
        let key = tokens.expect_ident("a thresholds key")?;
        tokens.expect_colon(&key)?;
        let name = key.to_string();
        if !THRESHOLD_KEYS.contains(&name.as_str()) {
            let mut message = format!("unknown thresholds key `{name}`");
            if let Some(close) = suggest(&name, THRESHOLD_KEYS.iter().copied()) {
                message.push_str(&format!("; did you mean `{close}`?"));
            }
            message.push_str(
                "\naccepted: `base: <color>;`, `mode: absolute | percentage;`, and steps written `<value> => <color>;`",
            );
            return Err(syn::Error::new(key.span(), message));
        }
        let value = value(tokens, &key)?;
        return Ok(match name.as_str() {
            "base" => color(cx, &value).map(|color| Threshold::Base(color, key.span())),
            "mode" => word(cx, &THRESHOLD_MODE, &value).map(Threshold::Mode),
            _ => {
                let method = THRESHOLD_NAMED_STEPS
                    .iter()
                    .find(|(named, _)| *named == name)
                    .map_or("step", |(_, method)| *method);
                Some(Threshold::Named(Ident::new(method, key.span()), value))
            }
        });
    }

    let step = tokens.take_until(Stop::SEMI_OR_FAT_ARROW);
    if step.is_empty() {
        return Err(tokens.error("expected a threshold step `<value> => <color>;`"));
    }
    tokens.expect_fat_arrow("the threshold value")?;
    let target = tokens.take_until(Stop::SEMI);
    if target.is_empty() {
        return Err(tokens.error("expected a color after `=>`"));
    }
    tokens.expect_semi("the threshold color")?;
    let span = step
        .to_token_stream()
        .into_iter()
        .next()
        .map_or_else(Span::call_site, |tree| tree.span());
    Ok(color(cx, &target).map(|color| Threshold::Step(step.to_token_stream(), color, span)))
}

/// Reads `<expr>;` after a key, checking that the value is not empty before
/// the `;` is consumed so that recovery never skips the next entry.
pub(crate) fn value(tokens: &mut Tokens, key: &Ident) -> syn::Result<Expr> {
    let value = tokens.take_until(Stop::SEMI);
    if value.is_empty() {
        return Err(tokens.error(format!("expected a value for `{key}`")));
    }
    tokens.expect_semi(&format!("the value of `{key}`"))?;
    Ok(value)
}
