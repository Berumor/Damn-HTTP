//! Errors from reading a file.

/// Why a file could not be read, and where.
///
/// Messages name keys and positions, never values: a value in a file may be
/// a secret or a local literal pasted by hand.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("line {line}, column {column}: {kind}")]
pub struct FormatError {
    /// 1-based.
    pub line: usize,
    /// 1-based.
    pub column: usize,
    pub kind: FormatErrorKind,
}

/// `path` names the place in the file, such as `query[1].enabled`; it is
/// empty for the top level.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FormatErrorKind {
    #[error("not valid YAML: {0}")]
    Syntax(String),
    #[error("the file is empty")]
    Empty,
    #[error("{0} are not supported in Damn HTTP files")]
    Unsupported(&'static str),
    #[error("the key `{0}` appears more than once")]
    DuplicateKey(String),
    #[error("unknown key `{key}`{}", at(path))]
    UnknownKey { path: String, key: String },
    #[error("missing key `{key}`{}", at(path))]
    MissingKey { path: String, key: String },
    #[error("expected {expected}{}", at(path))]
    WrongType {
        path: String,
        expected: &'static str,
    },
    #[error("{problem}{}", at(path))]
    InvalidValue { path: String, problem: &'static str },
    #[error("this is a `{found}` file, expected `{expected}`")]
    WrongKind {
        expected: &'static str,
        found: String,
    },
    #[error(
        "the file has format version {found}, this app reads up to version {supported}; update the app to open it"
    )]
    NewerVersion { found: u64, supported: u64 },
}

fn at(path: &str) -> String {
    if path.is_empty() {
        String::new()
    } else {
        format!(" in `{path}`")
    }
}
