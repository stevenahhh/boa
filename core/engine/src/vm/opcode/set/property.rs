use crate::JsExpect;
use crate::JsValue;
use crate::value::JsVariant;
use crate::vm::opcode::{IndexOperand, RegisterOperand};
use crate::{
    Context, JsNativeError, JsObject, JsResult,
    builtins::{array::ARRAY_EXOTIC_INTERNAL_METHODS, function::set_function_name},
    object::{
        internal_methods::{
            InternalMethodPropertyContext,
            immutable_prototype::IMMUTABLE_PROTOTYPE_EXOTIC_INTERNAL_METHODS,
        },
        shape::slot::SlotAttributes,
    },
    property::{PropertyDescriptor, PropertyKey},
    vm::opcode::Operation,
};
use boa_macros::js_str;

fn set_by_name(
    value: RegisterOperand,
    value_object: &JsValue,
    receiver: &JsValue,
    index: IndexOperand,
    context: &mut Context,
) -> JsResult<()> {
    let value = context.vm.get_register(value.into()).clone();

    let object = value_object.to_object(context)?;

    let ic = &context.vm.frame().code_block().ic[usize::from(index)];

    let object_borrowed = object.borrow();
    let cached = ic.get(object_borrowed.properties(), |properties, slot| {
        slot.attributes
            .is_accessor_descriptor()
            .then(|| properties.storage[slot.index as usize + 1].clone())
    });
    drop(object_borrowed);
    if let Some((slot, setter)) = cached {
        match setter {
            // An assignment through an accessor without a setter fails, which throws in strict
            // code: the slow path below does that.
            Some(setter) => {
                if slot.attributes.has_set()
                    && let Some(setter) = setter.as_object()
                {
                    setter.call(receiver, std::slice::from_ref(&value), context)?;
                    return Ok(());
                }
            }
            // `OrdinarySet` never caches a data property found on the prototype: assigning to it
            // creates a property on the receiver.
            None if !slot.attributes.contains(SlotAttributes::PROTOTYPE) => {
                let mut object_borrowed = object.borrow_mut();
                object_borrowed.properties_mut().storage[slot.index as usize] = value.clone();
                return Ok(());
            }
            None => {}
        }
    }

    let name: PropertyKey = ic.name.clone().into();

    let context = &mut InternalMethodPropertyContext::new(context);
    let succeeded = object.__set__(name.clone(), value.clone(), receiver.clone(), context)?;
    if !succeeded && context.vm.frame().code_block.strict() {
        return Err(JsNativeError::typ()
            .with_message(format!("cannot set non-writable property: {name}"))
            .into());
    }

    // Cache the property.
    let slot = *context.slot();
    if succeeded && slot.is_cacheable() {
        let ic = &context.vm.frame().code_block.ic[usize::from(index)];
        let object_borrowed = object.borrow();
        let shape = object_borrowed.shape();
        ic.set(shape, slot);
    }

    Ok(())
}

/// `SetPropertyByName` implements the Opcode Operation for `Opcode::SetPropertyByName`
///
/// Operation:
///  - Sets a property by name of an object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetPropertyByName;

impl SetPropertyByName {
    #[inline(always)]
    pub(crate) fn operation(
        (value, object, index): (RegisterOperand, RegisterOperand, IndexOperand),
        context: &mut Context,
    ) -> JsResult<()> {
        let object = context.vm.get_register(object.into()).clone();
        set_by_name(value, &object, &object, index, context)
    }
}

impl Operation for SetPropertyByNameWithThis {
    const NAME: &'static str = "SetPropertyByNameWithThis";
    const INSTRUCTION: &'static str = "INST - SetPropertyByNameWithThis";
    const COST: u8 = 4;
}

/// `SetPropertyByNameWithThis` implements the Opcode Operation for `Opcode::SetPropertyByNameWithThis`
///
/// Operation:
///  - Sets a property by name of an object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetPropertyByNameWithThis;

impl SetPropertyByNameWithThis {
    #[inline(always)]
    pub(crate) fn operation(
        (value, receiver, object, index): (
            RegisterOperand,
            RegisterOperand,
            RegisterOperand,
            IndexOperand,
        ),
        context: &mut Context,
    ) -> JsResult<()> {
        let value_object = context.vm.get_register(object.into()).clone();
        let receiver = context.vm.get_register(receiver.into()).clone();
        set_by_name(value, &value_object, &receiver, index, context)
    }
}

impl Operation for SetPropertyByName {
    const NAME: &'static str = "SetPropertyByName";
    const INSTRUCTION: &'static str = "INST - SetPropertyByName";
    const COST: u8 = 4;
}

/// `SetPropertyByValue` implements the Opcode Operation for `Opcode::SetPropertyByValue`
///
/// Operation:
///  - Sets a property by value of an object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetPropertyByValue;

impl SetPropertyByValue {
    #[inline(always)]
    pub(crate) fn operation(
        (value, key, receiver, object): (
            RegisterOperand,
            RegisterOperand,
            RegisterOperand,
            RegisterOperand,
        ),
        context: &mut Context,
    ) -> JsResult<()> {
        let same_register = u32::from(receiver) == u32::from(object);
        let value = context.vm.get_register(value.into()).clone();
        let key = context.vm.get_register(key.into()).clone();
        let receiver = context.vm.get_register(receiver.into()).clone();
        let object = context.vm.get_register(object.into()).clone();
        let object = object.to_object(context)?;

        let key = key.to_property_key(context)?;

        // Fast Path:
        //
        // Writing a dense element the array owns and appending one are what
        // `OrdinarySet(array, index, value, receiver)` does only when the receiver is the array.
        // With another receiver, such as the `this` of `super[index] = value` in a method whose
        // home object inherits from an array, the property is defined on the receiver. A receiver
        // read from the register of an array is that array, so `o[k] = v`, which passes one
        // register as both, skips comparing the values.
        'fast_path: {
            if object.is_array()
                && let PropertyKey::Index(index) = &key
                && (same_register
                    || receiver
                        .as_object()
                        .is_some_and(|receiver| JsObject::equals(&receiver, &object)))
            {
                let mut object_borrowed = object.borrow_mut();

                // Cannot modify if not extensible.
                if !object_borrowed.extensible {
                    break 'fast_path;
                }

                if object_borrowed
                    .properties_mut()
                    .set_dense_property(index.get(), &value)
                {
                    return Ok(());
                }
                let is_append = object_borrowed.properties().dense_indexed_properties_len()
                    == Some(index.get());
                drop(object_borrowed);

                if is_append && append_dense_array_element(&object, index.get(), &value, context) {
                    return Ok(());
                }
            }
        }

        // Slow path:
        let succeeded = object.__set__(
            key.clone(),
            value.clone(),
            receiver.clone(),
            &mut context.into(),
        )?;
        if !succeeded && context.vm.frame().code_block.strict() {
            return Err(JsNativeError::typ()
                .with_message(format!("cannot set non-writable property: {key}"))
                .into());
        }

        Ok(())
    }
}

/// Appends `value` as the element `index` of a dense array when `OrdinarySet` would do exactly that.
///
/// `OrdinarySet(array, index, value, array)` for an index the array does not own asks each object
/// on the prototype chain for `index`. When none of them has it, it ends in
/// `CreateDataProperty(array, index, value)`, which adds a writable, enumerable and configurable
/// element and, for an index at or past `length`, sets `length` to `index + 1`.
///
/// The shortcut is only taken when no step of that path can run user code or fail:
/// - the array has the realm's array template shape, so `length` is its only named property and
///   is writable, and its prototype is the realm's `%Array.prototype%`;
/// - the array is extensible and `index` is the length of its dense element storage, so the array
///   does not own `index` and adding it leaves no hole;
/// - each object on the prototype chain is an array or the immutable prototype
///   `%Object.prototype%`, whose `[[GetOwnProperty]]`, `[[GetPrototypeOf]]` and `[[Set]]` are the
///   ordinary ones, and owns no indexed property.
fn append_dense_array_element(
    array: &JsObject,
    index: u32,
    value: &JsValue,
    context: &Context,
) -> bool {
    let (old_len, mut prototype) = {
        let array = array.borrow();
        let properties = array.properties();
        if !array.extensible
            || properties.shape.to_addr_usize()
                != context
                    .intrinsics()
                    .templates()
                    .array()
                    .shape()
                    .to_addr_usize()
            || properties.dense_indexed_properties_len() != Some(index)
        {
            return false;
        }
        let Some(old_len) = properties.storage[0].as_i32() else {
            return false;
        };
        (old_len, array.prototype())
    };

    while let Some(object) = prototype {
        let vtable = object.vtable();
        if !std::ptr::eq(vtable, &raw const ARRAY_EXOTIC_INTERNAL_METHODS)
            && !std::ptr::eq(
                vtable,
                &raw const IMMUTABLE_PROTOTYPE_EXOTIC_INTERNAL_METHODS,
            )
        {
            return false;
        }
        let object = object.borrow();
        if object.properties().index_property_keys().next().is_some() {
            return false;
        }
        prototype = object.prototype();
    }

    let mut array = array.borrow_mut();
    let properties = array.properties_mut();
    if !properties.indexed_properties.push_dense(value) {
        return false;
    }
    if i64::from(index) >= i64::from(old_len) {
        properties.storage[0] = JsValue::new(index + 1);
    }
    true
}

impl Operation for SetPropertyByValue {
    const NAME: &'static str = "SetPropertyByValue";
    const INSTRUCTION: &'static str = "INST - SetPropertyByValue";
    const COST: u8 = 4;
}

/// `SetPropertyGetterByName` implements the Opcode Operation for `Opcode::SetPropertyGetterByName`
///
/// Operation:
///  - Sets a getter property by name of an object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetPropertyGetterByName;

impl SetPropertyGetterByName {
    #[inline(always)]
    pub(crate) fn operation(
        (object, value, index): (RegisterOperand, RegisterOperand, IndexOperand),
        context: &mut Context,
    ) -> JsResult<()> {
        let object = context.vm.get_register(object.into()).clone();
        let value = context.vm.get_register(value.into()).clone();
        let name = context
            .vm
            .frame()
            .code_block()
            .constant_string(index.into())
            .into();

        let object = object.to_object(context)?;
        let set = object
            .__get_own_property__(&name, &mut InternalMethodPropertyContext::new(context))?
            .as_ref()
            .and_then(PropertyDescriptor::set)
            .cloned();
        object.__define_own_property__(
            &name,
            PropertyDescriptor::builder()
                .maybe_get(Some(value.clone()))
                .maybe_set(set)
                .enumerable(true)
                .configurable(true)
                .build(),
            &mut InternalMethodPropertyContext::new(context),
        )?;
        Ok(())
    }
}

impl Operation for SetPropertyGetterByName {
    const NAME: &'static str = "SetPropertyGetterByName";
    const INSTRUCTION: &'static str = "INST - SetPropertyGetterByName";
    const COST: u8 = 4;
}

/// `SetPropertyGetterByValue` implements the Opcode Operation for `Opcode::SetPropertyGetterByValue`
///
/// Operation:
///  - Sets a getter property by value of an object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetPropertyGetterByValue;

impl SetPropertyGetterByValue {
    #[inline(always)]
    pub(crate) fn operation(
        (value, key, object): (RegisterOperand, RegisterOperand, RegisterOperand),
        context: &mut Context,
    ) -> JsResult<()> {
        let value = context.vm.get_register(value.into()).clone();
        let key = context.vm.get_register(key.into()).clone();
        let object = context.vm.get_register(object.into()).clone();
        let object = object.to_object(context)?;
        let name = key.to_property_key(context)?;

        let set = object
            .__get_own_property__(&name, &mut InternalMethodPropertyContext::new(context))?
            .as_ref()
            .and_then(PropertyDescriptor::set)
            .cloned();
        object.__define_own_property__(
            &name,
            PropertyDescriptor::builder()
                .maybe_get(Some(value.clone()))
                .maybe_set(set)
                .enumerable(true)
                .configurable(true)
                .build(),
            &mut InternalMethodPropertyContext::new(context),
        )?;
        Ok(())
    }
}

impl Operation for SetPropertyGetterByValue {
    const NAME: &'static str = "SetPropertyGetterByValue";
    const INSTRUCTION: &'static str = "INST - SetPropertyGetterByValue";
    const COST: u8 = 4;
}

/// `SetPropertySetterByName` implements the Opcode Operation for `Opcode::SetPropertySetterByName`
///
/// Operation:
///  - Sets a setter property by name of an object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetPropertySetterByName;

impl SetPropertySetterByName {
    #[inline(always)]
    pub(crate) fn operation(
        (object, value, index): (RegisterOperand, RegisterOperand, IndexOperand),
        context: &mut Context,
    ) -> JsResult<()> {
        let object = context.vm.get_register(object.into()).clone();
        let value = context.vm.get_register(value.into()).clone();
        let name = context
            .vm
            .frame()
            .code_block()
            .constant_string(index.into())
            .into();

        let object = object.to_object(context)?;

        let get = object
            .__get_own_property__(&name, &mut InternalMethodPropertyContext::new(context))?
            .as_ref()
            .and_then(PropertyDescriptor::get)
            .cloned();
        object.__define_own_property__(
            &name,
            PropertyDescriptor::builder()
                .maybe_set(Some(value.clone()))
                .maybe_get(get)
                .enumerable(true)
                .configurable(true)
                .build(),
            &mut InternalMethodPropertyContext::new(context),
        )?;
        Ok(())
    }
}

impl Operation for SetPropertySetterByName {
    const NAME: &'static str = "SetPropertySetterByName";
    const INSTRUCTION: &'static str = "INST - SetPropertySetterByName";
    const COST: u8 = 4;
}

/// `SetPropertySetterByValue` implements the Opcode Operation for `Opcode::SetPropertySetterByValue`
///
/// Operation:
///  - Sets a setter property by value of an object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetPropertySetterByValue;

impl SetPropertySetterByValue {
    #[inline(always)]
    pub(crate) fn operation(
        (value, key, object): (RegisterOperand, RegisterOperand, RegisterOperand),
        context: &mut Context,
    ) -> JsResult<()> {
        let value = context.vm.get_register(value.into()).clone();
        let key = context.vm.get_register(key.into()).clone();
        let object = context.vm.get_register(object.into()).clone();

        let object = object.to_object(context)?;
        let name = key.to_property_key(context)?;

        let get = object
            .__get_own_property__(&name, &mut InternalMethodPropertyContext::new(context))?
            .as_ref()
            .and_then(PropertyDescriptor::get)
            .cloned();
        object.__define_own_property__(
            &name,
            PropertyDescriptor::builder()
                .maybe_set(Some(value.clone()))
                .maybe_get(get)
                .enumerable(true)
                .configurable(true)
                .build(),
            &mut InternalMethodPropertyContext::new(context),
        )?;
        Ok(())
    }
}

impl Operation for SetPropertySetterByValue {
    const NAME: &'static str = "SetPropertySetterByValue";
    const INSTRUCTION: &'static str = "INST - SetPropertySetterByValue";
    const COST: u8 = 4;
}

/// `SetFunctionName` implements the Opcode Operation for `Opcode::SetFunctionName`
///
/// Operation:
///  - Sets the name of a function object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetFunctionName;

impl SetFunctionName {
    #[inline(always)]
    pub(crate) fn operation(
        (function, name, prefix): (RegisterOperand, RegisterOperand, IndexOperand),
        context: &mut Context,
    ) -> JsResult<()> {
        let function = context.vm.get_register(function.into()).clone();
        let name = context.vm.get_register(name.into()).clone();
        let name = match name.variant() {
            JsVariant::String(name) => PropertyKey::from(name.clone()),
            JsVariant::Symbol(name) => PropertyKey::from(name.clone()),
            _ => unreachable!(),
        };

        let prefix = match u32::from(prefix) {
            1 => Some(js_str!("get")),
            2 => Some(js_str!("set")),
            _ => None,
        };

        set_function_name(
            &function
                .as_object()
                .js_expect("function is not an object")?,
            &name,
            prefix,
            context,
        )?;
        Ok(())
    }
}

impl Operation for SetFunctionName {
    const NAME: &'static str = "SetFunctionName";
    const INSTRUCTION: &'static str = "INST - SetFunctionName";
    const COST: u8 = 4;
}
