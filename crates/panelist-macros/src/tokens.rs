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

//! A cursor over one delimited token stream, and the expression runs the
//! grammar reads from it.
//!
//! The macro never parses Rust expressions. A value is the run of tokens up
//! to a stop token at the top level of the current group, so anything Rust
//! accepts as an expression is accepted unchanged and type-checked by the
//! compiler where the builder call receives it.

use proc_macro2::{Delimiter, Group, Ident, Literal, Spacing, Span, TokenStream, TokenTree};
use quote::ToTokens;

/// Tokens that end an expression run when they appear at the top level.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Stop {
    semi: bool,
    comma: bool,
    colon: bool,
    fat_arrow: bool,
    brace: bool,
    from: bool,
    by: bool,
}

impl Stop {
    /// `;`
    pub(crate) const SEMI: Self = Self {
        semi: true,
        comma: false,
        colon: false,
        fat_arrow: false,
        brace: false,
        from: false,
        by: false,
    };

    /// `;` or a `{ … }` group.
    pub(crate) const SEMI_OR_BRACE: Self = Self {
        brace: true,
        ..Self::SEMI
    };

    /// `;` or a single `:`.
    pub(crate) const SEMI_OR_COLON: Self = Self {
        colon: true,
        ..Self::SEMI
    };

    /// `;` or `=>`.
    pub(crate) const SEMI_OR_FAT_ARROW: Self = Self {
        fat_arrow: true,
        ..Self::SEMI
    };

    /// `,`
    pub(crate) const COMMA: Self = Self {
        semi: false,
        comma: true,
        colon: false,
        fat_arrow: false,
        brace: false,
        from: false,
        by: false,
    };

    /// `;`, a `{ … }` group, or the identifier `from` outside a path.
    pub(crate) const TITLE: Self = Self {
        from: true,
        ..Self::SEMI_OR_BRACE
    };

    /// [`Stop::TITLE`], or the identifier `by` outside a path.
    pub(crate) const TITLE_OR_BY: Self = Self {
        by: true,
        ..Self::TITLE
    };

    /// `;`, a `{ … }` group, or the identifier `by` outside a path.
    pub(crate) const BASE_OR_BY: Self = Self {
        by: true,
        ..Self::SEMI_OR_BRACE
    };
}

/// A run of tokens that the grammar treats as one Rust expression.
#[derive(Debug, Clone)]
pub(crate) struct Expr {
    trees: Vec<TokenTree>,
}

impl Expr {
    pub(crate) fn new(trees: Vec<TokenTree>) -> Self {
        Self { trees }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.trees.is_empty()
    }

    /// The identifier when the run is exactly one identifier.
    pub(crate) fn single_ident(&self) -> Option<&Ident> {
        match self.trees.as_slice() {
            [TokenTree::Ident(ident)] => Some(ident),
            _ => None,
        }
    }

    /// The literal when the run is exactly one string literal.
    pub(crate) fn single_string(&self) -> Option<&Literal> {
        match self.trees.as_slice() {
            [TokenTree::Literal(literal)] if is_string_literal(literal) => Some(literal),
            _ => None,
        }
    }

    /// `(name, contents)` when the run is exactly `name(<contents>)`.
    pub(crate) fn as_call(&self) -> Option<(&Ident, Expr)> {
        match self.trees.as_slice() {
            [TokenTree::Ident(name), TokenTree::Group(group)]
                if group.delimiter() == Delimiter::Parenthesis =>
            {
                Some((name, Expr::from_stream(group.stream())))
            }
            _ => None,
        }
    }

    /// The arguments when the run is exactly `name!( … )`.
    pub(crate) fn as_call_of_macro(&self, name: &str) -> Option<TokenStream> {
        match self.trees.as_slice() {
            [
                TokenTree::Ident(ident),
                TokenTree::Punct(bang),
                TokenTree::Group(group),
            ] if ident == name
                && bang.as_char() == '!'
                && group.delimiter() == Delimiter::Parenthesis =>
            {
                Some(group.stream())
            }
            _ => None,
        }
    }

    /// The comma-separated elements when the run is exactly one group with
    /// the given delimiter.
    pub(crate) fn as_list(&self, delimiter: Delimiter) -> Option<Vec<Expr>> {
        match self.trees.as_slice() {
            [TokenTree::Group(group)] if group.delimiter() == delimiter => {
                Some(split_commas(group.stream()))
            }
            _ => None,
        }
    }

    pub(crate) fn from_stream(stream: TokenStream) -> Self {
        Self::new(stream.into_iter().collect())
    }
}

impl ToTokens for Expr {
    /// Emits the expression, dropping one pair of parentheses around the
    /// whole of it. `stat (title) { … }` is the documented way to write a
    /// title that would otherwise end early, and the builder call already
    /// delimits the argument, so keeping them would only earn the caller an
    /// `unused_parens` warning. A parenthesized tuple is left alone.
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let [TokenTree::Group(group)] = self.trees.as_slice()
            && group.delimiter() == Delimiter::Parenthesis
            && !group.stream().is_empty()
            && split_commas(group.stream()).len() == 1
            && !group.stream().into_iter().last().is_some_and(
                |tree| matches!(tree, TokenTree::Punct(punct) if punct.as_char() == ','),
            )
        {
            tokens.extend(group.stream());
        } else {
            tokens.extend(self.trees.iter().cloned());
        }
    }
}

/// Splits a stream at its top-level commas, dropping one trailing comma.
pub(crate) fn split_commas(stream: TokenStream) -> Vec<Expr> {
    let mut tokens = Tokens::new(stream, Span::call_site());
    let mut elements = Vec::new();
    while !tokens.is_empty() {
        elements.push(tokens.take_until(Stop::COMMA));
        tokens.eat_punct(',');
    }
    elements
}

fn is_string_literal(literal: &Literal) -> bool {
    let text = literal.to_string();
    text.starts_with('"') || text.starts_with("r\"") || text.starts_with("r#")
}

/// A cursor over the token trees of one group.
#[derive(Debug)]
pub(crate) struct Tokens {
    trees: Vec<TokenTree>,
    position: usize,
    /// Where to report a token that is missing at the end of the group.
    end: Span,
}

impl Tokens {
    pub(crate) fn new(stream: TokenStream, end: Span) -> Self {
        Self {
            trees: stream.into_iter().collect(),
            position: 0,
            end,
        }
    }

    /// The tokens inside a group, reporting missing tokens at its close.
    pub(crate) fn inside(group: &Group) -> Self {
        Self::new(group.stream(), group.span_close())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.position >= self.trees.len()
    }

    fn peek_nth(&self, offset: usize) -> Option<&TokenTree> {
        self.trees.get(self.position + offset)
    }

    pub(crate) fn next(&mut self) -> Option<TokenTree> {
        let tree = self.trees.get(self.position).cloned();
        if tree.is_some() {
            self.position += 1;
        }
        tree
    }

    /// The span of the next token, or of the group's close at the end.
    pub(crate) fn span(&self) -> Span {
        self.peek_nth(0).map_or(self.end, TokenTree::span)
    }

    pub(crate) fn peek_ident(&self) -> Option<&Ident> {
        match self.peek_nth(0) {
            Some(TokenTree::Ident(ident)) => Some(ident),
            _ => None,
        }
    }

    pub(crate) fn is_ident(&self, name: &str) -> bool {
        self.peek_ident().is_some_and(|ident| ident == name)
    }

    fn punct_at(&self, offset: usize, ch: char) -> Option<Spacing> {
        match self.peek_nth(offset) {
            Some(TokenTree::Punct(punct)) if punct.as_char() == ch => Some(punct.spacing()),
            _ => None,
        }
    }

    pub(crate) fn is_punct(&self, ch: char) -> bool {
        self.punct_at(0, ch).is_some()
    }

    /// A `:` that does not start a `::` path separator.
    pub(crate) fn is_colon(&self) -> bool {
        self.is_colon_at(0)
    }

    fn is_colon_at(&self, offset: usize) -> bool {
        match self.punct_at(offset, ':') {
            Some(Spacing::Joint) => self.punct_at(offset + 1, ':').is_none(),
            Some(Spacing::Alone) => true,
            None => false,
        }
    }

    /// Whether the next two tokens are an identifier and a single `:`.
    pub(crate) fn is_key_colon(&self) -> bool {
        matches!(self.peek_nth(0), Some(TokenTree::Ident(_))) && self.is_colon_at(1)
    }

    pub(crate) fn is_fat_arrow(&self) -> bool {
        self.punct_at(0, '=') == Some(Spacing::Joint) && self.punct_at(1, '>').is_some()
    }

    pub(crate) fn is_group(&self, delimiter: Delimiter) -> bool {
        self.group_at(0, delimiter).is_some()
    }

    fn group_at(&self, offset: usize, delimiter: Delimiter) -> Option<&Group> {
        match self.peek_nth(offset) {
            Some(TokenTree::Group(group)) if group.delimiter() == delimiter => Some(group),
            _ => None,
        }
    }

    /// Whether the tokens ahead are `name ! ( … )` followed by `{` or `;`.
    pub(crate) fn is_macro_call_then_block_or_semi(&self) -> bool {
        matches!(self.peek_nth(0), Some(TokenTree::Ident(_)))
            && self.punct_at(1, '!').is_some()
            && self.group_at(2, Delimiter::Parenthesis).is_some()
            && (self.group_at(3, Delimiter::Brace).is_some() || self.punct_at(3, ';').is_some())
    }

    /// Whether the token after a `name ! ( … )` call is the punctuation `ch`.
    pub(crate) fn is_punct_after_macro_call(&self, ch: char) -> bool {
        self.punct_at(3, ch).is_some()
    }

    /// Whether the tokens ahead are an identifier and a `{ … }` group.
    pub(crate) fn is_ident_then_brace(&self) -> bool {
        matches!(self.peek_nth(0), Some(TokenTree::Ident(_)))
            && self.group_at(1, Delimiter::Brace).is_some()
    }

    pub(crate) fn eat_punct(&mut self, ch: char) -> bool {
        if self.is_punct(ch) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    pub(crate) fn expect_ident(&mut self, expected: &str) -> syn::Result<Ident> {
        match self.peek_nth(0) {
            Some(TokenTree::Ident(ident)) => {
                let ident = ident.clone();
                self.position += 1;
                Ok(ident)
            }
            _ => Err(self.error(format!("expected {expected}"))),
        }
    }

    pub(crate) fn expect_colon(&mut self, after: &Ident) -> syn::Result<()> {
        if self.is_colon() {
            self.position += 1;
            Ok(())
        } else {
            Err(self.error(format!("expected `:` after `{after}`")))
        }
    }

    pub(crate) fn expect_semi(&mut self, after: &str) -> syn::Result<()> {
        if self.eat_punct(';') {
            Ok(())
        } else {
            Err(self.error(format!("expected `;` after {after}")))
        }
    }

    pub(crate) fn expect_fat_arrow(&mut self, after: &str) -> syn::Result<()> {
        if self.is_fat_arrow() {
            self.position += 2;
            Ok(())
        } else {
            Err(self.error(format!("expected `=>` after {after}")))
        }
    }

    pub(crate) fn expect_group(
        &mut self,
        delimiter: Delimiter,
        expected: &str,
    ) -> syn::Result<Group> {
        match self.group_at(0, delimiter) {
            Some(group) => {
                let group = group.clone();
                self.position += 1;
                Ok(group)
            }
            None => Err(self.error(format!("expected {expected}"))),
        }
    }

    /// Reads the tokens up to the first top-level stop token, which is left
    /// in place.
    pub(crate) fn take_until(&mut self, stop: Stop) -> Expr {
        let start = self.position;
        while let Some(tree) = self.peek_nth(0) {
            let ends = match tree {
                TokenTree::Group(group) => stop.brace && group.delimiter() == Delimiter::Brace,
                TokenTree::Punct(punct) => match punct.as_char() {
                    ';' => stop.semi,
                    ',' => stop.comma,
                    ':' if self.punct_at(1, ':').is_some() && punct.spacing() == Spacing::Joint => {
                        // A `::` path separator is never a stop.
                        self.position += 2;
                        continue;
                    }
                    ':' => stop.colon,
                    '=' => stop.fat_arrow && self.is_fat_arrow(),
                    _ => false,
                },
                TokenTree::Ident(ident) => {
                    ((stop.from && ident == "from") || (stop.by && ident == "by"))
                        && !self.after_path()
                }
                TokenTree::Literal(_) => false,
            };
            if ends {
                break;
            }
            self.position += 1;
        }
        Expr::new(self.trees[start..self.position].to_vec())
    }

    /// Whether the previous token makes the next identifier part of a path
    /// or a method call rather than a keyword of the grammar.
    fn after_path(&self) -> bool {
        self.position
            .checked_sub(1)
            .and_then(|previous| self.trees.get(previous))
            .is_some_and(|tree| {
                matches!(tree, TokenTree::Punct(punct) if matches!(punct.as_char(), '.' | ':'))
            })
    }

    /// Skips past the current entry after an error: through the next
    /// top-level `;`, or through the next `{ … }` group and any scope or
    /// `;` that follows it.
    pub(crate) fn recover(&mut self) {
        while let Some(tree) = self.next() {
            match tree {
                TokenTree::Punct(punct) if punct.as_char() == ';' => return,
                TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                    if self.is_ident("only") {
                        while let Some(tree) = self.next() {
                            if matches!(&tree, TokenTree::Punct(punct) if punct.as_char() == ';') {
                                return;
                            }
                        }
                    }
                    self.eat_punct(';');
                    return;
                }
                _ => {}
            }
        }
    }

    pub(crate) fn error(&self, message: impl std::fmt::Display) -> syn::Error {
        syn::Error::new(self.span(), message)
    }
}
