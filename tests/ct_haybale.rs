//! Whether each of the functions in haybale's test suite are constant-time

use pitchfork::*;
use std::path::Path;

fn init_logging() {
    // capture log messages with test harness
    let _ = env_logger::builder().is_test(true).try_init();
}

/// Whether each of the functions in haybale's `basic.bc` are constant-time in their inputs
#[test]
fn haybale_basic() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let project = Project::from_bc_path(&Path::new("../haybale/tests/bcfiles/basic.bc"))
        .unwrap_or_else(|e| panic!("Failed to create project: {}", e));

    // Most of the functions in basic.bc are constant-time
    assert!(is_constant_time_in_inputs(&ctx, "no_args_nozero", &project, Config::default()));
    assert!(is_constant_time_in_inputs(&ctx, "no_args_zero", &project, Config::default()));
    assert!(is_constant_time_in_inputs(&ctx, "one_arg", &project, Config::default()));
    assert!(is_constant_time_in_inputs(&ctx, "two_args", &project, Config::default()));
    assert!(is_constant_time_in_inputs(&ctx, "three_args", &project, Config::default()));
    assert!(is_constant_time_in_inputs(&ctx, "four_args", &project, Config::default()));
    assert!(is_constant_time_in_inputs(&ctx, "five_args", &project, Config::default()));
    assert!(is_constant_time_in_inputs(&ctx, "binops", &project, Config::default()));

    // These functions branch on conditions influenced by their inputs, so they're not constant-time
    assert!(!is_constant_time_in_inputs(&ctx, "conditional_true", &project, Config::default()));
    assert!(!is_constant_time_in_inputs(&ctx, "conditional_false", &project, Config::default()));
    assert!(!is_constant_time_in_inputs(&ctx, "conditional_nozero", &project, Config::default()));

    // LLVM actually compiles this function to be branch-free and therefore constant-time
    assert!(is_constant_time_in_inputs(&ctx, "conditional_with_and", &project, Config::default()));

    // These functions are also naturally constant-time
    assert!(is_constant_time_in_inputs(&ctx, "int8t", &project, Config::default()));
    assert!(is_constant_time_in_inputs(&ctx, "int16t", &project, Config::default()));
    assert!(is_constant_time_in_inputs(&ctx, "int32t", &project, Config::default()));
    assert!(is_constant_time_in_inputs(&ctx, "int64t", &project, Config::default()));
    assert!(is_constant_time_in_inputs(&ctx, "mixed_bitwidths", &project, Config::default()));
}

/// Whether each of the functions in haybale's `memory.bc` are constant-time in their inputs
#[test]
fn haybale_memory() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let project = Project::from_bc_path(&Path::new("../haybale/tests/bcfiles/memory.bc"))
        .unwrap_or_else(|e| panic!("Failed to create project: {}", e));

    // local_ptr is the only function in this file that is constant-time in its inputs
    assert!(is_constant_time_in_inputs(&ctx, "local_ptr", &project, Config::default()));

    // All other functions in the module perform memory accesses whose addresses depend on function arguments
    assert!(!is_constant_time_in_inputs(&ctx, "load_and_store", &project, Config::default()));
    assert!(!is_constant_time_in_inputs(&ctx, "overwrite", &project, Config::default()));
    assert!(!is_constant_time_in_inputs(&ctx, "load_and_store_mult", &project, Config::default()));
    assert!(!is_constant_time_in_inputs(&ctx, "array", &project, Config::default()));
    assert!(!is_constant_time_in_inputs(&ctx, "pointer_arith", &project, Config::default()));
}
