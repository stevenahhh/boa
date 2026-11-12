use crate::{
    bytecompiler::{ByteCompiler, Label, Register},
    vm::opcode::RegisterOperand,
};
use boa_ast::{
    Expression,
    expression::operator::{
        Assign, Binary, BinaryInPrivate, Update,
        binary::{ArithmeticOp, BinaryOp, BitwiseOp, LogicalOp, RelationalOp},
    },
    function::{
        ArrowFunction, AsyncArrowFunction, AsyncFunctionExpression, AsyncGeneratorExpression,
        FunctionExpression, GeneratorExpression,
    },
    visitor::Visitor,
};
use std::ops::ControlFlow;

/// Returns `true` if evaluating `expr` can assign a binding: it contains an assignment or an
/// update expression outside of a nested function body.
///
/// Code in a nested function cannot write a local binding, because a binding referenced from a
/// nested function is no longer local and lives in an environment instead of a register.
fn may_assign_binding(expr: &Expression) -> bool {
    struct AssignFinder;

    impl<'ast> Visitor<'ast> for AssignFinder {
        type BreakTy = ();

        fn visit_assign(&mut self, _: &'ast Assign) -> ControlFlow<()> {
            ControlFlow::Break(())
        }
        fn visit_update(&mut self, _: &'ast Update) -> ControlFlow<()> {
            ControlFlow::Break(())
        }
        fn visit_function_expression(&mut self, _: &'ast FunctionExpression) -> ControlFlow<()> {
            ControlFlow::Continue(())
        }
        fn visit_generator_expression(&mut self, _: &'ast GeneratorExpression) -> ControlFlow<()> {
            ControlFlow::Continue(())
        }
        fn visit_async_function_expression(
            &mut self,
            _: &'ast AsyncFunctionExpression,
        ) -> ControlFlow<()> {
            ControlFlow::Continue(())
        }
        fn visit_async_generator_expression(
            &mut self,
            _: &'ast AsyncGeneratorExpression,
        ) -> ControlFlow<()> {
            ControlFlow::Continue(())
        }
        fn visit_arrow_function(&mut self, _: &'ast ArrowFunction) -> ControlFlow<()> {
            ControlFlow::Continue(())
        }
        fn visit_async_arrow_function(&mut self, _: &'ast AsyncArrowFunction) -> ControlFlow<()> {
            ControlFlow::Continue(())
        }
    }

    AssignFinder.visit_expression(expr).is_break()
}

impl ByteCompiler<'_> {
    /// Compiles the left operand of `lhs <op> rhs` like
    /// [`compile_expr_operand`](Self::compile_expr_operand).
    ///
    /// A local's register is used directly only when `rhs` cannot assign a binding. Otherwise the
    /// value is copied first, so that evaluating `rhs` cannot change the left operand after it
    /// was evaluated (`let a = 1; a + (a = 2, 0)` is `1`).
    pub(crate) fn compile_lhs_operand(
        &mut self,
        lhs: &Expression,
        rhs: &Expression,
        inner_fn: impl FnOnce(&mut Self, RegisterOperand),
    ) {
        if matches!(lhs, Expression::Identifier(_)) && may_assign_binding(rhs) {
            let reg = self.register_allocator.alloc();
            self.compile_expr(lhs, &reg);
            inner_fn(self, reg.variable());
            self.register_allocator.dealloc(reg);
        } else {
            self.compile_expr_operand(lhs, inner_fn);
        }
    }

    pub(crate) fn compile_binary(&mut self, binary: &Binary, dst: &Register) {
        match binary.op() {
            BinaryOp::Arithmetic(op) => {
                self.compile_lhs_operand(binary.lhs(), binary.rhs(), |self_, lhs| {
                    self_.compile_binary_arithmetic(op, binary.rhs(), dst, lhs);
                });
            }
            BinaryOp::Bitwise(op) => {
                self.compile_lhs_operand(binary.lhs(), binary.rhs(), |self_, lhs| {
                    self_.compile_binary_bitwise(op, binary.rhs(), dst, lhs);
                });
            }
            BinaryOp::Relational(op) => {
                self.compile_lhs_operand(binary.lhs(), binary.rhs(), |self_, lhs| {
                    self_.compile_binary_relational(op, binary.rhs(), dst, lhs);
                });
            }
            BinaryOp::Logical(op) => {
                self.compile_expr(binary.lhs(), dst);
                let exit = self.next_opcode_location();
                match op {
                    LogicalOp::And => self
                        .bytecode
                        .emit_logical_and(Self::DUMMY_ADDRESS, dst.variable()),
                    LogicalOp::Or => self
                        .bytecode
                        .emit_logical_or(Self::DUMMY_ADDRESS, dst.variable()),
                    LogicalOp::Coalesce => self
                        .bytecode
                        .emit_coalesce(Self::DUMMY_ADDRESS, dst.variable()),
                }
                self.compile_expr(binary.rhs(), dst);
                self.patch_jump(Label { index: exit });
            }
            BinaryOp::Comma => {
                // Evaluate LHS for side effects, then RHS is the result.
                self.compile_expr_operand(binary.lhs(), |_, _| {});
                self.compile_expr(binary.rhs(), dst);
            }
        }
    }

    fn compile_binary_arithmetic(
        &mut self,
        op: ArithmeticOp,
        rhs_expr: &Expression,
        dst: &Register,
        lhs: RegisterOperand,
    ) {
        self.compile_expr_operand(rhs_expr, |self_, rhs| {
            let bytecode = &mut self_.bytecode;
            match op {
                ArithmeticOp::Add => bytecode.emit_add(dst.variable(), lhs, rhs),
                ArithmeticOp::Sub => bytecode.emit_sub(dst.variable(), lhs, rhs),
                ArithmeticOp::Div => bytecode.emit_div(dst.variable(), lhs, rhs),
                ArithmeticOp::Mul => bytecode.emit_mul(dst.variable(), lhs, rhs),
                ArithmeticOp::Exp => bytecode.emit_pow(dst.variable(), lhs, rhs),
                ArithmeticOp::Mod => bytecode.emit_mod(dst.variable(), lhs, rhs),
            }
        });
    }

    fn compile_binary_bitwise(
        &mut self,
        op: BitwiseOp,
        rhs_expr: &Expression,
        dst: &Register,
        lhs: RegisterOperand,
    ) {
        self.compile_expr_operand(rhs_expr, |self_, rhs| {
            let bytecode = &mut self_.bytecode;
            match op {
                BitwiseOp::And => bytecode.emit_bit_and(dst.variable(), lhs, rhs),
                BitwiseOp::Or => bytecode.emit_bit_or(dst.variable(), lhs, rhs),
                BitwiseOp::Xor => bytecode.emit_bit_xor(dst.variable(), lhs, rhs),
                BitwiseOp::Shl => {
                    bytecode.emit_shift_left(dst.variable(), lhs, rhs);
                }
                BitwiseOp::Shr => {
                    bytecode.emit_shift_right(dst.variable(), lhs, rhs);
                }
                BitwiseOp::UShr => {
                    bytecode.emit_unsigned_shift_right(dst.variable(), lhs, rhs);
                }
            }
        });
    }

    fn compile_binary_relational(
        &mut self,
        op: RelationalOp,
        rhs_expr: &Expression,
        dst: &Register,
        lhs: RegisterOperand,
    ) {
        self.compile_expr_operand(rhs_expr, |self_, rhs| {
            let bytecode = &mut self_.bytecode;
            match op {
                RelationalOp::Equal => bytecode.emit_eq(dst.variable(), lhs, rhs),
                RelationalOp::NotEqual => {
                    bytecode.emit_not_eq(dst.variable(), lhs, rhs);
                }
                RelationalOp::StrictEqual => {
                    bytecode.emit_strict_eq(dst.variable(), lhs, rhs);
                }
                RelationalOp::StrictNotEqual => {
                    bytecode.emit_strict_not_eq(dst.variable(), lhs, rhs);
                }
                RelationalOp::GreaterThan => {
                    bytecode.emit_greater_than(dst.variable(), lhs, rhs);
                }
                RelationalOp::GreaterThanOrEqual => {
                    bytecode.emit_greater_than_or_eq(dst.variable(), lhs, rhs);
                }
                RelationalOp::LessThan => {
                    bytecode.emit_less_than(dst.variable(), lhs, rhs);
                }
                RelationalOp::LessThanOrEqual => {
                    bytecode.emit_less_than_or_eq(dst.variable(), lhs, rhs);
                }
                RelationalOp::In => bytecode.emit_in(dst.variable(), lhs, rhs),
                RelationalOp::InstanceOf => {
                    bytecode.emit_instance_of(dst.variable(), lhs, rhs);
                }
            }
        });
    }

    pub(crate) fn compile_binary_in_private(&mut self, binary: &BinaryInPrivate, dst: &Register) {
        let index = self.get_or_insert_private_name(*binary.lhs());
        self.compile_expr(binary.rhs(), dst);
        self.bytecode
            .emit_in_private(dst.variable(), index.into(), dst.variable());
    }
}
