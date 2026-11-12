//! Exponentiation operator parsing.
//!
//! More information:
//!  - [MDN documentation][mdn]
//!  - [ECMAScript specification][spec]
//!
//! [mdn]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Arithmetic_Operators#Exponentiation
//! [spec]: https://tc39.es/ecma262/#sec-exp-operator

use crate::{
    lexer::TokenKind,
    parser::{
        AllowAwait, AllowYield, Cursor, OrAbrupt, ParseResult, TokenParser,
        expression::{
            FormalParameterListOrExpression, unary::UnaryExpression, update::UpdateExpression,
        },
    },
    source::ReadChar,
};
use boa_ast::{
    Keyword, Punctuator,
    expression::operator::{Binary, binary::ArithmeticOp},
};
use boa_interner::Interner;

/// Parses an exponentiation expression.
///
/// More information:
///  - [MDN documentation][mdn]
///  - [ECMAScript specification][spec]
///
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Arithmetic_Operators#Exponentiation
/// [spec]: https://tc39.es/ecma262/#prod-ExponentiationExpression
#[derive(Debug, Clone, Copy)]
pub(in crate::parser::expression) struct ExponentiationExpression {
    allow_yield: AllowYield,
    allow_await: AllowAwait,
}

impl ExponentiationExpression {
    /// Creates a new `ExponentiationExpression` parser.
    pub(in crate::parser::expression) fn new<Y, A>(allow_yield: Y, allow_await: A) -> Self
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

impl<R> TokenParser<R> for ExponentiationExpression
where
    R: ReadChar,
{
    type Output = FormalParameterListOrExpression;

    fn parse(self, cursor: &mut Cursor<R>, interner: &mut Interner) -> ParseResult<Self::Output> {
        cursor.check_stack()?;
        let (first, unary) = self.parse_operand(cursor, interner)?;
        let FormalParameterListOrExpression::Expression(first) = first else {
            return Ok(first);
        };
        // A `UnaryExpression` is never the left operand of `**`.
        if unary || !next_is_exponentiation(cursor, interner)? {
            return Ok(first.into());
        }

        // `**` is right-associative. The operands of a chain of them are read in a loop, and
        // each level of the tree they make is charged with the stack it costs the passes over
        // the tree, which recurse once per level.
        let mut operands = vec![first];
        loop {
            cursor.advance(interner);
            cursor.check_stack_for(operands.len() * EXPONENTIATION_LEVEL_STACK)?;
            let (operand, unary) = self.parse_operand(cursor, interner)?;
            operands.push(operand.try_into_expression()?);
            if unary || !next_is_exponentiation(cursor, interner)? {
                break;
            }
        }

        let mut rhs = operands.pop().expect("a chain has at least two operands");
        while let Some(lhs) = operands.pop() {
            rhs = Binary::new(ArithmeticOp::Exp.into(), lhs, rhs).into();
        }
        Ok(rhs.into())
    }
}

/// The native stack charged for each level of a chain of `**` operators.
const EXPONENTIATION_LEVEL_STACK: usize = 448;

impl ExponentiationExpression {
    /// Parses the `UnaryExpression` or `UpdateExpression` that is an operand of `**`, and returns
    /// whether it is a `UnaryExpression`.
    fn parse_operand<R>(
        self,
        cursor: &mut Cursor<R>,
        interner: &mut Interner,
    ) -> ParseResult<(FormalParameterListOrExpression, bool)>
    where
        R: ReadChar,
    {
        let unary = match cursor.peek(0, interner).or_abrupt()?.kind() {
            TokenKind::Keyword((Keyword::Delete | Keyword::Void | Keyword::TypeOf, _))
            | TokenKind::Punctuator(
                Punctuator::Add | Punctuator::Sub | Punctuator::Not | Punctuator::Neg,
            ) => true,
            TokenKind::Keyword((Keyword::Await, _)) => self.allow_await.0,
            _ => false,
        };
        let operand = if unary {
            UnaryExpression::new(self.allow_yield, self.allow_await).parse(cursor, interner)?
        } else {
            UpdateExpression::new(self.allow_yield, self.allow_await).parse(cursor, interner)?
        };
        Ok((operand, unary))
    }
}

/// Returns whether the next token is `**`.
fn next_is_exponentiation<R>(cursor: &mut Cursor<R>, interner: &mut Interner) -> ParseResult<bool>
where
    R: ReadChar,
{
    Ok(cursor
        .peek(0, interner)?
        .is_some_and(|tok| tok.kind() == &TokenKind::Punctuator(Punctuator::Exp)))
}
