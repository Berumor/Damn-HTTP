//! Problems that make a model value unfit to save.

/// One problem found by a `validate()` method. Validation collects every
/// problem instead of stopping at the first.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    #[error("the name is empty")]
    EmptyName,
    #[error("`{0}` is not a valid HTTP method")]
    InvalidMethod(String),
    #[error("`{0}` is not a valid variable name (use letters, digits, `_`, `.` and `-`)")]
    InvalidVariableName(String),
    #[error("the variable `{0}` is defined more than once")]
    DuplicateVariable(String),
    #[error("the variable `{0}` is marked secret, but only environment variables can be secret")]
    SecretNotAllowed(String),
    #[error(
        "`{0}` is not a valid path variable name (use letters, digits and `_`, not starting with a digit)"
    )]
    InvalidPathVariableName(String),
    #[error("the path variable `{0}` is listed more than once")]
    DuplicatePathVariable(String),
    #[error("the form field `{0}` is a file, which only a multipart body can send")]
    FileInUrlencodedBody(String),
}
