use crate::abstractdata::*;
use crate::secret;
use haybale::{layout, State};
use haybale::backend::*;
use llvm_ir::*;

pub fn allocate_arg<'p>(state: &mut State<'p, secret::Backend>, param: &'p function::Parameter, arg: AbstractData) {
    if arg.size() != layout::size(&param.ty) {
        panic!("Parameter size mismatch for parameter {:?}: parameter is {} bits but AbstractData is {} bits", &param.name, layout::size(&param.ty), arg.size());
    }
    match arg {
        AbstractData::Secret { bits } => {
            state.overwrite_latest_version_of_bv(&param.name, secret::BV::Secret { btor: state.solver.clone(), width: bits as u32, symbol: None });
        },
        AbstractData::PublicValue { bits, value: AbstractValue::ExactValue(value) } => {
            state.overwrite_latest_version_of_bv(&param.name, secret::BV::from_u64(state.solver.clone(), value, bits as u32));
        },
        AbstractData::PublicValue { bits, value: AbstractValue::Range(min, max) } => {
            let parambv = state.new_bv_with_name(param.name.clone(), bits as u32).unwrap();
            parambv.ugte(&secret::BV::from_u64(state.solver.clone(), min, bits as u32)).assert();
            parambv.ulte(&secret::BV::from_u64(state.solver.clone(), max, bits as u32)).assert();
            state.overwrite_latest_version_of_bv(&param.name, parambv);
        }
        AbstractData::PublicValue { value: AbstractValue::Unconstrained, .. } => {
            // nothing to do
        },
        AbstractData::PublicPointerTo(pointee) => {
            let ptr = state.allocate(pointee.size() as u64);
            state.overwrite_latest_version_of_bv(&param.name, ptr.clone());
            initialize_data_in_memory(state, &ptr, &*pointee);
        },
        AbstractData::PublicPointerToFunction(funcname) => {
            let ptr = state.get_pointer_to_function(funcname.clone())
                .unwrap_or_else(|| panic!("Failed to find function {:?}", &funcname))
                .clone();
            state.overwrite_latest_version_of_bv(&param.name, ptr);
        }
        AbstractData::PublicPointerToHook(funcname) => {
            let ptr = state.get_pointer_to_function_hook(&funcname)
                .unwrap_or_else(|| panic!("Failed to find hook for function {:?}", &funcname))
                .clone();
            state.overwrite_latest_version_of_bv(&param.name, ptr);
        }
        AbstractData::PublicPointerToUnconstrainedPublic => {
            // nothing to do
        },
        AbstractData::Array { .. } => unimplemented!("Array passed by value"),
        AbstractData::Struct { .. } => unimplemented!("Struct passed by value"),
    }
}

pub fn initialize_data_in_memory(state: &mut State<'_, secret::Backend>, ptr: &secret::BV, arg: &AbstractData) {
    match arg {
        AbstractData::Secret { bits } => {
            state.write(&ptr, secret::BV::Secret { btor: state.solver.clone(), width: *bits as u32, symbol: None });
        },
        AbstractData::PublicValue { bits, value: AbstractValue::ExactValue(value) } => {
            state.write(&ptr, secret::BV::from_u64(state.solver.clone(), *value, *bits as u32));
        },
        AbstractData::PublicValue { bits, value: AbstractValue::Range(min, max) } => {
            let bv = state.read(&ptr, *bits as u32);
            bv.ugte(&secret::BV::from_u64(state.solver.clone(), *min, *bits as u32)).assert();
            bv.ulte(&secret::BV::from_u64(state.solver.clone(), *max, *bits as u32)).assert();
        }
        AbstractData::PublicValue { value: AbstractValue::Unconstrained, .. } => {
            // nothing to do
        },
        AbstractData::PublicPointerTo(pointee) => {
            let inner_ptr = state.allocate(pointee.size() as u64);
            state.write(&ptr, inner_ptr.clone()); // make `ptr` point to a pointer to the newly allocated memory
            initialize_data_in_memory(state, &inner_ptr, &**pointee);
        },
        AbstractData::PublicPointerToFunction(funcname) => {
            let inner_ptr = state.get_pointer_to_function(funcname.clone())
                .unwrap_or_else(|| panic!("Failed to find function {:?}", &funcname))
                .clone();
            state.write(&ptr, inner_ptr); // make `ptr` point to a pointer to the function
        }
        AbstractData::PublicPointerToHook(funcname) => {
            let inner_ptr = state.get_pointer_to_function_hook(funcname)
                .unwrap_or_else(|| panic!("Failed to find hook for function {:?}", &funcname))
                .clone();
            state.write(&ptr, inner_ptr); // make `ptr` point to a pointer to the hook
        }
        AbstractData::PublicPointerToUnconstrainedPublic => {
            // nothing to do
        },
        AbstractData::Array { element_type, num_elements } => {
            let element_size_bits = element_type.size();
            match **element_type {
                AbstractData::Secret { .. } => {
                    // special-case this, as we can initialize with one big write
                    let array_size_bits = element_size_bits * *num_elements;
                    initialize_data_in_memory(state, &ptr, &AbstractData::Secret { bits: array_size_bits });
                },
                _ => {
                    // the general case. This would work in all cases, but would be slower than the optimized special-case above
                    if element_size_bits % 8 != 0 {
                        panic!("Array element size is not a multiple of 8 bits: {}", element_size_bits);
                    }
                    let element_size_bytes = element_size_bits / 8;
                    for i in 0 .. *num_elements {
                        initialize_data_in_memory(state, &ptr.add(&secret::BV::from_u64(state.solver.clone(), (i*element_size_bytes) as u64, ptr.get_width())), element_type);
                    }
                },
            }
        },
        AbstractData::Struct(elements) => {
            let mut cur_ptr = ptr.clone();
            for element in elements {
                let element_size_bits = element.size();
                if element_size_bits % 8 != 0 {
                    panic!("Struct element size is not a multiple of 8 bits: {}", element_size_bits);
                }
                let element_size_bytes = element_size_bits / 8;
                initialize_data_in_memory(state, &cur_ptr, element);
                cur_ptr = cur_ptr.add(&secret::BV::from_u64(state.solver.clone(), element_size_bytes as u64, ptr.get_width()));
            }
        }
    }
}
