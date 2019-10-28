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
        AbstractData::pub_i32(AbstractValue::Unconstrained),
        AbstractData::sec_i32(),
    );
    let secretx_publicy = iterator_length_two(
        AbstractData::sec_i32(),
        AbstractData::pub_i32(AbstractValue::Unconstrained),
    );
    assert!(is_constant_time("ct_onearg", &project, publicx_secrety, &StructDescriptions::new(), Config::default()));
    assert!(!is_constant_time("ct_onearg", &project, secretx_publicy, &StructDescriptions::new(), Config::default()));
}

#[test]
fn ct_secrets() {
    init_logging();
    let project = get_project();
    let arg = iterator_length_one(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::sec_i32(), 100)),
    );
    assert!(is_constant_time("ct_secrets", &project, arg, &StructDescriptions::new(), Config::default()));
}

#[test]
fn notct_secrets() {
    init_logging();
    let project = get_project();
    let arg = iterator_length_one(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::sec_i32(), 100)),
    );
    assert!(!is_constant_time("notct_secrets", &project, arg, &StructDescriptions::new(), Config::default()));
}

fn struct_partially_secret() -> AbstractData {
    AbstractData::struct_of(vec![
        AbstractData::pub_i32(AbstractValue::Unconstrained),
        AbstractData::sec_i32(),
    ])
}

#[test]
fn ct_struct() {
    init_logging();
    let project = get_project();
    let args = iterator_length_two(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(struct_partially_secret()),
    );
    assert!(is_constant_time("ct_struct", &project, args, &StructDescriptions::new(), Config::default()));
    // now check again, using `default()` and `StructDescriptions`
    let args = iterator_length_two(
        AbstractData::default(),
        AbstractData::default(),
    );
    let sd = iterator_length_one(("struct.PartiallySecret".to_owned(), struct_partially_secret())).into_iter().collect();
    assert!(is_constant_time("ct_struct", &project, args, &sd, Config::default()));
}

#[test]
fn notct_struct() {
    init_logging();
    let project = get_project();
    let args = iterator_length_two(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(struct_partially_secret()),
    );
    assert!(!is_constant_time("notct_struct", &project, args, &StructDescriptions::new(), Config::default()));
    // now check again, using `default()` and `StructDescriptions`
    let args = iterator_length_two(
        AbstractData::default(),
        AbstractData::default(),
    );
    let sd = iterator_length_one(("struct.PartiallySecret".to_owned(), struct_partially_secret())).into_iter().collect();
    assert!(!is_constant_time("notct_struct", &project, args, &sd, Config::default()));
}

fn ptr_to_ptr_to_secrets() -> AbstractData {
    AbstractData::pub_pointer_to(AbstractData::array_of(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::sec_i32(), 30)),
        5,
    ))
}

#[test]
fn ct_doubleptr() {
    init_logging();
    let project = get_project();
    assert!(is_constant_time("ct_doubleptr", &project, iterator_length_one(ptr_to_ptr_to_secrets()), &StructDescriptions::new(), Config::default()));
}

#[test]
fn notct_doubleptr() {
    init_logging();
    let project = get_project();
    assert!(!is_constant_time("notct_doubleptr", &project, iterator_length_one(ptr_to_ptr_to_secrets()), &StructDescriptions::new(), Config::default()));
}
