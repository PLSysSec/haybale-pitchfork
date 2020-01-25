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

fn assert_no_ct_violation(res: ConstantTimeResultForFunction) {
    match res.first_error_or_violation() {
        None => {},  // pass
        Some(ConstantTimeResultForPath::IsConstantTime) => panic!("first_error_or_violation should return an error or violation"),
        Some(ConstantTimeResultForPath::NotConstantTime { violation_message }) =>
            panic!("Expected no ct violation, but found one:\n  {}", violation_message),
        Some(ConstantTimeResultForPath::OtherError { full_message, .. }) =>
            panic!("Encountered an unexpected error:\n  {}", full_message),
    }
}

fn assert_is_ct_violation(res: ConstantTimeResultForFunction) {
    // we check for other-errors first, and fail if any are encountered,
    // even if there was also a ct violation reported
    for path_result in &res.path_results {
        match path_result {
            ConstantTimeResultForPath::IsConstantTime => {},
            ConstantTimeResultForPath::NotConstantTime { .. } => {},
            ConstantTimeResultForPath::OtherError { full_message, .. } => {
                panic!("Encountered an unexpected error: {}", full_message);
            }
        }
    }
    // If we get here, there are no `OtherError`s, so just check for the ct violation we're interested in
    let _ = res.first_ct_violation().expect("Expected a ct violation but didn't get one");
}

#[test]
fn ct_simple() {
    init_logging();
    let project = get_project();
    let result = check_for_ct_violation_in_inputs("ct_simple", &project, Config::default(), true);
    assert_no_ct_violation(result);
}

#[test]
fn ct_simple2() {
    init_logging();
    let project = get_project();
    let result = check_for_ct_violation_in_inputs("ct_simple2", &project, Config::default(), true);
    assert_no_ct_violation(result);
}

#[test]
fn notct_branch() {
    init_logging();
    let project = get_project();
    let result = check_for_ct_violation_in_inputs("notct_branch", &project, Config::default(), true);
    assert_is_ct_violation(result);
}

#[test]
fn notct_mem() {
    init_logging();
    let project = get_project();
    let result = check_for_ct_violation_in_inputs("notct_mem", &project, Config::default(), true);
    assert_is_ct_violation(result);
}

#[test]
fn notct_truepath() {
    init_logging();
    let project = get_project();
    let result = check_for_ct_violation(
        "notct_truepath",
        &project,
        iterator_length_three(AbstractData::sec_i32(), AbstractData::sec_i32(), AbstractData::pub_i32(AbstractValue::Unconstrained)),
        &StructDescriptions::new(),
        Config::default(),
        true,
    );
    assert_is_ct_violation(result);
}

#[test]
fn notct_falsepath() {
    init_logging();
    let project = get_project();
    let result = check_for_ct_violation(
        "notct_falsepath",
        &project,
        iterator_length_three(AbstractData::sec_i32(), AbstractData::sec_i32(), AbstractData::pub_i32(AbstractValue::Unconstrained)),
        &StructDescriptions::new(),
        Config::default(),
        true,
    );
    assert_is_ct_violation(result);
}

#[test]
fn two_ct_violations() {
    init_logging();
    let project = get_project();
    // should report two violations and one path without a violation
    let result = check_for_ct_violation(
        "two_ct_violations",
        &project,
        iterator_length_three(AbstractData::sec_i32(), AbstractData::sec_i32(), AbstractData::pub_i32(AbstractValue::Unconstrained)),
        &StructDescriptions::new(),
        Config::default(),
        true,
    );
    let path_stats = result.path_statistics();
    assert_eq!(path_stats.num_ct_paths, 1, "Expected exactly one 'passing' path, but found {}", path_stats.num_ct_paths);
    assert_eq!(path_stats.num_ct_violations, 2, "Expected exactly two ct violations, but found {}", path_stats.num_ct_violations);
    assert_eq!(result.path_results.len(), 3, "Encountered an unexpected error: {}",
        result.path_results.iter().find_map(|res| match res {
            ConstantTimeResultForPath::IsConstantTime => None,
            ConstantTimeResultForPath::NotConstantTime { .. } => None,
            ConstantTimeResultForPath::OtherError { full_message, .. } => Some(full_message),
        }).expect("Expected to find a non-ct-violation error here, but didn't")
    );

    // with keep_going = false, we should get only one violation
    let result = check_for_ct_violation(
        "two_ct_violations",
        &project,
        iterator_length_three(AbstractData::sec_i32(), AbstractData::sec_i32(), AbstractData::pub_i32(AbstractValue::Unconstrained)),
        &StructDescriptions::new(),
        Config::default(),
        false,
    );
    let path_stats = result.path_statistics();
    assert_eq!(path_stats.num_ct_violations, 1, "Expected exactly one ct violation, but found {}", path_stats.num_ct_violations);
    for res in &result.path_results {
         match res {
            ConstantTimeResultForPath::IsConstantTime => {},
            ConstantTimeResultForPath::NotConstantTime { .. } => {},
            ConstantTimeResultForPath::OtherError { full_message, .. } => {
                panic!("Encountered an unexpected error: {}", full_message);
            },
         }
    }
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
    let result = check_for_ct_violation("ct_onearg", &project, publicx_secrety, &StructDescriptions::new(), Config::default(), true);
    assert_no_ct_violation(result);
    let result = check_for_ct_violation("ct_onearg", &project, secretx_publicy, &StructDescriptions::new(), Config::default(), true);
    assert_is_ct_violation(result);
}

#[test]
fn ct_secrets() {
    init_logging();
    let project = get_project();
    let arg = iterator_length_one(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::sec_i32(), 100)),
    );
    let result = check_for_ct_violation("ct_secrets", &project, arg, &StructDescriptions::new(), Config::default(), true);
    assert_no_ct_violation(result);
}

#[test]
fn notct_secrets() {
    init_logging();
    let project = get_project();
    let arg = iterator_length_one(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::sec_i32(), 100)),
    );
    let result = check_for_ct_violation("notct_secrets", &project, arg, &StructDescriptions::new(), Config::default(), true);
    assert_is_ct_violation(result);
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
    let result = check_for_ct_violation("ct_struct", &project, args, &StructDescriptions::new(), Config::default(), true);
    assert_no_ct_violation(result);
    // now check again, using `default()` and `StructDescriptions`
    let args = iterator_length_two(
        AbstractData::default(),
        AbstractData::default(),
    );
    let sd = iterator_length_one(("struct.PartiallySecret".to_owned(), struct_partially_secret())).into_iter().collect();
    let result = check_for_ct_violation("ct_struct", &project, args, &sd, Config::default(), true);
    assert_no_ct_violation(result);
}

#[test]
fn notct_struct() {
    init_logging();
    let project = get_project();
    let args = iterator_length_two(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(struct_partially_secret()),
    );
    let result = check_for_ct_violation("notct_struct", &project, args, &StructDescriptions::new(), Config::default(), true);
    assert_is_ct_violation(result);
    // now check again, using `default()` and `StructDescriptions`
    let args = iterator_length_two(
        AbstractData::default(),
        AbstractData::default(),
    );
    let sd = iterator_length_one(("struct.PartiallySecret".to_owned(), struct_partially_secret())).into_iter().collect();
    let result = check_for_ct_violation("notct_struct", &project, args, &sd, Config::default(), true);
    assert_is_ct_violation(result);
}

#[test]
fn notct_maybenull_null() {
    init_logging();
    let project = get_project();
    let args = iterator_length_three(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_maybe_null_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(struct_partially_secret()),
    );
    let result = check_for_ct_violation("notct_maybenull_null", &project, args, &StructDescriptions::new(), Config::default(), true);
    assert_is_ct_violation(result);
}

#[test]
fn notct_maybenull_notnull() {
    init_logging();
    let project = get_project();
    let args = iterator_length_three(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_maybe_null_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(struct_partially_secret()),
    );
    let result = check_for_ct_violation("notct_maybenull_notnull", &project, args, &StructDescriptions::new(), Config::default(), true);
    assert_is_ct_violation(result);
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
    let result = check_for_ct_violation("ct_doubleptr", &project, iterator_length_one(ptr_to_ptr_to_secrets()), &StructDescriptions::new(), Config::default(), true);
    assert_no_ct_violation(result);
}

#[test]
fn notct_doubleptr() {
    init_logging();
    let project = get_project();
    let result = check_for_ct_violation("notct_doubleptr", &project, iterator_length_one(ptr_to_ptr_to_secrets()), &StructDescriptions::new(), Config::default(), true);
    assert_is_ct_violation(result);
}

#[test]
fn ct_struct_voidptr() {
    init_logging();
    let project = get_project();
    let args = iterator_length_two(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(AbstractData::void_override(None, struct_partially_secret())),
    );
    let result = check_for_ct_violation("ct_struct_voidptr", &project, args, &StructDescriptions::new(), Config::default(), true);
    assert_no_ct_violation(result);
}

#[test]
fn notct_struct_voidptr() {
    init_logging();
    let project = get_project();
    let args = iterator_length_two(
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(AbstractData::void_override(None, struct_partially_secret())),
    );
    let result = check_for_ct_violation("notct_struct_voidptr", &project, args, &StructDescriptions::new(), Config::default(), true);
    assert_is_ct_violation(result);
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
    let result = check_for_ct_violation("indirectly_recursive_struct", &project, args, &sd, Config::default(), true);
    assert_is_ct_violation(result);
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
    let result = check_for_ct_violation("related_args", &project, args, &StructDescriptions::new(), Config::default(), true);
    assert_no_ct_violation(result);

    // but if we don't have the constraint, then there should be a violation
    let args = iterator_length_three(
        AbstractData::pub_i32(AbstractValue::Range(0, 20)),
        AbstractData::default(),
        AbstractData::sec_i32(),
    );
    let result = check_for_ct_violation("related_args", &project, args, &StructDescriptions::new(), Config::default(), true);
    assert_is_ct_violation(result);
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
    let result = check_for_ct_violation("struct_related_fields", &project, args, &StructDescriptions::new(), Config::default(), true);
    assert_no_ct_violation(result);

    // but if we don't have the constraint, then there should be a violation
    let args = iterator_length_one(AbstractData::pub_pointer_to(AbstractData::_struct("StructWithRelatedFields", vec![
        AbstractData::pub_i32(AbstractValue::Range(0, 20)),
        AbstractData::default(),
        AbstractData::sec_i32(),
    ])));
    let result = check_for_ct_violation("struct_related_fields", &project, args, &StructDescriptions::new(), Config::default(), true);
    assert_is_ct_violation(result);
}
