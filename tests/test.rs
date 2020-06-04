use forward_goto::*;


#[rewrite_forward_goto]
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

#[derive(Eq, PartialEq)]
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




#[rewrite_forward_goto]
fn test_multi_goto_method(three: Three) -> Vec<&'static str>{
    let mut result = vec!["begin"];

    if three == Three::A {
        result.push("before first goto");
        forward_goto!('test);
    }

    if three == Three::B {
        result.push("before second goto");
        forward_goto!('test);
    }

    result.push("in between");

    forward_label!('test);

    result.push("end");
    result
}

#[test]
fn test_multi_goto() {
    assert_eq!(test_multi_goto_method(Three::A),
        vec![
            "begin",
            "before first goto",
            "end",
        ]
    );

    assert_eq!(test_multi_goto_method(Three::B),
        vec![
            "begin",
            "before second goto",
            "end",
        ]
    );

    assert_eq!(test_multi_goto_method(Three::C),
        vec![
            "begin",
            "in between",
            "end",
        ]
    );
}



#[rewrite_forward_goto]
fn test_multi_cross_method(three: Three) -> Vec<&'static str>{
    let mut result = vec!["begin"];

    if three == Three::A {
        result.push("before first goto");
        forward_goto!('test);
    }

    if three == Three::B {
        result.push("before second goto");
        forward_goto!('test_2);
    }

    result.push("after ifs");

    forward_label!('test);

    result.push("in between labels");

    forward_label!('test_2);

    result.push("end");
    result
}


#[test]
fn test_multi_cross() {
    assert_eq!(test_multi_cross_method(Three::A),
        vec![
            "begin",
            "before first goto",
            "in between labels",
            "end",
        ]
    );

    assert_eq!(test_multi_cross_method(Three::B),
        vec![
            "begin",
            "before second goto",
            "end",
        ]
    );

    assert_eq!(test_multi_cross_method(Three::C),
        vec![
            "begin",
            "after ifs",
            "in between labels",
            "end",
        ]
    );
}


#[rewrite_forward_goto]
fn test_multi_stack_like_method(three: Three) -> Vec<&'static str>{
    let mut result = vec!["begin"];

    if three == Three::A {
        result.push("before first goto");
        forward_goto!('test);
    }
    
    fn f() {}

    if three == Three::B {
        result.push("before second goto");
        forward_goto!('test_2);
    }

    f();

    result.push("after ifs");

    forward_label!('test_2);

    f();

    result.push("in between labels");

    forward_label!('test);


    result.push("end");
    result
}


#[test]
fn test_multi_stack_like() {
    assert_eq!(test_multi_stack_like_method(Three::A),
        vec![
            "begin",
            "before first goto",
            "end",
        ]
    );

    assert_eq!(test_multi_stack_like_method(Three::B),
        vec![
            "begin",
            "before second goto",
            "in between labels",
            "end",
        ]
    );

    assert_eq!(test_multi_stack_like_method(Three::C),
        vec![
            "begin",
            "after ifs",
            "in between labels",
            "end",
        ]
    );
}


#[rewrite_forward_goto]
fn test_if_merging_method(b: bool) -> Vec<&'static str>{
    let mut result = vec!["begin"];

    if b {
        result.push("before first goto");
        forward_goto!('test);
    }

    
    {
        result.push("before if");

        if b {

            forward_label!('test);

            result.push("after test");
        }

        result.push("after if");
    }

    result.push("end");
    result
}

#[test]
fn test_if_merging() {
    assert_eq!(test_if_merging_method(true),
        vec![
            "begin",
            "before first goto",
            "after test",
            "after if",
            "end",
        ]
    );

    assert_eq!(test_if_merging_method(false),
        vec![
            "begin",
            "before if",
            "after if",
            "end",
        ]
    );
}



#[rewrite_forward_goto]
fn test_jump_in_continuation_method(b: bool) -> Vec<&'static str>{
    let mut result = vec!["begin"];

    if b {
        result.push("before first goto");
        forward_goto!('test);
    }

    
    {
        result.push("before if");

        if b {
            forward_label!('test);

            result.push("after test");

            forward_goto!('test_2);
        }

        result.push("after if");

        forward_label!('test_2);

        result.push("after test_2");
    }

    result.push("end");
    result
}

#[test]
fn test_jump_in_continuation() {
    assert_eq!(test_jump_in_continuation_method(true),
        vec![
            "begin",
            "before first goto",
            "after test",
            "after test_2",
            "end",
        ]
    );

    assert_eq!(test_jump_in_continuation_method(false),
        vec![
            "begin",
            "before if",
            "after if",
            "after test_2",
            "end",
        ]
    );
}


#[rewrite_forward_goto]
fn test_jump_into_match_method(three: Three) -> Vec<&'static str>{
    let mut result = vec!["begin"];

    match three {
        Three::A => {
            forward_goto!('a);
        },
        Three::B => {
            forward_goto!('b);
        },
        Three::C => {
            forward_goto!('c);
        }
    };

    match 1 as i32 {
        1 => {
            forward_label!('a);
            result.push("a");
        },
        2 => {
            forward_label!('b);
            result.push("b");
        },
        _ => {
            forward_label!('c);
            result.push("c");
        },
    };

    result.push("end");
    result
}

#[test]
fn test_jump_into_match() {
    assert_eq!(test_jump_into_match_method(Three::A),
        vec![
            "begin",
            "a",
            "end",
        ]
    );

    assert_eq!(test_jump_into_match_method(Three::B),
        vec![
            "begin",
            "b",
            "end",
        ]
    );

    assert_eq!(test_jump_into_match_method(Three::C),
        vec![
            "begin",
            "c",
            "end",
        ]
    );
}



#[rewrite_forward_goto]
fn test_jump_from_match_method(three: Three) -> Vec<&'static str>{
    let mut result = vec!["begin"];

    if three == Three::B {
        forward_goto!('b);
    }

    match three {
        Three::A => {
            forward_goto!('a);
        },
        Three::B => {
            //forward_goto!('b);
        },
        Three::C => {
            forward_goto!('c);
        }
    };

    match 1 as i32 {
        1 => {
            forward_label!('a);
            result.push("a");
        },
        2 => {
            forward_label!('b);
            result.push("b");
        },
        _ => {
            forward_label!('c);
            result.push("c");
        },
    };

    result.push("end");
    result
}

#[test]
fn test_jump_from_match() {
    assert_eq!(test_jump_from_match_method(Three::A),
        vec![
            "begin",
            "a",
            "end",
        ]
    );

    assert_eq!(test_jump_from_match_method(Three::B),
        vec![
            "begin",
            "b",
            "end",
        ]
    );

    assert_eq!(test_jump_from_match_method(Three::C),
        vec![
            "begin",
            "c",
            "end",
        ]
    );
}
