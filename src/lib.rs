mod abstractdata;
pub use abstractdata::*;
mod allocation;
mod coverage;
use coverage::*;
mod default_hook;
use default_hook::pitchfork_default_hook;
pub mod hooks;
pub mod hook_helpers;
pub mod secret;
mod logging;
mod progress;

use colored::*;
use haybale::{layout, symex_function, backend::Backend, ExecutionManager, State, ReturnValue};
use haybale::{Error, Result};
pub use haybale::{Config, Project};
use haybale::function_hooks::IsCall;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Holds information about the results of a constant-time analysis of a single
/// path.
#[derive(Clone, Debug)]
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

/// Holds information about the results of a constant-time analysis of a
/// particular function.
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
    /// If we logged all the detailed error messages, then this is the name of
    /// the file they were logged to.
    /// Otherwise, if this is `None`, we did not log the detailed error messages.
    /// (In either case, all the detailed error messages are available in the
    /// `path_results` field above.)
    pub error_filename: Option<String>,
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
        let mut path_stats = PathStatistics::new();
        for result in &self.path_results {
            path_stats.add_path_result(result);
        }
        path_stats
    }
}

/// Some statistics which can be computed from a
/// [`ConstantTimeResultForFunction`](struct.ConstantTimeResultForFunction.html).
#[derive(PartialEq, Eq, Clone, Debug)]
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

impl PathStatistics {
    /// A fresh `PathStatistics` with all zeroes
    pub(crate) fn new() -> Self {
        Self {
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
        }
    }

    pub(crate) fn add_path_result(&mut self, path_result: &ConstantTimeResultForPath) {
        match path_result {
            ConstantTimeResultForPath::IsConstantTime => self.num_ct_paths += 1,
            ConstantTimeResultForPath::NotConstantTime { .. } => self.num_ct_violations += 1,
            ConstantTimeResultForPath::OtherError { error: Error::Unsat, .. } => self.num_unsats += 1,
            ConstantTimeResultForPath::OtherError { error: Error::LoopBoundExceeded(_), .. } => self.num_loop_bound_exceeded += 1,
            ConstantTimeResultForPath::OtherError { error: Error::NullPointerDereference, .. } => self.num_null_ptr_deref += 1,
            ConstantTimeResultForPath::OtherError { error: Error::FunctionNotFound(_), .. } => self.num_function_not_found += 1,
            ConstantTimeResultForPath::OtherError { error: Error::SolverError(_), .. } => self.num_solver_errors += 1,
            ConstantTimeResultForPath::OtherError { error: Error::UnsupportedInstruction(_), .. } => self.num_unsupported_instruction += 1,
            ConstantTimeResultForPath::OtherError { error: Error::MalformedInstruction(_), .. } => self.num_malformed_instruction += 1,
            ConstantTimeResultForPath::OtherError { error: Error::OtherError(_), .. } => self.num_other_errors += 1,
        }
    }
}

impl fmt::Display for PathStatistics {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // We always show "verified paths" and "constant-time violations found"
        writeln!(f, "verified paths: {}",
            if self.num_ct_paths > 0 {
                self.num_ct_paths.to_string().green()
            } else {
                self.num_ct_paths.to_string().normal()
            }
        )?;
        writeln!(f, "constant-time violations found: {}",
            if self.num_ct_violations > 0 {
                self.num_ct_violations.to_string().red()
            } else {
                self.num_ct_violations.to_string().normal()
            }
        )?;

        // For the other error types, we only show the entry if it's > 0
        if self.num_null_ptr_deref > 0 {
            writeln!(f, "null-pointer dereferences found: {}",
                self.num_null_ptr_deref.to_string().red()
            )?;
        }
        if self.num_function_not_found > 0 {
            writeln!(f, "function-not-found errors: {}",
                self.num_function_not_found.to_string().red()
            )?;
        }
        if self.num_unsupported_instruction > 0 {
            writeln!(f, "unsupported-instruction errors: {}",
                self.num_unsupported_instruction.to_string().red()
            )?;
        }
        if self.num_malformed_instruction > 0 {
            writeln!(f, "malformed-instruction errors: {}",
                self.num_malformed_instruction.to_string().red()
            )?;
        }
        if self.num_unsats > 0 {
            writeln!(f, "unsat errors: {}",
                self.num_unsats.to_string().red()
            )?;
        }
        if self.num_loop_bound_exceeded > 0 {
            writeln!(f, "paths exceeding the loop bound: {}",
                self.num_loop_bound_exceeded.to_string().red()
            )?;
        }
        if self.num_solver_errors > 0 {
            writeln!(f, "solver errors, including timeouts: {}",
                self.num_solver_errors.to_string().red()
            )?;
        }
        if self.num_other_errors > 0 {
            writeln!(f, "other errors: {}",
                self.num_other_errors.to_string().red()
            )?;
        }
        Ok(())
    }
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
        path_stats.fmt(f)?;
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
                    if let Some(filename) = &self.error_filename {
                        writeln!(f, "All errors have been logged to {}", filename)?;
                        writeln!(f, "  and the first constant-time violation is described below:\n\n{}", violation_message)?;
                    } else {
                        writeln!(f, "First constant-time violation encountered:\n\n{}", violation_message)?;
                    }
                },
            }
        } else if !is_ct {
            match self.first_error_or_violation() {
                None => panic!("we counted a non-ct path, but now can't find one"),
                Some(ConstantTimeResultForPath::IsConstantTime) => panic!("first_error_or_violation shouldn't return an IsConstantTime"),
                Some(ConstantTimeResultForPath::NotConstantTime { .. }) => panic!("we counted no ct violations, but now somehow found one"),
                Some(ConstantTimeResultForPath::OtherError { full_message, .. }) => {
                    if let Some(filename) = &self.error_filename {
                        writeln!(f, "All errors have been logged to {}", filename)?;
                        writeln!(f, "  and the first error encountered is described below:\n\n{}", full_message)?;
                    } else {
                        writeln!(f, "First error encountered:\n\n{}", full_message)?;
                    }
                },
            }
        } else {
            writeln!(f, "{} {}", self.funcname, "is constant-time".green())?;
        }

        Ok(())
    }
}

/// `pitchfork`-specific configuration options, in addition to the configuration
/// options in `haybale::Config`.
///
/// Like `haybale::Config`, `PitchforkConfig` uses the (new-to-Rust-1.40)
/// `#[non_exhaustive]` attribute to indicate that fields may be added even in a
/// point release (that is, without incrementing the major or minor version).
/// Users should start with `PitchforkConfig::default()` and then change the
/// settings they want to change.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PitchforkConfig {
    /// If `true`, then even if we encounter an error or violation, we will
    /// continue exploring as many paths as we can in the function before
    /// returning, possibly reporting many different errors and/or violations.
    /// (Although we can't keep going on the errored path itself, we can still try to
    /// explore other paths that don't contain the error.)
    /// If `false`, then as soon as we encounter an error or violation, we will quit
    /// and return the results we have.
    /// It is recommended to only use `keep_going == true` in conjunction with solver
    /// query timeouts; see the `solver_query_timeout` setting in `Config`.
    ///
    /// Default is `false`.
    pub keep_going: bool,

    /// Even if `keep_going` is set to `true`, the `Display` impl for
    /// `ConstantTimeResultForFunction` only displays a summary of the kinds of
    /// errors encountered, and full details about a single error.
    /// With `dump_errors == true`, `pitchfork` will dump detailed descriptions
    /// of all errors encountered to a file.
    ///
    /// This setting only applies if `keep_going == true`; it is completely ignored
    /// if `keep_going == false`.
    ///
    /// Default is `true`, meaning that if `keep_going` is enabled, then detailed
    /// error descriptions will be dumped to a file.
    pub dump_errors: bool,

    /// If `true`, `pitchfork` will provide detailed progress updates in a
    /// continuously-updated terminal display. This includes counts of paths
    /// verified / errors encountered / warnings generated; the current code
    /// location being executed (in terms of both LLVM and source if available),
    /// the most recent log message generated, etc.
    ///
    /// Also, if `true`, log messages other than the most recent will not be
    /// shown in the terminal; instead, all log messages will be routed to a
    /// file.
    ///
    /// `progress_updates == true` requires `pitchfork` to take control of the
    /// global logger; users should not initialize their own logging backends
    /// such as `env_logger`.
    /// On the other hand, if `progress_updates == false`, `pitchfork` will not
    /// touch the global logger, and it is up to users to initialize a logging
    /// backend such as `env_logger` if they want to see log messages.
    ///
    /// This setting requires the `progress-updates` crate feature, which is
    /// enabled by default. If the `progress-updates` feature is disabled, this
    /// setting will be treated as `false` regardless of its actual value.
    ///
    /// If you encounter a Rust panic (as opposed to merely a `haybale::Error`),
    /// you may want to temporarily disable `progress_updates` for debugging, in
    /// order to get a clear panic message; otherwise, the
    /// progress-display-updater thread may interfere with the printing of the
    /// panic message.
    ///
    /// Default is `true`.
    pub progress_updates: bool,

    /// If `progress_updates == true`, `pitchfork` takes control of the global
    /// logger, as noted in docs there.
    /// This setting controls which log messages will be recorded in the
    /// designated log file: messages with `DEBUG` and higher priority (`true`),
    /// or only messages with `INFO` and higher priority (`false`).
    ///
    /// If `progress_updates == false`, this setting has no effect; you should
    /// configure debug logging via your own chosen logging backend such as
    /// `env_logger`.
    ///
    /// Default is `false`.
    pub debug_logging: bool,
}

impl Default for PitchforkConfig {
    fn default() -> Self {
        Self {
            keep_going: false,
            dump_errors: true,
            progress_updates: true,
            debug_logging: false,
        }
    }
}

/// Checks whether a function is "constant-time" in its inputs. That is, does the
/// function ever make branching decisions, or perform address calculations, based
/// on its inputs.
///
/// `pitchfork_config`: see [docs on `PitchforkConfig`](struct.PitchforkConfig.html).
///
/// Other arguments are the same as for
/// [`haybale::symex_function()`](https://PLSysSec.github.io/haybale/haybale/fn.symex_function.html).
pub fn check_for_ct_violation_in_inputs<'p>(
    funcname: &'p str,
    project: &'p Project,
    config: Config<'p, secret::Backend>,
    pitchfork_config: &PitchforkConfig,
) -> ConstantTimeResultForFunction<'p> {
    lazy_static! {
        static ref BLANK_STRUCT_DESCRIPTIONS: StructDescriptions = StructDescriptions::new();
    }

    let (func, _) = project.get_func_by_name(funcname).expect("Failed to find function");
    let args = func.parameters.iter().map(|p| AbstractData::sec_integer(layout::size(&p.ty))).collect();
    check_for_ct_violation(funcname, project, Some(args), &BLANK_STRUCT_DESCRIPTIONS, config, pitchfork_config)
}

/// Checks whether a function is "constant-time" in the secrets identified by the
/// `args` data structure. That is, does the function ever make branching
/// decisions, or perform address calculations, based on secrets.
///
/// `args`: for each function parameter, an `AbstractData` describing whether the
/// parameter is secret data itself, public data, a public pointer to secret data
/// (and if so how much), etc; or `AbstractData::default()` to use the default
/// based on the LLVM parameter type and/or the struct descriptions in `sd`.
/// Specifying `None` for `args` is equivalent to supplying a `Vec` with only
/// `AbstractData::default()`s.
///
/// `sd`: a mapping of LLVM struct names to `AbstractData` descriptions of those
/// structs. These will be used whenever a struct of the appropriate type is
/// found while processing an `AbstractData::default()`; for more details, see
/// [docs on `AbstractData::default()`](struct.AbstractData.html#method.default).
///
/// `pitchfork_config`: see [docs on `PitchforkConfig`](struct.PitchforkConfig.html).
///
/// Other arguments are the same as for
/// [`haybale::symex_function()`](https://PLSysSec.github.io/haybale/haybale/fn.symex_function.html).
pub fn check_for_ct_violation<'p>(
    funcname: &'p str,
    project: &'p Project,
    args: Option<Vec<AbstractData>>,
    sd: &StructDescriptions,
    mut config: Config<'p, secret::Backend>,
    pitchfork_config: &PitchforkConfig,
) -> ConstantTimeResultForFunction<'p> {
    // add our uninitialized-function-pointer hook, but don't override the user
    // if they provided a different uninitialized-function-pointer hook
    if !config.function_hooks.is_hooked("hook_uninitialized_function_pointer") {
        config.function_hooks.add("hook_uninitialized_function_pointer", &hook_uninitialized_function_pointer);
    }

    // insert the `pitchfork_default_hook` as the default function hook, but
    // don't override the user if they provided a different default function hook
    if !config.function_hooks.has_default_hook() {
        config.function_hooks.add_default_hook(&pitchfork_default_hook);
    }

    let (log_filename, error_filename) = {
        use chrono::prelude::Local;
        let time = Local::now().format("%Y-%m-%d_%H:%M:%S").to_string();
        let log_filename = format!("pitchfork_log_{}_{}.log", funcname, time);
        let error_filename = if pitchfork_config.keep_going && pitchfork_config.dump_errors {
            Some(format!("pitchfork_errors_{}_{}.log", funcname, time))
        } else {
            None
        };
        (log_filename, error_filename)
    };

    let mut progress_updater: Box<dyn ProgressUpdater<secret::Backend>> = if pitchfork_config.progress_updates {
        Box::new(initialize_progress_updater(&log_filename, &mut config, pitchfork_config.debug_logging))
    } else {
        Box::new(NullProgressUpdater { })
    };

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
    match args {
        Some(args) => {
            assert_eq!(params.len(), args.len(), "Function {:?} has {} parameters, but we received only {} argument `AbstractData`s", funcname, params.len(), args.len());
            allocation::allocate_args(project, em.mut_state(), sd, params.zip(args.into_iter())).unwrap();
        },
        None => {
            allocation::allocate_args(project, em.mut_state(), sd, params.zip(std::iter::repeat(AbstractData::default()))).unwrap();
        },
    }
    debug!("Done allocating memory for function parameters");

    let mut blocks_seen = BlocksSeen::new();
    let mangled_funcname = {
        let (func, _) = project.get_func_by_name(funcname).unwrap();
        &func.name
    };
    let mut path_results = Vec::new();
    let mut error_file = error_filename.as_ref().map(|filename| {
        use std::fs::File;
        use std::path::Path;
        File::create(&Path::new(filename))
            .unwrap_or_else(|e| panic!("Failed to open file {} to dump errors: {}", filename, e))
    });

    loop {
        match em.next() {
            Some(Ok(_)) => {
                info!("Finished a path with no errors or violations");
                blocks_seen.update_with_current_path(&em);
                let path_result = ConstantTimeResultForPath::IsConstantTime;
                progress_updater.update_path_result(&path_result);
                path_results.push(path_result);
            },
            Some(Err(error)) => {
                blocks_seen.update_with_current_path(&em);
                let mut full_message = em.state().full_error_message_with_context(error.clone());
                if full_message.contains("debug-level logging messages") {
                    // add our own Pitchfork-specific logging advice
                    full_message.push_str("note: To enable debug-level logging messages when `progress_updates` is\n");
                    full_message.push_str("      enabled in `PitchforkConfig`, use the `debug_logging` setting\n");
                }
                if let Some(ref mut file) = error_file {
                    use std::io::Write;
                    write!(file, "==================\n\n{}\n\n", full_message)
                        .unwrap_or_else(|e| warn!("Failed to write an error message to file: {}", e));
                }
                let path_result = if full_message.contains("Constant-time violation:") {
                    info!("Found a constant-time violation on this path");
                    ConstantTimeResultForPath::NotConstantTime { violation_message: full_message }
                } else {
                    info!("Encountered an error (other than a constant-time violation) on this path: {}", error);
                    ConstantTimeResultForPath::OtherError { error, full_message }
                };
                progress_updater.update_path_result(&path_result);
                path_results.push(path_result);
                if !pitchfork_config.keep_going {
                    break;
                }
            },
            None => break,
        }
    }

    let block_coverage = blocks_seen.full_coverage_stats();
    info!("Block coverage of toplevel function ({:?}): {:.1}%", funcname, 100.0 * block_coverage.get(mangled_funcname).unwrap().percentage);

    progress_updater.finalize();

    ConstantTimeResultForFunction {
        funcname,
        mangled_funcname,
        path_results,
        block_coverage,
        error_filename,
    }
}

fn hook_uninitialized_function_pointer(
    proj: &Project,
    state: &mut State<secret::Backend>,
    call: &dyn IsCall,
) -> Result<ReturnValue<secret::BV>> {
    info!("Function pointer is uninitialized; trying Pitchfork default hook");
    default_hook::pitchfork_default_hook(proj, state, call)
}

trait ProgressUpdater<B: Backend> {
    fn update_progress(&self, state: &State<B>) -> Result<()>;
    fn update_path_result(&self, path_result: &ConstantTimeResultForPath);
    fn process_log_message(&self, record: &log::Record) -> std::result::Result<(), Box<dyn std::error::Error + Sync + Send>>;
    fn finalize(&mut self);
}

/// a progress-updater which just no-ops all the progress-update functions
struct NullProgressUpdater { }

impl<B: Backend> ProgressUpdater<B> for NullProgressUpdater {
    fn update_progress(&self, _state: &State<B>) -> Result<()> { Ok(()) }
    fn update_path_result(&self, _path_result: &ConstantTimeResultForPath) { }
    fn process_log_message(&self, _record: &log::Record) -> std::result::Result<(), Box<dyn std::error::Error + Sync + Send>> { Ok(()) }
    fn finalize(&mut self) { }
}

// initializes and returns a `progress::ProgressUpdater` if the crate feature is
// enabled, else initializes and returns a `NullProgressUpdater`
#[cfg(feature = "progress-updates")]
fn initialize_progress_updater<B: Backend>(log_filename: &str, config: &mut Config<B>, debug_logging: bool) -> progress::ProgressUpdater {
    // the 'real' implementation is in the `progress` module, which only exists if the `progress_updates` crate feature is enabled
    progress::initialize_progress_updater(log_filename, config, debug_logging)
}
#[cfg(not(feature = "progress-updates"))]
fn initialize_progress_updater<B: Backend>(_log_filename: &str, _config: &mut Config<B>, _debug_logging: bool) -> NullProgressUpdater {
    NullProgressUpdater { }
}
