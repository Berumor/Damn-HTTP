//! Workspace, collections and folders.
//!
//! These types hold each item's own settings. The tree (which folder contains
//! which request) is the directory structure on disk and belongs to the
//! workspace store.

use super::auth::Auth;
use super::order::OrderKey;
use super::validation::ValidationError;
use super::variable::{Variable, validate_variables};

/// A workspace: one folder, normally a git repository root (D-005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub name: String,
}

impl Workspace {
    pub fn validate(&self) -> Vec<ValidationError> {
        validate_name(&self.name)
    }
}

/// A collection: a top-level group of folders and requests with its own
/// variables and default auth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collection {
    pub name: String,
    pub order: OrderKey,
    pub description: String,
    /// Committed with their values and never secret (D-019).
    pub variables: Vec<Variable>,
    /// The default for everything in the collection. `Inherit` here means no auth.
    pub auth: Auth,
}

impl Collection {
    pub fn validate(&self) -> Vec<ValidationError> {
        let mut errors = validate_name(&self.name);
        validate_variables(&self.variables, false, &mut errors);
        errors
    }
}

/// A folder inside a collection or another folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    pub name: String,
    pub order: OrderKey,
    pub description: String,
    pub auth: Auth,
}

impl Folder {
    pub fn validate(&self) -> Vec<ValidationError> {
        validate_name(&self.name)
    }
}

fn validate_name(name: &str) -> Vec<ValidationError> {
    if name.trim().is_empty() {
        vec![ValidationError::EmptyName]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_variables_cannot_be_secret() {
        let collection = Collection {
            name: "Petstore".to_owned(),
            order: OrderKey::new("a0").unwrap(),
            description: String::new(),
            variables: vec![Variable {
                name: "token".to_owned(),
                secret: true,
                ..Variable::default()
            }],
            auth: Auth::None,
        };
        assert_eq!(
            collection.validate(),
            [ValidationError::SecretNotAllowed("token".to_owned())]
        );
    }

    #[test]
    fn names_must_not_be_blank() {
        let folder = Folder {
            name: " ".to_owned(),
            order: OrderKey::new("a0").unwrap(),
            description: String::new(),
            auth: Auth::Inherit,
        };
        assert_eq!(folder.validate(), [ValidationError::EmptyName]);
        assert_eq!(
            Workspace {
                name: String::new()
            }
            .validate(),
            [ValidationError::EmptyName]
        );
    }
}
