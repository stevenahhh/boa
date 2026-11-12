//! Unary operator parsing.
//!
//! More information:
//!  - [MDN documentation][mdn]
//!  - [ECMAScript specification][spec]
//!
//! [mdn]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Expressions_and_Operators#Unary
//! [spec]: https://tc39.es/ecma262/#sec-unary-operators

use crate::{
    Error,
    lexer::{Error as LexError, TokenKind},
    parser::{
        AllowAwait, AllowYield, Cursor, OrAbrupt, ParseResult, TokenParser,
        expression::{
            FormalParameterListOrExpression, await_expr::AwaitExpression, update::UpdateExpression,
        },
    },
    source::ReadChar,
};
use boa_ast::{
    Expression, Keyword, Position, Punctuator, Span, Spanned,
    expression::{
        access::PropertyAccess,
        operator::{Unary, unary::UnaryOp},
    },
};
use boa_interner::Interner;

/// Parses a unary expression.
///
/// More information:
///  - [MDN documentation][mdn]
///  - [ECMAScript specification][spec]
///
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Expressions_and_Operators#Unary
/// [spec]: https://tc39.es/ecma262/#prod-UnaryExpression
#[derive(Debug, Clone, Copy)]
pub(in crate::parser) struct UnaryExpression {
    allow_yield: AllowYield,
    allow_await: AllowAwait,
}

impl UnaryExpression {
    /// Creates a new `UnaryExpression` parser.
    pub(in crate::parser) fn new<Y, A>(allow_yield: Y, allow_await: A) -> Self
    where
        Y: Into<AllowYield>,
        A: Into<AllowAwait>,
    {
        Self {
            allow_yield: allow_yield.into(),
            allow_await: allow_await.into(),
        }
    }
}

impl<R> TokenParser<R> for UnaryExpression
where
    R: ReadChar,
{
    type Output = FormalParameterListOrExpression;

    #[inline(never)]
    fn parse(self, cursor: &mut Cursor<R>, interner: &mut Interner) -> ParseResult<Self::Output> {
        cursor.check_stack()?;
        let tok = cursor.peek(0, interner).or_abrupt()?;
        let token_start = tok.span().start();
        let op = match tok.kind() {
            TokenKind::Keyword((Keyword::Delete | Keyword::Void | Keyword::TypeOf, true)) => {
                return Err(Error::general(
                    "Keyword must not contain escaped characters",
                    token_start,
                ));
            }
            TokenKind::Keyword((Keyword::Delete, false)) => UnaryOp::Delete,
            TokenKind::Keyword((Keyword::Void, false)) => UnaryOp::Void,
            TokenKind::Keyword((Keyword::TypeOf, false)) => UnaryOp::TypeOf,
            TokenKind::Punctuator(Punctuator::Add) => UnaryOp::Plus,
            TokenKind::Punctuator(Punctuator::Sub) => UnaryOp::Minus,
            TokenKind::Punctuator(Punctuator::Neg) => UnaryOp::Tilde,
            TokenKind::Punctuator(Punctuator::Not) => UnaryOp::Not,
            TokenKind::Keyword((Keyword::Await, true)) if self.allow_await.0 => {
                return Err(Error::general(
                    "Keyword 'await' must not contain escaped characters",
                    token_start,
                ));
            }
            TokenKind::Keyword((Keyword::Await, false)) if self.allow_await.0 => {
                return Ok(
                    (AwaitExpression::new(self.allow_yield).parse(cursor, interner)?).into(),
                );
            }
            _ => {
                return UpdateExpression::new(self.allow_yield, self.allow_await)
                    .parse(cursor, interner);
            }
        };
        cursor.advance(interner);

        // Every operator shares this one recursive call, so a level of nesting takes one frame.
        let target_start = if op == UnaryOp::Delete {
            Some(cursor.peek(0, interner).or_abrupt()?.span().start())
        } else {
            None
        };
        let target = self.parse(cursor, interner)?.try_into_expression()?;
        if let Some(target_start) = target_start {
            check_delete_target(&target, token_start, target_start, cursor.strict())?;
        }

        let target_span_end = target.span().end();
        Ok(Unary::new(op, target, Span::new(token_start, target_span_end)).into())
    }
}

/// The early errors of `delete` with the operand `target`.
fn check_delete_target(
    target: &Expression,
    delete_start: Position,
    target_start: Position,
    strict: bool,
) -> ParseResult<()> {
    match target.flatten() {
        Expression::Identifier(_) if strict => Err(Error::lex(LexError::Syntax(
            "cannot delete variables in strict mode".into(),
            delete_start,
        ))),
        Expression::PropertyAccess(PropertyAccess::Private(_)) => Err(Error::lex(
            LexError::Syntax("cannot delete private fields".into(), target_start),
        )),
        _ => Ok(()),
    }
}
