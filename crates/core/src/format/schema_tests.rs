//! The JSON Schemas in `schemas/v1/` must accept every file the writer
//! produces and reject what the reader refuses or moves out.

use serde_json::{Map, Number, Value as Json};

use super::tree::{Node, NodeKind, parse};
use super::*;
use crate::model::{Auth, Body, OrderKey, Part, PartValue, QueryParam, Request};

const BASE: &str = "https://raw.githubusercontent.com/Berumor/Damn-HTTP/main/schemas/v1/";

const SCHEMAS: &[(&str, &str)] = &[
    (
        "common",
        include_str!("../../../../schemas/v1/common.schema.json"),
    ),
    (
        "workspace",
        include_str!("../../../../schemas/v1/workspace.schema.json"),
    ),
    (
        "collection",
        include_str!("../../../../schemas/v1/collection.schema.json"),
    ),
    (
        "folder",
        include_str!("../../../../schemas/v1/folder.schema.json"),
    ),
    (
        "request",
        include_str!("../../../../schemas/v1/request.schema.json"),
    ),
    (
        "environment",
        include_str!("../../../../schemas/v1/environment.schema.json"),
    ),
    (
        "values",
        include_str!("../../../../schemas/v1/values.schema.json"),
    ),
    (
        "file",
        include_str!("../../../../schemas/v1/file.schema.json"),
    ),
];

macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!("../../tests/fixtures/format/", $name))
    };
}

struct Validator {
    schemas: boon::Schemas,
    file: boon::SchemaIndex,
}

impl Validator {
    fn new() -> Self {
        let mut compiler = boon::Compiler::new();
        for (name, text) in SCHEMAS {
            let url = format!("{BASE}{name}.schema.json");
            let json: Json = serde_json::from_str(text).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(json["$id"], url.as_str(), "{name}");
            compiler.add_resource(&url, json).unwrap();
        }
        let mut schemas = boon::Schemas::new();
        for (name, _) in SCHEMAS {
            compiler
                .compile(&format!("{BASE}{name}.schema.json"), &mut schemas)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
        }
        let file = compiler
            .compile(&format!("{BASE}file.schema.json"), &mut schemas)
            .unwrap();
        Self { schemas, file }
    }

    fn check(&self, yaml: &str) -> Result<(), String> {
        let json = to_json(&parse(yaml).unwrap());
        self.schemas
            .validate(&json, self.file)
            .map_err(|e| format!("{e:#}"))
    }
}

/// YAML 1.2 core schema typing, as an editor's YAML plugin applies it.
fn to_json(node: &Node) -> Json {
    match &node.kind {
        NodeKind::Scalar {
            value,
            plain: false,
        } => Json::String(value.clone()),
        NodeKind::Scalar { value, plain: true } => match value.as_str() {
            "" | "~" | "null" | "Null" | "NULL" => Json::Null,
            "true" | "True" | "TRUE" => Json::Bool(true),
            "false" | "False" | "FALSE" => Json::Bool(false),
            text => match text.parse::<u64>() {
                Ok(n) if text.bytes().all(|b| b.is_ascii_digit()) => Json::Number(Number::from(n)),
                _ => Json::String(text.to_owned()),
            },
        },
        NodeKind::List(items) => Json::Array(items.iter().map(to_json).collect()),
        NodeKind::Map(entries) => Json::Object(
            entries
                .iter()
                .map(|e| (e.key.clone(), to_json(&e.value)))
                .collect::<Map<_, _>>(),
        ),
    }
}

#[test]
fn every_fixture_matches_its_schema() {
    let validator = Validator::new();
    for (name, text) in [
        ("workspace", fixture!("workspace.yaml")),
        ("collection", fixture!("collection.yaml")),
        ("folder", fixture!("folder.yaml")),
        ("environment", fixture!("environment.yaml")),
        ("values", fixture!("values.yaml")),
        ("request-minimal", fixture!("request-minimal.yaml")),
        ("request-full", fixture!("request-full.yaml")),
        ("request-form", fixture!("request-form.yaml")),
        ("request-multipart", fixture!("request-multipart.yaml")),
        ("request-text", fixture!("request-text.yaml")),
    ] {
        validator
            .check(text)
            .unwrap_or_else(|e| panic!("{name}:\n{e}"));
    }
}

#[test]
fn written_files_match_the_schema() {
    let validator = Validator::new();
    let mut request = Request::new("No", OrderKey::new("a").unwrap());
    request.url = "123".to_owned();
    request.query = vec![
        QueryParam {
            name: "true".to_owned(),
            value: "{{ x }}".to_owned(),
            enabled: false,
            description: "1.5".to_owned(),
        },
        QueryParam {
            name: "literal".to_owned(),
            value: "1234".to_owned(),
            enabled: true,
            description: String::new(),
        },
    ];
    request.auth = Auth::Bearer {
        token: String::new(),
    };
    request.body = Body::Multipart(vec![
        Part {
            name: "a".to_owned(),
            value: PartValue::Text(String::new()),
            enabled: true,
        },
        Part {
            name: "b".to_owned(),
            value: PartValue::File(String::new()),
            enabled: false,
        },
    ]);
    let (committed, _) = request.split();
    let text = write_request(&committed);
    validator
        .check(&text)
        .unwrap_or_else(|e| panic!("{text}\n{e}"));
}

#[test]
fn the_schema_rejects_what_does_not_belong() {
    let validator = Validator::new();
    let request = fixture!("request-minimal.yaml");
    for bad in [
        // A literal query value belongs in the local values (D-021).
        format!("{request}query:\n  - name: id\n    value: \"1234\"\n"),
        format!("{request}path_params:\n  - name: id\n    value: \"{{{{a}}}}-1\"\n"),
        format!("{request}unknown: x\n"),
        format!(
            "{request}body:\n  type: form-urlencoded\n  fields:\n    - name: f\n      file: a.png\n"
        ),
        format!("{request}auth:\n  type: basic\n  token: x\n"),
        request.replace("version: 1", "version: 2"),
        request.replace("order: a0", "order: a-0"),
        // A secret variable carries no value (D-019).
        format!(
            "{}  - name: leaked\n    value: x\n    secret: true\n",
            fixture!("environment.yaml")
        ),
        format!(
            "{}  - name: s\n    secret: true\n",
            fixture!("collection.yaml")
                .replace("auth:\n  type: bearer\n  token: \"{{apiToken}}\"\n", "")
        ),
    ] {
        assert!(validator.check(&bad).is_err(), "accepted:\n{bad}");
    }
}
