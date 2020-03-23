use crate::check_for_ct_violation;
use crate::{AbstractData, PitchforkConfig, StructDescriptions};
use crate::secret;

use colored::*;
use haybale::{Config, Project};
use haybale::config::NullPointerChecking;
use haybale::function_hooks::FunctionHooks;
use itertools::Itertools;
use std::time::Duration;

fn usage() {
    let progname = std::env::args().next().unwrap();
    println!("Usage:");
    println!("  {} <options> funcname1 [funcname2] [...]", progname);
    println!("Each function specified by name will be checked for constant-time violations.");
    println!();
    println!("Options: (these must come before non-option arguments)");
    println!();
    println!("  -h, --help: display this help message and exit");
    println!();
    println!("  --list-functions: list all functions defined in the LLVM bitcode and exit");
    println!();
    println!("  --loop-bound <n>: Use <n> as the value for the similarly named option in");
    println!("      `haybale::Config`; see docs there. If this option is not specified,");
    println!("      it defaults to 100.");
    println!();
    println!("  --max-callstack-depth <n>: Use <n> as the value for the similarly named");
    println!("      option in `haybale::Config`; see docs there. If this option is not");
    println!("      specified, it defaults to `None`.");
    println!();
    println!("  --max-memcpy-length <n>: Use <n> as the value for the similarly named");
    println!("      option in `haybale::Config`; see docs there. If this option is not");
    println!("      specified, it defaults to 4096.");
    println!();
    println!("  --solver-timeout <n>: Set the solver timeout to <n> seconds. For more");
    println!("      information, see docs on the `solver_query_timeout` option in");
    println!("      `haybale::Config`. If this option is not specified, it defaults to 300.");
    println!();
    println!("  --debug-logging: record log messages with `DEBUG` and higher priority in the");
    println!("      designated log file. If this option is not specified, only log messages");
    println!("      with `INFO` and higher priority will be recorded.");
    println!();
    println!("  --no-progress-updates: disable the progress-updates UI. This may be useful");
    println!("      for debugging Rust panics (as opposed to ordinary errors), as the");
    println!("      progress-display-updater thread may otherwise interfere with the printing");
    println!("      of the panic message.");
    println!("      With this option, instead of the progress-updates UI, log messages will");
    println!("      be printed directly to stderr. You may redirect stderr if you still want");
    println!("      log messages recorded in a file.");
    println!();
    println!("  --prefix: instead of each non-option argument being a function name, it will");
    println!("      indicate a prefix, and all functions defined in the LLVM bitcode which");
    println!("      have names beginning with that prefix will be checked for constant-time");
    println!("      violations.");
}

/// A struct which represents the options the user specified at the command-line
struct CommandLineOptions {
    pitchfork_config: PitchforkConfig,
    loop_bound: usize,
    max_callstack_depth: Option<usize>,
    max_memcpy_length: u64,
    solver_timeout: Option<Duration>,
    prefix: bool,
}

impl Default for CommandLineOptions {
    fn default() -> Self {
        Self {
            pitchfork_config: {
                let mut pitchfork_config = PitchforkConfig::default();
                // Our desired defaults may not be the same as the PitchforkConfig defaults
                pitchfork_config.keep_going = true;
                pitchfork_config.dump_errors = true;
                pitchfork_config.progress_updates = true;
                pitchfork_config.debug_logging = false;
                pitchfork_config
            },
            loop_bound: 100,
            max_callstack_depth: None,
            max_memcpy_length: 4096,
            solver_timeout: Some(Duration::from_secs(300)),
            prefix: false,
        }
    }
}

/// This function is designed to be called in your main().
/// It processes command-line arguments and coordinates the overall analysis.
///
/// All you have to provide is:
///     `get_project`: a closure which, when called, produces the `Project` you want
///         to analyze
///     `get_struct_descriptions`: a closure which, when called, produces the
///         `StructDescriptions` you want to use
///     `get_args_for_funcname`: a function which takes a function name and returns
///         the `AbstractData` arguments to use for its arguments. `None` implies to just
///         use all `AbstractData::default()`s.
///     `add_function_hooks`: a function which takes a `FunctionHooks` and adds any
///         function hooks you wish to use for your analysis
pub fn main_func(
    get_project: impl FnOnce() -> Project,
    get_struct_descriptions: impl FnOnce() -> StructDescriptions,
    get_args_for_funcname: impl Fn(&str) -> Option<Vec<AbstractData>>,
    add_function_hooks: impl Fn(&mut FunctionHooks<secret::Backend>),
) {
    let mut cmdlineoptions = CommandLineOptions::default();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                usage();
                return ();
            },
            "--list-functions" => {
                let proj = get_project();
                println!("\nFunctions defined in the LLVM bitcode:\n");
                for funcname in proj.all_functions().map(|(func, _)| &func.name).sorted() {
                    println!("{}", funcname);
                }
                return ();
            },
            "--loop-bound" => {
                cmdlineoptions.loop_bound = args.next().expect("--loop-bound argument requires a value").parse().unwrap();
            },
            "--max-callstack-depth" => {
                cmdlineoptions.max_callstack_depth = Some(args.next().expect("--max-callstack-depth argument requires a value").parse().unwrap());
            },
            "--max-memcpy-length" => {
                cmdlineoptions.max_memcpy_length = args.next().expect("--max-memcpy-length requires a value").parse().unwrap();
            },
            "--solver-timeout" => {
                cmdlineoptions.solver_timeout = Some(Duration::from_secs(args.next().expect("--solver-timeout argument requires a value").parse().unwrap()));
            },
            "--debug-logging" => {
                cmdlineoptions.pitchfork_config.debug_logging = true;
            },
            "--no-progress-updates" => {
                cmdlineoptions.pitchfork_config.progress_updates = false;
            },
            "--prefix" => {
                cmdlineoptions.prefix = true;
            },
            s if s.starts_with("--") || s.starts_with("-") => {
                eprintln!("error: unrecognized option {}", s);
                return ();
            },
            funcname => {
                process_nonoption_args(std::iter::once(funcname.into()).chain(args), cmdlineoptions, get_project, get_struct_descriptions, get_args_for_funcname, add_function_hooks);
                return ();
            },
        }
    }
    // if we got here, we didn't get any nonoption arguments, or -h, --help, or --list-functions
    println!("Error: No functions specified");
    println!();
    usage();
}

fn process_nonoption_args(
    nonoption_args: impl Iterator<Item = String>,
    cmdlineoptions: CommandLineOptions,
    get_project: impl FnOnce() -> Project,
    get_struct_descriptions: impl FnOnce() -> StructDescriptions,
    get_args_for_funcname: impl Fn(&str) -> Option<Vec<AbstractData>>,
    add_function_hooks: impl Fn(&mut FunctionHooks<secret::Backend>),
) {
    let mut results = Vec::new();
    if !cmdlineoptions.pitchfork_config.progress_updates || cfg!(not(feature = "progress-updates")) {
        use env_logger::Env;
        if cmdlineoptions.pitchfork_config.debug_logging {
            env_logger::from_env(Env::default().default_filter_or("debug")).init();
        } else {
            env_logger::from_env(Env::default().default_filter_or("info")).init();
        }
    }
    let proj = get_project();
    let struct_descriptions = get_struct_descriptions();
    let nonoption_args = nonoption_args.collect::<Vec<_>>();  // collecting here shouldn't be necessary, but ensures that all the args outlive the for loop so that `results` can be used after it
    for funcname in nonoption_args.iter() {
        if funcname.starts_with("--") || funcname.starts_with("-") {
            eprintln!("error: options must come before non-option arguments. Use --help for more information.");
            return ();
        }
        if cmdlineoptions.prefix {
            for full_funcname in proj.all_functions().map(|(func, _)| &func.name).filter(|proj_funcname| proj_funcname.starts_with(funcname)) {
                let mut config = make_config(&cmdlineoptions);
                add_function_hooks(&mut config.function_hooks);
                let result = check_for_ct_violation(
                    full_funcname,
                    &proj,
                    get_args_for_funcname(full_funcname),
                    &struct_descriptions,
                    config,
                    &cmdlineoptions.pitchfork_config,
                );
                println!("{}", result);
                results.push(result);
            }
        } else {
            let mut config = make_config(&cmdlineoptions);
            add_function_hooks(&mut config.function_hooks);
            let result = check_for_ct_violation(
                funcname,
                &proj,
                get_args_for_funcname(funcname),
                &struct_descriptions,
                config,
                &cmdlineoptions.pitchfork_config,
            );
            println!("{}", result);
            results.push(result);
        }
    }
    if results.len() > 1 {
        println!("\n=======\n\nSummary of results:\n");
        for result in results {
            let path_stats = result.path_statistics();
            let is_ct = result.path_results.len() == path_stats.num_ct_paths;
            let have_violation = path_stats.num_ct_violations > 0;
            println!("{} {}", result.funcname,
                if is_ct { "is constant-time".green() }
                else if have_violation { "is not constant-time".red() }
                else { "encountered errors".red() }
            );
            println!("{}", path_stats);
        }
    }
}

fn make_config<'p>(cmdlineoptions: &CommandLineOptions) -> Config<'p, secret::Backend> {
    let mut config = Config::default();
    config.loop_bound = cmdlineoptions.loop_bound;
    config.max_callstack_depth = cmdlineoptions.max_callstack_depth;
    config.solver_query_timeout = cmdlineoptions.solver_timeout;
    config.max_memcpy_length = Some(cmdlineoptions.max_memcpy_length);
    config.null_pointer_checking = NullPointerChecking::SplitPath;
    config
}
