use std::ptr::fn_addr_eq;

use boa_ast::scope::{BindingLocator, BindingLocatorScope};

use crate::{
    Context, JsError, JsExpect, JsNativeError, JsResult, JsValue,
    environments::Environment,
    object::{internal_methods::ORDINARY_INTERNAL_METHODS, shape::slot::SlotAttributes},
    property::PropertyKey,
    vm::opcode::{IndexOperand, Operation, RegisterOperand},
};

/// `ThrowMutateImmutable` implements the Opcode Operation for `Opcode::ThrowMutateImmutable`
///
/// Operation:
///  - Throws an error because the binding access is illegal.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ThrowMutateImmutable;

impl ThrowMutateImmutable {
    #[inline(always)]
    pub(crate) fn operation(index: IndexOperand, context: &mut Context) -> JsError {
        let name = context
            .vm
            .frame()
            .code_block()
            .constant_string(index.into());

        JsNativeError::typ()
            .with_message(format!(
                "cannot mutate an immutable binding '{}'",
                name.to_std_string_escaped()
            ))
            .into()
    }
}

impl Operation for ThrowMutateImmutable {
    const NAME: &'static str = "ThrowMutateImmutable";
    const INSTRUCTION: &'static str = "INST - ThrowMutateImmutable";
    const COST: u8 = 2;
}

/// `SetName` implements the Opcode Operation for `Opcode::SetName`
///
/// Operation:
///  - Find a binding on the environment chain and assign its value.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetName;

impl SetName {
    #[inline(always)]
    pub(crate) fn operation(
        (value, index): (RegisterOperand, IndexOperand),
        context: &mut Context,
    ) -> JsResult<()> {
        let value = context.vm.get_register(value.into()).clone();
        let code_block = context.vm.frame().code_block();
        if context.set_static_declarative_binding(&code_block.bindings[usize::from(index)], &value)
        {
            return Ok(());
        }
        let mut binding_locator = code_block.bindings[usize::from(index)].clone();
        let strict = code_block.strict();

        context.find_runtime_binding(&mut binding_locator)?;

        if set_global_object_data_property(&binding_locator, &value, context) {
            return Ok(());
        }

        verify_initialized(&binding_locator, context)?;

        context.set_binding(&binding_locator, value.clone(), strict)?;

        Ok(())
    }
}

impl Operation for SetName {
    const NAME: &'static str = "SetName";
    const INSTRUCTION: &'static str = "INST - SetName";
    const COST: u8 = 4;
}

/// `SetNameByLocator` implements the Opcode Operation for `Opcode::SetNameByLocator`
///
/// Operation:
///  - Assigns a value to the binding pointed by the `current_binding` of the current frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetNameByLocator;

impl SetNameByLocator {
    #[inline(always)]
    pub(crate) fn operation(value: RegisterOperand, context: &mut Context) -> JsResult<()> {
        let frame = context.vm.frame_mut();
        let strict = frame.code_block.strict();
        let mut binding_locator = frame
            .binding_stack
            .pop()
            .js_expect("locator should have been popped before")?;
        let value = context.vm.get_register(value.into()).clone();

        if context.is_deleted_binding(&binding_locator) {
            context.find_runtime_binding(&mut binding_locator)?;
        }

        if set_global_object_data_property(&binding_locator, &value, context) {
            return Ok(());
        }

        verify_initialized(&binding_locator, context)?;

        context.set_binding(&binding_locator, value.clone(), strict)?;

        Ok(())
    }
}

impl Operation for SetNameByLocator {
    const NAME: &'static str = "SetNameByLocator";
    const INSTRUCTION: &'static str = "INST - SetNameByLocator";
    const COST: u8 = 4;
}

/// Assigns `value` to a binding of the global object that is an own writable data property of it,
/// when the global object has the ordinary internal methods.
///
/// That is what `verify_initialized` and `set_binding` do for such a binding: `[[HasProperty]]`
/// finds the own property, and `OrdinarySet` writes the value of an own writable data property
/// of its receiver, without calling anything. Returns `false`, having done nothing, when the
/// binding is not one: then the caller takes the generic path.
fn set_global_object_data_property(
    locator: &BindingLocator,
    value: &JsValue,
    context: &Context,
) -> bool {
    if locator.scope() != BindingLocatorScope::GlobalObject {
        return false;
    }
    let global = context.vm.frame().realm.global_object();
    let vtable = global.vtable();
    if !(fn_addr_eq(
        vtable.__has_property__,
        ORDINARY_INTERNAL_METHODS.__has_property__,
    ) && fn_addr_eq(vtable.__set__, ORDINARY_INTERNAL_METHODS.__set__)
        && fn_addr_eq(
            vtable.__get_own_property__,
            ORDINARY_INTERNAL_METHODS.__get_own_property__,
        )
        && fn_addr_eq(
            vtable.__define_own_property__,
            ORDINARY_INTERNAL_METHODS.__define_own_property__,
        ))
    {
        return false;
    }
    // A binding name is an identifier, which is never an array index.
    let key = PropertyKey::String(locator.name().clone());
    let mut global = global.borrow_mut();
    let properties = global.properties_mut();
    let Some(slot) = properties.shape.lookup(&key) else {
        return false;
    };
    if slot.attributes.is_accessor_descriptor()
        || !slot.attributes.contains(SlotAttributes::WRITABLE)
    {
        return false;
    }
    properties.storage[slot.index as usize] = value.clone();
    true
}

/// Checks that the binding pointed by `locator` exists and is initialized.
fn verify_initialized(locator: &BindingLocator, context: &mut Context) -> JsResult<()> {
    if !context.is_initialized_binding(locator)? {
        let key = locator.name();
        let strict = context.vm.frame().code_block.strict();

        let message = match locator.scope() {
            BindingLocatorScope::GlobalObject if strict => Some(format!(
                "cannot assign to uninitialized global property `{}`",
                key.to_std_string_escaped()
            )),
            BindingLocatorScope::GlobalObject => None,
            BindingLocatorScope::GlobalDeclarative => Some(format!(
                "cannot assign to uninitialized binding `{}`",
                key.to_std_string_escaped()
            )),
            BindingLocatorScope::Stack(index) => match context.environment_expect(index) {
                Environment::Declarative(_) => Some(format!(
                    "cannot assign to uninitialized binding `{}`",
                    key.to_std_string_escaped()
                )),
                Environment::Object(_) if strict => Some(format!(
                    "cannot assign to uninitialized property `{}`",
                    key.to_std_string_escaped()
                )),
                Environment::Object(_) => None,
            },
        };

        if let Some(message) = message {
            return Err(JsNativeError::reference().with_message(message).into());
        }
    }

    Ok(())
}
