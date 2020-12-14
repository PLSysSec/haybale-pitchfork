use haybale_pitchfork::*;
use std::path::Path;

fn init_logging() {
    // since our tests will run with `progress_updates == false`,
    // we are responsible for capturing log messages ourselves.
    let _ = env_logger::builder().is_test(true).try_init();
}

fn get_project() -> Project {
    Project::from_bc_path(&Path::new("tests/bcfiles/ct.bc"))
        .unwrap_or_else(|e| panic!("Failed to create project: {}", e))
}

fn pitchfork_config() -> PitchforkConfig {
    let mut pconfig = PitchforkConfig::default();
    pconfig.keep_going = KeepGoing::StopPerPath;
    pconfig.dump_errors = false;
    pconfig.progress_updates = false;
    pconfig
}

fn assert_no_ct_violation(res: FunctionResult) {
    if let Some(error) = res.first_error() {
        match error {
            PathResult::PathComplete => panic!("first_error should return an error, not a PathComplete"),
            PathResult::Error { full_message, .. } => panic!("Encountered an unexpected error:\n  {}", full_message),
        }
    }
    match res.ct_violations.get(0) {
        None => {},  // pass
        Some(CTViolation { msg, .. }) =>
            panic!("Expected no ct violation, but found one:\n  {}", msg),
    }
}

fn assert_is_ct_violation(res: FunctionResult) {
    // we check for errors first, and fail if any are encountered,
    // even if there was also a ct violation reported
    if let Some(error) = res.first_error() {
        match error {
            PathResult::PathComplete => panic!("first_error should return an error, not a PathComplete"),
            PathResult::Error { full_message, .. } => panic!("Encountered an unexpected error:\n  {}", full_message),
        }
    }
    // If we get here, there are no errors, so just check for the ct violation we're interested in
    let _ = res.ct_violations.get(0).expect("Expected a ct violation but didn't get one");
}

#[test]
fn ct_simple() {
    init_logging();
    let project = get_project();
    let result = check_for_ct_violation_in_inputs("ct_simple", &project, Config::default(), &pitchfork_config());
    assert_no_ct_violation(result);
}

#[test]
fn ct_simple2() {
    init_logging();
    let project = get_project();
    let result = check_for_ct_violation_in_inputs("ct_simple2", &project, Config::default(), &pitchfork_config());
    assert_no_ct_violation(result);
}

#[test]
fn notct_branch() {
    init_logging();
    let project = get_project();
    let result = check_for_ct_violation_in_inputs("notct_branch", &project, Config::default(), &pitchfork_config());
    assert_is_ct_violation(result);
}

#[test]
fn notct_mem() {
    init_logging();
    let project = get_project();
    let result = check_for_ct_violation_in_inputs("notct_mem", &project, Config::default(), &pitchfork_config());
    assert_is_ct_violation(result);
}

#[test]
fn notct_truepath() {
    init_logging();
    let project = get_project();
    let result = check_for_ct_violation(
        "notct_truepath",
        &project,
        Some(vec![AbstractData::sec_i32(), AbstractData::sec_i32(), AbstractData::pub_i32(AbstractValue::Unconstrained)]),
        &StructDescriptions::new(),
        Config::default(),
        &pitchfork_config(),
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
        Some(vec![AbstractData::sec_i32(), AbstractData::sec_i32(), AbstractData::pub_i32(AbstractValue::Unconstrained)]),
        &StructDescriptions::new(),
        Config::default(),
        &pitchfork_config(),
    );
    assert_is_ct_violation(result);
}

#[test]
fn two_ct_violations() {
    init_logging();
    let project = get_project();
    // should report a total of two violations on a total of three paths
    let result = check_for_ct_violation(
        "two_ct_violations",
        &project,
        Some(vec![AbstractData::sec_i32(), AbstractData::sec_i32(), AbstractData::pub_i32(AbstractValue::Unconstrained)]),
        &StructDescriptions::new(),
        Config::default(),
        &pitchfork_config(),
    );
    let path_stats = result.path_statistics();
    assert_eq!(path_stats.num_complete, 3, "Expected exactly three completed paths, but found {}", path_stats.num_complete);
    assert_eq!(path_stats.num_ct_violations, 2, "Expected exactly two ct violations, but found {}", path_stats.num_ct_violations);
    assert_eq!(result.path_results.len(), 3, "Encountered an unexpected error: {}",
        match result.first_error() {
            None => panic!("Expected to find an error here, but didn't"),
            Some(PathResult::PathComplete) => panic!("first_error should return an error, not a PathComplete"),
            Some(PathResult::Error { full_message, .. }) => full_message,
        }
    );

    // with KeepGoing::Stop, we should get only one violation
    let mut pitchfork_config = pitchfork_config();
    pitchfork_config.keep_going = KeepGoing::Stop;
    let result = check_for_ct_violation(
        "two_ct_violations",
        &project,
        Some(vec![AbstractData::sec_i32(), AbstractData::sec_i32(), AbstractData::pub_i32(AbstractValue::Unconstrained)]),
        &StructDescriptions::new(),
        Config::default(),
        &pitchfork_config,
    );
    let path_stats = result.path_statistics();
    assert_eq!(path_stats.num_ct_violations, 1, "Expected exactly one ct violation, but found {}", path_stats.num_ct_violations);
    for res in &result.path_results {
         match res {
            PathResult::PathComplete => {},
            PathResult::Error { full_message, .. } => {
                panic!("Encountered an unexpected error: {}", full_message);
            },
         }
    }
}

#[test]
fn ct_onearg() {
    init_logging();
    let project = get_project();
    let publicx_secrety = vec![
        AbstractData::pub_i32(AbstractValue::Unconstrained),
        AbstractData::sec_i32(),
    ];
    let secretx_publicy = vec![
        AbstractData::sec_i32(),
        AbstractData::pub_i32(AbstractValue::Unconstrained),
    ];
    let result = check_for_ct_violation("ct_onearg", &project, Some(publicx_secrety), &StructDescriptions::new(), Config::default(), &pitchfork_config());
    assert_no_ct_violation(result);
    let result = check_for_ct_violation("ct_onearg", &project, Some(secretx_publicy), &StructDescriptions::new(), Config::default(), &pitchfork_config());
    assert_is_ct_violation(result);
}

#[test]
fn ct_secrets() {
    init_logging();
    let project = get_project();
    let arg = vec![
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::sec_i32(), 100)),
    ];
    let result = check_for_ct_violation("ct_secrets", &project, Some(arg), &StructDescriptions::new(), Config::default(), &pitchfork_config());
    assert_no_ct_violation(result);
}

#[test]
fn notct_secrets() {
    init_logging();
    let project = get_project();
    let arg = vec![
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::sec_i32(), 100)),
    ];
    let result = check_for_ct_violation("notct_secrets", &project, Some(arg), &StructDescriptions::new(), Config::default(), &pitchfork_config());
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
    let args = vec![
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(struct_partially_secret()),
    ];
    let result = check_for_ct_violation("ct_struct", &project, Some(args), &StructDescriptions::new(), Config::default(), &pitchfork_config());
    assert_no_ct_violation(result);
    // now check again, using `default()` and `StructDescriptions`
    let args = vec![
        AbstractData::default(),
        AbstractData::default(),
    ];
    let sd = std::iter::once(("struct.PartiallySecret".to_owned(), struct_partially_secret())).collect();
    let result = check_for_ct_violation("ct_struct", &project, Some(args), &sd, Config::default(), &pitchfork_config());
    assert_no_ct_violation(result);
    // now check again, using `args==None` and `StructDescriptions`
    let result = check_for_ct_violation("ct_struct", &project, None, &sd, Config::default(), &pitchfork_config());
    assert_no_ct_violation(result);
}

#[test]
fn notct_struct() {
    init_logging();
    let project = get_project();
    let args = vec![
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(struct_partially_secret()),
    ];
    let result = check_for_ct_violation("notct_struct", &project, Some(args), &StructDescriptions::new(), Config::default(), &pitchfork_config());
    assert_is_ct_violation(result);
    // now check again, using `default()` and `StructDescriptions`
    let args = vec![
        AbstractData::default(),
        AbstractData::default(),
    ];
    let sd = std::iter::once(("struct.PartiallySecret".to_owned(), struct_partially_secret())).collect();
    let result = check_for_ct_violation("notct_struct", &project, Some(args), &sd, Config::default(), &pitchfork_config());
    assert_is_ct_violation(result);
    // now check again, using `args==None` and `StructDescriptions`
    let result = check_for_ct_violation("notct_struct", &project, None, &sd, Config::default(), &pitchfork_config());
    assert_is_ct_violation(result);
}

#[test]
fn notct_maybenull_null() {
    init_logging();
    let project = get_project();
    let args = vec![
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_maybe_null_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(struct_partially_secret()),
    ];
    let result = check_for_ct_violation("notct_maybenull_null", &project, Some(args), &StructDescriptions::new(), Config::default(), &pitchfork_config());
    assert_is_ct_violation(result);
}

#[test]
fn notct_maybenull_notnull() {
    init_logging();
    let project = get_project();
    let args = vec![
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_maybe_null_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(struct_partially_secret()),
    ];
    let result = check_for_ct_violation("notct_maybenull_notnull", &project, Some(args), &StructDescriptions::new(), Config::default(), &pitchfork_config());
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
    let result = check_for_ct_violation("ct_doubleptr", &project, Some(vec![ptr_to_ptr_to_secrets()]), &StructDescriptions::new(), Config::default(), &pitchfork_config());
    assert_no_ct_violation(result);
}

#[test]
fn notct_doubleptr() {
    init_logging();
    let project = get_project();
    let result = check_for_ct_violation("notct_doubleptr", &project, Some(vec![ptr_to_ptr_to_secrets()]), &StructDescriptions::new(), Config::default(), &pitchfork_config());
    assert_is_ct_violation(result);
}

#[test]
fn ct_struct_voidptr() {
    init_logging();
    let project = get_project();
    let args = vec![
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(AbstractData::void_override(None, struct_partially_secret())),
    ];
    let result = check_for_ct_violation("ct_struct_voidptr", &project, Some(args), &StructDescriptions::new(), Config::default(), &pitchfork_config());
    assert_no_ct_violation(result);
}

#[test]
fn notct_struct_voidptr() {
    init_logging();
    let project = get_project();
    let args = vec![
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(AbstractData::void_override(None, struct_partially_secret())),
    ];
    let result = check_for_ct_violation("notct_struct_voidptr", &project, Some(args), &StructDescriptions::new(), Config::default(), &pitchfork_config());
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
    let args = vec![
        AbstractData::pub_pointer_to(AbstractData::array_of(AbstractData::pub_i32(AbstractValue::Unconstrained), 100)),
        AbstractData::pub_pointer_to(struct_parent_secretx()),
    ];
    let sd = std::iter::once(("struct.Child".to_owned(), struct_child())).collect();
    let result = check_for_ct_violation("indirectly_recursive_struct", &project, Some(args), &sd, Config::default(), &pitchfork_config());
    assert_is_ct_violation(result);
}

#[test]
fn related_args() {
    init_logging();
    let project = get_project();
    let args = vec![
        AbstractData::pub_i32(AbstractValue::named("length", AbstractValue::Range(0, 20))),
        AbstractData::pub_i32(AbstractValue::UnsignedLessThan("length".to_owned())),
        AbstractData::sec_i32(),
    ];
    let result = check_for_ct_violation("related_args", &project, Some(args), &StructDescriptions::new(), Config::default(), &pitchfork_config());
    assert_no_ct_violation(result);

    // but if we don't have the constraint, then there should be a violation
    let args = vec![
        AbstractData::pub_i32(AbstractValue::Range(0, 20)),
        AbstractData::default(),
        AbstractData::sec_i32(),
    ];
    let result = check_for_ct_violation("related_args", &project, Some(args), &StructDescriptions::new(), Config::default(), &pitchfork_config());
    assert_is_ct_violation(result);
}

#[test]
fn struct_related_fields() {
    init_logging();
    let project = get_project();
    let args = vec![AbstractData::pub_pointer_to(AbstractData::_struct("StructWithRelatedFields", vec![
        AbstractData::pub_i32(AbstractValue::named("length", AbstractValue::Range(0, 20))),
        AbstractData::pub_i32(AbstractValue::UnsignedLessThan("length".to_owned())),
        AbstractData::sec_i32(),
    ]))];
    let result = check_for_ct_violation("struct_related_fields", &project, Some(args), &StructDescriptions::new(), Config::default(), &pitchfork_config());
    assert_no_ct_violation(result);

    // but if we don't have the constraint, then there should be a violation
    let args = vec![AbstractData::pub_pointer_to(AbstractData::_struct("StructWithRelatedFields", vec![
        AbstractData::pub_i32(AbstractValue::Range(0, 20)),
        AbstractData::default(),
        AbstractData::sec_i32(),
    ]))];
    let result = check_for_ct_violation("struct_related_fields", &project, Some(args), &StructDescriptions::new(), Config::default(), &pitchfork_config());
    assert_is_ct_violation(result);
}
