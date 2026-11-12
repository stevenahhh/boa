//! This module implements the `StringIterator` object.
//!
//! More information:
//!  - [ECMAScript reference][spec]
//!
//! [spec]: https://tc39.es/ecma262/#sec-string-iterator-objects

use crate::{
    Context, JsData, JsResult, JsString, JsValue,
    builtins::{BuiltInBuilder, IntrinsicObject, iterable::create_iter_result_object},
    context::intrinsics::Intrinsics,
    error::JsNativeError,
    js_string,
    object::JsObject,
    property::Attribute,
    realm::Realm,
    symbol::JsSymbol,
};
use boa_gc::{Finalize, Trace};

/// The `StringIterator` object represents an iteration over a string. It implements the iterator protocol.
///
/// More information:
///  - [ECMAScript reference][spec]
///
/// [spec]: https://tc39.es/ecma262/#sec-string-iterator-objects
#[derive(Debug, Clone, Finalize, Trace, JsData)]
pub(crate) struct StringIterator {
    string: JsString,
    next_index: usize,
}

impl IntrinsicObject for StringIterator {
    fn init(realm: &Realm) {
        let next = BuiltInBuilder::callable_with_object(
            realm,
            realm
                .intrinsics()
                .objects()
                .string_iterator_next()
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
                js_string!("String Iterator"),
                Attribute::CONFIGURABLE,
            )
            .build();
    }

    fn get(intrinsics: &Intrinsics) -> JsObject {
        intrinsics.objects().iterator_prototypes().string()
    }
}

impl StringIterator {
    /// Create a new `StringIterator`.
    pub(crate) fn create_string_iterator(string: JsString, context: &mut Context) -> JsObject {
        JsObject::from_proto_and_data_with_shared_shape(
            context.root_shape(),
            context
                .intrinsics()
                .objects()
                .iterator_prototypes()
                .string(),
            Self {
                string,
                next_index: 0,
            },
        )
        .upcast()
    }

    /// `StringIterator.prototype.next( )`
    pub(crate) fn next(this: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let object = this.as_object();
        let mut string_iterator = object
            .as_ref()
            .and_then(JsObject::downcast_mut::<Self>)
            .ok_or_else(|| JsNativeError::typ().with_message("`this` is not an ArrayIterator"))?;
        let (value, done) = string_iterator.advance(context)?;
        Ok(create_iter_result_object(value, done, context))
    }

    /// Runs `%StringIteratorPrototype%.next` on `iterator` without creating the result object,
    /// returning its `value` and `done`, or `None`, changing nothing, if `iterator` is not a
    /// `StringIterator`. The steps of `next` run no user code.
    pub(crate) fn step(
        iterator: &JsObject,
        context: &mut Context,
    ) -> JsResult<Option<(JsValue, bool)>> {
        let Some(mut string_iterator) = iterator.downcast_mut::<Self>() else {
            return Ok(None);
        };
        string_iterator.advance(context).map(Some)
    }

    /// The steps of `%StringIteratorPrototype%.next` after the `this` check, returning the
    /// `value` and `done` of the result.
    fn advance(&mut self, context: &mut Context) -> JsResult<(JsValue, bool)> {
        let string_iterator = self;
        if string_iterator.string.is_empty() {
            return Ok((JsValue::undefined(), true));
        }
        let native_string = &string_iterator.string;
        let len = native_string.len();
        let position = string_iterator.next_index;
        if position >= len {
            string_iterator.string = js_string!();
            return Ok((JsValue::undefined(), true));
        }
        let code_point = native_string.code_point_at(position);
        string_iterator.next_index += code_point.code_unit_count();
        let result_string = crate::builtins::string::String::substring(
            &string_iterator.string.clone().into(),
            &[position.into(), string_iterator.next_index.into()],
            context,
        )?;
        Ok((result_string, false))
    }
}
