//! Request files: one YAML file per request.

use super::common::{auth_value, header, open, read_auth};
use super::emit::{MapBuilder, Value, render};
use super::error::{FormatError, FormatErrorKind};
use super::read::{MapReader, item_path};
use super::tree::parse;
use crate::model::{
    Auth, Body, CommittedRequest, Header, Method, OrderKey, Part, PartValue, PathParam, QueryParam,
    Request, RequestLocalValues, RequestSettings,
};

/// A request file as loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestFile {
    /// The committed part, as the file holds it after [`Request::split`].
    pub request: CommittedRequest,
    /// Literal query or path variable values found in the file. They can
    /// only get there by a hand edit (D-021). They are not in `request`;
    /// the caller stores them with the local values and reports them, and
    /// the next save writes the file without them.
    pub literals: RequestLocalValues,
}

/// Writes a request file. Only a [`CommittedRequest`] can be written, so a
/// literal query or path variable value never reaches the file (D-033).
pub fn write_request(request: &CommittedRequest) -> String {
    let r = request.as_request();
    let mut map = header(Some(r.id.to_string()), "request");
    map.put("name", Value::str(&r.name))
        .put("order", Value::str(r.order.as_str()))
        .put_str("description", &r.description)
        .put("method", Value::str(r.method.as_str()))
        .put("url", Value::str(&r.url))
        .put(
            "path_params",
            Value::List(r.path_params.iter().map(path_param_value).collect()),
        )
        .put(
            "query",
            Value::List(r.query.iter().map(query_value).collect()),
        )
        .put(
            "headers",
            Value::List(r.headers.iter().map(header_value).collect()),
        );
    if !r.auth.is_inherit() {
        map.put("auth", auth_value(&r.auth));
    }
    if r.body != Body::None {
        map.put("body", body_value(&r.body));
    }
    map.put("settings", settings_value(&r.settings));
    render(&map.build())
}

fn path_param_value(param: &PathParam) -> Value {
    let mut map = MapBuilder::new();
    map.put("name", Value::str(&param.name))
        .put_str("value", &param.value)
        .put_str("description", &param.description);
    map.build()
}

fn query_value(param: &QueryParam) -> Value {
    let mut map = MapBuilder::new();
    map.put("name", Value::str(&param.name))
        .put_str("value", &param.value)
        .put_disabled("enabled", param.enabled)
        .put_str("description", &param.description);
    map.build()
}

fn header_value(header: &Header) -> Value {
    let mut map = MapBuilder::new();
    map.put("name", Value::str(&header.name))
        .put_str("value", &header.value)
        .put_disabled("enabled", header.enabled)
        .put_str("description", &header.description);
    map.build()
}

fn body_value(body: &Body) -> Value {
    let mut map = MapBuilder::new();
    match body {
        Body::None => {
            map.put("type", Value::str("none"));
        }
        Body::Json(content) => {
            map.put("type", Value::str("json"))
                .put_str("content", content);
        }
        Body::Text {
            content,
            content_type,
        } => {
            map.put("type", Value::str("text"))
                .put_str("content", content);
            if let Some(content_type) = content_type {
                map.put("content_type", Value::str(content_type));
            }
        }
        Body::FormUrlencoded(parts) => {
            map.put("type", Value::str("form-urlencoded"))
                .put("fields", parts_value(parts));
        }
        Body::Multipart(parts) => {
            map.put("type", Value::str("multipart"))
                .put("fields", parts_value(parts));
        }
    }
    map.build()
}

fn parts_value(parts: &[Part]) -> Value {
    Value::List(
        parts
            .iter()
            .map(|part| {
                let mut map = MapBuilder::new();
                map.put("name", Value::str(&part.name));
                match &part.value {
                    PartValue::Text(text) => {
                        map.put_str("value", text);
                    }
                    PartValue::File(path) => {
                        map.put("file", Value::str(path));
                    }
                }
                map.put_disabled("enabled", part.enabled);
                map.build()
            })
            .collect(),
    )
}

fn settings_value(settings: &RequestSettings) -> Value {
    let mut map = MapBuilder::new();
    if let Some(follow) = settings.follow_redirects {
        map.put("follow_redirects", Value::Bool(follow));
    }
    if let Some(max) = settings.max_redirects {
        map.put("max_redirects", Value::Int(u64::from(max)));
    }
    if let Some(timeout) = settings.timeout_ms {
        map.put("timeout_ms", Value::Int(timeout));
    }
    map.build()
}

/// Reads a request file. The result goes through [`Request::split`], so a
/// literal value written into the file by hand comes back in
/// [`RequestFile::literals`], not in the committed part (D-033).
///
/// Only the structure is checked here; call [`Request::validate`] for the
/// rest (names, methods, duplicates).
pub fn read_request(text: &str) -> Result<RequestFile, FormatError> {
    let root = parse(text)?;
    let mut map = open(&root, "request")?;
    let request = Request {
        id: map.required_parsed(
            "id",
            "not a valid request id (expected a lowercase, hyphenated UUID)",
            |s| s.parse().ok(),
        )?,
        name: map.required_string("name")?,
        order: map.required_parsed("order", "not a valid order key", |s| OrderKey::new(s).ok())?,
        description: map.string("description")?,
        method: Method::from_token(&map.required_string("method")?),
        url: map.required_string("url")?,
        path_params: read_path_params(&mut map)?,
        query: read_query(&mut map)?,
        headers: read_headers(&mut map)?,
        auth: read_auth(&mut map, Auth::Inherit)?,
        body: read_body(&mut map)?,
        settings: read_settings(&mut map)?,
        skip_tls_verify: false,
    };
    map.finish()?;
    let (request, literals) = request.split();
    Ok(RequestFile { request, literals })
}

/// Calls `read` with a reader for each item of the list under `key`.
fn read_list<T>(
    parent: &mut MapReader<'_>,
    key: &str,
    mut read: impl FnMut(&mut MapReader<'_>) -> Result<T, FormatError>,
) -> Result<Vec<T>, FormatError> {
    let path = parent.child_path(key);
    let mut items = Vec::new();
    for (index, node) in parent.list(key)?.iter().enumerate() {
        let mut map = MapReader::new(node, item_path(&path, index))?;
        items.push(read(&mut map)?);
        map.finish()?;
    }
    Ok(items)
}

fn read_path_params(parent: &mut MapReader<'_>) -> Result<Vec<PathParam>, FormatError> {
    read_list(parent, "path_params", |map| {
        Ok(PathParam {
            name: map.required_string("name")?,
            value: map.string("value")?,
            description: map.string("description")?,
        })
    })
}

fn read_query(parent: &mut MapReader<'_>) -> Result<Vec<QueryParam>, FormatError> {
    read_list(parent, "query", |map| {
        Ok(QueryParam {
            name: map.required_string("name")?,
            value: map.string("value")?,
            enabled: map.bool_or("enabled", true)?,
            description: map.string("description")?,
        })
    })
}

fn read_headers(parent: &mut MapReader<'_>) -> Result<Vec<Header>, FormatError> {
    read_list(parent, "headers", |map| {
        Ok(Header {
            name: map.required_string("name")?,
            value: map.string("value")?,
            enabled: map.bool_or("enabled", true)?,
            description: map.string("description")?,
        })
    })
}

fn read_body(parent: &mut MapReader<'_>) -> Result<Body, FormatError> {
    let Some(mut map) = parent.optional_map("body")? else {
        return Ok(Body::None);
    };
    let type_node = map.required("type")?;
    let body = match map.required_string("type")?.as_str() {
        "none" => Body::None,
        "json" => Body::Json(map.string("content")?),
        "text" => Body::Text {
            content: map.string("content")?,
            content_type: map.optional_string("content_type")?,
        },
        "form-urlencoded" => Body::FormUrlencoded(read_parts(&mut map)?),
        "multipart" => Body::Multipart(read_parts(&mut map)?),
        _ => {
            return Err(type_node.error(FormatErrorKind::InvalidValue {
                path: map.child_path("type"),
                problem: "unknown body type (expected none, json, text, form-urlencoded or multipart)",
            }));
        }
    };
    map.finish()?;
    Ok(body)
}

fn read_parts(parent: &mut MapReader<'_>) -> Result<Vec<Part>, FormatError> {
    read_list(parent, "fields", |map| {
        let name = map.required_string("name")?;
        let text = map.optional_string("value")?;
        let file = map.optional_string("file")?;
        let value = match (text, file) {
            (Some(_), Some(_)) => {
                let node = map.required("file")?;
                return Err(node.error(FormatErrorKind::InvalidValue {
                    path: map.path().to_owned(),
                    problem: "a field has either `value` or `file`, not both",
                }));
            }
            (_, Some(file)) => PartValue::File(file),
            (text, None) => PartValue::Text(text.unwrap_or_default()),
        };
        Ok(Part {
            name,
            value,
            enabled: map.bool_or("enabled", true)?,
        })
    })
}

fn read_settings(parent: &mut MapReader<'_>) -> Result<RequestSettings, FormatError> {
    let Some(mut map) = parent.optional_map("settings")? else {
        return Ok(RequestSettings::default());
    };
    let settings = RequestSettings {
        follow_redirects: map.optional_bool("follow_redirects")?,
        max_redirects: map.optional_u32("max_redirects")?,
        timeout_ms: map.optional_u64("timeout_ms")?,
    };
    map.finish()?;
    Ok(settings)
}
