use super::*;

#[test]
fn invalid_declarations_report_the_relevant_contract() {
    for (source, expected) in [
        ("struct Tuple(Shared<u32>);", "named fields"),
        ("struct Unit;", "named fields"),
        ("enum Value { A }", "only supports structs"),
        (
            "struct Value { dep: u32 }",
            "Shared<T>, Option<Shared<T>>, or Vec<Shared<T>>",
        ),
        (
            "struct Value { dep: Option<u32> }",
            "Shared<T>, Option<Shared<T>>, or Vec<Shared<T>>",
        ),
        (
            "struct Value { dep: Shared<u32, u64> }",
            "Shared<T>, Option<Shared<T>>, or Vec<Shared<T>>",
        ),
        (
            "struct Value { #[inject] dep: Shared<u32> }",
            "expected attribute arguments",
        ),
        (
            "struct Value { #[inject()] dep: Shared<u32> }",
            "expected inject",
        ),
        (
            "struct Value { #[inject(other = true)] dep: Shared<u32> }",
            "expected name",
        ),
        (
            "struct Value { #[inject(name = 42)] dep: Shared<u32> }",
            "expected string literal",
        ),
        (
            "struct Value { #[inject(name = \"a\", name = \"b\")] dep: Shared<u32> }",
            "duplicate binding name",
        ),
        (
            "struct Value { #[inject(name = \"a\")] #[inject(name = \"b\")] dep: Shared<u32> }",
            "duplicate binding name",
        ),
        (
            "struct Value { #[inject(name = \"a\")] dep: Vec<Shared<u32>> }",
            "set dependencies cannot have a binding name",
        ),
        (
            "#[inject(name = \"a\")] struct Value { dep: Shared<u32> }",
            "inject is a field attribute",
        ),
    ] {
        let input = syn::parse_str::<DeriveInput>(source).unwrap();
        let error = expand_injectable(&input).unwrap_err().to_string();
        assert!(
            error.contains(expected),
            "{source}: expected {expected:?}, got {error:?}"
        );
    }
}
