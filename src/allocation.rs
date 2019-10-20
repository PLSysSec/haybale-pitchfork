use crate::abstractdata::*;
use crate::secret;
use haybale::{layout, State};
use haybale::backend::*;
use haybale::Result;
use llvm_ir::*;
use log::debug;
use std::sync::{Arc, RwLock};

pub fn allocate_arg<'p>(state: &mut State<'p, secret::Backend>, param: &'p function::Parameter, arg: AbstractData) -> Result<()> {
    debug!("Allocating function parameter {:?}", &param.name);
    let arg = arg.to_complete(&param.ty);
    let arg_size = arg.size_in_bits();
    let param_size = layout::size(&param.ty);
    assert_eq!(arg_size, param_size, "Parameter size mismatch for parameter {:?}: parameter is {} bits but CompleteAbstractData is {} bits", &param.name, param_size, arg_size);
    match arg {
        CompleteAbstractData::Secret { bits } => {
            debug!("Parameter is marked secret");
            state.overwrite_latest_version_of_bv(&param.name, secret::BV::Secret { btor: state.solver.clone(), width: bits as u32, symbol: None });
            Ok(())
        },
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::ExactValue(value) } => {
            debug!("Parameter is marked public, equal to {}", value);
            state.overwrite_latest_version_of_bv(&param.name, secret::BV::from_u64(state.solver.clone(), value, bits as u32));
            Ok(())
        },
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::Range(min, max) } => {
            debug!("Parameter is marked public, in the range ({}, {}) inclusive", min, max);
            let parambv = state.new_bv_with_name(param.name.clone(), bits as u32).unwrap();
            parambv.ugte(&secret::BV::from_u64(state.solver.clone(), min, bits as u32)).assert()?;
            parambv.ulte(&secret::BV::from_u64(state.solver.clone(), max, bits as u32)).assert()?;
            state.overwrite_latest_version_of_bv(&param.name, parambv);
            Ok(())
        }
        CompleteAbstractData::PublicValue { value: AbstractValue::Unconstrained, .. } => {
            debug!("Parameter is marked public, unconstrained value");
            // nothing to do
            Ok(())
        },
        CompleteAbstractData::PublicPointerTo(pointee) => {
            debug!("Parameter is marked as a public pointer");
            let ptr = state.allocate(pointee.size_in_bits() as u64);
            debug!("Allocated the parameter at {:?}", ptr);
            state.overwrite_latest_version_of_bv(&param.name, ptr.clone());
            let pointee_ty = match &param.ty {
                Type::PointerType { pointee_type, .. } => pointee_type,
                ty => panic!("Mismatch for parameter {:?}: CompleteAbstractData specifies a pointer but parameter type is {:?}", &param.name, ty),
            };
            initialize_data_in_memory(state, &ptr, &*pointee, pointee_ty)
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
            state.overwrite_latest_version_of_bv(&param.name, ptr);
            Ok(())
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
            state.overwrite_latest_version_of_bv(&param.name, ptr);
            Ok(())
        }
        CompleteAbstractData::PublicPointerToUnconstrainedPublic => {
            debug!("Parameter is marked as a public pointer to unconstrained public data");
            // nothing to do, just check that the type matches
            match &param.ty {
                Type::PointerType { .. } => {},
                ty => panic!("Mismatch for parameter {:?}: CompleteAbstractData specifies a pointer but parameter type is {:?}", &param.name, ty),
            };
            Ok(())
        },
        CompleteAbstractData::Array { .. } => unimplemented!("Array passed by value"),
        CompleteAbstractData::Struct { .. } => unimplemented!("Struct passed by value"),
    }
}

/// Initialize the data in memory at `addr` according to the given `CompleteAbstractData`.
///
/// `ty` should be the type of the pointed-to object, not the type of `addr`.
pub fn initialize_data_in_memory(state: &mut State<'_, secret::Backend>, addr: &secret::BV, data: &CompleteAbstractData, ty: &Type) -> Result<()> {
    debug!("Initializing data in memory at address {:?}", addr);
    match data {
        CompleteAbstractData::Secret { bits } => {
            debug!("marking {} bits secret at address {:?}", bits, addr);
            state.write(&addr, secret::BV::Secret { btor: state.solver.clone(), width: *bits as u32, symbol: None })
        },
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::ExactValue(value) } => {
            debug!("setting the memory contents equal to {}", value);
            if *bits != layout::size(ty) {
                panic!("Size mismatch when initializing data at {:?}: type {:?} is {} bits but CompleteAbstractData is {} bits", addr, ty, layout::size(ty), bits);
            }
            state.write(&addr, secret::BV::from_u64(state.solver.clone(), *value, *bits as u32))
        },
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::Range(min, max) } => {
            debug!("constraining the memory contents to be in the range ({}, {}) inclusive", min, max);
            if *bits != layout::size(ty) {
                panic!("Size mismatch when initializing data at {:?}: type {:?} is {} bits but CompleteAbstractData is {} bits", addr, ty, layout::size(ty), bits);
            }
            let bv = state.read(&addr, *bits as u32)?;
            bv.ugte(&secret::BV::from_u64(state.solver.clone(), *min, *bits as u32)).assert()?;
            bv.ulte(&secret::BV::from_u64(state.solver.clone(), *max, *bits as u32)).assert()?;
            Ok(())
        }
        CompleteAbstractData::PublicValue { bits, value: AbstractValue::Unconstrained } => {
            debug!("memory contents are indicated as unconstrained");
            // nothing to do, just check that the type matches
            if *bits != layout::size(ty) {
                panic!("Size mismatch when initializing data at {:?}: type {:?} is {} bits but CompleteAbstractData is {} bits", addr, ty, layout::size(ty), bits);
            }
            Ok(())
        },
        CompleteAbstractData::PublicPointerTo(pointee) => {
            debug!("memory contents are marked as a public pointer");
            let inner_ptr = state.allocate(pointee.size_in_bits() as u64);
            debug!("allocated memory for the pointee at {:?}, and will constrain the memory contents at {:?} to have that pointer value", inner_ptr, addr);
            state.write(&addr, inner_ptr.clone())?; // make `addr` point to a pointer to the newly allocated memory
            let pointee_ty = match ty {
                Type::PointerType { pointee_type, .. } => pointee_type,
                _ => panic!("Type mismatch: CompleteAbstractData specifies a pointer, but found type {:?}", ty),
            };
            initialize_data_in_memory(state, &inner_ptr, &**pointee, pointee_ty)
        },
        CompleteAbstractData::PublicPointerToFunction(funcname) => {
            debug!("memory contents are marked as a public pointer to the function {:?}", funcname);
            match ty {
                Type::PointerType { .. } => {},
                _ => panic!("Type mismatch: CompleteAbstractData specifies a pointer, but found type {:?}", ty),
            };
            let inner_ptr = state.get_pointer_to_function(funcname.clone())
                .unwrap_or_else(|| panic!("Failed to find function {:?}", &funcname))
                .clone();
            debug!("setting the memory contents equal to {:?}", inner_ptr);
            state.write(&addr, inner_ptr) // make `addr` point to a pointer to the function
        }
        CompleteAbstractData::PublicPointerToHook(funcname) => {
            debug!("memory contents are marked as a public pointer to the active hook for function {:?}", funcname);
            match ty {
                Type::PointerType { .. } => {},
                _ => panic!("Type mismatch: CompleteAbstractData specifies a pointer, but found type {:?}", ty),
            };
            let inner_ptr = state.get_pointer_to_function_hook(funcname)
                .unwrap_or_else(|| panic!("Failed to find hook for function {:?}", &funcname))
                .clone();
            debug!("setting the memory contents equal to {:?}", inner_ptr);
            state.write(&addr, inner_ptr) // make `addr` point to a pointer to the hook
        }
        CompleteAbstractData::PublicPointerToUnconstrainedPublic => {
            debug!("memory contents are marked as a public pointer to unconstrained public data");
            // nothing to do, just check that the type matches
            match ty {
                Type::PointerType { .. } => {},
                _ => panic!("Type mismatch: CompleteAbstractData specifies a pointer, but found type {:?}", ty),
            };
            Ok(())
        },
        CompleteAbstractData::Array { element_type: element_abstractdata, num_elements } => {
            debug!("memory contents are marked as an array of {} elements", num_elements);
            let element_type = match ty {
                Type::ArrayType { element_type, num_elements: found_num_elements } => {
                    if *found_num_elements != 0 {
                        assert_eq!(num_elements, found_num_elements, "Type mismatch: CompleteAbstractData specifies an array with {} elements, but found an array with {} elements", num_elements, found_num_elements);
                    } else {
                        // do nothing.  If it is a 0-element array in LLVM, that probably just means an array of unspecified length, so we don't compare with the CompleteAbstractData length
                    }
                    element_type
                },
                _ => ty,  // an array, but the LLVM type is just pointer.  E.g., *int instead of *{array of 16 ints}.
            };
            let element_size_bits = element_abstractdata.size_in_bits();
            let llvm_element_size_bits = layout::size(element_type);
            if llvm_element_size_bits != 0 {
                assert_eq!(element_size_bits, llvm_element_size_bits, "CompleteAbstractData element size of {} bits does not match LLVM element size of {} bits", element_size_bits, llvm_element_size_bits);
            }
            match **element_abstractdata {
                CompleteAbstractData::Secret { .. } => {
                    // special-case this, as we can initialize with one big write
                    let array_size_bits = element_size_bits * *num_elements;
                    debug!("initializing the entire array as {} secret bits", array_size_bits);
                    initialize_data_in_memory(state, &addr, &CompleteAbstractData::Secret { bits: array_size_bits }, ty)
                },
                CompleteAbstractData::PublicValue { value: AbstractValue::Unconstrained, .. } => {
                    // special-case this, as no initialization is necessary for the entire array
                    debug!("array contents are entirely public unconstrained bits");
                    Ok(())
                },
                _ => {
                    // the general case. This would work in all cases, but would be slower than the optimized special-case above
                    if element_size_bits % 8 != 0 {
                        panic!("Array element size is not a multiple of 8 bits: {}", element_size_bits);
                    }
                    let element_size_bytes = element_size_bits / 8;
                    for i in 0 .. *num_elements {
                        debug!("initializing element {} of the array", i);
                        initialize_data_in_memory(state, &addr.add(&secret::BV::from_u64(state.solver.clone(), (i*element_size_bytes) as u64, addr.get_width())), element_abstractdata, element_type)?;
                    }
                    debug!("done initializing the array at {:?}", addr);
                    Ok(())
                },
            }
        },
        CompleteAbstractData::Struct(elements) => {
            debug!("memory contents are marked as a struct");
            let mut cur_addr = addr.clone();
            let element_types = match ty {
                Type::StructType { element_types, .. } => element_types.clone(),
                Type::NamedStructType { ty: None, .. } => panic!("can't initialize an opaque struct type"),
                Type::NamedStructType { ty: Some(weak), .. } => {
                    let ty: Arc<RwLock<Type>> = weak.upgrade().expect("Failed to upgrade weak reference");
                    let actual_ty: &Type = &ty.read().unwrap();
                    match actual_ty {
                        Type::StructType { element_types, .. } => element_types.clone(),
                        ty => panic!("NamedStructType referred to type {:?} which is not a StructType variant", ty),
                    }
                }
                _ => panic!("Type mismatch: CompleteAbstractData specifies a struct, but found type {:?}", ty),
            };
            for (element_idx, (element, element_ty)) in elements.iter().zip(element_types).enumerate() {
                let element_size_bits = element.size_in_bits();
                let llvm_element_size_bits = layout::size(&element_ty);
                if llvm_element_size_bits != 0 {
                    assert_eq!(element_size_bits, llvm_element_size_bits, "CompleteAbstractData element size of {} bits does not match LLVM element size of {} bits", element_size_bits, llvm_element_size_bits);
                }
                if element_size_bits % 8 != 0 {
                    panic!("Struct element size is not a multiple of 8 bits: {}", element_size_bits);
                }
                let element_size_bytes = element_size_bits / 8;
                debug!("initializing element {} of the struct; element's address is {:?}", element_idx, &cur_addr);
                initialize_data_in_memory(state, &cur_addr, element, &element_ty)?;
                cur_addr = cur_addr.add(&secret::BV::from_u64(state.solver.clone(), element_size_bytes as u64, addr.get_width()));
            }
            debug!("done initializing the struct at {:?}", addr);
            Ok(())
        }
    }
}
