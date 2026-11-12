//! Serializes plain objects and dense arrays into one buffer, for `JSON.stringify` calls without
//! a replacer function, a property list or a gap.
//!
//! [`serialize`] produces the string `SerializeJSONObject` or `SerializeJSONArray` would produce,
//! taking the same observable steps in the same order:
//! - a value is serialized here only if it is an ordinary object or an array whose `toJSON`
//!   lookup provably finds nothing: the object and every prototype have ordinary `[[Get]]`,
//!   `[[GetOwnProperty]]` and `[[GetPrototypeOf]]`, and none has an own `toJSON` property;
//! - own data properties are read from the object's storage, which is what `Get` returns for them;
//!   everything else (accessors, holes, `BigInt` values, other objects) goes through
//!   [`Json::serialize_json_property`], the generic step, one property at a time;
//! - the generic step can run user code, which can change any object being serialized. Before it
//!   runs, every object still being walked here saves the keys it has left from
//!   `EnumerableOwnPropertyNames`, the list the specification iterates over, and then finishes
//!   with the generic step for each of them. Arrays need no saved state: their length is read
//!   once, and each element is read from dense storage only while it is there.

use std::ptr::fn_addr_eq;

use crate::{
    Context, JsResult, JsString, JsValue,
    builtins::{Array, object::OrdinaryObject},
    error::JsNativeError,
    js_string,
    object::{JsObject, internal_methods::ORDINARY_INTERNAL_METHODS, shape::slot::SlotAttributes},
    property::{PropertyKey, PropertyNameKind},
    value::JsVariant,
    vm::check_native_stack,
};

use super::{Json, StateRecord};

/// How [`serialize`] writes a value.
#[derive(Clone, Copy)]
pub(super) enum Kind {
    Object,
    Array,
}

/// An object whose properties are being written here.
struct Frame {
    object: JsObject,
    /// Keys of `object` visited so far, counted in `EnumerableOwnPropertyNames` order.
    visited: usize,
    /// The keys left to visit, saved before user code could run.
    remaining: Option<Vec<JsString>>,
}

/// State of the direct serializer, kept in [`StateRecord`].
pub(super) struct DirectState {
    frames: Vec<Frame>,
    /// Prototypes whose chain has ordinary lookups and no `toJSON`; cleared whenever user code
    /// may run.
    clean_prototypes: Vec<JsObject>,
    to_json: PropertyKey,
}

impl DirectState {
    pub(super) fn new() -> Self {
        Self {
            frames: Vec::new(),
            clean_prototypes: Vec::new(),
            to_json: js_string!("toJSON").into(),
        }
    }
}

/// Key of the property being serialized, for the generic step.
#[derive(Clone, Copy)]
enum Key<'a> {
    String(&'a JsString),
    Index(u32),
}

fn has_ordinary_lookup(object: &JsObject) -> bool {
    let vtable = object.vtable();
    fn_addr_eq(vtable.__get__, ORDINARY_INTERNAL_METHODS.__get__)
        && fn_addr_eq(
            vtable.__get_own_property__,
            ORDINARY_INTERNAL_METHODS.__get_own_property__,
        )
        && fn_addr_eq(
            vtable.__get_prototype_of__,
            ORDINARY_INTERNAL_METHODS.__get_prototype_of__,
        )
}

/// Returns how `value` can be serialized here, or `None` if the generic steps must serialize it.
pub(super) fn kind(value: &JsObject, state: &mut StateRecord) -> Option<Kind> {
    let kind = if value.is::<OrdinaryObject>() {
        Kind::Object
    } else if value.is::<Array>() {
        Kind::Array
    } else {
        return None;
    };
    if !has_ordinary_lookup(value)
        || !fn_addr_eq(
            value.vtable().__own_property_keys__,
            ORDINARY_INTERNAL_METHODS.__own_property_keys__,
        )
    {
        return None;
    }
    let direct = &mut state.direct;
    let mut object = value.clone();
    let mut first_prototype = None;
    loop {
        let prototype = {
            let borrowed = object.borrow();
            if borrowed
                .properties()
                .shape
                .lookup(&direct.to_json)
                .is_some()
            {
                return None;
            }
            borrowed.prototype()
        };
        let Some(prototype) = prototype else {
            break;
        };
        if direct
            .clean_prototypes
            .iter()
            .any(|clean| JsObject::equals(clean, &prototype))
        {
            break;
        }
        if !has_ordinary_lookup(&prototype) {
            return None;
        }
        if first_prototype.is_none() {
            first_prototype = Some(prototype.clone());
        }
        object = prototype;
    }
    if let Some(prototype) = first_prototype {
        direct.clean_prototypes.push(prototype);
    }
    Some(kind)
}

/// Serializes `value`, which [`kind`] classified as `kind`.
pub(super) fn serialize(
    state: &mut StateRecord,
    value: &JsObject,
    kind: Kind,
    context: &mut Context,
) -> JsResult<JsString> {
    // The generic steps that led here may have run user code.
    state.direct.clean_prototypes.clear();
    let mut out = Vec::new();
    write_kind(state, value, kind, &mut out, context)?;
    Ok(JsString::from(&out[..]))
}

fn write_kind(
    state: &mut StateRecord,
    value: &JsObject,
    kind: Kind,
    out: &mut Vec<u16>,
    context: &mut Context,
) -> JsResult<()> {
    check_native_stack(state.stack_floor)?;
    if !state.stack_set.insert(value.clone()) {
        return Err(JsNativeError::typ()
            .with_message("cyclic object value")
            .into());
    }
    match kind {
        Kind::Object => write_object(state, value, out, context)?,
        Kind::Array => write_array(state, value, out, context)?,
    }
    state.stack_set.remove(value);
    Ok(())
}

/// Writes `value`, read from the property `key` of `holder`. Returns `false` and writes nothing if
/// the property serializes to undefined.
fn write_value(
    state: &mut StateRecord,
    value: &JsValue,
    key: Key<'_>,
    holder: &JsObject,
    out: &mut Vec<u16>,
    context: &mut Context,
) -> JsResult<bool> {
    match value.variant() {
        JsVariant::Null => out.extend_from_slice(&NULL),
        JsVariant::Undefined | JsVariant::Symbol(_) => return Ok(false),
        JsVariant::Boolean(true) => out.extend_from_slice(&TRUE),
        JsVariant::Boolean(false) => out.extend_from_slice(&FALSE),
        JsVariant::String(string) => Json::quote_json_string_into(&string, out),
        JsVariant::Integer32(integer) => out.extend(JsString::from(integer).iter()),
        JsVariant::Float64(number) => {
            if number.is_finite() {
                out.extend(JsString::from(number).iter());
            } else {
                out.extend_from_slice(&NULL);
            }
        }
        JsVariant::BigInt(_) => return generic(state, key, holder, out, context),
        JsVariant::Object(object) => match kind(&object, state) {
            Some(kind) => write_kind(state, &object, kind, out, context)?,
            None => return generic(state, key, holder, out, context),
        },
    }
    Ok(true)
}

/// Runs the generic `SerializeJSONProperty` for one property, after every object being walked
/// here has saved the keys it has left.
fn generic(
    state: &mut StateRecord,
    key: Key<'_>,
    holder: &JsObject,
    out: &mut Vec<u16>,
    context: &mut Context,
) -> JsResult<bool> {
    for index in 0..state.direct.frames.len() {
        let frame = &state.direct.frames[index];
        if frame.remaining.is_none() {
            // Each frame is in the middle of its `visited`-th key, which it finishes itself.
            let (object, skip) = (frame.object.clone(), frame.visited + 1);
            let keys = remaining_keys(&object, skip, context)?;
            state.direct.frames[index].remaining = Some(keys);
        }
    }
    let key = match key {
        Key::String(key) => key.clone(),
        Key::Index(index) => index.into(),
    };
    state.direct.clean_prototypes.clear();
    let result = Json::serialize_json_property(state, key, holder, context)?;
    state.direct.clean_prototypes.clear();
    Ok(match result {
        Some(string) => {
            out.extend(string.iter());
            true
        }
        None => false,
    })
}

/// `EnumerableOwnPropertyNames(object, key)` without its first `skip` keys.
fn remaining_keys(
    object: &JsObject,
    skip: usize,
    context: &mut Context,
) -> JsResult<Vec<JsString>> {
    let keys = object.enumerable_own_property_names(PropertyNameKind::Key, context)?;
    Ok(keys
        .into_iter()
        .skip(skip)
        .filter_map(|key| key.as_string())
        .collect())
}

/// Writes the member for `key` unless `write` reports that its value serializes to undefined.
fn write_member(
    out: &mut Vec<u16>,
    first: &mut bool,
    key: &JsString,
    write: impl FnOnce(&mut Vec<u16>) -> JsResult<bool>,
) -> JsResult<()> {
    let mark = out.len();
    if !*first {
        out.push(u16::from(b','));
    }
    Json::quote_json_string_into(key, out);
    out.push(u16::from(b':'));
    if write(out)? {
        *first = false;
    } else {
        out.truncate(mark);
    }
    Ok(())
}

fn write_object(
    state: &mut StateRecord,
    object: &JsObject,
    out: &mut Vec<u16>,
    context: &mut Context,
) -> JsResult<()> {
    let depth = state.direct.frames.len();
    state.direct.frames.push(Frame {
        object: object.clone(),
        visited: 0,
        remaining: None,
    });
    out.push(u16::from(b'{'));
    let mut first = true;

    let dense_len = object.borrow().properties().dense_indexed_properties_len();
    if let Some(len) = dense_len {
        for index in 0..len {
            let value = object.borrow().properties().get_dense_property(index);
            write_member(out, &mut first, &JsString::from(index), |out| match value {
                Some(value) => write_value(state, &value, Key::Index(index), object, out, context),
                None => generic(state, Key::Index(index), object, out, context),
            })?;
            state.direct.frames[depth].visited += 1;
            if state.direct.frames[depth].remaining.is_some() {
                break;
            }
        }
    } else {
        // Sparse integer keys are visited in ascending order; the saved key list has that order.
        let keys = remaining_keys(object, 0, context)?;
        state.direct.frames[depth].remaining = Some(keys);
    }

    let mut entry = 0;
    while state.direct.frames[depth].remaining.is_none() {
        let read = {
            let borrowed = object.borrow();
            let properties = borrowed.properties();
            properties.shape.with_entry(entry, |key, slot| {
                let PropertyKey::String(key) = key else {
                    return None;
                };
                if !slot.attributes.contains(SlotAttributes::ENUMERABLE) {
                    return None;
                }
                let value = (!slot.attributes.is_accessor_descriptor())
                    .then(|| properties.storage[slot.index as usize].clone());
                Some((key.clone(), value))
            })
        };
        entry += 1;
        let Some(read) = read else {
            break;
        };
        let Some((key, value)) = read else {
            continue;
        };
        write_member(out, &mut first, &key, |out| match value {
            Some(value) => write_value(state, &value, Key::String(&key), object, out, context),
            None => generic(state, Key::String(&key), object, out, context),
        })?;
        state.direct.frames[depth].visited += 1;
    }

    let remaining = state.direct.frames[depth].remaining.take();
    // Deeper frames are gone; this one needs no saved keys once it stops reading storage.
    state.direct.frames.truncate(depth);
    for key in remaining.iter().flatten() {
        write_member(out, &mut first, key, |out| {
            generic(state, Key::String(key), object, out, context)
        })?;
    }
    out.push(u16::from(b'}'));
    Ok(())
}

fn write_array(
    state: &mut StateRecord,
    array: &JsObject,
    out: &mut Vec<u16>,
    context: &mut Context,
) -> JsResult<()> {
    let len = array.length_of_array_like(context)?;
    out.push(u16::from(b'['));
    for index in 0..len {
        if index > 0 {
            out.push(u16::from(b','));
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the length of an array is at most u32::MAX"
        )]
        let index = index as u32;
        let value = array.borrow().properties().get_dense_property(index);
        let written = match value {
            Some(value) => write_value(state, &value, Key::Index(index), array, out, context)?,
            None => generic(state, Key::Index(index), array, out, context)?,
        };
        if !written {
            out.extend_from_slice(&NULL);
        }
    }
    out.push(u16::from(b']'));
    Ok(())
}

const NULL: [u16; 4] = [b'n' as u16, b'u' as u16, b'l' as u16, b'l' as u16];
const TRUE: [u16; 4] = [b't' as u16, b'r' as u16, b'u' as u16, b'e' as u16];
const FALSE: [u16; 5] = [
    b'f' as u16,
    b'a' as u16,
    b'l' as u16,
    b's' as u16,
    b'e' as u16,
];
