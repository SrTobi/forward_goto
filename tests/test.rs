#[macro_use]
extern crate forward_goto;



#[rewrite_forward_goto]
#[allow(unreachable_code)]
fn test_easy_method() -> Vec<&'static str>{
    let mut result = vec!["begin"];

    forward_goto!('test);

    result.push("should not happen");

    forward_label!('test);

    result.push("end");
    result
}

#[test]
fn test_easy() {
    assert_eq!(test_easy_method(),
        vec![
            "begin",
            "end",
        ]
    );
}



#[rewrite_forward_goto]
fn test_if_method(b: bool) -> Vec<&'static str>{
    let mut result = vec!["begin"];

    if !b {
        forward_goto!('test);
    }
    result.push("happens if b");

    forward_label!('test);

    result.push("end");
    result
}

#[test]
fn test_if() {
    assert_eq!(test_if_method(true),
        vec![
            "begin",
            "happens if b",
            "end",
        ]
    );

    assert_eq!(test_if_method(false),
        vec![
            "begin",
            "end",
        ]
    );
}


enum Three {
    A, B, C
}

#[rewrite_forward_goto]
fn test_jump_into_if_method(three: Three, b: bool) -> Vec<&'static str>{
    let mut result = vec!["begin"];

    match three {
        Three::A => forward_goto!('test),
        Three::B => forward_goto!('test_2),
        Three::C => (),
    }
    result.push("in between");

    if b {
        result.push("before label");
        forward_label!('test);
        result.push("after label");
    } else {
        result.push("before label 2");
        forward_label!('test_2);
        result.push("after label 2");
    }

    result.push("end");
    result
}

#[test]
fn test_jump_into_if() {
    // jump to test
    assert_eq!(test_jump_into_if_method(Three::A, true),
        vec![
            "begin",
            "after label",
            "end",
        ]
    );

    assert_eq!(test_jump_into_if_method(Three::A, false),
        vec![
            "begin",
            "after label",
            "end",
        ]
    );

    // jump to test_2
    assert_eq!(test_jump_into_if_method(Three::B, true),
    vec![
        "begin",
        "after label 2",
        "end",
        ]
    );

    assert_eq!(test_jump_into_if_method(Three::B, false),
        vec![
            "begin",
            "after label 2",
            "end",
        ]
    );

    // don't jump
    assert_eq!(test_jump_into_if_method(Three::C, true),
        vec![
            "begin",
            "in between",
            "before label",
            "after label",
            "end",
        ]
    );

    assert_eq!(test_jump_into_if_method(Three::C, false),
        vec![
            "begin",
            "in between",
            "before label 2",
            "after label 2",
            "end",
        ]
    );
}



#[rewrite_forward_goto]
fn test_jump_into_double_if_method(three: Three, b1: bool, b2: bool) -> Vec<&'static str>{
    let mut result = vec!["begin"];

    match three {
        Three::A => forward_goto!('test),
        Three::B => forward_goto!('test_2),
        Three::C => (),
    }
    result.push("in between");

    if b1 {
        if b2 {
            result.push("before label");
            forward_label!('test);
            result.push("after label");
        } else {
            result.push("before label 2");
            forward_label!('test_2);
            result.push("after label 2");
        }
        result.push("after after");
    } else {
        result.push("alternative");
    }

    result.push("end");
    result
}

#[test]
fn test_jump_into_double_if() {
    // jump to test
    assert_eq!(test_jump_into_double_if_method(Three::A, true, true),
        vec![
            "begin",
            "after label",
            "after after",
            "end",
        ]
    );

    assert_eq!(test_jump_into_double_if_method(Three::A, true, false),
        vec![
            "begin",
            "after label",
            "after after",
            "end",
        ]
    );

    assert_eq!(test_jump_into_double_if_method(Three::A, false, true),
        vec![
            "begin",
            "after label",
            "after after",
            "end",
        ]
    );

    assert_eq!(test_jump_into_double_if_method(Three::A, false, false),
        vec![
            "begin",
            "after label",
            "after after",
            "end",
        ]
    );

    // jump to test_2
    assert_eq!(test_jump_into_double_if_method(Three::B, true, true),
    vec![
        "begin",
        "after label 2",
        "after after",
        "end",
        ]
    );

    assert_eq!(test_jump_into_double_if_method(Three::B, true, false),
    vec![
        "begin",
        "after label 2",
        "after after",
        "end",
        ]
    );

    assert_eq!(test_jump_into_double_if_method(Three::B, false, true),
        vec![
            "begin",
            "after label 2",
            "after after",
            "end",
        ]
    );

    assert_eq!(test_jump_into_double_if_method(Three::B, false, false),
        vec![
            "begin",
            "after label 2",
            "after after",
            "end",
        ]
    );

    // don't jump
    assert_eq!(test_jump_into_double_if_method(Three::C, true, true),
        vec![
            "begin",
            "in between",
            "before label",
            "after label",
            "after after",
            "end",
        ]
    );

    assert_eq!(test_jump_into_double_if_method(Three::C, true, false),
        vec![
            "begin",
            "in between",
            "before label 2",
            "after label 2",
            "after after",
            "end",
        ]
    );

    assert_eq!(test_jump_into_double_if_method(Three::C, false, true),
        vec![
            "begin",
            "in between",
            "alternative",
            "end",
        ]
    );

    assert_eq!(test_jump_into_double_if_method(Three::C, false, false),
        vec![
            "begin",
            "in between",
            "alternative",
            "end",
        ]
    );
}