//! Upper bounds on the size of the AST's node enums: every slot that holds an expression or a
//! statement pays for the largest inline variant, so a large variant must stay boxed.

#![allow(unused_crate_dependencies)]

use boa_ast::{Expression, Statement};

#[test]
fn expression_and_statement_stay_small() {
    let expression = size_of::<Expression>();
    let statement = size_of::<Statement>();
    assert!(expression <= 56, "Expression is {expression} bytes");
    assert!(statement <= 88, "Statement is {statement} bytes");
}
