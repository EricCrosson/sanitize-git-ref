use sanitize_git_ref::sanitize_git_ref_onelevel;

fn main() {
    let unsafe_branch_name = "feature/my..branch@{123}";
    let safe_branch_name = sanitize_git_ref_onelevel(unsafe_branch_name);
    assert_eq!(safe_branch_name, "feature/my.-branch-123}");
    println!(
        "Sanitized '{}' to '{}'",
        unsafe_branch_name, safe_branch_name
    );
}
