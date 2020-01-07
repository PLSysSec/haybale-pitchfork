mod abstractdata;
pub use abstractdata::*;
pub mod allocation;
mod coverage;
use coverage::*;
pub mod hook_helpers;
pub mod secret;

use colored::*;
use haybale::{layout, symex_function, backend::Backend, ExecutionManager, State, ReturnValue};
use haybale::{Error, Result};
pub use haybale::{Config, Project};
use haybale::function_hooks::IsCall;
use log::{debug, info};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Is a function "constant-time" in its inputs. That is, does the function ever
/// make branching decisions, or perform address calculations, based on its inputs.
///
/// For argument descriptions, see `haybale::symex_function()`.
pub fn is_constant_time_in_inputs<'p>(
    funcname: &str,
    project: &'p Project,
    config: Config<'p, secret::Backend>
) -> bool {
    match check_for_ct_violation_in_inputs(funcname, project, config).ct_result {
        ConstantTimeResult::IsConstantTime { .. } => true,
        _ => false,
    }
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
    match check_for_ct_violation(funcname, project, args, sd, config).ct_result {
        ConstantTimeResult::IsConstantTime { .. } => true,
        _ => false,
    }
}

pub enum ConstantTimeResult {
    IsConstantTime {
        /// Map from function names to statistics on the block coverage of those
        /// functions. Functions not appearing in the map were not encountered on
        /// any path, or were hooked.
        block_coverage: HashMap<String, BlockCoverage>,
    },
    NotConstantTime {
        /// A `String` describing the violation. (If there is more than one
        /// violation, this will simply be the first violation found.)
        violation_message: String,
    },
    OtherError {
        /// A `String` describing the error
        error_message: String,
    },
}

pub struct ConstantTimeResultForFunction<'a> {
    /// a `ConstantTimeResult` for which toplevel function?
    pub funcname: &'a str,
    /// the `ConstantTimeResult` for that function
    pub ct_result: ConstantTimeResult,
}

/// Produces a pretty (even colored!) description of the
/// `ConstantTimeResultForFunction`, including selected coverage statistics in
/// the success case
impl<'a> fmt::Display for ConstantTimeResultForFunction<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f)?;
        match &self.ct_result {
            ConstantTimeResult::IsConstantTime { block_coverage } => {
                writeln!(f, "{} {}", self.funcname, "is constant-time".green())?;
                writeln!(f)?;
                let toplevel_coverage = block_coverage.get(self.funcname).unwrap();
                writeln!(f, "  Block coverage of toplevel function ({}): {:.1}%", self.funcname, 100.0 * toplevel_coverage.percentage)?;
                if toplevel_coverage.percentage < 1.0 {
                    writeln!(f, "  Missed blocks in toplevel function: {:?}", toplevel_coverage.missed_blocks.iter())?;
                }
                writeln!(f)?;
                for (fname, coverage) in block_coverage {
                    if fname != self.funcname {
                        writeln!(f, "  Block coverage of {}: {:.1}%", fname, 100.0 * coverage.percentage)?;
                    }
                }
            },
            ConstantTimeResult::NotConstantTime { violation_message } => {
                writeln!(f, "{} {}", self.funcname, "is not constant-time".red())?;
                writeln!(f, "{}", violation_message)?;
            },
            ConstantTimeResult::OtherError { error_message } => {
                writeln!(f, "While analyzing {}, {}", self.funcname, "received a fatal error:".red())?;
                writeln!(f, "{}", error_message)?;
            },
        }
        Ok(())
    }
}

/// Checks whether a function is "constant-time" in its inputs. That is, does the
/// function ever make branching decisions, or perform address calculations, based
/// on its inputs.
///
/// For argument descriptions, see `is_constant_time_in_inputs()`.
pub fn check_for_ct_violation_in_inputs<'f, 'p>(
    funcname: &'f str,
    project: &'p Project,
    config: Config<'p, secret::Backend>
) -> ConstantTimeResultForFunction<'f> {
    let (func, _) = project.get_func_by_name(funcname).expect("Failed to find function");
    let args = func.parameters.iter().map(|p| AbstractData::sec_integer(layout::size(&p.ty)));
    check_for_ct_violation(funcname, project, args, &StructDescriptions::new(), config)
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
pub fn check_for_ct_violation<'f, 'p>(
    funcname: &'f str,
    project: &'p Project,
    args: impl IntoIterator<Item = AbstractData>,
    sd: &StructDescriptions,
    mut config: Config<'p, secret::Backend>,
) -> ConstantTimeResultForFunction<'f> {
    if !config.function_hooks.is_hooked("hook_uninitialized_function_pointer") {
        config.function_hooks.add("hook_uninitialized_function_pointer", &hook_uninitialized_function_pointer);
    }

    // first sanity-check the StructDescriptions, ensure that all its struct names are valid
    let sd_names: HashSet<_> = sd.iter().map(|(name, _)| name).collect();
    let proj_names: HashSet<_> = project.all_named_struct_types().map(|(name, _, _)| name).collect();
    for name in sd_names.difference(&proj_names) {
        panic!("Struct name {:?} appears in StructDescriptions but not found in the Project", name);
    }

    info!("Checking function {:?} for ct violations", funcname);
    let mut em: ExecutionManager<secret::Backend> = symex_function(funcname, project, config);

    info!("Allocating memory for function parameters");
    let params = em.state().cur_loc.func.parameters.iter();
    allocation::allocate_args(project, em.mut_state(), sd, params.zip(args.into_iter())).unwrap();
    debug!("Done allocating memory for function parameters");

    let mut blocks_seen = BlocksSeen::new();
    loop {
        match em.next() {
            Some(Ok(_)) => {
                info!("Finished a path");
                blocks_seen.update_with_current_path(&em);
            },
            Some(Err(s)) if s.contains("Constant-time violation:") => return ConstantTimeResultForFunction {
                funcname,
                ct_result: ConstantTimeResult::NotConstantTime { violation_message: s },
            },
            Some(Err(s)) => return ConstantTimeResultForFunction {
                funcname,
                ct_result: ConstantTimeResult::OtherError { error_message: s },
            },
            None => break,
        }
    }

    // If we reach this point, then no paths had ct violations
    info!("Done checking function {:?}; no ct violations found", funcname);

    let block_coverage = compute_coverage_stats(project, &blocks_seen);
    info!("Block coverage of toplevel function ({:?}): {:.1}%", funcname, 100.0 * block_coverage.get(funcname).unwrap().percentage);

    ConstantTimeResultForFunction {
        funcname,
        ct_result: ConstantTimeResult::IsConstantTime { block_coverage },
    }
}

fn hook_uninitialized_function_pointer<B: Backend>(
    _proj: &Project,
    _state: &mut State<B>,
    _call: &dyn IsCall,
) -> Result<ReturnValue<B::BV>> {
    Err(Error::OtherError("Call of an uninitialized function pointer".to_owned()))
}
