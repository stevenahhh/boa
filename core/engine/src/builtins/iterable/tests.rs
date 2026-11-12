//! Tests for the Iterator Helpers proposal implementation (#4444).

use crate::{JsNativeErrorKind, JsValue, TestAction, run_test_actions};
use boa_macros::js_str;

// ── Iterator Constructor ──────────────────────────────────────────────────────

#[test]
fn iterator_constructor_requires_new() {
    run_test_actions([TestAction::assert_native_error(
        "Iterator()",
        JsNativeErrorKind::Type,
        "Iterator constructor requires 'new'",
    )]);
}

#[test]
fn iterator_constructor_abstract_class() {
    run_test_actions([TestAction::assert_native_error(
        "new Iterator()",
        JsNativeErrorKind::Type,
        "Abstract class Iterator not directly constructable",
    )]);
}

#[test]
fn iterator_subclass_can_be_constructed() {
    run_test_actions([TestAction::assert(
        "class MyIter extends Iterator {
           next() { return {value: 42, done: false}; }
         }
         new MyIter() instanceof Iterator",
    )]);
}

#[test]
fn iterator_to_string_tag() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.prototype[Symbol.toStringTag]",
        js_str!("Iterator"),
    )]);
}

// ── Iterator.from() ───────────────────────────────────────────────────────────

#[test]
fn iterator_from_array() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1, 2, 3]).toArray().join(',')",
        js_str!("1,2,3"),
    )]);
}

#[test]
fn iterator_from_string() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from('abc').toArray().join(',')",
        js_str!("a,b,c"),
    )]);
}

#[test]
fn iterator_from_returns_object() {
    run_test_actions([TestAction::assert_eq(
        "typeof Iterator.from([1, 2, 3])",
        js_str!("object"),
    )]);
}

// ── Lazy — map ────────────────────────────────────────────────────────────────

#[test]
fn iterator_map_basic() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1, 2, 3]).map(x => x * 2).toArray().join(',')",
        js_str!("2,4,6"),
    )]);
}

#[test]
fn iterator_map_counter() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from(['a','b','c']).map((v, i) => i).toArray().join(',')",
        js_str!("0,1,2"),
    )]);
}

#[test]
fn iterator_map_mapper_not_callable_throws() {
    run_test_actions([TestAction::assert_native_error(
        "Iterator.from([1]).map(42)",
        JsNativeErrorKind::Type,
        "Iterator.prototype.map: mapper is not callable",
    )]);
}

// ── Lazy — filter ─────────────────────────────────────────────────────────────

#[test]
fn iterator_filter_basic() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1,2,3,4,5]).filter(x => x % 2 === 0).toArray().join(',')",
        js_str!("2,4"),
    )]);
}

#[test]
fn iterator_filter_none_match() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1,2,3]).filter(x => x > 99).toArray().length",
        0,
    )]);
}

// ── Lazy — take ───────────────────────────────────────────────────────────────

#[test]
fn iterator_take_basic() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1,2,3,4,5]).take(3).toArray().join(',')",
        js_str!("1,2,3"),
    )]);
}

#[test]
fn iterator_take_zero() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1,2,3]).take(0).toArray().length",
        0,
    )]);
}

#[test]
fn iterator_take_negative_throws() {
    run_test_actions([TestAction::assert_native_error(
        "Iterator.from([1]).take(-1)",
        JsNativeErrorKind::Range,
        "Iterator.prototype.take: limit cannot be negative",
    )]);
}

#[test]
fn iterator_take_nan_throws() {
    run_test_actions([TestAction::assert_native_error(
        "Iterator.from([1]).take(NaN)",
        JsNativeErrorKind::Range,
        "Iterator.prototype.take: limit cannot be NaN",
    )]);
}

#[test]
fn iterator_take_more_than_length() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1,2,3]).take(100).toArray().join(',')",
        js_str!("1,2,3"),
    )]);
}

// ── Lazy — drop ───────────────────────────────────────────────────────────────

#[test]
fn iterator_drop_basic() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1,2,3,4,5]).drop(2).toArray().join(',')",
        js_str!("3,4,5"),
    )]);
}

#[test]
fn iterator_drop_more_than_length() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1,2,3]).drop(10).toArray().length",
        0,
    )]);
}

#[test]
fn iterator_drop_zero() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1,2,3]).drop(0).toArray().join(',')",
        js_str!("1,2,3"),
    )]);
}

// ── Lazy — flatMap ────────────────────────────────────────────────────────────

#[test]
fn iterator_flat_map_basic() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1,2,3]).flatMap(x => [x, x*10]).toArray().join(',')",
        js_str!("1,10,2,20,3,30"),
    )]);
}

#[test]
fn iterator_flat_map_inner_empty() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1,2,3]).flatMap(x => []).toArray().length",
        0,
    )]);
}

#[test]
fn iterator_flat_map_not_callable_throws() {
    run_test_actions([TestAction::assert_native_error(
        "Iterator.from([1]).flatMap(42)",
        JsNativeErrorKind::Type,
        "Iterator.prototype.flatMap: mapper is not callable",
    )]);
}

// ── Eager — reduce ────────────────────────────────────────────────────────────

#[test]
fn iterator_reduce_with_initial_value() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1,2,3,4]).reduce((acc, x) => acc + x, 0)",
        10,
    )]);
}

#[test]
fn iterator_reduce_no_initial_value() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1,2,3]).reduce((acc, x) => acc + x)",
        6,
    )]);
}

#[test]
fn iterator_reduce_empty_no_initial_throws() {
    run_test_actions([TestAction::assert_native_error(
        "Iterator.from([]).reduce((acc, x) => acc + x)",
        JsNativeErrorKind::Type,
        "Iterator.prototype.reduce: cannot reduce empty iterator with no initial value",
    )]);
}

// ── Eager — toArray ───────────────────────────────────────────────────────────

#[test]
fn iterator_to_array_basic() {
    run_test_actions([TestAction::assert_eq(
        "JSON.stringify(Iterator.from([1,2,3]).toArray())",
        js_str!("[1,2,3]"),
    )]);
}

#[test]
fn iterator_to_array_empty() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([]).toArray().length",
        0,
    )]);
}

// ── Eager — forEach ───────────────────────────────────────────────────────────

#[test]
fn iterator_for_each_basic() {
    run_test_actions([
        TestAction::run("let sum = 0; Iterator.from([1,2,3]).forEach(x => { sum += x; });"),
        TestAction::assert_eq("sum", 6),
    ]);
}

// ── Eager — some / every / find ───────────────────────────────────────────────

#[test]
fn iterator_some_true() {
    run_test_actions([TestAction::assert(
        "Iterator.from([1,2,3]).some(x => x === 2)",
    )]);
}

#[test]
fn iterator_some_false() {
    run_test_actions([TestAction::assert(
        "!Iterator.from([1,2,3]).some(x => x === 99)",
    )]);
}

#[test]
fn iterator_every_true() {
    run_test_actions([TestAction::assert(
        "Iterator.from([2,4,6]).every(x => x % 2 === 0)",
    )]);
}

#[test]
fn iterator_every_false() {
    run_test_actions([TestAction::assert(
        "!Iterator.from([2,4,5]).every(x => x % 2 === 0)",
    )]);
}

#[test]
fn iterator_find_found() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1,2,3]).find(x => x > 1)",
        2,
    )]);
}

#[test]
fn iterator_find_not_found() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1,2,3]).find(x => x > 99)",
        JsValue::undefined(),
    )]);
}

// ── IteratorHelper protocol ───────────────────────────────────────────────────

#[test]
fn iterator_helper_to_string_tag() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1]).map(x => x)[Symbol.toStringTag]",
        js_str!("Iterator Helper"),
    )]);
}

#[test]
fn iterator_helper_return_closes_underlying() {
    run_test_actions([
        TestAction::run(
            "let closed = false;
             const iter = {
               [Symbol.iterator]() { return this; },
               next() { return {value: 1, done: false}; },
               return() { closed = true; return {value: undefined, done: true}; }
             };
             const helper = Iterator.from(iter).map(x => x);
             helper.return();",
        ),
        TestAction::assert("closed"),
    ]);
}

// ── Chaining ─────────────────────────────────────────────────────────────────

#[test]
fn iterator_chaining() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.from([1,2,3,4,5,6,7,8,9,10])
           .filter(x => x % 2 === 0)
           .map(x => x * 3)
           .take(3)
           .toArray()
           .join(',')",
        js_str!("6,12,18"),
    )]);
}

#[test]
fn iterator_prototype_iterator_self() {
    run_test_actions([TestAction::assert(
        "Iterator.prototype[Symbol.iterator]() === Iterator.prototype",
    )]);
}

#[test]
fn iterator_concat_basic() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.concat([1,2],[3,4]).toArray().join(',')",
        js_str!("1,2,3,4"),
    )]);
}

#[test]
fn iterator_concat_zero_arguments() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.concat().toArray().length",
        0,
    )]);
}

#[test]
fn iterator_concat_single_argument() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.concat([1,2]).toArray().join(',')",
        js_str!("1,2"),
    )]);
}

#[test]
fn iterator_concat_three_arguments() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.concat([1],[2],[3]).toArray().join(',')",
        js_str!("1,2,3"),
    )]);
}

#[test]
fn iterator_concat_lazy_next() {
    run_test_actions([TestAction::assert_eq(
        "Iterator.concat([1,2],[3,4]).next().value",
        1,
    )]);
}

#[test]
fn iterator_concat_non_object_throws() {
    run_test_actions([TestAction::assert_native_error(
        "Iterator.concat(42)",
        JsNativeErrorKind::Type,
        "Iterator.concat requires iterable objects",
    )]);
}

#[test]
fn iterator_concat_missing_iterator_throws() {
    run_test_actions([TestAction::assert_native_error(
        "Iterator.concat({})",
        JsNativeErrorKind::Type,
        "Iterator.concat requires objects with @@iterator",
    )]);
}

#[test]
fn iterator_concat_return_closes_inner() {
    run_test_actions([
        TestAction::run(
            "let closed = false;
             const iter = {
                 [Symbol.iterator]() {
                     return {
                         next() { return { value: 1, done: false }; },
                         return() { closed = true; return { done: true }; }
                     };
                 }
             };
             const it = Iterator.concat(iter);
             it.next();
             it.return();",
        ),
        TestAction::assert("closed"),
    ]);
}

#[test]
fn iterator_concat_return_result_shape() {
    run_test_actions([TestAction::assert(
        "const it = Iterator.concat([1,2]); it.next();
         const r = it.return(); r.done === true && r.value === undefined",
    )]);
}

#[test]
#[cfg(feature = "experimental")]
fn iterator_includes_basic() {
    run_test_actions([
        TestAction::run("const gen = () => Iterator.from([1, 3]);"),
        TestAction::assert_eq("gen().includes(1)", true),
        TestAction::assert_eq("gen().includes(2)", false),
        TestAction::assert_eq("gen().includes(3)", true),
        TestAction::assert_eq("gen().drop(1).includes(1)", false),
        TestAction::assert_eq("gen().drop(1).includes(3)", true),
        TestAction::assert_eq("gen().drop(2).includes(3)", false),
        TestAction::assert_eq("gen().includes(1, 1)", false),
        TestAction::assert_eq("gen().includes(3, 1)", true),
        TestAction::assert_eq("gen().includes(3, 2)", false),
    ]);
}

#[test]
#[cfg(feature = "experimental")]
fn iterator_includes_generator() {
    run_test_actions([
        TestAction::run("function* gen() { yield 1; yield 3; }"),
        TestAction::assert_eq("gen().includes(1)", true),
        TestAction::assert_eq("gen().includes(2)", false),
        TestAction::assert_eq("gen().includes(3)", true),
        TestAction::assert_eq("gen().drop(1).includes(1)", false),
        TestAction::assert_eq("gen().drop(1).includes(3)", true),
        TestAction::assert_eq("gen().drop(2).includes(3)", false),
        TestAction::assert_eq("gen().includes(1, 1)", false),
        TestAction::assert_eq("gen().includes(3, 1)", true),
        TestAction::assert_eq("gen().includes(3, 2)", false),
    ]);
}

#[test]
#[cfg(feature = "experimental")]
fn iterator_includes_errors() {
    run_test_actions([
        TestAction::run("const gen = () => Iterator.from([1, 3]);"),
        TestAction::assert_native_error(
            "gen().includes(1, NaN)",
            JsNativeErrorKind::Type,
            "skippedElements must be a number",
        ),
        TestAction::assert_native_error(
            "gen().includes(1, 'a string')",
            JsNativeErrorKind::Type,
            "skippedElements must be a number",
        ),
        TestAction::assert_native_error(
            "gen().includes(1, -1)",
            JsNativeErrorKind::Range,
            "skippedElements must be a positive number",
        ),
    ]);
}

// ── Iteration of arrays by the built-in ArrayIterator next (fast path bail-outs) ──
// Expected values come from node v26.7.0.

#[test]
fn array_iteration_next_replaced_before() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const o = Object.getPrototypeOf([][Symbol.iterator]()); const n = o.next; let c = 0; o.next = function () { c++; return n.call(this); }; let s = 0; for (const x of [1, 2, 3]) s += x; o.next = n; return s + ":" + c; })())"#,
        js_str!("6:4"),
    )]);
}

#[test]
fn array_iteration_next_replaced_during() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const o = Object.getPrototypeOf([][Symbol.iterator]()); const n = o.next; let s = ""; for (const x of [1, 2, 3]) { s += x; o.next = function () { return { value: 9, done: true }; }; } o.next = n; return s; })())"#,
        js_str!("123"),
    )]);
}

#[test]
fn array_iteration_symbol_iterator_replaced() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const d = Object.getOwnPropertyDescriptor(Array.prototype, Symbol.iterator); Array.prototype[Symbol.iterator] = function* () { yield "a"; yield "b"; }; const r = [...[1, 2, 3]].join(); Object.defineProperty(Array.prototype, Symbol.iterator, d); return r; })())"#,
        js_str!("a,b"),
    )]);
}

#[test]
fn array_iteration_own_symbol_iterator() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const a = [1, 2]; a[Symbol.iterator] = function* () { yield 7; }; return [...a].join(); })())"#,
        js_str!("7"),
    )]);
}

#[test]
fn array_iteration_holes_proto() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { Array.prototype[1] = "p"; const r = [...[1, , 3]].join(); delete Array.prototype[1]; return r; })())"#,
        js_str!("1,p,3"),
    )]);
}

#[test]
fn array_iteration_holes_plain() {
    run_test_actions([TestAction::assert_eq(
        r#"String([...[1, , 3]].map(String).join())"#,
        js_str!("1,undefined,3"),
    )]);
}

#[test]
fn array_iteration_getter_element() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const a = [1, 2, 3]; let g = 0; Object.defineProperty(a, 1, { get() { g++; return 20; } }); let s = []; for (const x of a) s.push(x); return s.join() + ":" + g; })())"#,
        js_str!("1,20,3:1"),
    )]);
}

#[test]
fn array_iteration_push_during() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const a = [1, 2, 3]; let s = []; for (const x of a) { s.push(x); if (a.length < 6) a.push(x * 10); } return s.join(); })())"#,
        js_str!("1,2,3,10,20,30"),
    )]);
}

#[test]
fn array_iteration_shrink_during() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const a = [1, 2, 3, 4, 5]; let s = []; for (const x of a) { s.push(x); a.length = 2; } return s.join(); })())"#,
        js_str!("1,2"),
    )]);
}

#[test]
fn array_iteration_length_extended() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const a = [1, 2]; a.length = 4; return [...a].map(String).join(); })())"#,
        js_str!("1,2,undefined,undefined"),
    )]);
}

#[test]
fn array_iteration_splice_during() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const a = [1, 2, 3, 4]; let s = []; for (const x of a) { s.push(x); if (x === 1) a.splice(0, 1); } return s.join(); })())"#,
        js_str!("1,3,4"),
    )]);
}

#[test]
fn array_iteration_proxy_array() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const log = []; const p = new Proxy([1, 2], { get(t, k, r) { log.push(String(k)); return Reflect.get(t, k, r); } }); const r = [...p].join(); return r + "|" + log.join(); })())"#,
        js_str!("1,2|Symbol(Symbol.iterator),length,0,length,1,length"),
    )]);
}

#[test]
fn array_iteration_subclass() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { class A extends Array { *[Symbol.iterator]() { yield* super[Symbol.iterator](); yield "x"; } } const a = A.from([1, 2]); return [...a].join(); })())"#,
        js_str!("1,2,x"),
    )]);
}

#[test]
fn array_iteration_subclass_plain() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { class A extends Array {} const a = new A(); a.push(1, 2); let s = 0; for (const x of a) s += x; return s; })())"#,
        js_str!("3"),
    )]);
}

#[test]
fn array_iteration_frozen() {
    run_test_actions([TestAction::assert_eq(
        r#"String([...Object.freeze([1, 2, 3])].join())"#,
        js_str!("1,2,3"),
    )]);
}

#[test]
fn array_iteration_typed_array() {
    run_test_actions([TestAction::assert_eq(
        r#"String([...new Int8Array([1, 2, 3])].join())"#,
        js_str!("1,2,3"),
    )]);
}

#[test]
fn array_iteration_arguments_mapped() {
    run_test_actions([TestAction::assert_eq(
        r#"String((function (a, b) { arguments[1] = 9; return [...arguments].join(); })(1, 2, 3))"#,
        js_str!("1,9,3"),
    )]);
}

#[test]
fn array_iteration_arguments_strict() {
    run_test_actions([TestAction::assert_eq(
        r#"String((function (a, b) { "use strict"; return [...arguments].join(); })(1, 2, 3))"#,
        js_str!("1,2,3"),
    )]);
}

#[test]
fn array_iteration_break_return() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const P = Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]())); let c = 0; P.return = function () { c++; return {}; }; for (const x of [1, 2, 3]) { if (x === 2) break; } const [a] = [5, 6]; delete P.return; return c + ":" + a; })())"#,
        js_str!("2:5"),
    )]);
}

#[test]
fn array_iteration_entries_keys() {
    run_test_actions([TestAction::assert_eq(
        r#"String([...["a", "b"].entries()].join(";") + "|" + [...["a", "b"].keys()].join())"#,
        js_str!("0,a;1,b|0,1"),
    )]);
}

#[test]
fn array_iteration_entries_destructure() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { let s = ""; for (const [i, v] of ["a", "b"].entries()) s += i + v; return s; })())"#,
        js_str!("0a1b"),
    )]);
}

#[test]
fn array_iteration_destructure_defaults_rest() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const [a, b = 5, , ...r] = [1, undefined, 3, 4, 5]; return [a, b, r.join()].join(";"); })())"#,
        js_str!("1;5;4,5"),
    )]);
}

#[test]
fn array_iteration_getter_throws() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const a = [1, 2]; Object.defineProperty(a, 1, { get() { throw new Error("boom"); } }); try { [...a]; } catch (e) { return e.message; } })())"#,
        js_str!("boom"),
    )]);
}

#[test]
fn array_iteration_yield_star() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { function* g() { yield* [1, 2]; } const it = g(); const r = it.next(); return Object.keys(r).join() + ":" + r.value + ":" + (Object.getPrototypeOf(r) === Object.prototype); })())"#,
        js_str!("value,done:1:true"),
    )]);
}

#[test]
fn array_iteration_manual_next() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const it = [1][Symbol.iterator](); const a = it.next(), b = it.next(), c = it.next(); return [a.value, a.done, b.value, b.done, c.done, a !== it.next()].join(); })())"#,
        js_str!("1,false,,true,true,true"),
    )]);
}

#[test]
fn array_iteration_iterator_reuse() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const it = [1, 2, 3][Symbol.iterator](); it.next(); return [...it].join() + "|" + [...it].length; })())"#,
        js_str!("2,3|0"),
    )]);
}

#[test]
fn array_iteration_sparse() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const a = []; a[1000000] = 1; a[0] = 2; let n = 0; for (const x of a) { n++; if (n > 3) break; } return n + ":" + [...a.slice(0, 2)].map(String).join(); })())"#,
        js_str!("4:2,undefined"),
    )]);
}

#[test]
fn array_iteration_array_from_iter() {
    run_test_actions([TestAction::assert_eq(
        r#"String(Array.from([1, 2, 3], (x) => x * 2).join())"#,
        js_str!("2,4,6"),
    )]);
}

#[test]
fn array_iteration_array_like_iterator_call() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const o = { length: 2, 0: "a", 1: "b" }; return [...Array.prototype.values.call(o)].join(); })())"#,
        js_str!("a,b"),
    )]);
}

#[test]
fn array_iteration_non_array_with_dense() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const o = { 0: "a", 1: "b", 2: "c", length: 1 }; return [...Array.prototype.values.call(o)].join(); })())"#,
        js_str!("a"),
    )]);
}

#[test]
fn array_iteration_typed_out_of_bounds() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const b = new ArrayBuffer(4, { maxByteLength: 8 }); const t = new Uint8Array(b, 0, 4); const it = t[Symbol.iterator](); it.next(); b.resize(2); try { it.next(); return "no"; } catch (e) { return e.constructor.name; } })())"#,
        js_str!("TypeError"),
    )]);
}

// ── Iteration by the built-in Map, Set and String iterator next (fast path bail-outs) ──
// Expected values come from node v26.7.0.

#[test]
fn builtin_iteration_map_delete_add_during() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const m = new Map([[1, "a"], [2, "b"], [3, "c"]]); let s = ""; for (const [k, v] of m) { s += k + v; if (k === 1) { m.delete(2); m.set(4, "d"); } } return s; })())"#,
        js_str!("1a3c4d"),
    )]);
}

#[test]
fn builtin_iteration_map_clear_during() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const m = new Map([[1, 1], [2, 2]]); let n = 0; for (const x of m.keys()) { n++; m.clear(); } return n + ":" + m.size; })())"#,
        js_str!("1:0"),
    )]);
}

#[test]
fn builtin_iteration_map_readd_during() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const m = new Map([[1, 1], [2, 2]]); let s = ""; for (const k of m.keys()) { s += k; if (k === 1) { m.delete(1); m.set(1, 1); } if (s.length > 5) break; } return s; })())"#,
        js_str!("121111"),
    )]);
}

#[test]
fn builtin_iteration_map_next_replaced() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const P = Object.getPrototypeOf(new Map().entries()); const n = P.next; let c = 0; P.next = function () { c++; return n.call(this); }; let s = 0; for (const v of new Map([[1, 2]]).values()) s += v; P.next = n; return s + ":" + c; })())"#,
        js_str!("2:2"),
    )]);
}

#[test]
fn builtin_iteration_map_next_wrong_this() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const n = Object.getPrototypeOf(new Map().entries()).next; const it = { [Symbol.iterator]() { return { next: n }; } }; try { for (const x of it); } catch (e) { return e.constructor.name; } })())"#,
        js_str!("TypeError"),
    )]);
}

#[test]
fn builtin_iteration_map_entries_result() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const it = new Map([[1, 2]]).entries(); const r = it.next(); return Object.keys(r).join() + ":" + r.value.join() + ":" + it.next().done; })())"#,
        js_str!("value,done:1,2:true"),
    )]);
}

#[test]
fn builtin_iteration_map_subclass_iterator() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { class M extends Map { *[Symbol.iterator]() { yield "own"; } } return [...new M([[1, 1]])].join() + "|" + [...new M([[1, 1]]).keys()].join(); })())"#,
        js_str!("own|1"),
    )]);
}

#[test]
fn builtin_iteration_map_spread_from() {
    run_test_actions([TestAction::assert_eq(
        r#"String(Array.from(new Map([["a", 1], ["b", 2]]).values()).join() + "|" + [...new Map([["a", 1]])].join(";"))"#,
        js_str!("1,2|a,1"),
    )]);
}

#[test]
fn builtin_iteration_set_mutation_during() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const st = new Set([1, 2, 3]); let s = ""; for (const x of st) { s += x; if (x === 1) { st.delete(2); st.add(5); } } return s; })())"#,
        js_str!("135"),
    )]);
}

#[test]
fn builtin_iteration_set_entries_keys() {
    run_test_actions([TestAction::assert_eq(
        r#"String([...new Set(["a"]).entries()].join(";") + "|" + [...new Set(["a", "b"]).keys()].join())"#,
        js_str!("a,a|a,b"),
    )]);
}

#[test]
fn builtin_iteration_set_next_replaced_during() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const P = Object.getPrototypeOf(new Set().values()); const n = P.next; let s = ""; for (const x of new Set([1, 2, 3])) { s += x; P.next = function () { return { done: true }; }; } P.next = n; return s; })())"#,
        js_str!("123"),
    )]);
}

#[test]
fn builtin_iteration_set_iterator_exhausted() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const st = new Set([1]); const it = st.values(); [...it]; st.add(2); return String(it.next().done) + [...it].length; })())"#,
        js_str!("true0"),
    )]);
}

#[test]
fn builtin_iteration_string_surrogates() {
    run_test_actions([TestAction::assert_eq(
        r#"String([..."a\u{1F600}b\uD800c"].map(c => c.length).join())"#,
        js_str!("1,2,1,1,1"),
    )]);
}

#[test]
fn builtin_iteration_string_empty() {
    run_test_actions([TestAction::assert_eq(
        r#"String([...""].length + ":" + [..."x"].join())"#,
        js_str!("0:x"),
    )]);
}

#[test]
fn builtin_iteration_string_object() {
    run_test_actions([TestAction::assert_eq(
        r#"String([...new String("hi")].join())"#,
        js_str!("h,i"),
    )]);
}

#[test]
fn builtin_iteration_string_iterator_replaced() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const d = Object.getOwnPropertyDescriptor(String.prototype, Symbol.iterator); String.prototype[Symbol.iterator] = function* () { yield "z"; }; const r = [..."abc"].join(); Object.defineProperty(String.prototype, Symbol.iterator, d); return r; })())"#,
        js_str!("z"),
    )]);
}

#[test]
fn builtin_iteration_string_next_replaced() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const P = Object.getPrototypeOf(""[Symbol.iterator]()); const n = P.next; let c = 0; P.next = function () { c++; return n.call(this); }; const r = [..."ab"].join(); P.next = n; return r + ":" + c; })())"#,
        js_str!("a,b:3"),
    )]);
}

#[test]
fn builtin_iteration_string_destructure() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const [a, , c, ...r] = "wxyz12"; return a + c + r.join(""); })())"#,
        js_str!("wyz12"),
    )]);
}

#[test]
fn builtin_iteration_string_manual_next() {
    run_test_actions([TestAction::assert_eq(
        r#"String((() => { const it = "ab"[Symbol.iterator](); const x = it.next(); return x.value + x.done + it.next().value + it.next().done + it.next().done; })())"#,
        js_str!("afalsebtruetrue"),
    )]);
}
