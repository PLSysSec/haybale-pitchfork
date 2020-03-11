//! This module contains a few simple built-in function hooks which can be used
//! with `Config.function_hooks`.

use crate::default_hook::{ArgumentKind, is_or_points_to_secret};
use crate::secret;
use haybale::function_hooks::{IsCall, generic_stub_hook};
use haybale::{Project, Result, ReturnValue, State};
use llvm_ir::{Name, Type, Typed};

/// This hook will ignore all of the function arguments and simply return an
/// unconstrained public value of the appropriate size, or void for void-typed
/// functions.
///
/// This is merely a convenience alias for `haybale`'s `generic_stub_hook`.
pub fn return_public_unconstrained(
    proj: &Project,
    state: &mut State<secret::Backend>,
    call: &dyn IsCall,
) -> Result<ReturnValue<secret::BV>> {
    generic_stub_hook(proj, state, call)
}

/// This hook will ignore all of the function arguments and simply return a
/// secret value of the appropriate size, or void for void-typed functions.
pub fn return_secret(
    _proj: &Project,
    state: &mut State<secret::Backend>,
    call: &dyn IsCall,
) -> Result<ReturnValue<secret::BV>> {
    match call.get_type() {
        Type::VoidType => Ok(ReturnValue::ReturnVoid),
        ty => {
            let width = haybale::layout::size(&ty);
            let bv = state.new_bv_with_name(Name::from("return_secret_retval"), width as u32)?;
            Ok(ReturnValue::Return(bv))
        },
    }
}

/// This hook will return a secret value if any of the arguments are secret, or
/// if any of the arguments contain a pointer to any secret data.
/// Otherwise, it will return an unconstrained public value.
///
/// Assumes that opaque struct types do not contain secret data or pointers to
/// secret data.
pub fn propagate_taint(
    proj: &Project,
    state: &mut State<secret::Backend>,
    call: &dyn IsCall,
) -> Result<ReturnValue<secret::BV>> {
    for arg in call.get_arguments().iter().map(|(arg, _)| arg) {
        let arg_bv = state.operand_to_bv(arg)?;
        match is_or_points_to_secret(proj, state, &arg_bv, &arg.get_type())? {
            ArgumentKind::Public | ArgumentKind::Unknown => {},
            ArgumentKind::Secret => return return_secret(proj, state, call),
        }
    }
    // if we got here, we didn't find any secret data
    return_public_unconstrained(proj, state, call)
}
