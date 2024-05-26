// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#[cfg(test)]
mod tests;

use super::{keyword::Keyword, scan::ParserContext, ty::ty, Error, Parser, Result};
use crate::{
    item::throw_away_doc,
    lex::{Delim, TokenKind},
    ErrorKind,
};
use qsc_ast::ast::{Ident, NodeId, Pat, PatKind, Path};
use qsc_data_structures::span::{Span, WithSpan};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FinalSep {
    Present,
    Missing,
}

impl FinalSep {
    pub(super) fn reify<T, U>(
        self,
        mut xs: Vec<T>,
        mut as_paren: impl FnMut(T) -> U,
        mut as_seq: impl FnMut(Box<[T]>) -> U,
    ) -> U {
        if self == Self::Missing && xs.len() == 1 {
            as_paren(xs.pop().expect("vector should have exactly one item"))
        } else {
            as_seq(xs.into_boxed_slice())
        }
    }
}

pub(super) fn token(s: &mut ParserContext, t: TokenKind) -> Result<()> {
    if s.peek().kind == t {
        s.advance();
        Ok(())
    } else {
        Err(Error(ErrorKind::Token(t, s.peek().kind, s.peek().span)))
    }
}

pub(super) fn apos_ident(s: &mut ParserContext) -> Result<Box<Ident>> {
    let peek = s.peek();
    if peek.kind == TokenKind::AposIdent {
        let name = s.read().into();
        s.advance();
        Ok(Box::new(Ident {
            id: NodeId::default(),
            span: peek.span,
            name,
        }))
    } else {
        Err(Error(ErrorKind::Rule(
            "generic parameter",
            peek.kind,
            peek.span,
        )))
    }
}

pub(super) fn ident(s: &mut ParserContext) -> Result<Box<Ident>> {
    let peek = s.peek();
    if peek.kind == TokenKind::Ident {
        let name = s.read().into();
        s.advance();
        Ok(Box::new(Ident {
            id: NodeId::default(),
            span: peek.span,
            name,
        }))
    } else {
        Err(Error(ErrorKind::Rule("identifier", peek.kind, peek.span)))
    }
}

/// A `path` is a dot-separated list of idents like "Foo.Bar.Baz"
/// this can be either a namespace name (in an open statement or namespace declaration) or
/// it can be a direct reference to something in a namespace, like `Microsoft.Quantum.Diagnostics.DumpMachine()`
pub(super) fn path(s: &mut ParserContext) -> Result<Box<Path>> {
    let lo = s.peek().span.lo;
    let mut parts = vec![*ident(s)?];
    while token(s, TokenKind::Dot).is_ok() {
        parts.push(*ident(s)?);
    }

    Ok(Box::new(Path::new(parts)))
}

pub(super) fn pat(s: &mut ParserContext) -> Result<Box<Pat>> {
    throw_away_doc(s);
    let lo = s.peek().span.lo;
    let kind = if token(s, TokenKind::Keyword(Keyword::Underscore)).is_ok() {
        let ty = if token(s, TokenKind::Colon).is_ok() {
            Some(Box::new(ty(s)?))
        } else {
            None
        };
        Ok(PatKind::Discard(ty))
    } else if token(s, TokenKind::DotDotDot).is_ok() {
        Ok(PatKind::Elided)
    } else if token(s, TokenKind::Open(Delim::Paren)).is_ok() {
        let (pats, final_sep) = comma_separated_seq(s, pat)?;
        token(s, TokenKind::Close(Delim::Paren))?;
        Ok(final_sep.reify(pats, PatKind::Paren, PatKind::Tuple))
    } else {
        let name = ident(s).map_err(|e| map_rule_name("pattern", e))?;
        let ty = if token(s, TokenKind::Colon).is_ok() {
            Some(Box::new(ty(s)?))
        } else {
            None
        };
        Ok(PatKind::Bind(name, ty))
    }?;

    Ok(Box::new(Pat {
        id: NodeId::default(),
        span: s.span(lo),
        kind: Box::new(kind),
    }))
}

pub(super) fn opt<T>(s: &mut ParserContext, mut p: impl Parser<T>) -> Result<Option<T>> {
    let offset = s.peek().span.lo;
    match p(s) {
        Ok(x) => Ok(Some(x)),
        Err(error) if advanced(s, offset) => Err(error),
        Err(_) => Ok(None),
    }
}

pub(super) fn many<T>(s: &mut ParserContext, mut p: impl Parser<T>) -> Result<Vec<T>> {
    let mut xs = Vec::new();
    while let Some(x) = opt(s, &mut p)? {
        xs.push(x);
    }
    Ok(xs)
}
/// Parses a sequence of items separated by `tok`.
pub(super) fn seq<T>(
    s: &mut ParserContext,
    mut p: impl Parser<T>,
    tok: TokenKind,
    recover_on_missing_item: bool,
) -> Result<(Vec<T>, FinalSep)>
where
    T: Default + WithSpan,
{
    let mut xs = Vec::new();
    let mut final_sep = FinalSep::Missing;
    while s.peek().kind == tok {
        let mut span = s.peek().span;
        span.hi = span.lo;
        let err = Error(ErrorKind::MissingSeqEntry(span));
        if recover_on_missing_item {
            s.push_error(err);
            xs.push(T::default().with_span(span));
            s.advance();
        } else {
            return Err(err);
        }
    }
    while let Some(x) = opt(s, &mut p)? {
        xs.push(x);
        if token(s, tok).is_ok() {
            while s.peek().kind == tok {
                let mut span = s.peek().span;
                span.hi = span.lo;
                let err = Error(ErrorKind::MissingSeqEntry(span));
                if recover_on_missing_item {
                    s.push_error(err);
                    xs.push(T::default().with_span(span));
                    s.advance();
                } else {
                    return Err(err);
                }
            }
            final_sep = FinalSep::Present;
        } else {
            final_sep = FinalSep::Missing;
            break;
        }
    }
    Ok((xs, final_sep))
}

/// Parses a sequence of items separated by commas.
pub(super) fn comma_separated_seq<T>(
    s: &mut ParserContext,
    p: impl Parser<T>,
) -> Result<(Vec<T>, FinalSep)>
where
    T: Default + WithSpan,
{
    seq(s, p, TokenKind::Comma, true)
}

pub(super) fn recovering<T>(
    s: &mut ParserContext,
    default: impl FnOnce(Span) -> T,
    tokens: &[TokenKind],
    mut p: impl Parser<T>,
) -> Result<T> {
    let offset = s.peek().span.lo;
    match p(s) {
        Ok(value) => Ok(value),
        Err(error) if advanced(s, offset) => {
            s.push_error(error);
            s.recover(tokens);
            Ok(default(s.span(offset)))
        }
        Err(error) => Err(error),
    }
}

pub(super) fn recovering_semi(s: &mut ParserContext) {
    if let Err(error) = token(s, TokenKind::Semi) {
        // no recovery, just move on to the next token
        s.push_error(error);
    }
}

pub(super) fn recovering_token(s: &mut ParserContext, t: TokenKind) {
    if let Err(error) = token(s, t) {
        s.push_error(error);
        s.recover(&[t]);
    }
}

pub(super) fn barrier<'a, T>(
    s: &mut ParserContext<'a>,
    tokens: &'a [TokenKind],
    mut p: impl Parser<T>,
) -> Result<T> {
    s.push_barrier(tokens);
    let result = p(s);
    s.pop_barrier().expect("barrier should be popped");
    result
}

pub(super) fn shorten(from_start: usize, from_end: usize, s: &str) -> &str {
    &s[from_start..s.len() - from_end]
}

fn advanced(s: &ParserContext, from: u32) -> bool {
    s.peek().span.lo > from
}

fn map_rule_name(name: &'static str, error: Error) -> Error {
    Error(match error.0 {
        ErrorKind::Rule(_, found, span) => ErrorKind::Rule(name, found, span),
        ErrorKind::Convert(_, found, span) => ErrorKind::Convert(name, found, span),
        kind => kind,
    })
}
