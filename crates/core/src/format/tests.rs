//! Round-trip, determinism and invariant tests for the file format.

use std::collections::BTreeMap;

use super::emit::{Value, render};
use super::tree::{NodeKind, parse};
use super::*;
use crate::model::{
    Auth, Body, Collection, Environment, EnvironmentId, Folder, Header, LocalQueryValue,
    LocalValues, Method, OrderKey, Part, PartValue, PathParam, QueryParam, Request, RequestId,
    RequestLocalValues, RequestSettings, Variable, Workspace,
};

macro_rules! fixture {
    ($name:literal) => {
        (
            $name,
            include_str!(concat!("../../tests/fixtures/format/", $name)),
        )
    };
}

const REQUEST_FIXTURES: &[(&str, &str)] = &[
    fixture!("request-minimal.yaml"),
    fixture!("request-full.yaml"),
    fixture!("request-form.yaml"),
    fixture!("request-multipart.yaml"),
    fixture!("request-text.yaml"),
];

fn order(key: &str) -> OrderKey {
    OrderKey::new(key).unwrap()
}

fn query(name: &str, value: &str) -> QueryParam {
    QueryParam {
        name: name.to_owned(),
        value: value.to_owned(),
        enabled: true,
        description: String::new(),
    }
}

fn path(name: &str, value: &str, description: &str) -> PathParam {
    PathParam {
        name: name.to_owned(),
        value: value.to_owned(),
        description: description.to_owned(),
    }
}

fn header(name: &str, value: &str) -> Header {
    Header {
        name: name.to_owned(),
        value: value.to_owned(),
        enabled: true,
        description: String::new(),
    }
}

/// A request using every field, with awkward strings in many places.
fn full_request() -> Request {
    let mut r = Request::new("Update: pet #1", order("a1V"));
    r.description = "Line one\n\n  indented: yes\ttab\n".to_owned();
    r.method = Method::Custom("PROPFIND".to_owned());
    r.url = "{{baseUrl}}/stores/:storeId/pets/:petId".to_owned();
    r.path_params = vec![
        path("storeId", "{{storeId}}", ""),
        path("petId", "", "The pet: \"quoted\""),
    ];
    r.query = vec![
        QueryParam {
            enabled: false,
            ..query("verbose", "")
        },
        query("tenant", "{{tenantId}}"),
        query("", ""),
    ];
    r.headers = vec![
        header("Accept", "application/json"),
        header("X-Bool", "true"),
        header("X-Number", "007"),
        header("X-Hash", "a #b"),
        Header {
            enabled: false,
            description: "  spaced  ".to_owned(),
            ..header("X-Off", "- not a list")
        },
        header("X-Unicode", "é \u{2028} \u{1F600}"),
    ];
    r.auth = Auth::Basic {
        username: "{{user}}".to_owned(),
        password: String::new(),
    };
    r.body = Body::Json("{\n  \"a\": [1, 2]\n}".to_owned());
    r.settings = RequestSettings {
        follow_redirects: Some(true),
        max_redirects: Some(0),
        timeout_ms: Some(u64::MAX),
    };
    r
}

fn round_trip(request: Request) -> Request {
    let (committed, _) = request.split();
    let text = write_request(&committed);
    let file = read_request(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
    assert!(file.literals.is_empty(), "{text}");
    assert_eq!(file.request, committed, "{text}");
    assert_eq!(write_request(&file.request), text, "not deterministic");
    file.request.as_request().clone()
}

#[test]
fn request_fixtures_are_canonical() {
    for (name, text) in REQUEST_FIXTURES {
        let file = read_request(text).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(file.literals.is_empty(), "{name}");
        assert_eq!(write_request(&file.request), *text, "{name}");
        assert_eq!(file.request.as_request().validate(), [], "{name}");
    }
}

#[test]
fn other_fixtures_are_canonical() {
    let (_, text) = fixture!("workspace.yaml");
    assert_eq!(write_workspace(&read_workspace(text).unwrap()), text);

    let (_, text) = fixture!("collection.yaml");
    let collection = read_collection(text).unwrap();
    assert_eq!(collection.validate(), []);
    assert_eq!(write_collection(&collection), text);

    let (_, text) = fixture!("folder.yaml");
    assert_eq!(write_folder(&read_folder(text).unwrap()), text);

    let (_, text) = fixture!("environment.yaml");
    let file = read_environment(text).unwrap();
    assert!(file.secret_values_in_file.is_empty());
    assert_eq!(file.environment.validate(), []);
    assert_eq!(write_environment(&file.environment), text);

    let (_, text) = fixture!("values.yaml");
    assert_eq!(write_local_values(&read_local_values(text).unwrap()), text);
}

#[test]
fn a_full_request_round_trips() {
    round_trip(full_request());
}

#[test]
fn every_body_kind_round_trips() {
    let part = |name: &str, value: PartValue, enabled: bool| Part {
        name: name.to_owned(),
        value,
        enabled,
    };
    for body in [
        Body::None,
        Body::Json(String::new()),
        Body::Json("[]".to_owned()),
        Body::Text {
            content: "plain".to_owned(),
            content_type: None,
        },
        Body::Text {
            content: "a\r\nb".to_owned(),
            content_type: Some(String::new()),
        },
        Body::FormUrlencoded(vec![
            part("a", PartValue::Text("1".to_owned()), true),
            part("b", PartValue::Text(String::new()), false),
        ]),
        Body::Multipart(vec![
            part("a", PartValue::Text("x\ny".to_owned()), true),
            part("f", PartValue::File(String::new()), true),
            part("g", PartValue::File("dir/a b.png".to_owned()), false),
        ]),
    ] {
        let mut request = Request::new("Body", order("a"));
        request.body = body.clone();
        assert_eq!(round_trip(request).body, body);
    }
}

#[test]
fn every_auth_kind_round_trips() {
    for auth in [
        Auth::Inherit,
        Auth::None,
        Auth::Basic {
            username: String::new(),
            password: String::new(),
        },
        Auth::Bearer {
            token: "{{t}}".to_owned(),
        },
    ] {
        let mut request = Request::new("Auth", order("a"));
        request.auth = auth.clone();
        assert_eq!(round_trip(request).auth, auth);

        let folder = Folder {
            name: "F".to_owned(),
            order: order("a"),
            description: String::new(),
            auth: auth.clone(),
        };
        assert_eq!(read_folder(&write_folder(&folder)).unwrap(), folder);

        let collection = Collection {
            name: "C".to_owned(),
            order: order("a"),
            description: String::new(),
            variables: Vec::new(),
            auth,
        };
        assert_eq!(
            read_collection(&write_collection(&collection)).unwrap(),
            collection
        );
    }
}

#[test]
fn defaults_are_left_out() {
    let mut request = Request::new("Empty", order("a0"));
    request.id = "0b6f3d2a-9c1e-4f5a-8b7d-2e4c6a8f0d13".parse().unwrap();
    let (committed, _) = request.split();
    assert_eq!(
        write_request(&committed),
        "id: 0b6f3d2a-9c1e-4f5a-8b7d-2e4c6a8f0d13\nversion: 1\nkind: request\nname: Empty\norder: a0\nmethod: GET\nurl: \"\"\n"
    );

    let folder = Folder {
        name: "F".to_owned(),
        order: order("a"),
        description: String::new(),
        auth: Auth::Inherit,
    };
    assert_eq!(
        write_folder(&folder),
        "version: 1\nkind: folder\nname: F\norder: a\n"
    );

    let collection = Collection {
        name: "C".to_owned(),
        order: order("a"),
        description: String::new(),
        variables: Vec::new(),
        auth: Auth::None,
    };
    assert_eq!(
        write_collection(&collection),
        "version: 1\nkind: collection\nname: C\norder: a\n"
    );
}

/// D-021: whatever literal values a request holds, the written file
/// contains none of them, and reading the file finds none.
#[test]
fn no_literal_query_or_path_value_reaches_a_request_file() {
    let literals = [
        "LIT-1234-query",
        "LIT-it-path",
        "{{a}}-LIT",
        "LIT {{b}}",
        "{{LIT}}{{x}}",
        "Bearer LIT.eyJhbGciOi",
    ];
    let mut request = full_request();
    for (i, literal) in literals.iter().enumerate() {
        request.query.push(query(&format!("q{i}"), literal));
        request.query.push(QueryParam {
            enabled: false,
            ..query("same", literal)
        });
        request
            .path_params
            .push(path(&format!("p{i}"), literal, ""));
        request
            .path_params
            .push(path(&format!("d{i}"), literal, "described"));
    }
    request.skip_tls_verify = true;

    let (committed, local) = request.split();
    let text = write_request(&committed);

    assert!(!text.contains("LIT"), "{text}");
    assert!(!text.contains("skip_tls"), "{text}");
    assert_eq!(local.query.len(), literals.len() * 2);
    assert_eq!(local.path_params.len(), literals.len() * 2);
    assert!(read_request(&text).unwrap().literals.is_empty());
}

#[test]
fn a_literal_written_by_hand_is_moved_out_on_load() {
    let text = "\
id: 3f2b8c1e-7a4d-4e9b-9c55-0d6a1b2e4f70
version: 1
kind: request
name: Stations
order: a0
method: GET
url: https://some.service/:country/stations/
path_params:
  - name: country
    value: it
query:
  - name: id
    value: 1234
  - name: token
    value: '{{ token }}'
";
    let file = read_request(text).unwrap();

    assert_eq!(
        file.literals.path_params,
        BTreeMap::from([("country".to_owned(), "it".to_owned())])
    );
    assert_eq!(
        file.literals.query,
        [LocalQueryValue {
            name: "id".to_owned(),
            occurrence: 0,
            value: "1234".to_owned()
        }]
    );
    let rewritten = write_request(&file.request);
    assert!(!rewritten.contains("1234") && !rewritten.contains("value: it"));
    assert!(rewritten.contains("value: \"{{token}}\""), "{rewritten}");
}

#[test]
fn secret_values_never_reach_an_environment_file() {
    let environment = Environment {
        id: EnvironmentId::generate(),
        name: "Prod".to_owned(),
        variables: vec![
            Variable {
                name: "token".to_owned(),
                value: "SECRET-sk-live-123".to_owned(),
                secret: true,
                description: String::new(),
            },
            Variable {
                name: "baseUrl".to_owned(),
                value: "https://example.com".to_owned(),
                secret: false,
                description: String::new(),
            },
        ],
    };
    let text = write_environment(&environment);
    assert!(!text.contains("SECRET"), "{text}");

    let file = read_environment(&text).unwrap();
    assert!(file.secret_values_in_file.is_empty());
    assert_eq!(file.environment.variables[0].value, "");
    assert!(file.environment.variables[0].secret);
    assert_eq!(write_environment(&file.environment), text);
}

#[test]
fn a_secret_value_written_by_hand_is_reported() {
    let text = "id: b81d0c34-5e2f-4a67-8f19-6c3e9a7d2b05\nversion: 1\nkind: environment\nname: P\nvariables:\n  - name: token\n    value: leaked\n    secret: true\n";
    let file = read_environment(text).unwrap();
    assert_eq!(file.secret_values_in_file, ["token"]);
    assert!(!write_environment(&file.environment).contains("leaked"));
}

#[test]
fn local_values_round_trip() {
    let id_a: RequestId = "00000000-0000-4000-8000-000000000001".parse().unwrap();
    let id_b: RequestId = "ffffffff-ffff-4fff-bfff-ffffffffffff".parse().unwrap();
    let id_empty = RequestId::generate();
    let values = LocalValues {
        requests: BTreeMap::from([
            (
                id_b,
                RequestLocalValues {
                    path_params: BTreeMap::from([
                        ("true".to_owned(), "no".to_owned()),
                        ("a: b".to_owned(), "x\ny".to_owned()),
                    ]),
                    query: vec![
                        LocalQueryValue {
                            name: "id".to_owned(),
                            occurrence: 3,
                            value: "{{a}}-1".to_owned(),
                        },
                        LocalQueryValue {
                            name: "".to_owned(),
                            occurrence: 0,
                            value: " ".to_owned(),
                        },
                    ],
                    skip_tls_verify: false,
                },
            ),
            (
                id_a,
                RequestLocalValues {
                    skip_tls_verify: true,
                    ..RequestLocalValues::default()
                },
            ),
            (id_empty, RequestLocalValues::default()),
        ]),
    };
    let text = write_local_values(&values);
    let read = read_local_values(&text).unwrap();

    let mut expected = values;
    expected.requests.remove(&id_empty);
    assert_eq!(read, expected);
    assert_eq!(write_local_values(&read), text);
    assert!(text.find(&id_a.to_string()) < text.find(&id_b.to_string()));
    assert_eq!(
        write_local_values(&LocalValues::default()),
        "version: 1\nkind: values\n"
    );
}

#[test]
fn workspace_collection_and_environment_round_trip() {
    let workspace = Workspace {
        name: "null".to_owned(),
    };
    assert_eq!(
        read_workspace(&write_workspace(&workspace)).unwrap(),
        workspace
    );

    let collection = Collection {
        name: "Petstore".to_owned(),
        order: order("Zz9"),
        description: "multi\nline".to_owned(),
        variables: vec![
            Variable {
                name: "a.b-c_d".to_owned(),
                value: "1.0".to_owned(),
                secret: false,
                description: "#".to_owned(),
            },
            Variable {
                name: "empty".to_owned(),
                ..Variable::default()
            },
        ],
        auth: Auth::Bearer {
            token: String::new(),
        },
    };
    let text = write_collection(&collection);
    assert_eq!(read_collection(&text).unwrap(), collection);
    assert_eq!(write_collection(&read_collection(&text).unwrap()), text);

    let environment = Environment {
        id: EnvironmentId::generate(),
        name: "Dev".to_owned(),
        variables: Vec::new(),
    };
    let text = write_environment(&environment);
    assert!(text.starts_with(&format!("id: {}\n", environment.id)));
    assert_eq!(read_environment(&text).unwrap().environment, environment);
}

#[test]
fn ids_come_first_in_request_files() {
    let request = Request::new("R", order("a"));
    let id = request.id;
    let (committed, _) = request.split();
    assert!(write_request(&committed).starts_with(&format!("id: {id}\n")));
}

#[test]
fn hand_edits_in_other_styles_load_and_are_rewritten_canonically() {
    let text = "\u{FEFF}kind: 'request'\r\nversion: 1\r\nname: \"Health check\"\r\norder: a0\r\nid: 0b6f3d2a-9c1e-4f5a-8b7d-2e4c6a8f0d13\r\nurl: '{{baseUrl}}/health'\r\nmethod: GET\r\nquery: []\r\nheaders: [{name: Accept, value: '*/*', enabled: true}]\r\nauth: {type: inherit}\r\nbody: {type: none}\r\nsettings: {}\r\n";
    let file = read_request(text).unwrap();
    let (_, canonical) = fixture!("request-minimal.yaml");
    let expected = format!("{canonical}headers:\n  - name: Accept\n    value: \"*/*\"\n");
    assert_eq!(write_request(&file.request), expected);
}

fn error_of(result: Result<impl std::fmt::Debug, FormatError>) -> FormatError {
    result.unwrap_err()
}

#[test]
fn errors_name_the_problem_and_the_line() {
    let base = "id: 0b6f3d2a-9c1e-4f5a-8b7d-2e4c6a8f0d13\nversion: 1\nkind: request\nname: R\norder: a0\nmethod: GET\nurl: x\n";

    let err = error_of(read_request(&format!("{base}headres:\n  - name: A\n")));
    assert_eq!(err.line, 8);
    assert_eq!(
        err.kind,
        FormatErrorKind::UnknownKey {
            path: String::new(),
            key: "headres".to_owned()
        }
    );

    let err = error_of(read_request(&format!(
        "{base}query:\n  - name: a\n  - name: b\n    enabled: maybe\n"
    )));
    assert_eq!(err.line, 11);
    assert_eq!(
        err.kind,
        FormatErrorKind::WrongType {
            path: "query[1].enabled".to_owned(),
            expected: "true or false"
        }
    );

    let err = error_of(read_request(&base.replace("0b6f3d2a", "0B6F3D2A")));
    assert_eq!(err.line, 1);
    assert!(matches!(err.kind, FormatErrorKind::InvalidValue { .. }));

    let err = error_of(read_request(&base.replace("order: a0\n", "")));
    assert_eq!(
        err.kind,
        FormatErrorKind::MissingKey {
            path: String::new(),
            key: "order".to_owned()
        }
    );

    let err = error_of(read_request(&base.replace("order: a0", "order: a-0")));
    assert!(matches!(err.kind, FormatErrorKind::InvalidValue { .. }));

    let err = error_of(read_request(&format!("{base}auth:\n  type: oauth2\n")));
    assert_eq!(err.line, 9);
    assert!(matches!(
        err.kind,
        FormatErrorKind::InvalidValue { ref path, .. } if path == "auth.type"
    ));

    let err = error_of(read_request(&format!(
        "{base}auth:\n  type: basic\n  token: x\n"
    )));
    assert_eq!(
        err.kind,
        FormatErrorKind::UnknownKey {
            path: "auth".to_owned(),
            key: "token".to_owned()
        }
    );

    let err = error_of(read_request(&format!(
        "{base}body:\n  type: multipart\n  fields:\n    - name: a\n      value: x\n      file: y\n"
    )));
    assert!(matches!(err.kind, FormatErrorKind::InvalidValue { .. }));

    let err = error_of(read_request(&format!(
        "{base}settings:\n  max_redirects: 4294967296\n"
    )));
    assert!(matches!(err.kind, FormatErrorKind::InvalidValue { .. }));

    let err = error_of(read_request(&format!("{base}name: again\n")));
    assert_eq!(err.kind, FormatErrorKind::DuplicateKey("name".to_owned()));
}

#[test]
fn errors_never_contain_values() {
    let text = "id: 0b6f3d2a-9c1e-4f5a-8b7d-2e4c6a8f0d13\nversion: 1\nkind: request\nname: R\norder: a0\nmethod: GET\nurl: x\nquery:\n  - name: a\n    enabled: SECRETVALUE\n";
    let message = error_of(read_request(text)).to_string();
    assert!(!message.contains("SECRETVALUE"), "{message}");
    assert!(message.starts_with("line 10, column 14:"), "{message}");
}

#[test]
fn version_and_kind_are_checked_first() {
    let err = error_of(read_request(
        "version: 2\nkind: request\nsomething_new: true\n",
    ));
    assert_eq!(
        err.kind,
        FormatErrorKind::NewerVersion {
            found: 2,
            supported: 1
        }
    );
    assert_eq!(err.line, 1);

    let err = error_of(read_request("version: 0\nkind: request\n"));
    assert!(matches!(err.kind, FormatErrorKind::InvalidValue { .. }));

    let err = error_of(read_request("kind: request\n"));
    assert_eq!(
        err.kind,
        FormatErrorKind::MissingKey {
            path: String::new(),
            key: "version".to_owned()
        }
    );

    let (_, folder) = fixture!("folder.yaml");
    let err = error_of(read_request(folder));
    assert_eq!(
        err.kind,
        FormatErrorKind::WrongKind {
            expected: "request",
            found: "folder".to_owned()
        }
    );
    assert!(read_collection(folder).is_err());
    assert!(read_local_values(folder).is_err());
}

#[test]
fn local_values_errors() {
    let err = error_of(read_local_values(
        "version: 1\nkind: values\nrequests:\n  not-an-id:\n    skip_tls_verify: true\n",
    ));
    assert!(matches!(err.kind, FormatErrorKind::InvalidValue { .. }));

    let err = error_of(read_local_values(
        "version: 1\nkind: values\nrequests:\n  00000000-0000-4000-8000-000000000001:\n    query:\n      - name: a\n        value: x\n      - name: a\n        occurrence: 0\n        value: y\n",
    ));
    assert_eq!(err.line, 8);
    assert!(matches!(err.kind, FormatErrorKind::InvalidValue { .. }));
}

/// A small deterministic generator, so the test needs no dependency and
/// every run checks the same strings.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Every string, however awkward, comes back unchanged through the writer
/// and the parser: as a value, inside a list, and as a mapping key.
#[test]
fn any_string_survives_the_writer_and_the_parser() {
    const PIECES: &[&str] = &[
        "a", "Z", "0", "1", " ", "  ", "\n", "\t", "\r", ":", ": ", "#", " #", "-", "- ", "?", "'",
        "\"", "\\", "{", "}", "[", "]", ",", "&", "*", "!", "|", ">", "%", "@", "`", ".", "~", "é",
        "\u{85}", "\u{2028}", "\u{FEFF}", "\u{0}", "\u{7F}", "😀", "true", "null", "no", "---",
        "...", "{{x}}", "1.5", "0x1", "<<",
    ];
    let mut rng = XorShift(0x9E37_79B9_7F4A_7C15);
    for _ in 0..20_000 {
        let len = (rng.next() % 8) as usize;
        let s: String = (0..len)
            .map(|_| PIECES[(rng.next() % PIECES.len() as u64) as usize])
            .collect();
        let doc = Value::Map(vec![
            ("value".to_owned(), Value::Str(s.clone())),
            (
                "list".to_owned(),
                Value::List(vec![Value::Map(vec![
                    ("a".to_owned(), Value::Str(s.clone())),
                    ("b".to_owned(), Value::Str(s.clone())),
                ])]),
            ),
            (
                "keys".to_owned(),
                Value::Map(vec![(s.clone(), Value::Str("x".to_owned()))]),
            ),
        ]);
        let text = render(&doc);
        let root = parse(&text).unwrap_or_else(|e| panic!("{s:?}\n{text}\n{e}"));
        let NodeKind::Map(entries) = &root.kind else {
            panic!()
        };
        let text_of = |node: &super::tree::Node| match &node.kind {
            NodeKind::Scalar { value, plain } => {
                // An empty string must not come back as null.
                assert!(!(value.is_empty() && *plain), "{s:?}\n{text}");
                value.clone()
            }
            other => panic!("{s:?}\n{text}\n{other:?}"),
        };
        assert_eq!(text_of(&entries[0].value), s, "{text}");
        let NodeKind::List(items) = &entries[1].value.kind else {
            panic!("{text}")
        };
        let NodeKind::Map(item) = &items[0].kind else {
            panic!("{text}")
        };
        assert_eq!(text_of(&item[0].value), s, "{text}");
        assert_eq!(text_of(&item[1].value), s, "{text}");
        let NodeKind::Map(keys) = &entries[2].value.kind else {
            panic!("{text}")
        };
        assert_eq!(keys[0].key, s, "{text}");
    }
}
