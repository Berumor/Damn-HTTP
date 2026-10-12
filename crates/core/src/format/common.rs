//! Pieces shared by several kinds of file: the header, auth and variables.

use super::FORMAT_VERSION;
use super::emit::{MapBuilder, Value};
use super::error::{FormatError, FormatErrorKind};
use super::read::{MapReader, item_path};
use super::tree::Node;
use crate::model::{Auth, Variable};

/// Starts a file: `id` (requests and environments, D-022), `version`, `kind`.
pub(crate) fn header(id: Option<String>, kind: &str) -> MapBuilder {
    let mut map = MapBuilder::new();
    if let Some(id) = id {
        map.put("id", Value::Str(id));
    }
    map.put("version", Value::Int(FORMAT_VERSION))
        .put("kind", Value::str(kind));
    map
}

/// Checks `version` and `kind` and returns a reader for the other keys.
///
/// The version is checked first, so a file from a newer app is reported as
/// such instead of failing on a key this app does not know (D-009).
pub(crate) fn open<'n>(root: &'n Node, kind: &'static str) -> Result<MapReader<'n>, FormatError> {
    let mut map = MapReader::new(root, "")?;
    let version_node = map.required("version")?;
    let version = map.optional_u64("version")?.unwrap_or_default();
    if version == 0 {
        return Err(version_node.error(FormatErrorKind::InvalidValue {
            path: "version".to_owned(),
            problem: "the version must be 1 or higher",
        }));
    }
    if version > FORMAT_VERSION {
        return Err(version_node.error(FormatErrorKind::NewerVersion {
            found: version,
            supported: FORMAT_VERSION,
        }));
    }
    let kind_node = map.required("kind")?;
    let found = map.required_string("kind")?;
    if found != kind {
        return Err(kind_node.error(FormatErrorKind::WrongKind {
            expected: kind,
            found,
        }));
    }
    Ok(map)
}

pub(crate) fn auth_value(auth: &Auth) -> Value {
    let mut map = MapBuilder::new();
    match auth {
        Auth::Inherit => {
            map.put("type", Value::str("inherit"));
        }
        Auth::None => {
            map.put("type", Value::str("none"));
        }
        Auth::Basic { username, password } => {
            map.put("type", Value::str("basic"))
                .put_str("username", username)
                .put_str("password", password);
        }
        Auth::Bearer { token } => {
            map.put("type", Value::str("bearer"))
                .put_str("token", token);
        }
    }
    map.build()
}

/// Reads `auth`, or returns `default` when the key is absent.
pub(crate) fn read_auth(parent: &mut MapReader<'_>, default: Auth) -> Result<Auth, FormatError> {
    let Some(mut map) = parent.optional_map("auth")? else {
        return Ok(default);
    };
    let type_node = map.required("type")?;
    let auth = match map.required_string("type")?.as_str() {
        "inherit" => Auth::Inherit,
        "none" => Auth::None,
        "basic" => Auth::Basic {
            username: map.string("username")?,
            password: map.string("password")?,
        },
        "bearer" => Auth::Bearer {
            token: map.string("token")?,
        },
        _ => {
            return Err(type_node.error(FormatErrorKind::InvalidValue {
                path: map.child_path("type"),
                problem: "unknown auth type (expected inherit, none, basic or bearer)",
            }));
        }
    };
    map.finish()?;
    Ok(auth)
}

/// Variables as a list. The value of a secret variable is never written
/// (D-019): it lives in `.damnhttp/` only.
pub(crate) fn variables_value(variables: &[Variable]) -> Value {
    Value::List(
        variables
            .iter()
            .map(|variable| {
                let mut map = MapBuilder::new();
                map.put("name", Value::str(&variable.name));
                if variable.secret {
                    map.put("secret", Value::Bool(true));
                } else {
                    map.put_str("value", &variable.value);
                }
                map.put_str("description", &variable.description);
                map.build()
            })
            .collect(),
    )
}

/// Reads `variables`. Also returns the names of secret variables that have
/// a value in the file, which can only come from a hand edit.
pub(crate) fn read_variables(
    parent: &mut MapReader<'_>,
) -> Result<(Vec<Variable>, Vec<String>), FormatError> {
    let path = parent.child_path("variables");
    let mut variables = Vec::new();
    let mut secret_values = Vec::new();
    for (index, node) in parent.list("variables")?.iter().enumerate() {
        let mut map = MapReader::new(node, item_path(&path, index))?;
        let variable = Variable {
            name: map.required_string("name")?,
            value: map.string("value")?,
            secret: map.bool_or("secret", false)?,
            description: map.string("description")?,
        };
        map.finish()?;
        if variable.secret && !variable.value.is_empty() {
            secret_values.push(variable.name.clone());
        }
        variables.push(variable);
    }
    Ok((variables, secret_values))
}
