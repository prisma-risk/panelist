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
        ANNOTATION, CELL_BLOCKS, CELL_TYPE, COLUMN, Context, DASHBOARD, Form, KEYED_TABLE_INDEX,
        Key, MAPPING, MATCHERS, Matcher, PANEL, PANEL_KINDS, QUERY, QUERY_KINDS, QUERY_MACROS,
        REDUCER, ROW, SORT_DIRECTION, STACKING_MODE, TRANSFORM_SCOPES, TRANSFORMS, TransformForm,
        VARIABLE,
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
    /// Statements for the keyed table a `keyed_table` panel becomes, which
    /// run after every table statement. `Some` only while one is lowered.
    keyed: Option<TokenStream>,
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
        keyed: None,
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
            let base = if tokens.is_group(Delimiter::Parenthesis) {
                let group = tokens.expect_group(Delimiter::Parenthesis, "`(<base color>)`")?;
                Some(Expr::from_stream(group.stream()))
            } else {
                None
            };
            let group =
                tokens.expect_group(Delimiter::Brace, &format!("`{{ … }}` after `{name}`"))?;
            tokens.eat_punct(';');
            called(method, values::thresholds(cx, name, base.as_ref(), &group))
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
            let (left, right) = pair(tokens, name)?;
            let init = {
                let ty = cx.path(ty, span);
                quote!(#ty::new(#left, #right))
            };
            call(method, pair_block(cx, body, init, tokens, name)?)
        }
        Form::Mapping(method) => {
            let (left, right) = pair(tokens, name)?;
            let mapping = mapping(cx, span, &left, &right);
            let block = pair_block(
                cx,
                &MAPPING,
                mapping.clone().unwrap_or_default(),
                tokens,
                name,
            )?;
            called(method, mapping.map(|_| block))
        }
        Form::Annotation => {
            if tokens.is_colon() {
                tokens.expect_colon(name)?;
                call("annotation", value(tokens, name)?.to_token_stream())
            } else {
                let (left, right) = pair(tokens, name)?;
                let init = {
                    let ty = cx.path("PrometheusAnnotation", span);
                    quote!(#ty::new(#left, #right))
                };
                call(
                    "annotation",
                    pair_block(cx, &ANNOTATION, init, tokens, name)?,
                )
            }
        }
        Form::Repeat(method) => {
            tokens.expect_colon(name)?;
            let value = value(tokens, name)?;
            called(method, repeat(cx, &value))
        }
        Form::Keyed(method) => {
            tokens.expect_colon(name)?;
            let value = value(tokens, name)?;
            keyed(cx, call_on_keyed(span, method, value.to_token_stream()))
        }
        Form::Column { trend } => {
            let column = column(cx, name, *trend, tokens)?;
            keyed(
                cx,
                column.map_or_else(TokenStream::new, |column| {
                    call_on_keyed(span, "column", column)
                }),
            )
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

/// Reads `<a> => <b>` up to a `;` or a `{ … }` block, which stays unread.
fn pair(tokens: &mut Tokens, name: &Ident) -> syn::Result<(Expr, Expr)> {
    let left = tokens.take_until(Stop::SEMI_OR_FAT_ARROW);
    if left.is_empty() {
        return Err(tokens.error(format!("expected `{name} <expr> => <expr>`")));
    }
    tokens.expect_fat_arrow(&format!("the first value of `{name}`"))?;
    let right = tokens.take_until(Stop::SEMI_OR_BRACE);
    if right.is_empty() {
        return Err(tokens.error(format!("expected a value after `{name} … =>`")));
    }
    Ok((left, right))
}

/// `init` with the optional `{ … }` block after a pair applied to it, or
/// `init` alone when the pair ends in `;`.
fn pair_block(
    cx: &mut Cx,
    body: &'static Context,
    init: TokenStream,
    tokens: &mut Tokens,
    name: &Ident,
) -> syn::Result<TokenStream> {
    if tokens.is_group(Delimiter::Brace) {
        let group = tokens.expect_group(Delimiter::Brace, "`{ … }`")?;
        tokens.eat_punct(';');
        Ok(block(cx, body, None, init, &group, TokenStream::new()))
    } else {
        tokens.expect_semi(&format!("`{name} … => …`"))?;
        Ok(init)
    }
}

/// The `ValueMapping` a `mapping <match> => <text>` builds: `range(<from>,
/// <to>)`, `regex(<pattern>)`, and `special(<value>)` select those kinds,
/// and anything else is an exact value.
fn mapping(cx: &mut Cx, span: Span, matched: &Expr, text: &Expr) -> Option<TokenStream> {
    let ty = cx.path("ValueMapping", span);
    let kind = match matched.as_call() {
        Some((name, inner)) if name == "range" => {
            let bounds = crate::tokens::split_commas(inner.to_token_stream());
            let [from, to] = bounds.as_slice() else {
                cx.error(syn::Error::new_spanned(
                    matched,
                    "expected `range(<from>, <to>)`",
                ));
                return None;
            };
            let method = Ident::new("range", name.span());
            quote!(#ty::#method(#from, #to))
        }
        Some((name, inner)) if name == "regex" => {
            let method = Ident::new("regex", name.span());
            quote!(#ty::#method(#inner))
        }
        Some((name, inner)) if name == "special" => {
            let special = values::special_value(cx, &inner)?;
            let method = Ident::new("special", name.span());
            quote!(#ty::#method(#special))
        }
        _ => quote!(#ty::value(#matched)),
    };
    Some(quote!(#kind.text(#text)))
}

/// `horizontal(<variable>[, <max per row>])` and `vertical(<variable>)`
/// build a `Repeat`; any other expression is passed through.
fn repeat(cx: &mut Cx, value: &Expr) -> Option<TokenStream> {
    let Some((name, inner)) = value.as_call() else {
        return Some(value.to_token_stream());
    };
    let ty = cx.path("Repeat", name.span());
    let arguments = crate::tokens::split_commas(inner.to_token_stream());
    match (name.to_string().as_str(), arguments.as_slice()) {
        ("horizontal", [variable]) => Some(quote!(#ty::#name(#variable))),
        ("horizontal", [variable, per_row]) => {
            Some(quote!(#ty::#name(#variable).max_per_row(#per_row)))
        }
        ("vertical", [variable]) => Some(quote!(#ty::#name(#variable))),
        ("horizontal" | "vertical", _) => {
            cx.error(syn::Error::new_spanned(
                value,
                "expected `horizontal(<variable>)`, `horizontal(<variable>, <max per row>)`, or `vertical(<variable>)`",
            ));
            None
        }
        _ => Some(value.to_token_stream()),
    }
}

/// `let keyed = keyed.method(arguments);` on the keyed table.
fn call_on_keyed(span: Span, method: &str, arguments: TokenStream) -> TokenStream {
    call(&local(KEYED_VAR), span, method, arguments)
}

const KEYED_VAR: &str = "__panelist_keyed";

/// Defers statements on the keyed table until the table keys are applied.
fn keyed(cx: &mut Cx, statements: TokenStream) -> TokenStream {
    match &mut cx.keyed {
        Some(keyed) => {
            keyed.extend(statements);
            TokenStream::new()
        }
        None => statements,
    }
}

/// A keyed table column: `column <name> => <query> [{ … }]`, or
/// `trend <reducer> <name> => <query> [{ … }]`.
fn column(
    cx: &mut Cx,
    keyword: &Ident,
    trend: bool,
    tokens: &mut Tokens,
) -> syn::Result<Option<TokenStream>> {
    let reducer = if trend {
        Some(tokens.expect_ident("a reducer such as `last` after `trend`")?)
    } else {
        None
    };
    let (name, query) = pair(tokens, keyword)?;
    let query = match query.as_call_of_macro("promql") {
        Some(arguments) => {
            let ty = cx.path("PrometheusQuery", keyword.span());
            quote!(#ty::new(#arguments))
        }
        None => query.to_token_stream(),
    };
    let ty = cx.path("Column", keyword.span());
    let init = match reducer {
        None => Some(quote!(#ty::new(#name, #query))),
        Some(reducer) => values::word(
            cx,
            &REDUCER,
            &Expr::from_stream(reducer.into_token_stream()),
        )
        .map(|reducer| quote!(#ty::trend(#name, #query, #reducer))),
    };
    let column = pair_block(
        cx,
        &COLUMN,
        init.clone().unwrap_or_default(),
        tokens,
        keyword,
    )?;
    Ok(init.map(|_| column))
}

/// What a panel or row is started from.
enum Head {
    Title(Expr),
    Base(Expr),
}

/// `<title> { … }` or `from <builder> { … }` after a panel or row keyword.
fn head(tokens: &mut Tokens, keyword: &Ident, what: &str) -> syn::Result<(Head, Group)> {
    let (head, by, group) = head_by(tokens, keyword, what, false)?;
    debug_assert!(by.is_none(), "`by` is only read for keyed tables");
    Ok((head, group))
}

/// [`head`], reading `by <key>` before the block when `keyed` is set.
fn head_by(
    tokens: &mut Tokens,
    keyword: &Ident,
    what: &str,
    keyed: bool,
) -> syn::Result<(Head, Option<Expr>, Group)> {
    let stop = if keyed {
        Stop::TITLE_OR_BY
    } else {
        Stop::TITLE
    };
    let head = if tokens.is_ident("from") {
        tokens.next();
        let base = tokens.take_until(if keyed {
            Stop::BASE_OR_BY
        } else {
            Stop::SEMI_OR_BRACE
        });
        if base.is_empty() {
            return Err(tokens.error(format!("expected a {what} builder expression after `from`")));
        }
        Head::Base(base)
    } else {
        Head::Title(head_title(tokens, keyword, what, stop)?)
    };
    let by = if keyed {
        if !tokens.is_ident("by") {
            return Err(tokens.error(format!(
                "a {what} needs `by <label>` after its title, naming the label its rows are keyed on"
            )));
        }
        tokens.next();
        let key = tokens.take_until(Stop::SEMI_OR_BRACE);
        if key.is_empty() {
            return Err(tokens.error("expected the label to key the table on after `by`"));
        }
        Some(key)
    } else {
        None
    };
    let group = tokens.expect_group(
        Delimiter::Brace,
        &format!("`{{ … }}` after the {what} title"),
    )?;
    Ok((head, by, group))
}

fn head_title(tokens: &mut Tokens, keyword: &Ident, what: &str, stop: Stop) -> syn::Result<Expr> {
    let title = tokens.take_until(stop);
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
    Ok(title)
}

fn panel(
    cx: &mut Cx,
    receiver: &Ident,
    index: usize,
    keyword: &Ident,
    tokens: &mut Tokens,
) -> syn::Result<TokenStream> {
    let kind = &PANEL_KINDS[index];
    let is_keyed = index == KEYED_TABLE_INDEX;
    let (head, by, group) = head_by(
        tokens,
        keyword,
        &format!("`{}` panel", kind.keyword),
        is_keyed,
    )?;
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
    let Some(by) = by else {
        let panel = block(cx, &PANEL, Some(index), init, &group, TokenStream::new());
        return Ok(call(receiver, keyword.span(), "panel", panel));
    };
    // A keyed table is configured as a table, then turned into a keyed
    // table, so its keyed-only keys run after every table key.
    let outer = cx.keyed.replace(TokenStream::new());
    let statements = body(cx, &PANEL, Some(index), &var, &mut Tokens::inside(&group));
    let deferred = std::mem::replace(&mut cx.keyed, outer).unwrap_or_default();
    let keyed = local(KEYED_VAR);
    let keyed_method = Ident::new("keyed", keyword.span());
    let panel = quote!({
        let #var = #init;
        #statements
        let #keyed = #var.#keyed_method(#by);
        #deferred
        #keyed
    });
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
        TransformForm::CallWithTwo { ty } => {
            let arguments = arguments(tokens, &name, "(<a>, <b>)")?;
            let [first, second] = arguments.as_slice() else {
                return Err(argument_count(&name, "(<a>, <b>)"));
            };
            let ty = cx.path(ty, name.span());
            statements.extend(quote!(let #var = #ty::new(#first, #second);));
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
