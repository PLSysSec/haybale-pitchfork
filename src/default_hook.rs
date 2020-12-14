use crate::secret;
use either::Either;
use haybale::{Error, Result, ReturnValue, State};
use haybale::backend::BV;
use haybale::function_hooks::IsCall;
use llvm_ir::{Constant, Name, Operand, Type};
use llvm_ir::types::NamedStructDef;
use log::info;

pub fn pitchfork_default_hook(
    state: &mut State<secret::Backend>,
    call: &dyn IsCall,
) -> Result<ReturnValue<secret::BV>> {
    let called_funcname = match call.get_called_func() {
        Either::Left(_) => panic!("invoked default hook for an inline assembly call"),  // this shouldn't happen
        Either::Right(Operand::ConstantOperand(cref)) => match cref.as_ref() {
            Constant::GlobalReference { name: Name::Name(name), .. } => Some(name),
            Constant::GlobalReference { name, .. } => panic!("Function with a numbered name: {:?}", name),
            _ => None,  // some constant function pointer apparently
        },
        Either::Right(_) => None,  // a function pointer
    };
    let pretty_funcname = match called_funcname {
        Some(funcname) => format!("a function named {:?}", state.demangle(funcname)),
        None => "a function pointer".into(),
    };
    info!("Using Pitchfork default hook for {}", pretty_funcname);

    for (i, arg) in call.get_arguments().iter().map(|(arg, _)| arg).enumerate() {
        // if the arg is secret, or points to any secret data, then raise an error and require a manually-specified hook or LLVM definition
        let arg_bv = state.operand_to_bv(arg)?;
        match is_or_points_to_secret(state, &arg_bv, &state.type_of(arg))? {
            ArgumentKind::Secret => match called_funcname {
                Some(funcname) => {
                    let demangled = state.demangle(funcname);
                    return Err(Error::OtherError(format!("Encountered a call of {}, but didn't find an LLVM definition or function hook for it; and its argument #{} (zero-indexed) may refer to secret data.\nTo fix this error, you can do one of these three options:\n  (1) choose to simply ignore this function call by adding it to `config.function_hooks` with `haybale::function_hooks::generic_stub_hook` as the hook;\n  (2) rerun with more bitcode files in the `Project` so that the symbolic execution can find an LLVM definition for {:?};\n  (3) write your own custom hook for {:?}", pretty_funcname, i, demangled, demangled)));
                },
                None => {
                    // TODO: does this situation actually ever come up? What error message / hints are appropriate?
                    return Err(Error::OtherError(format!("pitchfork_default_hook on a function pointer, and argument #{} (zero-indexed) may refer to secret data", i)));
                },
            },
            ArgumentKind::Unknown => match called_funcname {
                Some(funcname) => {
                    let demangled = state.demangle(funcname);
                    return Err(Error::OtherError(format!("Encountered a call of {}, but didn't find an LLVM definition or function hook for it; and its argument #{} (zero-indexed) involves an opaque struct type, so we're not sure if it may contain secret data.\nTo fix this error, you can do one of these three options:\n  (1) choose to simply ignore this function call by adding it to `config.function_hooks` with `haybale::function_hooks::generic_stub_hook` as the hook;\n  (2) rerun with more bitcode files in the `Project` so that the symbolic execution can find an LLVM definition for {:?};\n  (3) write your own custom hook for {:?}", pretty_funcname, i, demangled, demangled)));
                },
                None => {
                    // TODO: does this situation actually ever come up? What error message / hints are appropriate?
                    return Err(Error::OtherError(format!("pitchfork_default_hook on a function pointer, and argument #{} (zero-indexed) involves an opaque struct type, so we're not sure if it may contain secret data", i)));
                },
            },
            ArgumentKind::Public => {},
        }
    }

    // if we get here, no secret data is being handled by this function, so we just default to generic_stub_hook
    haybale::function_hooks::generic_stub_hook(state, call)
}

#[derive(Clone, Debug)]
pub(crate) enum ArgumentKind {
    /// The argument is fully public, and (if it's a pointer or contains pointer(s)) any pointed-to data is also public
    Public,
    /// The argument is secret, or it's a pointer or contains pointer(s) and some pointed-to data is secret
    Secret,
    /// Couldn't fully analyze the argument because it points to, or contains pointer(s) to, an opaque struct type
    Unknown,
}

/// Classifies the `bv` into an `ArgumentKind` - see notes on `ArgumentKind`
pub(crate) fn is_or_points_to_secret(state: &mut State<secret::Backend>, bv: &secret::BV, ty: &llvm_ir::Type) -> Result<ArgumentKind> {
    if bv.is_secret() {
        Ok(ArgumentKind::Secret)
    } else {
        match ty {
            Type::PointerType { pointee_type, .. } => {
                // also check if it points to any secret data
                if let Type::FuncType { .. } = &**pointee_type {
                    // function pointers don't point to secret data
                    return Ok(ArgumentKind::Public);
                }
                let pointee_size_bits = match state.size_in_bits(&pointee_type) {
                    None => return Ok(ArgumentKind::Unknown),
                    Some(size) => size,
                };
                let mut need_pop = false;
                if state.bvs_can_be_equal(&bv, &state.zero(bv.get_width()))? {
                    // If the pointer is NULL then it clearly doesn't point to secret.
                    // So we only need to investigate the case where it's not NULL.
                    // We also need to temporarily constrain it to be not-NULL in order
                    // to avoid a null dereference when reading the pointed-to data
                    state.solver.push(1);
                    need_pop = true;
                    state.assert(&bv._ne(&state.zero(bv.get_width())))?;
                }
                let pointee = match state.read(&bv, pointee_size_bits as u32) {
                    Ok(pointee) => pointee,
                    Err(e) => {
                        if need_pop {
                            state.solver.pop(1);
                        }
                        return Err(e);
                    },
                };
                let retval = is_or_points_to_secret(state, &pointee, &**pointee_type);
                if need_pop {
                    state.solver.pop(1);
                }
                retval
            },
            Type::VectorType { element_type, num_elements, .. } | Type::ArrayType { element_type, num_elements } => {
                // TODO: this could be made more efficient
                let element_bits = match state.size_in_bits(&element_type) {
                    None => return Ok(ArgumentKind::Unknown),
                    Some(size) => size,
                };
                if element_bits == 0 {
                    Ok(ArgumentKind::Public)  // Elements of size 0 bits can't contain secret information
                } else {
                    let mut retval = ArgumentKind::Public;
                    for i in 0 .. *num_elements {
                        let i = i as u32;
                        let element = bv.slice((i+1) * element_bits - 1, i * element_bits);
                        match is_or_points_to_secret(state, &element, &**element_type)? {
                            ArgumentKind::Secret => return Ok(ArgumentKind::Secret),  // we're done, there's definitely a Secret
                            ArgumentKind::Unknown => retval = ArgumentKind::Unknown,  // keep going, maybe we'll find a Secret later
                            ArgumentKind::Public => {},  // leave in place the previous retval
                        }
                    }
                    Ok(retval)  // this will be Unknown if we ever encountered an Unknown, or Public if everything came back Public
                }
            },
            Type::StructType { element_types, .. } => {
                let mut offset_bits = 0;
                let mut retval = ArgumentKind::Public;
                for element_ty in element_types {
                    let element_bits = match state.size_in_bits(element_ty) {
                        None => return Ok(ArgumentKind::Unknown),  // we have no way to keep going - we don't know the next offset
                        Some(size) => size,
                    };
                    if element_bits == 0 {
                        // nothing to do.  An element of size 0 bits can't contain secret information, and we don't need to update the current offset
                    } else {
                        let element = bv.slice(offset_bits + element_bits - 1, offset_bits);
                        match is_or_points_to_secret(state, &element, element_ty)? {
                            ArgumentKind::Secret => return Ok(ArgumentKind::Secret),  // we're done, there's definitely a Secret
                            ArgumentKind::Unknown => retval = ArgumentKind::Unknown,  // keep going, maybe we'll find a Secret later
                            ArgumentKind::Public => {},  // leave in place the previous retval
                        }
                        offset_bits += element_bits;
                        assert_eq!(offset_bits % 8, 0, "Struct offset of {} bits is not a multiple of 8 bits", offset_bits);
                    }
                }
                Ok(retval)  // this will be Unknown if we ever encountered an Unknown, or Public if everything came back Public
            },
            Type::NamedStructType { name } => {
                match state.proj.get_named_struct_def(name)? {
                    (NamedStructDef::Opaque, _) => Ok(ArgumentKind::Unknown),
                    (NamedStructDef::Defined(ty), _) => is_or_points_to_secret(state, bv, &ty),
                }
            },
            _ => Ok(ArgumentKind::Public),  // for any other type, the `is_secret()` check above was sufficient
        }
    }
}
