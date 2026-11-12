use boa_gc::Gc;
use boa_macros::js_str;
use boa_parser::Source;
use indoc::indoc;

use crate::{
    Context, JsNativeErrorKind, JsObject, JsResult, JsValue, TestAction,
    builtins::{OrdinaryObject, function::OrdinaryFunction},
    js_string,
    object::{
        ObjectInitializer, internal_methods::InternalMethodPropertyContext,
        shape::slot::SlotAttributes,
    },
    property::{Attribute, PropertyDescriptor, PropertyKey},
    run_test_actions,
    vm::CodeBlock,
};

#[test]
fn get_own_property_internal_method() {
    let context = &mut Context::default();

    let o = context
        .intrinsics()
        .templates()
        .ordinary_object()
        .create(OrdinaryObject, Vec::default());

    let property: PropertyKey = js_string!("prop").into();
    let value = 100;

    o.set(property.clone(), value, true, context)
        .expect("should not fail");

    let context = &mut InternalMethodPropertyContext::new(context);

    assert_eq!(context.slot().index, 0);
    assert_eq!(context.slot().attributes, SlotAttributes::empty());

    o.__get_own_property__(&property, context)
        .expect("should not fail");

    assert!(
        !context.slot().in_prototype(),
        "Since it's an owned property, the prototype bit should not be set"
    );

    assert!(
        context.slot().is_cacheable(),
        "Since it's an owned property, this should be cacheable"
    );

    let shape = o.borrow().shape().clone();

    let slot = shape.lookup(&property);

    assert!(slot.is_some(), "the property should be found in the object");

    let slot = slot.expect("the property should be found in the object");

    assert_eq!(context.slot().index, slot.index);
}

#[test]
fn get_internal_method() {
    let context = &mut Context::default();

    let o = context
        .intrinsics()
        .templates()
        .ordinary_object()
        .create(OrdinaryObject, Vec::default());

    let property: PropertyKey = js_string!("prop").into();
    let value = 100;

    o.set(property.clone(), value, true, context)
        .expect("should not fail");

    let context = &mut InternalMethodPropertyContext::new(context);

    assert_eq!(context.slot().index, 0);
    assert_eq!(context.slot().attributes, SlotAttributes::empty());

    o.__get__(&property, o.clone().into(), context)
        .expect("should not fail");

    assert!(
        !context.slot().in_prototype(),
        "Since it's an owned property, the prototype bit should not be set"
    );

    assert!(
        context.slot().is_cacheable(),
        "Since it's an owned property, this should be cacheable"
    );

    let shape = o.borrow().shape().clone();

    let slot = shape.lookup(&property);

    assert!(slot.is_some(), "the property should be found in the object");

    let slot = slot.expect("the property should be found in the object");

    assert_eq!(context.slot().index, slot.index);
}

#[test]
fn get_internal_method_in_prototype() {
    let context = &mut Context::default();

    let o = context
        .intrinsics()
        .templates()
        .ordinary_object()
        .create(OrdinaryObject, Vec::default());

    let property: PropertyKey = js_string!("prop").into();
    let value = 100;

    let prototype = context.intrinsics().constructors().object().prototype();

    prototype
        .set(property.clone(), value, true, context)
        .expect("should not fail");

    let context = &mut InternalMethodPropertyContext::new(context);

    assert_eq!(context.slot().index, 0);
    assert_eq!(context.slot().attributes, SlotAttributes::empty());

    o.__get__(&property, o.clone().into(), context)
        .expect("should not fail");

    assert!(
        context.slot().in_prototype(),
        "Since it's an prototype property, the prototype bit should not be set"
    );

    assert!(
        context.slot().is_cacheable(),
        "Since it's an prototype property, this should be cacheable"
    );

    let shape = prototype.borrow().shape().clone();

    let slot = shape.lookup(&property);

    assert!(slot.is_some(), "the property should be found in the object");

    let slot = slot.expect("the property should be found in the object");

    assert_eq!(context.slot().index, slot.index);
}

#[test]
fn define_own_property_internal_method_non_existent_property() {
    let context = &mut Context::default();

    let o = context
        .intrinsics()
        .templates()
        .ordinary_object()
        .create(OrdinaryObject, Vec::default());

    let property: PropertyKey = js_string!("prop").into();
    let value = 100;

    o.set(property.clone(), value, true, context)
        .expect("should not fail");

    let context = &mut InternalMethodPropertyContext::new(context);

    assert_eq!(context.slot().index, 0);
    assert_eq!(context.slot().attributes, SlotAttributes::empty());

    o.__define_own_property__(
        &property,
        PropertyDescriptor::builder()
            .value(value)
            .writable(true)
            .configurable(true)
            .enumerable(true)
            .build(),
        context,
    )
    .expect("should not fail");

    assert!(
        !context.slot().in_prototype(),
        "Since it's an owned property, the prototype bit should not be set"
    );

    assert!(
        context.slot().is_cacheable(),
        "Since it's an owned property, this should be cacheable"
    );

    let shape = o.borrow().shape().clone();

    let slot = shape.lookup(&property);

    assert!(slot.is_some(), "the property should be found in the object");

    let slot = slot.expect("the property should be found in the object");

    assert_eq!(context.slot().index, slot.index);
}

#[test]
fn define_own_property_internal_method_existing_property_property() {
    let context = &mut Context::default();

    let o = context
        .intrinsics()
        .templates()
        .ordinary_object()
        .create(OrdinaryObject, Vec::default());

    let property: PropertyKey = js_string!("prop").into();
    let value = 100;

    o.set(property.clone(), value, true, context)
        .expect("should not fail");

    o.__define_own_property__(
        &property,
        PropertyDescriptor::builder()
            .value(value)
            .writable(true)
            .configurable(true)
            .enumerable(true)
            .build(),
        &mut context.into(),
    )
    .expect("should not fail");

    let context = &mut InternalMethodPropertyContext::new(context);

    assert_eq!(context.slot().index, 0);
    assert_eq!(context.slot().attributes, SlotAttributes::empty());

    o.__define_own_property__(
        &property,
        PropertyDescriptor::builder()
            .value(value + 100)
            .writable(true)
            .configurable(true)
            .enumerable(true)
            .build(),
        context,
    )
    .expect("should not fail");

    assert!(
        !context.slot().in_prototype(),
        "Since it's an owned property, the prototype bit should not be set"
    );

    assert!(
        context.slot().is_cacheable(),
        "Since it's an owned property, this should be cacheable"
    );

    let shape = o.borrow().shape().clone();

    let slot = shape.lookup(&property);

    assert!(slot.is_some(), "the property should be found in the object");

    let slot = slot.expect("the property should be found in the object");

    assert_eq!(context.slot().index, slot.index);
}

#[test]
fn set_internal_method() {
    let context = &mut Context::default();

    let o = context
        .intrinsics()
        .templates()
        .ordinary_object()
        .create(OrdinaryObject, Vec::default());

    let property: PropertyKey = js_string!("prop").into();
    let value = 100;

    o.set(property.clone(), value, true, context)
        .expect("should not fail");

    let context = &mut InternalMethodPropertyContext::new(context);

    assert_eq!(context.slot().index, 0);
    assert_eq!(context.slot().attributes, SlotAttributes::empty());

    o.__set__(property.clone(), value.into(), o.clone().into(), context)
        .expect("should not fail");

    assert!(
        !context.slot().in_prototype(),
        "Since it's an owned property, the prototype bit should not be set"
    );

    assert!(
        context.slot().is_cacheable(),
        "Since it's an owned property, this should be cacheable"
    );

    let shape = o.borrow().shape().clone();

    let slot = shape.lookup(&property);

    assert!(slot.is_some(), "the property should be found in the object");

    let slot = slot.expect("the property should be found in the object");

    assert_eq!(context.slot().index, slot.index);
}

fn get_codeblock(value: &JsValue) -> Option<(JsObject, Gc<CodeBlock>)> {
    let object = value.as_object()?.clone();
    let code = object.downcast_ref::<OrdinaryFunction>()?.code.clone();

    Some((object, code))
}

#[test]
fn set_property_by_name_set_inline_cache_on_property_load() -> JsResult<()> {
    let context = &mut Context::default();
    let function = context.eval(Source::from_bytes("(function (o) { return o.test; })"))?;
    let (function, code) = get_codeblock(&function).unwrap();

    assert_eq!(code.ic.len(), 1);
    assert_eq!(code.ic[0].entries.borrow().len(), 0);

    let o = ObjectInitializer::new(context)
        .property(js_string!("test"), 0, Attribute::all())
        .build();
    let o_shape = o.borrow().shape().clone();

    function.call(&JsValue::undefined(), &[o.clone().into()], context)?;

    assert_eq!(code.ic[0].entries.borrow().len(), 1);
    assert_eq!(
        code.ic[0].entries.borrow()[0]
            .shape
            .upgrade()
            .unwrap()
            .to_addr_usize(),
        o_shape.to_addr_usize()
    );

    Ok(())
}

#[test]
fn get_property_by_name_set_inline_cache_on_property_load() -> JsResult<()> {
    let context = &mut Context::default();
    let function = context.eval(Source::from_bytes("(function (o) { o.test = 30; })"))?;
    let (function, code) = get_codeblock(&function).unwrap();

    assert_eq!(code.ic.len(), 1);
    assert_eq!(code.ic[0].entries.borrow().len(), 0);

    let o = ObjectInitializer::new(context)
        .property(js_string!("test"), 0, Attribute::all())
        .build();
    let o_shape = o.borrow().shape().clone();

    function.call(&JsValue::undefined(), &[o.clone().into()], context)?;

    assert_eq!(code.ic[0].entries.borrow().len(), 1);
    assert_eq!(
        code.ic[0].entries.borrow()[0]
            .shape
            .upgrade()
            .unwrap()
            .to_addr_usize(),
        o_shape.to_addr_usize()
    );

    Ok(())
}

#[test]
fn test_polymorphic_inline_cache() -> JsResult<()> {
    let context = &mut Context::default();
    let function = context.eval(Source::from_bytes("(function (o) { return o.test; })"))?;
    let (function, code) = get_codeblock(&function).unwrap();

    assert_eq!(code.ic.len(), 1);
    assert_eq!(code.ic[0].entries.borrow().len(), 0);
    assert!(!code.ic[0].megamorphic.get());

    let shapes = vec![
        ObjectInitializer::new(context)
            .property(js_string!("test"), 1, Attribute::all())
            .build(),
        ObjectInitializer::new(context)
            .property(js_string!("a"), 2, Attribute::all())
            .property(js_string!("test"), 3, Attribute::all())
            .build(),
        ObjectInitializer::new(context)
            .property(js_string!("b"), 4, Attribute::all())
            .property(js_string!("test"), 5, Attribute::all())
            .build(),
        ObjectInitializer::new(context)
            .property(js_string!("c"), 6, Attribute::all())
            .property(js_string!("test"), 7, Attribute::all())
            .build(),
    ];

    for o in &shapes {
        function.call(&JsValue::undefined(), &[o.clone().into()], context)?;
    }

    assert_eq!(code.ic[0].entries.borrow().len(), 4);
    assert!(!code.ic[0].megamorphic.get());

    Ok(())
}

#[test]
fn test_megamorphic_inline_cache() -> JsResult<()> {
    let context = &mut Context::default();
    let function = context.eval(Source::from_bytes("(function (o) { return o.test; })"))?;
    let (function, code) = get_codeblock(&function).unwrap();

    let shapes = vec![
        ObjectInitializer::new(context)
            .property(js_string!("test"), 1, Attribute::all())
            .build(),
        ObjectInitializer::new(context)
            .property(js_string!("a"), 1, Attribute::all())
            .property(js_string!("test"), 1, Attribute::all())
            .build(),
        ObjectInitializer::new(context)
            .property(js_string!("b"), 1, Attribute::all())
            .property(js_string!("test"), 1, Attribute::all())
            .build(),
        ObjectInitializer::new(context)
            .property(js_string!("c"), 1, Attribute::all())
            .property(js_string!("test"), 1, Attribute::all())
            .build(),
        ObjectInitializer::new(context)
            .property(js_string!("d"), 1, Attribute::all())
            .property(js_string!("test"), 1, Attribute::all())
            .build(),
    ];

    for o in &shapes {
        function.call(&JsValue::undefined(), &[o.clone().into()], context)?;
    }

    assert_eq!(code.ic[0].entries.borrow().len(), 0);
    assert!(code.ic[0].megamorphic.get());

    // Regression check: repeated miss should remain empty
    let o6 = ObjectInitializer::new(context)
        .property(js_string!("e"), 1, Attribute::all())
        .property(js_string!("test"), 1, Attribute::all())
        .build();
    function.call(&JsValue::undefined(), &[o6.clone().into()], context)?;
    assert_eq!(code.ic[0].entries.borrow().len(), 0);
    assert!(code.ic[0].megamorphic.get());

    Ok(())
}

/// A method `m` and a getter `g` of `top`, found `DEPTH` prototypes up from `receiver`. The inline
/// cache stores lookups that find the property one prototype up; three prototypes up is looked
/// up without it.
const CHAIN: &str = indoc! {r#"
    const top = { m() { return "top.m"; } };
    Object.defineProperty(top, "g", { get() { return "top.g"; }, configurable: true });
    const middle = Object.create(top);
    const near = Object.create(middle);
    const receiver = Object.create(DEPTH === 1 ? top : near);
    function read() { return receiver.m() + " " + receiver.g; }
"#};

fn chain(depth: u8) -> String {
    format!("const DEPTH = {depth};\n{CHAIN}")
}

/// Runs `source`, which defines `read()`, calls `read()` until its property accesses are cached,
/// runs `mutate` and checks that `read()` then returns `expected`, as an uncached lookup does.
#[track_caller]
fn assert_warm_site_sees(source: &str, mutate: &str, expected: &'static str) {
    run_test_actions([
        TestAction::run(source.to_owned()),
        TestAction::run("for (let i = 0; i < 5; i++) read();"),
        TestAction::run(mutate.to_owned()),
        TestAction::assert_eq("String(read())", js_string!(expected)),
    ]);
}

#[test]
fn prototype_hits_are_cached_one_level_up() {
    run_test_actions([
        TestAction::run(chain(1)),
        TestAction::run("for (let i = 0; i < 5; i++) read();"),
        TestAction::assert_eq("read()", js_string!("top.m top.g")),
        TestAction::inspect_context(|context| {
            let read = context
                .global_object()
                .get(js_string!("read"), context)
                .expect("read is defined");
            let (_, code) = get_codeblock(&read).expect("read is an ordinary function");
            let prototype_hits = code
                .ic
                .iter()
                .filter(|ic| {
                    ic.entries
                        .borrow()
                        .iter()
                        .any(|entry| entry.slot.attributes.contains(SlotAttributes::PROTOTYPE))
                })
                .count();
            assert_eq!(prototype_hits, 2, "the method and the getter are cached");
        }),
    ]);
}

#[test]
fn shadowing_on_the_receiver_invalidates_a_prototype_hit() {
    for depth in [1, 3] {
        assert_warm_site_sees(
            &chain(depth),
            r#"receiver.m = () => "own.m"; Object.defineProperty(receiver, "g", { value: "own.g" });"#,
            "own.m own.g",
        );
    }
}

#[test]
fn shadowing_on_an_intermediate_prototype_invalidates_a_prototype_hit() {
    assert_warm_site_sees(
        &chain(3),
        r#"middle.m = () => "middle.m"; Object.defineProperty(near, "g", { get() { return "near.g"; } });"#,
        "middle.m near.g",
    );
}

#[test]
fn shadowing_on_a_receiver_with_a_unique_shape_invalidates_a_prototype_hit() {
    assert_warm_site_sees(
        indoc! {r#"
            function f() {}
            function read() { return [f.hasOwnProperty === Object.prototype.hasOwnProperty, Math.hasOwnProperty === Object.prototype.hasOwnProperty].join(" "); }
        "#},
        "Function.prototype.hasOwnProperty = () => 0; Math.hasOwnProperty = () => 0;",
        "false false",
    );
}

#[test]
fn redefining_the_holders_property_is_seen_by_a_prototype_hit() {
    for depth in [1, 3] {
        assert_warm_site_sees(
            &chain(depth),
            r#"top.m = () => "new.m"; Object.defineProperty(top, "g", { get() { return "new.g"; } });"#,
            "new.m new.g",
        );
    }
}

#[test]
fn deleting_the_holders_property_invalidates_a_prototype_hit() {
    for depth in [1, 3] {
        assert_warm_site_sees(&chain(depth), "delete top.g;", "top.m undefined");
    }
}

#[test]
fn deleting_an_earlier_property_of_the_holder_keeps_a_prototype_hit_correct() {
    assert_warm_site_sees(
        indoc! {r#"
            function P() {}
            P.prototype.a = 1;
            P.prototype.m = 2;
            const p = new P();
            function read() { return p.m; }
        "#},
        "delete P.prototype.a;",
        "2",
    );
}

#[test]
fn data_and_accessor_conversions_invalidate_a_prototype_hit() {
    for depth in [1, 3] {
        assert_warm_site_sees(
            &chain(depth),
            indoc! {r#"
                Object.defineProperty(top, "m", { get() { return () => "accessor.m"; }, configurable: true });
                Object.defineProperty(top, "g", { value: "data.g", writable: true, configurable: true });
            "#},
            "accessor.m data.g",
        );
    }
}

#[test]
fn prototype_changes_invalidate_a_prototype_hit() {
    assert_warm_site_sees(
        &chain(3),
        r#"Object.setPrototypeOf(middle, { m() { return "other.m"; }, get g() { return "other.g"; } });"#,
        "other.m other.g",
    );
    assert_warm_site_sees(
        &chain(3),
        r#"near.__proto__ = { m() { return "proto.m"; }, get g() { return "proto.g"; } };"#,
        "proto.m proto.g",
    );
    for depth in [1, 3] {
        assert_warm_site_sees(
            &chain(depth),
            r#"Object.setPrototypeOf(receiver, { m() { return "direct.m"; }, g: "direct.g" });"#,
            "direct.m direct.g",
        );
    }
    assert_warm_site_sees(
        &chain(1),
        r#"Object.setPrototypeOf(top, { m() { return "above.m"; }, g: "above.g" });"#,
        "top.m top.g",
    );
}

#[test]
fn freezing_the_holder_keeps_prototype_hits_correct() {
    for depth in [1, 3] {
        assert_warm_site_sees(&chain(depth), "Object.freeze(top);", "top.m top.g");
    }
    assert_warm_site_sees(
        &chain(1),
        r#"Object.defineProperty(top, "m", { configurable: false }); Object.defineProperty(top, "g", { configurable: false });"#,
        "top.m top.g",
    );
    run_test_actions([
        TestAction::run(indoc! {r#"
            const log = [];
            const top = { set x(v) { log.push(v); } };
            const receiver = Object.create(top);
            function write(v) { receiver.x = v; }
            for (let i = 0; i < 5; i++) write(i);
            Object.freeze(top);
            write("frozen");
        "#}),
        TestAction::assert_eq(
            r#"log.join() + " " + Object.hasOwn(receiver, "x")"#,
            js_string!("0,1,2,3,4,frozen false"),
        ),
    ]);
}

#[test]
fn each_receiver_shape_of_a_site_checks_its_own_holder() {
    assert_warm_site_sees(
        indoc! {r#"
            const a = { first: 1, m: "a.m" };
            const b = { m: "b.m" };
            const fromA = Object.create(a);
            const fromB = Object.create(b);
            function get(o) { return o.m; }
            function read() { return get(fromA) + " " + get(fromB); }
        "#},
        r#"delete a.first; b.m = "b.m2";"#,
        "a.m b.m2",
    );
}

#[test]
fn lookups_through_a_proxy_are_not_cached() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            let traps = 0;
            const top = { m: "top.m" };
            const proxy = new Proxy(Object.create(top), {
                get(target, key, receiver) { traps++; return Reflect.get(target, key, receiver); },
            });
            const receiver = Object.create(proxy);
            function read() { return receiver.m; }
            for (let i = 0; i < 5; i++) read();
        "#}),
        TestAction::assert_eq("read() + \" \" + traps", js_string!("top.m 6")),
    ]);
}

/// `write()` assigns through a setter of `top`, `DEPTH` prototypes up from `receiver`, until the
/// assignment is cached; then that property becomes a data property and `write("own")` must create
/// a property on `receiver`.
const SETTER_CHAIN: &str = indoc! {r#"
    let stored = "";
    const top = { set x(v) { stored = v; } };
    const middle = Object.create(top);
    const receiver = Object.create(DEPTH === 1 ? top : Object.create(middle));
    function write(v) { receiver.x = v; }
    for (let i = 0; i < 5; i++) write("setter");
    Object.defineProperty(DEPTH === 1 ? top : middle, "x", { value: "data", writable: true, configurable: true });
    write("own");
"#};

#[test]
fn assignment_through_a_prototype_setter_sees_a_new_data_property() {
    for depth in [1, 3] {
        run_test_actions([
            TestAction::run(format!("const DEPTH = {depth};\n{SETTER_CHAIN}")),
            TestAction::assert_eq(
                r#"stored + " " + Object.hasOwn(receiver, "x") + " " + receiver.x"#,
                js_string!("setter true own"),
            ),
        ]);
    }
}

#[test]
fn deleting_an_earlier_property_of_the_holder_keeps_a_prototype_setter_hit_correct() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            const log = [];
            function P() {}
            P.prototype.a = 1;
            Object.defineProperty(P.prototype, "x", { set(v) { log.push("set:" + v); }, configurable: true });
            P.prototype.b = function () { log.push("b called"); };
            const p = new P();
            function write(v) { p.x = v; }
            for (let i = 0; i < 5; i++) write(i);
            log.length = 0;
            delete P.prototype.a;
            write("after");
        "#}),
        TestAction::assert_eq(
            r#"log.join() + " " + Object.hasOwn(p, "x")"#,
            js_string!("set:after false"),
        ),
    ]);
}

#[test]
fn global_name_lookups_see_changes_of_the_global_objects_prototype_chain() {
    assert_warm_site_sees(
        indoc! {r#"
            Object.prototype.answer = 1;
            Object.prototype.question = 2;
            function read() { return question; }
        "#},
        "delete Object.prototype.answer;",
        "2",
    );
    assert_warm_site_sees(
        indoc! {r#"
            Object.prototype.question = 2;
            function read() { return question; }
        "#},
        "globalThis.question = 3;",
        "3",
    );
}

#[test]
fn an_accessor_that_changes_the_objects_during_the_lookup_does_not_cache_a_stale_slot() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            const o = { get x() { delete this.x; this.y = 1; return 1; }, z: 5 };
            function read() { return o.x; }
            const first = read();
        "#}),
        TestAction::assert_eq("first + \" \" + read()", js_string!("1 undefined")),
    ]);
    run_test_actions([
        TestAction::run(indoc! {r#"
            const top = { a: 1, get x() { delete top.a; return "x"; }, w: "w" };
            const p = Object.create(top);
            function read() { return p.x; }
            const first = read();
        "#}),
        TestAction::assert_eq("first + \" \" + read()", js_string!("x x")),
    ]);
    run_test_actions([
        TestAction::run(indoc! {r#"
            const log = [];
            const o = { set x(v) { log.push(v); delete this.x; this.y = 1; }, z() { log.push("z called"); } };
            function write(v) { o.x = v; }
            write(1);
            write(2);
        "#}),
        TestAction::assert_eq(
            r#"log.join() + " " + o.x + " " + Object.hasOwn(o, "x")"#,
            js_string!("1 2 true"),
        ),
    ]);
}

/// `read()` finds `m` on `top`, `DEPTH` prototypes up from `receiver`; then every object of that
/// chain becomes garbage and new chains are allocated, possibly at the same addresses.
const COLLECTED_CHAIN: &str = indoc! {r#"
    function chain(tag) {
        const top = { m: tag };
        return DEPTH === 1 ? Object.create(top) : Object.create(Object.create(Object.create(top)));
    }
    let receiver = chain("first");
    function read() { return receiver.m; }
    for (let i = 0; i < 5; i++) read();
    receiver = null;
"#};

/// Writes `Math.x` from a sloppy and a strict function and reads it until all three are cached.
/// `Math` has a unique shape.
const WARM_UNIQUE_WRITES: &str = indoc! {r#"
    Math.x = 49;
    function write(v) { Math.x = v; }
    function strictWrite(v) { "use strict"; Math.x = v; }
    function read() { return Math.x; }
    for (let i = 0; i < 5; i++) { write(49); strictWrite(49); read(); }
"#};

#[test]
fn making_a_property_of_a_unique_shape_read_only_stops_cached_writes() {
    for change in [
        r#"Object.defineProperty(Math, "x", { writable: false });"#,
        "Object.freeze(Math);",
    ] {
        run_test_actions([
            TestAction::run(WARM_UNIQUE_WRITES),
            TestAction::run(change),
            TestAction::run("write(999);"),
            TestAction::assert_eq("read()", 49),
            TestAction::assert_native_error(
                "strictWrite(1000)",
                JsNativeErrorKind::Type,
                "cannot set non-writable property: x",
            ),
            TestAction::assert_eq("read()", 49),
        ]);
    }
}

#[test]
fn enumerable_and_configurable_changes_of_a_unique_shape_keep_cached_accesses_correct() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            Math.x = 1;
            function write(v) { Math.x = v; }
            function read() { return Math.x; }
            for (let i = 0; i < 5; i++) { write(i); read(); }
            Object.defineProperty(Math, "x", { enumerable: false });
            write(5);
            const afterEnumerable = read();
            Object.defineProperty(Math, "x", { configurable: false });
            write(6);
        "#}),
        TestAction::assert_eq(
            r#"afterEnumerable + " " + read() + " " + Object.keys(Math).includes("x") + " " + delete Math.x + " " + Math.x"#,
            js_string!("5 6 false false 6"),
        ),
    ]);
}

#[test]
fn replacing_an_accessor_is_seen_by_cached_accesses() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            const log = [];
            const shared = { get acc() { return "shared"; }, set acc(v) { log.push("shared:" + v); } };
            Object.defineProperty(Math, "acc", { get() { return "unique"; }, set(v) { log.push("unique:" + v); }, configurable: true });
            Object.defineProperty(Object.prototype, "inherited", { get() { return "proto"; }, set(v) { log.push("proto:" + v); }, configurable: true });
            const plain = {};
            function read(o) { return o.acc; }
            function readInherited() { return plain.inherited; }
            function strictWrite(o, v) { "use strict"; o.acc = v; }
            function strictWriteInherited(v) { "use strict"; plain.inherited = v; }
            for (let i = 0; i < 5; i++) { read(shared); read(Math); readInherited(); strictWrite(shared, i); strictWrite(Math, i); strictWriteInherited(i); }
            log.length = 0;
            Object.defineProperty(shared, "acc", { get() { return "shared2"; }, set: undefined });
            Object.defineProperty(Math, "acc", { get() { return "unique2"; }, set: undefined });
            Object.defineProperty(Object.prototype, "inherited", { get() { return "proto2"; }, set: undefined });
        "#}),
        TestAction::assert_eq(
            r#"read(shared) + " " + read(Math) + " " + readInherited()"#,
            js_string!("shared2 unique2 proto2"),
        ),
        TestAction::assert_native_error(
            "strictWrite(shared, 9)",
            JsNativeErrorKind::Type,
            "cannot set non-writable property: acc",
        ),
        TestAction::assert_native_error(
            "strictWrite(Math, 9)",
            JsNativeErrorKind::Type,
            "cannot set non-writable property: acc",
        ),
        TestAction::assert_native_error(
            "strictWriteInherited(9)",
            JsNativeErrorKind::Type,
            "cannot set non-writable property: inherited",
        ),
        TestAction::assert_eq(
            r#"log.length + " " + Object.hasOwn(plain, "inherited")"#,
            js_string!("0 false"),
        ),
    ]);
}

#[test]
fn deleting_a_property_of_a_unique_shape_keeps_cached_accesses_correct() {
    run_test_actions([
        TestAction::run(indoc! {r#"
            Math.x = 1;
            Math.y = 2;
            function write(v) { Math.y = v; }
            function read() { return Math.y; }
            for (let i = 0; i < 5; i++) { write(2); read(); }
            delete Math.x;
            const afterEarlier = read();
            write(3);
            const afterWrite = read();
            delete Math.y;
            const afterOwn = String(read());
            write(4);
        "#}),
        TestAction::assert_eq(
            r#"afterEarlier + " " + afterWrite + " " + afterOwn + " " + read()"#,
            js_string!("2 3 undefined 4"),
        ),
    ]);
}

#[test]
fn a_collected_holder_does_not_satisfy_a_prototype_hit() {
    for depth in [1, 3] {
        run_test_actions([
            TestAction::run(format!("const DEPTH = {depth};\n{COLLECTED_CHAIN}")),
            TestAction::inspect_context(|_| boa_gc::force_collect()),
            TestAction::run(indoc! {r#"
                const fresh = [];
                for (let i = 0; i < 100; i++) fresh.push(chain("second" + i));
                receiver = fresh[99];
            "#}),
            TestAction::assert_eq("read()", js_string!("second99")),
        ]);
    }
}

#[test]
fn cached_length_assignment_keeps_the_length_logic_of_exotic_objects() {
    run_test_actions([
        // Each loop runs the assignment site three times, the later ones through its cache.
        TestAction::assert_eq(
            indoc! {r#"
                var out = [];
                for (var i = 0; i < 3; i++) {
                    var a = [1, 2, 3];
                    a.length = 1;
                    out.push(a[1] + ':' + a.length + ':' + Object.keys(a).length);
                }
                out.join(' ')
            "#},
            js_str!("undefined:1:1 undefined:1:1 undefined:1:1"),
        ),
        TestAction::assert_eq(
            indoc! {r#"
                var out = [];
                for (var i = 0; i < 3; i++) {
                    var a = [1, 2];
                    try { a.length = i === 2 ? -1 : '1'; out.push(typeof a.length + a.length); }
                    catch (e) { out.push(e.name); }
                }
                out.join(' ')
            "#},
            js_str!("number1 number1 RangeError"),
        ),
        TestAction::assert_eq(
            indoc! {r#"
                var out = [];
                for (var i = 0; i < 3; i++) {
                    var a = [1, 2, 3];
                    a.foo = 1;
                    a.length = 0;
                    out.push(a.length + ':' + a[0]);
                }
                out.join(' ')
            "#},
            js_str!("0:undefined 0:undefined 0:undefined"),
        ),
        // The length of an arguments object is an ordinary property, and the mapping of the
        // parameters does not depend on it.
        TestAction::assert_eq(
            indoc! {r#"
                function g(a, b) { arguments.length = 0; b = 5; return arguments[1] + ':' + arguments.length; }
                [g(1, 2), g(1, 2), g(1, 2)].join(' ')
            "#},
            js_str!("5:0 5:0 5:0"),
        ),
        TestAction::assert_eq(
            indoc! {r#"
                (function () {
                    'use strict';
                    var out = [];
                    for (var i = 0; i < 3; i++) {
                        var t = new Uint8Array(4);
                        var s = new String('abc');
                        try { t.length = 1; out.push('set'); } catch (e) { out.push(e.name + t.length); }
                        try { s.length = 1; out.push('set'); } catch (e) { out.push(e.name + s.length); }
                    }
                    return out.join(' ');
                })()
            "#},
            js_str!("TypeError4 TypeError3 TypeError4 TypeError3 TypeError4 TypeError3"),
        ),
    ]);
}

#[test]
fn a_function_that_never_runs_allocates_no_cache_entries() -> JsResult<()> {
    let context = &mut Context::default();
    let function = context.eval(Source::from_bytes(
        "(function (o) { o.a = o.b.c; return o.a + o.d(); })",
    ))?;
    let (function, code) = get_codeblock(&function).unwrap();

    assert!(!code.ic.is_empty());
    for ic in &*code.ic {
        assert_eq!(ic.entries.borrow().capacity(), 0);
    }

    let o = context.eval(Source::from_bytes(
        "({ a: 1, b: { c: 2 }, d() { return 3; } })",
    ))?;
    let result = function.call(&JsValue::undefined(), &[o], context)?;

    assert_eq!(result.as_number(), Some(5.0));
    for ic in &*code.ic {
        assert_eq!(ic.entries.borrow().len(), 1);
    }

    Ok(())
}

#[test]
fn access_sites_compiled_apart_share_the_string_of_their_name() -> JsResult<()> {
    let context = &mut Context::default();
    let f = context.eval(Source::from_bytes(
        "(function (o) { return o.sharedName; })",
    ))?;
    let (_, f) = get_codeblock(&f).unwrap();
    let before = f.ic[0].name.refcount();

    let g = context.eval(Source::from_bytes(
        "(function (o) { return o.sharedName; })",
    ))?;
    let (_, g) = get_codeblock(&g).unwrap();

    assert_eq!(f.ic[0].name, g.ic[0].name);
    assert!(before.is_some());
    assert!(f.ic[0].name.refcount() > before);

    Ok(())
}
