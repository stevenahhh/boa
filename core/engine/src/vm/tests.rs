use crate::error::RuntimeLimitError;
use crate::vm::CallFrame;
use crate::vm::call_frame::CallFrameLocation;
use crate::vm::source_info::SourcePath;
use crate::{
    Context, JsNativeErrorKind, JsValue, NativeFunction, Script, TestAction, js_string,
    property::Attribute, run_test_actions, run_test_actions_with,
};
use boa_ast::Position;
use boa_macros::js_str;
use boa_parser::Source;
use indoc::indoc;

#[test]
fn typeof_string() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            const a = "hello";
            typeof a;
        "#},
        js_str!("string"),
    )]);
}

#[test]
fn typeof_number() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            let a = 1234;
            typeof a;
        "#},
        js_str!("number"),
    )]);
}

#[test]
fn basic_op() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            const a = 1;
            const b = 2;
            a + b
        "#},
        3,
    )]);
}

#[test]
fn position() {
    let context = &mut Context::default();
    context
        .register_global_callable(
            js_string!("check_stack"),
            2,
            NativeFunction::from_copy_closure(|_, _, context| {
                let frame = context.stack_trace().collect::<Vec<&CallFrame>>();

                assert_eq!(frame.len(), 4);
                assert_eq!(
                    frame[0].position(),
                    CallFrameLocation {
                        function_name: js_string!("myOtherFunction"),
                        path: SourcePath::None,
                        position: Some(Position::new(2, 16))
                    }
                );
                assert_eq!(
                    frame[1].position(),
                    CallFrameLocation {
                        function_name: js_string!("<eval>"),
                        path: SourcePath::Eval,
                        position: Some(Position::new(1, 16))
                    }
                );
                assert_eq!(
                    frame[2].position(),
                    CallFrameLocation {
                        function_name: js_string!("myFunction"),
                        path: SourcePath::None,
                        position: Some(Position::new(5, 9))
                    }
                );
                assert_eq!(
                    frame[3].position(),
                    CallFrameLocation {
                        function_name: js_string!("<main>"),
                        path: SourcePath::None,
                        position: Some(Position::new(8, 11))
                    }
                );
                Ok(JsValue::undefined())
            }),
        )
        .expect("Could not register function");
    run_test_actions_with(
        [TestAction::run(indoc! {r#"
            const myOtherFunction = () => {
                check_stack();
            };
            function myFunction() {
                eval("myOtherFunction()");
            }

            myFunction();
        "#})],
        context,
    );
}

#[test]
fn try_catch_finally_from_init() {
    // the initialisation of the array here emits a PopOnReturnAdd op
    //
    // here we test that the stack is not popped more than intended due to multiple catches in the
    // same function, which could lead to VM stack corruption
    run_test_actions([TestAction::assert_opaque_error(
        indoc! {r#"
            try {
                [(() => {throw "h";})()];
            } catch (x) {
                throw "h";
            } finally {
            }
        "#},
        js_str!("h"),
    )]);
}

#[test]
fn multiple_catches() {
    // see explanation on `try_catch_finally_from_init`
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            try {
                try {
                    [(() => {throw "h";})()];
                } catch (x) {
                    throw "h";
                }
            } catch (y) {
            }
        "#},
        JsValue::undefined(),
    )]);
}

#[test]
fn use_last_expr_try_block() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            try {
                19;
                7.5;
                "Hello!";
            } catch (y) {
                14;
                "Bye!"
            }
        "#},
        js_str!("Hello!"),
    )]);
}

#[test]
fn use_last_expr_catch_block() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            try {
                throw Error("generic error");
                19;
                7.5;
            } catch (y) {
                14;
                "Hello!";
            }
        "#},
        js_str!("Hello!"),
    )]);
}

#[test]
fn no_use_last_expr_finally_block() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            try {
            } catch (y) {
            } finally {
                "Unused";
            }
        "#},
        JsValue::undefined(),
    )]);
}

#[test]
fn finally_block_binding_env() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            let buf = "Hey hey";
            try {
            } catch (y) {
            } finally {
                let x = " people";
                buf += x;
            }
            buf
        "#},
        js_str!("Hey hey people"),
    )]);
}

#[test]
fn run_super_method_in_object() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            let proto = {
                m() { return "super"; }
            };
            let obj = {
                v() { return super.m(); }
            };
            Object.setPrototypeOf(obj, proto);
            obj.v();
        "#},
        js_str!("super"),
    )]);
}

#[test]
fn get_reference_by_super() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            var fromA, fromB;
            var A = { fromA: 'a', fromB: 'a' };
            var B = { fromB: 'b' };
            Object.setPrototypeOf(B, A);
            var obj = {
                fromA: 'c',
                fromB: 'c',
                method() {
                    fromA = (() => { return super.fromA; })();
                    fromB = (() => { return super.fromB; })();
                }
            };
            Object.setPrototypeOf(obj, B);
            obj.method();
            fromA + fromB
        "#},
        js_str!("ab"),
    )]);
}

#[test]
fn super_call_constructor_null() {
    run_test_actions([TestAction::assert_native_error(
        indoc! {r#"
            class A extends Object {
                constructor() {
                    Object.setPrototypeOf(A, null);
                    super(A);
                }
            }
            new A();
        "#},
        JsNativeErrorKind::Type,
        "super constructor object must be constructor",
    )]);
}

#[test]
fn super_call_get_constructor_before_arguments_execution() {
    run_test_actions([TestAction::assert(indoc! {r#"
        class A extends Object {
            constructor() {
                super(Object.setPrototypeOf(A, null));
            }
        }
        new A() instanceof A;
    "#})]);
}

#[test]
fn order_of_execution_in_assignment() {
    run_test_actions([
        TestAction::run(indoc! {r#"
                let i = 0;
                let array = [[]];

                array[i++][i++] = i++;
            "#}),
        TestAction::assert_eq("i", 3),
        TestAction::assert_eq("array.length", 1),
        TestAction::assert_eq("array[0].length", 2),
    ]);
}

#[test]
fn order_of_execution_in_assignment_with_comma_expressions() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            let result = "";
            function f(i) {
                result += i;
            }
            let a = [[]];
            (f(1), a)[(f(2), 0)][(f(3), 0)] = (f(4), 123);
            result
        "#},
        js_str!("1234"),
    )]);
}

#[test]
fn loop_runtime_limit() {
    run_test_actions([
        TestAction::assert_eq(
            indoc! {r#"
                for (let i = 0; i < 20; ++i) { }
            "#},
            JsValue::undefined(),
        ),
        TestAction::inspect_context(|context| {
            context.runtime_limits_mut().set_loop_iteration_limit(10);
        }),
        TestAction::assert_runtime_limit_error(
            indoc! {r#"
                for (let i = 0; i < 20; ++i) { }
            "#},
            RuntimeLimitError::LoopIteration,
        ),
        TestAction::assert_eq(
            indoc! {r#"
                for (let i = 0; i < 10; ++i) { }
            "#},
            JsValue::undefined(),
        ),
        TestAction::assert_runtime_limit_error(
            indoc! {r#"
                while (1) { }
            "#},
            RuntimeLimitError::LoopIteration,
        ),
        TestAction::assert_runtime_limit_error(
            indoc! {r#"
                try { while (1) { } } catch (e) { }
            "#},
            RuntimeLimitError::LoopIteration,
        ),
    ]);
}

#[test]
fn recursion_runtime_limit() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            function factorial(n) {
                if (n == 0) {
                    return 1;
                }

                return n * factorial(n - 1);
            }
        "#}),
        TestAction::assert_eq("factorial(8)", JsValue::new(40_320)),
        TestAction::assert_eq("factorial(11)", JsValue::new(39_916_800)),
        TestAction::inspect_context(|context| {
            context.runtime_limits_mut().set_recursion_limit(10);
        }),
        TestAction::assert_native_error(
            "factorial(11)",
            JsNativeErrorKind::Range,
            "Maximum call stack size exceeded",
        ),
        TestAction::assert_eq("factorial(8)", JsValue::new(40_320)),
        TestAction::assert_native_error(
            indoc! {r#"
                function x() {
                    x()
                }

                x()
            "#},
            JsNativeErrorKind::Range,
            "Maximum call stack size exceeded",
        ),
        TestAction::assert_eq(
            indoc! {r#"
                let first = "none", second = "none";
                try { x(); } catch (e) { first = e instanceof RangeError; }
                try { x(); } catch (e) { second = e.name; }
                `${first} ${second} ${factorial(8)}`
            "#},
            js_str!("true RangeError 40320"),
        ),
    ]);
}

#[test]
fn stack_size_limit_throws_a_catchable_range_error() {
    run_test_actions([
        TestAction::inspect_context(|context| {
            context.runtime_limits_mut().set_stack_size_limit(1024);
        }),
        TestAction::assert_eq(
            indoc! {r#"
                function x() { x(); }
                let name = "none";
                try { x(); } catch (e) { name = e.name; }
                name
            "#},
            js_str!("RangeError"),
        ),
    ]);
}

#[test]
fn caught_stack_overflow_leaves_room_for_the_next_one() {
    run_test_actions([
        TestAction::inspect_context(|context| {
            context.runtime_limits_mut().set_recursion_limit(100_000);
            context.runtime_limits_mut().set_stack_size_limit(4096);
        }),
        TestAction::assert_eq(
            indoc! {r#"
                let depth = 0;
                function cnt(a, b, c) { let x = a, y = b, z = c; depth++; cnt(x, y, z); }
                function overflow() {
                    depth = 0;
                    try { cnt(1, 2, 3); } catch (e) { return e instanceof RangeError ? depth : -1; }
                }
                const depths = [];
                for (let i = 0; i < 8; i++) { depths.push(overflow()); }
                let nested = -1;
                try { cnt(1, 2, 3); } catch (e) { nested = overflow(); }
                [depths[0] > 100, depths.every((d) => d === depths[0]), nested === depths[0]].join()
            "#},
            js_str!("true,true,true"),
        ),
    ]);
}

#[test]
fn stack_overflow_returned_to_the_host_leaves_its_stack_as_it_was() {
    let context = &mut Context::default();
    context.runtime_limits_mut().set_recursion_limit(100_000);
    context.runtime_limits_mut().set_stack_size_limit(1024);
    // The overflow unwinds the frames of g before it reaches the frame of f, the one the call
    // from the host runs.
    let f = context
        .eval(Source::from_bytes(
            "function f() { return g(0); } function g(n) { let m = n + 1; return g(m); } f",
        ))
        .expect("script must run");
    let f = f.as_callable().expect("f must be callable").clone();
    let stack_len = context.vm.stack.stack.len();
    for _ in 0..4 {
        let error = f
            .call(&JsValue::undefined(), &[], context)
            .expect_err("the recursion must overflow");
        assert!(
            error
                .as_native()
                .is_some_and(|e| matches!(e.kind(), JsNativeErrorKind::Range))
        );
        assert_eq!(context.vm.stack.stack.len(), stack_len);
    }
}

#[test]
fn stack_overflow_caught_again_and_again_on_a_small_thread() {
    // render runs scripts on a thread with std's default stack of 2 MiB, with these limits.
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let context = &mut Context::default();
            context.runtime_limits_mut().set_recursion_limit(10_000);
            context.runtime_limits_mut().set_stack_size_limit(1_000_000);
            let result = context.eval(Source::from_bytes(indoc! {r#"
                function viaMap() { return [0].map(viaMap)[0]; }
                function plain(n) { return plain(n + 1) + 1; }
                const names = [];
                for (let i = 0; i < 20; i++) {
                    try { viaMap(); } catch (e) {
                        try { plain(0); } catch (e2) { names.push(e.name + e2.name); }
                    }
                }
                names.length + " " + names.every((n) => n === "RangeErrorRangeError")
            "#}));
            assert_eq!(result, Ok(js_str!("20 true").into()));
        })
        .expect("spawn a thread")
        .join()
        .expect("the thread must not panic");
}

#[test]
fn stack_overflow_through_native_code_throws_a_catchable_range_error() {
    run_test_actions([
        TestAction::inspect_context(|context| {
            context.runtime_limits_mut().set_recursion_limit(128);
        }),
        TestAction::assert_eq(
            indoc! {r#"
                const results = [];
                const shapes = {
                  call: () => { function f() { return f.call(null); } f(); },
                  map: () => { function f() { return [0].map(f)[0]; } f(); },
                  getter: () => { const o = { get x() { return this.x; } }; o.x; },
                  proxy: () => { const p = new Proxy({}, { get(t, k, r) { return r[k]; } }); p.x; },
                  sort: () => { function f() { [2, 1].sort(() => { f(); return 0; }); } f(); },
                  json: () => { const o = { toJSON() { return JSON.stringify(o); } }; JSON.stringify(o); },
                  generator: () => { function* g() { yield* g(); } g().next(); },
                  eval: () => { function f() { eval("f()"); } f(); },
                };
                for (const [name, run] of Object.entries(shapes)) {
                  try { run(); results.push(name + ":none"); } catch (e) { results.push(name + ":" + e.name); }
                }
                results.join(" ")
            "#},
            js_str!(
                "call:RangeError map:RangeError getter:RangeError proxy:RangeError \
                 sort:RangeError json:RangeError generator:RangeError eval:RangeError"
            ),
        ),
    ]);
}

#[test]
fn native_stack_limit_stops_recursion_through_native_code() {
    run_test_actions([
        TestAction::inspect_context(|context| {
            // Without the native stack limit, this recursion stops at the recursion limit after
            // 63 levels, each a JavaScript frame and a nested run, far from overflowing.
            context.runtime_limits_mut().set_recursion_limit(128);
            // Parsing takes more than 8 KiB of native stack too, so the script is parsed first.
            let script = Script::parse(
                Source::from_bytes(indoc! {r#"
                    let depth = 0;
                    function f(n) { depth = n; return [n + 1].map(f)[0]; }
                    let name = "none";
                    try { f(0); } catch (e) { name = e.name; }
                    `${name} ${depth < 40}`
                "#}),
                None,
                context,
            )
            .expect("the script must parse");
            context
                .runtime_limits_mut()
                .set_native_stack_limit(8 * 1024);
            assert_eq!(
                script.evaluate(context),
                Ok(js_str!("RangeError true").into())
            );
        }),
        TestAction::inspect_context(|context| {
            context
                .runtime_limits_mut()
                .set_native_stack_limit(crate::vm::RuntimeLimits::default().native_stack_limit());
        }),
        TestAction::assert_eq(
            "function d(n) { return n === 0 ? 0 : 1 + [n - 1].map(d)[0]; } d(5)",
            5,
        ),
    ]);
}

#[test]
fn arguments_object_constructor_valid_index() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            let args;
            function F(a = 1) {
                args = arguments;
            }
            new F();
            typeof args
        "#},
        js_str!("object"),
    )]);
}

#[test]
fn empty_return_values() {
    run_test_actions([
        TestAction::run(indoc! {r#"do {{}} while (false);"#}),
        TestAction::run(indoc! {r#"do try {{}} catch {} while (false);"#}),
        TestAction::run(indoc! {r#"do {} while (false);"#}),
        TestAction::run(indoc! {r#"do try {{}{}} catch {} while (false);"#}),
        TestAction::run(indoc! {r#"do {{}{}} while (false);"#}),
        TestAction::run(indoc! {r#"do {;{}} while (false);"#}),
        TestAction::run(indoc! {r#"do {e: {}} while (false);"#}),
        TestAction::run(indoc! {r#"do {e: ;} while (false);"#}),
        TestAction::run(indoc! {r#"do { break } while (false);"#}),
        TestAction::run(indoc! {r#"while (true) a: break"#}),
        TestAction::run(indoc! {r#"while (true) a: {"a"; break};"#}),
        TestAction::run(indoc! {r#"do {"a";{}} while (false);"#}),
        TestAction::run(indoc! {r#"
            switch (false) {
                default: {}
            }
        "#}),
        TestAction::run(indoc! {r#"
            switch (false) {
                default: {}{}
            }
        "#}),
        TestAction::run(indoc! {r#"
            switch (false) {
                default: ;{}{}
            }
        "#}),
    ]);
}

#[test]
fn truncate_environments_on_non_caught_native_error() {
    let source = "with (new Proxy({}, {has: p => false})) {a}";
    run_test_actions([
        TestAction::assert_native_error(source, JsNativeErrorKind::Reference, "a is not defined"),
        TestAction::assert_native_error(source, JsNativeErrorKind::Reference, "a is not defined"),
    ]);
}

#[test]
fn super_construction_with_parameter_expression() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            class Person {
                constructor(name) {
                    this.name = name;
                }
            }

            class Student extends Person {
                constructor(name = 'unknown') {
                    super(name);
                }
            }
        "#}),
        TestAction::assert_eq("new Student().name", js_str!("unknown")),
        TestAction::assert_eq("new Student('Jack').name", js_str!("Jack")),
    ]);
}

#[test]
fn cross_context_function_call() {
    let context1 = &mut Context::default();
    let result = context1.eval(Source::from_bytes(indoc! {r"
        var global = 100;

        (function x() {
            return global;
        })
    "}));

    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(result.is_callable());

    let context2 = &mut Context::default();

    context2
        .register_global_property(js_string!("func"), result, Attribute::all())
        .unwrap();

    let result = context2.eval(Source::from_bytes("func()"));

    assert_eq!(result, Ok(JsValue::new(100)));
}

// See: https://github.com/boa-dev/boa/issues/1848
#[test]
fn long_object_chain_gc_trace_stack_overflow() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            let old = {};
            for (let i = 0; i < 100000; i++) {
                old = { old };
            }
        "#}),
        TestAction::inspect_context(|_| boa_gc::force_collect()),
    ]);
}

// See: https://github.com/boa-dev/boa/issues/4515
#[test]
fn recursion_in_async_gen_rejects_with_a_range_error() {
    run_test_actions([
        TestAction::inspect_context(|context| {
            context.runtime_limits_mut().set_recursion_limit(128);
        }),
        TestAction::run(indoc! {r#"
            async function* f() {}
            let reason = "pending";
            f().return({
              get then() {
                this.then;
              },
            }).then(() => { reason = "fulfilled"; }, (e) => { reason = e.name; });
        "#}),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("reason", js_str!("RangeError")),
    ]);
}

#[test]
fn recursion_in_setter_throws_a_range_error() {
    run_test_actions([
        TestAction::inspect_context(|context| {
            context.runtime_limits_mut().set_recursion_limit(128);
        }),
        TestAction::assert_native_error(
            indoc! {r#"
                const obj = {
                  set x(value) {
                    this.x = value;
                  },
                };
                obj.x = 1;
            "#},
            JsNativeErrorKind::Range,
            "Maximum call stack size exceeded",
        ),
    ]);
}

/// Environment variable that tells a run of the test binary which deep nesting case to run.
const DEEP_NESTING_CASE: &str = "BOA_DEEP_NESTING_CASE";

/// Scripts that nest deeper than the native stack the engine may use, by default half of the
/// stack of a thread spawned by std, and the error each must end with.
fn deep_nesting_cases() -> Vec<(&'static str, String, &'static str)> {
    let script = "[".repeat(100_000) + &"]".repeat(100_000);
    [
        ("script", script.as_str(), "RangeError"),
        ("stringify", "let a = []; for (let i = 0; i < 200000; i++) a = [a]; JSON.stringify(a)", "RangeError"),
        ("stringify indented", "let o = {}; for (let i = 0; i < 200000; i++) o = { o }; JSON.stringify(o, null, 1)", "RangeError"),
        ("flat", "let a = []; for (let i = 0; i < 200000; i++) a = [a]; a.flat(Infinity)", "RangeError"),
        ("eval array", "eval('['.repeat(100000) + ']'.repeat(100000))", "RangeError"),
        ("eval parentheses", "eval('('.repeat(100000) + '1' + ')'.repeat(100000))", "RangeError"),
        ("function body", "Function('return ' + '['.repeat(100000) + ']'.repeat(100000))", "RangeError"),
        ("function parameters", "Function('a = ' + '['.repeat(100000) + ']'.repeat(100000), '')", "RangeError"),
        ("unary", "eval('!'.repeat(100000) + '1')", "RangeError"),
        ("exponent", "eval('2' + '**1'.repeat(100000))", "RangeError"),
        ("new", "eval('new '.repeat(100000) + 'Object')", "RangeError"),
        ("blocks", "eval('{'.repeat(100000) + '}'.repeat(100000))", "RangeError"),
        ("if", "eval('if (1) '.repeat(100000) + ';')", "RangeError"),
        ("functions", "eval('(function () {'.repeat(20000) + '})'.repeat(20000))", "RangeError"),
        ("var pattern", "eval('var ' + '['.repeat(50000) + 'x' + ']'.repeat(50000) + ' = [];')", "RangeError"),
        ("assignment pattern", "var x; eval('['.repeat(50000) + 'x' + ']'.repeat(50000) + ' = [];')", "RangeError"),
        ("object pattern", "eval('var ' + '{a:'.repeat(50000) + 'x' + '}'.repeat(50000) + ' = {};')", "RangeError"),
        ("to string", "let a = []; for (let i = 0; i < 200000; i++) a = [a]; String(a)", "RangeError"),
        ("json parse", "JSON.parse('['.repeat(200000) + ']'.repeat(200000))", "SyntaxError"),
        ("regexp", "new RegExp('('.repeat(100000) + ')'.repeat(100000))", "SyntaxError"),
        ("copy", "let o = {}; for (let i = 0; i < 200000; i++) o = { o }; const copy = (v) => typeof v !== 'object' ? v : { o: copy(v.o) }; copy(o)", "RangeError"),
    ]
    .into_iter()
    .map(|(name, script, error)| (name, script.to_owned(), error))
    .collect()
}

/// Runs `script` with the limits render sets and returns the name of the error it ends with.
fn deep_nesting_outcome(script: &str) -> String {
    let context = &mut Context::default();
    context.runtime_limits_mut().set_recursion_limit(10_000);
    context.runtime_limits_mut().set_stack_size_limit(1_000_000);
    match context.eval(Source::from_bytes(script)) {
        Ok(_) => "no error".to_owned(),
        Err(error) => error.as_native().map_or_else(
            || "not a native error".to_owned(),
            |error| error.kind().to_string(),
        ),
    }
}

#[test]
fn deep_nesting_throws_instead_of_overflowing_the_native_stack() {
    // A native stack overflow aborts the process, so each case runs in a run of this test binary
    // that runs only this test, on a thread with the 2 MiB stack of the thread render runs
    // scripts on.
    if let Ok(name) = std::env::var(DEEP_NESTING_CASE) {
        let (_, script, _) = deep_nesting_cases()
            .into_iter()
            .find(|case| case.0 == name)
            .expect("the case must exist");
        let outcome = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || deep_nesting_outcome(&script))
            .expect("spawn a thread")
            .join()
            .expect("the thread must not panic");
        println!("deep nesting outcome: {outcome}");
        return;
    }
    let (_, module) = module_path!().split_once("::").expect("a module path");
    let test = format!("{module}::deep_nesting_throws_instead_of_overflowing_the_native_stack");
    let binary = std::env::current_exe().expect("the path of the test binary");
    let mut wrong = Vec::new();
    for (name, _, expected) in deep_nesting_cases() {
        let output = std::process::Command::new(&binary)
            .args([test.as_str(), "--exact", "--nocapture", "--test-threads=1"])
            .env(DEEP_NESTING_CASE, name)
            .output()
            .expect("run the test binary");
        let stdout = String::from_utf8_lossy(&output.stdout);
        // The test harness prints the name of the test on the same line.
        let outcome = stdout
            .lines()
            .find_map(|line| Some(line.split_once("deep nesting outcome: ")?.1));
        if !output.status.success() || outcome != Some(expected) {
            wrong.push(format!(
                "{name}: {}, {outcome:?} instead of {expected}",
                output.status
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
