mod abstractdata;
pub use abstractdata::*;
mod allocation;
mod coverage;
use coverage::*;
pub mod hook_helpers;
pub mod secret;

use colored::*;
use haybale::{layout, symex_function, backend::Backend, ExecutionManager, State, ReturnValue};
use haybale::{Error, Result};
pub use haybale::{Config, Project};
use haybale::function_hooks::IsCall;
use lazy_static::lazy_static;
use log::{debug, info};
use std::collections::{HashMap, HashSet};
use std::fmt;

pub enum ConstantTimeResultForPath {
    IsConstantTime,
    NotConstantTime {
        /// A `String` describing the violation found on this path.
        violation_message: String,
    },
    OtherError {
        /// The `Error` encountered on this path.
        error: Error,
        /// The full error message with "rich context" (backtrace, full path, etc)
        full_message: String,
    },
}

pub struct ConstantTimeResultForFunction<'a> {
    /// Name of the toplevel function we analyzed
    pub funcname: &'a str,
    /// Mangled name of the toplevel function we analyzed
    /// (this may be the same as `funcname`, e.g. for C code)
    mangled_funcname: &'a str,
    /// the `ConstantTimeResultForPath`s for each path in that function.
    /// Note that since we can't progress beyond a `NotConstantTime` or
    /// `OtherError` result on a particular path, there may be many more paths
    /// than the ones listed here.
    /// We simply have no way of knowing how many more paths there might be
    /// beyond one of these errors.
    pub path_results: Vec<ConstantTimeResultForPath>,
    /// Map from function names to statistics on the block coverage of those
    /// functions. Functions not appearing in the map were not encountered on
    /// any path, or were hooked.
    ///
    /// Note that in the case of `ConstantTimeResultForPath::NotConstantTime` or
    /// `ConstantTimeResultForPath::OtherError`, the coverage stats consider the
    /// block in which the error occurred to be covered, even if the portion of
    /// the block after where the error occurred was not covered.
    pub block_coverage: HashMap<String, BlockCoverage>,
}

impl<'a> ConstantTimeResultForFunction<'a> {
    /// Return the `violation_message` for the first `NotConstantTime` result
    /// encountered, if there is one.
    pub fn first_ct_violation(&self) -> Option<&str> {
        self.path_results.iter().find_map(|path_result| match path_result {
            ConstantTimeResultForPath::IsConstantTime => None,
            ConstantTimeResultForPath::NotConstantTime { violation_message } => Some(violation_message as &str),
            ConstantTimeResultForPath::OtherError { .. } => None,
        })
    }

    /// Return the first `NotConstantTime` or `OtherError` result encountered,
    /// if there is one.
    pub fn first_error_or_violation(&self) -> Option<&ConstantTimeResultForPath> {
        self.path_results.iter().find(|path_result| match path_result {
            ConstantTimeResultForPath::IsConstantTime => false,
            ConstantTimeResultForPath::NotConstantTime { .. } => true,
            ConstantTimeResultForPath::OtherError { .. } => true,
        })
    }

    pub fn path_statistics(&self) -> PathStatistics {
        let mut path_stats = PathStatistics {
            num_ct_paths: 0,
            num_ct_violations: 0,
            num_unsats: 0,
            num_loop_bound_exceeded: 0,
            num_null_ptr_deref: 0,
            num_function_not_found: 0,
            num_solver_errors: 0,
            num_unsupported_instruction: 0,
            num_malformed_instruction: 0,
            num_other_errors: 0,
        };
        for result in &self.path_results {
            match result {
                ConstantTimeResultForPath::IsConstantTime => path_stats.num_ct_paths += 1,
                ConstantTimeResultForPath::NotConstantTime { .. } => path_stats.num_ct_violations += 1,
                ConstantTimeResultForPath::OtherError { error: Error::Unsat, .. } => path_stats.num_unsats += 1,
                ConstantTimeResultForPath::OtherError { error: Error::LoopBoundExceeded, .. } => path_stats.num_loop_bound_exceeded += 1,
                ConstantTimeResultForPath::OtherError { error: Error::NullPointerDereference, .. } => path_stats.num_null_ptr_deref += 1,
                ConstantTimeResultForPath::OtherError { error: Error::FunctionNotFound(_), .. } => path_stats.num_function_not_found += 1,
                ConstantTimeResultForPath::OtherError { error: Error::SolverError(_), .. } => path_stats.num_solver_errors += 1,
                ConstantTimeResultForPath::OtherError { error: Error::UnsupportedInstruction(_), .. } => path_stats.num_unsupported_instruction += 1,
                ConstantTimeResultForPath::OtherError { error: Error::MalformedInstruction(_), .. } => path_stats.num_malformed_instruction += 1,
                ConstantTimeResultForPath::OtherError { error: Error::OtherError(_), .. } => path_stats.num_other_errors += 1,
            }
        }
        path_stats
    }
}

pub struct PathStatistics {
    /// How many paths "passed", that is, had no error or constant-time violation
    pub num_ct_paths: usize,
    /// How many constant-time violations did we find
    pub num_ct_violations: usize,
    /// How many Unsat errors did we find
    pub num_unsats: usize,
    /// How many LoopBoundExceeded errors did we find
    pub num_loop_bound_exceeded: usize,
    /// How many NullPointerDereference errors did we find
    pub num_null_ptr_deref: usize,
    /// How many FunctionNotFound errors did we find
    pub num_function_not_found: usize,
    /// How many solver errors (including timeouts) did we find
    pub num_solver_errors: usize,
    /// How many UnsupportedInstruction errors did we find
    pub num_unsupported_instruction: usize,
    /// How many MalformedInstruction errors did we find
    pub num_malformed_instruction: usize,
    /// How many other errors (including solver timeouts) did we encounter
    pub num_other_errors: usize,
}

/// Produces a pretty (even colored!) description of the
/// `ConstantTimeResultForFunction`, including selected coverage statistics
impl<'a> fmt::Display for ConstantTimeResultForFunction<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "\nResults for {}:\n", self.funcname)?;

        if self.path_results.is_empty() {
            writeln!(f, "No valid paths were found and no errors or violations were encountered")?;
            return Ok(());
        }

        let path_stats = self.path_statistics();

        // We always show "verified paths" and "constant-time violations found"
        writeln!(f, "verified paths: {}",
            if path_stats.num_ct_paths > 0 {
                path_stats.num_ct_paths.to_string().green()
            } else {
                path_stats.num_ct_paths.to_string().normal()
            }
        )?;
        writeln!(f, "constant-time violations found: {}",
            if path_stats.num_ct_violations > 0 {
                path_stats.num_ct_violations.to_string().red()
            } else {
                path_stats.num_ct_violations.to_string().normal()
            }
        )?;

        // For the other error types, we only show the entry if it's > 0
        if path_stats.num_null_ptr_deref > 0 {
            writeln!(f, "null-pointer dereferences found: {}",
                path_stats.num_null_ptr_deref.to_string().red()
            )?;
        }
        if path_stats.num_function_not_found > 0 {
            writeln!(f, "function-not-found errors: {}",
                path_stats.num_function_not_found.to_string().red()
            )?;
        }
        if path_stats.num_unsupported_instruction > 0 {
            writeln!(f, "unsupported-instruction errors: {}",
                path_stats.num_unsupported_instruction.to_string().red()
            )?;
        }
        if path_stats.num_malformed_instruction > 0 {
            writeln!(f, "malformed-instruction errors: {}",
                path_stats.num_malformed_instruction.to_string().red()
            )?;
        }
        if path_stats.num_unsats > 0 {
            writeln!(f, "unsat errors: {}",
                path_stats.num_unsats.to_string().red()
            )?;
        }
        if path_stats.num_loop_bound_exceeded > 0 {
            writeln!(f, "paths exceeding the loop bound: {}",
                path_stats.num_loop_bound_exceeded.to_string().red()
            )?;
        }
        if path_stats.num_solver_errors > 0 {
            writeln!(f, "solver errors, including timeouts: {}",
                path_stats.num_solver_errors.to_string().red()
            )?;
        }
        if path_stats.num_other_errors > 0 {
            writeln!(f, "other errors: {}",
                path_stats.num_other_errors.to_string().red()
            )?;
        }
        writeln!(f)?;

        // is the function entirely verified (no CT violations or other errors)?
        let is_ct = self.path_results.len() == path_stats.num_ct_paths;

        let show_coverage_stats = is_ct || match std::env::var("PITCHFORK_COVERAGE_STATS") {
            Ok(val) if val == "1" => true,
            _ => false,
        };
        if show_coverage_stats {
            writeln!(f, "Coverage stats:\n")?;
            let toplevel_coverage = self.block_coverage.get(self.mangled_funcname).unwrap();
            writeln!(f, "  Block coverage of toplevel function ({}): {:.1}%", self.funcname, 100.0 * toplevel_coverage.percentage)?;
            if toplevel_coverage.percentage < 1.0 {
                writeln!(f, "  Missed blocks in toplevel function: {:?}", toplevel_coverage.missed_blocks.iter())?;
            }
            writeln!(f)?;
            for (fname, coverage) in &self.block_coverage {
                if fname != self.mangled_funcname {
                    writeln!(f, "  Block coverage of {}: {:.1}%", fname, 100.0 * coverage.percentage)?;
                }
            }
        } else {
            writeln!(f, "(for detailed block-coverage stats, rerun with PITCHFORK_COVERAGE_STATS=1 environment variable.)")?;
        }
        writeln!(f)?;

        if path_stats.num_ct_violations > 0 {
            match self.first_ct_violation() {
                None => panic!("we counted a ct violation, but now can't find one"),
                Some(violation_message) => {
                    writeln!(f, "{} {}", self.funcname, "is not constant-time".red())?;
                    writeln!(f, "First constant-time violation encountered:\n\n{}", violation_message)?;
                },
            }
        } else if !is_ct {
            match self.first_error_or_violation() {
                None => panic!("we counted a non-ct path, but now can't find one"),
                Some(ConstantTimeResultForPath::IsConstantTime) => panic!("first_error_or_violation shouldn't return an IsConstantTime"),
                Some(ConstantTimeResultForPath::NotConstantTime { .. }) => panic!("we counted no ct violations, but now somehow found one"),
                Some(ConstantTimeResultForPath::OtherError { full_message, .. }) => {
                    writeln!(f, "First error encountered:\n\n{}", full_message)?;
                },
            }
        } else {
            writeln!(f, "{} {}", self.funcname, "is constant-time".green())?;
        }

        Ok(())
    }
}

/// Checks whether a function is "constant-time" in its inputs. That is, does the
/// function ever make branching decisions, or perform address calculations, based
/// on its inputs.
///
/// `keep_going`: if `true`, then even if we encounter an error or violation, we
/// will continue exploring as many paths as we can in the function before
/// returning, possibly reporting many different errors and/or violations.
/// (Although we can't keep going on the errored path itself, we can still try to
/// explore other paths that don't contain the error.)
/// If `false`, then as soon as we encounter an error or violation, we will quit
/// and return the results we have.
/// It is recommended to only use `keep_going == true` in conjunction with solver
/// query timeouts; see the `solver_query_timeout` setting in `Config`.
///
/// Other arguments are the same as for
/// [`haybale::symex_function()`](https://PLSysSec.github.io/haybale/haybale/fn.symex_function.html).
pub fn check_for_ct_violation_in_inputs<'p>(
    funcname: &'p str,
    project: &'p Project,
    config: Config<'p, secret::Backend>,
    keep_going: bool,
) -> ConstantTimeResultForFunction<'p> {
    lazy_static! {
        static ref BLANK_STRUCT_DESCRIPTIONS: StructDescriptions = StructDescriptions::new();
    }

    let (func, _) = project.get_func_by_name(funcname).expect("Failed to find function");
    let args = func.parameters.iter().map(|p| AbstractData::sec_integer(layout::size(&p.ty)));
    check_for_ct_violation(funcname, project, args, &BLANK_STRUCT_DESCRIPTIONS, config, keep_going)
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
/// `sd`: a mapping of LLVM struct names to `AbstractData` descriptions of those
/// structs. These will be used whenever a struct of the appropriate type is
/// found while processing an `AbstractData::default()`; for more details, see
/// [docs on `AbstractData::default()`](struct.AbstractData.html#method.default).
///
/// `keep_going`: see the description of the `keep_going` argument to
/// [`check_for_ct_violation_in_inputs()`](fn.check_for_ct_violation_in_inputs.html).
///
/// Other arguments are the same as for
/// [`haybale::symex_function()`](https://PLSysSec.github.io/haybale/haybale/fn.symex_function.html).
pub fn check_for_ct_violation<'p>(
    funcname: &'p str,
    project: &'p Project,
    args: impl IntoIterator<Item = AbstractData>,
    sd: &StructDescriptions,
    mut config: Config<'p, secret::Backend>,
    keep_going: bool,
) -> ConstantTimeResultForFunction<'p> {
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
    let mangled_funcname = {
        let (func, _) = project.get_func_by_name(funcname).unwrap();
        &func.name
    };
    let mut path_results = Vec::new();
    loop {
        match em.next() {
            Some(Ok(_)) => {
                info!("Finished a path with no errors or violations");
                blocks_seen.update_with_current_path(&em);
                path_results.push(ConstantTimeResultForPath::IsConstantTime);
            },
            Some(Err(error)) => {
                blocks_seen.update_with_current_path(&em);
                let mut full_message = em.state().full_error_message_with_context(error.clone());
                if full_message.contains("RUST_LOG=haybale") {
                    // add our own Pitchfork-specific logging advice
                    full_message.push_str("note: for pitchfork-related issues, you might try `RUST_LOG=info,haybale_pitchfork,haybale`.");
                }
                if full_message.contains("Constant-time violation:") {
                    info!("Found a constant-time violation on this path");
                    path_results.push(ConstantTimeResultForPath::NotConstantTime { violation_message: full_message });
                    if !keep_going {
                        break;
                    }
                } else {
                    info!("Encountered an error (other than a constant-time violation) on this path: {}", error);
                    path_results.push(ConstantTimeResultForPath::OtherError { error, full_message });
                    if !keep_going {
                        break;
                    }
                }
            },
            None => break,
        }
    }

    let block_coverage = blocks_seen.full_coverage_stats();
    info!("Block coverage of toplevel function ({:?}): {:.1}%", funcname, 100.0 * block_coverage.get(mangled_funcname).unwrap().percentage);

    ConstantTimeResultForFunction {
        funcname,
        mangled_funcname,
        path_results,
        block_coverage,
    }
}

fn hook_uninitialized_function_pointer<B: Backend>(
    _proj: &Project,
    _state: &mut State<B>,
    _call: &dyn IsCall,
) -> Result<ReturnValue<B::BV>> {
    Err(Error::OtherError("Call of an uninitialized function pointer".to_owned()))
}
