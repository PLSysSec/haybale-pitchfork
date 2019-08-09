use llvm_ir::Module;
use pitchfork::*;
use std::path::Path;

fn init_logging() {
    // capture log messages with test harness
    let _ = env_logger::builder().is_test(true).try_init();
}

fn get_module() -> Module {
    Module::from_bc_path(&Path::new("tests/bcfiles/ct.bc"))
        .expect("Failed to parse module")
}

#[test]
fn ct_simple() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let module = get_module();
    let func = module.get_func_by_name("ct_simple").expect("Failed to find function");
    assert!(is_constant_time_in_inputs(&ctx, &func, &module, &Config::default()));
}

#[test]
fn ct_simple2() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let module = get_module();
    let func = module.get_func_by_name("ct_simple2").expect("Failed to find function");
    assert!(is_constant_time_in_inputs(&ctx, &func, &module, &Config::default()));
}

#[test]
fn notct_branch() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let module = get_module();
    let func = module.get_func_by_name("notct_branch").expect("Failed to find function");
    assert!(!is_constant_time_in_inputs(&ctx, &func, &module, &Config::default()));
}

#[test]
fn notct_mem() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let module = get_module();
    let func = module.get_func_by_name("notct_mem").expect("Failed to find function");
    assert!(!is_constant_time_in_inputs(&ctx, &func, &module, &Config::default()));
}

#[test]
fn notct_onepath() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let module = get_module();
    let func = module.get_func_by_name("notct_onepath").expect("Failed to find function");
    assert!(!is_constant_time_in_inputs(&ctx, &func, &module, &Config::default()));
}

#[test]
fn ct_onearg() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let module = get_module();
    let func = module.get_func_by_name("ct_onearg").expect("Failed to find function");
    let publicx_secrety = std::iter::once(AbstractData::PublicNonPointer { bits: 32, value: AbstractValue::Unconstrained })
        .chain(std::iter::once(AbstractData::Secret { bits: 32 }));
    let secretx_publicy = std::iter::once(AbstractData::Secret { bits: 32 })
        .chain(std::iter::once(AbstractData::PublicNonPointer { bits: 32, value: AbstractValue::Unconstrained }));
    assert!(is_constant_time(&ctx, &func, &module, publicx_secrety, &Config::default()));
    assert!(!is_constant_time(&ctx, &func, &module, secretx_publicy, &Config::default()));
}

#[test]
fn ct_secrets() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let module = get_module();
    let func = module.get_func_by_name("ct_secrets").expect("Failed to find function");
    let arg = std::iter::once(AbstractData::PublicPointer(Box::new(AbstractData::Array {
        element_type: Box::new(AbstractData::Secret { bits: 32 }),
        num_elements: 100,
    })));
    assert!(is_constant_time(&ctx, &func, &module, arg, &Config::default()));
}

#[test]
fn notct_secrets() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let module = get_module();
    let func = module.get_func_by_name("notct_secrets").expect("Failed to find function");
    let arg = std::iter::once(AbstractData::PublicPointer(Box::new(AbstractData::Array {
        element_type: Box::new(AbstractData::Secret { bits: 32 }),
        num_elements: 100,
    })));
    assert!(!is_constant_time(&ctx, &func, &module, arg, &Config::default()));
}

fn ptr_to_struct_partially_secret() -> AbstractData {
    AbstractData::PublicPointer(Box::new(AbstractData::Struct(vec![
        AbstractData::PublicNonPointer { bits: 32, value: AbstractValue::Unconstrained },
        AbstractData::Secret { bits: 32 },
    ])))
}

#[test]
fn ct_struct() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let module = get_module();
    let func = module.get_func_by_name("ct_struct").expect("Failed to find function");
    let args = std::iter::once(AbstractData::PublicPointer(Box::new(AbstractData::Array {
        element_type: Box::new(AbstractData::PublicNonPointer { bits: 32, value: AbstractValue::Unconstrained }),
        num_elements: 100,
    }))).chain(std::iter::once(ptr_to_struct_partially_secret()));
    assert!(is_constant_time(&ctx, &func, &module, args, &Config::default()));
}

#[test]
fn notct_struct() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let module = get_module();
    let func = module.get_func_by_name("notct_struct").expect("Failed to find function");
    let args = std::iter::once(AbstractData::PublicPointer(Box::new(AbstractData::Array {
        element_type: Box::new(AbstractData::PublicNonPointer { bits: 32, value: AbstractValue::Unconstrained }),
        num_elements: 100,
    }))).chain(std::iter::once(ptr_to_struct_partially_secret()));
    assert!(!is_constant_time(&ctx, &func, &module, args, &Config::default()));
}

fn ptr_to_ptr_to_secrets() -> AbstractData {
    AbstractData::PublicPointer(Box::new(AbstractData::Array {
        element_type: Box::new(AbstractData::PublicPointer(Box::new(AbstractData::Array {
            element_type: Box::new(AbstractData::Secret { bits: 32 }),
            num_elements: 30,
        }))),
        num_elements: 5,
    }))
}

#[test]
fn ct_doubleptr() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let module = get_module();
    let func = module.get_func_by_name("ct_doubleptr").expect("Failed to find function");
    assert!(is_constant_time(&ctx, &func, &module, std::iter::once(ptr_to_ptr_to_secrets()), &Config::default()));
}

#[test]
fn notct_doubleptr() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let module = get_module();
    let func = module.get_func_by_name("notct_doubleptr").expect("Failed to find function");
    assert!(!is_constant_time(&ctx, &func, &module, std::iter::once(ptr_to_ptr_to_secrets()), &Config::default()));
}
