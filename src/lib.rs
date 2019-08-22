mod abstractdata;
pub use abstractdata::*;
pub mod secret;

use haybale::{layout, symex_function, ExecutionManager, State};
use haybale::backend::*;
pub use haybale::{Config, Project};
use llvm_ir::*;
use log::debug;

/// Is a function "constant-time" in its inputs. That is, does the function ever
/// make branching decisions, or perform address calculations, based on its inputs.
///
/// For argument descriptions, see `haybale::symex_function()`.
pub fn is_constant_time_in_inputs<'ctx, 'p>(
    ctx: &'ctx z3::Context,
    funcname: &str,
    project: &'p Project,
    config: Config<'ctx, secret::Backend<'ctx>>
) -> bool {
    let (func, _) = project.get_func_by_name(funcname).expect("Failed to find function");
    let args = func.parameters.iter().map(|p| AbstractData::Secret { bits: layout::size(&p.ty) });
    is_constant_time(ctx, funcname, project, args, config)
}

/// Is a function "constant-time" in the secrets identified by the `args` data
/// structure. That is, does the function ever make branching decisions, or
/// perform address calculations, based on secrets.
///
/// `args`: for each function parameter, an `AbstractData` describing whether the
/// parameter is secret data itself, public data, a public pointer to secret data
/// (and if so how much), etc.
///
/// Other arguments are the same as for `is_constant_time_in_inputs()` above.
pub fn is_constant_time<'ctx, 'p>(
    ctx: &'ctx z3::Context,
    funcname: &str,
    project: &'p Project,
    args: impl IntoIterator<Item = AbstractData>,
    config: Config<'ctx, secret::Backend<'ctx>>
) -> bool {
    let mut em: ExecutionManager<secret::Backend> = symex_function(&ctx, funcname, project, config);

    debug!("Allocating memory for function parameters");
    let params = em.state().cur_loc.func.parameters.iter();
    for (param, arg) in params.zip(args.into_iter()) {
        allocate_arg(&ctx, em.mut_state(), &param, arg);
    }
    debug!("Done allocating memory for function parameters");

    while em.next().is_some() {
        if em.state().backend_state.borrow().ct_violation_observed() {
            return false;
        }
    }

    // no paths had ct violations
    true
}

fn allocate_arg<'ctx, 'p>(ctx: &'ctx z3::Context, state: &mut State<'ctx, 'p, secret::Backend<'ctx>>, param: &'p function::Parameter, arg: AbstractData) {
    if arg.size() != layout::size(&param.ty) {
        panic!("Parameter size mismatch for parameter {:?}: parameter is {} bits but AbstractData is {} bits", &param.name, layout::size(&param.ty), arg.size());
    }
    match arg {
        AbstractData::Secret { bits } => {
            state.overwrite_latest_version_of_bv(&param.name, secret::BV::Secret(bits as u32));
        },
        AbstractData::PublicValue { bits, value: AbstractValue::ExactValue(value) } => {
            state.overwrite_latest_version_of_bv(&param.name, secret::BV::from_u64(ctx, value, bits as u32));
        },
        AbstractData::PublicValue { bits, value: AbstractValue::Range(min, max) } => {
            let parambv = state.operand_to_bv(&Operand::LocalOperand { name: param.name.clone(), ty: param.ty.clone() }).unwrap();
            state.assert(&parambv.uge(&secret::BV::from_u64(ctx, min, bits as u32)));
            state.assert(&parambv.ule(&secret::BV::from_u64(ctx, max, bits as u32)));
        }
        AbstractData::PublicValue { value: AbstractValue::Unconstrained, .. } => {
            // nothing to do
        },
        AbstractData::PublicPointerTo(pointee) => {
            let ptr = state.allocate(pointee.size() as u64);
            state.overwrite_latest_version_of_bv(&param.name, ptr.clone());
            initialize_data_in_memory(ctx, state, &ptr, &*pointee);
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

fn initialize_data_in_memory<'ctx>(ctx: &'ctx z3::Context, state: &mut State<'ctx, '_, secret::Backend<'ctx>>, ptr: &secret::BV<'ctx>, arg: &AbstractData) {
    match arg {
        AbstractData::Secret { bits } => {
            state.write(&ptr, secret::BV::Secret(*bits as u32));
        },
        AbstractData::PublicValue { bits, value: AbstractValue::ExactValue(value) } => {
            state.write(&ptr, secret::BV::from_u64(ctx, *value, *bits as u32));
        },
        AbstractData::PublicValue { bits, value: AbstractValue::Range(min, max) } => {
            let bv = state.read(&ptr, *bits as u32);
            state.assert(&bv.uge(&secret::BV::from_u64(ctx, *min, *bits as u32)));
            state.assert(&bv.ule(&secret::BV::from_u64(ctx, *max, *bits as u32)));
        }
        AbstractData::PublicValue { value: AbstractValue::Unconstrained, .. } => {
            // nothing to do
        },
        AbstractData::PublicPointerTo(pointee) => {
            let inner_ptr = state.allocate(pointee.size() as u64);
            state.write(&ptr, inner_ptr.clone()); // make `ptr` point to a pointer to the newly allocated memory
            initialize_data_in_memory(ctx, state, &inner_ptr, &**pointee);
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
            if element_size_bits % 8 != 0 {
                panic!("Array element size is not a multiple of 8 bits: {}", element_size_bits);
            }
            let element_size_bytes = element_size_bits / 8;
            for i in 0 .. *num_elements {
                // TODO: this could be done more efficiently for certain `element_type`s
                initialize_data_in_memory(ctx, state, &ptr.add(&secret::BV::from_u64(ctx, (i*element_size_bytes) as u64, ptr.get_size())), element_type);
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
                initialize_data_in_memory(ctx, state, &cur_ptr, element);
                cur_ptr = cur_ptr.add(&secret::BV::from_u64(ctx, element_size_bytes as u64, ptr.get_size()));
            }
        }
    }
}
