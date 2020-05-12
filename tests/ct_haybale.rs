//! Whether each of the functions in haybale's test suite are constant-time

use haybale_pitchfork::*;
use std::path::Path;

fn init_logging() {
    // since our tests will run with `progress_updates == false`,
    // we are responsible for capturing log messages ourselves.
    let _ = env_logger::builder().is_test(true).try_init();
}

pub fn is_constant_time_in_inputs<'p>(
    funcname: &'p str,
    project: &'p Project,
    config: Config<'p, secret::Backend>
) -> bool {
    let mut pitchfork_config = PitchforkConfig::default();
    pitchfork_config.keep_going = false;
    pitchfork_config.dump_errors = false;
    pitchfork_config.progress_updates = false;
    check_for_ct_violation_in_inputs(funcname, project, config, &pitchfork_config)
        .first_error_or_violation()
        .is_none()
}

/// Whether each of the functions in haybale's `basic.bc` are constant-time in their inputs
#[test]
fn haybale_basic() {
    init_logging();
    let project = Project::from_bc_path(&Path::new("tests/bcfiles/haybale/basic.bc"))
        .unwrap_or_else(|e| panic!("Failed to create project: {}", e));

    // Most of the functions in basic.bc are constant-time
    assert!(is_constant_time_in_inputs("no_args_nozero", &project, Config::default()));
    assert!(is_constant_time_in_inputs("no_args_zero", &project, Config::default()));
    assert!(is_constant_time_in_inputs("one_arg", &project, Config::default()));
    assert!(is_constant_time_in_inputs("two_args", &project, Config::default()));
    assert!(is_constant_time_in_inputs("three_args", &project, Config::default()));
    assert!(is_constant_time_in_inputs("four_args", &project, Config::default()));
    assert!(is_constant_time_in_inputs("five_args", &project, Config::default()));
    assert!(is_constant_time_in_inputs("binops", &project, Config::default()));

    // These functions branch on conditions influenced by their inputs, so they're not constant-time
    assert!(!is_constant_time_in_inputs("conditional_true", &project, Config::default()));
    assert!(!is_constant_time_in_inputs("conditional_false", &project, Config::default()));
    assert!(!is_constant_time_in_inputs("conditional_nozero", &project, Config::default()));

    // LLVM actually compiles this function to be branch-free and therefore constant-time
    assert!(is_constant_time_in_inputs("conditional_with_and", &project, Config::default()));

    // These functions are also naturally constant-time
    assert!(is_constant_time_in_inputs("int8t", &project, Config::default()));
    assert!(is_constant_time_in_inputs("int16t", &project, Config::default()));
    assert!(is_constant_time_in_inputs("int32t", &project, Config::default()));
    assert!(is_constant_time_in_inputs("int64t", &project, Config::default()));
    assert!(is_constant_time_in_inputs("mixed_bitwidths", &project, Config::default()));
}

/// Whether each of the functions in haybale's `memory.bc` are constant-time in their inputs
#[test]
fn haybale_memory() {
    init_logging();
    let project = Project::from_bc_path(&Path::new("tests/bcfiles/haybale/memory.bc"))
        .unwrap_or_else(|e| panic!("Failed to create project: {}", e));

    // local_ptr is the only function in this file that is constant-time in its inputs
    assert!(is_constant_time_in_inputs("local_ptr", &project, Config::default()));

    // All other functions in the module perform memory accesses whose addresses depend on function arguments
    assert!(!is_constant_time_in_inputs("load_and_store", &project, Config::default()));
    assert!(!is_constant_time_in_inputs("overwrite", &project, Config::default()));
    assert!(!is_constant_time_in_inputs("load_and_store_mult", &project, Config::default()));
    assert!(!is_constant_time_in_inputs("array", &project, Config::default()));
    assert!(!is_constant_time_in_inputs("pointer_arith", &project, Config::default()));
}
