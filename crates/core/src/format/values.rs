//! The local values file: `.damnhttp/values.yaml` (D-021).
//!
//! It is git-ignored and written by the same emitter as committed files,
//! so it is just as stable to read and diff by hand.

use super::common::{header, open};
use super::emit::{MapBuilder, Value, render};
use super::error::{FormatError, FormatErrorKind};
use super::read::{MapReader, item_path, scalar_string};
use super::tree::parse;
use crate::model::{LocalQueryValue, LocalValues, RequestId, RequestLocalValues};

/// Writes `.damnhttp/values.yaml`. Requests are sorted by id; requests with
/// nothing to store are left out.
pub fn write_local_values(values: &LocalValues) -> String {
    let mut requests = MapBuilder::new();
    for (id, local) in &values.requests {
        if !local.is_empty() {
            requests.put(&id.to_string(), request_value(local));
        }
    }
    let mut map = header(None, "values");
    map.put("requests", requests.build());
    render(&map.build())
}

fn request_value(local: &RequestLocalValues) -> Value {
    let mut path_params = MapBuilder::new();
    for (name, value) in &local.path_params {
        path_params.put(name, Value::str(value));
    }
    let query = local
        .query
        .iter()
        .map(|q| {
            let mut map = MapBuilder::new();
            map.put("name", Value::str(&q.name));
            if q.occurrence > 0 {
                map.put("occurrence", Value::Int(q.occurrence as u64));
            }
            map.put("value", Value::str(&q.value));
            map.build()
        })
        .collect();
    let mut map = MapBuilder::new();
    map.put("path_params", path_params.build())
        .put("query", Value::List(query));
    if local.skip_tls_verify {
        map.put("skip_tls_verify", Value::Bool(true));
    }
    map.build()
}

pub fn read_local_values(text: &str) -> Result<LocalValues, FormatError> {
    let root = parse(text)?;
    let mut map = open(&root, "values")?;
    let mut values = LocalValues::default();
    if let Some(requests) = map.optional_map("requests")? {
        let path = requests.path().to_owned();
        for entry in requests.entries() {
            let (key, node) = (&entry.key, &entry.value);
            let id: RequestId = key.parse().map_err(|_| {
                entry.error(FormatErrorKind::InvalidValue {
                    path: path.clone(),
                    problem: "a key is not a valid request id",
                })
            })?;
            let mut request = MapReader::new(node, format!("{path}.{key}"))?;
            let local = read_request_values(&mut request)?;
            request.finish()?;
            values.requests.insert(id, local);
        }
    }
    map.finish()?;
    Ok(values)
}

fn read_request_values(map: &mut MapReader<'_>) -> Result<RequestLocalValues, FormatError> {
    let mut local = RequestLocalValues::default();
    if let Some(path_params) = map.optional_map("path_params")? {
        let path = path_params.path().to_owned();
        for entry in path_params.entries() {
            let value = scalar_string(&entry.value, &format!("{path}.{}", entry.key))?;
            local.path_params.insert(entry.key.clone(), value);
        }
    }
    let path = map.child_path("query");
    for (index, node) in map.list("query")?.iter().enumerate() {
        let item_path = item_path(&path, index);
        let mut item = MapReader::new(node, item_path.clone())?;
        let name = item.required_string("name")?;
        let occurrence = match item.optional_u64("occurrence")? {
            None => 0,
            Some(n) => usize::try_from(n).map_err(|_| {
                node.error(FormatErrorKind::InvalidValue {
                    path: item_path.clone(),
                    problem: "the number is too large",
                })
            })?,
        };
        let value = item.required_string("value")?;
        item.finish()?;
        if local
            .query
            .iter()
            .any(|q| q.name == name && q.occurrence == occurrence)
        {
            return Err(node.error(FormatErrorKind::InvalidValue {
                path: item_path,
                problem: "the same query parameter is listed twice",
            }));
        }
        local.query.push(LocalQueryValue {
            name,
            occurrence,
            value,
        });
    }
    local.skip_tls_verify = map.bool_or("skip_tls_verify", false)?;
    Ok(local)
}
