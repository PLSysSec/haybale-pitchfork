use crate::secret;
use either::Either;
use haybale::{Error, Project, Result, ReturnValue, State};
use haybale::backend::BV;
use haybale::function_hooks::IsCall;
use llvm_ir::{Constant, Name, Operand, Type, Typed};
use log::info;

pub fn pitchfork_default_hook(
    proj: &Project,
    state: &mut State<secret::Backend>,
    call: &dyn IsCall,
) -> Result<ReturnValue<secret::BV>> {
    let called_funcname = match call.get_called_func() {
        Either::Left(_) => panic!("invoked default hook for an inline assembly call"),  // this shouldn't happen
        Either::Right(Operand::ConstantOperand(Constant::GlobalReference { name: Name::Name(name), .. })) => Some(name),
        Either::Right(Operand::ConstantOperand(Constant::GlobalReference { name, .. })) => panic!("Function with a numbered name: {:?}", name),
        Either::Right(_) => None,  // a function pointer
    };
    match called_funcname {
        Some(funcname) => info!("Using Pitchfork default hook for a function named {:?}", state.demangle(funcname)),
        None => info!("Using Pitchfork default hook for a function pointer"),
    };

    for (i, arg) in call.get_arguments().iter().map(|(arg, _)| arg).enumerate() {
        // if the arg is secret, or points to any secret data, then raise an error and require a manually-specified hook or LLVM definition
        let arg_bv = state.operand_to_bv(arg)?;
        if is_or_points_to_secret(proj, state, &arg_bv, &arg.get_type())? {
            return Err(Error::OtherError(format!("Encountered a call of a function named {:?}, but didn't find an LLVM definition or function hook for it; and its argument #{} (zero-indexed) may refer to secret data.\nTo fix this error, you can do one of these three options:\n  (1) choose to simply ignore this function call by adding it to `config.function_hooks` with `haybale::function_hooks::generic_stub_hook` as the hook;\n  (2) rerun with more bitcode files in the `Project` so that the symbolic execution can find an LLVM definition for {:?};\n  (3) write your own custom hook for {:?}", called_funcname, i, called_funcname, called_funcname)));
        }
    }

    // if we get here, no secret data is being handled by this function, so we just default to generic_stub_hook
    haybale::function_hooks::generic_stub_hook(proj, state, call)
}

/// Returns `true` if the operand either is secret itself, or points to any secret data.
fn is_or_points_to_secret(proj: &Project, state: &mut State<secret::Backend>, bv: &secret::BV, ty: &llvm_ir::Type) -> Result<bool> {
    if bv.is_secret() {
        Ok(true)
    } else {
        match ty {
            Type::PointerType { pointee_type, .. } => {
                // also check if it points to any secret data
                let pointee = state.read(&bv, haybale::layout::size_opaque_aware(&**pointee_type, proj) as u32)?;
                is_or_points_to_secret(proj, state, &pointee, &**pointee_type)
            },
            Type::VectorType { element_type, num_elements } | Type::ArrayType { element_type, num_elements } => {
                // TODO: this could be made more efficient
                let bv_width = bv.get_width();
                for i in 0 .. *num_elements {
                    let ptr_to_element = bv.add(&state.bv_from_u32(i as u32, bv_width));
                    if is_or_points_to_secret(proj, state, &ptr_to_element, &**element_type)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            },
            Type::StructType { element_types, .. } => {
                let mut offset_bits = 0;
                let bv_width = bv.get_width();
                for element_ty in element_types {
                    let ptr_to_element = bv.add(&state.bv_from_u32(offset_bits / 8, bv_width));
                    if is_or_points_to_secret(proj, state, &ptr_to_element, element_ty)? {
                        return Ok(true);
                    }
                    offset_bits += haybale::layout::size_opaque_aware(element_ty, proj) as u32;
                    assert_eq!(offset_bits % 8, 0, "Struct offset of {} bits is not a multiple of 8 bits", offset_bits);
                }
                Ok(false)
            },
            Type::NamedStructType { .. } => {
                match proj.get_inner_struct_type_from_named(ty) {
                    None => Err(Error::OtherError("is_or_points_to_secret on an opaque struct type".into())),
                    Some(arc) => is_or_points_to_secret(proj, state, bv, &arc.read().unwrap()),
                }
            },
            _ => Ok(false),  // for any other type, the `is_secret()` check above was sufficient
        }
    }
}
