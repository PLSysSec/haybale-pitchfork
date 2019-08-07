mod abstractdata;
pub use abstractdata::AbstractData;
mod secret;

use haybale::{size, symex_function, ExecutionManager, State};
use haybale::backend::*;
pub use haybale::Config;
use llvm_ir::*;

/// Is a function "constant-time" in its inputs. That is, does the function ever
/// make branching decisions, or perform address calculations, based on its inputs.
pub fn is_constant_time_in_inputs(func: &Function, module: &Module, config: &Config) -> bool {
    let args = func.parameters.iter().map(|p| AbstractData::Secret { bits: size(&p.ty) });
    is_constant_time(func, module, args, config)
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
pub fn is_constant_time(func: &Function, module: &Module, args: impl Iterator<Item = AbstractData>, config: &Config) -> bool {
    let cfg = z3::Config::new();
    let ctx = z3::Context::new(&cfg);

    let mut em: ExecutionManager<secret::Backend> = symex_function(&ctx, module, func, config);

    // overwrite the default function parameters with values marked to be `Secret`
    for (param, arg) in func.parameters.iter().zip(args) {
        allocate_arg(&ctx, em.mut_state(), &param, arg);
    }

    while em.next().is_some() {
        if em.state().backend_state.borrow().ct_violation_observed() {
            return false;
        }
    }

    // no paths had ct violations
    true
}

fn allocate_arg<'ctx, 'm>(ctx: &'ctx z3::Context, state: &mut State<'ctx, 'm, secret::Backend<'ctx>>, param: &'m function::Parameter, arg: AbstractData) {
    assert_eq!(arg.size(), size(&param.ty));
    match arg {
        AbstractData::Secret { bits } => {
            state.overwrite_latest_version_of_bv(&param.name, secret::BV::Secret(bits as u32));
        },
        AbstractData::PublicNonPointer { bits, value: Some(value) } => {
            state.overwrite_latest_version_of_bv(&param.name, secret::BV::from_u64(ctx, value, bits as u32));
        },
        AbstractData::PublicNonPointer { value: None, .. } => {
            // nothing to do
        },
        AbstractData::PublicPointer(pointee) => {
            let ptr = state.allocate(pointee.size() as u64);
            state.overwrite_latest_version_of_bv(&param.name, ptr.clone());
            initialize_data_in_memory(ctx, state, &ptr, &*pointee);
        },
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
        AbstractData::PublicNonPointer { bits, value: Some(value) } => {
            state.write(&ptr, secret::BV::from_u64(ctx, *value, *bits as u32));
        },
        AbstractData::PublicNonPointer { value: None, .. } => {
            // nothing to do
        },
        AbstractData::PublicPointer(pointee) => {
            let inner_ptr = state.allocate(pointee.size() as u64);
            state.write(&ptr, inner_ptr.clone()); // make `ptr` point to the newly allocated memory
            initialize_data_in_memory(ctx, state, &inner_ptr, &**pointee);
        },
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
