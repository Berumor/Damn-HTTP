//! Variables and environments.

use std::collections::HashSet;

use super::id::EnvironmentId;
use super::validation::ValidationError;
use crate::reference::is_valid_variable_name;

/// A named value referenced as `{{name}}`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Variable {
    pub name: String,
    /// For a secret variable this is the locally stored value and is never
    /// written to a committed file (D-019).
    pub value: String,
    /// Only environment variables can be secret.
    pub secret: bool,
    pub description: String,
}

/// A set of variables for one target, such as "dev" or "staging". Shared by
/// every collection in the workspace (D-005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    pub id: EnvironmentId,
    pub name: String,
    pub variables: Vec<Variable>,
}

impl Environment {
    pub fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push(ValidationError::EmptyName);
        }
        validate_variables(&self.variables, true, &mut errors);
        errors
    }
}

/// Checks names, duplicates and, when `allow_secret` is false, the secret flag.
pub(super) fn validate_variables(
    variables: &[Variable],
    allow_secret: bool,
    errors: &mut Vec<ValidationError>,
) {
    let mut seen = HashSet::new();
    for variable in variables {
        if !is_valid_variable_name(&variable.name) {
            errors.push(ValidationError::InvalidVariableName(variable.name.clone()));
        } else if !seen.insert(variable.name.as_str()) {
            errors.push(ValidationError::DuplicateVariable(variable.name.clone()));
        }
        if variable.secret && !allow_secret {
            errors.push(ValidationError::SecretNotAllowed(variable.name.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(name: &str) -> Variable {
        Variable {
            name: name.to_owned(),
            ..Variable::default()
        }
    }

    #[test]
    fn a_well_formed_environment_is_valid() {
        let env = Environment {
            id: EnvironmentId::generate(),
            name: "Staging".to_owned(),
            variables: vec![
                var("baseUrl"),
                Variable {
                    secret: true,
                    ..var("apiToken")
                },
            ],
        };
        assert_eq!(env.validate(), []);
    }

    #[test]
    fn problems_are_all_reported() {
        let env = Environment {
            id: EnvironmentId::generate(),
            name: "  ".to_owned(),
            variables: vec![var("a"), var("a"), var("b c"), var("")],
        };
        assert_eq!(
            env.validate(),
            [
                ValidationError::EmptyName,
                ValidationError::DuplicateVariable("a".to_owned()),
                ValidationError::InvalidVariableName("b c".to_owned()),
                ValidationError::InvalidVariableName(String::new()),
            ]
        );
    }
}
