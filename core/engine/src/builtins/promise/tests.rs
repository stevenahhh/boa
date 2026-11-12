use crate::{TestAction, run_test_actions};
use boa_macros::js_str;
use indoc::indoc;

#[test]
fn promise() {
    run_test_actions([
        TestAction::run(indoc! {r#"
                    let count = 0;
                    const promise = new Promise((resolve, reject) => {
                        count += 1;
                        resolve(undefined);
                    }).then((_) => (count += 1));
                    count += 1;
                "#}),
        TestAction::assert_eq("count", 2),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("count", 3),
    ]);
}

#[test]
fn promise_all_resolves_values() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            var values = [];
            var p = Promise.all([Promise.resolve(1), Promise.resolve(2)]);
            p.then(v => { values = v; });
        "#}),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("values.length", 2),
        TestAction::assert_eq("values[0]", 1),
        TestAction::assert_eq("values[1]", 2),
    ]);
}

#[test]
fn promise_all_rejects() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            var err = null;
            var p = Promise.all([Promise.resolve(1), Promise.reject(2)]);
            p.catch(e => { err = e; });
        "#}),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("err", 2),
    ]);
}

#[test]
fn promise_any_resolves_first_success() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            var val = null;
            var p = Promise.any([Promise.reject(1), Promise.resolve(2)]);
            p.then(v => { val = v; });
        "#}),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("val", 2),
    ]);
}

#[test]
fn promise_all_settled_resolves_results() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            var values = [];
            Promise.allSettled([
                Promise.resolve(1),
                Promise.reject(2)
            ]).then(results => {
                values = [
                    results[0].status,
                    results[0].value,
                    results[1].status,
                    results[1].reason
                ];
            });
        "#}),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("values[0]", crate::js_string!("fulfilled")),
        TestAction::assert_eq("values[1]", 1),
        TestAction::assert_eq("values[2]", crate::js_string!("rejected")),
        TestAction::assert_eq("values[3]", 2),
    ]);
}

#[test]
fn promise_race_resolves_first() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            var val = null;
            Promise.race([
                Promise.resolve(10),
                Promise.resolve(20)
            ]).then(v => { val = v; });
        "#}),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("val", 10),
    ]);
}

// ── Promise capabilities and await of %Promise% (fast path equivalence) ──
// Expected values come from node v26.7.0 (the log after the job queue is empty).

#[test]
fn capability_await_order() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; (async () => { out.push("a1"); await 1; out.push("a2"); await null; out.push("a3"); })(); Promise.resolve().then(() => out.push("t1")).then(() => out.push("t2")).then(() => out.push("t3")); out.push("sync");"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("a1,sync,a2,t1,a3,t2,t3")),
    ]);
}

#[test]
fn capability_await_native_vs_then() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; (async () => { await Promise.resolve(); out.push("A"); })(); Promise.resolve().then(() => out.push("B")); (async () => { await new Promise(r => r()); out.push("C"); })();"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("A,B,C")),
    ]);
}

#[test]
fn capability_await_thenable() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; (async () => { const v = await { then(r) { out.push("then"); r(7); } }; out.push("v" + v); })(); Promise.resolve().then(() => out.push("t1")).then(() => out.push("t2"));"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("then,t1,v7,t2")),
    ]);
}

#[test]
fn capability_await_then_getter() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; const p = Promise.resolve(3); Object.defineProperty(p, "constructor", { get() { out.push("ctor"); return Promise; } }); (async () => { out.push("got" + await p); })();"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("ctor,got3")),
    ]);
}

#[test]
fn capability_await_foreign_constructor() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; const p = Promise.resolve(4); p.constructor = function () {}; (async () => { out.push("v" + await p); })(); Promise.resolve().then(() => out.push("t1")).then(() => out.push("t2")).then(() => out.push("t3"));"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("t1,t2,v4,t3")),
    ]);
}

#[test]
fn capability_await_then_override() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; const t = Promise.prototype.then; Promise.prototype.then = function (a, b) { out.push("then"); return t.call(this, a, b); }; (async () => { await 1; await Promise.resolve(2); out.push("done"); })().then(() => { Promise.prototype.then = t; out.push("restored"); });"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("then,done,restored")),
    ]);
}

#[test]
fn capability_subclass_then() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; class P extends Promise { constructor(e) { out.push("ctor"); super(e); } } P.resolve(1).then(v => out.push("v" + v)); out.push(P.resolve(2) instanceof P);"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("ctor,ctor,ctor,true,v1")),
    ]);
}

#[test]
fn capability_species_override() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; class Q extends Promise { static get [Symbol.species]() { out.push("species"); return Promise; } } const r = Q.resolve(1).then(x => x); out.push(r instanceof Q); out.push(r instanceof Promise);"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("species,false,true")),
    ]);
}

#[test]
fn capability_then_chain_values() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; let p = Promise.resolve(0); for (let i = 0; i < 5; i++) p = p.then(v => { out.push(v); return v + 1; }); p.then(v => out.push("end" + v));"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("0,1,2,3,4,end5")),
    ]);
}

#[test]
fn capability_then_reject_chain() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; Promise.reject(new Error("x")).then(() => out.push("no")).catch(e => out.push(e.message)).finally(() => out.push("fin"));"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("x,fin")),
    ]);
}

#[test]
fn capability_executor_throws() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; new Promise(() => { throw 5; }).catch(e => out.push("caught" + e)); out.push("after");"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("after,caught5")),
    ]);
}

#[test]
fn capability_resolve_twice() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; new Promise((res, rej) => { res(1); res(2); rej(3); }).then(v => out.push(v), e => out.push("e" + e));"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("1")),
    ]);
}

#[test]
fn capability_async_return_promise() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; (async () => Promise.resolve(9))().then(v => out.push("v" + v)); Promise.resolve().then(() => out.push("t1")).then(() => out.push("t2")).then(() => out.push("t3")).then(() => out.push("t4"));"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("t1,t2,v9,t3,t4")),
    ]);
}

#[test]
fn capability_async_throw() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; (async () => { await 1; throw new TypeError("bad"); })().catch(e => out.push(e.name + e.message));"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("TypeErrorbad")),
    ]);
}

#[test]
fn capability_await_rejected() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; (async () => { try { await Promise.reject(8); } catch (e) { out.push("c" + e); } })();"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("c8")),
    ]);
}

#[test]
fn capability_promise_all_order() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; Promise.all([1, Promise.resolve(2), { then(r) { r(3); } }]).then(v => out.push(v.join()));"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("1,2,3")),
    ]);
}

#[test]
fn capability_promise_resolve_identity() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; const p = Promise.resolve(1); out.push(Promise.resolve(p) === p); out.push(new Promise(r => r(p)) === p);"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("true,false")),
    ]);
}

#[test]
fn capability_capability_function_props() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; let f; new Promise((res) => { f = res; }); out.push(typeof f, f.length, f.name === "", Object.getPrototypeOf(f) === Function.prototype);"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("function,1,true,true")),
    ]);
}

#[test]
fn capability_new_promise_proto() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; const p = new Promise(() => {}); out.push(Object.getPrototypeOf(p) === Promise.prototype); const q = p.then(); out.push(Object.getPrototypeOf(q) === Promise.prototype, q !== p);"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("true,true,true")),
    ]);
}

#[test]
fn capability_async_generator_await() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; (async function* () { yield await 1; yield 2; })().next().then(r => out.push(r.value + ":" + r.done));"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("1:false")),
    ]);
}

#[test]
fn capability_for_await_values() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; (async () => { for await (const x of [1, Promise.resolve(2), 3]) out.push(x); })();"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("1,2,3")),
    ]);
}

// ── PromiseResolve of a value that is not an object with %Promise% (fast path equivalence) ──
// Expected values come from node v26.7.0 (the log after the job queue is empty).

#[test]
fn promise_resolve_resolve_primitive_order() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; Promise.resolve(1).then(v => out.push("a" + v)); Promise.resolve("s").then(v => out.push("b" + v)); Promise.resolve().then(v => out.push("c" + v)); out.push("sync");"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("sync,a1,bs,cundefined")),
    ]);
}

#[test]
fn promise_resolve_await_primitive_kinds() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; (async () => { out.push(typeof await undefined, typeof await Symbol(), typeof await 1n, await "x", await null); })();"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("undefined,symbol,bigint,x,")),
    ]);
}

#[test]
fn promise_resolve_resolve_subclass_primitive() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; class S extends Promise {} const p = S.resolve(1); out.push(p instanceof S, Object.getPrototypeOf(p) === S.prototype); p.then(v => out.push("v" + v));"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("true,true,v1")),
    ]);
}

#[test]
fn promise_resolve_resolve_object_value() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; const o = { then: undefined }; Promise.resolve(o).then(v => out.push(v === o)); Promise.resolve(function () {}).then(v => out.push(typeof v));"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("true,function")),
    ]);
}

#[test]
fn promise_resolve_resolve_state_inspect() {
    run_test_actions([
        TestAction::run(
            r#"var out = []; const p = Promise.resolve(5); out.push(Object.getPrototypeOf(p) === Promise.prototype, p instanceof Promise); p.then(v => out.push(v)).then(() => out.push("after"));"#,
        ),
        TestAction::inspect_context(|ctx| ctx.run_jobs().unwrap()),
        TestAction::assert_eq("out.join()", js_str!("true,true,5,after")),
    ]);
}
