use crate::abstractdata::*;
use crate::secret;
use haybale::{layout, Project, State};
use haybale::backend::*;
use haybale::Result;
use llvm_ir::*;
use log::debug;
use std::collections::HashMap;
use std::collections::hash_map::Entry::*;
use std::sync::{Arc, RwLock};

pub struct Context<'a> {
    sd: &'a StructDescriptions,
    namedvals: HashMap<String, secret::BV>,
}

impl<'a> Context<'a> {
    pub fn new(sd: &'a StructDescriptions) -> Self {
        Self {
            sd,
            namedvals: HashMap::new(),
        }
    }
}

/// Returns the `secret::BV` representing the argument. Many callers won't need this, though.
pub fn allocate_arg<'p>(proj: &'p Project, state: &mut State<'p, secret::Backend>, param: &'p function::Parameter, arg: AbstractData, ctx: &mut Context) -> Result<secret::BV> {
    debug!("Allocating function parameter {:?}", &param.name);
    let arg = arg.to_complete(&param.ty, proj, &ctx.sd);
    let arg_size = arg.size_in_bits();
    let param_size = layout::size(&param.ty);
    assert_eq!(arg_size, param_size, "Parameter size mismatch for parameter {:?}: parameter is {} bits but CompleteAbstractData is {} bits", &param.name, param_size, arg_size);
    match arg {
        CompleteAbstractData::Secret { bits } => {
            debug!("Parameter is marked secret");
            let bv = secret::BV::Secret { btor: state.solver.clone(), width: bits as u32, symbol: None };
            state.overwrite_latest_version_of_bv(&param.name, bv.clone());
            Ok(bv)
        },
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::ExactValue(value) } => {
            debug!("Parameter is marked public, equal to {}", value);
            let bv = secret::BV::from_u64(state.solver.clone(), value, bits as u32);
            state.overwrite_latest_version_of_bv(&param.name, bv.clone());
            Ok(bv)
        },
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::Range(min, max) } => {
            debug!("Parameter is marked public, in the range ({}, {}) inclusive", min, max);
            let parambv = state.new_bv_with_name(param.name.clone(), bits as u32).unwrap();
            parambv.ugte(&secret::BV::from_u64(state.solver.clone(), min, bits as u32)).assert()?;
            parambv.ulte(&secret::BV::from_u64(state.solver.clone(), max, bits as u32)).assert()?;
            state.overwrite_latest_version_of_bv(&param.name, parambv.clone());
            Ok(parambv)
        }
        CompleteAbstractData::PublicValue { value: AbstractValue::Unconstrained, .. } => {
            debug!("Parameter is marked public, unconstrained value");
            // nothing to do, just return the BV representing that parameter
            let op = Operand::LocalOperand { name: param.name.clone(), ty: param.ty.clone() };
            state.operand_to_bv(&op)
        },
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::Named { name, value } } => {
            let unwrapped_arg = AbstractData(UnderspecifiedAbstractData::Complete(CompleteAbstractData::PublicValue { bits, value: *value }));
            let bv = allocate_arg(proj, state, param, unwrapped_arg, ctx)?;
            match ctx.namedvals.entry(name.to_owned()) {
                Vacant(v) => {
                    v.insert(bv.clone());
                },
                Occupied(bv_for_name) => {
                    let bv_for_name = bv_for_name.get();
                    let width = bv_for_name.get_width();
                    assert_eq!(width, bits as u32, "AbstractValue::Named {:?}: multiple values with different bitwidths given this name: one with width {} bits, another with width {} bits", name, width, bits);
                    bv._eq(&bv_for_name).assert()?;
                },
            };
            state.overwrite_latest_version_of_bv(&param.name, bv.clone());
            Ok(bv)
        }
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::EqualTo(name) } => {
            match ctx.namedvals.get(&name) {
                None => panic!("AbstractValue::Named {:?} not found", name),
                Some(bv) => {
                    let width = bv.get_width();
                    assert_eq!(width, bits as u32, "AbstractValue::EqualTo {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                    state.overwrite_latest_version_of_bv(&param.name, bv.clone());
                    Ok(bv.clone())
                }
            }
        }
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::SignedLessThan(name) } => {
            match ctx.namedvals.get(&name) {
                None => panic!("AbstractValue::Named {:?} not found", name),
                Some(bv) => {
                    let width = bv.get_width();
                    assert_eq!(width, bits as u32, "AbstractValue::SignedLessThan {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                    let new_bv = secret::BV::new(state.solver.clone(), width, Some(&format!("SignedLessThan{}:", name)));
                    new_bv.slt(&bv).assert()?;
                    state.overwrite_latest_version_of_bv(&param.name, new_bv.clone());
                    Ok(new_bv)
                }
            }
        }
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::SignedGreaterThan(name) } => {
            match ctx.namedvals.get(&name) {
                None => panic!("AbstractValue::Named {:?} not found", name),
                Some(bv) => {
                    let width = bv.get_width();
                    assert_eq!(width, bits as u32, "AbstractValue::SignedGreaterThan {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                    let new_bv = secret::BV::new(state.solver.clone(), width, Some(&format!("SignedGreaterThan:{}", name)));
                    new_bv.sgt(&bv).assert()?;
                    state.overwrite_latest_version_of_bv(&param.name, new_bv.clone());
                    Ok(new_bv)
                }
            }
        }
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::UnsignedLessThan(name) } => {
            match ctx.namedvals.get(&name) {
                None => panic!("AbstractValue::Named {:?} not found", name),
                Some(bv) => {
                    let width = bv.get_width();
                    assert_eq!(width, bits as u32, "AbstractValue::UnsignedLessThan {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                    let new_bv = secret::BV::new(state.solver.clone(), width, Some(&format!("UnsignedLessThan:{}", name)));
                    new_bv.ult(&bv).assert()?;
                    state.overwrite_latest_version_of_bv(&param.name, new_bv.clone());
                    Ok(new_bv)
                }
            }
        }
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::UnsignedGreaterThan(name) } => {
            match ctx.namedvals.get(&name) {
                None => panic!("AbstractValue::Named {:?} not found", name),
                Some(bv) => {
                    let width = bv.get_width();
                    assert_eq!(width, bits as u32, "AbstractValue::UnsignedGreaterThan {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                    let new_bv = secret::BV::new(state.solver.clone(), width, Some(&format!("UnsignedGreaterThan:{}", name)));
                    new_bv.ugt(&bv).assert()?;
                    state.overwrite_latest_version_of_bv(&param.name, new_bv.clone());
                    Ok(new_bv)
                }
            }
        }
        CompleteAbstractData::PublicPointerTo(pointee) => {
            debug!("Parameter is marked as a public pointer");
            let ptr = state.allocate(pointee.size_in_bits() as u64);
            debug!("Allocated the parameter at {:?}", ptr);
            state.overwrite_latest_version_of_bv(&param.name, ptr.clone());
            let pointee_ty = match &param.ty {
                Type::PointerType { pointee_type, .. } => pointee_type,
                ty => panic!("Mismatch for parameter {:?}: CompleteAbstractData specifies a pointer but parameter type is {:?}", &param.name, ty),
            };
            initialize_data_in_memory(proj, state, &ptr, &*pointee, Some(pointee_ty), None, None, ctx)
        },
        CompleteAbstractData::PublicPointerToFunction(funcname) => {
            debug!("Parameter is marked as a public pointer to the function {:?}", funcname);
            match &param.ty {
                Type::PointerType { .. } => {},
                ty => panic!("Mismatch for parameter {:?}: CompleteAbstractData specifies a pointer but parameter type is {:?}", &param.name, ty),
            };
            let ptr = state.get_pointer_to_function(funcname.clone())
                .unwrap_or_else(|| panic!("Failed to find function {:?}", &funcname))
                .clone();
            state.overwrite_latest_version_of_bv(&param.name, ptr.clone());
            Ok(ptr)
        }
        CompleteAbstractData::PublicPointerToHook(funcname) => {
            debug!("Parameter is marked as a public pointer to the active hook for function {:?}", funcname);
            match &param.ty {
                Type::PointerType { .. } => {},
                ty => panic!("Mismatch for parameter {:?}: CompleteAbstractData specifies a pointer but parameter type is {:?}", &param.name, ty),
            };
            let ptr = state.get_pointer_to_function_hook(&funcname)
                .unwrap_or_else(|| panic!("Failed to find hook for function {:?}", &funcname))
                .clone();
            state.overwrite_latest_version_of_bv(&param.name, ptr.clone());
            Ok(ptr)
        }
        CompleteAbstractData::PublicPointerToParent => panic!("Pointer-to-parent is not supported for toplevel parameter; we have no way to know what struct it is contained in"),
        CompleteAbstractData::PublicUnconstrainedPointer => {
            debug!("Parameter is marked as a public unconstrained pointer");
            // nothing to do, just check that the type matches
            match &param.ty {
                Type::PointerType { .. } => {},
                ty => panic!("Mismatch for parameter {:?}: CompleteAbstractData specifies a pointer but parameter type is {:?}", &param.name, ty),
            };
            // return the BV representing the parameter
            let op = Operand::LocalOperand { name: param.name.clone(), ty: param.ty.clone() };
            state.operand_to_bv(&op)
        },
        CompleteAbstractData::Array { .. } => unimplemented!("Array passed by value"),
        CompleteAbstractData::Struct { .. } => unimplemented!("Struct passed by value"),
        CompleteAbstractData::VoidOverride { .. } => unimplemented!("VoidOverride used as an argument directly.  You probably meant to use a pointer to a VoidOverride"),
    }
}

/// Initialize the data in memory at `addr` according to the given `CompleteAbstractData`.
///
/// `ty` should be the type of the pointed-to object, not the type of `addr`.
/// It is used only for type-checking, to ensure that the `CompleteAbstractData` actually matches the intended LLVM type.
/// Setting `ty` to `None` disables this type-checking.
///
/// `cur_struct`: if present, it is a pointer to the struct containing the
/// element we're initializing, as well as the type of that struct.
///
/// `parent`: if present, it is a pointer to the struct containing `cur_struct`,
/// as well as the type of that parent struct.
///
/// Returns a `secret::BV` representing the initialized data. Many callers won't need this, though.
pub fn initialize_data_in_memory(
    proj: &Project,
    state: &mut State<'_, secret::Backend>,
    addr: &secret::BV,
    data: &CompleteAbstractData,
    ty: Option<&Type>,
    cur_struct: Option<(&secret::BV, &Type)>,
    parent: Option<(&secret::BV, &Type)>,
    ctx: &mut Context,
) -> Result<secret::BV> {
    if let Some(Type::ArrayType { num_elements: 1, element_type }) | Some(Type::VectorType { num_elements: 1, element_type }) = ty {
        match data {
            CompleteAbstractData::Array { num_elements: 1, element_type: element_abstractdata } => {
                // both LLVM and CAD type are array-of-one-element.  Unwrap and call recursively
                return initialize_data_in_memory(proj, state, addr, element_abstractdata, Some(element_type), cur_struct, parent, ctx);
            },
            data => {
                // LLVM type is array-of-one-element but CAD type is not.  Unwrap the LLVM type and call recursively
                return initialize_data_in_memory(proj, state, addr, data, Some(element_type), cur_struct, parent, ctx);
            },
        }
    };
    debug!("Initializing data in memory at address {:?}", addr);
    match data {
        CompleteAbstractData::Secret { bits } => {
            debug!("marking {} bits secret at address {:?}", bits, addr);
            let bv = secret::BV::Secret { btor: state.solver.clone(), width: *bits as u32, symbol: None };
            state.write(&addr, bv.clone())?;
            Ok(bv)
        },
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::ExactValue(value) } => {
            debug!("setting the memory contents equal to {}", value);
            if let Some(ty) = ty {
                if *bits != layout::size(ty) {
                    panic!("Size mismatch when initializing data at {:?}: type {:?} is {} bits but CompleteAbstractData is {} bits", addr, ty, layout::size(ty), bits);
                }
            }
            let bv = secret::BV::from_u64(state.solver.clone(), *value, *bits as u32);
            state.write(&addr, bv.clone())?;
            Ok(bv)
        },
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::Range(min, max) } => {
            debug!("constraining the memory contents to be in the range ({}, {}) inclusive", min, max);
            if let Some(ty) = ty {
                if *bits != layout::size(ty) {
                    panic!("Size mismatch when initializing data at {:?}: type {:?} is {} bits but CompleteAbstractData is {} bits", addr, ty, layout::size(ty), bits);
                }
            }
            let bv = state.read(&addr, *bits as u32)?;
            bv.ugte(&secret::BV::from_u64(state.solver.clone(), *min, *bits as u32)).assert()?;
            bv.ulte(&secret::BV::from_u64(state.solver.clone(), *max, *bits as u32)).assert()?;
            Ok(bv)
        }
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::Unconstrained } => {
            debug!("memory contents are indicated as unconstrained");
            // nothing to do, just check that the type matches
            if let Some(ty) = ty {
                if *bits != layout::size(ty) {
                    panic!("Size mismatch when initializing data at {:?}: type {:?} is {} bits but CompleteAbstractData is {} bits", addr, ty, layout::size(ty), bits);
                }
            }
            // return the BV representing those memory contents
            state.read(&addr, *bits as u32)
        },
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::Named { name, value } } => {
            let unwrapped_data = CompleteAbstractData::PublicValue { bits: *bits, value: (**value).clone() };
            let bv = initialize_data_in_memory(proj, state, addr, &unwrapped_data, ty, cur_struct, parent, ctx)?;
            match ctx.namedvals.entry(name.to_owned()) {
                Vacant(v) => {
                    v.insert(bv.clone());
                },
                Occupied(bv_for_name) => {
                    let bv_for_name = bv_for_name.get();
                    let width = bv_for_name.get_width();
                    assert_eq!(width, *bits as u32, "AbstractValue::Named {:?}: multiple values with different bitwidths given this name: one with width {} bits, another with width {} bits", name, width, *bits);
                    bv._eq(&bv_for_name).assert()?;
                },
            };
            Ok(bv)
        },
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::EqualTo(name) } => {
            match ctx.namedvals.get(name) {
                None => panic!("AbstractValue::Named {:?} not found", name),
                Some(bv) => {
                    let width = bv.get_width();
                    assert_eq!(width, *bits as u32, "AbstractValue::EqualTo {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                    if let Some(ty) = ty {
                        if *bits != layout::size(ty) {
                            panic!("Size mismatch when initializing data at {:?}: type {:?} is {} bits but CompleteAbstractData is {} bits", addr, ty, layout::size(ty), bits);
                        }
                    }
                    state.write(&addr, bv.clone())?;
                    Ok(bv.clone())
                }
            }
        }
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::SignedLessThan(name) } => {
            match ctx.namedvals.get(name) {
                None => panic!("AbstractValue::Named {:?} not found", name),
                Some(bv) => {
                    let width = bv.get_width();
                    assert_eq!(width, *bits as u32, "AbstractValue::SignedLessThan {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                    if let Some(ty) = ty {
                        if *bits != layout::size(ty) {
                            panic!("Size mismatch when initializing data at {:?}: type {:?} is {} bits but CompleteAbstractData is {} bits", addr, ty, layout::size(ty), bits);
                        }
                    }
                    let new_bv = secret::BV::new(state.solver.clone(), width, Some(&format!("SignedLessThan:{}", name)));
                    new_bv.slt(&bv).assert()?;
                    state.write(&addr, new_bv.clone())?;
                    Ok(new_bv)
                }
            }
        }
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::SignedGreaterThan(name) } => {
            match ctx.namedvals.get(name) {
                None => panic!("AbstractValue::Named {:?} not found", name),
                Some(bv) => {
                    let width = bv.get_width();
                    assert_eq!(width, *bits as u32, "AbstractValue::SignedGreaterThan {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                    if let Some(ty) = ty {
                        if *bits != layout::size(ty) {
                            panic!("Size mismatch when initializing data at {:?}: type {:?} is {} bits but CompleteAbstractData is {} bits", addr, ty, layout::size(ty), bits);
                        }
                    }
                    let new_bv = secret::BV::new(state.solver.clone(), width, Some(&format!("SignedGreaterThan:{}", name)));
                    new_bv.sgt(&bv).assert()?;
                    state.write(&addr, new_bv.clone())?;
                    Ok(new_bv)
                }
            }
        }
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::UnsignedLessThan(name) } => {
            match ctx.namedvals.get(name) {
                None => panic!("AbstractValue::Named {:?} not found", name),
                Some(bv) => {
                    let width = bv.get_width();
                    assert_eq!(width, *bits as u32, "AbstractValue::UnsignedLessThan {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                    if let Some(ty) = ty {
                        if *bits != layout::size(ty) {
                            panic!("Size mismatch when initializing data at {:?}: type {:?} is {} bits but CompleteAbstractData is {} bits", addr, ty, layout::size(ty), bits);
                        }
                    }
                    let new_bv = secret::BV::new(state.solver.clone(), width, Some(&format!("UnsignedLessThan:{}", name)));
                    new_bv.ult(&bv).assert()?;
                    state.write(&addr, new_bv.clone())?;
                    Ok(new_bv)
                }
            }
        }
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::UnsignedGreaterThan(name) } => {
            match ctx.namedvals.get(name) {
                None => panic!("AbstractValue::Named {:?} not found", name),
                Some(bv) => {
                    let width = bv.get_width();
                    assert_eq!(width, *bits as u32, "AbstractValue::UnsignedGreaterThan {:?}, which has {} bits, but current value has {} bits", name, width, bits);
                    if let Some(ty) = ty {
                        if *bits != layout::size(ty) {
                            panic!("Size mismatch when initializing data at {:?}: type {:?} is {} bits but CompleteAbstractData is {} bits", addr, ty, layout::size(ty), bits);
                        }
                    }
                    let new_bv = secret::BV::new(state.solver.clone(), width, Some(&format!("UnsignedGreaterThan:{}", name)));
                    new_bv.ugt(&bv).assert()?;
                    state.write(&addr, new_bv.clone())?;
                    Ok(new_bv)
                }
            }
        }
        CompleteAbstractData::PublicPointerTo(pointee) => {
            debug!("memory contents are marked as a public pointer");
            let inner_ptr = state.allocate(pointee.size_in_bits() as u64);
            debug!("allocated memory for the pointee at {:?}, and will constrain the memory contents at {:?} to have that pointer value", inner_ptr, addr);
            state.write(&addr, inner_ptr.clone())?; // make `addr` point to a pointer to the newly allocated memory
            let pointee_ty = ty.map(|ty| match ty {
                Type::PointerType { pointee_type, .. } => &**pointee_type,
                _ => panic!("Type mismatch: CompleteAbstractData specifies a pointer, but found type {:?}", ty),
            });
            initialize_data_in_memory(proj, state, &inner_ptr, &**pointee, pointee_ty, cur_struct, parent, ctx)
        },
        CompleteAbstractData::PublicPointerToFunction(funcname) => {
            debug!("memory contents are marked as a public pointer to the function {:?}", funcname);
            if let Some(ty) = ty {
                match ty {
                    Type::PointerType { .. } => {},
                    _ => panic!("Type mismatch: CompleteAbstractData specifies a pointer, but found type {:?}", ty),
                };
            }
            let inner_ptr = state.get_pointer_to_function(funcname.clone())
                .unwrap_or_else(|| panic!("Failed to find function {:?}", &funcname))
                .clone();
            debug!("setting the memory contents equal to {:?}", inner_ptr);
            state.write(&addr, inner_ptr.clone())?; // make `addr` point to a pointer to the function
            Ok(inner_ptr)
        }
        CompleteAbstractData::PublicPointerToHook(funcname) => {
            debug!("memory contents are marked as a public pointer to the active hook for function {:?}", funcname);
            if let Some(ty) = ty {
                match ty {
                    Type::PointerType { .. } => {},
                    _ => panic!("Type mismatch: CompleteAbstractData specifies a pointer, but found type {:?}", ty),
                };
            }
            let inner_ptr = state.get_pointer_to_function_hook(funcname)
                .unwrap_or_else(|| panic!("Failed to find hook for function {:?}", &funcname))
                .clone();
            debug!("setting the memory contents equal to {:?}", inner_ptr);
            state.write(&addr, inner_ptr.clone())?; // make `addr` point to a pointer to the hook
            Ok(inner_ptr)
        }
        CompleteAbstractData::PublicPointerToParent => {
            debug!("memory contents are marked as a public pointer to this struct's parent");
            match parent {
                None => panic!("Pointer-to-parent used but there is no immediate parent"),
                Some((parent_ptr, parent_ty)) => {
                    // first typecheck: is this actually a pointer to the correct parent type
                    match ty {
                        Some(Type::PointerType { pointee_type, .. }) => {
                            let pointee_ty = &**pointee_type;
                            if pointee_ty == parent_ty {
                                // typecheck passes, do nothing
                            } else if let Type::NamedStructType { name, ty } = pointee_ty {
                                // LLVM type is pointer to a named struct type, try unwrapping it and see if that makes the types equal
                                let ty: Arc<RwLock<Type>> = ty
                                    .as_ref()
                                    .expect("CompleteAbstractData specifies pointer-to-parent, but found pointer to an opaque struct type")
                                    .upgrade()
                                    .expect("Failed to upgrade weak reference");
                                let actual_ty: &Type = &ty.read().unwrap();
                                if actual_ty == parent_ty {
                                    // typecheck passes, do nothing
                                } else {
                                    panic!("Type mismatch: CompleteAbstractData specifies pointer-to-parent, but found pointer to a different type.\n  Parent type: {:?}?n  Found type: struct named {:?}: {:?}\n", parent_ty, name, actual_ty);
                                }
                            }
                        },
                        Some(_) => panic!("Type mismatch: CompleteAbstractData specifies a pointer, but found type {:?}", ty),
                        None => {},
                    };
                    // typecheck passed, write the pointer
                    debug!("setting the memory contents equal to {:?}", parent_ptr);
                    state.write(&addr, parent_ptr.clone())?;
                    Ok(parent_ptr.clone())
                },
            }
        }
        CompleteAbstractData::PublicUnconstrainedPointer => {
            debug!("memory contents are marked as a public unconstrained pointer");
            // nothing to do, just check that the type matches
            if let Some(ty) = ty {
                match ty {
                    Type::PointerType { .. } => {},
                    _ => panic!("Type mismatch: CompleteAbstractData specifies a pointer, but found type {:?}", ty),
                };
            }
            // return the BV representing those memory contents
            state.read(&addr, AbstractData::POINTER_SIZE_BITS as u32)
        },
        CompleteAbstractData::Array { element_type: element_abstractdata, num_elements } => {
            debug!("memory contents are marked as an array of {} elements", num_elements);
            let element_type = ty.map(|ty| match ty {
                Type::ArrayType { element_type, num_elements: found_num_elements } => {
                    if *found_num_elements != 0 {
                        assert_eq!(num_elements, found_num_elements, "Type mismatch: CompleteAbstractData specifies an array with {} elements, but found an array with {} elements", num_elements, found_num_elements);
                    } else {
                        // do nothing.  If it is a 0-element array in LLVM, that probably just means an array of unspecified length, so we don't compare with the CompleteAbstractData length
                    }
                    element_type
                },
                _ => ty,  // an array, but the LLVM type is just pointer.  E.g., *int instead of *{array of 16 ints}.
            });
            let element_size_bits = element_abstractdata.size_in_bits();
            if let Some(element_type) = element_type {
                let llvm_element_size_bits = layout::size(element_type);
                if llvm_element_size_bits != 0 {
                    assert_eq!(element_size_bits, llvm_element_size_bits, "CompleteAbstractData element size of {} bits does not match LLVM element size of {} bits", element_size_bits, llvm_element_size_bits);
                }
            }
            match **element_abstractdata {
                CompleteAbstractData::Secret { .. } => {
                    // special-case this, as we can initialize with one big write
                    let array_size_bits = element_size_bits * *num_elements;
                    debug!("initializing the entire array as {} secret bits", array_size_bits);
                    initialize_data_in_memory(proj, state, &addr, &CompleteAbstractData::Secret { bits: array_size_bits }, ty, cur_struct, parent, ctx)
                },
                CompleteAbstractData::PublicValue { bits, value: AbstractValue::Unconstrained } => {
                    // special-case this, as no initialization is necessary for the entire array
                    debug!("array contents are entirely public unconstrained bits");
                    // return the BV representing those memory contents
                    state.read(&addr, bits as u32)
                },
                _ => {
                    // the general case. This would work in all cases, but would be slower than the optimized special-case above
                    if element_size_bits % 8 != 0 {
                        panic!("Array element size is not a multiple of 8 bits: {}", element_size_bits);
                    }
                    let element_size_bytes = element_size_bits / 8;
                    for i in 0 .. *num_elements {
                        debug!("initializing element {} of the array", i);
                        let element_addr = addr.add(&secret::BV::from_u64(state.solver.clone(), (i*element_size_bytes) as u64, addr.get_width()));
                        initialize_data_in_memory(proj, state, &element_addr, element_abstractdata, element_type, cur_struct, parent, ctx)?;
                    }
                    debug!("done initializing the array at {:?}", addr);
                    // return the BV representing those memory contents. We just do a read here because it's easier than collecting the initialize() return values and concatenating them
                    state.read(&addr, (element_size_bits * *num_elements) as u32)
                },
            }
        },
        CompleteAbstractData::Struct { name, elements } => {
            debug!("memory contents are marked as a struct ({})", name);
            let mut cur_addr = addr.clone();
            let element_types = match ty {
                Some(ty) => match ty {
                    Type::StructType { element_types, .. } => element_types.iter().cloned().map(Some).collect::<Vec<_>>(),
                    Type::NamedStructType { ty: None, name: llvm_struct_name } => {
                        // This is an opaque struct definition. Try to find a non-opaque definition for the same struct.
                        let (ty, _) = proj.get_named_struct_type_by_name(&llvm_struct_name).unwrap_or_else(|| panic!("Struct name {:?} (LLVM name {:?}) not found in the project", name, llvm_struct_name));
                        let actual_ty: &Type = &ty.as_ref()
                            .unwrap_or_else(|| panic!("Can't convert struct named {:?} (LLVM name {:?}) to complete: it has only opaque definitions in this project", name, llvm_struct_name))
                            .read()
                            .unwrap();
                        match actual_ty {
                            Type::StructType { element_types, .. } => element_types.iter().cloned().map(Some).collect::<Vec<_>>(),
                            _ => panic!("NamedStructType referred to type {:?} which is not a StructType variant", actual_ty),
                        }
                    },
                    Type::NamedStructType { ty: Some(weak), .. } => {
                        let ty: Arc<RwLock<Type>> = weak.upgrade().expect("Failed to upgrade weak reference");
                        let actual_ty: &Type = &ty.read().unwrap();
                        match actual_ty {
                            Type::StructType { element_types, .. } => element_types.iter().cloned().map(Some).collect::<Vec<_>>(),
                            ty => panic!("NamedStructType referred to type {:?} which is not a StructType variant", ty),
                        }
                    }
                    _ => panic!("Type mismatch: CompleteAbstractData specifies a struct named {}, but found type {:?}", name, ty),
                },
                None => itertools::repeat_n(None, elements.len()).collect(),
            };
            let mut total_bits = 0;
            for (element_idx, (element, element_ty)) in elements.iter().zip(element_types).enumerate() {
                let element_size_bits = element.size_in_bits();
                if let Some(element_ty) = &element_ty {
                    let llvm_element_size_bits = layout::size(&element_ty);
                    if llvm_element_size_bits != 0 {
                        assert_eq!(element_size_bits, llvm_element_size_bits, "CompleteAbstractData element size of {} bits does not match LLVM element size of {} bits", element_size_bits, llvm_element_size_bits);
                    }
                }
                if element_size_bits % 8 != 0 {
                    panic!("Struct element size is not a multiple of 8 bits: {}", element_size_bits);
                }
                total_bits += element_size_bits;
                let element_size_bytes = element_size_bits / 8;
                debug!("initializing element {} of struct {}; element's address is {:?}", element_idx, name, &cur_addr);
                let new_cur_struct = ty.map(|ty| (addr, ty));
                let new_parent = cur_struct;
                initialize_data_in_memory(proj, state, &cur_addr, element, element_ty.as_ref(), new_cur_struct, new_parent, ctx)?;
                cur_addr = cur_addr.add(&secret::BV::from_u64(state.solver.clone(), element_size_bytes as u64, addr.get_width()));
            }
            debug!("done initializing struct {} at {:?}", name, addr);
            // return the BV representing those memory contents. We just do a read here because it's easier than collecting the initialize() return values and concatenating them
            state.read(&addr, total_bits as u32)
        }
        CompleteAbstractData::VoidOverride { llvm_struct_name, data } => {
            // first check that the type we're overriding is `i8`: LLVM seems to use `i8*` when C uses `void*`
            match ty {
                Some(Type::IntegerType { bits: 8 }) => {},
                Some(Type::PointerType { .. }) => panic!("attempt to use VoidOverride to override LLVM type {:?} rather than i8. You may want to use a pointer to a VoidOverride rather than a VoidOverride directly.", ty),
                Some(ty) => panic!("attempt to use VoidOverride to override LLVM type {:?} rather than i8", ty),
                None => {},  // could be a nested VoidOverride, for instance
            }
            match llvm_struct_name {
                None => initialize_data_in_memory(proj, state, addr, &data, None, cur_struct, parent, ctx),
                Some(llvm_struct_name) => {
                    let (llvm_ty, _) = proj.get_named_struct_type_by_name(&llvm_struct_name)
                        .unwrap_or_else(|| panic!("VoidOverride: llvm_struct_name {:?} not found in Project", llvm_struct_name));
                    let arc = llvm_ty.as_ref().unwrap_or_else(|| panic!("VoidOverride: llvm_struct_name {:?} is an opaque type", llvm_struct_name));
                    let llvm_ty: &Type = &arc.read().unwrap();
                    initialize_data_in_memory(proj, state, addr, &data, Some(llvm_ty), cur_struct, parent, ctx)
                },
            }
        },
    }
}
