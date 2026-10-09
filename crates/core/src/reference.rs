//! Variable references: `{{name}}`.
//!
//! This module holds the one rule that decides whether a query parameter or
//! path variable value may be written to a committed file (D-021, D-024):
//! after trimming, the whole value must be exactly one `{{name}}`. Spaces
//! inside the braces are allowed. Anything else is a literal and stays local.

/// Returns `true` if `name` is a valid variable name: one or more of
/// `A-Z a-z 0-9 _ . -`.
pub fn is_valid_variable_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// Returns `true` if `name` is a valid path variable name:
/// `[A-Za-z_][A-Za-z0-9_]*`.
pub fn is_valid_path_variable_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

/// If `value` is exactly one variable reference, returns the variable name.
///
/// `"{{token}}"` and `"  {{ token }} "` give `Some("token")`. A value with
/// any literal text around the reference, such as `"{{a}}-1"` or
/// `"{{a}}{{b}}"`, gives `None`.
pub fn reference_name(value: &str) -> Option<&str> {
    let inner = value.trim().strip_prefix("{{")?.strip_suffix("}}")?;
    let name = inner.trim();
    is_valid_variable_name(name).then_some(name)
}

/// The form in which a reference to `name` is written to files: `{{name}}`.
pub fn canonical_reference(name: &str) -> String {
    format!("{{{{{name}}}}}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_reference_is_recognized() {
        assert_eq!(reference_name("{{token}}"), Some("token"));
        assert_eq!(reference_name("{{api.base-url_2}}"), Some("api.base-url_2"));
    }

    #[test]
    fn spaces_around_and_inside_the_braces_are_allowed() {
        assert_eq!(reference_name("  {{token}}  "), Some("token"));
        assert_eq!(reference_name("{{ token }}"), Some("token"));
        assert_eq!(reference_name("\t{{  token\t}}\n"), Some("token"));
    }

    #[test]
    fn anything_else_is_a_literal() {
        for literal in [
            "",
            " ",
            "1234",
            "it",
            "{{a}}-1",
            "v{{a}}",
            "{{a}}{{b}}",
            "{{a}} {{b}}",
            "{{}}",
            "{{ }}",
            "{{a b}}",
            "{{a:b}}",
            "{{{a}}}",
            "{a}",
            "{{a}",
            "{a}}",
            "{{a}}}",
            "{ {a} }",
            "{{é}}",
        ] {
            assert_eq!(reference_name(literal), None, "{literal:?}");
        }
    }

    #[test]
    fn canonical_form_has_no_spaces() {
        assert_eq!(canonical_reference("token"), "{{token}}");
        assert_eq!(reference_name(&canonical_reference("a.b")), Some("a.b"));
    }

    #[test]
    fn path_variable_names() {
        for ok in ["id", "_id", "petId", "a1_b2"] {
            assert!(is_valid_path_variable_name(ok), "{ok:?}");
        }
        for bad in ["", "1a", "a-b", "a.b", "é", "a b"] {
            assert!(!is_valid_path_variable_name(bad), "{bad:?}");
        }
    }
}
