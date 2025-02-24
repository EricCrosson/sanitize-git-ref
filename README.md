# sanitize-git-ref

_sanitize-git-ref_ sanitizes Git reference names (branches, tags, etc.) according to Git's reference naming rules.

## Install

In your `Cargo.toml`:

```toml
[dependencies]
sanitize-git-ref = "1"
```

## Use

```rust
use sanitize_git_ref::sanitize_git_ref_onelevel;

fn main() {
    let unsafe_branch_name = "feature/my..branch@{123}";
    let safe_branch_name = sanitize_git_ref_onelevel(unsafe_branch_name);
    assert_eq!(safe_branch_name, "feature/my.-branch-123}");
}
```

## Rules Enforced

The library enforces Git's reference naming rules, including:

- No ASCII control characters
- No spaces
- No special characters: `~`, `^`, `:`, `?`, `*`, `[`, `\`, `@`
- No consecutive dots (`..`)
- No consecutive slashes (`//`)
- No components starting with `.` or ending with `.lock`
- No `@{` sequences
- No leading/trailing slashes or dots

Invalid characters are converted to hyphens, and multiple consecutive hyphens are collapsed into a single hyphen.

## License

Licensed under either of [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0) or [MIT license](https://opensource.org/licenses/MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
