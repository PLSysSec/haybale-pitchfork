//! Whether each of the functions in haybale's test suite are constant-time

use llvm_ir::Module;
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
    let module = Module::from_bc_path(&Path::new("../haybale/tests/bcfiles/basic.bc"))
        .expect("Failed to parse module");
    let no_args_nozero = module.get_func_by_name("no_args_nozero").expect("Failed to find function");
    let no_args_zero = module.get_func_by_name("no_args_zero").expect("Failed to find function");
    let one_arg = module.get_func_by_name("one_arg").expect("Failed to find function");
    let two_args = module.get_func_by_name("two_args").expect("Failed to find function");
    let three_args = module.get_func_by_name("three_args").expect("Failed to find function");
    let four_args = module.get_func_by_name("four_args").expect("Failed to find function");
    let five_args = module.get_func_by_name("five_args").expect("Failed to find function");
    let binops = module.get_func_by_name("binops").expect("Failed to find function");
    let conditional_true = module.get_func_by_name("conditional_true").expect("Failed to find function");
    let conditional_false = module.get_func_by_name("conditional_false").expect("Failed to find function");
    let conditional_nozero = module.get_func_by_name("conditional_nozero").expect("Failed to find function");
    let conditional_with_and = module.get_func_by_name("conditional_with_and").expect("Failed to find function");
    let int8t = module.get_func_by_name("int8t").expect("Failed to find function");
    let int16t = module.get_func_by_name("int16t").expect("Failed to find function");
    let int32t = module.get_func_by_name("int32t").expect("Failed to find function");
    let int64t = module.get_func_by_name("int64t").expect("Failed to find function");
    let mixed_bitwidths = module.get_func_by_name("mixed_bitwidths").expect("Failed to find function");

    // Most of the functions in basic.bc are constant-time
    assert!(is_constant_time_in_inputs(&no_args_nozero, &module, 20));
    assert!(is_constant_time_in_inputs(&no_args_zero, &module, 20));
    assert!(is_constant_time_in_inputs(&one_arg, &module, 20));
    assert!(is_constant_time_in_inputs(&two_args, &module, 20));
    assert!(is_constant_time_in_inputs(&three_args, &module, 20));
    assert!(is_constant_time_in_inputs(&four_args, &module, 20));
    assert!(is_constant_time_in_inputs(&five_args, &module, 20));
    assert!(is_constant_time_in_inputs(&binops, &module, 20));

    // These functions branch on conditions influenced by their inputs, so they're not constant-time
    assert!(!is_constant_time_in_inputs(&conditional_true, &module, 20));
    assert!(!is_constant_time_in_inputs(&conditional_false, &module, 20));
    assert!(!is_constant_time_in_inputs(&conditional_nozero, &module, 20));

    // LLVM actually compiles this function to be branch-free and therefore constant-time
    assert!(is_constant_time_in_inputs(&conditional_with_and, &module, 20));

    // These functions are also naturally constant-time
    assert!(is_constant_time_in_inputs(&int8t, &module, 20));
    assert!(is_constant_time_in_inputs(&int16t, &module, 20));
    assert!(is_constant_time_in_inputs(&int32t, &module, 20));
    assert!(is_constant_time_in_inputs(&int64t, &module, 20));
    assert!(is_constant_time_in_inputs(&mixed_bitwidths, &module, 20));
}

/// Whether each of the functions in haybale's `memory.bc` are constant-time in their inputs
#[test]
fn haybale_memory() {
    init_logging();
    let module = Module::from_bc_path(&Path::new("../haybale/tests/bcfiles/memory.bc"))
        .expect("Failed to parse module");
    let load_and_store = module.get_func_by_name("load_and_store").expect("Failed to find function");
    let local_ptr = module.get_func_by_name("local_ptr").expect("Failed to find function");
    let overwrite = module.get_func_by_name("overwrite").expect("Failed to find function");
    let load_and_store_mult = module.get_func_by_name("load_and_store_mult").expect("Failed to find function");
    let array = module.get_func_by_name("array").expect("Failed to find function");
    let pointer_arith = module.get_func_by_name("pointer_arith").expect("Failed to find function");

    // local_ptr is the only function in this file that is constant-time in its inputs
    assert!(is_constant_time_in_inputs(&local_ptr, &module, 20));

    // All other functions in the module perform memory accesses whose addresses depend on function arguments
    assert!(!is_constant_time_in_inputs(&load_and_store, &module, 20));
    assert!(!is_constant_time_in_inputs(&overwrite, &module, 20));
    assert!(!is_constant_time_in_inputs(&load_and_store_mult, &module, 20));
    assert!(!is_constant_time_in_inputs(&array, &module, 20));
    assert!(!is_constant_time_in_inputs(&pointer_arith, &module, 20));
}
