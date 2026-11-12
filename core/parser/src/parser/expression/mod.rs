//! Expression parsing.
//!
//! More information:
//!  - [MDN documentation][mdn]
//!  - [ECMAScript specification][spec]
//!
//! [mdn]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators
//! [spec]: https://tc39.es/ecma262/#sec-ecmascript-language-expressions

mod assignment;
mod fpl_or_exp;
mod identifiers;
mod left_hand_side;
mod primary;
mod unary;
mod update;

pub(in crate::parser) mod await_expr;

#[cfg(test)]
mod tests;

use crate::parser::CHAIN_LINK_STACK;
use crate::{
    Error,
    lexer::{InputElement, TokenKind},
    parser::{
        AllowAwait, AllowIn, AllowYield, Cursor, OrAbrupt, ParseResult, TokenParser,
        expression::assignment::ExponentiationExpression,
    },
    source::ReadChar,
};
use boa_ast::{
    self as ast, Keyword, Position, Punctuator, Spanned,
    expression::{
        Identifier,
        operator::{
            Binary, BinaryInPrivate,
            binary::{BinaryOp, LogicalOp},
        },
    },
    function::PrivateName,
};
use boa_interner::{Interner, Sym};

pub(super) use self::{assignment::AssignmentExpression, primary::Initializer};
pub(in crate::parser) use {
    fpl_or_exp::FormalParameterListOrExpression,
    identifiers::{BindingIdentifier, LabelIdentifier},
    left_hand_side::LeftHandSideExpression,
    primary::object_initializer::{
        AsyncGeneratorMethod, AsyncMethod, GeneratorMethod, PropertyName,
    },
};

/// Expression parsing.
///
/// More information:
///  - [MDN documentation][mdn]
///  - [ECMAScript specification][spec]
///
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators
/// [spec]: https://tc39.es/ecma262/#prod-Expression
#[derive(Debug, Clone, Copy)]
pub(super) struct Expression {
    allow_in: AllowIn,
    allow_yield: AllowYield,
    allow_await: AllowAwait,
}

impl Expression {
    /// Creates a new `Expression` parser.
    pub(super) fn new<I, Y, A>(allow_in: I, allow_yield: Y, allow_await: A) -> Self
    where
        I: Into<AllowIn>,
        Y: Into<AllowYield>,
        A: Into<AllowAwait>,
    {
        Self {
            allow_in: allow_in.into(),
            allow_yield: allow_yield.into(),
            allow_await: allow_await.into(),
        }
    }
}

impl<R> TokenParser<R> for Expression
where
    R: ReadChar,
{
    type Output = ast::Expression;

    fn parse(self, cursor: &mut Cursor<R>, interner: &mut Interner) -> ParseResult<Self::Output> {
        let lhs = AssignmentExpression::new(self.allow_in, self.allow_yield, self.allow_await)
            .parse(cursor, interner)?;

        self.parse_sequence(lhs, cursor, interner)
    }
}
impl Expression {
    /// Parses the comma operators after `lhs` and their right operands.
    ///
    /// It stays out of the frame of `parse`, which every level of nesting goes through.
    #[inline(never)]
    fn parse_sequence<R>(
        self,
        mut lhs: ast::Expression,
        cursor: &mut Cursor<R>,
        interner: &mut Interner,
    ) -> ParseResult<ast::Expression>
    where
        R: ReadChar,
    {
        let mut links = 0;
        loop {
            links += 1;
            cursor.check_stack_for(links * CHAIN_LINK_STACK)?;
            let Some(tok) = cursor.peek(0, interner)? else {
                break;
            };
            match *tok.kind() {
                TokenKind::Punctuator(Punctuator::Comma) => {
                    if cursor.peek(1, interner).or_abrupt()?.kind()
                        == &TokenKind::Punctuator(Punctuator::CloseParen)
                    {
                        return Ok(lhs);
                    }

                    if cursor.peek(1, interner).or_abrupt()?.kind()
                        == &TokenKind::Punctuator(Punctuator::Spread)
                    {
                        return Ok(lhs);
                    }

                    cursor.advance(interner);

                    lhs = Binary::new(
                        Punctuator::Comma
                            .as_binary_op()
                            .expect("Could not get binary operation."),
                        lhs,
                        AssignmentExpression::new(
                            self.allow_in,
                            self.allow_yield,
                            self.allow_await,
                        )
                        .parse(cursor, interner)?,
                    )
                    .into();
                }
                _ => break,
            }
        }

        Ok(lhs)
    }
}

/// Parses a logical expression expression.
///
/// More information:
///  - [MDN documentation][mdn]
///  - [ECMAScript specification][spec]
///
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Logical_Operators
/// [spec]: https://tc39.es/ecma262/#prod-ShortCircuitExpression
#[derive(Debug, Clone, Copy)]
struct ShortCircuitExpression {
    allow_in: AllowIn,
    allow_yield: AllowYield,
    allow_await: AllowAwait,
    previous: PreviousExpr,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PreviousExpr {
    None,
    Logical,
    Coalesce,
}

impl ShortCircuitExpression {
    /// Creates a new `ShortCircuitExpression` parser.
    pub(super) fn new<I, Y, A>(allow_in: I, allow_yield: Y, allow_await: A) -> Self
    where
        I: Into<AllowIn>,
        Y: Into<AllowYield>,
        A: Into<AllowAwait>,
    {
        Self {
            allow_in: allow_in.into(),
            allow_yield: allow_yield.into(),
            allow_await: allow_await.into(),
            previous: PreviousExpr::None,
        }
    }

    fn with_previous<I, Y, A>(
        allow_in: I,
        allow_yield: Y,
        allow_await: A,
        previous: PreviousExpr,
    ) -> Self
    where
        I: Into<AllowIn>,
        Y: Into<AllowYield>,
        A: Into<AllowAwait>,
    {
        Self {
            allow_in: allow_in.into(),
            allow_yield: allow_yield.into(),
            allow_await: allow_await.into(),
            previous,
        }
    }
}

impl<R> TokenParser<R> for ShortCircuitExpression
where
    R: ReadChar,
{
    type Output = FormalParameterListOrExpression;

    fn parse(self, cursor: &mut Cursor<R>, interner: &mut Interner) -> ParseResult<Self::Output> {
        let current_node =
            BitwiseORExpression::new(self.allow_in, self.allow_yield, self.allow_await)
                .parse(cursor, interner)?;
        let FormalParameterListOrExpression::Expression(current_node) = current_node else {
            return Ok(current_node);
        };

        self.parse_operators(current_node, cursor, interner)
    }
}
impl ShortCircuitExpression {
    /// Parses the operators `&&`, `||` and `??` after `current_node` and their right operands.
    ///
    /// It stays out of the frame of `parse`, which every level of nesting goes through.
    #[inline(never)]
    fn parse_operators<R>(
        self,
        mut current_node: ast::Expression,
        cursor: &mut Cursor<R>,
        interner: &mut Interner,
    ) -> ParseResult<FormalParameterListOrExpression>
    where
        R: ReadChar,
    {
        let mut previous = self.previous;

        let mut links = 0;
        loop {
            links += 1;
            cursor.check_stack_for(links * CHAIN_LINK_STACK)?;
            let Some(tok) = cursor.peek(0, interner)? else {
                break;
            };
            match tok.kind() {
                TokenKind::Punctuator(Punctuator::BoolAnd) => {
                    if previous == PreviousExpr::Coalesce {
                        return Err(Error::expected(
                            ["??".to_owned()],
                            tok.to_string(interner),
                            tok.span(),
                            "logical expression (cannot use '??' without parentheses within '||' or '&&')",
                        ));
                    }
                    cursor.advance(interner);
                    previous = PreviousExpr::Logical;
                    let rhs =
                        BitwiseORExpression::new(self.allow_in, self.allow_yield, self.allow_await)
                            .parse(cursor, interner)?
                            .try_into_expression()?;

                    current_node =
                        Binary::new(BinaryOp::Logical(LogicalOp::And), current_node, rhs).into();
                }
                TokenKind::Punctuator(Punctuator::BoolOr) => {
                    if previous == PreviousExpr::Coalesce {
                        return Err(Error::expected(
                            ["??".to_owned()],
                            tok.to_string(interner),
                            tok.span(),
                            "logical expression (cannot use '??' without parentheses within '||' or '&&')",
                        ));
                    }
                    cursor.advance(interner);
                    previous = PreviousExpr::Logical;
                    let rhs = Self::with_previous(
                        self.allow_in,
                        self.allow_yield,
                        self.allow_await,
                        PreviousExpr::Logical,
                    )
                    .parse(cursor, interner)?
                    .try_into_expression()?;
                    current_node =
                        Binary::new(BinaryOp::Logical(LogicalOp::Or), current_node, rhs).into();
                }
                TokenKind::Punctuator(Punctuator::Coalesce) => {
                    if previous == PreviousExpr::Logical {
                        return Err(Error::expected(
                            ["&&".to_owned(), "||".to_owned()],
                            tok.to_string(interner),
                            tok.span(),
                            "cannot use '??' unparenthesized within '||' or '&&'",
                        ));
                    }
                    cursor.advance(interner);
                    previous = PreviousExpr::Coalesce;
                    let rhs =
                        BitwiseORExpression::new(self.allow_in, self.allow_yield, self.allow_await)
                            .parse(cursor, interner)?
                            .try_into_expression()?;
                    current_node =
                        Binary::new(BinaryOp::Logical(LogicalOp::Coalesce), current_node, rhs)
                            .into();
                }
                _ => break,
            }
        }
        Ok(current_node.into())
    }
}

/// Parses the binary operators from `|` to `*`, `/` and `%`: the productions from
/// `BitwiseORExpression` down to `MultiplicativeExpression`, by precedence climbing.
///
/// All of them are left-associative, so one loop parses the operators of one precedence and
/// tighter ones, and only a right operand that binds tighter than its operator recurses. A level
/// of nesting in the source then takes one frame of this parser instead of one per precedence.
///
/// More information:
///  - [MDN documentation][mdn]
///  - [ECMAScript specification][spec]
///
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/Bitwise_Operators#Bitwise_OR
/// [spec]: https://tc39.es/ecma262/#prod-BitwiseORExpression
#[derive(Debug, Clone, Copy)]
struct BitwiseORExpression {
    allow_in: AllowIn,
    allow_yield: AllowYield,
    allow_await: AllowAwait,
}

impl BitwiseORExpression {
    /// Creates a new `BitwiseORExpression` parser.
    pub(super) fn new<I, Y, A>(allow_in: I, allow_yield: Y, allow_await: A) -> Self
    where
        I: Into<AllowIn>,
        Y: Into<AllowYield>,
        A: Into<AllowAwait>,
    {
        Self {
            allow_in: allow_in.into(),
            allow_yield: allow_yield.into(),
            allow_await: allow_await.into(),
        }
    }
}

impl<R> TokenParser<R> for BitwiseORExpression
where
    R: ReadChar,
{
    type Output = FormalParameterListOrExpression;

    fn parse(self, cursor: &mut Cursor<R>, interner: &mut Interner) -> ParseResult<Self::Output> {
        BinaryExpression {
            min: BITWISE_OR,
            allow_in: self.allow_in,
            allow_yield: self.allow_yield,
            allow_await: self.allow_await,
        }
        .parse(cursor, interner)
    }
}

/// The precedences of the operators of [`BinaryExpression`], from the loosest to the tightest.
const BITWISE_OR: u8 = 1;
const BITWISE_XOR: u8 = 2;
const BITWISE_AND: u8 = 3;
const EQUALITY: u8 = 4;
const RELATIONAL: u8 = 5;
const SHIFT: u8 = 6;
const ADDITIVE: u8 = 7;
const MULTIPLICATIVE: u8 = 8;

/// Returns the binary operator `kind` stands for and its precedence, if it is one of the
/// operators of [`BinaryExpression`]; `in` only where `allow_in` permits it.
fn binary_operator(kind: &TokenKind, allow_in: AllowIn) -> Option<(BinaryOp, u8)> {
    match *kind {
        TokenKind::Punctuator(punctuator) => {
            let precedence = match punctuator {
                Punctuator::Or => BITWISE_OR,
                Punctuator::Xor => BITWISE_XOR,
                Punctuator::And => BITWISE_AND,
                Punctuator::Eq
                | Punctuator::NotEq
                | Punctuator::StrictEq
                | Punctuator::StrictNotEq => EQUALITY,
                Punctuator::LessThan
                | Punctuator::GreaterThan
                | Punctuator::LessThanOrEq
                | Punctuator::GreaterThanOrEq => RELATIONAL,
                Punctuator::LeftSh | Punctuator::RightSh | Punctuator::URightSh => SHIFT,
                Punctuator::Add | Punctuator::Sub => ADDITIVE,
                Punctuator::Mul | Punctuator::Div | Punctuator::Mod => MULTIPLICATIVE,
                _ => return None,
            };
            Some((punctuator.as_binary_op()?, precedence))
        }
        TokenKind::Keyword((keyword @ Keyword::InstanceOf, false)) => {
            Some((keyword.as_binary_op()?, RELATIONAL))
        }
        TokenKind::Keyword((keyword @ Keyword::In, false)) if allow_in.0 => {
            Some((keyword.as_binary_op()?, RELATIONAL))
        }
        _ => None,
    }
}

/// Parses an expression of the operators of [`BinaryExpression`] of precedence `min` and tighter.
#[derive(Debug, Clone, Copy)]
struct BinaryExpression {
    min: u8,
    allow_in: AllowIn,
    allow_yield: AllowYield,
    allow_await: AllowAwait,
}

impl BinaryExpression {
    /// The same parser for the operators of precedence `min` and tighter.
    const fn with_min(self, min: u8) -> Self {
        Self { min, ..self }
    }
}

impl<R> TokenParser<R> for BinaryExpression
where
    R: ReadChar,
{
    type Output = FormalParameterListOrExpression;

    fn parse(self, cursor: &mut Cursor<R>, interner: &mut Interner) -> ParseResult<Self::Output> {
        // RelationalExpression : PrivateIdentifier `in` ShiftExpression
        if self.min <= RELATIONAL && self.allow_in.0 {
            let token = cursor.peek(0, interner).or_abrupt()?;
            if let TokenKind::PrivateIdentifier(identifier) = token.kind() {
                let identifier = *identifier;
                let identifier_span = token.span();
                let token = cursor.peek(1, interner).or_abrupt()?;
                match token.kind() {
                    TokenKind::Keyword((Keyword::In, true)) => {
                        return Err(Error::general(
                            "Keyword must not contain escaped characters",
                            token.span().start(),
                        ));
                    }
                    TokenKind::Keyword((Keyword::In, false)) => {
                        cursor.advance(interner);
                        cursor.advance(interner);

                        let rhs = self
                            .with_min(SHIFT)
                            .parse(cursor, interner)?
                            .try_into_expression()?;
                        let lhs = BinaryInPrivate::new(
                            PrivateName::new(identifier, identifier_span),
                            rhs,
                        )
                        .into();

                        // Like a relational operator, it takes no relational operator after it.
                        return self.parse_operators(lhs, EQUALITY, cursor, interner);
                    }
                    _ => {}
                }
            }
        }

        // A MultiplicativeExpression reads the token after its operands in the `Div` goal.
        cursor.set_goal(InputElement::Div);
        let lhs = ExponentiationExpression::new(self.allow_yield, self.allow_await)
            .parse(cursor, interner)?;
        let FormalParameterListOrExpression::Expression(lhs) = lhs else {
            return Ok(lhs);
        };

        self.parse_operators(lhs, MULTIPLICATIVE, cursor, interner)
    }
}

impl BinaryExpression {
    /// Parses the operators of precedence `self.min` to `max` after `lhs` and their right
    /// operands.
    fn parse_operators<R>(
        self,
        mut lhs: ast::Expression,
        max: u8,
        cursor: &mut Cursor<R>,
        interner: &mut Interner,
    ) -> ParseResult<FormalParameterListOrExpression>
    where
        R: ReadChar,
    {
        let levels = self.min..=max;
        let mut links = 0;
        loop {
            links += 1;
            cursor.check_stack_for(links * CHAIN_LINK_STACK)?;
            let Some(token) = cursor.peek(0, interner)? else {
                break;
            };
            if let TokenKind::Keyword((Keyword::InstanceOf | Keyword::In, true)) = token.kind() {
                if levels.contains(&RELATIONAL) {
                    return Err(Error::general(
                        "Keyword must not contain escaped characters",
                        token.span().start(),
                    ));
                }
                break;
            }
            let Some((op, precedence)) = binary_operator(token.kind(), self.allow_in) else {
                break;
            };
            if !levels.contains(&precedence) {
                break;
            }
            cursor.advance(interner);

            let rhs = if precedence == MULTIPLICATIVE {
                ExponentiationExpression::new(self.allow_yield, self.allow_await)
                    .parse(cursor, interner)?
            } else {
                self.with_min(precedence + 1).parse(cursor, interner)?
            };
            lhs = Binary::new(op, lhs, rhs.try_into_expression()?).into();
        }

        Ok(lhs.into())
    }
}

/// Returns an error if `arguments` or `eval` are used as identifier in strict mode.
fn check_strict_arguments_or_eval(ident: Identifier, position: Position) -> ParseResult<()> {
    match ident.sym() {
        Sym::ARGUMENTS => Err(Error::general(
            "unexpected identifier `arguments` in strict mode",
            position,
        )),
        Sym::EVAL => Err(Error::general(
            "unexpected identifier `eval` in strict mode",
            position,
        )),
        _ => Ok(()),
    }
}
