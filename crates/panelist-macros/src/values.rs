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
        COLOR, COLOR_SCHEME, CONTINUOUS_PALETTE, CURRENCY, REDUCER, SPECIAL_VALUE, THRESHOLD_KEYS,
        THRESHOLD_MODE, UNIT, Vocab,
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

/// A unit word, `currency(<currency>)`, `custom(<expr>)`, or an expression
/// of type `Unit`.
pub(crate) fn unit(cx: &mut Cx, value: &Expr) -> Option<TokenStream> {
    if let Some((name, inner)) = value.as_call() {
        let ty = cx.path("Unit", name.span());
        if name == "custom" {
            let method = Ident::new("custom", name.span());
            return Some(quote!(#ty::#method(#inner)));
        }
        if name == "currency" {
            let currency = word(cx, &CURRENCY, &inner)?;
            return Some(quote!(#ty::Currency(#currency)));
        }
    }
    word_or(
        cx,
        &UNIT,
        value,
        &["currency(<currency>)", "custom(<unit>)"],
    )
}

/// A special-value word such as `null`, or an expression of type
/// `SpecialValue`.
pub(crate) fn special_value(cx: &mut Cx, value: &Expr) -> Option<TokenStream> {
    word(cx, &SPECIAL_VALUE, value)
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
        if name == "shades" {
            let color = color(cx, &inner)?;
            return Some(quote!(#ty::Shades(#color)));
        }
        if name == "continuous" {
            let palette = word(cx, &CONTINUOUS_PALETTE, &inner)?;
            return Some(quote!(#ty::Continuous(#palette)));
        }
    }
    word_or(
        cx,
        &COLOR_SCHEME,
        value,
        &["fixed(<color>)", "shades(<color>)", "continuous(<palette>)"],
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
    Mode(TokenStream, Span),
    Step(TokenStream, TokenStream),
}

/// Lowers a thresholds block to an expression of type `Thresholds`, or
/// `None` when it has no base color and that has been reported.
///
/// A threshold set always has a base color, the color below the lowest
/// step. It is written either in parentheses, `thresholds(<color>) { … }`,
/// or as `base: <color>;` inside the block. Steps are `<value> => <color>;`,
/// which takes any color, or `<color word>: <value>;`. `mode: percentage;`
/// switches from absolute values to percentages.
pub(crate) fn thresholds(
    cx: &mut Cx,
    keyword: &Ident,
    base: Option<&Expr>,
    group: &Group,
) -> Option<TokenStream> {
    let mut tokens = Tokens::inside(group);
    let mut entries = Vec::new();
    // A rejected entry may have been the base, so a missing base is only
    // reported for a block that is otherwise clean.
    let mut rejected = false;
    while !tokens.is_empty() {
        if tokens.eat_punct(';') {
            continue;
        }
        match threshold_entry(cx, &mut tokens) {
            Ok(Some(entry)) => entries.push(entry),
            Ok(None) => rejected = true,
            Err(error) => {
                rejected = true;
                cx.error(error);
                tokens.recover();
            }
        }
    }

    let mut base = base.and_then(|base| color(cx, base));
    let mut written_base = None;
    let mut mode = None;
    let mut steps = TokenStream::new();
    let var = Ident::new("__panelist_thresholds", Span::mixed_site());
    for entry in entries {
        match entry {
            Threshold::Base(color, span) => {
                if base.is_some() || written_base.is_some() {
                    cx.error(syn::Error::new(span, "thresholds take one base color"));
                } else {
                    written_base = Some(color);
                }
            }
            Threshold::Mode(value, span) => {
                if mode.is_some() {
                    cx.error(syn::Error::new(span, "thresholds take one `mode:`"));
                } else {
                    mode = Some(value);
                }
            }
            Threshold::Step(value, color) => steps.extend(quote!(
                let #var = #var.step(<f64 as ::core::convert::From<_>>::from(#value), #color);
            )),
        }
    }
    base = base.or(written_base);
    let ty = cx.path("Thresholds", keyword.span());
    let init = match (base, mode) {
        (Some(base), None) => quote!(#ty::absolute(#base)),
        (Some(base), Some(mode)) => quote!(#ty::new(#mode, #base)),
        (None, _) => {
            if !rejected {
                cx.error(syn::Error::new(
                    keyword.span(),
                    "thresholds need a base color, the color below the lowest step: write `thresholds(<color>) { … }` or put `base: <color>;` in the block",
                ));
            }
            return None;
        }
    };
    Some(quote!({
        let #var = #init;
        #steps
        #var
    }))
}

fn threshold_entry(cx: &mut Cx, tokens: &mut Tokens) -> syn::Result<Option<Threshold>> {
    if tokens.is_key_colon() {
        let key = tokens.expect_ident("a thresholds key")?;
        tokens.expect_colon(&key)?;
        let name = key.to_string();
        let value = value(tokens, &key)?;
        return Ok(match name.as_str() {
            "base" => color(cx, &value).map(|color| Threshold::Base(color, key.span())),
            "mode" => {
                word(cx, &THRESHOLD_MODE, &value).map(|mode| Threshold::Mode(mode, key.span()))
            }
            _ => match COLOR.variant(&name) {
                Some(variant) => Some(Threshold::Step(
                    value.to_token_stream(),
                    cx.variant(COLOR.ty, variant, key.span()),
                )),
                None => {
                    let candidates = THRESHOLD_KEYS
                        .iter()
                        .copied()
                        .chain(COLOR.words.iter().map(|(word, _)| *word));
                    let mut message = format!("unknown thresholds key `{name}`");
                    if let Some(close) = suggest(&name, candidates) {
                        message.push_str(&format!("; did you mean `{close}`?"));
                    }
                    message.push_str(
                        "\naccepted: `base: <color>;`, `mode: absolute | percentage;`, steps written `<value> => <color>;`, and `<color word>: <value>;` for `green`, `yellow`, `red`, `blue`, `orange`, or `purple`",
                    );
                    cx.error(syn::Error::new(key.span(), message));
                    None
                }
            },
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
    Ok(color(cx, &target).map(|color| Threshold::Step(step.to_token_stream(), color)))
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
