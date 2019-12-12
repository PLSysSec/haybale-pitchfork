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

fn iterator_length_three<I>(a: I, b: I, c: I) -> impl IntoIterator<Item = I> {
    std::iter::once(a).chain(std::iter::once(b)).chain(std::iter::once(c))
}

fn assert_no_ct_violation(violation: Option<String>) {
    assert!(violation.is_none(), "{}", violation.unwrap());
}

fn assert_is_ct_violation(violation: Option<String>) {
    match violation {
        None => panic!("Expected a ct violation but didn't get one"),
        Some(violation) => {
            if violation.contains("Constant-time violation:") {
                // pass
            } else {
                panic!("Expected a ct violation but got a different error:\n  {}", violation)
            }
        }
    }
}

#[test]
fn ct_simple() {
    init_logging();
    let project = get_project();
    let violation = check_for_ct_violation_in_inputs("ct_simple", &project, Config::default());
    assert_no_ct_violation(violation);
}

#[test]
fn ct_simple2() {
    init_logging();
    let project = get_project();
    let violation = check_for_ct_violation_in_inputs("ct_simple2", &project, Config::default());
    assert_no_ct_violation(violation);
}

#[test]
fn notct_branch() {
    init_logging();
    let project = get_project();
    let violation = check_for_ct_violation_in_inputs("notct_branch", &project, Config::default());
    assert_is_ct_violation(violation);
}

#[test]
fn notct_mem() {
    init_logging();
    let project = get_project();
    let violation = check_for_ct_violation_in_inputs("notct_mem", &project, Config::default());
    assert_is_ct_violation(violation);
}

#[test]
fn notct_onepath() {
    init_logging();
    let project = get_project();
    let violation = check_for_ct_violation_in_inputs("notct_onepath", &project, Config::default());
    assert_is_ct_violation(violation);
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
    let violation = check_for_ct_violation("ct_onearg", &project, publicx_secrety, &StructDescriptions::new(), Config::default(), false);
    assert_no_ct_violation(violation);
    let violation = check_for_ct_violation("ct_onearg", &project, secretx_publicy, &StructDescriptions::new(), Config::default(), false);
    assert_is_ct_violation(violation);
}

#[test]
fn ct_secrets() {
    init_logging();
    let project = get_project();
    let arg = iterator_length_one(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::sec_i32(), 100)),
    );
    let violation = check_for_ct_violation("ct_secrets", &project, arg, &StructDescriptions::new(), Config::default(), false);
    assert_no_ct_violation(violation);
}

#[test]
fn notct_secrets() {
    init_logging();
    let project = get_project();
    let arg = iterator_length_one(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::sec_i32(), 100)),
    );
    let violation = check_for_ct_violation("notct_secrets", &project, arg, &StructDescriptions::new(), Config::default(), false);
    assert_is_ct_violation(violation);
}

fn struct_partially_secret() -> AbstractData {
    AbstractData::_struct("PartiallySecret", vec![
        AbstractData::pub_i32(AbstractValue::Range(0, 4096)),
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
    let violation = check_for_ct_violation("ct_struct", &project, args, &StructDescriptions::new(), Config::default(), false);
    assert_no_ct_violation(violation);
    // now check again, using `default()` and `StructDescriptions`
    let args = iterator_length_two(
        AbstractData::default(),
        AbstractData::default(),
    );
    let sd = iterator_length_one(("struct.PartiallySecret".to_owned(), struct_partially_secret())).into_iter().collect();
    let violation = check_for_ct_violation("ct_struct", &project, args, &sd, Config::default(), false);
    assert_no_ct_violation(violation);
}

#[test]
fn notct_struct() {
    init_logging();
    let project = get_project();
    let args = iterator_length_two(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(struct_partially_secret()),
    );
    let violation = check_for_ct_violation("notct_struct", &project, args, &StructDescriptions::new(), Config::default(), false);
    assert_is_ct_violation(violation);
    // now check again, using `default()` and `StructDescriptions`
    let args = iterator_length_two(
        AbstractData::default(),
        AbstractData::default(),
    );
    let sd = iterator_length_one(("struct.PartiallySecret".to_owned(), struct_partially_secret())).into_iter().collect();
    let violation = check_for_ct_violation("notct_struct", &project, args, &sd, Config::default(), false);
    assert_is_ct_violation(violation);
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
    let violation = check_for_ct_violation("ct_doubleptr", &project, iterator_length_one(ptr_to_ptr_to_secrets()), &StructDescriptions::new(), Config::default(), false);
    assert_no_ct_violation(violation);
}

#[test]
fn notct_doubleptr() {
    init_logging();
    let project = get_project();
    let violation = check_for_ct_violation("notct_doubleptr", &project, iterator_length_one(ptr_to_ptr_to_secrets()), &StructDescriptions::new(), Config::default(), false);
    assert_is_ct_violation(violation);
}

#[test]
fn ct_struct_voidptr() {
    init_logging();
    let project = get_project();
    let args = iterator_length_two(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(AbstractData::void_override(None, struct_partially_secret())),
    );
    let violation = check_for_ct_violation("ct_struct_voidptr", &project, args, &StructDescriptions::new(), Config::default(), false);
    assert_no_ct_violation(violation);
}

#[test]
fn notct_struct_voidptr() {
    init_logging();
    let project = get_project();
    let args = iterator_length_two(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(AbstractData::void_override(None, struct_partially_secret())),
    );
    let violation = check_for_ct_violation("notct_struct_voidptr", &project, args, &StructDescriptions::new(), Config::default(), false);
    assert_is_ct_violation(violation);
}

fn struct_parent_secretx() -> AbstractData {
    AbstractData::_struct("Parent", vec![
        AbstractData::sec_i32(),  // x
        AbstractData::default(),  // child1
        AbstractData::default(),  // child2
    ])
}

fn struct_child() -> AbstractData {
    AbstractData::_struct("Child", vec![
        AbstractData::pub_i32(AbstractValue::Unconstrained),  // y
        AbstractData::pub_pointer_to_parent(),
    ])
}

#[test]
fn indirectly_recursive_struct() {
    init_logging();
    let project = get_project();
    // Both of the `Child` structs that get instantiated should have a pointer
    // to precisely the `Parent` we pass in. In that case, we should get a
    // violation since `Parent.x` is secret. However, if the children have a
    // pointer to any other `Parent` (e.g., a generic `Parent`), `Parent.x` will
    // be public and we won't have a violation.
    let args = iterator_length_two(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(struct_parent_secretx()),
    );
    let sd = iterator_length_one(("struct.Child".to_owned(), struct_child())).into_iter().collect();
    let violation = check_for_ct_violation("indirectly_recursive_struct", &project, args, &sd, Config::default(), false);
    assert_is_ct_violation(violation);
}

#[test]
fn related_args() {
    init_logging();
    let project = get_project();
    let args = iterator_length_three(
        AbstractData::pub_i32(AbstractValue::named("length", AbstractValue::Range(0, 20))),
        AbstractData::pub_i32(AbstractValue::UnsignedLessThan("length".to_owned())),
        AbstractData::sec_i32(),
    );
    let violation = check_for_ct_violation("related_args", &project, args, &StructDescriptions::new(), Config::default(), false);
    assert_no_ct_violation(violation);

    // but if we don't have the constraint, then there should be a violation
    let args = iterator_length_three(
        AbstractData::pub_i32(AbstractValue::Range(0, 20)),
        AbstractData::default(),
        AbstractData::sec_i32(),
    );
    let violation = check_for_ct_violation("related_args", &project, args, &StructDescriptions::new(), Config::default(), false);
    assert_is_ct_violation(violation);
}

#[test]
fn struct_related_fields() {
    init_logging();
    let project = get_project();
    let args = iterator_length_one(AbstractData::pub_pointer_to(AbstractData::_struct("StructWithRelatedFields", vec![
        AbstractData::pub_i32(AbstractValue::named("length", AbstractValue::Range(0, 20))),
        AbstractData::pub_i32(AbstractValue::UnsignedLessThan("length".to_owned())),
        AbstractData::sec_i32(),
    ])));
    let violation = check_for_ct_violation("struct_related_fields", &project, args, &StructDescriptions::new(), Config::default(), false);
    assert_no_ct_violation(violation);

    // but if we don't have the constraint, then there should be a violation
    let args = iterator_length_one(AbstractData::pub_pointer_to(AbstractData::_struct("StructWithRelatedFields", vec![
        AbstractData::pub_i32(AbstractValue::Range(0, 20)),
        AbstractData::default(),
        AbstractData::sec_i32(),
    ])));
    let violation = check_for_ct_violation("struct_related_fields", &project, args, &StructDescriptions::new(), Config::default(), false);
    assert_is_ct_violation(violation);
}
