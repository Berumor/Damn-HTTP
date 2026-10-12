//! Workspace manifest, collection and folder files.

use super::common::{auth_value, header, open, read_auth, read_variables, variables_value};
use super::emit::{Value, render};
use super::error::FormatError;
use super::tree::parse;
use crate::model::{Auth, Collection, Folder, OrderKey, Workspace};

/// Writes `damnhttp.yaml`.
pub fn write_workspace(workspace: &Workspace) -> String {
    let mut map = header(None, "workspace");
    map.put("name", Value::str(&workspace.name));
    render(&map.build())
}

pub fn read_workspace(text: &str) -> Result<Workspace, FormatError> {
    let root = parse(text)?;
    let mut map = open(&root, "workspace")?;
    let workspace = Workspace {
        name: map.required_string("name")?,
    };
    map.finish()?;
    Ok(workspace)
}

/// Writes `collection.yaml`. Collection variables are written with their
/// values; they cannot be secret (D-019).
pub fn write_collection(collection: &Collection) -> String {
    let mut map = header(None, "collection");
    map.put("name", Value::str(&collection.name))
        .put("order", Value::str(collection.order.as_str()))
        .put_str("description", &collection.description)
        .put("variables", variables_value(&collection.variables));
    // On a collection the default is `none`.
    if collection.auth != Auth::None {
        map.put("auth", auth_value(&collection.auth));
    }
    render(&map.build())
}

/// Reads `collection.yaml`. Only the structure is checked; call
/// [`Collection::validate`] for the rest.
pub fn read_collection(text: &str) -> Result<Collection, FormatError> {
    let root = parse(text)?;
    let mut map = open(&root, "collection")?;
    let collection = Collection {
        name: map.required_string("name")?,
        order: read_order(&mut map)?,
        description: map.string("description")?,
        variables: read_variables(&mut map)?.0,
        auth: read_auth(&mut map, Auth::None)?,
    };
    map.finish()?;
    Ok(collection)
}

/// Writes `folder.yaml`.
pub fn write_folder(folder: &Folder) -> String {
    let mut map = header(None, "folder");
    map.put("name", Value::str(&folder.name))
        .put("order", Value::str(folder.order.as_str()))
        .put_str("description", &folder.description);
    if !folder.auth.is_inherit() {
        map.put("auth", auth_value(&folder.auth));
    }
    render(&map.build())
}

pub fn read_folder(text: &str) -> Result<Folder, FormatError> {
    let root = parse(text)?;
    let mut map = open(&root, "folder")?;
    let folder = Folder {
        name: map.required_string("name")?,
        order: read_order(&mut map)?,
        description: map.string("description")?,
        auth: read_auth(&mut map, Auth::Inherit)?,
    };
    map.finish()?;
    Ok(folder)
}

fn read_order(map: &mut super::read::MapReader<'_>) -> Result<OrderKey, FormatError> {
    map.required_parsed("order", "not a valid order key", |s| OrderKey::new(s).ok())
}
