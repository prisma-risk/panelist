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

//! Reads each block against its [`Context`] and lowers every entry to one
//! typed builder call.
//!
//! A block becomes a sequence of `let builder = builder.method(…);`
//! statements on a local bound with `Span::mixed_site()`, so user
//! expressions can never see or shadow it. The method name carries the
//! span of the key that produced it and each argument keeps the user's own
//! tokens, so compiler errors land on the offending line.

use std::ptr;

use proc_macro2::{Delimiter, Group, Ident, Span, TokenStream};
use quote::{ToTokens, quote};

use crate::{
    diag::{Diagnostics, describe, unknown_entry, unknown_keyword, word_of_other_kind, wrong_kind},
    spec::{
        CELL_BLOCKS, CELL_TYPE, Context, DASHBOARD, Form, Key, MATCHERS, Matcher, PANEL,
        PANEL_KINDS, QUERY, QUERY_KINDS, QUERY_MACROS, ROW, SORT_DIRECTION, STACKING_MODE,
        TRANSFORM_SCOPES, TRANSFORMS, TransformForm, VARIABLE,
    },
    tokens::{Expr, Stop, Tokens},
    values::{self, value},
};

/// State shared by one expansion.
#[derive(Debug)]
pub(crate) struct Cx {
    /// The path of the `panelist` crate, as `$crate` from the wrapper.
    krate: TokenStream,
    diagnostics: Diagnostics,
}

impl Cx {
    /// `panelist::Name`, with the last segment at `span`.
    pub(crate) fn path(&self, name: &str, span: Span) -> TokenStream {
        let krate = &self.krate;
        let name = Ident::new(name, span);
        quote!(#krate::#name)
    }

    /// `panelist::Enum::Variant`, with the variant at `span`.
    pub(crate) fn variant(&self, ty: &str, variant: &str, span: Span) -> TokenStream {
        let ty = self.path(ty, span);
        let variant = Ident::new(variant, span);
        quote!(#ty::#variant)
    }

    pub(crate) fn error(&mut self, error: syn::Error) {
        self.diagnostics.push(error);
    }
}

fn local(name: &str) -> Ident {
    Ident::new(name, Span::mixed_site())
}

/// `let receiver = receiver.method(arguments);`, with the method at `span`.
fn call(receiver: &Ident, span: Span, method: &str, arguments: impl ToTokens) -> TokenStream {
    let method = Ident::new(method, span);
    quote!(let #receiver = #receiver.#method(#arguments);)
}

/// Expands a `dashboard!` invocation. The input starts with the `panelist`
/// crate path and a `;`, which the `macro_rules!` wrapper supplies.
pub(crate) fn dashboard(input: TokenStream) -> TokenStream {
    let mut tokens = Tokens::new(input, Span::call_site());
    let krate = tokens.take_until(Stop::SEMI).to_token_stream();
    tokens.eat_punct(';');
    let mut cx = Cx {
        krate,
        diagnostics: Diagnostics::default(),
    };

    let title = match title(&mut tokens) {
        Ok(title) => title.to_token_stream(),
        Err(error) => {
            cx.error(error);
            quote!("")
        }
    };
    let receiver = local(DASHBOARD.var);
    let statements = body(&mut cx, &DASHBOARD, None, &receiver, &mut tokens);
    let dashboard = cx.path("Dashboard", Span::call_site());
    let errors = cx.diagnostics.into_compile_errors();
    quote!({
        #errors
        let #receiver = #dashboard::new(#title);
        #statements
        #receiver
    })
}

fn title(tokens: &mut Tokens) -> syn::Result<Expr> {
    if !(tokens.is_ident("title") && tokens.is_key_colon()) {
        return Err(tokens.error("a dashboard starts with `title: <expr>;`"));
    }
    let key = tokens.expect_ident("`title`")?;
    tokens.expect_colon(&key)?;
    value(tokens, &key)
}

/// Lowers every entry of a block to statements on `receiver`.
fn body(
    cx: &mut Cx,
    context: &'static Context,
    kind: Option<usize>,
    receiver: &Ident,
    tokens: &mut Tokens,
) -> TokenStream {
    let mut statements = TokenStream::new();
    while !tokens.is_empty() {
        if tokens.eat_punct(';') {
            continue;
        }
        match entry(cx, context, kind, receiver, tokens) {
            Ok(entry) => statements.extend(entry),
            Err(error) => {
                cx.error(error);
                tokens.recover();
            }
        }
    }
    statements
}

/// Lowers a nested `{ … }` block, binding its builder to `context.var`.
fn block(
    cx: &mut Cx,
    context: &'static Context,
    kind: Option<usize>,
    init: TokenStream,
    group: &Group,
    finish: TokenStream,
) -> TokenStream {
    let receiver = local(context.var);
    let statements = body(cx, context, kind, &receiver, &mut Tokens::inside(group));
    quote!({
        let #receiver = #init;
        #statements
        #receiver #finish
    })
}

fn entry(
    cx: &mut Cx,
    context: &'static Context,
    kind: Option<usize>,
    receiver: &Ident,
    tokens: &mut Tokens,
) -> syn::Result<TokenStream> {
    let name = tokens.expect_ident(&format!(
        "a {} of {}",
        context.entry,
        describe(context, kind)
    ))?;
    if context.panels
        && let Some(index) = PANEL_KINDS.iter().position(|kind| name == kind.keyword)
    {
        return panel(cx, receiver, index, &name, tokens);
    }
    if ptr::eq(context, &DASHBOARD) && name == "title" {
        return Err(syn::Error::new(
            name.span(),
            "the dashboard title is set once, by the first item `title: <expr>;`",
        ));
    }
    let key = find(context, kind, &name)?;
    lower(cx, context, kind, receiver, &name, key, tokens)
}

/// The key of a context with this name that applies to `kind`.
fn find(context: &'static Context, kind: Option<usize>, name: &Ident) -> syn::Result<&'static Key> {
    let mut named = context
        .keys
        .iter()
        .filter(|key| *name == key.name)
        .peekable();
    let Some(first) = named.peek().copied() else {
        return Err(unknown_entry(context, kind, name, None));
    };
    match kind {
        None => Ok(first),
        Some(index) => named
            .find(|key| key.kinds.contains(index))
            .ok_or_else(|| wrong_kind(context, index, name)),
    }
}

fn lower(
    cx: &mut Cx,
    context: &'static Context,
    kind: Option<usize>,
    receiver: &Ident,
    name: &Ident,
    key: &'static Key,
    tokens: &mut Tokens,
) -> syn::Result<TokenStream> {
    let span = name.span();
    let call = |method: &str, arguments: TokenStream| call(receiver, span, method, arguments);
    let called = |method: &str, arguments: Option<TokenStream>| {
        arguments.map_or_else(TokenStream::new, |arguments| call(method, arguments))
    };
    Ok(match &key.form {
        Form::Expr(method) => {
            tokens.expect_colon(name)?;
            call(method, value(tokens, name)?.to_token_stream())
        }
        Form::Word(method) => call(method, value(tokens, name)?.to_token_stream()),
        Form::Enum(method, vocab) => {
            tokens.expect_colon(name)?;
            let value = value(tokens, name)?;
            if let (Some(word), Some(kind)) = (value.single_ident(), kind)
                && let Some(error) = word_of_other_kind(context, kind, key, vocab, word)
            {
                cx.error(error);
                return Ok(TokenStream::new());
            }
            called(method, values::word(cx, vocab, &value))
        }
        Form::Unit(method) => {
            tokens.expect_colon(name)?;
            let value = value(tokens, name)?;
            called(method, values::unit(cx, &value))
        }
        Form::Color(method) => {
            tokens.expect_colon(name)?;
            let value = value(tokens, name)?;
            called(method, values::color(cx, &value))
        }
        Form::ColorScheme(method) => {
            tokens.expect_colon(name)?;
            let value = value(tokens, name)?;
            called(method, values::color_scheme(cx, &value))
        }
        Form::Reducers(method) => {
            tokens.expect_colon(name)?;
            let value = value(tokens, name)?;
            called(method, values::reducers(cx, &value))
        }
        Form::Stacking(method) => {
            tokens.expect_colon(name)?;
            let value = value(tokens, name)?;
            let stacking = match value.single_ident() {
                Some(word) => values::word(cx, &STACKING_MODE, &value).map(|mode| {
                    let ty = cx.path("Stacking", word.span());
                    quote!(#ty::new(#mode))
                }),
                None => Some(value.to_token_stream()),
            };
            called(method, stacking)
        }
        Form::Grid(method) => {
            tokens.expect_colon(name)?;
            let value = value(tokens, name)?;
            called(
                method,
                list_constructor(cx, &value, "GridPos", &["x", "y", "width", "height"]),
            )
        }
        Form::TimeRange(method) => {
            tokens.expect_colon(name)?;
            let value = value(tokens, name)?;
            called(
                method,
                list_constructor(cx, &value, "TimeRange", &["from", "to"]),
            )
        }
        Form::SortBy(method) => {
            tokens.expect_colon(name)?;
            let value = value(tokens, name)?;
            match value.as_list(Delimiter::Parenthesis).as_deref() {
                Some([field, direction]) => {
                    let direction = values::word(cx, &SORT_DIRECTION, direction);
                    called(
                        method,
                        direction.map(|direction| quote!(#field, #direction)),
                    )
                }
                _ => {
                    cx.error(syn::Error::new_spanned(
                        &value,
                        format!("expected `{name}: (<field>, asc | desc);`"),
                    ));
                    TokenStream::new()
                }
            }
        }
        Form::Block { method, ty, body } => {
            let group =
                tokens.expect_group(Delimiter::Brace, &format!("`{{ … }}` after `{name}`"))?;
            tokens.eat_punct(';');
            let init = {
                let ty = cx.path(ty, span);
                quote!(#ty::new())
            };
            call(
                method,
                block(cx, body, None, init, &group, TokenStream::new()),
            )
        }
        Form::Inline(body) => {
            let group =
                tokens.expect_group(Delimiter::Brace, &format!("`{{ … }}` after `{name}`"))?;
            tokens.eat_punct(';');
            self::body(cx, body, None, receiver, &mut Tokens::inside(&group))
        }
        Form::Thresholds(method) => {
            let group =
                tokens.expect_group(Delimiter::Brace, &format!("`{{ … }}` after `{name}`"))?;
            tokens.eat_punct(';');
            call(method, values::thresholds(cx, name, &group))
        }
        Form::Query(method) => {
            tokens.expect_colon(name)?;
            call(method, query(cx, name, tokens)?)
        }
        Form::Cell { bare } => {
            tokens.expect_colon(name)?;
            cell(cx, receiver, name, bare, tokens)?
        }
        Form::Json(method) => {
            let field = tokens.take_until(Stop::SEMI_OR_COLON);
            if field.is_empty() {
                return Err(tokens.error(format!("expected `{name} <key>: <json>;`")));
            }
            tokens.expect_colon(name)?;
            let value = value(tokens, name)?;
            let json = cx.path("__private", span);
            call(method, quote!(#field, #json::serde_json::json!(#value)))
        }
        Form::Pair { method, ty, body } => {
            let left = tokens.take_until(Stop::SEMI_OR_FAT_ARROW);
            if left.is_empty() {
                return Err(tokens.error(format!("expected `{name} <expr> => <expr>`")));
            }
            tokens.expect_fat_arrow(&format!("the first value of `{name}`"))?;
            let right = tokens.take_until(Stop::SEMI_OR_BRACE);
            if right.is_empty() {
                return Err(tokens.error(format!("expected a value after `{name} … =>`")));
            }
            let init = {
                let ty = cx.path(ty, span);
                quote!(#ty::new(#left, #right))
            };
            if tokens.is_group(Delimiter::Brace) {
                let group = tokens.expect_group(Delimiter::Brace, "`{ … }`")?;
                tokens.eat_punct(';');
                call(
                    method,
                    block(cx, body, None, init, &group, TokenStream::new()),
                )
            } else {
                tokens.expect_semi(&format!("`{name} … => …`"))?;
                call(method, init)
            }
        }
        Form::Override => call("override_field", field_override(cx, name, tokens)?),
        Form::Transform => called("transform", transform(cx, name, tokens)?),
        Form::Rename => {
            let from = tokens.take_until(Stop::SEMI_OR_FAT_ARROW);
            if from.is_empty() {
                return Err(tokens.error("expected `rename <from> => <to>;`"));
            }
            tokens.expect_fat_arrow("the field to rename")?;
            let to = value(tokens, name)?;
            call("rename", quote!(#from, #to))
        }
        Form::SeriesQuery => {
            let reference = tokens.take_until(Stop::SEMI_OR_COLON);
            if reference.is_empty() {
                return Err(
                    tokens.error("expected `query <ref id>;` or `query <ref id>: <reducer>;`")
                );
            }
            if tokens.is_colon() {
                tokens.expect_colon(name)?;
                let value = value(tokens, name)?;
                let reducer = values::word(cx, &crate::spec::REDUCER, &value);
                called(
                    "query_with",
                    reducer.map(|reducer| quote!(#reference, #reducer)),
                )
            } else {
                tokens.expect_semi("the query reference")?;
                call("query", reference.to_token_stream())
            }
        }
        Form::TimeField => {
            let reference = tokens.take_until(Stop::SEMI_OR_COLON);
            if reference.is_empty() {
                return Err(tokens.error("expected `time_field <ref id>: <field>;`"));
            }
            tokens.expect_colon(name)?;
            let field = value(tokens, name)?;
            call("time_field", quote!(#reference, #field))
        }
        Form::Row => row(cx, receiver, name, tokens)?,
        Form::Variable => variable(cx, receiver, name, tokens)?,
    })
}

/// `Type::new(a, b, …)` for a bracketed list of exactly `fields`, or the
/// expression unchanged.
fn list_constructor(cx: &mut Cx, value: &Expr, ty: &str, fields: &[&str]) -> Option<TokenStream> {
    let Some(elements) = value.as_list(Delimiter::Bracket) else {
        return Some(value.to_token_stream());
    };
    if elements.len() != fields.len() {
        let expected = fields
            .iter()
            .map(|field| format!("<{field}>"))
            .collect::<Vec<_>>()
            .join(", ");
        cx.error(syn::Error::new_spanned(
            value,
            format!("expected [{expected}], found {} elements", elements.len()),
        ));
        return None;
    }
    let ty = cx.path(ty, Span::call_site());
    Some(quote!(#ty::new(#(#elements),*)))
}

/// What a panel or row is started from.
enum Head {
    Title(Expr),
    Base(Expr),
}

/// `<title> { … }` or `from <builder> { … }` after a panel or row keyword.
fn head(tokens: &mut Tokens, keyword: &Ident, what: &str) -> syn::Result<(Head, Group)> {
    if tokens.is_ident("from") {
        tokens.next();
        let base = tokens.take_until(Stop::SEMI_OR_BRACE);
        if base.is_empty() {
            return Err(tokens.error(format!("expected a {what} builder expression after `from`")));
        }
        let group = tokens.expect_group(
            Delimiter::Brace,
            &format!("`{{ … }}` after the base {what} builder"),
        )?;
        return Ok((Head::Base(base), group));
    }
    let title = tokens.take_until(Stop::TITLE);
    if tokens.is_ident("from") {
        return Err(syn::Error::new_spanned(
            &title,
            format!(
                "a {what} started `from` a builder keeps that builder's title; remove this title, or drop `from …` and build the {what} here"
            ),
        ));
    }
    if title.is_empty() {
        return Err(tokens.error(format!(
            "expected a title expression or `from <builder>` after `{keyword}`"
        )));
    }
    let group = tokens.expect_group(
        Delimiter::Brace,
        &format!("`{{ … }}` after the {what} title"),
    )?;
    Ok((Head::Title(title), group))
}

fn panel(
    cx: &mut Cx,
    receiver: &Ident,
    index: usize,
    keyword: &Ident,
    tokens: &mut Tokens,
) -> syn::Result<TokenStream> {
    let kind = &PANEL_KINDS[index];
    let (head, group) = head(tokens, keyword, &format!("`{}` panel", kind.keyword))?;
    tokens.eat_punct(';');
    let ty = cx.path(kind.ty, keyword.span());
    let var = local(PANEL.var);
    let init = match head {
        Head::Title(title) => quote!(#ty::new(#title)),
        Head::Base(base) => quote!({
            let #var: #ty = #base;
            #var
        }),
    };
    let panel = block(cx, &PANEL, Some(index), init, &group, TokenStream::new());
    Ok(call(receiver, keyword.span(), "panel", panel))
}

fn row(
    cx: &mut Cx,
    receiver: &Ident,
    keyword: &Ident,
    tokens: &mut Tokens,
) -> syn::Result<TokenStream> {
    let (head, group) = head(tokens, keyword, "row")?;
    tokens.eat_punct(';');
    let ty = cx.path("Row", keyword.span());
    let var = local(ROW.var);
    let init = match head {
        Head::Title(title) => quote!(#ty::new(#title)),
        Head::Base(base) => quote!({
            let #var: #ty = #base;
            #var
        }),
    };
    let row = block(cx, &ROW, None, init, &group, TokenStream::new());
    Ok(call(receiver, keyword.span(), "row", row))
}

fn variable(
    cx: &mut Cx,
    receiver: &Ident,
    keyword: &Ident,
    tokens: &mut Tokens,
) -> syn::Result<TokenStream> {
    let name = tokens.take_until(Stop::SEMI_OR_BRACE);
    if name.is_empty() {
        return Err(tokens.error("expected `variable <name> { … }`"));
    }
    let group = tokens.expect_group(Delimiter::Brace, "`{ … }` after the variable name")?;
    tokens.eat_punct(';');
    let init = {
        let ty = cx.path("VariableBuilder", keyword.span());
        quote!(#ty::new(#name))
    };
    let variable = block(cx, &VARIABLE, None, init, &group, quote!(.build()));
    Ok(call(receiver, keyword.span(), "variable", variable))
}

/// The value of `query:`: a `promql!`, `loki!`, or `loki_query!` call with
/// an optional options block, or any expression convertible to a query.
fn query(cx: &mut Cx, key: &Ident, tokens: &mut Tokens) -> syn::Result<TokenStream> {
    if tokens.is_macro_call_then_block_or_semi()
        && let Some(name) = tokens.peek_ident().cloned()
    {
        let known = QUERY_MACROS
            .iter()
            .find(|(macro_name, _)| name == *macro_name);
        match known {
            Some((_, index)) => {
                tokens.next();
                tokens.next();
                let arguments = tokens
                    .expect_group(Delimiter::Parenthesis, "`(…)`")?
                    .stream();
                let init = {
                    let ty = cx.path(QUERY_KINDS[*index].ty, name.span());
                    quote!(#ty::new(#arguments))
                };
                if tokens.is_group(Delimiter::Brace) {
                    let group = tokens.expect_group(Delimiter::Brace, "`{ … }`")?;
                    tokens.eat_punct(';');
                    return Ok(block(
                        cx,
                        &QUERY,
                        Some(*index),
                        init,
                        &group,
                        TokenStream::new(),
                    ));
                }
                tokens.expect_semi(&format!("the value of `{key}`"))?;
                return Ok(init);
            }
            None if !tokens.is_punct_after_macro_call(';') => {
                // `name!(…) { … }` with a macro that has no options block.
                let candidates = QUERY_MACROS.iter().map(|(macro_name, _)| *macro_name);
                return Err(unknown_keyword(
                    "query macro",
                    &name,
                    candidates,
                    Some(
                        "only these take a `{ … }` options block; write any other query as `query: <expr>;`",
                    ),
                ));
            }
            None => {}
        }
    }
    Ok(value(tokens, key)?.to_token_stream())
}

/// The value of `cell:`: a cell type, a cell type with an options block, or
/// any expression convertible to a table cell.
fn cell(
    cx: &mut Cx,
    receiver: &Ident,
    key: &Ident,
    bare: &str,
    tokens: &mut Tokens,
) -> syn::Result<TokenStream> {
    if tokens.is_ident_then_brace() {
        let word = tokens.expect_ident("a cell type")?;
        let group = tokens.expect_group(Delimiter::Brace, "`{ … }`")?;
        tokens.eat_punct(';');
        let Some((_, ty, body)) = CELL_BLOCKS.iter().find(|(cell, _, _)| word == *cell) else {
            cx.error(unknown_keyword(
                "cell type with options",
                &word,
                CELL_BLOCKS.iter().map(|(cell, _, _)| *cell),
                Some("other cell types take no options block"),
            ));
            return Ok(TokenStream::new());
        };
        let init = {
            let ty = cx.path(ty, word.span());
            quote!(#ty::new())
        };
        let cell = block(cx, body, None, init, &group, TokenStream::new());
        return Ok(call(receiver, key.span(), "cell", cell));
    }
    let value = value(tokens, key)?;
    if value.single_ident().is_some() {
        let cell = values::word(cx, &CELL_TYPE, &value);
        return Ok(cell.map_or_else(TokenStream::new, |cell| {
            call(receiver, key.span(), bare, cell)
        }));
    }
    Ok(call(receiver, key.span(), "cell", value.to_token_stream()))
}

/// `override <matcher> { … }`, lowered to a `FieldOverride` expression.
fn field_override(cx: &mut Cx, keyword: &Ident, tokens: &mut Tokens) -> syn::Result<TokenStream> {
    let matcher = tokens.expect_ident("a field matcher such as `field(\"name\")`")?;
    let Some((_, form)) = MATCHERS.iter().find(|(name, _)| matcher == *name) else {
        return Err(unknown_keyword(
            "field matcher",
            &matcher,
            MATCHERS.iter().map(|(name, _)| *name),
            None,
        ));
    };
    let ty = cx.path("FieldOverride", keyword.span());
    let init = match form {
        Matcher::Call(constructor) => {
            let arguments = tokens
                .expect_group(Delimiter::Parenthesis, &format!("`(…)` after `{matcher}`"))?
                .stream();
            let constructor = Ident::new(constructor, matcher.span());
            quote!(#ty::#constructor(#arguments))
        }
        Matcher::Expr(constructor) => {
            let names = tokens.take_until(Stop::SEMI_OR_BRACE);
            if names.is_empty() {
                return Err(tokens.error(format!("expected field names after `{matcher}`")));
            }
            let constructor = Ident::new(constructor, matcher.span());
            quote!(#ty::#constructor(#names))
        }
        Matcher::Bare(constructor) => {
            let constructor = Ident::new(constructor, matcher.span());
            quote!(#ty::#constructor())
        }
    };
    let group = tokens.expect_group(Delimiter::Brace, "`{ … }` after the field matcher")?;
    tokens.eat_punct(';');
    Ok(block(
        cx,
        &crate::spec::OVERRIDE,
        None,
        init,
        &group,
        TokenStream::new(),
    ))
}

/// `transform …`, lowered to an expression convertible to a transformation,
/// or `None` when an argument was rejected and has been reported.
fn transform(
    cx: &mut Cx,
    keyword: &Ident,
    tokens: &mut Tokens,
) -> syn::Result<Option<TokenStream>> {
    if tokens.is_colon() {
        tokens.expect_colon(keyword)?;
        return Ok(Some(value(tokens, keyword)?.to_token_stream()));
    }
    let name = tokens
        .expect_ident("a transformation such as `organize { … }`, or `transform: <expr>;`")?;
    let Some((_, form)) = TRANSFORMS.iter().find(|(transform, _)| name == *transform) else {
        return Err(unknown_keyword(
            "transformation",
            &name,
            TRANSFORMS.iter().map(|(transform, _)| *transform),
            Some("any other transformation is written `transform: <expr>;`"),
        ));
    };

    let var = local("__panelist_transform");
    let mut statements = TokenStream::new();
    let mut ended_in_block = false;
    let mut accepted = true;
    match form {
        TransformForm::CallWithOption { ty, method, vocab } => {
            let arguments = arguments(tokens, &name, "(<field>) or (<field>, <mode>)")?;
            let ty = cx.path(ty, name.span());
            match arguments.as_slice() {
                [field] => statements.extend(quote!(let #var = #ty::new(#field);)),
                [field, option] => {
                    statements.extend(quote!(let #var = #ty::new(#field);));
                    if let Some(option) = values::word(cx, vocab, option) {
                        statements.extend(call(&var, name.span(), method, option));
                    }
                }
                _ => return Err(argument_count(&name, "(<field>) or (<field>, <mode>)")),
            }
        }
        TransformForm::CallWithArgument { ty, vocab } => {
            let arguments = arguments(tokens, &name, "(<field>, asc | desc)")?;
            let [field, word] = arguments.as_slice() else {
                return Err(argument_count(&name, "(<field>, asc | desc)"));
            };
            let ty = cx.path(ty, name.span());
            match values::word(cx, vocab, word) {
                Some(word) => statements.extend(quote!(let #var = #ty::new(#field, #word);)),
                None => accepted = false,
            }
        }
        TransformForm::Block { ty, body, bare } => {
            let init = {
                let ty = cx.path(ty, name.span());
                quote!(#ty::new())
            };
            if tokens.is_group(Delimiter::Brace) {
                let group = tokens.expect_group(Delimiter::Brace, "`{ … }`")?;
                let receiver = local(body.var);
                let inner = self::body(cx, body, None, &receiver, &mut Tokens::inside(&group));
                statements.extend(quote!(
                    let #var = {
                        let #receiver = #init;
                        #inner
                        #receiver
                    };
                ));
                ended_in_block = true;
            } else if *bare {
                statements.extend(quote!(let #var = #init;));
            } else {
                return Err(tokens.error(format!("expected `{{ … }}` after `{name}`")));
            }
        }
    }

    if tokens.is_ident("only") {
        tokens.next();
        let scope = tokens.expect_ident("`ref_id(…)` or `frame_name(…)` after `only`")?;
        let Some((_, method)) = TRANSFORM_SCOPES.iter().find(|(word, _)| scope == *word) else {
            return Err(unknown_keyword(
                "transformation scope",
                &scope,
                TRANSFORM_SCOPES.iter().map(|(word, _)| *word),
                None,
            ));
        };
        let argument = tokens
            .expect_group(Delimiter::Parenthesis, &format!("`(…)` after `{scope}`"))?
            .stream();
        statements.extend(call(&var, scope.span(), method, argument));
        tokens.expect_semi(&format!("`only {scope}(…)`"))?;
    } else if ended_in_block {
        tokens.eat_punct(';');
    } else {
        tokens.expect_semi(&format!("`transform {name}`"))?;
    }
    Ok(accepted.then(|| {
        quote!({
            #statements
            #var
        })
    }))
}

fn arguments(tokens: &mut Tokens, name: &Ident, expected: &str) -> syn::Result<Vec<Expr>> {
    let group = tokens.expect_group(Delimiter::Parenthesis, &format!("`{name}{expected}`"))?;
    Ok(crate::tokens::split_commas(group.stream()))
}

fn argument_count(name: &Ident, expected: &str) -> syn::Error {
    syn::Error::new(name.span(), format!("expected `{name}{expected}`"))
}
