use pitchfork::*;
use std::path::Path;

fn init_logging() {
    // capture log messages with test harness
    let _ = env_logger::builder().is_test(true).try_init();
}

fn get_project() -> Project {
    Project::from_bc_path(&Path::new("tests/bcfiles/ct.bc"))
        .unwrap_or_else(|e| panic!("Failed to create project: {}", e))
}

#[test]
fn ct_simple() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let project = get_project();
    assert!(is_constant_time_in_inputs(&ctx, "ct_simple", &project, Config::default()));
}

#[test]
fn ct_simple2() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let project = get_project();
    assert!(is_constant_time_in_inputs(&ctx, "ct_simple2", &project, Config::default()));
}

#[test]
fn notct_branch() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let project = get_project();
    assert!(!is_constant_time_in_inputs(&ctx, "notct_branch", &project, Config::default()));
}

#[test]
fn notct_mem() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let project = get_project();
    assert!(!is_constant_time_in_inputs(&ctx, "notct_mem", &project, Config::default()));
}

#[test]
fn notct_onepath() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let project = get_project();
    assert!(!is_constant_time_in_inputs(&ctx, "notct_onepath", &project, Config::default()));
}

#[test]
fn ct_onearg() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let project = get_project();
    let publicx_secrety = std::iter::once(AbstractData::PublicValue { bits: 32, value: AbstractValue::Unconstrained })
        .chain(std::iter::once(AbstractData::Secret { bits: 32 }));
    let secretx_publicy = std::iter::once(AbstractData::Secret { bits: 32 })
        .chain(std::iter::once(AbstractData::PublicValue { bits: 32, value: AbstractValue::Unconstrained }));
    assert!(is_constant_time(&ctx, "ct_onearg", &project, publicx_secrety, Config::default()));
    assert!(!is_constant_time(&ctx, "ct_onearg", &project, secretx_publicy, Config::default()));
}

#[test]
fn ct_secrets() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let project = get_project();
    let arg = std::iter::once(AbstractData::PublicPointerTo(Box::new(AbstractData::Array {
        element_type: Box::new(AbstractData::Secret { bits: 32 }),
        num_elements: 100,
    })));
    assert!(is_constant_time(&ctx, "ct_secrets", &project, arg, Config::default()));
}

#[test]
fn notct_secrets() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let project = get_project();
    let arg = std::iter::once(AbstractData::PublicPointerTo(Box::new(AbstractData::Array {
        element_type: Box::new(AbstractData::Secret { bits: 32 }),
        num_elements: 100,
    })));
    assert!(!is_constant_time(&ctx, "notct_secrets", &project, arg, Config::default()));
}

fn ptr_to_struct_partially_secret() -> AbstractData {
    AbstractData::PublicPointerTo(Box::new(AbstractData::Struct(vec![
        AbstractData::PublicValue { bits: 32, value: AbstractValue::Unconstrained },
        AbstractData::Secret { bits: 32 },
    ])))
}

#[test]
fn ct_struct() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let project = get_project();
    let args = std::iter::once(AbstractData::PublicPointerTo(Box::new(AbstractData::Array {
        element_type: Box::new(AbstractData::PublicValue { bits: 32, value: AbstractValue::Unconstrained }),
        num_elements: 100,
    }))).chain(std::iter::once(ptr_to_struct_partially_secret()));
    assert!(is_constant_time(&ctx, "ct_struct", &project, args, Config::default()));
}

#[test]
fn notct_struct() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let project = get_project();
    let args = std::iter::once(AbstractData::PublicPointerTo(Box::new(AbstractData::Array {
        element_type: Box::new(AbstractData::PublicValue { bits: 32, value: AbstractValue::Unconstrained }),
        num_elements: 100,
    }))).chain(std::iter::once(ptr_to_struct_partially_secret()));
    assert!(!is_constant_time(&ctx, "notct_struct", &project, args, Config::default()));
}

fn ptr_to_ptr_to_secrets() -> AbstractData {
    AbstractData::PublicPointerTo(Box::new(AbstractData::Array {
        element_type: Box::new(AbstractData::PublicPointerTo(Box::new(AbstractData::Array {
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
    let project = get_project();
    assert!(is_constant_time(&ctx, "ct_doubleptr", &project, std::iter::once(ptr_to_ptr_to_secrets()), Config::default()));
}

#[test]
fn notct_doubleptr() {
    init_logging();
    let ctx = z3::Context::new(&z3::Config::new());
    let project = get_project();
    assert!(!is_constant_time(&ctx, "notct_doubleptr", &project, std::iter::once(ptr_to_ptr_to_secrets()), Config::default()));
}
