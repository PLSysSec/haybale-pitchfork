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

fn iterator_length_one<I>(obj: I) -> impl IntoIterator<Item = I> {
    std::iter::once(obj)
}

fn iterator_length_two<I>(a: I, b: I) -> impl IntoIterator<Item = I> {
    std::iter::once(a).chain(std::iter::once(b))
}

#[test]
fn ct_simple() {
    init_logging();
    let project = get_project();
    assert!(is_constant_time_in_inputs("ct_simple", &project, Config::default()));
}

#[test]
fn ct_simple2() {
    init_logging();
    let project = get_project();
    assert!(is_constant_time_in_inputs("ct_simple2", &project, Config::default()));
}

#[test]
fn notct_branch() {
    init_logging();
    let project = get_project();
    assert!(!is_constant_time_in_inputs("notct_branch", &project, Config::default()));
}

#[test]
fn notct_mem() {
    init_logging();
    let project = get_project();
    assert!(!is_constant_time_in_inputs("notct_mem", &project, Config::default()));
}

#[test]
fn notct_onepath() {
    init_logging();
    let project = get_project();
    assert!(!is_constant_time_in_inputs("notct_onepath", &project, Config::default()));
}

#[test]
fn ct_onearg() {
    init_logging();
    let project = get_project();
    let publicx_secrety = iterator_length_two(
        UnderspecifiedAbstractData::pub_i32(AbstractValue::Unconstrained),
        UnderspecifiedAbstractData::sec_i32(),
    );
    let secretx_publicy = iterator_length_two(
        UnderspecifiedAbstractData::sec_i32(),
        UnderspecifiedAbstractData::pub_i32(AbstractValue::Unconstrained),
    );
    assert!(is_constant_time("ct_onearg", &project, publicx_secrety, Config::default()));
    assert!(!is_constant_time("ct_onearg", &project, secretx_publicy, Config::default()));
}

#[test]
fn ct_secrets() {
    init_logging();
    let project = get_project();
    let arg = iterator_length_one(
        UnderspecifiedAbstractData::pub_pointer_to(AbstractData::Array {
            element_type: Box::new(AbstractData::sec_i32()),
            num_elements: 100,
        })
    );
    assert!(is_constant_time("ct_secrets", &project, arg, Config::default()));
}

#[test]
fn notct_secrets() {
    init_logging();
    let project = get_project();
    let arg = iterator_length_one(
        UnderspecifiedAbstractData::pub_pointer_to(AbstractData::Array {
            element_type: Box::new(AbstractData::sec_i32()),
            num_elements: 100,
        })
    );
    assert!(!is_constant_time("notct_secrets", &project, arg, Config::default()));
}

fn ptr_to_struct_partially_secret() -> UnderspecifiedAbstractData {
    UnderspecifiedAbstractData::pub_pointer_to(AbstractData::Struct(vec![
        AbstractData::pub_i32(AbstractValue::Unconstrained),
        AbstractData::sec_i32(),
    ]))
}

#[test]
fn ct_struct() {
    init_logging();
    let project = get_project();
    let args = iterator_length_two(
        UnderspecifiedAbstractData::pub_pointer_to(AbstractData::Array {
            element_type: Box::new(AbstractData::pub_i32(AbstractValue::Unconstrained)),
            num_elements: 100,
        }),
        ptr_to_struct_partially_secret(),
    );
    assert!(is_constant_time("ct_struct", &project, args, Config::default()));
    // now check again, using Unspecified
    let args = iterator_length_two(
        UnderspecifiedAbstractData::Unspecified,
        ptr_to_struct_partially_secret(),
    );
    assert!(is_constant_time("ct_struct", &project, args, Config::default()));
}

#[test]
fn notct_struct() {
    init_logging();
    let project = get_project();
    let args = iterator_length_two(
        UnderspecifiedAbstractData::pub_pointer_to(AbstractData::Array {
            element_type: Box::new(AbstractData::pub_i32(AbstractValue::Unconstrained)),
            num_elements: 100,
        }),
        ptr_to_struct_partially_secret(),
    );
    assert!(!is_constant_time("notct_struct", &project, args, Config::default()));
    // now check again, using Unspecified
    let args = iterator_length_two(
        UnderspecifiedAbstractData::Unspecified,
        ptr_to_struct_partially_secret(),
    );
    assert!(!is_constant_time("notct_struct", &project, args, Config::default()));
}

fn ptr_to_ptr_to_secrets() -> UnderspecifiedAbstractData {
    UnderspecifiedAbstractData::pub_pointer_to(AbstractData::Array {
        element_type: Box::new(AbstractData::pub_pointer_to(AbstractData::Array {
            element_type: Box::new(AbstractData::sec_i32()),
            num_elements: 30,
        })),
        num_elements: 5,
    })
}

#[test]
fn ct_doubleptr() {
    init_logging();
    let project = get_project();
    assert!(is_constant_time("ct_doubleptr", &project, iterator_length_one(ptr_to_ptr_to_secrets()), Config::default()));
}

#[test]
fn notct_doubleptr() {
    init_logging();
    let project = get_project();
    assert!(!is_constant_time("notct_doubleptr", &project, iterator_length_one(ptr_to_ptr_to_secrets()), Config::default()));
}
