mod secret;

use haybale::{size, symex_function, ExecutionManager};
use llvm_ir::*;
use log::debug;

/// Is a function "constant-time" in its inputs. That is, does the function ever
/// make branching decisions, or perform address calculations, based on its inputs.
///
/// `loop_bound`: maximum number of times to execute any given line of LLVM IR.
/// This bounds both the number of iterations of loops, and also the depth of recursion.
/// For inner loops, this bounds the number of total iterations across all invocations of the loop.
pub fn is_constant_time(func: &Function, module: &Module, loop_bound: usize) -> bool {
    let cfg = z3::Config::new();
    let ctx = z3::Context::new(&cfg);

    let mut em: ExecutionManager<secret::Backend> = symex_function(&ctx, module, func, loop_bound);

    // overwrite the default function parameters with values marked to be `Secret`
    for param in &func.parameters {
        em.mut_state().overwrite_latest_version_of_bv(&param.name, secret::BV::Secret(size(&param.ty) as u32));
    }

    while em.next().is_some() {
        if em.state().backend_state.borrow().ct_violation_observed() {
            return false;
        }
    }

    // no paths had ct violations
    true
}
