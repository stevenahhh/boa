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

#[test]
fn json_parse_accepts_every_depth_that_serde_json_accepts() {
    // An unoptimized build needs more native stack for 127 levels than a test thread has.
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(|| {
            run_test_actions([
                TestAction::run(indoc! {r#"
                    function nest(n, open, close) {
                        return open.repeat(n) + "1" + close.repeat(n);
                    }
                    function depth(v, key) {
                        let d = 0;
                        while (typeof v === "object") { v = v[key]; d++; }
                        return d + ":" + v;
                    }
                "#}),
                TestAction::assert_eq(
                    r#"depth(JSON.parse(nest(127, "[", "]")), 0)"#,
                    js_str!("127:1"),
                ),
                TestAction::assert_eq(
                    r#"depth(JSON.parse(nest(127, '{"a":', "}")), "a")"#,
                    js_str!("127:1"),
                ),
                TestAction::assert_eq(
                    r#"depth(JSON.parse(nest(127, "[", "]"), (k, v) => v), 0)"#,
                    js_str!("127:1"),
                ),
                TestAction::assert_eq(
                    r#"depth(JSON.parse(nest(127, '{"a":', "}"), (k, v) => v), "a")"#,
                    js_str!("127:1"),
                ),
            ]);
        })
        .expect("spawn the test thread")
        .join()
        .expect("the test thread panicked");
}

#[test]
fn json_parse_without_a_reviver_takes_any_depth() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            function depth(value, key) {
                let levels = 0;
                while (typeof value === "object") {
                    value = value[key];
                    levels++;
                }
                return levels + ":" + value;
            }
        "#}),
        TestAction::assert_eq(
            r#"depth(JSON.parse("[".repeat(10000) + "1" + "]".repeat(10000)), 0)"#,
            js_str!("10000:1"),
        ),
        TestAction::assert_eq(
            r#"depth(JSON.parse('{"a":'.repeat(10000) + "1" + "}".repeat(10000)), "a")"#,
            js_str!("10000:1"),
        ),
        TestAction::assert_eq(
            r#"JSON.stringify(JSON.parse('[1, [2, [3]], {"b": [4], "a": {"c": 5}}, 6]'))"#,
            js_str!(r#"[1,[2,[3]],{"b":[4],"a":{"c":5}},6]"#),
        ),
    ]);
}

#[test]
fn json_stringify_getter_deletes_later_key() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var o = { a: 1, get b() { delete o.c; return 2; }, c: 3, d: 4 }; return JSON.stringify(o); })()"#,
        js_string!(r#"{"a":1,"b":2,"d":4}"#),
    )]);
}

#[test]
fn json_stringify_getter_adds_and_changes() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var o = { a: 1, get b() { o.z = 9; o.c = "c2"; return 2; }, c: 3 }; return JSON.stringify(o); })()"#,
        js_string!(r#"{"a":1,"b":2,"c":"c2"}"#),
    )]);
}

#[test]
fn json_stringify_to_json_mutates_parent() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var o = { a: { toJSON() { o.b = "mut"; delete o.c; o.d = 1; return "A"; } }, b: 1, c: 2 }; return JSON.stringify(o); })()"#,
        js_string!(r#"{"a":"A","b":"mut"}"#),
    )]);
}

#[test]
fn json_stringify_to_json_mutates_grandparent() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var o = { x: [{ toJSON() { o.y = "late"; delete o.z; return 1; } }], y: 1, z: 2, w: 3 }; return JSON.stringify(o); })()"#,
        js_string!(r#"{"x":[1],"y":"late","w":3}"#),
    )]);
}

#[test]
fn json_stringify_to_json_grows_and_shrinks_array() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var a = [{ toJSON() { a.push(9); a[2] = "m"; return 0; } }, 1, 2]; var b = [{ toJSON() { b.length = 1; return 0; } }, 1, 2]; return JSON.stringify([a, b]); })()"#,
        js_string!(r#"[[0,1,"m"],[0,null,null]]"#),
    )]);
}

#[test]
fn json_stringify_to_json_added_to_object_prototype_midway() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var o = { a: { toJSON() { Object.prototype.toJSON = function () { return "P"; }; return 1; } }, b: { c: 1 }, d: [1] }; try { return JSON.stringify(o); } finally { delete Object.prototype.toJSON; } })()"#,
        js_string!(r#"{"a":1,"b":"P","d":"P"}"#),
    )]);
}

#[test]
fn json_stringify_to_json_getter_on_prototype() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var n = 0; Object.defineProperty(Object.prototype, "toJSON", { configurable: true, get() { n++; return undefined; } }); try { return JSON.stringify({ a: { b: 1 }, c: [2] }) + n; } finally { delete Object.prototype.toJSON; } })()"#,
        js_string!(r#"{"a":{"b":1},"c":[2]}3"#),
    )]);
}

#[test]
fn json_stringify_to_json_inherited_deep() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var p = { toJSON() { return "p"; } }; var o = Object.create(Object.create(p)); o.a = 1; return JSON.stringify({ o: o, q: [Object.create(p)] }); })()"#,
        js_string!(r#"{"o":"p","q":["p"]}"#),
    )]);
}

#[test]
fn json_stringify_array_subclass_and_own_to_json() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { class A extends Array { toJSON() { return "sub"; } } var a = new A(); a.push(1); var b = [1]; b.toJSON = () => "own"; class B extends Array {} var c = new B(); c.push(2, 3); return JSON.stringify([a, b, c]); })()"#,
        js_string!(r#"["sub","own",[2,3]]"#),
    )]);
}

#[test]
fn json_stringify_holes_read_through_prototype() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { Array.prototype[1] = "p1"; try { return JSON.stringify([[0, , 2], new Array(3), { a: [, 1] }]); } finally { delete Array.prototype[1]; } })()"#,
        js_string!(r#"[[0,"p1",2],[null,"p1",null],{"a":[null,1]}]"#),
    )]);
}

#[test]
fn json_stringify_proxy_values_and_prototypes() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var log = []; var p = new Proxy({ a: 1 }, { get(t, k) { log.push(String(k)); return t[k]; } }); var o = Object.create(new Proxy({}, { get(t, k) { log.push("proto:" + String(k)); } })); o.b = 2; return JSON.stringify([p, o]) + log.join(","); })()"#,
        js_string!(r#"[{"a":1},{"b":2}]toJSON,a,proto:toJSON"#),
    )]);
}

#[test]
fn json_stringify_integer_keys_order() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var o = { b: 1, 2: 2, a: 3, 1: 4, "-1": 5, "01": 6 }; var s = {}; s[10] = "x"; s.k = 1; s[3] = "y"; return JSON.stringify([o, s]); })()"#,
        js_string!(r#"[{"1":4,"2":2,"b":1,"a":3,"-1":5,"01":6},{"3":"y","10":"x","k":1}]"#),
    )]);
}

#[test]
fn json_stringify_accessors_and_non_enumerable() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var o = { a: 1 }; Object.defineProperty(o, "h", { value: 2, enumerable: false }); Object.defineProperty(o, "g", { get() { return [3]; }, enumerable: true }); o[Symbol("s")] = 4; o.z = undefined; o.f = function () {}; return JSON.stringify(o); })()"#,
        js_string!(r#"{"a":1,"g":[3]}"#),
    )]);
}

#[test]
fn json_stringify_wrappers_and_bigint_to_json() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { BigInt.prototype.toJSON = function () { return "n" + this; }; try { return JSON.stringify({ a: [new Number(1), new String("s"), new Boolean(true), 2n], d: new Date(0) }); } finally { delete BigInt.prototype.toJSON; } })()"#,
        js_string!(r#"{"a":[1,"s",true,"n2"],"d":"1970-01-01T00:00:00.000Z"}"#),
    )]);
}

#[test]
fn json_stringify_frozen_and_null_prototype() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var n = Object.create(null); n.x = [1]; return JSON.stringify([Object.freeze({ a: [1, Object.freeze([2])] }), n]); })()"#,
        js_string!(r#"[{"a":[1,[2]]},{"x":[1]}]"#),
    )]);
}

#[test]
fn json_stringify_numbers_and_strings() {
    run_test_actions([TestAction::assert_eq(
        r#"JSON.stringify([0, -0, 1.5, 1e21, 1e-7, -2147483648, 2147483648, NaN, -Infinity, "a\"b\\c\n\u0001\u2028\ud800\udc00\udc00\u00e9\u4e2d"])"#,
        js_string!(
            r#"[0,0,1.5,1e+21,1e-7,-2147483648,2147483648,null,null,"a\"b\\c\n\u0001 𐀀\udc00é中"]"#
        ),
    )]);
}

#[test]
fn json_stringify_quotes_surrogates_and_controls() {
    run_test_actions([TestAction::assert_eq(
        r#"JSON.stringify(["\udc00\ud800", "\ud800\ud800\udc00x", "a\udbff", "\u00ff\u0000\u001f\u007f", "\u4e2d\"\\\b\f\n\r\t/"])"#,
        js_string!(
            "[\"\\udc00\\ud800\",\"\\ud800𐀀x\",\"a\\udbff\",\"ÿ\\u0000\\u001f\u{7f}\",\"中\\\"\\\\\\b\\f\\n\\r\\t/\"]"
        ),
    )]);
}

#[test]
fn json_parse_duplicate_and_escaped_keys() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var o = JSON.parse('{"a":1,"b":2,"\\u0061":3,"c":{"x":1},"b":4}'); return Object.keys(o).join() + "|" + JSON.stringify(o); })()"#,
        js_string!(r#"a,b,c|{"a":3,"b":4,"c":{"x":1}}"#),
    )]);
}

#[test]
fn json_parse_proto_and_integer_keys() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var o = JSON.parse('{"z":1,"__proto__":{"p":1},"2":"two","1":"one","-1":"m"}'); return Object.keys(o).join() + "|" + (Object.getPrototypeOf(o) === Object.prototype) + "|" + o.__proto__.p + "|" + ("p" in o); })()"#,
        js_string!(r#"1,2,z,__proto__,-1|true|1|false"#),
    )]);
}

#[test]
fn json_parse_objects_with_same_keys_stay_independent() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var a = JSON.parse('[{"k":1,"v":"a"},{"k":2,"v":"b"},{"k":3,"v":"c"}]'); a[0].extra = true; delete a[1].k; a[2].v = "changed"; Object.defineProperty(a[2], "k", { enumerable: false }); return JSON.stringify(a) + "|" + Object.keys(a[1]).join() + "|" + Object.keys(a[2]).join(); })()"#,
        js_string!(r#"[{"k":1,"v":"a","extra":true},{"v":"b"},{"v":"changed"}]|v|v"#),
    )]);
}

#[test]
fn json_parse_non_ascii_and_utf16_texts() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var r = []; ['{"\u00e9":"\u00e9"}', '{"\u4e2d":["\u4e2d",1]}', '{"k":"\ud83d\ude00"}', '{"a":"\\ud83d\\ude00"}', '[1, 2.5e3, -0, "x"]'].forEach(function (t) { var v = JSON.parse(t); r.push(JSON.stringify(v) + ":" + Object.is(v[2], -0)); }); return r.join(" "); })()"#,
        js_string!(
            r#"{"é":"é"}:false {"中":["中",1]}:false {"k":"😀"}:false {"a":"😀"}:false [1,2500,0,"x"]:true"#
        ),
    )]);
}

#[test]
fn json_parse_errors_keep_their_type() {
    run_test_actions([TestAction::assert_eq(
        r#"(() => { var r = []; ['{"a":1,}', '{"a" 1}', '{"\u0001":1}', '', '[1,]', '{"a":01}'].forEach(function (t) { try { r.push(typeof JSON.parse(t)); } catch (e) { r.push(e.constructor.name); } }); return r.join(); })()"#,
        js_string!(r#"SyntaxError,SyntaxError,SyntaxError,SyntaxError,SyntaxError,SyntaxError"#),
    )]);
}
