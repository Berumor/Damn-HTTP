//! Environment files: `environments/<slug>.yaml`.

use super::common::{header, open, read_variables, variables_value};
use super::emit::{Value, render};
use super::error::FormatError;
use super::tree::parse;
use crate::model::Environment;

/// An environment file as loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentFile {
    pub environment: Environment,
    /// Names of secret variables that have a value in the file. That can
    /// only come from a hand edit, and the value is already in the
    /// repository history. The value is kept in `environment`; the next
    /// save writes the file without it.
    pub secret_values_in_file: Vec<String>,
}

/// Writes an environment file. Secret variables are written as a name and
/// `secret: true`, never with their value (D-019).
pub fn write_environment(environment: &Environment) -> String {
    let mut map = header(Some(environment.id.to_string()), "environment");
    map.put("name", Value::str(&environment.name))
        .put("variables", variables_value(&environment.variables));
    render(&map.build())
}

/// Reads an environment file. Only the structure is checked; call
/// [`Environment::validate`] for the rest.
pub fn read_environment(text: &str) -> Result<EnvironmentFile, FormatError> {
    let root = parse(text)?;
    let mut map = open(&root, "environment")?;
    let id = map.required_parsed(
        "id",
        "not a valid environment id (expected a lowercase, hyphenated UUID)",
        |s| s.parse().ok(),
    )?;
    let name = map.required_string("name")?;
    let (variables, secret_values_in_file) = read_variables(&mut map)?;
    map.finish()?;
    Ok(EnvironmentFile {
        environment: Environment {
            id,
            name,
            variables,
        },
        secret_values_in_file,
    })
}
