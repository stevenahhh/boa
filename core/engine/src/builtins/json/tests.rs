use boa_macros::js_str;
use indoc::indoc;

use crate::{JsNativeErrorKind, JsValue, TestAction, js_string, run_test_actions};

#[test]
fn json_sanity() {
    run_test_actions([
        TestAction::assert_eq(r#"JSON.parse('{"aaa":"bbb"}').aaa"#, js_str!("bbb")),
        TestAction::assert_eq(
            r#"JSON.stringify({aaa: 'bbb'})"#,
            js_string!(r#"{"aaa":"bbb"}"#),
        ),
    ]);
}

#[test]
fn json_stringify_remove_undefined_values_from_objects() {
    run_test_actions([TestAction::assert_eq(
        r#"JSON.stringify({ aaa: undefined, bbb: 'ccc' })"#,
        js_string!(r#"{"bbb":"ccc"}"#),
    )]);
}

#[test]
fn json_stringify_remove_function_values_from_objects() {
    run_test_actions([TestAction::assert_eq(
        r#"JSON.stringify({ aaa: () => {}, bbb: 'ccc' })"#,
        js_string!(r#"{"bbb":"ccc"}"#),
    )]);
}

#[test]
fn json_stringify_remove_symbols_from_objects() {
    run_test_actions([TestAction::assert_eq(
        r#"JSON.stringify({ aaa: Symbol(), bbb: 'ccc' })"#,
        js_string!(r#"{"bbb":"ccc"}"#),
    )]);
}

#[test]
fn json_stringify_replacer_array_strings() {
    run_test_actions([TestAction::assert_eq(
        r#"JSON.stringify({aaa: 'bbb', bbb: 'ccc', ccc: 'ddd'}, ['aaa', 'bbb'])"#,
        js_string!(r#"{"aaa":"bbb","bbb":"ccc"}"#),
    )]);
}

#[test]
fn json_stringify_replacer_array_numbers() {
    run_test_actions([TestAction::assert_eq(
        r#"JSON.stringify({ 0: 'aaa', 1: 'bbb', 2: 'ccc'}, [1, 2])"#,
        js_string!(r#"{"1":"bbb","2":"ccc"}"#),
    )]);
}

#[test]
fn json_stringify_replacer_function() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            JSON.stringify({ aaa: 1, bbb: 2}, (key, value) => {
                if (key === 'aaa') {
                    return undefined;
                }

                return value;
            })
        "#},
        js_string!(r#"{"bbb":2}"#),
    )]);
}

#[test]
fn json_stringify_arrays() {
    run_test_actions([TestAction::assert_eq(
        "JSON.stringify(['a', 'b'])",
        js_string!(r#"["a","b"]"#),
    )]);
}

#[test]
fn json_stringify_object_array() {
    run_test_actions([TestAction::assert_eq(
        "JSON.stringify([{a: 'b'}, {b: 'c'}])",
        js_string!(r#"[{"a":"b"},{"b":"c"}]"#),
    )]);
}

#[test]
fn json_stringify_array_converts_undefined_to_null() {
    run_test_actions([TestAction::assert_eq(
        "JSON.stringify([undefined])",
        js_str!("[null]"),
    )]);
}

#[test]
fn json_stringify_array_converts_function_to_null() {
    run_test_actions([TestAction::assert_eq(
        "JSON.stringify([() => {}])",
        js_str!("[null]"),
    )]);
}

#[test]
fn json_stringify_array_converts_symbol_to_null() {
    run_test_actions([TestAction::assert_eq(
        "JSON.stringify([Symbol()])",
        js_str!("[null]"),
    )]);
}
#[test]
fn json_stringify_function_replacer_propagate_error() {
    run_test_actions([TestAction::assert_opaque_error(
        "JSON.stringify({x: 1}, (key, value) => { throw 1 })",
        1,
    )]);
}

#[test]
fn json_stringify_function() {
    run_test_actions([TestAction::assert_eq(
        "JSON.stringify(() => {})",
        JsValue::undefined(),
    )]);
}

#[test]
fn json_stringify_undefined() {
    run_test_actions([TestAction::assert_eq(
        "JSON.stringify(undefined)",
        JsValue::undefined(),
    )]);
}

#[test]
fn json_stringify_symbol() {
    run_test_actions([TestAction::assert_eq(
        "JSON.stringify(Symbol())",
        JsValue::undefined(),
    )]);
}

#[test]
fn json_stringify_no_args() {
    run_test_actions([TestAction::assert_eq(
        "JSON.stringify()",
        JsValue::undefined(),
    )]);
}

#[test]
fn json_stringify_fractional_numbers() {
    run_test_actions([TestAction::assert_eq("JSON.stringify(1.2)", js_str!("1.2"))]);
}

#[test]
fn json_stringify_pretty_print() {
    run_test_actions([TestAction::assert_eq(
        r#"JSON.stringify({a: "b", b: "c"}, undefined, 4)"#,
        js_string!(indoc! {r#"
            {
                "a": "b",
                "b": "c"
            }"#
        }),
    )]);
}

#[test]
fn json_stringify_pretty_print_four_spaces() {
    run_test_actions([TestAction::assert_eq(
        r#"JSON.stringify({a: "b", b: "c"}, undefined, 4.3)"#,
        js_string!(indoc! {r#"
            {
                "a": "b",
                "b": "c"
            }"#
        }),
    )]);
}

#[test]
fn json_stringify_pretty_print_twenty_spaces() {
    run_test_actions([TestAction::assert_eq(
        r#"JSON.stringify({a: "b", b: "c"}, undefined, 20)"#,
        js_string!(indoc! {r#"
            {
                      "a": "b",
                      "b": "c"
            }"#
        }),
    )]);
}

#[test]
fn json_stringify_pretty_print_with_number_object() {
    run_test_actions([TestAction::assert_eq(
        r#"JSON.stringify({a: "b", b: "c"}, undefined, new Number(10))"#,
        js_string!(indoc! {r#"
            {
                      "a": "b",
                      "b": "c"
            }"#
        }),
    )]);
}

#[test]
fn json_stringify_pretty_print_bad_space_argument() {
    run_test_actions([TestAction::assert_eq(
        r#"JSON.stringify({a: "b", b: "c"}, undefined, [])"#,
        js_string!(r#"{"a":"b","b":"c"}"#),
    )]);
}

#[test]
fn json_stringify_pretty_print_with_too_long_string() {
    run_test_actions([TestAction::assert_eq(
        r#"JSON.stringify({a: "b", b: "c"}, undefined, "abcdefghijklmn")"#,
        js_string!(indoc! {r#"
            {
            abcdefghij"a": "b",
            abcdefghij"b": "c"
            }"#
        }),
    )]);
}

#[test]
fn json_stringify_pretty_print_with_string_object() {
    run_test_actions([TestAction::assert_eq(
        r#"JSON.stringify({a: "b", b: "c"}, undefined, new String("abcd"))"#,
        js_string!(indoc! {r#"
            {
            abcd"a": "b",
            abcd"b": "c"
            }"#
        }),
    )]);
}

#[test]
fn json_parse_array_with_reviver() {
    run_test_actions([
        TestAction::run_harness(),
        TestAction::run(indoc! {r#"
                function reviver(k, v){
                    if (typeof v == 'number') {
                        return v * 2;
                    } else {
                        return v;
                    }
                }
            "#}),
        TestAction::assert("arrayEquals(JSON.parse('[1,2,3,4]', reviver), [2,4,6,8])"),
    ]);
}

#[test]
fn json_parse_object_with_reviver() {
    run_test_actions([
        TestAction::run(indoc! {r#"
                var jsonString = JSON.stringify({
                    firstname: "boa",
                    lastname: "snake"
                });

                function dataReviver(key, value) {
                    if (key == 'lastname') {
                        return 'interpreter';
                    } else {
                        return value;
                    }
                }

                var jsonObj = JSON.parse(jsonString, dataReviver);
            "#}),
        TestAction::assert_eq("jsonObj.firstname", js_str!("boa")),
        TestAction::assert_eq("jsonObj.lastname", js_str!("interpreter")),
    ]);
}

#[test]
fn json_parse_sets_prototypes() {
    run_test_actions([
        TestAction::run(indoc! {r#"
                const jsonString = "{\"ob\":{\"ject\":1},\"arr\": [0,1]}";
                const jsonObj = JSON.parse(jsonString);
            "#}),
        TestAction::assert("Object.getPrototypeOf(jsonObj.ob) === Object.prototype"),
        TestAction::assert("Object.getPrototypeOf(jsonObj.arr) === Array.prototype"),
    ]);
}

#[test]
fn json_fields_should_be_enumerable() {
    run_test_actions([
        TestAction::assert(indoc! {r#"
                var a = JSON.parse('{"x":0}');
                a.propertyIsEnumerable('x')
            "#}),
        TestAction::assert(indoc! {r#"
                var b = JSON.parse('[0, 1]');
                b.propertyIsEnumerable('0');
            "#}),
    ]);
}

#[test]
fn json_parse_with_no_args_throws_syntax_error() {
    run_test_actions([TestAction::assert_native_error(
        "JSON.parse();",
        JsNativeErrorKind::Syntax,
        "expected value at line 1 column 1",
    )]);
}

#[test]
fn json_stringify_cyclic_object_throws_type_error() {
    run_test_actions([TestAction::assert_native_error(
        "var a = {}; a.a = a; JSON.stringify(a);",
        JsNativeErrorKind::Type,
        "cyclic object value",
    )]);
}

#[test]
fn json_stringify_cyclic_array_throws_type_error() {
    run_test_actions([TestAction::assert_native_error(
        "var a = []; a[0] = a; JSON.stringify(a);",
        JsNativeErrorKind::Type,
        "cyclic object value",
    )]);
}

#[test]
fn json_stringify_cyclic_nested_object_throws_type_error() {
    run_test_actions([TestAction::assert_native_error(
        "var a = {}; var b = { a }; a.b = b; JSON.stringify(a);",
        JsNativeErrorKind::Type,
        "cyclic object value",
    )]);
}

#[test]
fn json_parse_duplicate_keys_keep_first_position_and_last_value() {
    run_test_actions([TestAction::assert_eq(
        r#"var o = JSON.parse('{"a":1,"b":2,"a":3}'); Object.keys(o).join() + " " + o.a"#,
        js_str!("a,b 3"),
    )]);
}

#[test]
fn json_parse_proto_key_is_an_own_data_property() {
    run_test_actions([
        TestAction::assert(indoc! {r#"
            var o = JSON.parse('{"__proto__":[1]}');
            Object.getPrototypeOf(o) === Object.prototype
                && Object.hasOwn(o, "__proto__")
                && Array.isArray(o.__proto__)
        "#}),
        TestAction::assert(r#"JSON.parse('{"__proto__":1,"__proto__":2}').__proto__ === 2"#),
    ]);
}

#[test]
fn json_parse_orders_integer_keys_first() {
    run_test_actions([TestAction::assert_eq(
        r#"Object.keys(JSON.parse('{"b":1,"2":2,"a":3,"1":4,"01":5,"4294967295":6,"4294967294":7}')).join()"#,
        js_str!("1,2,4294967294,b,a,01,4294967295"),
    )]);
}

#[test]
fn json_parse_keeps_negative_zero() {
    run_test_actions([
        TestAction::assert(r#"Object.is(JSON.parse("-0"), -0)"#),
        TestAction::assert(r#"Object.is(JSON.parse("[-0.0]")[0], -0)"#),
        TestAction::assert(r#"Object.is(JSON.parse("-0e5"), -0)"#),
        TestAction::assert(r#"Object.is(JSON.parse("0"), 0)"#),
    ]);
}

#[test]
fn json_parse_large_numbers() {
    run_test_actions([
        TestAction::run(
            r#"var big = JSON.parse("[12345678901234567890,9007199254740993,9007199254740995,-2147483649,1e300,1e-400,-1e-400]");"#,
        ),
        TestAction::assert("big[0] === 12345678901234567890"),
        TestAction::assert("big[1] === 9007199254740992"),
        TestAction::assert("big[2] === 9007199254740996"),
        TestAction::assert("big[3] === -2147483649"),
        TestAction::assert("big[4] === 1e300"),
        TestAction::assert("Object.is(big[5], 0)"),
        TestAction::assert("Object.is(big[6], -0)"),
        TestAction::run(r#"var huge = JSON.parse("[1e308,1.7976931348623157e308]");"#),
        TestAction::assert("huge[0] === 1e308"),
        TestAction::assert("huge[1] === Number.MAX_VALUE"),
    ]);
}

#[test]
fn json_parse_string_escapes_and_surrogate_pairs() {
    run_test_actions([
        TestAction::assert(
            r#"JSON.parse('"\\"\\\\\\/\\b\\f\\n\\r\\t\\u0041\\u00e9"') === "\"\\/\b\f\n\r\tA\u00e9""#,
        ),
        TestAction::assert(r#"JSON.parse('"\\ud83d\\ude00"') === "\ud83d\ude00""#),
        TestAction::assert(
            r#"JSON.parse('"\u00e9\u4e2d\ud83d\ude00\u2028"') === "\u00e9\u4e2d\ud83d\ude00\u2028""#,
        ),
    ]);
}

#[test]
fn json_parse_invalid_text_throws_syntax_error() {
    run_test_actions([TestAction::assert_eq(
        indoc! {r#"
            ["[1,]", "01", '{"a":1,}', "", "[1] x", '"\\x"'].map(function (text) {
                try { JSON.parse(text); return "ok"; } catch (e) { return e.name; }
            }).join()
        "#},
        js_str!("SyntaxError,SyntaxError,SyntaxError,SyntaxError,SyntaxError,SyntaxError"),
    )]);
}
