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
use lazy_static::lazy_static;
use log::{debug, info};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Is a function "constant-time" in its inputs. That is, does the function ever
/// make branching decisions, or perform address calculations, based on its inputs.
///
/// For argument descriptions, see `haybale::symex_function()`.
pub fn is_constant_time_in_inputs<'p>(
    funcname: &'p str,
    project: &'p Project,
    config: Config<'p, secret::Backend>
) -> bool {
    check_for_ct_violation_in_inputs(funcname, project, config, false)
        .first_error_or_violation()
        .is_none()
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
    funcname: &'p str,
    project: &'p Project,
    args: impl IntoIterator<Item = AbstractData>,
    sd: &StructDescriptions,
    config: Config<'p, secret::Backend>
) -> bool {
    check_for_ct_violation(funcname, project, args, sd, config, false)
        .first_error_or_violation()
        .is_none()
}

pub enum ConstantTimeResultForPath {
    IsConstantTime,
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

    /// Return the `error_message` for the first `OtherError` result
    /// encountered, if there is one.
    pub fn first_other_error(&self) -> Option<&str> {
        self.path_results.iter().find_map(|path_result| match path_result {
            ConstantTimeResultForPath::IsConstantTime => None,
            ConstantTimeResultForPath::NotConstantTime { .. } => None,
            ConstantTimeResultForPath::OtherError { error_message } => Some(error_message as &str),
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
        let mut num_ct_paths = 0;
        let mut num_ct_violations = 0;
        let mut num_other_errors = 0;
        for result in &self.path_results {
            match result {
                ConstantTimeResultForPath::IsConstantTime => num_ct_paths += 1,
                ConstantTimeResultForPath::NotConstantTime { .. } => num_ct_violations += 1,
                ConstantTimeResultForPath::OtherError { .. } => num_other_errors += 1,
            }
        }
        PathStatistics { num_ct_paths, num_ct_violations, num_other_errors }
    }
}

pub struct PathStatistics {
    /// How many paths "passed", that is, had no error or constant-time violation
    pub num_ct_paths: usize,
    /// How many constant-time violations did we find
    pub num_ct_violations: usize,
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
        writeln!(f, "other errors, including solver timeouts: {}",
            if path_stats.num_other_errors > 0 {
                path_stats.num_other_errors.to_string().red()
            } else {
                path_stats.num_other_errors.to_string().normal()
            }
        )?;
        writeln!(f)?;

        match std::env::var("PITCHFORK_COVERAGE_STATS") {
            Ok(val) if val == "1" => {
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
            },
            _ => {
                writeln!(f, "(for detailed block-coverage stats, rerun with PITCHFORK_COVERAGE_STATS=1 environment variable.)")?;
            },
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
        } else if path_stats.num_other_errors > 0 {
            match self.first_other_error() {
                None => panic!("we counted an other-error, but now can't find one"),
                Some(error_message) => {
                    writeln!(f, "First error encountered:\n\n{}", error_message)?;
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
/// Other arguments are the same as for `is_constant_time_in_inputs()`.
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
/// `keep_going`: see the description of the `keep_going` argument to
/// `check_for_ct_violation_in_inputs()`.
///
/// Other arguments are the same as for `is_constant_time()`.
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
            Some(Err(mut s)) => {
                blocks_seen.update_with_current_path(&em);
                if s.contains("RUST_LOG=haybale") {
                    // add our own Pitchfork-specific logging advice
                    s.push_str("note: for pitchfork-related issues, you might try `RUST_LOG=info,pitchfork,haybale`.");
                }
                if s.contains("Constant-time violation:") {
                    info!("Found a constant-time violation on this path");
                    path_results.push(ConstantTimeResultForPath::NotConstantTime { violation_message: s });
                    if !keep_going {
                        break;
                    }
                } else {
                    info!("Encountered an error (other than a constant-time violation) on this path");
                    path_results.push(ConstantTimeResultForPath::OtherError { error_message: s });
                    if !keep_going {
                        break;
                    }
                }
            },
            None => break,
        }
    }

    let block_coverage = compute_coverage_stats(project, &blocks_seen);
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
