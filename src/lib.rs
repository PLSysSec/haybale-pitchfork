mod abstractdata;
pub use abstractdata::*;
pub mod allocation;
pub mod secret;

use haybale::{layout, symex_function, backend::Backend, ExecutionManager, State, ReturnValue};
use haybale::{Error, Result};
pub use haybale::{Config, Project};
use llvm_ir::instruction;
use log::{debug, info};

/// Is a function "constant-time" in its inputs. That is, does the function ever
/// make branching decisions, or perform address calculations, based on its inputs.
///
/// For argument descriptions, see `haybale::symex_function()`.
pub fn is_constant_time_in_inputs<'p>(
    funcname: &str,
    project: &'p Project,
    config: Config<'p, secret::Backend>
) -> bool {
    let (func, _) = project.get_func_by_name(funcname).expect("Failed to find function");
    let args = func.parameters.iter().map(|p| AbstractData::sec_integer(layout::size(&p.ty)));
    is_constant_time(funcname, project, args, &StructDescriptions::new(), config)
}

/// Is a function "constant-time" in the secrets identified by the `args` data
/// structure. That is, does the function ever make branching decisions, or
/// perform address calculations, based on secrets.
///
/// `args`: for each function parameter, an `AbstractData` describing whether the
/// parameter is secret data itself, public data, a public pointer to secret data
/// (and if so how much), etc; or `AbstractData::default()` to use the default
/// based on the LLVM parameter type and/or the struct descriptions in `sd`.
///
/// Other arguments are the same as for `is_constant_time_in_inputs()` above.
pub fn is_constant_time<'p>(
    funcname: &str,
    project: &'p Project,
    args: impl IntoIterator<Item = AbstractData>,
    sd: &StructDescriptions,
    config: Config<'p, secret::Backend>
) -> bool {
    check_for_ct_violation(funcname, project, args, sd, config).is_none()
}

/// Checks whether a function is "constant-time" in the secrets identified by the
/// `args` data structure. That is, does the function ever make branching
/// decisions, or perform address calculations, based on secrets.
///
/// `args`: for each function parameter, an `AbstractData` describing whether the
/// parameter is secret data itself, public data, a public pointer to secret data
/// (and if so how much), etc; or `AbstractData::default()` to use the default
/// based on the LLVM parameter type and/or the struct descriptions in `sd`.
///
/// Other arguments are the same as for `is_constant_time_in_inputs()` above.
///
/// If the function is constant-time, this returns `None`. Otherwise, it returns
/// a `String` describing the violation. (If there is more than one violation,
/// this will simply return the first violation it finds.)
pub fn check_for_ct_violation<'p>(
    funcname: &str,
    project: &'p Project,
    args: impl IntoIterator<Item = AbstractData>,
    sd: &StructDescriptions,
    mut config: Config<'p, secret::Backend>
) -> Option<String> {
    if !config.function_hooks.is_hooked("hook_uninitialized_function_pointer") {
        config.function_hooks.add("hook_uninitialized_function_pointer", &hook_uninitialized_function_pointer);
    }

    info!("Checking function {:?} for ct violations", funcname);
    let mut em: ExecutionManager<secret::Backend> = symex_function(funcname, project, config);

    debug!("Allocating memory for function parameters");
    let params = em.state().cur_loc.func.parameters.iter();
    for (param, arg) in params.zip(args.into_iter()) {
        allocation::allocate_arg(em.mut_state(), &param, arg, sd).unwrap();
    }
    debug!("Done allocating memory for function parameters");

    for path_result in em {
        match path_result {
            Ok(_) => info!("Finished a path"),
            Err(s) => return Some(s),
        }
    }

    // If we reach this point, then no paths had ct violations
    info!("Done checking function {:?}; no ct violations found", funcname);
    None
}

fn hook_uninitialized_function_pointer<B: Backend>(
    _state: &mut State<B>,
    _call: &instruction::Call,
) -> Result<ReturnValue<B::BV>> {
    Err(Error::OtherError("Call of an uninitialized function pointer".to_owned()))
}
