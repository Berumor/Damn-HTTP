//! The split between what is committed and what stays on this machine (D-021).
//!
//! For query parameters and path variables, a request file holds names, the
//! enabled flag, order and descriptions. A value is written there only when
//! it is a variable reference. Literal values are local, keyed by request id.
//!
//! [`CommittedRequest`] is the only form the file writer accepts, and the
//! only way to build one is [`Request::split`], so a literal value cannot
//! reach a committed file by accident.

use std::collections::{BTreeMap, HashMap};

use super::request::{PathParam, Request};
use crate::reference::{canonical_reference, reference_name};

/// The literal value of one query parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalQueryValue {
    pub name: String,
    /// Which parameter of this name the value belongs to, counting from 0
    /// among all parameters with the same name (`?id=1&id=2`).
    pub occurrence: usize,
    pub value: String,
}

/// Everything about one request that is stored locally and never committed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RequestLocalValues {
    /// Path variable name to literal value.
    pub path_params: BTreeMap<String, String>,
    pub query: Vec<LocalQueryValue>,
    /// D-020.
    pub skip_tls_verify: bool,
}

impl RequestLocalValues {
    /// `true` when there is nothing to store for the request.
    pub fn is_empty(&self) -> bool {
        self.path_params.is_empty() && self.query.is_empty() && !self.skip_tls_verify
    }
}

/// A request holding only what may be written to a committed file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedRequest(Request);

impl CommittedRequest {
    /// Read access for the file writer.
    pub fn as_request(&self) -> &Request {
        &self.0
    }

    /// Puts the local values back, giving the request as the editor shows it.
    ///
    /// A local value is ignored when the committed file now holds a reference
    /// in that place (a teammate shared the value): the reference wins.
    pub fn merge(self, local: &RequestLocalValues) -> Request {
        let mut request = self.0;

        let mut occurrences: HashMap<String, usize> = HashMap::new();
        for param in &mut request.query {
            let counter = occurrences.entry(param.name.clone()).or_insert(0);
            let occurrence = *counter;
            *counter += 1;
            if !param.value.is_empty() {
                continue;
            }
            if let Some(found) = local
                .query
                .iter()
                .find(|l| l.name == param.name && l.occurrence == occurrence)
            {
                param.value = found.value.clone();
            }
        }

        for (name, value) in &local.path_params {
            match request.path_params.iter_mut().find(|p| &p.name == name) {
                Some(param) if param.value.is_empty() => param.value = value.clone(),
                Some(_) => {}
                None => request.path_params.push(PathParam {
                    name: name.clone(),
                    value: value.clone(),
                    description: String::new(),
                }),
            }
        }

        request.skip_tls_verify = local.skip_tls_verify;
        request
    }
}

impl Request {
    /// Splits the request into the part that may be committed and the part
    /// that stays local.
    ///
    /// - A query or path variable value that is exactly one `{{name}}` stays,
    ///   rewritten to its canonical form.
    /// - Any other non-empty value moves to the local values.
    /// - A path variable is kept in the committed part only if it has a
    ///   reference value or a description (D-028).
    /// - `skip_tls_verify` moves to the local values.
    ///
    /// The file loader calls this too: a literal found in a hand-edited file
    /// is moved to local values instead of being kept.
    pub fn split(mut self) -> (CommittedRequest, RequestLocalValues) {
        let mut local = RequestLocalValues {
            skip_tls_verify: self.skip_tls_verify,
            ..RequestLocalValues::default()
        };
        self.skip_tls_verify = false;

        let mut occurrences: HashMap<String, usize> = HashMap::new();
        for param in &mut self.query {
            let counter = occurrences.entry(param.name.clone()).or_insert(0);
            let occurrence = *counter;
            *counter += 1;
            match reference_name(&param.value) {
                Some(name) => param.value = canonical_reference(name),
                None => {
                    let value = std::mem::take(&mut param.value);
                    if !value.is_empty() {
                        local.query.push(LocalQueryValue {
                            name: param.name.clone(),
                            occurrence,
                            value,
                        });
                    }
                }
            }
        }

        let mut kept = Vec::with_capacity(self.path_params.len());
        for mut param in self.path_params {
            match reference_name(&param.value) {
                Some(name) => {
                    param.value = canonical_reference(name);
                    kept.push(param);
                }
                None => {
                    let value = std::mem::take(&mut param.value);
                    if !value.is_empty() {
                        local.path_params.insert(param.name.clone(), value);
                    }
                    if !param.description.is_empty() {
                        kept.push(param);
                    }
                }
            }
        }
        self.path_params = kept;

        (CommittedRequest(self), local)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{OrderKey, QueryParam};

    fn request() -> Request {
        Request::new("Stations", OrderKey::new("a0").unwrap())
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

    /// The maintainer's example: `https://some.service/:country/stations/?id=1234`.
    #[test]
    fn literal_values_move_to_local() {
        let mut request = request();
        request.url = "https://some.service/:country/stations/".to_owned();
        request.path_params = vec![path("country", "it", "")];
        request.query = vec![query("id", "1234")];

        let (committed, local) = request.split();

        let committed = committed.as_request();
        assert_eq!(committed.query, [query("id", "")]);
        assert_eq!(committed.path_params, []);
        assert_eq!(
            local.path_params.get("country").map(String::as_str),
            Some("it")
        );
        assert_eq!(
            local.query,
            [LocalQueryValue {
                name: "id".to_owned(),
                occurrence: 0,
                value: "1234".to_owned()
            }]
        );
    }

    #[test]
    fn references_stay_committed_in_canonical_form() {
        let mut request = request();
        request.path_params = vec![path("country", " {{ country }} ", "")];
        request.query = vec![query("token", "{{ apiToken }}")];

        let (committed, local) = request.split();

        assert_eq!(
            committed.as_request().query,
            [query("token", "{{apiToken}}")]
        );
        assert_eq!(
            committed.as_request().path_params,
            [path("country", "{{country}}", "")]
        );
        assert!(local.is_empty());
    }

    #[test]
    fn a_value_mixing_reference_and_text_is_local_as_a_whole() {
        let mut request = request();
        request.query = vec![query("page", "{{a}}-1"), query("x", "{{a}}{{b}}")];

        let (committed, local) = request.split();

        assert_eq!(
            committed.as_request().query,
            [query("page", ""), query("x", "")]
        );
        let values: Vec<&str> = local.query.iter().map(|l| l.value.as_str()).collect();
        assert_eq!(values, ["{{a}}-1", "{{a}}{{b}}"]);
    }

    #[test]
    fn no_literal_survives_in_the_committed_part() {
        let literals = [
            "1234",
            "it",
            " ",
            "{{a}}-1",
            "Bearer abc",
            "{a}",
            "{{a b}}",
            "0",
        ];
        let mut request = request();
        for (i, literal) in literals.iter().enumerate() {
            request.query.push(query(&format!("q{i}"), literal));
            request
                .path_params
                .push(path(&format!("p{i}"), literal, "kept for its description"));
        }

        let (committed, local) = request.split();

        let committed = committed.as_request();
        assert!(committed.query.iter().all(|p| p.value.is_empty()));
        assert!(committed.path_params.iter().all(|p| p.value.is_empty()));
        assert_eq!(local.query.len(), literals.len());
        assert_eq!(local.path_params.len(), literals.len());
    }

    #[test]
    fn path_variables_are_sparse_in_the_committed_part() {
        let mut request = request();
        request.path_params = vec![
            path("plain", "1", ""),
            path("empty", "", ""),
            path("described", "2", "The station id"),
            path("shared", "{{station}}", ""),
        ];

        let (committed, _) = request.split();

        assert_eq!(
            committed.as_request().path_params,
            [
                path("described", "", "The station id"),
                path("shared", "{{station}}", "")
            ]
        );
    }

    #[test]
    fn repeated_query_names_are_matched_by_occurrence() {
        let mut request = request();
        request.query = vec![
            query("id", "1"),
            query("other", "x"),
            query("id", "{{second}}"),
            query("id", "3"),
            query("id", ""),
        ];
        let original = request.clone();

        let (committed, local) = request.split();

        let occurrences: Vec<(&str, usize)> = local
            .query
            .iter()
            .map(|l| (l.name.as_str(), l.occurrence))
            .collect();
        assert_eq!(occurrences, [("id", 0), ("other", 0), ("id", 2)]);
        assert_eq!(committed.merge(&local), original);
    }

    #[test]
    fn split_then_merge_restores_the_request() {
        let mut request = request();
        request.url = "{{baseUrl}}/:country/stations/:id".to_owned();
        request.path_params = vec![path("country", "it", ""), path("id", "{{stationId}}", "")];
        request.query = vec![
            query("limit", "20"),
            query("token", "{{apiToken}}"),
            QueryParam {
                enabled: false,
                ..query("verbose", "true")
            },
        ];
        request.skip_tls_verify = true;
        let original = request.clone();

        let (committed, local) = request.split();
        assert!(!committed.as_request().skip_tls_verify);
        assert!(local.skip_tls_verify);

        let mut merged = committed.merge(&local);
        // The committed part is sparse, so path variables come back in a
        // different order; the URL parser restores URL order when it syncs them.
        merged.path_params.sort_by(|a, b| a.name.cmp(&b.name));
        let mut expected = original;
        expected.path_params.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(merged, expected);
    }

    #[test]
    fn a_teammate_sees_empty_fields() {
        let mut request = request();
        request.path_params = vec![path("country", "it", "")];
        request.query = vec![query("id", "1234")];

        let (committed, _) = request.split();
        let on_another_machine = committed.merge(&RequestLocalValues::default());

        assert_eq!(on_another_machine.query, [query("id", "")]);
        assert_eq!(on_another_machine.path_params, []);
    }

    #[test]
    fn a_committed_reference_wins_over_a_stale_local_value() {
        let mut request = request();
        request.path_params = vec![path("country", "{{country}}", "")];
        request.query = vec![query("id", "{{stationId}}")];
        let (committed, _) = request.split();

        let stale = RequestLocalValues {
            path_params: BTreeMap::from([("country".to_owned(), "it".to_owned())]),
            query: vec![LocalQueryValue {
                name: "id".to_owned(),
                occurrence: 0,
                value: "1234".to_owned(),
            }],
            skip_tls_verify: false,
        };
        let merged = committed.merge(&stale);

        assert_eq!(merged.query, [query("id", "{{stationId}}")]);
        assert_eq!(merged.path_params, [path("country", "{{country}}", "")]);
    }

    #[test]
    fn splitting_is_idempotent() {
        let mut request = request();
        request.query = vec![query("id", "1234"), query("t", "{{ t }}")];
        let (committed, _) = request.split();
        let once = committed.as_request().clone();

        let (again, local) = once.clone().split();

        assert_eq!(again.as_request(), &once);
        assert!(local.is_empty());
    }
}
