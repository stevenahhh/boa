//! This module implements the `ArrayIterator` object.
//!
//! More information:
//!  - [ECMAScript reference][spec]
//!
//! [spec]: https://tc39.es/ecma262/#sec-array-iterator-objects

use crate::{
    Context, JsData, JsResult,
    builtins::{
        Array, BuiltInBuilder, IntrinsicObject, JsValue, iterable::create_iter_result_object,
        typed_array::TypedArray,
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

/// The Array Iterator object represents an iteration over an array. It implements the iterator protocol.
///
/// More information:
///  - [ECMAScript reference][spec]
///
/// [spec]: https://tc39.es/ecma262/#sec-array-iterator-objects
#[derive(Debug, Clone, Finalize, Trace, JsData)]
pub(crate) struct ArrayIterator {
    array: JsObject,
    next_index: u64,
    #[unsafe_ignore_trace]
    kind: PropertyNameKind,
    done: bool,
}

impl IntrinsicObject for ArrayIterator {
    fn init(realm: &Realm) {
        let next = BuiltInBuilder::callable_with_object(
            realm,
            realm
                .intrinsics()
                .objects()
                .array_iterator_next()
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
                js_string!("Array Iterator"),
                Attribute::CONFIGURABLE,
            )
            .build();
    }

    fn get(intrinsics: &Intrinsics) -> JsObject {
        intrinsics.objects().iterator_prototypes().array()
    }
}

impl ArrayIterator {
    fn new(array: JsObject, kind: PropertyNameKind) -> Self {
        Self {
            array,
            kind,
            next_index: 0,
            done: false,
        }
    }

    /// `CreateArrayIterator( array, kind )`
    ///
    /// Creates a new iterator over the given array.
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-createarrayiterator
    pub(crate) fn create_array_iterator(
        array: JsObject,
        kind: PropertyNameKind,
        context: &Context,
    ) -> JsValue {
        let array_iterator = JsObject::from_proto_and_data_with_shared_shape(
            context.root_shape(),
            context.intrinsics().objects().iterator_prototypes().array(),
            Self::new(array, kind),
        );
        array_iterator.into()
    }

    /// Runs `%ArrayIteratorPrototype%.next` on `iterator` without creating the result object,
    /// returning its `value` and `done`, when that cannot run any user code: `iterator` is an
    /// `ArrayIterator` that is either done or reads an element of the dense storage of an
    /// `Array`. Elements of dense storage are own data properties below `length`, so the steps
    /// of `next` read the same value. Returns `None`, changing nothing, in every other case.
    pub(crate) fn step_dense(
        iterator: &JsObject,
        context: &mut Context,
    ) -> Option<(JsValue, bool)> {
        let mut array_iterator = iterator.downcast_mut::<Self>()?;
        if array_iterator.done {
            return Some((JsValue::undefined(), true));
        }
        if !array_iterator.array.is_array() {
            return None;
        }
        let index = u32::try_from(array_iterator.next_index).ok()?;
        let element = array_iterator
            .array
            .borrow()
            .properties()
            .get_dense_property(index)?;
        array_iterator.next_index += 1;
        let value = match array_iterator.kind {
            PropertyNameKind::Key => index.into(),
            PropertyNameKind::Value => element,
            PropertyNameKind::KeyAndValue => {
                Array::create_array_from_list([index.into(), element], context).into()
            }
        };
        Some((value, false))
    }

    /// %ArrayIteratorPrototype%.next( )
    ///
    /// Gets the next result in the array.
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-%arrayiteratorprototype%.next
    pub(crate) fn next(this: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        let object = this.as_object();
        let mut array_iterator = object
            .as_ref()
            .and_then(JsObject::downcast_mut::<Self>)
            .ok_or_else(|| JsNativeError::typ().with_message("`this` is not an ArrayIterator"))?;
        let index = array_iterator.next_index;
        if array_iterator.done {
            return Ok(create_iter_result_object(
                JsValue::undefined(),
                true,
                context,
            ));
        }

        let len = if let Some(f) = array_iterator.array.downcast_ref::<TypedArray>() {
            let buf = f.viewed_array_buffer().as_buffer();
            let Some(buf) = buf
                .bytes(std::sync::atomic::Ordering::SeqCst)
                .filter(|buf| !f.is_out_of_bounds(buf.len()))
            else {
                return Err(JsNativeError::typ()
                    .with_message("Cannot get value from out of bounds typed array")
                    .into());
            };

            f.array_length(buf.len())
        } else {
            array_iterator.array.length_of_array_like(context)?
        };

        if index >= len {
            array_iterator.done = true;
            return Ok(create_iter_result_object(
                JsValue::undefined(),
                true,
                context,
            ));
        }
        array_iterator.next_index = index + 1;
        match array_iterator.kind {
            PropertyNameKind::Key => Ok(create_iter_result_object(index.into(), false, context)),
            PropertyNameKind::Value => {
                let element_value = array_iterator.array.get(index, context)?;
                Ok(create_iter_result_object(element_value, false, context))
            }
            PropertyNameKind::KeyAndValue => {
                let element_value = array_iterator.array.get(index, context)?;
                let result = Array::create_array_from_list([index.into(), element_value], context);
                Ok(create_iter_result_object(result.into(), false, context))
            }
        }
    }
}
