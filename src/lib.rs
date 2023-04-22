mod error;

use crate::error::SanitizeGitRefError;

struct SanitizeOptions {
    allow_onelevel: bool,
}

/// Rules obtained from [git-check-ref-format].
///
/// This function sanitizes git refs with the assumption that `--allow-onelevel` is true.
///
/// [git-check-ref-format]: https://git-scm.com/docs/git-check-ref-format
pub fn sanitize_git_ref_onelevel(text: &str) -> String {
    let sanitized = sanitize(
        text,
        SanitizeOptions {
            allow_onelevel: true,
        },
    );
    sanitized.expect("Sanitization should always suceed when allow_onelevel is true")
}

/// Replace consecutive occurrences of `target` with hyphens
fn sanitize_consecutive_run(string: String, target: char) -> String {
    let mut current_run = 0;
    string
        .chars()
        .map(|c| {
            match c == target {
                true => current_run += 1,
                false => current_run = 0,
            };
            match current_run < 2 {
                true => c,
                false => '-',
            }
        })
        .collect()
}

/// Remove consecutive occurrences of `target`
fn elide_consecutive_run(mut string: String, target: char) -> String {
    let mut current_run = 0;
    string.retain(|c| {
        match c == target {
            true => current_run += 1,
            false => current_run = 0,
        };
        current_run < 2
    });
    string
}

fn sanitize(text: &str, options: SanitizeOptions) -> Result<String, Box<SanitizeGitRefError>> {
    let SanitizeOptions { allow_onelevel } = options;
    let mut result = text.to_owned();

    // They must contain at least one /. This enforces the presence of a
    // category like heads/, tags/ etc. but the actual names are not restricted.
    // If the --allow-onelevel option is used, this rule is waived.
    if !allow_onelevel {
        if !result.contains('/') {
            return Err(Box::new(SanitizeGitRefError::DoesNotContainForwardSlash));
        }
    }

    // They can include slash / for hierarchical (directory) grouping, but
    // no slash-separated component can begin with a dot . or end with the
    // sequence .lock.
    if result.starts_with('.') {
        result = result.replacen('.', "-", 1);
    }
    result = result.replace("/.", "/-");
    // FIXME: this is overly cautious
    result = result.replace(".lock", "-");

    // They cannot contain a sequence @{.
    result = result.replace("@{", "-");

    result = result
        .chars()
        .map(|c| -> char {
            // They cannot have ASCII control characters (i.e. bytes whose
            // values are lower than \040, or \177 DEL).
            if c.is_ascii_control() {
                return '-';
            }

            // They cannot have space anywhere.
            if c.is_whitespace() {
                return '-';
            }

            match c {
                // They cannot have tilde ~ anywhere.
                '~'
                // They cannot have caret ^ anywhere.
                | '^'

                // They cannot have colon : anywhere.
                | ':'

                // They cannot have question-mark ?, asterisk *, or open bracket
                // [ anywhere. See the --refspec-pattern option below for an
                // exception to this rule.
                | '?'
                | '*'
                | '['

                // They cannot contain a \.
                | '\\'

                // They cannot be the single character @.
                | '@'

                => '-',

                _ => c,
            }
        })
        .collect();

    // They cannot contain multiple consecutive slashes (see the --normalize option below for an exception to this rule)
    result = sanitize_consecutive_run(result, '/');

    // They cannot have two consecutive dots .. anywhere.
    result = sanitize_consecutive_run(result, '.');

    // They cannot begin with a slash / (see the --normalize option below for an exception to this rule)
    while result.starts_with('/') {
        result = result.replacen('/', "-", 1);
    }

    // They cannot end with a dot .
    // They cannot end with a slash / (see the --normalize option below for an exception to this rule)
    while result.ends_with('/') || result.ends_with('.') {
        result.pop();
    }

    // Convert any sequence of multiple hyphens into a single hyphen.
    // We convert invalid characters into hyphens to prevent shrinking the input into an empty string.
    result = elide_consecutive_run(result, '-');

    Ok(result)
}

#[cfg(test)]
mod test {
    use crate::sanitize_git_ref_onelevel;

    use proptest::prelude::*;

    macro_rules! test_does_not_violate_branch_naming_rule {
        ($unit_test:ident, $property_test:ident, $test_of_inclusion:expr, $unsanitized_branch_name:expr) => {
            #[test]
            fn $unit_test() {
                let sanitized_branch_name = sanitize_git_ref_onelevel(&$unsanitized_branch_name);
                assert!(
                    !$test_of_inclusion(&sanitized_branch_name),
                    "Expected unsanitized string {:?} to sanitize to a valid branch name, but {:?} is not a valid branch name",
                    &$unsanitized_branch_name,
                    &sanitized_branch_name
                );
            }

            proptest! {
                #[test]
                fn $property_test(unsanitized_branch_name in any::<String>()) {
                    let sanitized_branch_name = sanitize_git_ref_onelevel(&unsanitized_branch_name);
                    assert!(
                        !$test_of_inclusion(&sanitized_branch_name),
                        "Expected unsanitized string {:?} to sanitize to a valid branch name, but {:?} is not a valid branch name",
                        &unsanitized_branch_name,
                        &sanitized_branch_name
                    );
                }
            }
        };
    }

    // They can include slash / for hierarchical (directory) grouping, but no slash-separated component can begin with a dot.
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_contain_a_slash_separated_component_beginning_with_a_dot,
        proptest_branch_name_does_not_contain_a_slash_separated_component_beginning_with_a_dot,
        |branch_name: &str| -> bool {
            for slash_separated_sequence in branch_name.split("/") {
                if slash_separated_sequence.starts_with(".") {
                    return true;
                }
            }
            false
        },
        "refs/heads/.master"
    );

    // Branch names can include slash / for hierarchical (directory) grouping, but no slash-separated component can end with the sequence .lock.
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_contain_a_slash_separated_component_ending_with_dot_lock,
        proptest_branch_name_does_not_contain_a_slash_separated_component_ending_with_dot_lock,
        |branch_name: &str| -> bool {
            for slash_separated_sequence in branch_name.split("/") {
                if slash_separated_sequence.ends_with(".lock") {
                    return true;
                }
            }
            false
        },
        "refs/heads/master.lock"
    );

    // They must contain at least one /. This enforces the presence of a category like heads/, tags/ etc. but the actual names are not restricted.
    // If the --allow-onelevel option is used, this rule is waived.
    // FIXME: Turn on this test when we implement sanitize_git_ref (sans allow-onelevel)
    // fn has_at_least_one_slash<S: AsRef<str>>(branch_name: S) -> bool {
    //     branch_name.as_ref().contains("/")
    // }

    // #[test]
    // fn branch_name_has_at_least_one_slash() {
    //     assert!(has_at_least_one_slash(sanitize_git_ref_onelevel(
    //         "refs/heads/master"
    //     )))
    // }

    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_contain_two_consecutive_dots,
        proptest_branch_name_does_not_contain_two_consecutive_dots,
        |branch_name: &str| -> bool { branch_name.contains("..") },
        "refs/heads/master..foo"
    );

    // They cannot have ASCII control characters (i.e. bytes whose values are lower than \040, or \177 DEL).
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_contain_a_control_character,
        proptest_branch_name_does_not_contain_a_control_character,
        |branch_name: &str| -> bool { branch_name.contains(|c: char| c.is_ascii_control()) },
        String::from("/refs/heads/master") + std::str::from_utf8(&[039]).unwrap() + "foo"
    );

    // They cannot have space anywhere.
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_contain_a_space,
        proptest_branch_name_does_not_contain_a_space,
        |branch_name: &str| -> bool { branch_name.contains(char::is_whitespace) },
        "/refs/heads/master foo"
    );

    // They cannot have tilde ~ anywhere.
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_contain_a_tilde,
        proptest_branch_name_does_not_contain_a_tilde,
        |branch_name: &str| -> bool { branch_name.contains("?") },
        "/refs/heads/master~foo"
    );

    // They cannot have caret ^ anywhere.
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_contain_a_carat,
        proptest_branch_name_does_not_contain_a_carat,
        |branch_name: &str| -> bool { branch_name.contains("^") },
        "/refs/heads/master^foo"
    );

    // They cannot have colon : anywhere.
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_contain_a_colon,
        proptest_branch_name_does_not_contain_a_colon,
        |branch_name: &str| -> bool { branch_name.contains(":") },
        "/refs/heads/master:foo"
    );

    // They cannot have question-mark ? anywhere. See the --refspec-pattern option below for an exception to this rule.
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_contain_a_question_mark,
        proptest_branch_name_does_not_contain_a_question_mark,
        |branch_name: &str| -> bool { branch_name.starts_with("?") },
        "/refs/heads/master?foo"
    );

    // They cannot have asterisk * anywhere. See the --refspec-pattern option below for an exception to this rule.
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_contain_an_asterisk,
        proptest_branch_name_does_not_contain_an_asterisk,
        |branch_name: &str| -> bool { branch_name.starts_with("*") },
        "/refs/heads/master*foo"
    );

    // They cannot have open bracket [ anywhere. See the --refspec-pattern option below for an exception to this rule.
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_contain_an_open_bracket,
        proptest_branch_name_does_not_contain_an_open_bracket,
        |branch_name: &str| -> bool { branch_name.starts_with("[") },
        "/refs/heads/master[foo"
    );

    // They cannot begin with a slash (/) (see the --normalize option for an exception to this rule)
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_begin_with_a_forward_slash,
        proptest_branch_name_does_not_begin_with_a_forward_slash,
        |branch_name: &str| -> bool { branch_name.starts_with("/") },
        "/refs/heads/master"
    );

    // They cannot begin with a slash (/) (see the --normalize option for an exception to this rule)
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_end_with_a_forward_slash,
        proptest_branch_name_does_not_end_with_a_forward_slash,
        |branch_name: &str| -> bool { branch_name.ends_with("/") },
        "refs/heads/master/"
    );

    // They cannot contain multiple consecutive slashes (see the --normalize option for an exception to this rule)
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_contain_consecutive_forward_slashes,
        proptest_branch_name_does_not_contain_consecutive_forward_slashes,
        |branch_name: &str| -> bool { branch_name.contains("//") },
        "refs/heads/master//all-right"
    );

    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_contain_multiple_consecutive_forward_slashes,
        proptest_branch_name_does_not_contain_multiple_consecutive_forward_slashes,
        |branch_name: &str| -> bool { branch_name.contains("//") },
        "refs/heads/master///all////right"
    );

    // They cannot end with a dot .
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_end_with_dot,
        proptest_branch_name_does_not_end_with_dot,
        |branch_name: &str| -> bool { branch_name.ends_with(".") },
        "refs/heads/master."
    );

    // They cannot contain a sequence @{.
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_contain_ampersand_open_brace,
        proptest_branch_name_does_not_contain_ampersand_open_brace,
        |branch_name: &str| -> bool { branch_name.contains("@{") },
        "refs/heads/master-@{-branch"
    );

    // FIXME: this implementation is too restrictive but I'm not exactly sure of the rules right now.
    // Happy to widen this up if I get more clarity and feel confident we'll avoid false-positives.
    // They cannot be the single character @.
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_contain_ampersand,
        proptest_branch_name_does_not_contain_ampersand,
        |branch_name: &str| -> bool { branch_name.contains("@") },
        "refs/heads/master-@-branch"
    );

    // They cannot contain a \.
    test_does_not_violate_branch_naming_rule!(
        branch_name_does_not_contain_backslash,
        proptest_branch_name_does_not_contain_backslash,
        |branch_name: &str| -> bool { branch_name.contains(r"\") },
        r"refs/heads/master-\-branch"
    );
}
