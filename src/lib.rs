mod abstractdata;
pub use abstractdata::*;
pub mod allocation;
pub mod secret;
use secret::CTViolation;

use haybale::{layout, symex_function, ExecutionManager};
pub use haybale::{Config, Project};
use log::debug;

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
    let args = func.parameters.iter().map(|p| AbstractData::Secret { bits: layout::size(&p.ty) });
    is_constant_time(funcname, project, args, config)
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
pub fn is_constant_time<'p>(
    funcname: &str,
    project: &'p Project,
    args: impl IntoIterator<Item = AbstractData>,
    config: Config<'p, secret::Backend>
) -> bool {
    check_for_ct_violation(funcname, project, args, config).is_none()
}

/// Checks whether a function is "constant-time" in the secrets identified by the
/// `args` data structure. That is, does the function ever make branching
/// decisions, or perform address calculations, based on secrets.
///
/// If the function is constant-time, this returns `None`. Otherwise, it returns
/// a `CTViolation` describing the violation. (If there is more than one
/// violation, this will simply return the first violation it finds.)
fn check_for_ct_violation<'p>(
    funcname: &str,
    project: &'p Project,
    args: impl IntoIterator<Item = AbstractData>,
    config: Config<'p, secret::Backend>
) -> Option<CTViolation> {
    let mut em: ExecutionManager<secret::Backend> = symex_function(funcname, project, config);

    debug!("Allocating memory for function parameters");
    let params = em.state().cur_loc.func.parameters.iter();
    for (param, arg) in params.zip(args.into_iter()) {
        debug!("Allocating function parameter {:?}", param);
        allocation::allocate_arg(em.mut_state(), &param, arg);
    }
    debug!("Done allocating memory for function parameters");

    while em.next().is_some() {
        let violation = em.state().solver.ct_violation();
        if violation.is_some() {
            debug!("Discovered a violation: {:?}", violation);
            return violation;
        }
    }

    // no paths had ct violations
    return None;
}
