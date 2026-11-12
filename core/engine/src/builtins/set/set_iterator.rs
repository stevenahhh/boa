//! This module implements the `SetIterator` object.
//!
//! More information:
//!  - [ECMAScript reference][spec]
//!
//! [spec]: https://tc39.es/ecma262/#sec-set-iterator-objects

use super::ordered_set::OrderedSet;
use crate::{
    Context, JsData, JsResult,
    builtins::{
        Array, BuiltInBuilder, IntrinsicObject, JsValue, iterable::create_iter_result_object,
    },
    context::intrinsics::Intrinsics,
    error::JsNativeError,
    js_string,
    object::JsObject,
    property::{Attribute, PropertyNameKind},
    realm::Realm,
    symbol::JsSymbol,
};
use boa_gc::{Finalize, Trace};

#[derive(Debug, Trace)]
struct SetIteratorLock(JsObject<OrderedSet>);

impl SetIteratorLock {
    fn new(js_object: JsObject<OrderedSet>) -> Self {
        js_object.borrow_mut().data_mut().lock();
        Self(js_object)
    }
}

impl Finalize for SetIteratorLock {
    fn finalize(&self) {
        self.0.borrow_mut().data_mut().unlock();
    }
}

/// The Set Iterator object represents an iteration over a set. It implements the iterator protocol.
///
/// More information:
///  - [ECMAScript reference][spec]
///
/// [spec]: https://tc39.es/ecma262/#sec-set-iterator-objects
#[derive(Debug, Finalize, Trace, JsData)]
pub(crate) struct SetIterator {
    iterated_set: Option<SetIteratorLock>,
    next_index: usize,
    #[unsafe_ignore_trace]
    iteration_kind: PropertyNameKind,
}

impl IntrinsicObject for SetIterator {
    fn init(realm: &Realm) {
        let next = BuiltInBuilder::callable_with_object(
            realm,
            realm
                .intrinsics()
                .objects()
                .set_iterator_next()
                .clone()
                .into(),
            Self::next,
        )
        .name(js_string!("next"))
        .build();

        BuiltInBuilder::with_intrinsic::<Self>(realm)
            .prototype(realm.intrinsics().constructors().iterator().prototype())
            .static_property(
                js_string!("next"),
                next,
                Attribute::WRITABLE | Attribute::CONFIGURABLE,
            )
            .static_property(
                JsSymbol::to_string_tag(),
                js_string!("Set Iterator"),
                Attribute::CONFIGURABLE,
            )
            .build();
    }

    fn get(intrinsics: &Intrinsics) -> JsObject {
        intrinsics.objects().iterator_prototypes().set()
    }
}

impl SetIterator {
    /// Abstract operation `CreateSetIterator( set, kind )`
    ///
    /// Creates a new iterator over the given set.
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-createsetiterator
    pub(crate) fn create_set_iterator(
        set: JsObject<OrderedSet>,
        kind: PropertyNameKind,
        context: &Context,
    ) -> JsValue {
        let set_iterator = JsObject::from_proto_and_data_with_shared_shape(
            context.root_shape(),
            context.intrinsics().objects().iterator_prototypes().set(),
            Self {
                iterated_set: Some(SetIteratorLock::new(set)),
                next_index: 0,
                iteration_kind: kind,
            },
        );
        set_iterator.into()
    }

    /// %SetIteratorPrototype%.next( )
    ///
    /// Advances the iterator and gets the next result in the set.
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-%setiteratorprototype%.next
    pub(crate) fn next(this: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let object = this.as_object();
        let mut set_iterator = object
            .as_ref()
            .and_then(JsObject::downcast_mut::<Self>)
            .ok_or_else(|| JsNativeError::typ().with_message("`this` is not an SetIterator"))?;
        let (value, done) = set_iterator.advance(context);
        Ok(create_iter_result_object(value, done, context))
    }

    /// Runs `%SetIteratorPrototype%.next` on `iterator` without creating the result object,
    /// returning its `value` and `done`, or `None`, changing nothing, if `iterator` is not a
    /// `SetIterator`. The steps of `next` run no user code.
    pub(crate) fn step(iterator: &JsObject, context: &mut Context) -> Option<(JsValue, bool)> {
        let mut set_iterator = iterator.downcast_mut::<Self>()?;
        Some(set_iterator.advance(context))
    }

    /// The steps of `%SetIteratorPrototype%.next` after the `this` check, returning the `value`
    /// and `done` of the result.
    fn advance(&mut self, context: &mut Context) -> (JsValue, bool) {
        let set_iterator = self;
        let item_kind = set_iterator.iteration_kind;

        if let Some(obj) = set_iterator.iterated_set.take() {
            let e = {
                let mut entries = obj.0.borrow_mut();
                let entries = entries.data_mut();
                let len = entries.full_len();
                loop {
                    let element = entries.get_index(set_iterator.next_index);
                    set_iterator.next_index += 1;
                    if element.is_some() || set_iterator.next_index >= len {
                        break element.cloned();
                    }
                }
            };
            if let Some(element) = e {
                let item = match item_kind {
                    PropertyNameKind::KeyAndValue => {
                        Array::create_array_from_list([element.clone(), element], context).into()
                    }
                    _ => element,
                };
                set_iterator.iterated_set = Some(obj);
                return (item, false);
            }
        }

        (JsValue::undefined(), true)
    }
}
