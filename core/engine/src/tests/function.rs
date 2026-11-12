use crate::{JsNativeErrorKind, JsValue, TestAction, run_test_actions};
use boa_macros::js_str;
use indoc::indoc;

#[test]
fn function_declaration_returns_undefined() {
    run_test_actions([TestAction::assert_eq(
        "function abc() {}",
        JsValue::undefined(),
    )]);
}

#[test]
fn empty_function_returns_undefined() {
    run_test_actions([TestAction::assert_eq(
        "(function () {}) ()",
        JsValue::undefined(),
    )]);
}

#[test]
fn implicit_return_does_not_leak_last_evaluated_expression() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            let seen;

            function g(inner) {
                seen = inner;
            }

            function f() {}

            5;
            let outer = g(f());
            seen;
        "#},
        JsValue::undefined(),
    )]);
}

#[test]
fn implicit_constructor_return_uses_constructed_object() {
    run_test_actions([TestAction::assert(indoc! {r#"
        let seen;

        function g(inner) {
            seen = inner;
        }

        function f() {}

        ({});
        let outer = g(new f());
        Object.getPrototypeOf(seen) === f.prototype;
    "#})]);
}

#[test]
fn implicit_return_does_not_leak_from_previous_eval() {
    run_test_actions([
        TestAction::run("function f() {}"),
        TestAction::run("let seen; function g(inner) { seen = inner; }"),
        TestAction::run("5;"),
        TestAction::run("let outer = g(f());"),
        TestAction::assert_eq("seen", JsValue::undefined()),
    ]);
}

#[test]
fn implicit_return_does_not_leak_completion_value_from_same_eval() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            eval(`
                function f() {}
                globalThis.seen = "initial";
                function g(inner) { globalThis.seen = inner; }
                5;
                let outer = g(f());
            `);
        "#}),
        TestAction::assert_eq("globalThis.seen", JsValue::undefined()),
    ]);
}

#[test]
fn implicit_constructor_return_does_not_leak_completion_value_from_same_eval() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            eval(`
                function f() {}
                globalThis.prototype_matches = false;
                function g(inner) {
                    globalThis.prototype_matches = Object.getPrototypeOf(inner) === f.prototype;
                }
                ({});
                let outer = g(new f());
            `);
        "#}),
        TestAction::assert("globalThis.prototype_matches"),
    ]);
}

/// Regression test for issue #4485.
/// Checks that generator resumption via `g.next(f())` is not affected
/// by resetting the accumulator in `push_frame`.
#[test]
fn generator_resumption_not_affected_by_return_value_reset() {
    run_test_actions([TestAction::assert(indoc! {r#"
        let seen;

        function* gen() {
            seen = yield 1;
        }

        function f() {}

        5;
        var g = gen();
        g.next();
        g.next(f());
        seen === undefined;
    "#})]);
}

/// Regression test for issue #4485.
/// Checks that generator resumption via `g.next(new f())` receives
/// the constructed object, not a stale caller expression.
#[test]
fn generator_resumption_with_constructor_not_affected_by_return_value_reset() {
    run_test_actions([TestAction::assert(indoc! {r#"
        let seen;

        function* gen() {
            seen = yield 1;
        }

        function f() {}

        ({});
        var g = gen();
        g.next();
        g.next(new f());
        Object.getPrototypeOf(seen) === f.prototype;
    "#})]);
}

#[test]
fn property_accessor_member_expression_dot_notation_on_function() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            function asd () {};
            asd.name;
        "#},
        js_str!("asd"),
    )]);
}

#[test]
fn property_accessor_member_expression_bracket_notation_on_function() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            function asd () {};
            asd['name'];
        "#},
        js_str!("asd"),
    )]);
}

#[test]
fn early_return() {
    run_test_actions([
        TestAction::assert(indoc! {r#"
                function early_return() {
                    if (true) {
                        return true;
                    }
                    return false;
                }
                early_return()
            "#}),
        TestAction::assert_eq(
            indoc! {r#"
                function nested_fnct() {
                    return "nested";
                }
                function outer_fnct() {
                    nested_fnct();
                    return "outer";
                }
                outer_fnct()
            "#},
            js_str!("outer"),
        ),
    ]);
}

#[test]
fn should_set_this_value() {
    run_test_actions([
        TestAction::run(indoc! {r#"
                function Foo() {
                    this.a = "a";
                    this.b = "b";
                }

                var bar = new Foo();
            "#}),
        TestAction::assert_eq("bar.a", js_str!("a")),
        TestAction::assert_eq("bar.b", js_str!("b")),
    ]);
}

#[test]
fn should_type_error_when_new_is_not_constructor() {
    run_test_actions([TestAction::assert_native_error(
        "new ''()",
        JsNativeErrorKind::Type,
        "not a constructor",
    )]);
}

#[test]
fn new_instance_should_point_to_prototype() {
    // A new instance should point to a prototype object created with the constructor function
    run_test_actions([
        TestAction::run(indoc! {r#"
                function Foo() {}
                var bar = new Foo();
            "#}),
        TestAction::assert("Object.getPrototypeOf(bar) == Foo.prototype "),
    ]);
}

#[test]
fn calling_function_with_unspecified_arguments() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            function test(a, b) {
                return b;
            }

            test(10)
        "#},
        JsValue::undefined(),
    )]);
}

#[test]
fn not_a_function() {
    run_test_actions([
        TestAction::run(indoc! {r#"
                let a = {};
                let b = true;
            "#}),
        TestAction::assert_native_error("a()", JsNativeErrorKind::Type, "not a callable function"),
        TestAction::assert_native_error(
            "a.a()",
            JsNativeErrorKind::Type,
            "not a callable function",
        ),
        TestAction::assert_native_error("b()", JsNativeErrorKind::Type, "not a callable function"),
    ]);
}

#[test]
fn strict_mode_dup_func_parameters() {
    // Checks that a function cannot contain duplicate parameter
    // names in strict mode code as per https://tc39.es/ecma262/#sec-function-definitions-static-semantics-early-errors.
    run_test_actions([TestAction::assert_native_error(
        indoc! {r#"
            'use strict';
            function f(a, b, b) {}
        "#},
        JsNativeErrorKind::Syntax,
        "Duplicate parameter name not allowed in this context at line 2, col 12",
    )]);
}

#[test]
fn duplicate_function_name() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            function f () {}
            function f () {return 12;}
            f()
        "#},
        12,
    )]);
}

#[test]
fn eval_out_of_scope() {
    run_test_actions([
        TestAction::run(indoc! {
        r#"
            function f() {
                var x = null;
                return (function() {
                    return eval("x");
                })();
            }
        "#
        }),
        TestAction::assert_eq("f()", JsValue::null()),
    ]);
}

/// Regression test for issue #4531.
/// `Function` constructor with nested function containing lexical bindings
/// captured by a closure should not panic with "must be declarative environment".
#[test]
fn function_constructor_nested_lexical_binding() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            var code = "\
                function f() {\
                    const a = 42;\
                    return () => { return a; };\
                }\
                return f()();\
            ";
            Function(code)();
        "#},
        42,
    )]);
}

#[test]
fn arrow_functions_nested_in_arrow_functions_see_the_this_of_the_enclosing_function() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            class A {
                depth2() { const outer = () => { const inner = () => this; return inner(); }; return outer(); }
                depth3() { return (() => () => () => this)()()(); }
                depth4() { return (() => () => () => () => this)()()()(); }
                later() { return () => () => this; }
                get getter() { return (() => () => this)()(); }
                static staticMethod() { return (() => () => this)()(); }
                callback() { return [1].map(() => { const t = () => this; return t(); })[0]; }
                async asyncArrows() { const outer = async () => { const inner = async () => this; return inner(); }; return outer(); }
                executor() { return new Promise((resolve) => { const t = () => this; resolve(t()); }); }
                field = () => { const g = () => this; return g(); };
                static { A.fromStaticBlock = (() => () => this)()(); }
            }
            class B extends A {
                constructor() {
                    const early = () => () => this;
                    super();
                    this.afterSuper = (() => () => this)()();
                    this.early = early()();
                }
            }
            const a = new A();
            const b = new B();
            const object = { m() { return (() => () => this)()(); } };
            function plain() { return (() => () => this)()(); }
            let asyncResult = "pending";
            let executorResult = "pending";
            a.asyncArrows().then((v) => { asyncResult = v === a; });
            a.executor().then((v) => { executorResult = v === a; });
        "#}),
        TestAction::assert("a.depth2() === a"),
        TestAction::assert("a.depth3() === a"),
        TestAction::assert("a.depth4() === a"),
        TestAction::assert("a.later()()() === a"),
        TestAction::assert("a.getter === a"),
        TestAction::assert("A.staticMethod() === A"),
        TestAction::assert("a.callback() === a"),
        TestAction::assert("a.field() === a"),
        TestAction::assert("A.fromStaticBlock === A"),
        TestAction::assert("b.afterSuper === b && b.early === b"),
        TestAction::assert("object.m() === object"),
        TestAction::assert("plain.call(object) === object"),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert("asyncResult === true && executorResult === true"),
    ]);
}

#[test]
fn direct_eval_in_arrow_functions_sees_the_enclosing_function() {
    run_test_actions([
        TestAction::assert_eq(
            "(function (a, b) { return eval('arguments.length'); })(1, 2)",
            2,
        ),
        TestAction::assert_eq(
            "(function (a, b) { return (() => eval('arguments.length'))(); })(1, 2)",
            2,
        ),
        TestAction::assert_eq(
            "(function (a, b) { return (() => () => eval('arguments.length'))()(); })(1, 2, 3)",
            3,
        ),
        TestAction::assert_eq(
            "(function () { return (() => () => () => eval('arguments[3]'))()()(); })(1, 2, 3, 'four')",
            js_str!("four"),
        ),
        // A sloppy function maps its parameters to the arguments object, a strict one does not.
        TestAction::assert_eq(
            "(function (a) { (() => eval('arguments[0] = 5'))(); return a; })(1)",
            5,
        ),
        TestAction::assert_eq(
            "(function (a) { 'use strict'; return (() => eval('arguments[0] = 5; a'))(); })(1)",
            1,
        ),
        TestAction::assert_eq(
            "(function (arguments) { return (() => eval('arguments'))(); })(7)",
            7,
        ),
        TestAction::assert_eq(
            indoc! {r#"
                function e7() {
                    return (() => { var q = 1; if (q) { q++; } return eval('this.tag + arguments.length + q'); })();
                }
                e7.call({ tag: 'T' }, 1, 2)
            "#},
            js_str!("T22"),
        ),
        TestAction::assert_eq("(() => eval('typeof arguments'))()", js_str!("undefined")),
        TestAction::assert_eq(
            "new (function () { this.v = (() => () => () => eval('typeof new.target'))()()(); })().v",
            js_str!("function"),
        ),
        TestAction::assert_eq(
            "(function () { return (() => () => () => eval('this.k'))()()(); }).call({ k: 9 })",
            9,
        ),
        TestAction::assert_eq(
            "({ __proto__: { x: 'px' }, m() { return (() => () => () => eval('super.x'))()()(); } }).m()",
            js_str!("px"),
        ),
        TestAction::assert_eq(
            indoc! {r#"
                class B { get x() { return 'bx'; } }
                class C extends B { m() { return (() => () => eval('super.x'))()(); } }
                new C().m()
            "#},
            js_str!("bx"),
        ),
    ]);
}
