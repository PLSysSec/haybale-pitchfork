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
        match is_or_points_to_secret(proj, state, &arg_bv, &arg.get_type())? {
            ArgumentKind::Secret => return Err(Error::OtherError(format!("Encountered a call of a function named {:?}, but didn't find an LLVM definition or function hook for it; and its argument #{} (zero-indexed) may refer to secret data.\nTo fix this error, you can do one of these three options:\n  (1) choose to simply ignore this function call by adding it to `config.function_hooks` with `haybale::function_hooks::generic_stub_hook` as the hook;\n  (2) rerun with more bitcode files in the `Project` so that the symbolic execution can find an LLVM definition for {:?};\n  (3) write your own custom hook for {:?}", called_funcname, i, called_funcname, called_funcname))),
            ArgumentKind::Unknown => return Err(Error::OtherError(format!("Encountered a call of a function named {:?}, but didn't find an LLVM definition or function hook for it; and its argument #{} (zero-indexed) involves an opaque struct type, so we're not sure if it may contain secret data.\nTo fix this error, you can do one of these three options:\n  (1) choose to simply ignore this function call by adding it to `config.function_hooks` with `haybale::function_hooks::generic_stub_hook` as the hook;\n  (2) rerun with more bitcode files in the `Project` so that the symbolic execution can find an LLVM definition for {:?};\n  (3) write your own custom hook for {:?}", called_funcname, i, called_funcname, called_funcname))),
            ArgumentKind::Public => {},
        }
    }

    // if we get here, no secret data is being handled by this function, so we just default to generic_stub_hook
    haybale::function_hooks::generic_stub_hook(proj, state, call)
}

enum ArgumentKind {
    /// The argument is fully public, and (if it's a pointer or contains pointer(s)) any pointed-to data is also public
    Public,
    /// The argument is secret, or it's a pointer or contains pointer(s) and some pointed-to data is secret
    Secret,
    /// Couldn't fully analyze the argument because it points to, or contains pointer(s) to, an opaque struct type
    Unknown,
}

/// Classifies the `bv` into an `ArgumentKind` - see notes on `ArgumentKind`
fn is_or_points_to_secret(proj: &Project, state: &mut State<secret::Backend>, bv: &secret::BV, ty: &llvm_ir::Type) -> Result<ArgumentKind> {
    if bv.is_secret() {
        Ok(ArgumentKind::Secret)
    } else {
        match ty {
            Type::PointerType { pointee_type, .. } => {
                // also check if it points to any secret data
                let pointee_size_bits = match haybale::layout::size_opaque_aware(&**pointee_type, proj) {
                    None => return Ok(ArgumentKind::Unknown),
                    Some(size) => size,
                };
                let pointee = state.read(&bv, pointee_size_bits as u32)?;
                is_or_points_to_secret(proj, state, &pointee, &**pointee_type)
            },
            Type::VectorType { element_type, num_elements } | Type::ArrayType { element_type, num_elements } => {
                // TODO: this could be made more efficient
                let bv_width = bv.get_width();
                let mut retval = ArgumentKind::Public;
                for i in 0 .. *num_elements {
                    let ptr_to_element = bv.add(&state.bv_from_u32(i as u32, bv_width));
                    match is_or_points_to_secret(proj, state, &ptr_to_element, &**element_type)? {
                        ArgumentKind::Secret => return Ok(ArgumentKind::Secret),  // we're done, there's definitely a Secret
                        ArgumentKind::Unknown => retval = ArgumentKind::Unknown,  // keep going, maybe we'll find a Secret later
                        ArgumentKind::Public => {},  // leave in place the previous retval
                    }
                }
                Ok(retval)  // this will be Unknown if we ever encountered an Unknown, or Public if everything came back Public
            },
            Type::StructType { element_types, .. } => {
                let mut offset_bits = 0;
                let bv_width = bv.get_width();
                let mut retval = ArgumentKind::Public;
                for element_ty in element_types {
                    let ptr_to_element = bv.add(&state.bv_from_u32(offset_bits / 8, bv_width));
                    match is_or_points_to_secret(proj, state, &ptr_to_element, element_ty)? {
                        ArgumentKind::Secret => return Ok(ArgumentKind::Secret),  // we're done, there's definitely a Secret
                        ArgumentKind::Unknown => retval = ArgumentKind::Unknown,  // keep going, maybe we'll find a Secret later
                        ArgumentKind::Public => {},  // leave in place the previous retval
                    }
                    offset_bits += match haybale::layout::size_opaque_aware(element_ty, proj) {
                        Some(size) => size as u32,
                        None => return Ok(ArgumentKind::Unknown),  // we have no way to keep going - we don't know the next offset
                    };
                    assert_eq!(offset_bits % 8, 0, "Struct offset of {} bits is not a multiple of 8 bits", offset_bits);
                }
                Ok(retval)  // this will be Unknown if we ever encountered an Unknown, or Public if everything came back Public
            },
            Type::NamedStructType { .. } => {
                match proj.get_inner_struct_type_from_named(ty) {
                    None => Ok(ArgumentKind::Unknown),
                    Some(arc) => is_or_points_to_secret(proj, state, bv, &arc.read().unwrap()),
                }
            },
            _ => Ok(ArgumentKind::Public),  // for any other type, the `is_secret()` check above was sufficient
        }
    }
}
